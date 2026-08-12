//! Tier 1 — §10 rows 3-13, 15-20, 22-31 and 33's non-ordering arms. Zero namespace
//! dependency: nothing here runs `unshare`.
//!
//! The `ppfixture_*` tests at the bottom are not assertions about probe-pin — they are the
//! TARGETS the Tier-2 dogfood manifests and the child-process arms below drive. They are
//! inert (or trivially passing) unless `PP_FIXTURE` selects them, so a plain
//! `cargo test --test pure_logic` runs them as no-ops.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use probe_pin::block::{self, Outcome, ProjectionCount};
use probe_pin::isolate::{self, Run};
use probe_pin::manifest::{self, Manifest, Probe, Target};
use probe_pin::mutate;
use probe_pin::project;
use probe_pin::target;
use probe_pin::verdict::{self, Verdict};
use probe_pin::{Abort, CheckOutcome, DriftCause, HarnessMarker, Instrument, InstrumentChange};

// ------------------------------------------------------------------ helpers

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(name: &str) -> Manifest {
    Manifest::load(&fixtures().join(name)).expect("fixture manifest parses")
}

fn abort_of(e: anyhow::Error) -> Abort {
    e.downcast::<Abort>().expect("error is an Abort")
}

/// A `[target]` good enough to classify a synthetic record stream against.
fn target() -> Target {
    load("dogfood.toml").target
}

fn run_of(rc: i32, stdout: &str) -> Run {
    Run {
        rc,
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

/// A libtest JSON stream: suite started, the given test records, suite terminal.
fn stream(test_count: usize, records: &[&str], terminal: &str) -> String {
    let mut s =
        format!("{{\"type\":\"suite\",\"event\":\"started\",\"test_count\":{test_count}}}\n");
    for r in records {
        s.push_str(r);
        s.push('\n');
    }
    s.push_str(&format!(
        "{{\"type\":\"suite\",\"event\":\"{terminal}\",\"passed\":0,\"failed\":1,\"filtered_out\":3}}\n"
    ));
    s
}

fn probe(toml: &str) -> Probe {
    toml::from_str(toml).expect("probe fixture parses")
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

/// Re-exec THIS test binary as a libtest target: the cheapest real record stream there is.
/// `filter` is a SUBSTRING filter, exactly like the shipping argv; `extra` is where a test
/// adds `--exact` or a `[target].args` flag.
fn child(filter: &str, env: &[(&str, &str)], extra: &[&str]) -> Run {
    // Under `timeout`, exactly as the shipping script runs the target. That is not cosmetic:
    // a signal-killed child has no exit CODE at all, and `timeout` is what normalizes SIGABRT
    // to the 134 the pipeline classifies on.
    let mut cmd = Command::new("timeout");
    cmd.args(["-k", "5", "60"])
        .arg(std::env::current_exe().unwrap())
        .args([filter, "--test-threads=1", "--format", "json"])
        .args(["-Z", "unstable-options"])
        .args(extra);
    cmd.env_clear();
    for key in ["PATH", "HOME"] {
        if let Ok(v) = std::env::var(key) {
            cmd.env(key, v);
        }
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("child target runs");
    Run {
        rc: isolate::exit_rc(&out.status),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

// ------------------------------------------------------------------ row 3, 4, 5, 6

/// Row 3: anchors are scoped to ONE `{"type":"test"}` `failed` record — not to the whole
/// stdout, and not to an inferred prose block. DROP (scan the whole stdout) makes the forged
/// anchor hit; TRIVIALIZE (one buffer) makes the cross-record pair hit.
#[test]
fn json_scope() {
    let s = stream(
        3,
        &[
            r#"{"type":"test","name":"c_pass","event":"ok","stdout":"FORGED FROM A PASSING TEST\n"}"#,
            r#"{"type":"test","name":"a_fail","event":"failed","stdout":"REAL ANCHOR ONE\nREAL ANCHOR TWO\n"}"#,
            r#"{"type":"test","name":"b_fail","event":"failed","stdout":"OTHER FAILURE\n"}"#,
        ],
        "failed",
    );
    let obs =
        verdict::observe("P1", &run_of(101, &s), &target(), &[]).expect("no instrument failure");
    let anchors = |v: &[&str]| {
        verdict::all_anchors_in_one(
            &v.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            &obs.failed_captures,
        )
    };

    // hostile: a passing test's own output can never satisfy an anchor
    assert!(!anchors(&["FORGED FROM A PASSING TEST"]));
    // hostile: two anchors that only jointly exist ACROSS two failed records
    assert!(!anchors(&["REAL ANCHOR ONE", "OTHER FAILURE"]));
    // reach guard: the real two-fragment anchor, both fragments in ONE record
    assert!(anchors(&["REAL ANCHOR ONE", "REAL ANCHOR TWO"]));
    assert_eq!(obs.failed_captures.len(), 2);
}

/// Row 4: a `#[should_panic]` test that PASSES is not a failure. libtest reports it
/// `event:"ok"` with no capture, so it is unreachable by any anchor.
#[test]
fn should_panic() {
    // The `event:"started"` records are what a REAL libtest stream carries, and they are what
    // separates "failed" from the trivialized "every record whose event != ok".
    let s = stream(
        2,
        &[
            r#"{"type":"test","event":"started","name":"d_should_panic_passes"}"#,
            r#"{"type":"test","name":"d_should_panic_passes","event":"ok"}"#,
            r#"{"type":"test","event":"started","name":"a_fail"}"#,
            r#"{"type":"test","name":"a_fail","event":"failed","stdout":"REAL\n"}"#,
        ],
        "failed",
    );
    let obs = verdict::observe("P1", &run_of(101, &s), &target(), &[]).unwrap();
    assert_eq!(obs.failed_captures, vec!["REAL\n".to_string()]);
    assert!(!obs
        .failed_captures
        .iter()
        .any(|c| c.contains("should_panic")));
}

/// Row 5: a panic whose MESSAGE contains a `thread '…' panicked at` line stays ONE record.
/// The deleted prose block-splitter would have cut it in two and produced a false refutation.
#[test]
fn nested_thread_line() {
    let inner = "thread 'inner' (1) panicked at src/x.rs:1:1:\\nFRAGMENT TWO";
    let s = stream(
        1,
        &[&format!(
            r#"{{"type":"test","name":"e","event":"failed","stdout":"\nthread 'e' (2) panicked at tests/mount.rs:7:5:\nFRAGMENT ONE {inner}\n"}}"#
        )],
        "failed",
    );
    let obs = verdict::observe("P1", &run_of(101, &s), &target(), &[]).unwrap();
    assert_eq!(obs.failed_captures.len(), 1);
    assert!(verdict::all_anchors_in_one(
        &["FRAGMENT ONE".to_string(), "FRAGMENT TWO".to_string()],
        &obs.failed_captures
    ));
}

/// Row 6: JSON escaping round-trips exactly — quote, backslash, tab, ESC, non-ASCII.
#[test]
fn escaping() {
    let s = stream(
        1,
        &[r#"{"type":"test","name":"f","event":"failed","stdout":"q=\" b=\\ t=\t e=\u001b n=é"}"#],
        "failed",
    );
    let obs = verdict::observe("P1", &run_of(101, &s), &target(), &[]).unwrap();
    assert_eq!(obs.failed_captures[0], "q=\" b=\\ t=\t e=\u{1b} n=é");
}

// ------------------------------------------------------------------ row 7

/// Row 7: every reserved flag is rejected, on the option NAME rather than the raw token —
/// `--test-threads=4`, `--format=terse` and `-Zunstable-options` are the three spellings a
/// raw-token list misclassifies. The four negative controls still validate, so the fix does
/// not narrow the live `args` field.
#[test]
fn reserved_arg() {
    let mut m = load("dogfood.toml");
    for hostile in [
        "--nocapture",
        "--format",
        "--test-threads=4",
        "--format=terse",
        "-Zunstable-options",
        "--exact",
        "--show-output",
        "-q",
    ] {
        m.target.args = vec![hostile.to_string()];
        let e = abort_of(manifest::validate(&m).unwrap_err());
        assert!(
            matches!(e, Abort::ReservedArg { ref arg } if arg == hostile),
            "{hostile}"
        );
    }
    for benign in [
        "--skip=foo",
        "--skip",
        "--ignored",
        "--exclude-should-panic",
    ] {
        m.target.args = vec![benign.to_string()];
        assert!(manifest::validate(&m).is_ok(), "{benign}");
    }
    m.target.args = vec![];
    assert!(manifest::validate(&m).is_ok());

    // Why they are reserved: ONE non-JSON line on stdout breaks the strict parse.
    let mut s = stream(1, &[], "ok");
    s.insert_str(0, "SECOND-LINE-OF-B: got [Site { line: 11492 }]\n");
    let e = verdict::observe("P1", &run_of(0, &s), &target(), &[]).unwrap_err();
    assert!(matches!(e, Abort::HarnessOutputUnparseable { .. }));
}

// ------------------------------------------------------------------ row 8

/// Row 8: the harness must have COMPLETED. Fixture A aborts unconditionally, so no
/// `RUST_MIN_STACK` value rescues it — the suite terminal record is what fires, and dropping
/// the marker requirement would turn rc 134 into a `Verdict::Fail`.
#[test]
fn harness() {
    for stack in ["", "1", "16777216", "268435456"] {
        let env: Vec<(&str, &str)> = if stack.is_empty() {
            vec![("PP_FIXTURE", "abort")]
        } else {
            vec![("PP_FIXTURE", "abort"), ("RUST_MIN_STACK", stack)]
        };
        let run = child("ppfixture_abort", &env, &[]);
        assert_eq!(
            run.rc, 134,
            "RUST_MIN_STACK={stack:?} should not rescue an abort()"
        );
        let e = verdict::observe("P1", &run, &target(), &[]).unwrap_err();
        assert!(
            matches!(
                e,
                Abort::HarnessIncomplete {
                    missing: HarnessMarker::SuiteFinished,
                    rc: 134,
                    ..
                }
            ),
            "RUST_MIN_STACK={stack:?} -> {e}"
        );
    }
    // reach guard: the same target, not aborting, classifies cleanly
    let run = child("ppfixture_abort", &[], &[]);
    assert_eq!(run.rc, 0);
    assert!(verdict::observe("P1", &run, &target(), &[]).is_ok());
}

// ------------------------------------------------------------------ row 9

/// Row 9: no anchor may embed a source line number. Measured on the real corpus: 0 of 10
/// shipping anchors rejected, 7 of 9 hostile spellings rejected, and 0 of 8 plausible
/// legitimate anchors whose text merely ENDS in "line" rejected.
#[test]
fn anchor_lint() {
    let shipping = [
        "PROVENANCE SPLIT VIOLATED",
        "text: \"Ok(TargetPin::Player(*player))\"",
        "text: \"TargetRef::Player(pl) => Some(TargetPin::Player(*pl)),\"",
        "site 2 must remain `shortcut_drive_period`'s MATCH arm",
        "got \"TargetPin::Player(_) => 1,\"",
        "site 3 must remain the CR 701.34a proliferate CONSTRUCTION",
        "got \"Some(crate::analysis::decision_template::TargetPin::Player(*other_seat))\"",
        "got \"Some(crate::analysis::decision_template::TargetPin::Player(*pl))\"",
        "the TARGET-class spelling must be CONSTRUCTED by BOTH producers",
        "(\"engine/src/game/engine.rs\", 1)",
    ];
    let rejected = |xs: &[&str]| {
        xs.iter()
            .filter(|a| manifest::embeds_line_number(a))
            .count()
    };
    assert_eq!(
        rejected(&shipping),
        0,
        "a shipping anchor must never be refused"
    );

    let hostile = [
        "line: 8945",
        "line: 4689",
        "line: 11492",
        "loop_shortcut_seat_pin_census.rs:160:5",
        "got [Site { file: \"engine/src/game/engine.rs\", line: 11490 }]",
        "at line 11492",
        "engine.rs:11492",
        "L11492",                                 // named residual (§12.D row 3)
        "(\"engine/src/game/engine.rs\", 11492)", // named residual: P9's shape, up to the integer
    ];
    assert_eq!(rejected(&hostile), 7);
    assert!(!manifest::embeds_line_number("L11492"));
    assert!(!manifest::embeds_line_number(
        "(\"engine/src/game/engine.rs\", 11492)"
    ));

    // MINOR-E: `\bline`. Words ENDING in "line" followed by a count are legitimate, and this
    // repo's own assertion strings contain them.
    let legit = [
        "baseline: 1",
        "baseline: 0",
        "baseline 3 producers must construct the pin",
        "inline 2 callers remain",
        "the pipeline 4 stages are pinned",
        "deadline: 5 seconds",
        "airline 7",
        "outline: 3 sections",
    ];
    assert_eq!(rejected(&legit), 0);

    // and the lint is wired into validate()
    let mut m = load("dogfood.toml");
    m.probes[1].expect = toml::from_str("outcome='fail'\nanchor=['line: 4689']").unwrap();
    assert!(matches!(
        abort_of(manifest::validate(&m).unwrap_err()),
        Abort::AnchorEmbedsLineNumber { .. }
    ));
}

// ------------------------------------------------------------------ row 10

/// Row 10: all-pairs discrimination over the six archived failing probes. Self-misses 0 is
/// the reach guard; collisions 0 is the claim; r2's one-entry P1 anchor is the hostile
/// sibling that MUST abort, because it also fully matches P2's capture.
#[test]
fn all_pairs() {
    let corpus: [(&str, [&str; 2]); 6] = [
        (
            "P1",
            [
                "PROVENANCE SPLIT VIOLATED",
                "text: \"Ok(TargetPin::Player(*player))\"",
            ],
        ),
        (
            "P2",
            [
                "PROVENANCE SPLIT VIOLATED",
                "text: \"TargetRef::Player(pl) => Some(TargetPin::Player(*pl)),\"",
            ],
        ),
        (
            "P6a",
            [
                "site 2 must remain `shortcut_drive_period`'s MATCH arm",
                "got \"TargetPin::Player(_) => 1,\"",
            ],
        ),
        (
            "P6b",
            [
                "site 3 must remain the CR 701.34a proliferate CONSTRUCTION",
                "got \"Some(crate::analysis::decision_template::TargetPin::Player(*other_seat))\"",
            ],
        ),
        (
            "P7",
            [
                "site 2 must remain `shortcut_drive_period`'s MATCH arm",
                "got \"Some(crate::analysis::decision_template::TargetPin::Player(*pl))\"",
            ],
        ),
        (
            "P9",
            [
                "the TARGET-class spelling must be CONSTRUCTED by BOTH producers",
                "(\"engine/src/game/engine.rs\", 1)",
            ],
        ),
    ];
    let mut probes = Vec::new();
    let mut captures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, anchors) in &corpus {
        probes.push(probe(&format!(
            "id = {id:?}\n[[mutation]]\nkind='replace'\nfile='f'\nfind='a'\nreplace='b'\n[expect]\noutcome='fail'\nanchor={:?}\n",
            anchors
        )));
        captures.insert(
            (*id).to_string(),
            vec![format!(
                "panicked at tests/census.rs:1:1:\n{}\n{}\n",
                anchors[0], anchors[1]
            )],
        );
    }

    // reach guard: every probe's own anchors hit its own capture
    for p in &probes {
        assert!(
            verdict::all_anchors_in_one(p.anchors(), &captures[&p.id]),
            "self-miss on {}",
            p.id
        );
    }
    assert_eq!(verdict::discriminate(&probes, &captures), Ok(()));

    // hostile: r2's one-entry P1 anchor list also matches P2's capture
    let mut hostile = probes.clone();
    hostile[0].expect =
        toml::from_str("outcome='fail'\nanchor=['PROVENANCE SPLIT VIOLATED']").unwrap();
    assert!(matches!(
        verdict::discriminate(&hostile, &captures),
        Err(Abort::AnchorNotDiscriminating { ref probe, ref collides_with }) if probe == "P1" && collides_with == "P2"
    ));
}

// ------------------------------------------------------------------ row 11

/// Row 11: byte-identical anchor lists are caught at VALIDATE time — before any run. The
/// TRIVIALIZE (compare lengths only) would accept two same-length distinct lists.
#[test]
fn anchor_dup() {
    let mut m = load("dogfood.toml");
    let dup: probe_pin::manifest::Expect =
        toml::from_str("outcome='fail'\nanchor=['A','B']").unwrap();
    m.probes[1].expect = dup.clone();
    m.probes[2].expect = dup;
    assert!(matches!(
        abort_of(manifest::validate(&m).unwrap_err()),
        Abort::AnchorNotDiscriminating { .. }
    ));
    // positive: two distinct lists of the SAME LENGTH validate
    m.probes[2].expect = toml::from_str("outcome='fail'\nanchor=['A','C']").unwrap();
    assert!(manifest::validate(&m).is_ok());
}

// ------------------------------------------------------------------ row 12

/// Row 12: the target's environment is CONSTRUCTED, not inherited.
///
/// The claim is about what a PROCESS's ambient environment does, so the measurement needs a
/// process whose ambient environment the test owns — mutating this one would race its parallel
/// siblings. `ppfixture_envprobe` is that process: it calls the shipping `apply_child_env` and
/// writes the resulting capture length to `PP_OUT`. Both a DROP (no clear) and a TRIVIALIZE
/// (clear, then re-import everything) leak the ambient value into the innermost target.
#[test]
fn env_hermetic() {
    let dir = tempfile::tempdir().unwrap();
    let measure = |name: &str, ambient: &[(&str, &str)]| -> usize {
        let out = dir.path().join(name);
        let out_s = out.to_str().unwrap().to_string();
        let mut env: Vec<(&str, &str)> = vec![("PP_FIXTURE", "envprobe"), ("PP_OUT", &out_s)];
        env.extend_from_slice(ambient);
        let run = child("ppfixture_envprobe", &env, &[]);
        assert_eq!(run.rc, 0, "envprobe: {}", run.stdout);
        std::fs::read_to_string(&out)
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    };

    let clean = measure("clean", &[]);
    assert!(
        clean > 0,
        "the reach guard: a panic capture must be non-empty"
    );

    // an exported RUST_BACKTRACE is the developer-shell hazard: it changes the captured bytes,
    // and the CLEAR is the only thing stopping it — `apply_child_env` deliberately does not
    // re-set it to "0", which would make this arm pass with or without the clear (MINOR-5)
    assert_eq!(
        measure("backtrace", &[("RUST_BACKTRACE", "1")]),
        clean,
        "an ambient RUST_BACKTRACE must not reach the target"
    );
    // and RUST_TEST_NOCAPTURE is the severe one: libtest then drops the `stdout` field from
    // every failed record, so every anchor in every probe silently stops matching
    assert_eq!(
        measure("nocapture", &[("RUST_TEST_NOCAPTURE", "1")]),
        clean,
        "an ambient RUST_TEST_NOCAPTURE must not reach the target"
    );
    // the ALLOWLIST, not the clear, is what decides: [target].env turns it back on
    assert!(
        measure("forced", &[("PP_FORCE_BACKTRACE", "1")]) > clean,
        "[target].env must still be able to set RUST_BACKTRACE"
    );
}

// ------------------------------------------------------------------ row 13

/// Row 13: `[target].env` defaults to `RUST_MIN_STACK = "16777216"`, and that value is
/// load-bearing. Fixture B is RESCUABLE — unlike row 8's fixture, which no value rescues —
/// so `"1"` (the TRIVIALIZE: a default that exists and is useless) is measurably different
/// from the shipping default.
#[test]
fn env_default() {
    let t: Target = toml::from_str(
        "mode='runtime-read'\npackage='p'\ntest='t'\nfilter='f'\nfilter_match='substring'",
    )
    .unwrap();
    assert_eq!(
        t.env.get("RUST_MIN_STACK").map(String::as_str),
        Some("16777216")
    );
    assert_eq!(t.timeout_secs, 300);
    assert!(t.args.is_empty());

    let rc = |stack: Option<&str>| {
        let env: Vec<(&str, &str)> = match stack {
            Some(v) => vec![("PP_FIXTURE", "stack"), ("RUST_MIN_STACK", v)],
            None => vec![("PP_FIXTURE", "stack")],
        };
        child("ppfixture_stack", &env, &[]).rc
    };
    assert_eq!(rc(None), 134, "unset must overflow");
    assert_eq!(rc(Some("1")), 134, "a useless default must overflow");
    assert_eq!(
        rc(Some("16777216")),
        0,
        "the shipping default must rescue it"
    );
}

/// Schema adequacy (§6 / §11 gate 6), MEASURED rather than asserted: every shipped manifest
/// parses under `deny_unknown_fields` and passes validation. `p7.toml` is the archived P7
/// probe verbatim in shape — two sequential `Replace`s on one file whose second `find` exists
/// only in the first's output, two `assert_count`s, a two-entry line-free anchor — and `p8`
/// is the 5-file × 200 `Prepend`. Nothing in the corpus was reshaped to fit the schema.
#[test]
fn schema_adequacy() {
    for name in [
        "collide.toml",
        "control_fails.toml",
        "dogfood.toml",
        "p7.toml",
        "p8.toml",
        "projection.toml",
        "treewrite.toml",
        "unsorted_ids.toml",
    ] {
        let m = load(name);
        manifest::validate(&m).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    let p7 = load("p7.toml");
    let seq = &p7.probes[1];
    assert_eq!(seq.mutations.len(), 2);
    assert_eq!(seq.touched().len(), 1, "both ops target ONE file");
    assert_eq!(seq.assert_counts.len(), 1);
    assert_eq!(seq.anchors().len(), 2);
    match &load("p8.toml").probes[1].mutations[0] {
        probe_pin::manifest::Mutation::Prepend { files, repeat, .. } => {
            assert_eq!((files.len(), *repeat), (5, 200));
        }
        m => panic!("P8 must be a Prepend: {m:?}"),
    }
}

// ------------------------------------------------------------------ row 15

/// Row 15: `mode = "compiled"` PARSES and is rejected, visibly, with a deferral message.
#[test]
fn compiled_rejected() {
    let m = load("compiled.toml");
    let e = abort_of(manifest::validate(&m).unwrap_err());
    assert!(matches!(e, Abort::UnsupportedMode { .. }));
    assert!(e.to_string().contains("deferred, not removed"));

    // positive: runtime-read validates
    assert!(manifest::validate(&load("dogfood.toml")).is_ok());
    // hostile: a TYPO is a different failure — an unknown variant, at parse time
    let e = toml::from_str::<Manifest>(
        &std::fs::read_to_string(fixtures().join("compiled.toml"))
            .unwrap()
            .replace("\"compiled\"", "\"cmpiled\""),
    )
    .unwrap_err();
    assert!(e.to_string().contains("unknown variant"), "{e}");
}

// ------------------------------------------------------------------ row 16

/// Row 16: a `--no-run --message-format=json` stream carries MORE than one `executable` —
/// the package's `bin` reports one too, and it comes FIRST. Taking the first picks the bin.
#[test]
fn resolve_bin() {
    let s = concat!(
        r#"{"reason":"compiler-artifact","executable":"/t/debug/r4probe","target":{"kind":["bin"],"name":"r4probe"}}"#,
        "\n",
        r#"{"reason":"compiler-artifact","executable":"/t/debug/deps/blocks-47ca","target":{"kind":["test"],"name":"blocks"}}"#,
        "\n",
        r#"{"reason":"build-finished","success":true}"#,
    );
    assert_eq!(
        target::pick_executable(s, "p", "blocks").unwrap(),
        PathBuf::from("/t/debug/deps/blocks-47ca")
    );
    // hostile: zero test artifacts -> named candidates, never a silent first-match. The
    // REPORTED count is the number that MATCHED, not the number of candidates (MINOR-1): a
    // user with a typo'd [target].test was being told two test binaries were found and sent
    // hunting for a duplicate that does not exist.
    let e = target::pick_executable(s, "p", "absent").unwrap_err();
    assert!(
        matches!(e, Abort::TargetBinaryUnresolved { found: 0, ref candidates, .. } if candidates.len() == 2),
        "{e:?}"
    );
    assert!(e.to_string().contains("found 0. Candidates: ["), "{e}");
}

// ------------------------------------------------------------------ row 17

/// Row 17: `find` must occur EXACTLY once, against the RUNNING text — so a two-op sequence
/// whose second `find` only exists after the first op is expressible, and an ambiguous or
/// rotted `find` is refused.
#[test]
fn find_gate() {
    let dir = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    write(dir.path(), "f.txt", "alpha\nbeta\nalpha\n");

    let mk = |ops: &str| probe(&format!("id='P'\n{ops}\n[expect]\noutcome='pass'\n"));
    let op = |find: &str, replace: &str| {
        format!("[[mutation]]\nkind='replace'\nfile='f.txt'\nfind={find:?}\nreplace={replace:?}")
    };

    let e =
        abort_of(mutate::apply(&mk(&op("gamma", "x")), dir.path(), scratch.path()).unwrap_err());
    assert!(
        matches!(
            e,
            Abort::FindCount {
                found: 0,
                index: 0,
                ..
            }
        ),
        "{e}"
    );
    let e =
        abort_of(mutate::apply(&mk(&op("alpha", "x")), dir.path(), scratch.path()).unwrap_err());
    assert!(matches!(e, Abort::FindCount { found: 2, .. }), "{e}");

    // running text: op 2's `find` exists ONLY after op 1
    let seq = format!("{}\n{}", op("beta", "delta"), op("delta", "epsilon"));
    assert!(mutate::apply(&mk(&seq), dir.path(), scratch.path()).is_ok());
    // and the same op 2 alone is a rotted probe
    let e = abort_of(
        mutate::apply(&mk(&op("delta", "epsilon")), dir.path(), scratch.path()).unwrap_err(),
    );
    assert!(matches!(e, Abort::FindCount { found: 0, .. }), "{e}");
}

// ------------------------------------------------------------------ row 18

/// Row 18: the no-op gate is per FILE, on MATERIALIZED bytes. That one comparison covers
/// `Prepend { repeat: 0 }`, `Prepend { text: "" }` and a sequence that cancels out — none of
/// which the deleted per-op `replace != find` check ever caught. Without it a mutant that
/// equals the original defeats the mount readback: the `cmp` succeeds unmounted.
#[test]
fn noop() {
    let dir = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    write(dir.path(), "f.txt", "alpha\nbeta\n");
    let mk = |ops: &str| probe(&format!("id='P'\n{ops}\n[expect]\noutcome='pass'\n"));

    for hostile in [
        "[[mutation]]\nkind='prepend'\nfiles=['f.txt']\ntext='// pad\\n'\nrepeat=0",
        "[[mutation]]\nkind='prepend'\nfiles=['f.txt']\ntext=''\nrepeat=200",
        "[[mutation]]\nkind='replace'\nfile='f.txt'\nfind='alpha'\nreplace='gamma'\n\
         [[mutation]]\nkind='replace'\nfile='f.txt'\nfind='gamma'\nreplace='alpha'",
    ] {
        let e = abort_of(mutate::apply(&mk(hostile), dir.path(), scratch.path()).unwrap_err());
        assert!(matches!(e, Abort::NoOpMutation { .. }), "{hostile} -> {e}");
    }
    // positive: a DELETION is not a no-op
    assert!(mutate::apply(
        &mk("[[mutation]]\nkind='replace'\nfile='f.txt'\nfind='alpha'\nreplace=''"),
        dir.path(),
        scratch.path()
    )
    .is_ok());
}

// ------------------------------------------------------------------ row 19

/// Row 19: `assert_count` runs on the MUTANT, after all mutations have composed.
#[test]
fn assert_count() {
    let dir = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    write(dir.path(), "f.txt", "alpha\nbeta\nalpha\n");
    let mk = |text: &str, count: usize| {
        probe(&format!(
            "id='P'\n[[mutation]]\nkind='replace'\nfile='f.txt'\nfind='beta'\nreplace='alpha'\n\
             [[assert_count]]\nfile='f.txt'\ntext={text:?}\ncount={count}\n[expect]\noutcome='pass'\n"
        ))
    };
    assert!(mutate::apply(&mk("alpha", 3), dir.path(), scratch.path()).is_ok());
    // off-by-one
    let e = abort_of(mutate::apply(&mk("alpha", 2), dir.path(), scratch.path()).unwrap_err());
    assert!(
        matches!(
            e,
            Abort::AssertCount {
                expected: 2,
                found: 3,
                ..
            }
        ),
        "{e}"
    );
    // present in the ORIGINAL, absent from the mutant
    let e = abort_of(mutate::apply(&mk("beta", 1), dir.path(), scratch.path()).unwrap_err());
    assert!(
        matches!(
            e,
            Abort::AssertCount {
                expected: 1,
                found: 0,
                ..
            }
        ),
        "{e}"
    );
}

// ------------------------------------------------------------------ row 20

/// Row 20: probe-pin must never create files under the tree it is measuring. The assert is
/// on containment, not inequality — a parent-blind `path != workspace_root` accepts every
/// subdirectory of the worktree.
#[test]
fn scratch_outside() {
    let ws = tempfile::tempdir().unwrap();
    assert!(isolate::scratch_dir(&std::env::temp_dir(), ws.path()).is_ok());

    let inside = ws.path().join("x");
    std::fs::create_dir_all(&inside).unwrap();
    let e = isolate::scratch_dir(&inside, ws.path()).unwrap_err();
    assert!(matches!(e, Abort::ScratchInsideWorkspace { .. }), "{e}");
    // TRIVIALIZE control: the scratch is never EQUAL to the root, so equality catches nothing
    match e {
        Abort::ScratchInsideWorkspace { scratch, workspace } => assert_ne!(scratch, workspace),
        _ => unreachable!(),
    }
}

// ------------------------------------------- row 20's companion: the escaping key (MAJOR-1)

/// Row 20 constrains the CONTAINER (`scratch` is outside the workspace). This constrains the
/// KEYS joined onto it: `mutate::apply` writes `scratch.join(&probe.id).join(rel)`, so a `..`
/// in EITHER half walks back out of the container the row-20 assert just certified, into the
/// tree being measured. `verify_fingerprint` cannot see the id half either — the escaped write
/// is not in `probe.touched()`.
///
/// Arm 1 and arm 3 EXHIBIT the escape through the shipping `mutate::apply`. Arms 2 and 4 drive
/// the guarded path in pipeline order (§7 step 1 `validate`, then step 8 `apply`) and assert
/// both halves of the fix: the abort fires AND nothing appears outside the scratch dir.
#[test]
fn path_keys_cannot_escape_the_scratch_dir() {
    let base = tempfile::tempdir().unwrap();
    let ws = base.path().join("ws");
    std::fs::create_dir_all(&ws).unwrap();
    write(&ws, "f.txt", "alpha\n");
    write(&ws, "VICTIM2.rs", "alpha\n");
    let scratch = tempfile::tempdir().unwrap();

    // `../` × depth(dir), then down into the workspace: from `dir`, out to the filesystem root
    // and back in. Every component exists, so the kernel resolves the whole path.
    let escape_to = |dir: &Path, name: &str| {
        let up = dir
            .components()
            .filter(|c| matches!(c, std::path::Component::Normal(_)))
            .count();
        format!(
            "{}{}/{name}",
            "../".repeat(up),
            ws.strip_prefix("/").unwrap().display()
        )
    };
    let listing = || {
        let mut v: Vec<String> = std::fs::read_dir(&ws)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    };
    let manifest_with = |block: &str| -> Manifest {
        toml::from_str(&format!(
            "version = 1\n\
             [target]\nmode='runtime-read'\npackage='p'\ntest='t'\nfilter='f'\nfilter_match='substring'\n\
             [output]\nfile='o.md'\nmarker='PROBE-PIN'\n\
             [[probe]]\nid='P0_control'\n[probe.expect]\noutcome='pass'\n{block}"
        ))
        .expect("hostile manifest parses — the schema does not constrain these fields")
    };
    // §7 in miniature: validate, then apply every non-control probe.
    let guarded = |m: &Manifest| -> anyhow::Result<()> {
        manifest::validate(m)?;
        for p in m.probes.iter().filter(|p| !p.mutations.is_empty()) {
            mutate::apply(p, &ws, scratch.path())?;
        }
        Ok(())
    };
    let hostile_id = escape_to(scratch.path(), "VICTIM.rs");
    let id_manifest = manifest_with(&format!(
        "[[probe]]\nid={hostile_id:?}\n\
         [[probe.mutation]]\nkind='replace'\nfile='f.txt'\nfind='alpha'\nreplace='beta'\n\
         [probe.expect]\noutcome='pass'\n"
    ));

    // arm 1 — the phenomenon: an unvalidated id writes INSIDE the measured workspace
    mutate::apply(&id_manifest.probes[1], &ws, scratch.path()).expect("the escape succeeds");
    assert_eq!(
        std::fs::read_to_string(ws.join("VICTIM.rs/f.txt")).unwrap(),
        "beta\n",
        "the escape must be real, or arm 2 pins nothing"
    );
    std::fs::remove_dir_all(ws.join("VICTIM.rs")).unwrap();

    // arm 2 — the guard: refused, and NO file appears outside the scratch dir
    let e = guarded(&id_manifest).unwrap_err();
    assert!(
        format!("{e}").contains("is not a plain name"),
        "the id must be refused by content: {e}"
    );
    assert_eq!(listing(), ["VICTIM2.rs", "f.txt"], "nothing may be written");

    // arm 3 — the phenomenon, `.join(rel)` half: the mutant lands ON a workspace file
    let hostile_file = escape_to(&scratch.path().join("P1_escaping_file"), "VICTIM2.rs");
    let file_manifest = manifest_with(&format!(
        "[[probe]]\nid='P1_escaping_file'\n\
         [[probe.mutation]]\nkind='replace'\nfile={hostile_file:?}\nfind='alpha'\nreplace='beta'\n\
         [probe.expect]\noutcome='pass'\n"
    ));
    mutate::apply(&file_manifest.probes[1], &ws, scratch.path()).expect("the escape succeeds");
    assert_eq!(
        std::fs::read_to_string(ws.join("VICTIM2.rs")).unwrap(),
        "beta\n",
        "the mutant must have been written over the workspace file"
    );
    write(&ws, "VICTIM2.rs", "alpha\n");

    // arm 4 — the guard, same half
    let e = guarded(&file_manifest).unwrap_err();
    assert!(
        format!("{e}").contains("is not a workspace-relative path"),
        "the mutation file must be refused by content: {e}"
    );
    assert_eq!(
        std::fs::read_to_string(ws.join("VICTIM2.rs")).unwrap(),
        "alpha\n",
        "nothing may be written"
    );
    assert_eq!(listing(), ["VICTIM2.rs", "f.txt"]);

    // positive: an ordinary id and an ordinary relative file still validate and materialize
    let ok = manifest_with(
        "[[probe]]\nid='P1_ordinary'\n\
         [[probe.mutation]]\nkind='replace'\nfile='f.txt'\nfind='alpha'\nreplace='beta'\n\
         [probe.expect]\noutcome='pass'\n",
    );
    guarded(&ok).expect("the shipping shape must survive the guard");
    assert!(scratch.path().join("P1_ordinary/f.txt").exists());
    assert_eq!(listing(), ["VICTIM2.rs", "f.txt"]);
}

// ------------------------------------------------------------------ row 22

/// Row 22: the splice preserves every byte outside the marker pair — prose above, prose
/// below, and the file's own line endings.
#[test]
fn splice() {
    let file = Path::new("out.md");
    let text = "above\r\n// PROBE-PIN:BEGIN old\r\n// stale\r\n// PROBE-PIN:END\r\nbelow\r\n";
    let out = block::splice(
        text,
        "PROBE-PIN",
        file,
        "// PROBE-PIN:BEGIN new\n// PROBE-PIN:END",
    )
    .unwrap();
    assert_eq!(
        out,
        "above\r\n// PROBE-PIN:BEGIN new\n// PROBE-PIN:END\r\nbelow\r\n"
    );
    assert!(out.starts_with("above\r\n") && out.ends_with("below\r\n"));

    for (bad, begins, ends) in [
        ("// PROBE-PIN:BEGIN x\n", 1, 0),
        (
            "// PROBE-PIN:BEGIN x\n// PROBE-PIN:BEGIN y\n// PROBE-PIN:END\n",
            2,
            1,
        ),
        ("// PROBE-PIN:END\n// PROBE-PIN:BEGIN x\n", 1, 1), // END before BEGIN
    ] {
        let e = block::locate(bad, "PROBE-PIN", file).unwrap_err();
        assert!(
            matches!(e, Abort::MarkerNotUnique { begins: b, ends: n, .. } if b == begins && n == ends),
            "{bad:?} -> {e}"
        );
    }
}

// ------------------------------------------------------------------ row 23

/// Row 23: the render is byte-deterministic. The guard PARSES TWICE and renders each: a
/// render-twice-from-one-parse guard scores identically with and without the defect, because
/// a `HashMap`'s iteration order is stable within one instance.
#[test]
fn idempotent() {
    let ids: Vec<String> = load("unsorted_ids.toml")
        .probes
        .iter()
        .map(|p| p.id.clone())
        .collect();
    // MINOR-F: the fixture precondition is MEASURED, not merely stated. The corpus ids
    // P0..P9 are already sorted, so a sorted fixture makes the TRIVIALIZE arm a no-op.
    assert_ne!(ids, {
        let mut s = ids.clone();
        s.sort();
        s
    });

    let render_once = || {
        let m = load("unsorted_ids.toml");
        let outcomes: Vec<Outcome> = m
            .probes
            .iter()
            .map(|p| Outcome {
                id: p.id.clone(),
                rc: 0,
                verdict: Verdict::Pass { mounts_reached: 1 },
            })
            .collect();
        // The DIGEST, not just the table: `[target].env` reaches the block only through the
        // digest, so a `HashMap` there is invisible to a render-only guard (MINOR-4). It is
        // also invisible within ONE parse — `RandomState` is per-instance — hence the loop
        // below, and the fixture's four env keys.
        let digest = block::digest(&m, "m.toml", &outcomes, &[], &[]).unwrap();
        (
            block::render(&m, "m.toml", &digest, &[], &outcomes, &[]),
            digest,
        )
    };
    let (a, b) = (render_once(), render_once());
    assert_eq!(a, b);
    for _ in 0..16 {
        assert_eq!(render_once(), a, "two parses must render identical bytes");
    }
    // the rows appear in DECLARATION order, which is what `check` compares against
    let rows: Vec<&str> = a.0.lines().filter(|l| l.starts_with("// | P")).collect();
    assert!(
        rows[0].contains("P0_control")
            && rows[1].contains("P9_zulu")
            && rows[2].contains("P1_alpha")
    );
}

// ------------------------------------------------------------------ rows 24, 25, 27

fn digest_of(
    m: &Manifest,
    counts: &[ProjectionCount],
    instruments: &[(Instrument, String)],
) -> String {
    let outcomes: Vec<Outcome> = m
        .probes
        .iter()
        .map(|p| Outcome {
            id: p.id.clone(),
            rc: 0,
            verdict: Verdict::Pass { mounts_reached: 1 },
        })
        .collect();
    block::digest(m, "m.toml", &outcomes, counts, instruments).unwrap()
}

/// Row 24: the digest pins what was MEASURED, not just the outcome. Narrowing `[target]`'s
/// filter with identical verdicts flips it, and subsetting a projection's `paths` with an
/// identical count flips it — the two cases a digest over outcomes alone cannot see.
#[test]
fn digest_target() {
    let base = load("projection.toml");
    let counts = [ProjectionCount {
        id: "instrument_sites".into(),
        count: 3,
    }];
    let instruments = [(Instrument::Toolchain, "rustc 1.97.0".to_string())];
    let d0 = digest_of(&base, &counts, &instruments);

    let mut narrowed = base.clone();
    narrowed.target.filter = "ppfixture_census_narrower".into();
    assert_ne!(d0, digest_of(&narrowed, &counts, &instruments));

    let mut subset = base.clone();
    subset.projections[0].paths = vec!["crates/probe-pin/tests".into()];
    assert_ne!(d0, digest_of(&subset, &counts, &instruments));

    // positive: an unrelated COMMENT in the manifest is not an input
    let text = std::fs::read_to_string(fixtures().join("projection.toml")).unwrap();
    let commented: Manifest = toml::from_str(&format!("# an unrelated comment\n{text}")).unwrap();
    assert_eq!(d0, digest_of(&commented, &counts, &instruments));
    // determinism
    assert_eq!(d0, digest_of(&base, &counts, &instruments));
}

/// Row 25: injection-safe by construction. JSON escapes NUL as `\u0000` and keeps every field
/// in its own slot; a hand-rolled NUL-joined serializer gives the two manifests below —
/// which say DIFFERENT things — one and the same byte string, `P1\0claim\0c`.
#[test]
fn digest_inject() {
    let mk = |id: &str, claim: &str| {
        let mut m = load("dogfood.toml");
        m.probes[1].id = id.to_string();
        m.probes[1].claim = claim.to_string();
        digest_of(&m, &[], &[])
    };
    // one colliding pair per candidate joiner — every single-delimiter joiner has one
    assert_ne!(mk("P1\u{0}claim", "c"), mk("P1", "claim\u{0}c"));
    assert_ne!(mk("P1+claim", "c"), mk("P1", "claim+c"));
    // and a field-boundary shift is still visible
    assert_ne!(mk("P1\u{0}claim\u{0}c", "c"), mk("P1\u{0}claim", "c"));
    assert_ne!(mk("P1\u{0}claim", "c"), mk("P1", "c"));
}

/// Row 27: BOTH instruments are digest inputs. With every verdict and count identical, each
/// version string alone flips the digest.
#[test]
fn instrument_digest() {
    let m = load("projection.toml");
    let counts = [ProjectionCount {
        id: "instrument_sites".into(),
        count: 3,
    }];
    let v = |rustc: &str, ag: &str| {
        digest_of(
            &m,
            &counts,
            &[
                (Instrument::Toolchain, rustc.to_string()),
                (Instrument::AstGrep, ag.to_string()),
            ],
        )
    };
    let base = v("rustc 1.97.0", "ast-grep 0.44.1");
    assert_ne!(base, v("rustc 1.98.0", "ast-grep 0.44.1"));
    assert_ne!(base, v("rustc 1.97.0", "ast-grep 0.45.0"));
}

// ------------------------------------------------------------------ rows 28, 29

/// The five measured skew arms, shared by rows 28 and 29.
fn skew_arms() -> Vec<(&'static str, String, String)> {
    let blk = |rustc: &str, ag: &str, count: usize, cell: &str| {
        format!(
            "// PROBE-PIN:BEGIN manifest=m.toml digest=sha256:0123456789abcdef\n\
             // instrument rustc = {rustc}\n\
             // instrument ast-grep = {ag}\n\
             // | probe | mutation | expect | verdict | firing assertion (anchor) | provenance |\n\
             // |---|---|---|---|---|---|\n\
             // | P1 | f.rs ×1 | fail | fail | {cell} | census.rs |\n\
             // `TargetPin::Player` is constructed or matched at {count} sites.\n\
             // PROBE-PIN:END"
        )
    };
    let base = blk("rustc 1.97.0", "ast-grep 0.44.1", 3, "ANCHOR");
    vec![
        ("clean", base.clone(), base.clone()),
        (
            "code drift only",
            base.clone(),
            blk("rustc 1.97.0", "ast-grep 0.44.1", 3, "OTHER"),
        ),
        (
            "toolchain moved, numbers same",
            base.clone(),
            blk("rustc 1.98.0", "ast-grep 0.44.1", 3, "ANCHOR"),
        ),
        (
            "ast-grep moved AND 3->6",
            base.clone(),
            blk("rustc 1.97.0", "ast-grep 0.45.0", 6, "ANCHOR"),
        ),
        (
            "both moved, numbers same",
            base,
            blk("rustc 1.98.0", "ast-grep 0.45.0", 3, "ANCHOR"),
        ),
    ]
}

/// Row 28: an instrument change is not code drift. Both instruments participate, and when
/// both moved the message lists BOTH in one `Instrument(Vec<…>)` — no precedence rule.
#[test]
fn instrument_skew() {
    let arms = skew_arms();
    let got: Vec<CheckOutcome> = arms.iter().map(|(_, a, b)| block::check(a, b)).collect();

    assert_eq!(got[0], CheckOutcome::Clean, "Clean must be reachable");
    match &got[1] {
        CheckOutcome::Drift {
            cause,
            numbers_moved,
            ..
        } => {
            assert_eq!(*cause, DriftCause::Code);
            assert!(numbers_moved);
        }
        o => panic!("code drift -> {o:?}"),
    }
    let instrument_of = |o: &CheckOutcome| match o {
        CheckOutcome::Drift {
            cause: DriftCause::Instrument(c),
            numbers_moved,
            ..
        } => (
            c.iter().map(|c| c.instrument).collect::<Vec<_>>(),
            *numbers_moved,
        ),
        o => panic!("expected an instrument drift, got {o:?}"),
    };
    assert_eq!(instrument_of(&got[2]), (vec![Instrument::Toolchain], false));
    assert_eq!(instrument_of(&got[3]), (vec![Instrument::AstGrep], true));
    assert_eq!(
        instrument_of(&got[4]),
        (vec![Instrument::Toolchain, Instrument::AstGrep], false)
    );
    assert!(block::report(&got[4], "m.toml").contains("rustc 1.97.0 -> rustc 1.98.0"));
    assert!(block::report(&got[4], "m.toml").contains("ast-grep 0.44.1 -> ast-grep 0.45.0"));
}

/// Row 29: `run --write` refuses IFF an instrument moved AND a measured number moved — the
/// laundering case, where re-stamping would record 6 sites under a new tool and leave `check`
/// green. Refusing on ANY instrument change instead would make a toolchain bump unwritable.
#[test]
fn write_refusal() {
    let arms = skew_arms();
    let refused: Vec<bool> = arms
        .iter()
        .map(|(_, a, b)| block::write_refusal(&block::check(a, b), a, b).is_some())
        .collect();
    assert_eq!(refused, vec![false, false, false, true, false]);

    let (_, a, b) = &arms[3];
    let abort = block::write_refusal(&block::check(a, b), a, b).unwrap();
    let text = abort.to_string();
    assert!(
        text.contains("ast-grep 0.44.1 -> ast-grep 0.45.0"),
        "{text}"
    );
    assert!(
        text.contains("at 3 sites") && text.contains("at 6 sites"),
        "{text}"
    );
    assert!(
        text.contains("delete the PROBE-PIN:BEGIN…END block"),
        "{text}"
    );
}

// ------------------------------------------------------------------ rows 26, 30

/// Row 26: fail-closed on a missing projection tool — probe-pin will not emit a block whose
/// projection sentence is silently absent. Both `run --write` and `check` reach this through
/// the same step-4 call, because `used_instruments` is what puts ast-grep on the list.
#[test]
fn proj_missing() {
    let m = load("projection.toml");
    assert_eq!(
        block::used_instruments(&m),
        vec![Instrument::Toolchain, Instrument::AstGrep]
    );
    let missing = fixtures().join("astgrep_does_not_exist");
    let e =
        project::instrument_version(Instrument::AstGrep, missing.to_str().unwrap()).unwrap_err();
    assert!(matches!(e, Abort::ProjectionToolMissing { .. }), "{e}");
    // present but broken is ALSO fail-closed, not a warn-and-continue
    let broken = fixtures().join("astgrep_broken.sh");
    let e = project::count(&m.projections[0], &fixtures(), broken.to_str().unwrap()).unwrap_err();
    assert!(matches!(e, Abort::ProjectionToolMissing { .. }), "{e}");

    // positive: a working tool renders the sentence with the measured count
    let ok = fixtures().join("astgrep_three.sh");
    let count = project::count(&m.projections[0], &fixtures(), ok.to_str().unwrap()).unwrap();
    assert_eq!(count, 3);
    assert_eq!(
        project::sentence(&m.projections[0], count),
        "`Instrument` is named at 3 sites."
    );
    assert_eq!(
        project::instrument_version(Instrument::AstGrep, ok.to_str().unwrap()).unwrap(),
        "ast-grep 0.44.1"
    );
}

/// Row 30: no projections ⇒ no ast-grep dependency and no ast-grep line — but the `rustc`
/// line is still there. The asymmetry is derived from one rule (an instrument is recorded iff
/// it produced a number), not special-cased, so it is asserted in both directions.
#[test]
fn no_proj() {
    let m = load("dogfood.toml");
    assert!(m.projections.is_empty());
    assert_eq!(block::used_instruments(&m), vec![Instrument::Toolchain]);
    let outcomes: Vec<Outcome> = m
        .probes
        .iter()
        .map(|p| Outcome {
            id: p.id.clone(),
            rc: 0,
            verdict: Verdict::Pass { mounts_reached: 1 },
        })
        .collect();
    let rendered = block::render(
        &m,
        "m.toml",
        "0123456789abcdef",
        &[(Instrument::Toolchain, "rustc 1.97.0".into())],
        &outcomes,
        &[],
    );
    assert!(!rendered.contains("instrument ast-grep"));
    assert!(rendered.contains("// instrument rustc = rustc 1.97.0"));
    // and `check` classifies such a block with no tool installed at all
    assert_eq!(block::check(&rendered, &rendered), CheckOutcome::Clean);
    assert!(matches!(
        block::check(&rendered, &rendered.replace("P0_control", "P0_ctl")),
        CheckOutcome::Drift {
            cause: DriftCause::Code,
            ..
        }
    ));
}

// ------------------------------------------------------------------ row 31

/// Row 31: the exit-code contract. An instrument failure is 2, not the anyhow idiom's 1;
/// drift is 1, not 2. The `run <compiled.toml>` arm aborts at step 1, so it costs zero
/// target runs. The end-to-end drift-is-1 arm is `isolation_e2e::drift` (a) — it needs a
/// full pipeline run, which needs the namespace.
#[test]
fn exit_codes() {
    let code = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_probe-pin"))
            .args(args)
            .output()
            .unwrap()
            .status
            .code()
            .unwrap()
    };
    assert_eq!(
        code(&["run", fixtures().join("compiled.toml").to_str().unwrap()]),
        2
    );
    assert_eq!(
        code(&["check", fixtures().join("compiled.toml").to_str().unwrap()]),
        2
    );
    assert_eq!(code(&["run", "/nonexistent/manifest.toml"]), 2);

    assert_eq!(block::exit_code(&CheckOutcome::Clean), 0);
    assert_eq!(
        block::exit_code(&CheckOutcome::Drift {
            cause: DriftCause::Code,
            diff: String::new(),
            numbers_moved: true
        }),
        1
    );
    assert_eq!(
        block::exit_code(&CheckOutcome::Drift {
            cause: DriftCause::Instrument(vec![InstrumentChange {
                instrument: Instrument::Toolchain,
                was: "a".into(),
                now: "b".into()
            }]),
            diff: String::new(),
            numbers_moved: false
        }),
        1
    );
}

// ------------------------------------- §8 message fidelity on the two sentinel-free paths

/// §8's texts are the contract, and each of these rows promised a value the run never measured
/// (MINOR-2, MINOR-3): a rendered `-1` reads as an exit status when the process never started,
/// and `scratch_dir`'s failure named "the mutant" for a directory that holds none yet.
#[test]
fn abort_texts_promise_only_measured_values() {
    let spawn_failed = Abort::IsolationUnavailable {
        rc: None,
        stderr: "No such file or directory (os error 2)".into(),
    }
    .to_string();
    assert!(
        spawn_failed.contains("could not be spawned"),
        "{spawn_failed}"
    );
    assert!(!spawn_failed.contains("-1"), "no sentinel: {spawn_failed}");

    let ran = Abort::IsolationUnavailable {
        rc: Some(1),
        stderr: "unshare: write failed /proc/self/uid_map".into(),
    }
    .to_string();
    assert!(ran.contains("failed (exit 1):"), "{ran}");

    // the scratch-dir path names the DIRECTORY, and claims no mutant
    let e = isolate::scratch_dir(Path::new("/nonexistent-tmpdir"), Path::new("/")).unwrap_err();
    assert!(matches!(e, Abort::MaterializeFailed { .. }), "{e}");
    assert!(
        e.to_string()
            .starts_with("probe-pin: scratch-directory write failed at /nonexistent-tmpdir:"),
        "{e}"
    );
}

// ------------------------------------------------------------------ row 33

/// Row 33: a run that selected ZERO tests is an INSTRUMENT FAILURE, not a pass. Measured on
/// a real libtest selection: `filter_match = "exact"` against a module-prefix filter selects
/// nothing, and so does a rotted filter under a perfectly correct substring argv. The
/// ordering arm — the abort names the CONTROL and no probe runs — is
/// `isolation_e2e::no_tests_selected_ordering`.
#[test]
fn no_tests_selected() {
    let mut t = target();
    // arm 1: the shipping substring filter selects tests
    let run = child("ppfixture_census", &[], &[]);
    let obs = verdict::observe("P0_control", &run, &t, &[]).expect("substring filter selects");
    assert!(obs.test_count > 0 && !obs.target_failed);

    // arm 2: exact match on a filter that is a PREFIX selects nothing
    let run = child("ppfixture_cens", &[], &["--exact"]);
    t.filter = "ppfixture_cens".into();
    t.filter_match = manifest::FilterMatch::Exact;
    let e = verdict::observe("P0_control", &run, &t, &[]).unwrap_err();
    assert_eq!(run.rc, 0, "libtest reports a green exit for a 0-test run");
    assert!(
        matches!(e, Abort::NoTestsSelected { ref probe, filter_match: manifest::FilterMatch::Exact, .. } if probe == "P0_control"),
        "{e}"
    );
    assert!(e.to_string().contains("INSTRUMENT FAILURE"));

    // arm 3: a filter that ROTTED (renamed upstream), under a correct substring argv
    let run = child("ppfixture_census_OLDNAME", &[], &[]);
    t.filter = "ppfixture_census_OLDNAME".into();
    t.filter_match = manifest::FilterMatch::Substring;
    assert!(matches!(
        verdict::observe("P0_control", &run, &t, &[]).unwrap_err(),
        Abort::NoTestsSelected { .. }
    ));
}

// ------------------------------------------------------------------ argv + verdict wiring

/// `filter_match` and `[target].args` both reach libtest's own selection — the only place
/// where a dead schema field would show. Measured against this binary's real test names.
#[test]
fn argv_is_live() {
    let mut t = target();
    t.filter = "ppfixture".into();
    assert_eq!(
        isolate::argv(&t),
        vec![
            "ppfixture",
            "--test-threads=1",
            "--format",
            "json",
            "-Z",
            "unstable-options"
        ]
    );
    t.filter_match = manifest::FilterMatch::Exact;
    assert_eq!(isolate::argv(&t)[1], "--exact");
    t.filter_match = manifest::FilterMatch::Substring;
    t.args = vec!["--skip".into(), "ppfixture_census".into()];
    assert_eq!(isolate::argv(&t).last().unwrap(), "ppfixture_census");

    let all = child("ppfixture", &[], &[]);
    let skipped = child("ppfixture", &[], &["--skip", "ppfixture_census"]);
    let count = |r: &Run| {
        let mut t = target();
        t.filter = "ppfixture".into();
        verdict::observe("P", r, &t, &[]).unwrap().test_count
    };
    // `--exact` here would select 0 (the filter is a prefix), which is exactly row 33's arm 2
    assert!(
        count(&all) > count(&skipped),
        "[target].args must reach the argv"
    );
}

/// `Verdict::Fail` IS "target failed AND every anchor in ONE record". Its complement is a
/// verdict MISMATCH naming the missed anchor — never a `Pass`, whose rendering would
/// transcribe "visible-and-still-passing" for a run whose target failed.
#[test]
fn fail_complement_is_a_mismatch() {
    let p = probe(
        "id='P1'\n[[mutation]]\nkind='replace'\nfile='f'\nfind='a'\nreplace='b'\n\
         [expect]\noutcome='fail'\nanchor=['HIT','MISS']\n",
    );
    let mut obs = verdict::Observed {
        rc: 101,
        test_count: 1,
        target_failed: true,
        failed_captures: vec!["panicked at tests/c.rs:1:1:\nHIT\n".into()],
        ..Default::default()
    };
    // the mismatch is TYPED: `main` formats it, and the missed anchor is a carried value, not
    // a substring of a pre-formatted report (MINOR-7)
    let e = verdict::verdict(&p, &obs, 1).unwrap_err();
    assert_eq!(e.missed_anchor.as_deref(), Some("MISS"));
    assert_eq!(e.observed_rc, 101);
    assert!(e.to_string().contains("anchor \"MISS\""), "{e}");

    obs.failed_captures[0].push_str("MISS\n");
    assert_eq!(
        verdict::verdict(&p, &obs, 1),
        Ok(Verdict::Fail {
            provenance: vec![PathBuf::from("tests/c.rs")]
        })
    );
}

// ------------------------------------------------------------------ targets, not assertions

const PROD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/prod.txt");

/// The runtime-read census the Tier-2 manifests mutate. Reading at RUNTIME is what makes a
/// bind-mounted mutant visible; an `include_str!` copy would be baked in at build time.
#[test]
fn ppfixture_census() {
    let text = std::fs::read_to_string(PROD).expect("prod.txt");
    let sites = text.lines().filter(|l| l.contains("marker")).count();
    assert_eq!(
        sites, 2,
        "CENSUS VIOLATED: expected 2 sites, got {sites}; text: {text:?}"
    );
}

/// Row 8's fixture A: UNCONDITIONAL — no `RUST_MIN_STACK` value rescues it.
#[test]
fn ppfixture_abort() {
    if std::env::var("PP_FIXTURE").as_deref() == Ok("abort") {
        std::process::abort();
    }
}

/// Row 13's fixture B: RESCUABLE — 1200 × 4 KiB frames on a spawned thread, whose stack size
/// is what `RUST_MIN_STACK` sets.
#[test]
fn ppfixture_stack() {
    if std::env::var("PP_FIXTURE").as_deref() != Ok("stack") {
        return;
    }
    fn burn(n: u32) -> u64 {
        let frame = [0u8; 4096];
        if n == 0 {
            return std::hint::black_box(&frame)[0] as u64;
        }
        burn(n - 1) + std::hint::black_box(&frame)[7] as u64
    }
    std::thread::spawn(|| std::hint::black_box(burn(1200)))
        .join()
        .unwrap();
}

/// Row 12's fixture: a panic whose capture length depends on `RUST_BACKTRACE`.
#[test]
fn ppfixture_panic() {
    if std::env::var("PP_FIXTURE").as_deref() == Ok("panic") {
        panic!("PPFIXTURE PANIC");
    }
}

/// Row 12's instrument: a process whose ambient environment the caller owns, which runs
/// `ppfixture_panic` through the SHIPPING `apply_child_env` and reports the capture length.
#[test]
fn ppfixture_envprobe() {
    if std::env::var("PP_FIXTURE").as_deref() != Ok("envprobe") {
        return;
    }
    let mut t = target();
    t.env.clear();
    if std::env::var("PP_FORCE_BACKTRACE").is_ok() {
        t.env.insert("RUST_BACKTRACE".into(), "1".into());
    }
    let mut cmd = Command::new("timeout");
    cmd.args(["-k", "5", "60"])
        .arg(std::env::current_exe().unwrap())
        .args(["ppfixture_panic", "--exact", "--test-threads=1"])
        .args(["--format", "json", "-Z", "unstable-options"]);
    isolate::apply_child_env(&mut cmd, &t);
    cmd.env("PP_FIXTURE", "panic");
    let out = cmd.output().unwrap();
    let run = Run {
        rc: isolate::exit_rc(&out.status),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::new(),
    };
    let obs = verdict::observe("envprobe", &run, &t, &[]).unwrap();
    std::fs::write(
        std::env::var("PP_OUT").unwrap(),
        obs.failed_captures[0].len().to_string(),
    )
    .unwrap();
}

/// §7 step 10's fixture (MAJOR-2): a target that WRITES THE TREE. `unshare --mount` isolates
/// the mount table, not the filesystem, so this write reaches the real file whenever the path
/// is not mounted — which is exactly the control run. The appended line deliberately avoids
/// the word the manifest's `find` matches, so the write does not rot the probe it feeds.
#[test]
fn ppfixture_treewrite() {
    if std::env::var("PP_FIXTURE").as_deref() != Ok("treewrite") {
        return;
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/tmp/tree_write.txt"
    );
    let text = std::fs::read_to_string(path).expect("tree_write.txt");
    std::fs::write(path, format!("{text}the target wrote here\n")).unwrap();
}

/// Row 14's fixture: hangs until `timeout` kills it.
#[test]
fn ppfixture_hang() {
    if std::env::var("PP_FIXTURE").as_deref() == Ok("hang") {
        std::thread::sleep(std::time::Duration::from_secs(600));
    }
}
