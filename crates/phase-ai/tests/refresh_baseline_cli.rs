//! The baseline's file-state contract, asserted against the real binary.
//!
//! Review found that the refusal this crate's `--refresh-baseline` guard adds could fire *after*
//! the damage it exists to prevent: `run_suite` writes its report to the suite's output path
//! before any guard runs, so pointing `--current-output` at the baseline truncated the baseline
//! and only then printed "refusing to refresh". Measured on the pre-fix binary: 116 bytes in,
//! 250 out, different sha256, exit 1. A guard whose subject is already destroyed is not a guard.
//!
//! Every assertion here is on the pair (exit status, baseline bytes), because either alone is
//! satisfied by a broken implementation: exiting non-zero while having clobbered the file is the
//! bug itself, and leaving the file alone while exiting 0 would bless the run.
//!
//! These run the real CLI and deliberately never reach the card database — the argument check
//! under test rejects before any of that — so they cost milliseconds and add no card-data load
//! for `scripts/check-test-card-data-load.sh` to object to.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Not a valid suite report, and that is deliberate: these tests must fail if the binary gets
/// far enough to parse it, because getting that far means it also ran the suite.
const SENTINEL: &str = r#"{"this":"is the trusted baseline, byte for byte"}"#;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("phase-refresh-cli-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn seed_baseline(dir: &Path) -> PathBuf {
    let path = dir.join("suite-baseline.json");
    std::fs::write(&path, SENTINEL).expect("seed baseline");
    path
}

fn run(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ai-gate"))
        .args(args)
        .output()
        .expect("spawn ai-gate");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The recorded defect, in the direction that destroyed data: a refresh whose output path is the
/// baseline must be refused with the baseline still byte-identical.
#[test]
fn an_aliased_refresh_is_refused_before_the_baseline_can_be_overwritten() {
    let dir = scratch("refresh");
    let baseline = seed_baseline(&dir);
    let path = baseline.display().to_string();

    let (code, stderr) = run(&[
        "--refresh-baseline",
        "--baseline",
        &path,
        "--current-output",
        &path,
        "--suite-filter",
        "no-such-matchup",
    ]);

    assert_ne!(
        code,
        Some(0),
        "an aliased refresh must fail; stderr:\n{stderr}"
    );
    assert_eq!(
        std::fs::read_to_string(&baseline).expect("read baseline"),
        SENTINEL,
        "the baseline was modified by a run that was refused"
    );
    // LOAD-BEARING — see the module note. Measured: with the alias check deleted, or with
    // `same_file` forced to false, the two assertions above still pass, because the binary then
    // fails at the card database with the baseline equally untouched. This assertion is the only
    // one of the three that dies to those mutants.
    assert!(
        stderr.contains("same file"),
        "the refusal must be about aliasing, not something later; stderr:\n{stderr}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The same aliasing on the COMPARE path, which is the quieter half: there it made the gate read
/// back the run that had just overwritten the baseline and compare it to itself, reporting no
/// drift and exiting 0. Measured on the pre-fix binary with a baseline recording p0 at 100%
/// against a run that scored 0%: `0% | 0%`, zero flips, `0 FAIL`, exit 0.
#[test]
fn an_aliased_comparison_is_refused_before_the_baseline_can_be_overwritten() {
    let dir = scratch("compare");
    let baseline = seed_baseline(&dir);
    let path = baseline.display().to_string();

    let (code, stderr) = run(&["--baseline", &path, "--current-output", &path]);

    assert_ne!(
        code,
        Some(0),
        "an aliased comparison must fail; stderr:\n{stderr}"
    );
    assert_eq!(
        std::fs::read_to_string(&baseline).expect("read baseline"),
        SENTINEL,
        "the baseline was modified by a run that was refused"
    );
    // LOAD-BEARING — see the module note. Measured: with the alias check deleted, or with
    // `same_file` forced to false, the two assertions above still pass, because the binary then
    // fails at the card database with the baseline equally untouched. This assertion is the only
    // one of the three that dies to those mutants.
    assert!(
        stderr.contains("same file"),
        "the refusal must be about aliasing, not something later; stderr:\n{stderr}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Control arm. Every assertion above is satisfied by a binary that refuses everything, which
/// would break the gate far worse than the bug being fixed. Distinct paths must get past the
/// argument check — proven by the failure being about something LATER in the run (the card
/// database or the suite), never about aliasing.
#[test]
fn distinct_paths_are_not_treated_as_aliases() {
    let dir = scratch("distinct");
    let baseline = seed_baseline(&dir);
    let current = dir.join("current.json");

    let (_code, stderr) = run(&[
        "--baseline",
        &baseline.display().to_string(),
        "--current-output",
        &current.display().to_string(),
        "--data-root",
        &dir.join("no-such-data-root").display().to_string(),
    ]);

    assert!(
        !stderr.contains("same file"),
        "distinct paths must not be rejected as aliases; stderr:\n{stderr}"
    );
    // PREMISE: the run really did proceed past the argument check, so the assertion above is
    // about aliasing rather than about the process dying even earlier for some other reason.
    assert!(
        stderr.contains("failed to load card database"),
        "expected the run to proceed to the card database; stderr:\n{stderr}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
