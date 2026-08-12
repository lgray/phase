//! The manifest schema and everything `validate()` can refuse before a single target runs.
//!
//! `Serialize` is derived alongside `Deserialize` purely so `block::digest` needs no
//! hand-written canonical serializer: declaration order IS the canonical order and JSON
//! escaping IS the injection safety.

use std::collections::BTreeMap;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use crate::Abort;

/// probe-pin owns these libtest options; a manifest may not pass them through `[target].args`.
/// Compared against the option NAME (`--x=v` -> `--x`, any `-Z…` -> `-Z`), not the raw token.
const RESERVED: &[&str] = &[
    "--format",
    "-Z",
    "--nocapture",
    "--show-output",
    "--test-threads",
    "-q",
    "--quiet",
    "--exact",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunMode {
    RuntimeRead,
    Compiled,
}

impl std::fmt::Display for RunMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::RuntimeRead => "runtime-read",
            Self::Compiled => "compiled",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterMatch {
    Substring,
    Exact,
}

impl std::fmt::Display for FilterMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Substring => "substring",
            Self::Exact => "exact",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub mode: RunMode,
    pub package: String,
    pub test: String,
    pub filter: String,
    pub filter_match: FilterMatch,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_env")]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_env() -> BTreeMap<String, String> {
    BTreeMap::from([("RUST_MIN_STACK".to_string(), "16777216".to_string())])
}

fn default_timeout() -> u64 {
    300
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub file: String,
    pub marker: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Mutation {
    Replace {
        file: String,
        find: String,
        replace: String,
    },
    Prepend {
        files: Vec<String>,
        text: String,
        #[serde(default = "one")]
        repeat: u32,
    },
}

impl Mutation {
    pub fn files(&self) -> Vec<&str> {
        match self {
            Self::Replace { file, .. } => vec![file.as_str()],
            Self::Prepend { files, .. } => files.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssertCount {
    pub file: String,
    pub text: String,
    pub count: usize,
}

/// `Pass {}` and not a unit variant: a unit variant defeats `deny_unknown_fields`, so a stray
/// `anchor` under `outcome = "pass"` would be silently accepted.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "outcome", rename_all = "lowercase", deny_unknown_fields)]
pub enum Expect {
    Pass {},
    Fail { anchor: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    pub id: String,
    #[serde(default)]
    pub claim: String,
    #[serde(default, rename = "mutation")]
    pub mutations: Vec<Mutation>,
    #[serde(default, rename = "assert_count")]
    pub assert_counts: Vec<AssertCount>,
    pub expect: Expect,
}

impl Probe {
    /// Every file this probe mutates, in first-touch order — the fingerprint set of §7 step 6.
    pub fn touched(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for m in &self.mutations {
            for f in m.files() {
                if !out.iter().any(|p| p == f) {
                    out.push(f.to_string());
                }
            }
        }
        out
    }

    pub fn anchors(&self) -> &[String] {
        match &self.expect {
            Expect::Pass {} => &[],
            Expect::Fail { anchor } => anchor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub id: String,
    pub pattern: String,
    pub paths: Vec<String>,
    pub sentence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub target: Target,
    pub output: Output,
    #[serde(default, rename = "probe")]
    pub probes: Vec<Probe>,
    #[serde(default, rename = "projection")]
    pub projections: Vec<Projection>,
}

impl Manifest {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("probe-pin: cannot read manifest {}", path.display()))?;
        toml::from_str(&text)
            .with_context(|| format!("probe-pin: cannot parse manifest {}", path.display()))
    }

    /// The zero-mutation probe. `validate()` has already proved there is exactly one.
    pub fn control(&self) -> &Probe {
        self.probes
            .iter()
            .find(|p| p.mutations.is_empty())
            .expect("validate() proved exactly one zero-mutation probe exists")
    }
}

/// The anchor line-number lint — the invariant the deleted pad subsystem approximated, at
/// validation time and at zero target runs. Three forms:
/// `\bline\s*[:=]?\s*\d` ∪ `\.[A-Za-z]{1,5}:\d+` ∪ `:\d+:\d+`.
///
/// Named residual (docs/probe-pin.md): a positional integer that is syntactically
/// indistinguishable from a legitimate count — `("…/engine.rs", 11492)` and `L11492` — is
/// accepted, because every regex that rejects it also rejects P9's real shipping anchor.
pub fn embeds_line_number(anchor: &str) -> bool {
    fn is_word(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
    }
    let b = anchor.as_bytes();
    for i in 0..b.len() {
        if b[i..].starts_with(b"line") && (i == 0 || !is_word(b[i - 1])) {
            let mut j = i + 4;
            while b.get(j).is_some_and(u8::is_ascii_whitespace) {
                j += 1;
            }
            if matches!(b.get(j), Some(b':') | Some(b'=')) {
                j += 1;
            }
            while b.get(j).is_some_and(u8::is_ascii_whitespace) {
                j += 1;
            }
            if b.get(j).is_some_and(u8::is_ascii_digit) {
                return true;
            }
        }
        if b[i] == b'.' {
            let mut j = i + 1;
            while j - i <= 5 && b.get(j).is_some_and(u8::is_ascii_alphabetic) {
                j += 1;
            }
            if j > i + 1 && b.get(j) == Some(&b':') && b.get(j + 1).is_some_and(u8::is_ascii_digit)
            {
                return true;
            }
        }
        if b[i] == b':' {
            let mut j = i + 1;
            while b.get(j).is_some_and(u8::is_ascii_digit) {
                j += 1;
            }
            if j > i + 1 && b.get(j) == Some(&b':') && b.get(j + 1).is_some_and(u8::is_ascii_digit)
            {
                return true;
            }
        }
    }
    false
}

/// A probe id is joined onto the scratch dir as ONE path segment, so it is constrained by an
/// ACCEPTED charset, never by a rejected one: `..`, `.`, `/` and a leading `/` are each a way
/// out of the container, and a blacklist that names them is a list of the ways someone has
/// thought of. Ids are also the block's row keys and the digest's ordering keys.
pub fn is_plain_name(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Same rule, one level up: a manifest path is joined onto the workspace root AND onto the
/// scratch dir, so every component must be a plain name — no root, no prefix, no `..`, no `.`.
pub fn is_workspace_relative(path: &str) -> bool {
    !path.is_empty()
        && std::path::Path::new(path)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// The option NAME a token carries: `--test-threads=4` -> `--test-threads`, `-Zfoo` -> `-Z`.
pub fn option_name(arg: &str) -> &str {
    if arg.starts_with("-Z") {
        return "-Z";
    }
    arg.split_once('=').map_or(arg, |(k, _)| k)
}

/// §7 step 1. Everything refusable before the tree is touched or a target is built.
pub fn validate(m: &Manifest) -> anyhow::Result<()> {
    if m.version != 1 {
        bail!(
            "probe-pin: manifest version = {} is unknown; this probe-pin speaks schema version 1 only. Aborting.",
            m.version
        );
    }
    if m.target.mode != RunMode::RuntimeRead {
        return Err(Abort::UnsupportedMode {
            mode: m.target.mode,
        }
        .into());
    }
    for arg in &m.target.args {
        if RESERVED.contains(&option_name(arg)) {
            return Err(Abort::ReservedArg { arg: arg.clone() }.into());
        }
    }
    if m.target.timeout_secs == 0 {
        bail!("probe-pin: [target].timeout_secs = 0 disables `timeout` entirely — `timeout -k 5 0` runs the child to completion, so a hung target would never be killed and never named. Set a positive value. Aborting.");
    }
    // The block is the artifact this whole pipeline exists to produce, and `--check` re-verifies
    // it by resolving `[output].file` against the workspace root. `root.join("../x")` escapes,
    // and a pin written outside the workspace is a pin no checkout and no CI job can ever
    // re-measure — the verification loop it was rendered for cannot reach it. Measured: exit 0,
    // block spliced into a file outside the root. Same rule already governs every mutation path.
    if !is_workspace_relative(&m.output.file) {
        bail!("probe-pin: [output].file '{}' is not workspace-relative. The block is resolved against the workspace root, so a path with '..', a root, or a prefix writes the pin outside the repository — where `probe-pin check` and CI can never re-measure it, which is the one thing a pin exists to allow. Name it relative to the workspace root. Aborting.", m.output.file);
    }
    if m.probes.iter().filter(|p| p.mutations.is_empty()).count() != 1 {
        return Err(Abort::ControlMissing.into());
    }
    for (i, p) in m.probes.iter().enumerate() {
        if !is_plain_name(&p.id) {
            bail!("probe-pin: probe id '{}' is not a plain name. A probe id is joined onto probe-pin's scratch directory as ONE path segment, so a separator or a '..' in it materializes the mutant outside that directory — inside the tree probe-pin is measuring, which is the one thing this tool must never write. Use ASCII letters, digits, '_' and '-'. Aborting.", p.id);
        }
        if m.probes[..i].iter().any(|q| q.id == p.id) {
            bail!("probe-pin: duplicate probe id '{}'. Probe ids are the block's row keys and the digest's ordering; they must be unique. Aborting.", p.id);
        }
        // The control materializes no mutant, and `assert_count` is checked against the mutant
        // text — so one declared here is pinned into the digest and never evaluated. Measured:
        // a control carrying `count = 99` of a string that occurs nowhere exited 0, green.
        if p.mutations.is_empty() && !p.assert_counts.is_empty() {
            bail!("probe-pin: {} declares assert_count(s) with no mutation. assert_count is checked against the MUTANT text, and a probe with no mutation is the control — it never materializes one, so these would be pinned into the block's digest without ever being evaluated. Move them to the probe whose mutation they describe. Aborting.", p.id);
        }
        // An empty file list is not a mutation: it seeds no file, so `mutate::apply`'s per-file
        // no-op gate never executes, nothing is mounted, and the probe's verdict is about the
        // UNMODIFIED tree — the control's job, rendered as a `pass` row that claims a mutant was
        // visible. Refused per MUTATION so it cannot ride along beside one that names a file.
        for (index, mutation) in p.mutations.iter().enumerate() {
            if mutation.files().is_empty() {
                bail!("probe-pin: {} mutation[{index}] names no file. A mutation with an empty file list mutates nothing and mounts nothing, so this probe would measure the unmodified tree and render it as a mutant's verdict. Name the file(s) to mutate, or delete the probe. Aborting.", p.id);
            }
        }
        for file in p
            .mutations
            .iter()
            .flat_map(Mutation::files)
            .chain(p.assert_counts.iter().map(|a| a.file.as_str()))
        {
            if !is_workspace_relative(file) {
                bail!("probe-pin: {} names '{file}', which is not a workspace-relative path. Manifest paths are joined onto the workspace root AND onto probe-pin's scratch directory, so an absolute path or a '..' component reads outside the workspace and materializes the mutant outside the scratch directory — inside the tree probe-pin is measuring. Use a path relative to the workspace root, with no '..' component. Aborting.", p.id);
            }
        }
        if let Expect::Fail { anchor } = &p.expect {
            if anchor.is_empty() {
                bail!("probe-pin: {} expects outcome = \"fail\" with an empty anchor list. A failure with no anchor pins nothing — any failure would satisfy it. Aborting.", p.id);
            }
        }
        for a in p.anchors() {
            if embeds_line_number(a) {
                return Err(Abort::AnchorEmbedsLineNumber {
                    probe: p.id.clone(),
                    anchor: a.clone(),
                }
                .into());
            }
        }
        if let Some(twin) = m.probes[..i]
            .iter()
            .find(|q| !q.anchors().is_empty() && q.anchors() == p.anchors())
        {
            return Err(Abort::AnchorNotDiscriminating {
                probe: p.id.clone(),
                collides_with: twin.id.clone(),
            }
            .into());
        }
    }
    Ok(())
}
