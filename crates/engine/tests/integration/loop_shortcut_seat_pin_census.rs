//! **Row R2-b — the PROVENANCE census.** CR 601.2c vs CR 115.10a: a seat pin's SPELLING is its
//! provenance, so no TARGET-class producer may construct a `TargetPin::Player`.
//!
//! # Why a census and not a validator narrowing
//!
//! The alternative — teaching `validate_pins` / `declaration_conforms` to REJECT a
//! `TargetPin::Player` on a player-valued `Targets` slot — is refused for a measured reason.
//! `game::engine`'s `a_shrouded_player_pin_is_still_published_by_the_offer_builder` carries the
//! revert-probe *"route `resolve_target`'s `TargetPin::Player` arm through
//! `targeting::player_is_legal_target` ⇒ both assertions FAIL"*: that narrowing restores the
//! CR 115.10a over-veto, where a merely CHOSEN player (a CR 701.34a proliferate choice) is
//! refused for a targeting-only exclusion it is not subject to. It would also put a rejection
//! rule on a WIRE-VISIBLE type in order to stop a FUTURE producer from picking the wrong
//! spelling. A source census stops the same thing at no wire-compatibility cost, which is what
//! this file is.
//!
//! # What it asserts, in three conjuncts
//!
//! 1. **Neither TARGET-class producer constructs a `TargetPin::Player`.** The two producers are
//!    named individually — `game::engine::record_trigger_target_answer` (the engine's own
//!    CR 601.2c announcement journal) and
//!    `game::interaction::materialize_loop_shortcut_response` (the human ingress of the same
//!    point kind). Naming them individually is the point: a census asserting only a TOTAL
//!    production count would stay green with EITHER producer reverted, since a revert removes
//!    a ranked construction and adds a `TargetPin::Player` one, leaving the total unchanged.
//! 2. **The CHOICE-class and pass-through sites are still PRESENT and classified.** A census
//!    that counts zero of everything is vacuous, so every surviving production site is
//!    enumerated with its disposition and the list is pinned exactly.
//! 3. **Both producers do construct the ranked spelling**, keyed to the same instrument — one
//!    census returning a non-zero answer for one needle and a zero for another, on the same
//!    files, is what makes conjunct 1's zero a measurement rather than a broken grep.
//!
//! # Anti-vacuity: the instrument is validated before it is trusted
//!
//! [`the_seat_pin_census_instrument_reports_both_answers_on_planted_input`] feeds the classifier
//! a synthetic source carrying BOTH spellings inside and outside a `#[cfg(test)]` scope and
//! requires it to separate them. Without that arm, a needle that silently matched nothing would
//! report conjunct 1 as a pass.
//!
//! The `#[cfg(test)]` scope classifier and the directory walk are REUSED from
//! [`super::loop_shortcut_offer_writer_census`] rather than re-derived: that file records a
//! measured defect in the naive "nearest preceding attribute" rule, and a second copy of the
//! rule is a second place for it to be got wrong.

use std::collections::BTreeMap;
use std::path::Path;

use super::loop_shortcut_offer_writer_census::{cfg_test_scoped_lines, rs_files};

/// The CHOICE-class needle, ASSEMBLED AT RUNTIME for the same reason the sibling census
/// assembles its own: this file lives under `crates/engine/tests/`, which the walk does not
/// visit, but an instrument that would report its own text as a finding after a future move is
/// an instrument that lies about the surface it measures.
fn choice_needle() -> String {
    format!("{}::{}(", "TargetPin", "Player")
}

/// The TARGET-class needle — the subject kind that only ever reaches the resolver through a
/// `Ranking`, i.e. the spelling that IS the CR 601.2c provenance.
fn target_needle() -> String {
    format!("{}::{}(", "AnnouncementSubject", "Seat")
}

/// One classified construction/match site.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Site {
    file: String,
    line: usize,
    text: String,
}

/// Non-comment hits of `needle` in production (non-`#[cfg(test)]`) scope.
///
/// COMMENT LINES ARE EXCLUDED, and this is the same deviation the sibling census records: prose
/// writes no pin and reads none, so a doc mentioning a spelling is not a construction site. The
/// doc surface is swept separately (the commit's per-property bucket table); counting it here
/// would make the tripwire fire on prose. `//!`, `///` and `//` are all excluded; a trailing
/// comment on a code line still counts, because the CODE on that line is real.
fn production_sites(needle: &str) -> Vec<Site> {
    let engine_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let server_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("server-core")
        .join("src");
    let ai_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("phase-ai")
        .join("src");
    let mut out = Vec::new();
    for (root, prefix) in [
        (engine_src, "engine/src"),
        (server_src, "server-core/src"),
        (ai_src, "phase-ai/src"),
    ] {
        for path in rs_files(&root) {
            let src =
                std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
            let scoped = cfg_test_scoped_lines(&src);
            let rel = path
                .strip_prefix(&root)
                .expect("walked path is under its root")
                .to_string_lossy()
                .replace('\\', "/");
            for (n, line) in src.lines().enumerate() {
                if line.contains(needle) && !line.trim_start().starts_with("//") && !scoped[n] {
                    out.push(Site {
                        file: format!("{prefix}/{rel}"),
                        line: n + 1,
                        text: line.trim().to_string(),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

/// CONJUNCT 1 + 2 — neither TARGET-class producer constructs a `TargetPin::Player`, and every
/// surviving production site is CHOICE class or a pass-through arm.
///
/// # Discrimination — TWO independent revert-probes, one per producer
///
/// * restore `game::engine::record_trigger_target_answer`'s `TargetRef::Player(pl)` arm to
///   `Some(TargetPin::Player(*pl))` ⇒ `engine/src/game/engine.rs` gains an unclassified
///   `TargetPin::Player` construction ⇒ the pinned site list below FAILS with its file:line;
/// * restore `game::interaction::materialize_loop_shortcut_response`'s
///   `Target(TargetRef::Player(player))` arm to `Ok(TargetPin::Player(*player))` ⇒
///   `engine/src/game/interaction.rs` re-appears in the list ⇒ FAILS.
///
/// A census asserting only a TOTAL would catch NEITHER: each revert swaps one ranked
/// construction for one `TargetPin::Player` construction and leaves the sum where it was.
///
/// # Paired positive — the CHOICE class must still be THERE
///
/// The surviving sites are asserted PRESENT, not merely bounded: the CR 701.34a proliferate arm
/// in `engine.rs`, `resolve_target`'s CHOICE authority arm and the drive-period arm, the
/// redaction arm in `visibility.rs`, and the wire-bound arm in `server-core`. A census that
/// counted zero of everything — a broken needle, a walk that visited nothing — would satisfy
/// conjunct 1 alone.
#[test]
fn no_target_class_producer_constructs_a_choice_class_player_pin() {
    let sites = production_sites(&choice_needle());
    let located: Vec<(String, usize)> = sites
        .iter()
        .map(|s| (s.file.clone(), s.line))
        .collect::<Vec<_>>();
    let files: Vec<&str> = sites.iter().map(|s| s.file.as_str()).collect();

    // CONJUNCT 1: the two TARGET-class producers' files may still hold CHOICE-class sites
    // (`engine.rs` holds two), but a construction inside either producer is what this row
    // forbids — so the sites are pinned by exact file:line and text, not merely by file.
    assert_eq!(
        files,
        vec![
            "engine/src/analysis/decision_template.rs",
            "engine/src/game/engine.rs",
            "engine/src/game/engine.rs",
            "engine/src/game/visibility.rs",
            "server-core/src/game_action_payload_guard.rs",
        ],
        "CR 601.2c / CR 115.10a PROVENANCE SPLIT VIOLATED, or a new unclassified \
         `TargetPin::Player` production site appeared.\n\
         The FIVE surviving production sites and their dispositions:\n\
         1. `analysis/decision_template.rs` — `resolve_target`'s CHOICE authority arm \
            (CR 115.10a existence-only). This IS the class's authority; do not narrow it, \
            `a_shrouded_player_pin_is_still_published_by_the_offer_builder` is the shipped \
            guard against exactly that.\n\
         2. `game/engine.rs` — `shortcut_drive_period`'s period arm; the migrated pin lands on \
            the `Scheduled(Constant(_))` arm of the SAME match, which also returns 1.\n\
         3. `game/engine.rs` — `apply_action`'s CR 701.34a proliferate arm feeding \
            `record_loop_pin`. A proliferate choice is NOT a target, so this construction is \
            correct and is this census's positive control.\n\
         4. `game/visibility.rs` — `pins_name_hidden_source`'s redaction arm (`false`: seat \
            identity is public in this engine), kept for wire-sourced pins.\n\
         5. `server-core/src/game_action_payload_guard.rs` — the wire pass-through arm.\n\
         A hit in `game/interaction.rs` means the HUMAN ingress reverted to the choice-class \
         spelling; a THIRD hit in `game/engine.rs` means the announcement journal did. Either \
         re-creates two authorities selected by WHO SUBMITTED an answer rather than by WHAT IT \
         IS. got {sites:?}"
    );

    // The `engine.rs` pair is pinned to its two ARMS by text, so a construction added inside
    // `record_trigger_target_answer` cannot hide behind the file already being listed twice.
    let engine_texts: Vec<&str> = sites
        .iter()
        .filter(|s| s.file == "engine/src/game/engine.rs")
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(
        engine_texts.len(),
        2,
        "the file-level list above is pinned at two `engine.rs` sites; if that changes the \
         text pin below is measuring the wrong things. got {engine_texts:?}"
    );
    assert!(
        engine_texts[0].starts_with("| TargetPin::Player(_) =>"),
        "site 2 must remain `shortcut_drive_period`'s MATCH arm — a match arm reads a pin, it \
         cannot produce one. got {:?}",
        engine_texts[0]
    );
    assert!(
        engine_texts[1].contains("TargetPin::Player(*pl)"),
        "site 3 must remain the CR 701.34a proliferate CONSTRUCTION — this census's positive \
         control, and the one production construction of the choice class. got {:?}",
        engine_texts[1]
    );

    // CONJUNCT 3, keyed to the SAME instrument: both TARGET-class producers construct the
    // ranked spelling. One needle returning five sites and the other returning the two
    // producers, over the same walk, is what makes conjunct 1's absence a measurement.
    let ranked = production_sites(&target_needle());
    let mut ranked_per_file: BTreeMap<&str, usize> = BTreeMap::new();
    for s in &ranked {
        *ranked_per_file.entry(s.file.as_str()).or_default() += 1;
    }
    let ranked_files: Vec<(&str, usize)> = ranked_per_file.into_iter().collect();
    assert_eq!(
        ranked_files,
        vec![
            ("engine/src/analysis/decision_template.rs", 1),
            ("engine/src/game/engine.rs", 1),
            ("engine/src/game/interaction.rs", 1),
            ("engine/src/game/visibility.rs", 1),
        ],
        "the TARGET-class spelling must be CONSTRUCTED by BOTH producers — `game/engine.rs` \
         (`record_trigger_target_answer`) and `game/interaction.rs` \
         (`materialize_loop_shortcut_response`) — beside its two READ sites: \
         `evaluate_schedule`'s CR 601.2c resolver arm in `analysis/decision_template.rs` and \
         the wildcard-free redaction arm in `game/visibility.rs`. A MISSING producer is the \
         revert this row exists to catch; an EXTRA file is a new producer that must be \
         classified rather than absorbed. got {ranked:?}"
    );
    assert!(
        !located.is_empty(),
        "keying control: the choice-class needle must return a NON-EMPTY set, or its \
         per-producer absence above would be the answer a dead instrument gives"
    );
}

/// ANTI-VACUITY ARM — the instrument returns BOTH answers on planted input.
///
/// The classifier is fed one synthetic source carrying the choice-class spelling and the
/// target-class spelling, each in production scope, inside a `#[cfg(test)] pub(crate) mod`, and
/// on a comment line. It must report exactly the production, non-comment hits.
///
/// # Discrimination
///
/// * delete the `!scoped[n]` conjunct ⇒ the mod-scoped plants count ⇒ `(1, 1)` becomes
///   `(2, 2)` ⇒ FAILS;
/// * delete the comment filter ⇒ the prose plants count ⇒ `(1, 1)` becomes `(2, 2)` ⇒ FAILS.
///
/// It is measured on the real tree too, in the row above: the same classifier returns five
/// sites for one needle and four files for the other, so it is not constant in either
/// direction.
#[test]
fn the_seat_pin_census_instrument_reports_both_answers_on_planted_input() {
    let choice = choice_needle();
    let target = target_needle();
    let src = format!(
        "fn production_side() {{\n\
         \x20   let a = {choice}PlayerId(0));\n\
         \x20   let b = Ranking::one({target}PlayerId(1)));\n\
         \x20   // prose about {choice}..) and {target}..) that constructs nothing\n\
         }}\n\
         \n\
         #[cfg(test)]\n\
         pub(crate) mod tests {{\n\
         \x20   fn test_side() {{\n\
         \x20       let a = {choice}PlayerId(0));\n\
         \x20       let b = Ranking::one({target}PlayerId(1)));\n\
         \x20   }}\n\
         }}\n"
    );

    let scoped = cfg_test_scoped_lines(&src);
    let count = |needle: &str| {
        src.lines()
            .enumerate()
            .filter(|(n, line)| {
                line.contains(needle) && !line.trim_start().starts_with("//") && !scoped[*n]
            })
            .count()
    };

    assert_eq!(
        (count(&choice), count(&target)),
        (1, 1),
        "the classifier must see exactly the ONE production, non-comment construction of each \
         spelling: the `#[cfg(test)] pub(crate) mod` copies belong in the test column and the \
         prose line is not a construction site. Dropping either filter makes this (2, 2).\n\
         src:\n{src}"
    );
}
