//! The gate's process-level contract: exit status and stdout body, together.
//!
//! Review found that every test for this pairing stopped at the library call, while the two
//! statements that actually matter — print the body, exit with the code — lived in a binary no
//! test executed. A `main` that printed the refusal to stderr, or exited 0 on it, would revert
//! the fix with the whole unit suite green.
//!
//! `.github/workflows/ai-gate.yml` redirects the gate's **stdout** into a file, posts that file
//! as a drift issue only when the step **failed**, and aborts when the file is empty. Those two
//! conditions are one contract: satisfying either alone posts nothing. So these tests assert
//! both on the same invocation rather than in separate cases.
//!
//! `ai-duel compare` is the binary under test because it is the only one of the three sharing
//! `emit_gate_verdict` that needs no card database and plays no games — it reads two report
//! files and prints a verdict. That makes this a millisecond test instead of a full suite run,
//! and it exercises the same shared emitter `ai-gate` and `ai-perf-gate` end in.

use std::process::Command;

use phase_ai::duel_suite::run::{GameResult, MatchupResult, SuiteReport, SuiteStatus};
use phase_ai::duel_suite::Expected;

/// Build the fixture from the real types rather than hand-written JSON.
///
/// The first draft of this test hand-wrote the report and got `Expected`'s encoding wrong — it
/// is an internally tagged enum — so both arms failed on a parse error instead of on the
/// contract under test. Serialising the actual structs cannot drift from the schema: a field
/// added to `SuiteReport` breaks compilation here rather than silently producing a fixture the
/// binary rejects, and the parse is exercised by the binary, not asserted by the test.
///
/// `games_per_matchup` is the workload knob; everything else is held equal so the refusal in
/// the first test can only come from that field.
fn report_json(games_per_matchup: usize) -> String {
    let report = SuiteReport {
        schema_version: 2,
        git_sha: None,
        card_data_hash: None,
        unix_timestamp_secs: 0,
        difficulty: "Medium".to_string(),
        games_per_matchup,
        base_seed: 7,
        results: vec![MatchupResult {
            matchup_id: "red-mirror".to_string(),
            exercises: Vec::new(),
            p0_label: "a".to_string(),
            p1_label: "b".to_string(),
            expected: Expected::Mirror { tolerance: 0.4 },
            p0_wins: 1,
            p1_wins: 0,
            draws: 0,
            games: vec![GameResult {
                seed: 1,
                winner: Some(0),
                turns: 10,
            }],
            total_turns: 10,
            total_duration_ms: 1,
            avg_turns: 10.0,
            avg_duration_ms: 1.0,
            status: SuiteStatus::Pass,
            fail_reason: None,
            attribution: None,
        }],
    };
    serde_json::to_string_pretty(&report).expect("serialize fixture")
}

fn write(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write fixture");
    path
}

fn run(baseline: &std::path::Path, current: &std::path::Path) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ai-duel"))
        .args([
            "compare",
            &baseline.display().to_string(),
            &current.display().to_string(),
        ])
        .output()
        .expect("spawn ai-duel");
    (
        out.status.code().expect("exit code"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn tempdir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("phase-gate-cli-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create tempdir");
    dir
}

/// The refusal route, asserted as the PAIR the workflow needs. A non-zero exit with an empty
/// stdout aborts the publishing step ("failed without a drift report"); a populated stdout with
/// a zero exit never reaches it. Both, or the drift issue does not exist.
#[test]
fn a_refused_comparison_exits_nonzero_and_writes_its_reason_to_stdout() {
    let dir = tempdir("refuse");
    let baseline = write(&dir, "baseline.json", &report_json(10));
    let current = write(&dir, "current.json", &report_json(100));

    let (code, stdout, stderr) = run(&baseline, &current);

    assert_ne!(
        code, 0,
        "a refusal must fail the step; stdout was:\n{stdout}"
    );
    assert!(
        !stdout.trim().is_empty(),
        "an empty report body aborts the publishing step; stderr was:\n{stderr}"
    );
    // The body must be the refusal, not merely non-empty — a table of PASSing rows with no
    // statement of what failed is the outcome this whole change exists to prevent.
    assert!(stdout.contains("comparison refused"), "stdout:\n{stdout}");
    assert!(stdout.contains("games_per_matchup"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("10") && stdout.contains("100"),
        "stdout:\n{stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A report that cannot be READ must refuse on the same terms as one that cannot be COMPARED.
///
/// Review found these two arms returned 2 after an `eprintln!` alone, so the workflow's redirected
/// stdout stayed empty and its "failed without a drift report" abort fired instead of the refusal
/// being posted. Both inputs are covered because they are separate arms in the source — a fix
/// applied to one and not the other is exactly the shape of defect this file exists to catch.
///
/// Missing and malformed are both exercised because they take different `CompareError` variants
/// (`Io` vs `Parse`) to the same renderer, and a remedy keyed on only one of them would leave the
/// other with an empty body.
#[test]
fn an_unreadable_report_still_publishes_a_refusal_body() {
    for (case, make_bad) in [("missing", false), ("malformed", true)] {
        for bad_side in ["baseline", "current"] {
            let dir = tempdir(&format!("unreadable-{case}-{bad_side}"));
            let good = write(&dir, "good.json", &report_json(10));
            let bad = dir.join(format!("{bad_side}-bad.json"));
            if make_bad {
                std::fs::write(&bad, "{ this is not a suite report").expect("write malformed");
            }
            // PREMISE: the "missing" case really is missing, or it would be testing nothing.
            assert_eq!(bad.exists(), make_bad, "fixture for {case}/{bad_side}");

            let (baseline, current) = if bad_side == "baseline" {
                (bad.clone(), good.clone())
            } else {
                (good.clone(), bad.clone())
            };
            let (code, stdout, stderr) = run(&baseline, &current);

            assert_eq!(code, 2, "{case}/{bad_side} must exit 2; stderr:\n{stderr}");
            assert!(
                stdout.contains("Gate: comparison refused"),
                "{case}/{bad_side} must publish a refusal body on STDOUT, not stderr; \
                 stdout was {} bytes:\n{stdout}",
                stdout.len()
            );
            // The body must say more than the header — an envelope with no remedy is the same
            // empty-file problem wearing a title.
            assert!(
                stdout.contains("could not be read"),
                "{case}/{bad_side} body must carry the remedy; stdout:\n{stdout}"
            );
            // The side is what makes it actionable, and it lives on stderr by design.
            assert!(
                stderr.contains(bad_side),
                "{case}/{bad_side} stderr must name which report failed; stderr:\n{stderr}"
            );
            std::fs::remove_dir_all(&dir).ok();
        }
    }
}

/// Control arm. Without it every assertion above is satisfied by a binary that refuses
/// everything, which would be a worse regression than the one being fixed.
#[test]
fn a_comparable_pair_exits_zero_and_writes_a_table() {
    let dir = tempdir("accept");
    let baseline = write(&dir, "baseline.json", &report_json(10));
    let current = write(&dir, "current.json", &report_json(10));

    let (code, stdout, stderr) = run(&baseline, &current);

    assert_eq!(
        code, 0,
        "identical reports must compare clean; stderr:\n{stderr}"
    );
    assert!(stdout.contains("| red-mirror |"), "stdout:\n{stdout}");
    assert!(stdout.contains("compare: 0 FAIL"), "stdout:\n{stdout}");
    assert!(
        !stdout.contains("comparison refused"),
        "control arm must not refuse; stdout:\n{stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}
