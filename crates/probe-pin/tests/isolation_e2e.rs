//! Tier 2 — §10 rows 1, 2, 14, 21 and 32, plus the two arms of rows 26 and 33 that need a
//! real pipeline run. Everything here shells `unshare --map-root-user --mount`.
//!
//! The dogfood manifests pin `[target].test = "pure_logic"`, NEVER `isolation_e2e` — this
//! file is what runs probe-pin, so pinning it would recurse.
//!
//! Rows 1, 2 and 21 execute their own DROP arms: each is defined as a mutation of the
//! isolation SCRIPT (drop the readback / compare the mutant with itself / copy instead of
//! mount), so the test derives the mutant script and runs it through the same entry point.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use probe_pin::isolate::{self, Run};
use probe_pin::manifest::{FilterMatch, Manifest, Target};
use probe_pin::mutate::Mount;
use probe_pin::verdict::{self, Verdict};
use probe_pin::{Abort, HarnessMarker};

const PROD: &str = "crates/probe-pin/tests/fixtures/prod.txt";
/// Row 21 arm 2 WRITES this path. It is gitignored (`.gitignore:113` = `tmp/`), so the arm
/// cannot leak into the tree even when its assertion panics before a restore — and it is
/// never a tracked fixture a concurrent `nextest --workspace` reader might be reading.
const TMP: &str = "crates/probe-pin/tests/fixtures/tmp";

fn root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| probe_pin::target::workspace_root().expect("workspace root"))
}

/// Resolved once per process: the nested `cargo test --no-run` is the expensive part.
fn testbin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| probe_pin::target::resolve("probe-pin", "pure_logic").expect("test binary"))
}

fn target(filter: &str, secs: u64, env: &[(&str, &str)]) -> Target {
    let mut t = Manifest::load(&root().join("crates/probe-pin/tests/fixtures/dogfood.toml"))
        .unwrap()
        .target;
    t.filter = filter.to_string();
    t.filter_match = FilterMatch::Substring;
    t.timeout_secs = secs;
    for (k, v) in env {
        t.env.insert((*k).to_string(), (*v).to_string());
    }
    t
}

fn mutant(dir: &Path, name: &str, text: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, text).unwrap();
    p
}

/// The mutant that makes `ppfixture_census` fail: one of the two marker lines is gone.
fn dropped_site() -> String {
    std::fs::read_to_string(root().join(PROD))
        .unwrap()
        .replace("SITE-TWO marker beta\n", "")
}

/// The shipping SCRIPT minus the lines whose leading token matches — how rows 1, 2 and 21
/// build their DROP mutants.
fn script_without(tokens: &[&str]) -> String {
    isolate::SCRIPT
        .lines()
        .filter(|l| !tokens.iter().any(|t| l.trim_start().starts_with(t)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sha_of(rel: &str) -> String {
    probe_pin::sha256_hex(&std::fs::read(root().join(rel)).unwrap())
}

/// Every probe-pin child shells its own NESTED `cargo test --test pure_logic --no-run` into the
/// shared `CARGO_TARGET_DIR`; the three §7 wiring arms take the count of such children from four
/// to seven, so they run one at a time rather than contending on cargo's build lock. This is
/// hygiene, NOT MAJOR-3's fix: that flake's mechanism was isolated to the mutation harness (a
/// `cp -p` restore preserves the pre-mutation mtime, so cargo keeps the artifact built from the
/// mutant and the NEXT run measures the PREVIOUS mutant — reproduced deterministically, and
/// cleared by one `touch`). Poisoning is ignored on purpose: a panicking arm must report ITS OWN
/// failure rather than turn every later arm into a poison error that hides it.
fn probe_pin_bin(args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_probe-pin"));
    cmd.current_dir(root()).args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("probe-pin runs");
    (
        isolate::exit_rc(&out.status),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// ------------------------------------------------------------------ row 1

/// Row 1: the mutant is REACHED at the target's path inside the namespace, and the `cmp`
/// readback is what proves it. Arm A is the pinned behaviour; arm B removes the mount and the
/// readback catches it; arm C removes the readback too and the same unmounted run reports a
/// clean PASS — which is the whole reason a `pass` verdict is only worth its readback.
#[test]
fn mount_reach() {
    let dir = tempfile::tempdir().unwrap();
    let mounts = vec![Mount {
        mutant: mutant(dir.path(), "prod.txt", &dropped_site()),
        target: root().join(PROD),
    }];
    let t = target("ppfixture_census", 60, &[]);
    let before = sha_of(PROD);

    // arm A — real mount
    let run = isolate::run(testbin(), &mounts, &t).unwrap();
    assert_eq!(run.rc, 101, "the mutant must be visible to the target");
    let obs = verdict::observe("P1", &run, &t, &mounts).expect("no instrument failure");
    assert!(verdict::all_anchors_in_one(
        &["CENSUS VIOLATED: expected 2 sites, got 1".to_string()],
        &obs.failed_captures
    ));
    assert_eq!(sha_of(PROD), before, "the on-disk file must be untouched");

    // arm B — the mount never happened; the readback catches it, and the abort reported is
    // MountNotReached and NOT the co-firing HarnessIncomplete { SuiteStarted }.
    let run = isolate::run_with_script(&script_without(&["mount --bind"]), testbin(), &mounts, &t)
        .unwrap();
    assert_eq!(run.rc, 97);
    assert!(
        run.stdout.trim().is_empty(),
        "a reached mount emits suite-started first"
    );
    assert!(run.stderr.contains("MOUNT NOT REACHED"));
    let e = verdict::observe("P1", &run, &t, &mounts).unwrap_err();
    assert!(matches!(e, Abort::MountNotReached { .. }), "ordering: {e}");
    assert!(!matches!(
        e,
        Abort::HarnessIncomplete {
            missing: HarnessMarker::SuiteStarted,
            ..
        }
    ));

    // arm C (DROP) — without the readback, the very same unmounted run reports a clean pass
    let run = isolate::run_with_script(
        &script_without(&["mount --bind", "cmp -s"]),
        testbin(),
        &mounts,
        &t,
    )
    .unwrap();
    assert_eq!(run.rc, 0, "DROP arm: no mount, no readback -> a FALSE pass");
    let obs = verdict::observe("P1", &run, &t, &mounts).unwrap();
    assert!(!obs.target_failed);
    assert_eq!(sha_of(PROD), before);
}

// ------------------------------------------------------------------ row 2

/// Row 2: a `Prepend` mutant reaches the target too. The census reports 202 sites under a
/// real 200-line pad, and 2 when the mount is dropped.
#[test]
fn pad_reach() {
    let dir = tempfile::tempdir().unwrap();
    let padded = format!(
        "{}{}",
        "// pad marker\n".repeat(200),
        std::fs::read_to_string(root().join(PROD)).unwrap()
    );
    let mounts = vec![Mount {
        mutant: mutant(dir.path(), "prod.txt", &padded),
        target: root().join(PROD),
    }];
    let t = target("ppfixture_census", 60, &[]);
    let before = sha_of(PROD);

    let run = isolate::run(testbin(), &mounts, &t).unwrap();
    let obs = verdict::observe("P2", &run, &t, &mounts).unwrap();
    assert!(
        verdict::all_anchors_in_one(
            &["CENSUS VIOLATED: expected 2 sites, got 202".to_string()],
            &obs.failed_captures
        ),
        "the 200-line pad must be visible at the target's path"
    );

    // The readback catches an unmounted Prepend mutant too. This arm derives its script from
    // the SHIPPING one, so deleting the `cmp` line from `isolate::SCRIPT` flips it — which is
    // row 2's DROP (M-e: the readback is a single per-file `cmp` with no per-kind branch, so
    // this row exercises row 1's mutation on a Prepend mutant).
    let run = isolate::run_with_script(&script_without(&["mount --bind"]), testbin(), &mounts, &t)
        .unwrap();
    assert_eq!(
        run.rc, 97,
        "an unmounted pad must be caught by the readback"
    );
    assert!(matches!(
        verdict::observe("P2", &run, &t, &mounts).unwrap_err(),
        Abort::MountNotReached { .. }
    ));

    // and with the readback removed as well, the same unmounted run is a silent pass
    let run = isolate::run_with_script(
        &script_without(&["mount --bind", "cmp -s"]),
        testbin(),
        &mounts,
        &t,
    )
    .unwrap();
    assert_eq!(run.rc, 0);
    assert!(
        !verdict::observe("P2", &run, &t, &mounts)
            .unwrap()
            .target_failed
    );
    assert_eq!(sha_of(PROD), before);
}

// ------------------------------------------------------------------ row 14

/// Row 14: a hung target is killed and NAMED. Without `timeout` the run would hang past CI's
/// 240 s hard kill; with `timeout 0` it would never fire. The abort reported is
/// `TargetTimedOut` and not the co-firing `HarnessIncomplete { SuiteFinished }`.
#[test]
fn timeout() {
    let t = target("ppfixture_hang", 3, &[("PP_FIXTURE", "hang")]);
    let run = isolate::run(testbin(), &[], &t).unwrap();
    assert_eq!(run.rc, 124);
    let e = verdict::observe("P1", &run, &t, &[]).unwrap_err();
    assert!(
        matches!(e, Abort::TargetTimedOut { secs: 3, .. }),
        "ordering: {e}"
    );
    assert!(!matches!(e, Abort::HarnessIncomplete { .. }));

    // positive: a fast target under the SAME timeout passes
    let t = target("ppfixture_census", 3, &[]);
    let run = isolate::run(testbin(), &[], &t).unwrap();
    assert_eq!(run.rc, 0);
    assert!(verdict::observe("P1", &run, &t, &[]).is_ok());
}

// ------------------------------------------------------------------ row 21

/// Row 21: probe-pin never writes the files it mutates. TWO arms, because a one-armed row
/// reports the same outcome with and without the sha compare — it contains no phenomenon.
///
/// Arm 2's write goes to a GITIGNORED path AND the fingerprint set names that path: the two
/// move together, because `TreeMutated` only fires on a file in the fingerprint set, and that
/// set IS the manifest's mutated-file list. Relocating the write alone would stop the row
/// firing and silently re-vacuate it.
#[test]
fn tree() {
    let dir = tempfile::tempdir().unwrap();
    let t = target("ppfixture_census", 60, &[]);

    // arm 1 — a bind mount leaves the on-disk bytes alone
    let files = vec![PROD.to_string()];
    let mounts = vec![Mount {
        mutant: mutant(dir.path(), "prod.txt", &dropped_site()),
        target: root().join(PROD),
    }];
    let before = isolate::fingerprint(root(), &files).unwrap();
    isolate::run(testbin(), &mounts, &t).unwrap();
    let after = isolate::fingerprint(root(), &files).unwrap();
    assert_eq!(before, after);
    assert_eq!(isolate::verify_fingerprint(&before, &after), Ok(()));

    // arm 2 — a test-only "materialize" (copy over the real path instead of mounting it)
    std::fs::create_dir_all(root().join(TMP)).unwrap();
    let rel = format!("{TMP}/prod.txt");
    std::fs::write(
        root().join(&rel),
        "SITE-ONE marker alpha\nSITE-TWO marker beta\n",
    )
    .unwrap();
    let files = vec![rel.clone()];
    let mounts = vec![Mount {
        mutant: mutant(dir.path(), "arm2.txt", &dropped_site()),
        target: root().join(&rel),
    }];
    let before = isolate::fingerprint(root(), &files).unwrap();
    isolate::run_with_script(
        &isolate::SCRIPT.replace("mount --bind", "cp"),
        testbin(),
        &mounts,
        &t,
    )
    .unwrap();
    let after = isolate::fingerprint(root(), &files).unwrap();
    assert_ne!(
        before, after,
        "the copy must actually have written the tree"
    );
    let e = isolate::verify_fingerprint(&before, &after).unwrap_err();
    assert!(matches!(e, Abort::TreeMutated { .. }), "{e}");
    std::fs::remove_file(root().join(&rel)).ok();
}

// ------------------------------------------------------------------ row 32

/// Row 32: `check` catches drift in BOTH directions, and the exit codes discriminate — 1 for
/// drift or a verdict mismatch, 2 for rot (the pinned code moved so far the probe can no
/// longer be applied).
#[test]
fn drift() {
    let dir = root().join(TMP);
    std::fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("drift.toml");
    let block_file = dir.join("drift_block.md");
    let manifest_arg = manifest
        .strip_prefix(root())
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let base = std::fs::read_to_string(root().join("crates/probe-pin/tests/fixtures/dogfood.toml"))
        .unwrap()
        .replace(
            "crates/probe-pin/tests/fixtures/dogfood_block.md",
            "crates/probe-pin/tests/fixtures/tmp/drift_block.md",
        );
    std::fs::write(&manifest, &base).unwrap();
    std::fs::write(
        &block_file,
        "prose above\n// PROBE-PIN:BEGIN placeholder\n// PROBE-PIN:END\nprose below\n",
    )
    .unwrap();

    // (c) write, then an untouched check -> 0
    assert_eq!(probe_pin_bin(&["run", "--write", &manifest_arg], &[]).0, 0);
    let written = std::fs::read_to_string(&block_file).unwrap();
    assert!(written.starts_with("prose above\n") && written.ends_with("prose below\n"));
    assert_eq!(probe_pin_bin(&["check", &manifest_arg], &[]).0, 0);

    // (a) hand-edit a cell -> 1. The edit is the SAME LENGTH as what it replaces, so a
    // length-only comparison reports the tampered block as clean.
    let edited = written.replace("P1_drop_second_site", "P1_drop_second_SITE");
    assert_eq!(edited.len(), written.len());
    assert_ne!(edited, written);
    std::fs::write(&block_file, edited).unwrap();
    let (rc, err) = probe_pin_bin(&["check", &manifest_arg], &[]);
    assert_eq!(rc, 1, "{err}");
    assert!(err.contains("the generated block is stale"), "{err}");
    std::fs::write(&block_file, &written).unwrap();

    // (b) the verdict flips -> 1. P2's mutation still makes the target fail; the manifest now
    // says it should pass, so the run has a measured verdict that contradicts its expectation.
    let flipped = base.replace(
        "  outcome = \"fail\"\n  anchor  = [\"CENSUS VIOLATED: expected 2 sites, got 202\"]",
        "  outcome = \"pass\"",
    );
    assert_ne!(flipped, base, "the (b) arm's edit must actually apply");
    std::fs::write(&manifest, &flipped).unwrap();
    let (rc, err) = probe_pin_bin(&["run", "--write", &manifest_arg], &[]);
    assert_eq!(rc, 1, "{err}");
    assert!(err.contains("VERDICT MISMATCH"), "{err}");
    assert_eq!(
        std::fs::read_to_string(&block_file).unwrap(),
        written,
        "a non-zero exit must never splice"
    );

    // (d) the pinned code moved -> the probe cannot be applied -> 2
    std::fs::write(&manifest, base.replace("SITE-TWO marker beta", "SITE-GONE")).unwrap();
    let (rc, err) = probe_pin_bin(&["check", &manifest_arg], &[]);
    assert_eq!(rc, 2, "{err}");
    assert!(err.contains("'find' matched 0 times"), "{err}");

    std::fs::remove_file(&manifest).ok();
    std::fs::remove_file(&block_file).ok();
}

// ------------------------------------------------------------------ rows 26, 33 (binary arms)

/// Row 26's both-paths arm: `run --write` and `check` BOTH abort when a declared projection's
/// tool cannot be run. Neither emits a block with the sentence missing.
#[test]
fn proj_missing_both_paths() {
    let missing = root().join("crates/probe-pin/tests/fixtures/astgrep_does_not_exist");
    let env = [("PROBE_PIN_ASTGREP", missing.to_str().unwrap())];
    for args in [
        vec![
            "run",
            "--write",
            "crates/probe-pin/tests/fixtures/projection.toml",
        ],
        vec!["check", "crates/probe-pin/tests/fixtures/projection.toml"],
    ] {
        let (rc, err) = probe_pin_bin(&args, &env);
        assert_eq!(rc, 2, "{err}");
        assert!(
            err.contains("will not emit a block with its projection sentences missing"),
            "{err}"
        );
    }
}

/// Row 33's ordering arm: the `test_count > 0` floor is evaluated on the CONTROL first, so a
/// rotted filter aborts naming `P0_control` and no probe is ever run. Under the trivialized
/// ordering (check every probe but not the control) the run still exits 2, but the control has
/// already been certified on zero tests and the abort names the wrong probe.
#[test]
fn no_tests_selected_ordering() {
    let dir = root().join(TMP);
    std::fs::create_dir_all(&dir).unwrap();
    let manifest = dir.join("rotted.toml");
    std::fs::write(
        &manifest,
        std::fs::read_to_string(root().join("crates/probe-pin/tests/fixtures/dogfood.toml"))
            .unwrap()
            .replace(
                "filter       = \"ppfixture_census\"",
                "filter       = \"ppfixture_census_OLDNAME\"",
            ),
    )
    .unwrap();
    let arg = manifest
        .strip_prefix(root())
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // MAJOR-3 (impl review): this arm flaked ONCE in ~63 suite runs with text byte-identical to
    // row 33's own DROP signature (the `test_count` floor deleted), so a bare `assert_eq!(rc, 2)`
    // reports "the floor regressed" and "a one-off" in the SAME bytes. The one-off's mechanism
    // has since been reproduced — a mutation harness restoring with `cp -p` leaves cargo holding
    // the artifact built from the previous mutant — but the COLLISION is repaired here, because
    // any future one-off collides the same way: on failure the arm re-measures and names which
    // one it is. It never goes green on a one-off; it reports as one.
    let (rc, err) = probe_pin_bin(&["run", &arg], &[]);
    if rc != 2 {
        let again: Vec<i32> = (0..2)
            .map(|_| probe_pin_bin(&["run", &arg], &[]).0)
            .collect();
        let which = if again.iter().all(|r| *r != 2) {
            "REGRESSION (reproduced 3/3): the test_count floor is not firing on the control"
        } else {
            "FLAKE (1 of 3): the re-runs aborted on the control as specified — MAJOR-3's residual, NOT a floor regression"
        };
        panic!("probe-pin: {which}. exit: first={rc}, re-runs={again:?}, expected 2\n{err}");
    }
    assert!(err.contains("P0_control selected ZERO tests"), "{err}");
    assert!(err.contains("INSTRUMENT FAILURE"), "{err}");
    assert!(
        !err.contains("P1_drop_second_site") && !err.contains("P2_pad"),
        "the pipeline must stop at the control: {err}"
    );
    std::fs::remove_file(&manifest).ok();
}

// ------------------------------------------------ §7 wiring: steps 7, 9 and 10 (MAJOR-2)
//
// Rows 10, 21 and §8's control-failed row pin the FUNCTIONS. These three arms pin the CALLS:
// each drives the real binary end-to-end on a manifest that reaches only that step, and each
// was revert-probed by deleting its step from `main.rs` (measured: rc 2 -> rc 0, all three).
// No production code participates in them beyond the pipeline itself.

/// §7 step 9 is CALLED. `manifest::validate` compares anchor LISTS to each other, so P1's list
/// being a SUBSET of P2's captured failure is invisible to it; only the all-pairs scan over the
/// captures sees it, and that scan runs at exactly one call site.
#[test]
fn wiring_discriminate() {
    let (rc, err) = probe_pin_bin(
        &["run", "crates/probe-pin/tests/fixtures/collide.toml"],
        &[],
    );
    assert_eq!(rc, 2, "{err}");
    assert!(
        err.contains("These two probes are not distinguished"),
        "{err}"
    );
    assert!(
        err.contains("P1_drop_second_site") && err.contains("P2_pad_reaches_target"),
        "{err}"
    );
}

/// §7 step 10 is CALLED — invariant (a)'s runtime enforcement. The manifest's target writes an
/// unmounted path during the CONTROL run, which is a real write to the measured tree.
#[test]
fn wiring_verify_fingerprint() {
    std::fs::create_dir_all(root().join(TMP)).unwrap();
    let file = root().join(TMP).join("tree_write.txt");
    std::fs::write(&file, "ORIGINAL\n").unwrap();

    let (rc, err) = probe_pin_bin(
        &["run", "crates/probe-pin/tests/fixtures/treewrite.toml"],
        &[],
    );
    // reach guard: without the write, the step has nothing to detect and the arm is vacuous
    assert_ne!(
        std::fs::read_to_string(&file).unwrap(),
        "ORIGINAL\n",
        "the target must actually have written the tree: {err}"
    );
    assert_eq!(rc, 2, "{err}");
    assert!(err.contains("TREE MUTATED"), "{err}");
    assert!(err.contains("tree_write.txt"), "{err}");
    std::fs::remove_file(&file).ok();
}

/// §7 step 7's `ControlFailed` branch is CALLED: a control that fails on the unmodified tree is
/// a broken instrument, and §8's control-failed row is otherwise unreachable.
#[test]
fn wiring_control_failed() {
    let (rc, err) = probe_pin_bin(
        &["run", "crates/probe-pin/tests/fixtures/control_fails.toml"],
        &[],
    );
    assert_eq!(rc, 2, "{err}");
    assert!(
        err.contains("the control probe 'P0_control' (no mounts, unmodified tree) FAILED"),
        "{err}"
    );
    assert!(
        err.contains("every other verdict in this run is meaningless"),
        "{err}"
    );
}

/// The dogfood: probe-pin runs its own committed manifest against its own committed block.
/// This is the same invocation the Tilt `probe-pin-check` resource makes.
#[test]
fn dogfood_check() {
    let (rc, err) = probe_pin_bin(
        &["check", "crates/probe-pin/tests/fixtures/dogfood.toml"],
        &[],
    );
    assert_eq!(rc, 0, "{err}");
}

/// The isolation model itself: a run under `unshare` never sees the ambient environment, and
/// `Run`'s streams are never merged.
#[test]
fn streams_are_separate() {
    let t = target("ppfixture_census", 60, &[]);
    let run: Run = isolate::run(testbin(), &[], &t).unwrap();
    assert!(
        run.stdout.lines().next().unwrap().contains("\"suite\""),
        "stdout must be a pure record stream: {:?}",
        run.stdout.lines().next()
    );
    assert!(
        run.stderr.is_empty(),
        "streams are never merged: {:?}",
        run.stderr
    );
    assert_eq!(
        verdict::verdict(
            &Manifest::load(&root().join("crates/probe-pin/tests/fixtures/dogfood.toml"))
                .unwrap()
                .probes[0],
            &verdict::observe("P0_control", &run, &t, &[]).unwrap(),
            0
        ),
        Ok(Verdict::Pass { mounts_reached: 0 })
    );
}
