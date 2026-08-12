//! §7 steps 2-3: where the workspace is, and which binary the manifest's target names.
//!
//! Both shell out to cargo and both inherit the ambient environment — `env_clear()` is scoped
//! to the step-8 target run only (MAJOR-3), because clearing it here would strip `CARGO_HOME`
//! and `CARGO_TARGET_DIR` and silently rejoin the shared `~/.cargo`.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Abort;

/// `cargo locate-project --workspace` — authoritative, and it replaces a walk-up heuristic.
pub fn workspace_root() -> Result<PathBuf, Abort> {
    let out = Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .output()
        .map_err(|e| Abort::WorkspaceRootUnresolved {
            stderr: e.to_string(),
        })?;
    if !out.status.success() {
        return Err(Abort::WorkspaceRootUnresolved {
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    let manifest = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string());
    manifest
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Abort::WorkspaceRootUnresolved {
            stderr: format!("'{}' has no parent directory", manifest.display()),
        })
}

/// Pick the ONE test binary out of a `--message-format=json` artifact stream.
///
/// A `cargo test --test X --no-run` stream carries more than one `executable`: the package's
/// `bin` target is built too and also reports one. Filtering on `target.kind == ["test"]` and
/// `target.name` is what makes the choice unambiguous, and exactly-one is asserted rather
/// than taking the first.
pub fn pick_executable(stdout: &str, package: &str, test: &str) -> Result<PathBuf, Abort> {
    let mut candidates: Vec<String> = Vec::new();
    let mut chosen: Vec<String> = Vec::new();
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v["reason"] != "compiler-artifact" {
            continue;
        }
        let Some(exe) = v["executable"].as_str() else {
            continue;
        };
        candidates.push(exe.to_string());
        let kind_is_test = v["target"]["kind"]
            .as_array()
            .is_some_and(|k| k.len() == 1 && k[0] == "test");
        if kind_is_test && v["target"]["name"] == test {
            chosen.push(exe.to_string());
        }
    }
    match chosen.len() {
        1 => Ok(PathBuf::from(chosen.remove(0))),
        found => Err(Abort::TargetBinaryUnresolved {
            package: package.to_string(),
            test: test.to_string(),
            found,
            candidates,
        }),
    }
}

/// `cargo test -p <package> --test <test> --no-run` — run OUTSIDE the namespace, on the
/// pristine tree, before any mutant exists.
///
/// Pinned to `root`, matching `project::count`. Unlike `workspace_root`, which must inherit the
/// caller's cwd because that is how it discovers the root at all, this invocation already has
/// the root, and inheriting a cwd only makes its side effects depend on where probe-pin was run
/// from. `CARGO_TARGET_DIR` is commonly relative — the Tilt resources here pass
/// `target/probe-pin` — and a relative value resolves against the cwd of whichever process
/// finally execs cargo. Measured: with the cwd left inherited, running the Tier-2 suite (whose
/// tests run with cwd = the crate directory) materialized a 285M `crates/probe-pin/target/`
/// that no `.gitignore` rule covers, because the repo ignores a ROOT-anchored `/target`.
pub fn resolve(root: &Path, package: &str, test: &str) -> Result<PathBuf, Abort> {
    let out = Command::new("cargo")
        .current_dir(root)
        .args([
            "test",
            "-p",
            package,
            "--test",
            test,
            "--no-run",
            "--message-format=json",
        ])
        .output()
        .map_err(|e| Abort::TargetBuildFailed {
            package: package.to_string(),
            test: test.to_string(),
            stderr: e.to_string(),
        })?;
    if !out.status.success() {
        return Err(Abort::TargetBuildFailed {
            package: package.to_string(),
            test: test.to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    pick_executable(&String::from_utf8_lossy(&out.stdout), package, test)
}
