// pod-lab loop-3 Q5: native-binary throughput lever, gated in Cargo.toml so
// wasm32 builds of this crate's lib (pulled in by engine-wasm/draft-wasm)
// never see it.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::path::{Path, PathBuf};
use std::process::Command;

use engine::database::CardDatabase;
use phase_ai::config::AiDifficulty;
use phase_ai::duel_suite::compare::{
    compare, emit_gate_verdict, load_report, print_markdown, render_error_markdown, CompareOptions,
};
use phase_ai::duel_suite::run::{run_suite, SuiteOptions};

const DEFAULT_BASELINE: &str = "crates/phase-ai/baselines/suite-baseline.json";
const DEFAULT_CURRENT: &str = "target/ai-gate-current.json";
// Quick PR-gate matchup set (comma-separated id substrings). `red-mirror` is the
// fast aggro-mirror smoke; `affinity-mirror` and `enchantress-mirror` are the
// floor-crossing artifacts/enchantments decks that exercise ArtifactSynergyPolicy
// and EnchantmentsPayoffPolicy (commitment >= COMMITMENT_FLOOR), so the required
// gate actually runs the policies these baselines are meant to guard.
const DEFAULT_QUICK_FILTER: &str = "red-mirror,affinity-mirror,enchantress-mirror";
const DEFAULT_SEED: u64 = 0xA1_57A1;

struct Args {
    data_root: PathBuf,
    baseline: PathBuf,
    current_output: PathBuf,
    games: usize,
    seed: u64,
    difficulty: AiDifficulty,
    suite_filter: Option<String>,
    refresh_baseline: bool,
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            std::process::exit(2);
        }
    };

    // Refuse before ANY work — before the card database, before a single game — when the
    // suite's output path and the baseline are the same file. Review found this defeats the
    // refusal this PR adds: `run_suite` writes its report to `options.output_path` before any
    // guard runs, so an aliased pair truncated the baseline and only THEN printed "refusing to
    // refresh". Measured on the real binary at the reviewed head: 116 bytes in, 250 bytes out,
    // different sha256, exit 1.
    //
    // The compare path is worse and is why this check is not confined to `--refresh-baseline`.
    // There the same aliasing makes `load_report(&args.baseline)` read back the run that just
    // overwrote it, so the gate compares the run to ITSELF and exits 0. Measured: a baseline
    // recording p0 at 100% against a run that scored 0% printed `0% | 0%`, zero flips, `0 FAIL`,
    // exit 0. A gate that reports no drift because it destroyed its own reference is the exact
    // false-green this branch exists to close, and it is silent where the refresh case is loud.
    if same_file(&args.baseline, &args.current_output) {
        eprintln!(
            "--baseline and --current-output are the same file ({}); the suite would overwrite \
             the baseline before it could be compared or validated",
            args.baseline.display()
        );
        std::process::exit(2);
    }

    let db_path = args.data_root.join("card-data.json");
    let db = match CardDatabase::from_export(&db_path) {
        Ok(db) => db,
        Err(err) => {
            eprintln!(
                "failed to load card database from {}: {err}",
                db_path.display()
            );
            std::process::exit(2);
        }
    };

    let mut options = SuiteOptions::new(args.difficulty, args.games, args.seed);
    // On a refresh, the suite writes to a staging file BESIDE the baseline rather than to
    // `--current-output`, and the baseline is replaced by renaming that file only after every
    // guard has passed. The alias check above already refuses the one path that reached this
    // bug, but a check on argument values cannot be the whole answer: the property that has to
    // hold is that a rejected run never modifies the baseline, and that is a property of the
    // write ordering, not of the flags. Staging + rename gives it unconditionally.
    //
    // Beside the baseline because `rename` is only atomic within one filesystem; a staging file
    // in `/tmp` could land on a different mount and silently degrade to copy-then-truncate.
    // `--current-output` is therefore unused on the refresh path — the refreshed baseline IS
    // the run's report. No workflow passes `--refresh-baseline`, so nothing in CI depends on
    // the old behaviour.
    let staging = args.refresh_baseline.then(|| staging_path(&args.baseline));
    options.output_path = staging
        .clone()
        .unwrap_or_else(|| args.current_output.clone());
    options.filter = args.suite_filter.clone();
    options.git_sha = command_output("git", &["rev-parse", "--short=12", "HEAD"]);
    options.card_data_hash = command_output("git", &["hash-object", path_str(&db_path)]);

    // Read the baseline BEFORE the suite runs, and keep it in memory.
    //
    // This is the root-cause half of the aliasing fix, and it is what the argument check above
    // cannot give: reading first makes the comparison independent of ANYTHING the suite writes,
    // including aliases this process cannot detect from paths at all — a hard link resolves to
    // a different name and the same inode, so `same_file` calls it distinct while a write to
    // one truncates the other. Order does not care how the alias was constructed.
    //
    // It also fails a missing or corrupt baseline in a second instead of after a full suite run,
    // which is the difference between a typo costing nothing and costing a hundred games.
    let baseline = match load_report(&args.baseline) {
        Ok(report) => Some(report),
        // On a refresh there may be no baseline yet, and that is the normal first-run case.
        Err(_) if args.refresh_baseline && !args.baseline.exists() => None,
        Err(err) if args.refresh_baseline => {
            eprintln!("could not read the old baseline for comparison: {err}");
            None
        }
        Err(err) => {
            // Same reasoning as the compare refusal below: the nightly posts stdout, so a
            // read failure that spoke only to stderr produced a red job whose issue body was
            // the suite table and no statement of what went wrong. This is also the only
            // caller that can reach `render_error_markdown`'s I/O arm — `compare` does no
            // I/O, so before this the arm existed and was unreachable.
            eprintln!("failed to load baseline {}: {err}", args.baseline.display());
            print!("{}", render_error_markdown(&err));
            std::process::exit(2);
        }
    };

    let current = match run_suite(&db, &options) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("suite run failed: {err}");
            std::process::exit(1);
        }
    };

    if args.refresh_baseline {
        // A baseline is what every later run is judged against, so refreshing from a run
        // that failed its own `Expected` check blesses that failure permanently: the next
        // run compares equal to it and exits 0 forever, and the gate goes quiet about a
        // matchup that is still broken. Refuse. A matchup that genuinely has no verdict
        // yet says so with `Expected::Open` in the suite definition — that is the place
        // to express it, not a red baseline.
        //
        // ORDER MATTERS, and this one is strictly better on every input. The two conditions
        // are not exclusive: `failed_result` builds a matchup with an empty `games` vector
        // AND `SuiteStatus::Fail`, so a run whose deck payloads all failed to load satisfies
        // both. Reporting the failures names each matchup and its `setup error: …`; reporting
        // gamelessness first would replace that with a sentence about seeds. Nothing is lost
        // by checking failures first, because a run that is merely gameless — a
        // `--suite-filter` matching nothing — has no failing matchups to report.
        let staging = staging.expect("staging path is set whenever refresh_baseline is");
        // Every exit below leaves the staging file behind otherwise, and a stale
        // `*.staging.json` next to a baseline is exactly the kind of artefact someone later
        // mistakes for a real one.
        let refuse = |message: &str| -> ! {
            let _ = std::fs::remove_file(&staging);
            eprintln!("{message}");
            std::process::exit(1);
        };

        let failing: Vec<_> = current.failing_matchups().collect();
        if !failing.is_empty() {
            eprintln!(
                "refusing to refresh {}: {} matchup(s) failed their own suite check",
                args.baseline.display(),
                failing.len()
            );
            for result in failing {
                eprintln!(
                    "  {}: {}",
                    result.matchup_id,
                    result
                        .fail_reason
                        .as_deref()
                        .unwrap_or("no reason recorded")
                );
            }
            refuse(
                "fix the regression, or declare the matchup `Expected::Open` if it has no verdict yet",
            );
        }
        // A run that measured nothing is unfit for the same reason a red one is, reached from
        // the other side: comparison pairs by seed, so a gameless baseline scores zero on the
        // outcome axes forever and the drift signal dies quietly. Reached by a `--suite-filter`
        // that selects no matchups; `--games 0` is refused earlier, at parse time.
        if current.recorded_games() == 0 {
            refuse(&format!(
                "refusing to refresh {}: the run recorded no games, so every later comparison would score zero",
                args.baseline.display()
            ));
        }
        // Informational old-vs-new diff, from the copy read before the suite ran.
        if let Some(old) = &baseline {
            match compare(old, &current, &CompareOptions) {
                Ok(report) => print_markdown(&report),
                Err(err) => eprintln!("could not compare old baseline: {err}"),
            }
        }
        // The run is accepted: promote the staging file. `rename` replaces the baseline in one
        // step, so a reader never observes a half-written baseline and a failure here leaves the
        // previous one intact.
        if let Err(err) = std::fs::rename(&staging, &args.baseline) {
            let _ = std::fs::remove_file(&staging);
            eprintln!(
                "failed to write baseline {}: {err}",
                args.baseline.display()
            );
            std::process::exit(1);
        }
        eprintln!("baseline refreshed at {}", args.baseline.display());
        return;
    }

    let baseline =
        baseline.expect("the non-refresh path exits above when the baseline is unreadable");

    // stdout carries the report body — the nightly redirects it into the file it posts as a
    // drift issue — so a refusal has to be printed there too, not only to stderr. `gate_verdict`
    // owns both halves so the pair is testable; `main` prints and exits.
    let comparison = compare(&baseline, &current, &CompareOptions);
    if let Err(err) = &comparison {
        eprintln!("compare failed: {err}");
    }
    let code = emit_gate_verdict(&comparison);
    if code != 0 {
        std::process::exit(code);
    }
}

fn parse_args() -> Result<Args, String> {
    let mut data_root = std::env::var("PHASE_CARDS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"));
    let mut baseline = PathBuf::from(DEFAULT_BASELINE);
    let mut current_output = PathBuf::from(DEFAULT_CURRENT);
    let mut games = 10usize;
    let mut seed = DEFAULT_SEED;
    let mut difficulty = AiDifficulty::Medium;
    let mut suite_filter = Some(DEFAULT_QUICK_FILTER.to_string());
    let mut refresh_baseline = false;

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--data-root" => {
                data_root = next_path(&mut iter, "--data-root")?;
            }
            "--baseline" => {
                baseline = next_path(&mut iter, "--baseline")?;
            }
            "--current-output" => {
                current_output = next_path(&mut iter, "--current-output")?;
            }
            "--games" => {
                // `usize` alone accepts 0, which the error string already promised it would
                // not. A zero-game run classifies every matchup `Open` and produces a
                // baseline that can never detect drift, so reject it here rather than
                // burning a whole suite run to refuse it later.
                games = match next_value(&mut iter, "--games")?.parse() {
                    Ok(0) | Err(_) => return Err("--games must be a positive integer".to_string()),
                    Ok(value) => value,
                };
            }
            "--seed" => {
                seed = next_value(&mut iter, "--seed")?
                    .parse()
                    .map_err(|_| "--seed must be an integer".to_string())?;
            }
            "--difficulty" => {
                // Case-insensitive; unknown labels fall back to Medium via
                // `AiDifficulty::from_label`. Run the same difficulty on branch
                // and baseline so the pair isolates the code delta.
                difficulty = AiDifficulty::from_label(&next_value(&mut iter, "--difficulty")?);
            }
            "--suite-filter" => {
                suite_filter = Some(next_value(&mut iter, "--suite-filter")?);
            }
            "--full-suite" => suite_filter = None,
            "--refresh-baseline" => refresh_baseline = true,
            "--help" | "-h" => return Err(String::new()),
            _ => return Err(format!("unknown option: {arg}")),
        }
    }

    Ok(Args {
        data_root,
        baseline,
        current_output,
        games,
        seed,
        difficulty,
        suite_filter,
        refresh_baseline,
    })
}

fn next_path(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    next_value(iter, flag).map(PathBuf::from)
}

fn next_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    iter.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn path_str(path: &Path) -> &str {
    path.to_str().unwrap_or("")
}

/// Whether two paths designate the same file, including through symlinks and `..`.
///
/// `canonicalize` is the authority when a path exists, because it is the only thing that
/// resolves symlinks — a plain string or `absolute()` comparison calls `baselines/x.json` and a
/// symlink pointing at it different files, and then the write lands on the baseline anyway. The
/// current-output side usually does NOT exist yet, so it falls back to canonicalizing the parent
/// directory (which has to be real for the write to land) and rejoining the file name.
///
/// Returns false when neither resolution is possible, which is the right default: an
/// unresolvable path cannot be shown to alias, and refusing to run on a path we cannot inspect
/// would break invocations that are fine.
fn same_file(a: &Path, b: &Path) -> bool {
    fn resolved(path: &Path) -> Option<PathBuf> {
        if let Ok(canonical) = std::fs::canonicalize(path) {
            return Some(canonical);
        }
        let parent = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        Some(std::fs::canonicalize(parent).ok()?.join(path.file_name()?))
    }
    match (resolved(a), resolved(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// Where a refresh run stages its report before it earns the right to be the baseline.
///
/// Beside the baseline, so the later `rename` is a same-filesystem atomic replace.
fn staging_path(baseline: &Path) -> PathBuf {
    let name = baseline
        .file_name()
        .map(|n| {
            let mut s = n.to_os_string();
            s.push(".staging.json");
            s
        })
        .unwrap_or_else(|| "baseline.staging.json".into());
    baseline.with_file_name(name)
}

fn print_usage() {
    eprintln!("Usage: cargo ai-gate [--refresh-baseline] [--games N] [--seed S]");
    eprintln!("                     [--difficulty {{medium|hard|veryhard|cedh}}]");
    eprintln!("                     [--suite-filter STR[,STR...] | --full-suite]");
    eprintln!("                     [--data-root DIR] [--baseline PATH] [--current-output PATH]");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The staging file must be a SIBLING of the baseline. `rename` is only atomic within one
    /// filesystem, so a staging path that drifted to `/tmp` (or anywhere else the baseline is
    /// not) would silently degrade the final replace into copy-then-truncate — reintroducing the
    /// half-written baseline this staging exists to prevent, and doing it invisibly.
    ///
    /// Asserted as "same parent, different file name", which is the property atomicity needs,
    /// rather than as a literal string, which would pin a spelling nobody depends on.
    #[test]
    fn the_staging_file_is_a_sibling_of_the_baseline_it_will_replace() {
        for baseline in [
            "crates/phase-ai/baselines/suite-baseline.json",
            "/abs/path/base.json",
            "relative.json",
            "/weird/no-extension",
        ] {
            let baseline = Path::new(baseline);
            let staging = staging_path(baseline);
            assert_eq!(
                staging.parent(),
                baseline.parent(),
                "staging must sit beside {}, got {}",
                baseline.display(),
                staging.display()
            );
            assert_ne!(
                staging,
                baseline,
                "staging must not BE the baseline: {}",
                baseline.display()
            );
        }
    }

    /// A path and a symlink to it are the same file, and a string comparison cannot see that.
    /// This is the case that makes `same_file` more than `a == b`: the write lands on the
    /// baseline's bytes either way.
    #[test]
    fn same_file_sees_through_a_symlink() {
        let dir = std::env::temp_dir().join(format!("phase-same-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let real = dir.join("baseline.json");
        std::fs::write(&real, "{}").expect("write");
        let link = dir.join("link.json");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).expect("symlink");

        #[cfg(unix)]
        {
            assert!(same_file(&real, &link), "symlinked alias must be detected");
            // PREMISE: the two paths really are textually different, so the assertion above is
            // about resolution rather than about a trivially equal comparison.
            assert_ne!(real, link);
        }

        // Control: two genuinely distinct files must not be called aliases, or the guard would
        // refuse every legitimate invocation.
        let other = dir.join("current.json");
        std::fs::write(&other, "{}").expect("write");
        assert!(!same_file(&real, &other));
        // And a path that does not exist yet still resolves through its parent, which is the
        // normal case for `--current-output` on a clean tree.
        assert!(!same_file(&real, &dir.join("not-created-yet.json")));
        assert!(same_file(&real, &dir.join("baseline.json")));

        std::fs::remove_dir_all(&dir).ok();
    }
}
