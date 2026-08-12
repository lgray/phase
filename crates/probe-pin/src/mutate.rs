//! §7 step 8's first half: sequential mutation, materialization, and the two gates that make
//! a mount-reach readback meaningful.

use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::manifest::{Mutation, Probe};
use crate::Abort;

/// One `mount --bind <mutant> <target>` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub mutant: PathBuf,
    pub target: PathBuf,
}

/// Apply every mutation in declaration order, materialize the result outside the workspace,
/// and gate it.
///
/// Each `Replace`'s `count == 1` check runs against the RUNNING text for that file, so a
/// two-op sequence whose second `find` only exists after the first op is expressible. The
/// no-op gate is per FILE on MATERIALIZED bytes — one comparison that covers `Replace`,
/// `Prepend { repeat: 0 }`, `Prepend { text: "" }` and sequences that cancel out.
pub fn apply(probe: &Probe, root: &Path, scratch: &Path) -> anyhow::Result<Vec<Mount>> {
    let mut running: Vec<(String, String, String)> = Vec::new(); // (rel, original, current)
    for (index, mutation) in probe.mutations.iter().enumerate() {
        for rel in mutation.files() {
            if !running.iter().any(|(f, _, _)| f == rel) {
                let text = std::fs::read_to_string(root.join(rel)).with_context(|| {
                    format!(
                        "probe-pin: {} cannot read {}",
                        probe.id,
                        root.join(rel).display()
                    )
                })?;
                running.push((rel.to_string(), text.clone(), text));
            }
        }
        match mutation {
            Mutation::Replace {
                file,
                find,
                replace,
            } => {
                let slot = running
                    .iter_mut()
                    .find(|(f, _, _)| f == file)
                    .expect("seeded above");
                let found = slot.2.matches(find.as_str()).count();
                if found != 1 {
                    return Err(Abort::FindCount {
                        probe: probe.id.clone(),
                        index,
                        file: PathBuf::from(file),
                        found,
                    }
                    .into());
                }
                slot.2 = slot.2.replacen(find.as_str(), replace, 1);
            }
            Mutation::Prepend {
                files,
                text,
                repeat,
            } => {
                let pad = text.repeat(*repeat as usize);
                for file in files {
                    let slot = running
                        .iter_mut()
                        .find(|(f, _, _)| f == file)
                        .expect("seeded above");
                    slot.2 = format!("{pad}{}", slot.2);
                }
            }
        }
    }

    let mut mounts = Vec::new();
    for (rel, original, mutant) in &running {
        let dest = scratch.join(&probe.id).join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Abort::MaterializeFailed {
                path: parent.to_path_buf(),
                err: e.to_string(),
            })?;
        }
        std::fs::write(&dest, mutant).map_err(|e| Abort::MaterializeFailed {
            path: dest.clone(),
            err: e.to_string(),
        })?;
        let materialized = std::fs::read(&dest).map_err(|e| Abort::MaterializeFailed {
            path: dest.clone(),
            err: e.to_string(),
        })?;
        if materialized == original.as_bytes() {
            return Err(Abort::NoOpMutation {
                probe: probe.id.clone(),
                file: PathBuf::from(rel),
            }
            .into());
        }
        mounts.push(Mount {
            mutant: dest,
            target: root.join(rel),
        });
    }

    assert_counts(probe, root, &running)?;
    Ok(mounts)
}

/// `[[probe.assert_count]]` against the FINAL mutant text, after all mutations have composed.
fn assert_counts(
    probe: &Probe,
    root: &Path,
    running: &[(String, String, String)],
) -> anyhow::Result<()> {
    for ac in &probe.assert_counts {
        let text = match running.iter().find(|(f, _, _)| *f == ac.file) {
            Some((_, _, mutant)) => mutant.clone(),
            None => std::fs::read_to_string(root.join(&ac.file)).with_context(|| {
                format!(
                    "probe-pin: {} assert_count cannot read {}",
                    probe.id,
                    root.join(&ac.file).display()
                )
            })?,
        };
        let found = text.matches(ac.text.as_str()).count();
        if found != ac.count {
            return Err(Abort::AssertCount {
                probe: probe.id.clone(),
                file: PathBuf::from(&ac.file),
                text: ac.text.clone(),
                expected: ac.count,
                found,
            }
            .into());
        }
    }
    Ok(())
}
