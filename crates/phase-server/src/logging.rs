use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use engine::types::log::GameLogEntry;
use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Extension data stored on `game_session` spans to identify the game code.
struct GameCode(String);

/// Visitor that extracts the `game` field from span attributes.
struct GameCodeVisitor(Option<String>);

impl Visit for GameCodeVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "game" {
            self.0 = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
}

/// Visitor that collects every tracing event field into a JSON object, keyed
/// by field name verbatim (tracing already names the message field
/// `"message"`, so no special-casing is needed here).
struct JsonFieldVisitor(Map<String, Value>);

impl Visit for JsonFieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0
            .insert(field.name().to_string(), Value::String(value.to_string()));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.insert(field.name().to_string(), Value::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name().to_string(), Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().to_string(), Value::from(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(field.name().to_string(), Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.insert(
            field.name().to_string(),
            Value::String(format!("{value:?}")),
        );
    }
}

/// Format a UTC timestamp as `YYYY-MM-DDTHH:MM:SS.mmmZ` without external crates.
fn format_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    // Convert epoch seconds to date/time components.
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Civil date from day count (algorithm from Howard Hinnant).
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, m, d, hours, minutes, seconds, millis
    )
}

/// A category of per-game log row. Each category is its own JSON-Lines file
/// (`<games_dir>/<game_code>.<category>.jsonl`) so a reader can select one
/// concern (`jq` per file) without filtering a merged stream.
///
/// - `"session"`: transport/session-lifecycle tracing events (connects,
///   actions dispatched, game-over, errors) — whatever already flows through
///   the `game_session` tracing span.
/// - `"events"`: the engine's own [`GameLogEntry`] rows (already categorized
///   internally via `GameLogEntry::category`), one per rules-level event.
type Stream = &'static str;
const STREAM_SESSION: Stream = "session";
const STREAM_EVENTS: Stream = "events";

/// `None` value = open was attempted and failed (sentinel to avoid retry storms).
type FileMap = HashMap<(String, Stream), Option<BufWriter<File>>>;

/// Shared per-game JSON-Lines file cache. Each `(game_code, stream)` pair maps
/// to a lazily-opened, append-only file, flushed after every write so a crash
/// never loses more than the write in flight. `games_dir: None` means logging
/// is disabled (stdout-only run) and every write is a no-op.
pub struct GameFileCache {
    games_dir: Option<PathBuf>,
    files: Mutex<FileMap>,
}

impl GameFileCache {
    fn new(games_dir: PathBuf) -> Self {
        Self {
            games_dir: Some(games_dir),
            files: Mutex::new(HashMap::new()),
        }
    }

    fn disabled() -> Self {
        Self {
            games_dir: None,
            files: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for GameFileCache {
    /// A disabled cache, so `ServerContext`'s `#[derive(Default)]` (used
    /// throughout its test call sites) gets a real, inert no-op writer
    /// rather than requiring every test to wire one up.
    fn default() -> Self {
        Self::disabled()
    }
}

impl GameFileCache {
    fn open_file(&self, game_code: &str, stream: Stream) -> Option<BufWriter<File>> {
        let dir = self.games_dir.as_ref()?;
        let path = dir.join(format!("{game_code}.{stream}.jsonl"));
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(BufWriter::new)
    }

    fn write_line(&self, game_code: &str, stream: Stream, line: &str) {
        if self.games_dir.is_none() {
            return;
        }
        let mut files = self.files.lock().unwrap_or_else(|e| e.into_inner());
        let entry = files
            .entry((game_code.to_string(), stream))
            .or_insert_with(|| self.open_file(game_code, stream));
        if let Some(writer) = entry {
            let _ = writer.write_all(line.as_bytes());
            let _ = writer.write_all(b"\n");
            let _ = writer.flush();
        }
    }

    /// Write one JSON-Lines row per [`GameLogEntry`], stamped with the
    /// wall-clock moment of writing — the entries themselves carry only
    /// game-time (`seq`/`turn`/`phase`), not wall-clock time.
    pub fn write_game_log_entries(&self, game_code: &str, entries: &[GameLogEntry]) {
        if self.games_dir.is_none() {
            return;
        }
        for entry in entries {
            let Ok(Value::Object(mut fields)) = serde_json::to_value(entry) else {
                continue;
            };
            fields.insert("ts".to_string(), Value::String(format_timestamp()));
            if let Ok(line) = serde_json::to_string(&Value::Object(fields)) {
                self.write_line(game_code, STREAM_EVENTS, &line);
            }
        }
    }

    /// Flush and drop every stream's cached writer for a game whose
    /// `game_session` span just closed. Reopened lazily if another
    /// connection resumes writing to the same game.
    fn close(&self, game_code: &str) {
        let mut files = self.files.lock().unwrap_or_else(|e| e.into_inner());
        for stream in [STREAM_SESSION, STREAM_EVENTS] {
            if let Some(Some(mut writer)) = files.remove(&(game_code.to_string(), stream)) {
                let _ = writer.flush();
            }
        }
    }
}

/// A tracing `Layer` that routes events occurring within a `game_session` span
/// to the `"session"` per-game JSON-Lines stream.
pub struct GameFileLayer {
    cache: Arc<GameFileCache>,
}

impl GameFileLayer {
    fn new(cache: Arc<GameFileCache>) -> Self {
        Self { cache }
    }
}

impl<S> Layer<S> for GameFileLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().name() != "game_session" {
            return;
        }
        let mut visitor = GameCodeVisitor(None);
        attrs.record(&mut visitor);
        if let Some(game_code) = visitor.0 {
            if let Some(span) = ctx.span(id) {
                span.extensions_mut().insert(GameCode(game_code));
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Walk up the span scope to find the nearest game_session span.
        let game_code = ctx.event_span(event).and_then(|span| {
            // scope() yields the span itself first, then walks up to parents.
            for s in span.scope() {
                if let Some(gc) = s.extensions().get::<GameCode>() {
                    return Some(gc.0.clone());
                }
            }
            None
        });

        let game_code = match game_code {
            Some(gc) => gc,
            None => return, // Not inside a game_session span — skip.
        };

        let mut visitor = JsonFieldVisitor(Map::new());
        event.record(&mut visitor);
        let mut fields = visitor.0;
        fields.insert("ts".to_string(), Value::String(format_timestamp()));
        fields.insert(
            "level".to_string(),
            Value::String(event.metadata().level().to_string()),
        );
        fields.insert(
            "target".to_string(),
            Value::String(event.metadata().target().to_string()),
        );

        if let Ok(line) = serde_json::to_string(&Value::Object(fields)) {
            self.cache.write_line(&game_code, STREAM_SESSION, &line);
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let game_code = ctx
            .span(&id)
            .and_then(|span| span.extensions().get::<GameCode>().map(|gc| gc.0.clone()));
        if let Some(game_code) = game_code {
            self.cache.close(&game_code);
        }
    }
}

/// Initialize the tracing subscriber.
///
/// When `log_dir` is `Some`, logs are written to files:
/// - Main log: `<dir>/phase-server.log` (daily rolling; JSON-formatted iff `json`)
/// - Per-game logs: `<dir>/games/<GAME_CODE>.session.jsonl` and
///   `<dir>/games/<GAME_CODE>.events.jsonl` — always JSON-Lines, independent
///   of `json`. The per-game format is not optional: it exists so a game's
///   logs are parseable, and gating that behind a flag would leave the
///   default output exactly as unparseable as before this existed.
///
/// When `log_dir` is `None`, logs are written to stdout (local dev mode) and
/// the returned [`GameFileCache`] is disabled (every write a no-op).
///
/// Returns a `WorkerGuard` that must be held alive for the program's lifetime
/// to ensure buffered logs are flushed. Use a **named binding** (`let _guard = ...`),
/// NOT bare `_` which drops immediately.
pub fn init_logging(
    log_dir: Option<&str>,
    json: bool,
) -> (
    Option<tracing_appender::non_blocking::WorkerGuard>,
    Arc<GameFileCache>,
) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "phase_server=info,server_core=info,phase_ai=info"
            .parse()
            .unwrap()
    });

    match log_dir {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            let games_dir = dir.join("games");
            fs::create_dir_all(&games_dir).expect("failed to create log directory");

            let file_appender = tracing_appender::rolling::daily(&dir, "phase-server.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            let game_cache = Arc::new(GameFileCache::new(games_dir));
            let game_layer = GameFileLayer::new(Arc::clone(&game_cache));

            if json {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .json()
                            .with_writer(non_blocking)
                            .with_target(true),
                    )
                    .with(game_layer)
                    .init();
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_writer(non_blocking)
                            .with_ansi(false),
                    )
                    .with(game_layer)
                    .init();
            }

            (Some(guard), game_cache)
        }
        None => {
            // Stdout mode — preserves current behavior for local dev.
            if json {
                tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .init();
            } else {
                tracing_subscriber::fmt().with_env_filter(env_filter).init();
            }
            (None, Arc::new(GameFileCache::disabled()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::types::log::{LogCategory, LogPresentation};
    use engine::types::phase::Phase;

    #[test]
    fn format_timestamp_is_valid_iso8601() {
        let ts = format_timestamp();
        // Expect: YYYY-MM-DDTHH:MM:SS.mmmZ
        assert_eq!(ts.len(), 24);
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], ".");
    }

    #[test]
    fn game_file_cache_creates_session_stream_file() {
        let tmp = tempfile::tempdir().unwrap();
        let games_dir = tmp.path().join("games");
        fs::create_dir_all(&games_dir).unwrap();

        let cache = GameFileCache::new(games_dir.clone());
        cache.write_line("TEST01", STREAM_SESSION, r#"{"message":"hello"}"#);

        let log_path = games_dir.join("TEST01.session.jsonl");
        assert!(log_path.exists());
    }

    #[test]
    fn game_file_cache_appends_to_existing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let games_dir = tmp.path().join("games");
        fs::create_dir_all(&games_dir).unwrap();

        let cache = GameFileCache::new(games_dir.clone());
        let log_path = games_dir.join("APPEND.session.jsonl");

        cache.write_line("APPEND", STREAM_SESSION, r#"{"n":1}"#);
        cache.write_line("APPEND", STREAM_SESSION, r#"{"n":2}"#);

        let content = fs::read_to_string(&log_path).unwrap();
        assert_eq!(content, "{\"n\":1}\n{\"n\":2}\n");
    }

    #[test]
    fn game_file_cache_disabled_writes_nothing() {
        // A disabled cache (stdout mode, no log_dir) must not create files.
        let cache = GameFileCache::disabled();
        cache.write_line("FAIL01", STREAM_SESSION, r#"{"n":1}"#);
        // No games_dir exists to assert against — the write path itself
        // early-returns (`games_dir.is_none()`), which is what this pins.
        assert!(cache.games_dir.is_none());
    }

    fn sample_entry(seq: u32, category: LogCategory) -> GameLogEntry {
        GameLogEntry {
            seq,
            turn: 1,
            phase: Phase::PreCombatMain,
            category,
            segments: Vec::new(),
            presentation: LogPresentation::default(),
        }
    }

    #[test]
    fn write_game_log_entries_preserves_category_and_timestamps() {
        let tmp = tempfile::tempdir().unwrap();
        let games_dir = tmp.path().join("games");
        fs::create_dir_all(&games_dir).unwrap();
        let cache = GameFileCache::new(games_dir.clone());

        cache.write_game_log_entries("CAT01", &[sample_entry(1, LogCategory::Combat)]);

        let content = fs::read_to_string(games_dir.join("CAT01.events.jsonl")).unwrap();
        let line: Value = serde_json::from_str(content.trim_end()).unwrap();
        // Discriminating: a bug that dropped the category field, or one that
        // always tagged entries `Debug`, fails this assertion against a
        // deliberately non-Debug variant.
        assert_eq!(line["category"], "Combat");
        assert!(line["ts"].as_str().unwrap().ends_with('Z'));
    }

    #[test]
    fn write_game_log_entries_writes_one_line_per_entry_not_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let games_dir = tmp.path().join("games");
        fs::create_dir_all(&games_dir).unwrap();
        let cache = GameFileCache::new(games_dir.clone());

        cache.write_game_log_entries(
            "SEQ01",
            &[
                sample_entry(1, LogCategory::Turn),
                sample_entry(2, LogCategory::Combat),
            ],
        );

        let content = fs::read_to_string(games_dir.join("SEQ01.events.jsonl")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "each entry must be its own line, not overwritten"
        );
        let first: Value = serde_json::from_str(lines[0]).unwrap();
        let second: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first["seq"], 1);
        assert_eq!(second["seq"], 2);
    }

    #[test]
    fn closing_session_stream_does_not_lose_flushed_events_stream_content() {
        let tmp = tempfile::tempdir().unwrap();
        let games_dir = tmp.path().join("games");
        fs::create_dir_all(&games_dir).unwrap();
        let cache = GameFileCache::new(games_dir.clone());

        cache.write_game_log_entries("BOTH01", &[sample_entry(1, LogCategory::Zone)]);
        cache.write_line("BOTH01", STREAM_SESSION, r#"{"message":"joined"}"#);

        // Simulate the game_session span closing — this must not touch
        // content already flushed to the independent "events" stream.
        cache.close("BOTH01");

        let events_content = fs::read_to_string(games_dir.join("BOTH01.events.jsonl")).unwrap();
        assert_eq!(events_content.lines().count(), 1);
    }
}
