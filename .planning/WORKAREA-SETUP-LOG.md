# Work-area setup log — phase-rs/phase contribution

Date: 2026-06-23
Working dir: `/home/lgray/vibe-coding/phase-rs-workdir`

## Work area (DONE)

- Cloned fork `git@github.com:lgray/phase.git` into the working-dir root.
- Remotes:
  - `origin`  → `git@github.com:lgray/phase.git` (fork)
  - `upstream` → `git@github.com:phase-rs/phase.git` (canonical)
- Fetched `upstream` (all branches + tags through v0.4.0).
- Local git identity set (repo-local): `Lindsey Gray <lagray@fnal.gov>`.
- SSH auth to GitHub verified as user `lgray`. NOTE: `gh` CLI is NOT logged in
  (PR-status queries need `gh auth login` first).
- Retrieved `.planning/` from `origin/lgray-planning` (head `9122733` "planning
  information") via `git archive origin/lgray-planning -- .planning | tar -x`
  into the working tree. 69 files. **It is gitignored (`.gitignore` line 8) on
  every branch**, so it can NEVER appear in a PR — requirement satisfied with no
  extra action.

## State notes
- Fork `origin/main` **synced to `upstream/main`** (`ae663ee8c`) on 2026-06-23
  via `git push origin upstream/main:main` (was 267 behind). Local `main` ff'd to
  match. (`ssh_askpass` noise on push is benign — key auth succeeds.)

## PR cross-reference (2026-06-23, vs phase-rs/phase, author lgray)

### combo-detection — planning was STALE; all three landed
- PR-0 #4092 MERGED ✓, PR-1 #4097 MERGED ✓ (planning correct).
- **PR-2 #4119 is now MERGED** (2026-06-22T18:27:36Z, merge commit `b52860370`).
  PROGRESS.md still calls it "OPEN, unmerged" — OUT OF DATE.
  Verified `analysis::loop_check::detect_loop` + `corpus_tests.rs` are in
  `upstream/main`. ⇒ **PR-3 branches off `upstream/main`** (per the plan's own
  rule "base off upstream/main if #4119 merged"), not off `feat/combo-detect-pr2`.

### Open PRs ↔ saved-wip (all CI-GREEN but reviewDecision=CHANGES_REQUESTED)
| PR | branch | saved-wip dir | needs |
|----|--------|---------------|-------|
| #4186 | card/msh-modal-choose (Ruinous Wrecking Crew, "msh-e") | `saved-wip/ruinous-pr1/` | resolve review comments |
| #4182 | card/msh-doctor-doom | `saved-wip/doctor-doom-4182/` | resolve review comments |
| #4169 | card/msh-intervening-if (Hulkling, "msh-b") | `saved-wip/reentry-4169/` | resolve review comments |

All three: every CI check SUCCESS (Draft pools SKIPPED); blocker is human/AI
review changes-requested, not CI. These are the ready-to-resume work items.

### Card-coverage rollouts — snapshots now STALE
Plans (`std`, `msh`, `modern-commander`) are snapshotted @ `c55670fd0`; main has
since advanced to `ae663ee8c`. **60** `card/std*`+`card/msh*` PRs are MERGED, so
the unsupported-count tables (std 268, MSH 13, modern∩commander 1,551) are
outdated — re-run `coverage-analysis/coverage-breakdown.sh` before trusting them.
- #3822 `card/triumphant-chomp` is CLOSED (not merged) — only non-merged historical PR.
- `.planning/lgray-planning` branch itself is just a snapshot for transporting
  the gitignored planning dir; it is not a work branch.

## Planning landscape (what's in `.planning/`)

### combo-detection/ — PRIMARY ACTIVE THREAD
Infinite-combo detector. See `PROGRESS.md` (status board) + `IMPLEMENTATION.md`
(spec) + `FEASIBILITY-AND-PLAN.md` (theory).
- PR-0 (`ResourceVector`) — MERGED (#4092, `53bff896a`).
- PR-1 (analysis sim harness) — MERGED (#4097, `8a199028d`).
- PR-2 (net-progress detector + 53-row corpus, 10/49 driven) — **OPEN, under
  review**: PR #4119, branch `feat/combo-detect-pr2`, head `13a2c2ea6` (rebased
  onto upstream/main `f421952150`; both maintainer [HIGH] detector-correctness
  blockers + controller-only-damage [HIGH] fixed; diff = 4 `analysis/` files).
- **PR-3 = NEXT WORK**: live loop shortcut — wire `analysis::loop_check::
  detect_loop` into the real `loop_window`/`emit_resolution_halt` path
  (`game/engine.rs`). First PR that changes gameplay → soundness-critical. Base
  off `upstream/main` if #4119 merged, else off `feat/combo-detect-pr2`.
- PRs 4–8 not started (static ability-graph, `cargo combo-verify` CLI, `∞`
  display, opponent-response window, AI coupling).

### Card-coverage rollouts (hard rule from user: EVERY card implemented, no defer)
- `std-rollout/PLAN.md` — standard-legal: 268 unsupported, all clustered.
- `msh-rollout/PLAN.md` — Marvel set MSH: 13 unsupported, all clustered.
- `modern-commander-rollout/PLAN.md` — Modern ∩ Commander: 1,551 unsupported.
- `coverage-analysis/` — the scripts (`coverage-breakdown.sh`, `cluster-assign.sh`)
  + measured `out/` artifacts backing the rollout clusters. Snapshot @ `c55670fd0`.

### saved-wip/ — paused per-PR working dirs (investigations, fix-logs, PR bodies)
- `doctor-doom-4182/` (MSH Doctor Doom), `reentry-4169/` (MSH re-entry /
  intervening-if), `ruinous-pr1/` (MSH "e").

## Coverage regen (2026-06-23) — environment bring-up

This clone had NO Rust toolchain capable of building the repo: `Cargo.toml:1`
declares `cargo-features = ["codegen-backend"]` (nightly-only manifest feature);
system stable cargo 1.96.0 rejects the manifest (measured `exit=101`). Also no
`jq`. Brought up:
- Installed `rustup` (official installer, `--default-toolchain none`).
- Installed pinned `nightly-2026-04-19` (rustc 1.97.0-nightly) with rustfmt,
  clippy, rustc-codegen-cranelift-preview, target wasm32. `rustup show` confirms
  `rust-toolchain.toml` override active; `cargo metadata` now parses.
- Installed `jq` 1.7.1 static binary to `~/.local/bin` (no sudo).
- Use `export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"` for all builds.

KNOWN GOTCHA hit (matches PROGRESS.md note): `fetch-token-sets.sh` jq step errors
("No token-bearing set codes found") → empties tracked `crates/engine/data/
known-tokens.toml`. Non-fatal for card-data/coverage-data. **Restore it from HEAD
after gen** (`git checkout -- crates/engine/data/known-tokens.toml`). Data files
are produced against current `upstream/main` `ae663ee8c`.

## Coverage re-measure results (2026-06-23 @ `ae663ee8c`)

Data regenerated against current main; `coverage-breakdown.sh` re-run for all three
groups. Tables in std/msh/modern-commander PLAN.md updated with old→new columns.

| group | members | supported | unsupported (parser-gap + resolver) | vs c55670fd0 |
|---|---|---|---|---|
| standard | 4924 (+277) | 4647 (94.37%) | **277** (245+32) | was 268 (239+29) → +9 net |
| MSH | 286 (=) | 277 (96.85%) | **9** (6+3) | was 13 (10+3) → −4 |
| modern∩commander | 22,943 (+274) | 21,387 (93.22%) | **1,556** (1,380+176) | was 1,551 → +5 |

Key reads:
- std + modern∩cmd pools GREW faster than they were cleared (new set releases), so
  unsupported is flat/slightly up despite 60 merged card-PRs — the fixes are real.
- MSH dropped 13→9. The remaining 9 include exactly the 3 OPEN PRs (#4186 Ruinous
  Wrecking Crew, #4182 Doctor Doom, #4169 Hulkling); landing them → 6.
- Caveat: token-set fetch jq step failed (known gotcha) → tokens-gen saw 0 sets, so
  token-preset coverage may be marginally off for token-producing cards. Core oracle
  parse + coverage unaffected; counts are sound for parser-gap/resolver classes.
- Per-card CLUSTER assignments in the plans still reflect the c55670fd0 unsupported
  set — re-run `cluster-assign.sh <group>` to refresh those before dispatching work.

## Cluster-assignment refresh (2026-06-23) — plans ready to restart

Re-ran `cluster-assign.sh` for `standard` and `modern+commander` on the fresh
unsupported lists; rewrote the cluster tables in both PLAN.md (counts + fresh
representative cards, since some old representatives were since-implemented).

- **standard**: 277 cards, 0 unclustered. Tiers re-tallied: Tier1 **116** / Tier2
  **157** / Tier3 **4** = 277. Notable shifts: S01 17→18, R2 10→11, S03 6→7,
  S17 3→4, S19 23→24, S10 22→24, R5 10→11, + new **R4** (can't-restriction
  static, 1). Heavy-flagged cards (Quick Draw, Vraska the Silencer, Zimone
  Paradox Sculptor, Doppelgang) all re-verified present.
- **modern+commander**: 1556 cards, 0 unclustered. M-clusters re-measured:
  M1 flip 16, M2 repeat 15, M4 level-up 21, M5 speed 9 (all stable); **M3
  as-long-as 46→81** (old 46 was a Modern-single-gap hand-measure; 81 is the full
  intersection `R2` cluster). Shared-block table refreshed (C1 intersection
  89→76, Condition_If 304→306, DynamicQty 119→112).
- **Ruleset extension (the only code change):** `cluster-assign.sh` gained two
  rules so 3 previously-S99 cards get homes (invariant: 0 unclustered):
  `AlternativeKeywordCost` → **S26-alt-keyword-cost** (Heart of Kiran, New
  Perspectives); `trigger-subject` → **S27-trigger-subject-anaphora** (Psychic
  Possession). All `.planning/` (incl. this script) is gitignored — never in a PR.

## Not done (per instruction "do not start any work")
- No feature branch created, no code changes, no pushes, no fork-main sync.

## Tilt / Kubernetes infra investigation (2026-06-23)

User asked to "understand how it wants to use tilt and kubernetes, setup the
necessary infrastructure" (docker + tilt + kubectl present; KIND/k8s absent).

**MEASURED FINDING — the project does NOT use Kubernetes at all.**
- `Tiltfile` (only one in repo, root) uses *exclusively* `local_resource(...)`.
  Zero `k8s_yaml`, zero `k8s_resource`, zero `docker_build` calls.
- Repo-wide grep for `k8s_yaml|k8s_resource|docker_build|kubectl|kind create|
  kubernetes|helm|minikube|k3d` (excl. node_modules/target/data) → **no real
  hits**; only coincidences (pnpm-lock hashes, `decision_kind.rs`,
  `DOCKER_BUILD_RECORD_UPLOAD` in release.yml).
- No k8s/kind manifests anywhere. `project-reference` SKILL.md documents Tilt as
  a pure local-process orchestrator; setup path is `./scripts/setup.sh` +
  `tilt up`. No cluster step documented.
- Tilt v0.37.4: a Tiltfile with only `local_resource` does **not** require a
  Kubernetes context. KIND is therefore **unnecessary** for this repo.
  (`kubectl config current-context` → "not set"; `kind` not installed.)

**The actual "necessary infrastructure" for `tilt up` is the local toolchain.**
Tilt resources and their deps:
| resource | command | dep present? |
|---|---|---|
| wasm | build-wasm.sh → `cargo build --target wasm32` + `wasm-bindgen` | nightly ✅, wasm32 target ✅, **wasm-bindgen CLI ❌** |
| frontend/test-frontend/check-frontend/tauri | `pnpm ...` | **pnpm ❌** (node v26.2.0 ✅) |
| build-native/test-engine/test-ai | `cargo nextest run ...` | **cargo-nextest ❌** |
| server/tauri | `cargo build` | ✅ |
| card-data | gen-card-data.sh (jq) | jq ✅ |
| clippy/coverage | cargo | ✅ |

Toolchain gaps to fill (instead of KIND): `cargo-nextest`, `wasm-bindgen-cli`
(version-matched to the `wasm-bindgen` dep in Cargo.lock), `pnpm` (via corepack
or standalone). `wasm-opt` optional (release only). PATH needs
`$HOME/.cargo/bin:$HOME/.local/bin`.

### DONE (2026-06-23) — user chose "Local toolchain only" (no KIND)
- **cargo-nextest 0.9.138** → prebuilt tarball from `https://get.nexte.st/latest/linux`
  into `~/.cargo/bin`.
- **wasm-bindgen 0.2.121** → prebuilt musl tarball from rustwasm release
  `0.2.121/wasm-bindgen-0.2.121-x86_64-unknown-linux-musl.tar.gz` into
  `~/.cargo/bin`. **Version-matched** to `Cargo.lock` `wasm-bindgen 0.2.121`
  (mismatched CLI silently corrupts WASM bindings — must stay in lockstep).
- **pnpm 10.34.4** → `npm i -g pnpm@10` after `npm config set prefix ~/.local`
  (system npm prefix `/usr` is not writable). Installs to `~/.local/bin`.
  - **GOTCHA (important):** pnpm **11** dropped reading the `pnpm.overrides`
    field from `package.json` (moved to `pnpm-workspace.yaml`). The repo's v9.0
    lockfile + `client/package.json` `"pnpm": { "overrides": {...} }` layout
    needs **pnpm 10**. With pnpm 11, `pnpm install --frozen-lockfile` fails:
    `ERR_PNPM_LOCKFILE_CONFIG_MISMATCH ... "overrides" configuration doesn't
    match the value found in the lockfile`. **Pin pnpm to 10.x for this repo.**
- PATH is already persistent for the user's interactive **fish** shell: login
  shell (`fish -lc`) resolves `cargo`, `pnpm`, `wasm-bindgen`, `cargo nextest`
  (`~/.local/bin` via config.fish line 19; `~/.cargo/bin` via rustup env).

### Verification (non-invasive — did NOT run full `tilt up`)
- `pnpm install --frozen-lockfile` (client) → **passes** with pnpm 10 (936 lock
  entries, "ignored build scripts" warning is the normal postinstall-approval
  prompt, identical to `setup.sh`). `node_modules` now populated.
- `tilt alpha tiltfile-result` → **`Error: None`, all 14 manifests parse**
  (wasm, frontend, caddy, tauri, server, build-native, test-engine, test-ai,
  test-frontend, clippy, check-frontend, card-data, draft-pools, coverage).
- Deliberately did NOT run full `tilt up`: `card-data` (auto_init) would
  re-download MTGJSON and re-trigger the known `known-tokens.toml`-clobber
  gotcha — unwanted side effects merely to verify tooling. The three installed
  tools ARE the exact commands the resources shell out to, each independently
  proven working, so `tilt up` resources will no longer fail on missing tools.
- **Not done:** no KIND/k8s (project doesn't use it); no `wasm-opt` (release-only,
  optional); full green `tilt up` left for the user to start when ready.

### `tilt up` first-run failure + fix (2026-06-23)
First `tilt up` (launched via harness `!`) failed: `wasm`, `card-data`,
`draft-pools` all died instantly with *"the cargo feature `codegen-backend`
requires a nightly version of Cargo, but this is the `stable` channel."*
- **Root cause:** the harness Bash shell sources a captured
  `~/.claude/shell-snapshots/snapshot-bash-*.sh` whose PATH has `~/.local/bin`
  + miniforge but **NOT `~/.cargo/bin`**. So bare `cargo` → `/usr/bin/cargo`
  (Arch `rust 1.96.0` stable), which rejects `Cargo.toml:1`'s nightly-only
  `cargo-features = ["codegen-backend"]`. (`~/.cargo/env` IS sourced by
  `.bashrc`/`.bash_profile`/`.profile`, but the harness shell is
  non-login/non-interactive and replays only the snapshot.) Login *fish* is fine
  (`conf.d/rustup.fish` puts `~/.cargo/bin` at PATH pos 2 → nightly 1.97.0).
- **Fix:** relaunch `tilt up` with
  `export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"` so the rustup shim
  (honoring `rust-toolchain.toml` nightly-2026-04-19) is used. Verified: `wasm`
  → `in_progress` (compiling) instead of instant error; `card-data`/`draft-pools`
  `pending` behind the cargo lock; `frontend` (vite) serving on :5173; Tilt UI
  on :10350.
- The pre-existing `card-data` jq token-set error ("unexpected as" → "No
  token-bearing set codes found") still fires — documented gotcha, non-fatal;
  `known-tokens.toml` currently intact (229490 lines, clean).

### Two remaining Tilt update errors — diagnosed + fixed (2026-06-23)
After the nightly-PATH fix, `test-frontend` and `draft-pools` were the two
errored resources.

**1. `test-frontend` (440/1526 tests failing)** — `TypeError: Cannot read
properties of undefined (reading 'setItem')` in zustand persist middleware.
- *Root cause:* system `/usr/bin/node` is **v26.2.0**; Node 26 unconditionally
  defines a read-only `globalThis.localStorage` getter that returns `undefined`
  (warns "localStorage is not available because --localstorage-file was not
  provided"). vitest's happy-dom env can't override that read-only global, so
  `() => localStorage` → undefined → `setItem` crash. CI pins **Node 22**
  (`.github/workflows/*.yml`), where `globalThis.localStorage` is undefined AND
  writable, so happy-dom's assignment sticks. Probe confirmed both behaviors.
- *Fix:* installed Node **22.23.0** prebuilt tarball → `~/.local/node22`, and
  relaunch tilt with `~/.local/node22/bin` ahead of `/usr/bin`. Proven: the
  previously-failing `TargetingOverlay.test.tsx` → 17/17 pass under node22; full
  suite → **1508 passed, 0 failed** (was 440 failing).

**2. `draft-pools`** — compiled fine (nightly OK, 1m24s) but exited 1 at runtime:
"no draftable sets found in `data/mtgjson/sets`" (dir existed but **empty**).
- *Root cause:* per-set MTGJSON files are fetched by `./scripts/fetch-draft-sets.sh`,
  which is NOT run by `setup.sh` or `gen-card-data.sh` — a separate manual step.
- *Fix:* ran `./scripts/fetch-draft-sets.sh` (241 sets, 0 failed, ~5 min,
  idempotent skip-if-exists). draft-pools then extracted 17 draftable sets →
  `client/public/draft-pools.json` (7.4 MB). **ok**.

**Restart for both:** killed the nightly-only tilt, relaunched with
`PATH="$HOME/.cargo/bin:$HOME/.local/node22/bin:$HOME/.local/bin:$PATH"` via
`setsid nohup tilt up` (setsid/`</dev/null` avoids the harness process-group
signal that made bare `kill`/`pkill` return exit 144). **Final sweep: 0 errors**
— wasm/card-data/draft-pools/test-frontend/check-frontend ok, clippy compiling,
opt-in resources (server/test-engine/test-ai/tauri/caddy/coverage) not started.

## 2026-06-24 — card-implementation status sync (GitHub → MSH/standard plans)

Task: update card implementation status from GitHub, cross-reference with the
`.planning/msh-rollout` and `.planning/std-rollout` plans.

**Measured facts (GitHub API + local coverage artifacts):**
- Local `origin/main` was **stale** at `ae663ee8c` (SSH fetch needs passphrase, no
  agent). Remote main is **`d3d6b597`** (2026-06-24T03:36Z), ~22 commits ahead —
  confirmed via `gh api repos/:owner/:repo/commits/main`. The plans' "Where X stands"
  tables are the `ae663ee8c` snapshot (data/coverage-data.json + card-data.json
  dated 2026-06-23T11:31).
- **MSH (9 unsupported @ snapshot):** #4186 (Ruinous Wrecking Crew, dynamic modal
  max) **MERGED** `23c50148` → expected 9→8. #4182 (Doctor Doom) **APPROVED**, merge
  pending. #4169 (Hulkling) **CHANGES_REQUESTED**. Cosmic Cube / Hawkeye Young
  Avenger (heavy MSH-F), Hawkeye Master Marksman, Baron Zemo, Loki, Incredible Hulk
  still unassigned.
- **Standard (277 unsupported @ snapshot):** grepped `out/standard/unsupported.tsv`
  for every card named in the merged PRs since `ae663ee8c`. Only **Ruinous Wrecking
  Crew** (via #4186) was in the unsupported set → confirmable delta **−1 (277→276)**.
  #4112/#4193/#4190/#4188/#4191/#4196/#4194/#4167/#4276 are bug-fixes/subsystems on
  already-supported cards (none of their named cards were unsupported). #4202
  (ChoiceType::CardType) is groundwork for the still-open #4182.
- **MSH↔standard overlap:** Doctor Doom, Hawkeye (both), Ruinous Wrecking Crew
  appear in BOTH pools' unsupported sets (resolver-flagged), so MSH PRs move both.

**Edits made:** appended "GitHub delta since snapshot" sections to
`msh-rollout/PLAN.md` and `std-rollout/PLAN.md`, and updated MSH's 9-card status
table. Headline coverage counts deliberately left as the `ae663ee8c` snapshot and
flagged "NOT re-measured at `d3d6b597`" — per the measured-facts rule, no new
coverage numbers were fabricated.

**Re-measure not run this session:** blocked by (a) the GLM/fireconnect router
intermittently dropping the Bash safety classifier ("temporarily unavailable"),
and (b) a full re-measure = building the engine at `d3d6b597` in an isolated
worktree + `gen-card-data.sh` + `cargo coverage` + `coverage-breakdown.sh`
(heavy; would compete with the running Tilt tree). To run it later: fetch
`d3d6b597` (HTTPS public: `git fetch https://github.com/phase-rs/phase.git main`),
`git worktree add` at that ref (don't touch the main tree's `known-tokens.toml`),
then in the worktree with nightly-cargo+node22 PATH: `./scripts/gen-card-data.sh`,
`cargo coverage`, then `.planning/coverage-analysis/coverage-breakdown.sh --set MSH`
and `--format standard`, and `cluster-assign.sh` for each.

## 2026-06-24 — MSH-F pipeline resumed (engine-implementer)

Resuming the MSH-F heavy cluster (Cosmic Cube + Hawkeye, Young Avenger) via the
engine-implementer skill. Plan-gated per .planning/msh-rollout/PLAN.md step 6.

**Baseline investigation (measured):**
- Local HEAD: `ae663ee8c`. True remote main: `9ad977b0cf` (gh api; ssh fetch blocked
  by passphrase so origin/main is stale at ae663ee8c). Local lacks Ruinous #4186
  (not relevant to MSH-F). ~80 commits of remote drift, none re-measured into coverage.
- Both MSH-F cards still unsupported at local ae663ee8c coverage
  (`out/MSH/unsupported.tsv` matches both names).
- Cluster assignment: both `S10-dynamic-qty-bespoke` / handler `Swallow:DynamicQty`
  (crude coverage-system label — planner must derive the real class).
- Relevant primitives already in local tree (traced, not assumed):
  - Black Widow #4184 (`f4053d0e36`) IS an ancestor of local HEAD → impulse-cast
    exiled card until EOT with any-type mana exists. Files: `effects/exile_from_top_until.rs`,
    `effects/exile_top.rs`.
  - `Effect::FreeCastFromZones` (Invoke Calamity class) — types/ability.rs:9212.
  - Replacement infra: `ReplacementEvent::DamageDone`/`DealtDamage`,
    `apply_damage_after_replacement`, `pre_replacement_damage_gate`,
    `replace_combat_damage_batch` (game/replacement.rs, combat_damage.rs, engine_replacement.rs).
- Card Oracle text (out/MSH/oracle-full.tsv):
  - Cosmic Cube: "Ward {2} / Whenever you attack, look at the top six cards of your
    library. You may cast a spell from among them with mana value less than or equal
    to the greatest power among attacking creatures you control without paying its mana
    cost. Put the rest on the bottom of your library in a random order."
  - Hawkeye, Young Avenger: "Reach / If a source you control would deal noncombat
    damage to an opponent or a permanent an opponent controls, instead it deals that
    much damage plus X, where X is Hawkeye's power."

**Plan-gated:** running /engine-planner → /review-engine-plan (unbounded clean loop)
before any implementation, per the heavy-cluster gate.

## 2026-06-24 (later) — main updated + rollout plans RE-MEASURED at a2c3033f8

**Updated main:** fetched upstream (phase-rs/phase), `git merge --ff-only upstream/main`.
Local main `798857711` (#4169) → `a2c3033f8` (#4280). Single incoming commit beyond
the 34-commit FF was client-only (#4280 stack-pacing); verified it does NOT touch
`known-tokens.toml`, so the other agent's local mod to that file is preserved.

**Regenerated coverage data:** waited on Tilt `card-data` resource (status ok,
inputs stamp 2026-06-24T05:22Z) — first REAL re-measure since the merges landed
(prior plan numbers were ae663ee8c snapshot + GitHub-delta estimates).

**Measured results (coverage-breakdown.sh):**
- MSH: 9 → **7 unsupported** (96.85% → 97.55%). Cleared: Doctor Doom (#4182 `2af0f855e`),
  Hulkling (#4169 `798857711`). Remaining 7: Baron Helmut Zemo, Cosmic Cube,
  Hawkeye Young Avenger, Hawkeye Master Marksman, Loki, The Incredible Hulk,
  The Ruinous Wrecking Crew.
- standard: 277 → **274 unsupported** (94.37% → 94.44%). −3 = Doctor Doom + 2 others
  across the FF. parser-gap 245→242, resolver-flagged held 32.

**TRUST-BUT-VERIFY FINDING (prediction refuted):** both plans previously predicted
#4186 (Ruinous Wrecking Crew, "dynamic modal max") would clear that card (277→276).
Measured against fresh data with #4186's code `23c50148a` CONFIRMED an ancestor of HEAD,
Ruinous is **STILL resolver-flagged unsupported** in both pools (gap_count==0 yet
supported==false). The merged primitive did not satisfy the coverage tool's resolver
audit. Recorded in both plans as a follow-up to audit, NOT credited as done.

**Edits:** .planning/msh-rollout/PLAN.md + .planning/std-rollout/PLAN.md headers,
"Where X stands" tables, GitHub-delta sections (rewritten from "since snapshot,
predicted" → "resolved by re-measure, measured"), MSH cluster status marks (B/D done,
E Ruinous flagged), dispatch order. No engine code touched. No commit.

---

## 2026-06-26 — main updated to v0.7.0; rollout plans RE-MEASURED at `5eca83b8c`

**Trigger:** user FF'd local main again (`a2c3033f8` → `5eca83b8c`, release **v0.7.0**,
75 commits) and asked to update MSH + standard rollout plans.

**Freshness check (avoided a needless regen):** `data/card-data.json` mtime showed
`07:47` *local* (CDT −0500) = `12:47Z`; Tilt `card-data` buildHistory finishTime
`12:47:35Z`, right after the FF (reflog: FF at `07:26:44 -0500` = `12:26Z`). So coverage
data was already fresh against v0.7.0 — no regeneration needed. (Initial `ls ...Z` label
was misleading: ls prints local time; the literal `Z` I appended was wrong. Resolved by
cross-checking Tilt buildHistory.)

**Measured (coverage-breakdown.sh, inputs 2026-06-26T13:12Z):**
- **MSH:** 286 / 279 supported (97.55%) / **7 unsupported** (5 parser-gap + 2 resolver-flagged).
  **Identical set** to `a2c3033f8` — v0.7.0 cleared no MSH card. Same 7: Baron Helmut Zemo,
  The Incredible Hulk, Cosmic Cube, Hawkeye Young Avenger, Loki God of Mischief (parser-gap);
  Hawkeye Master Marksman, The Ruinous Wrecking Crew (resolver-flagged).
- **Standard:** 4924 / 4651 supported (94.46%) / **273 unsupported** (241 parser-gap + 32
  resolver-flagged). **Net −1** vs `a2c3033f8` (274); resolver-flagged held at 32, so the
  −1 is one parser-gap card. Top handlers unchanged (Condition_If 76, DynamicQty 37,
  Duration_ThisTurn 15, Effect:for 13).

**Trust-but-verify / honesty notes:**
- Could NOT pinpoint the single cleared standard card: the prior `out/standard/unsupported.tsv`
  was overwritten by this run, so no exact set-diff. Recorded the −1 as the measured headline,
  did NOT credit it to a named PR. #4391 (The Second Doctor) checked — NOT a standard member,
  so not the −1.
- Ruinous (#4186) still resolver-flagged unsupported in v0.7.0 — earlier refuted prediction
  STILL stands; left as a follow-up audit, not credited.

**Edits:** .planning/std-rollout/PLAN.md (header, stands table, Net read, new v0.7.0
GitHub-delta subsection above the a2c3033f8 one) + .planning/msh-rollout/PLAN.md (header,
re-measure note, stands table, current-7 section). No engine code touched. No commit
(.planning never enters a PR).

---

## 2026-06-26 — /remote-control: 3-line parallel orchestration set up

**User directive:** max 3 lines of work in parallel; ONE slot ALWAYS reserved for PR
comments/failed CI. Worktrees per line. Planning @ xhigh, impl/review @ high. When MSH-F
done → next MSH workload; if none → standard highest-priority cluster.

**Model constraint (why the lead drives all sub-agents):** `/engine-implementer` runs in
the MAIN thread because spawned agents cannot spawn sub-agents (planner/executor/reviewer).
So parallelism = each line's CURRENT step runs as a concurrent background agent that I (lead)
spawn; worktrees isolate the executor edits.

**Topology measured:** main worktree = `/home/lgray/vibe-coding/phase-rs-workdir` @
`5eca83b8c` (v0.7.0). No shared cargo target-dir → each worktree gets its own `target/`
(executors run cargo DIRECTLY; Tilt only watches main, no lock contention). Gitignored
static assets (card-data 94M, coverage 56M, engine-inventory, docs/MagicCompRules.txt,
data/mtgjson) live ONLY in main; symlinked into each worktree's data/ + docs/. `.planning/`
is gitignored → main-only → planners read specs from `…/phase-rs-workdir/.planning/`.

**Worktrees created:**
- Line 1 MSH-F  → `/home/lgray/vibe-coding/wt-msh-f`     branch `feat/msh-f-cosmic-cube-hawkeye`
- Line 2 PR-3   → `/home/lgray/vibe-coding/wt-combo-pr3`  branch `feat/combo-detect-pr3`
- Line 3 reserved → no worktree (created on demand for PR/CI work).

**Line 2 fact:** combo PR-2 (#4119) is MERGED to main; `analysis::loop_check::{detect_loop,
LoopCertificate}` present. PR-3 branches off main, NOT off feat/combo-detect-pr2.
PR-3 = live shortcut: wire detect_loop into loop_window/emit_resolution_halt in
crates/engine/src/game/engine.rs (CR 732.2a). Soundness-critical.

**Line 1 cards (exact oracle, from card-data.json @ v0.7.0):**
- Cosmic Cube ({5} Artifact): Ward {2}; "Whenever you attack, look at top six, may cast a
  spell from among them with MV ≤ greatest power among attacking creatures you control
  without paying, rest on bottom random." Class: max-aggregate QuantityRef + impulse/
  look-then-cast (trace Black Widow std S11).
- Hawkeye, Young Avenger ({3} 2/4): Reach; noncombat-damage REPLACEMENT (CR 614) from a
  source you control to opponent/their permanent: "plus X, X = Hawkeye's power."

**Status @ setup:** both planners spawned in background (xhigh). Slot 3 idle — NO open
lgray-authored PRs (`gh pr list --author lgray` empty), so nothing reactive pending yet.
Next: drive each line plan → review-plan (loop) → implement → review-impl (loop) → commit.

---

## 2026-06-26 — RESUME HERE after tmux relaunch (remote-control parallel mode)

The /remote-control 3-line parallel run was blocked: tmux not installed (agent swarms
require it) and sudo needs a password. User chose **Install tmux + go parallel**. After
relaunching Claude Code INSIDE tmux, resume as follows (all state below is on disk):

**Persisted state:**
- Worktrees (clean, off main `5eca83b8c` v0.7.0, with symlinked gitignored assets in
  data/ + docs/MagicCompRules.txt):
  - Line 1 MSH-F  → `/home/lgray/vibe-coding/wt-msh-f`     branch `feat/msh-f-cosmic-cube-hawkeye`
  - Line 2 PR-3   → `/home/lgray/vibe-coding/wt-combo-pr3`  branch `feat/combo-detect-pr3`
- Re-create the task list (was session-scoped): Line1 MSH-F, Line2 combo PR-3, Line3
  RESERVED for failed CI + PR comments (NEVER feature work; keep free).

**Orchestration model (lead = main thread):** `/engine-implementer` runs in the main
thread (spawned agents can't spawn sub-agents). Parallelism = each line's CURRENT step is a
concurrent background agent the LEAD spawns; worktrees isolate executor edits; each worktree
has its own cargo `target/` so executors run cargo DIRECTLY (Tilt only watches main).
Effort: planning xhigh, impl/review high. Constraints: NEVER >3 lines; Line 3 ALWAYS free
for PR comments/CI.

**Next action on resume:** spawn the two PLANNERS as background agents (xhigh), one per
feature line, each invoking `/engine-planner`. Full prompts (cards, anchors, requirements)
are in this session's history; regenerate from:
- Line 1: Cosmic Cube ({5} Artifact, Ward {2} + "look at top six… cast a spell with MV ≤
  greatest power among attacking creatures you control without paying… rest on bottom
  random" — max-aggregate QuantityRef + impulse/look-then-cast, trace Black Widow S11) AND
  Hawkeye, Young Avenger ({3} 2/4, Reach + noncombat-damage REPLACEMENT CR 614 "plus X,
  X = Hawkeye's power" from a source you control to opponent/their permanent).
- Line 2: combo PR-3 LIVE SHORTCUT — wire `analysis::loop_check::detect_loop` into the live
  `loop_window`/`emit_resolution_halt` site in `crates/engine/src/game/engine.rs` (CR
  732.2a); soundness-critical; keep strict CR 104.4b comparator untouched; promote
  mandatory-unbounded corpus rows to driven. Specs: `.planning/combo-detection/{PROGRESS.md
  PR-3 section, IMPLEMENTATION.md §6/§7/§9/§10/§11, FEASIBILITY-AND-PLAN.md}`.
Then drive each line: plan → review-plan (loop until clean) → implement (executor in the
line's worktree) → review-impl (loop) → commit by pathspec. When MSH-F done → next MSH
workload; if none → standard highest-priority cluster (`.planning/std-rollout/PLAN.md`).

---

## 2026-06-26 (resumed in tmux) — parallel run live

tmux 3.7 confirmed, session active. Worktrees/branches/symlinks all intact & clean; no
open lgray PRs (reserved slot free). Task list re-created (Line 1 MSH-F in_progress, Line 2
PR-3 in_progress, Line 3 reserved pending).

**Spawned (background, opus, xhigh):**
- `mshf-planner` → /engine-planner for MSH-F (Cosmic Cube + Hawkeye, Young Avenger) in wt-msh-f.
- `pr3-planner`  → /engine-planner for combo PR-3 live shortcut in wt-combo-pr3.

Next: on each planner return → spawn /review-engine-plan (fresh agent) → loop until clean →
engine-implementation-executor in that line's worktree → /review-impl loop → commit by
pathspec. Line 3 held for reactive PR/CI work.

## 2026-06-26 — both plans delivered + in plan-review r1
- Line 1 MSH-F plan → .planning/msh-rollout/MSH-F-PLAN.md (2 sub-plans: A Cosmic Cube parser-only;
  B Hawkeye DamageModification::Plus field-lift u32→QuantityExpr + resolver + parser). No new variant.
  Open decision flagged: Plus-only vs lift Minus/SetTo too (~40 sites). Reviewer: mshf-plan-reviewer-r1.
- Line 2 PR-3 plan → .planning/combo-detection/PR3-PLAN.md (live shortcut at run_auto_pass_loop;
  new pub(crate) live_mandatory_loop_winner in loop_check.rs; reuse GameOver event, no new variant;
  single-faller/2-player/life-axis firewall). Reviewer: pr3-plan-reviewer-r1.
- Both reviewers background/opus/xhigh. Reserved slot free (no lgray PRs).

## 2026-06-26 — STOP BOUNDARIES (user directive)
- Line 1 (MSH): MSH-F → complete ALL remaining MSH unsupported cards → HARD STOP, wait for
  explicit user word before starting standard. Overrides the earlier "if none, switch to
  standard" default. Remaining MSH after MSH-F: Baron Helmut Zemo, Loki God of Mischief,
  The Incredible Hulk, Hawkeye Master Marksman, The Ruinous Wrecking Crew (audit).
- Line 2 (combo): PR-3 → HARD STOP, do NOT start PR-4 until explicit user word.
- Also: cull each spawned background agent immediately after capturing its output
  (SendMessage shutdown_request; TaskStop does not work on Agent-tool agents).

## 2026-06-26 — Line 1 → implementation; Line 2 → plan-review r2
- Line 1 MSH-F: plan-review CLEAN (both sub-plans, round 2; MSH-F-REVIEW-r2.md). 4 LOW NEW
  findings passed to executor as fix-constraints (NEW-1 CR 122 not 121 prose; NEW-2 &value;
  NEW-3 take_until-alt pseudocode→scan helper; NEW-4 rest greediness note). Executor
  mshf-executor-r1 running in wt-msh-f: Cosmic Cube (parser-only) then Hawkeye (Plus field-lift).
  Executor runs cargo DIRECTLY (worktree not Tilt-watched, own target); must NOT regen
  card-data.json (symlink to main) — verify via unit tests. Lead commits (2 pathspec commits).
- Line 2 PR-3: round-2 reviser self-discovered BLOCKER 0 — modulo scan can never match a
  mandatory trigger cascade (fresh ObjectId per trigger ⇒ stack never eq), original design was
  dead code. Fix: positional stack-id canonicalization in project_out_resources (modulo path
  ONLY; strict comparator untouched). Plus resolved BLOCKER 1 (cant-lose/cant-win firewall via
  pub(crate) player_has_cant_lose + player_has_cant_win), BLOCKER 2 (rows 17/18 not 1/2),
  HIGH (all-living-players no-meaningful-action gate), MEDIUM (BTreeMap get-unwrap_or). Round-2
  reviewer pr3-plan-reviewer-r2 focused on BLOCKER 0 fix soundness (new false-positive vector?).

## 2026-06-26 — BOTH lines in implementation (peak parallelism)
- Line 2 PR-3 plan-review CLEAN (round 2; PR3-REVIEW-r2.md): BLOCKER 0 fix ruled SOUND
  (positional stack-id canon in project_out_resources compares entry CONTENT via im::Vector;
  raw cross-refs left as-is = fail-safe; strict comparator confirmed untouched). All round-1
  findings resolved. 4 LOW fix-constraints → executor (drop CR 810.8a 2HG cite; §7 in-kind
  enumeration; perf note; loop_check module-doc update). Scope note: §7 touches shared PR-2
  code → full analysis:: suite is the no-regression gate.
- Executors running concurrently in disjoint worktrees (separate target dirs, no contention):
  mshf-executor-r1 (wt-msh-f) + pr3-executor-r1 (wt-combo-pr3). Both: cargo direct (not Tilt),
  do NOT regen symlinked card-data/engine-inventory, Step-0 non-vacuity measurement first,
  revert-proofs required, lead commits by pathspec. Reserved slot free.

## 2026-06-26 — Line 2 PR-3 STOP-AND-RETURN (plan-vs-code), re-planning; Line 1 impl-fix
- Line 1 MSH-F impl-review r1: VERDICT A CHANGES REQUIRED (1 HIGH — A2 deferral overruled with
  measured evidence: Cosmic Cube is the FIRST card routing Ref(Aggregate) through
  cast_permission_constraint_allows_cast at finalize; all prior dynamic ceilings are computed
  Fixed → zero runtime coverage; focused unit test required). VERDICT B Hawkeye CLEAN.
  mshf-executor-r2 adding the focused finalize-seam unit test (MSH-F-IMPL-REVIEW-r1.md).
  All 9 prior tests pass + clippy clean. Both commit together once A clean.
- Line 2 PR-3 STOP-AND-RETURN (PR3-IMPL-STOP-r1.md): Step-0 measurement found run_auto_pass_loop
  is UNREACHABLE for the target self-refilling same-controller cascades (UntilStackEmpty session
  dies on transient empty stack; UntilEndOfTurn ends on opponent-observed trigger) → never
  reaches FINGERPRINT_AFTER_ITERS=32. §7 makes the comparator able to match but the site is never
  reached → live win dead code. Executor correctly STOPPED (no improvised redesign); §7/§8/§9
  building blocks sound + unit-tested; did NOT promote corpus rows (would be vacuous).
  CORRECTION: real drive path resolve_all_fast_forward is ENGINE-owned
  (game/engine_resolve_batch.rs:38), NOT the WASM bridge → PR-3 viable, hook must relocate.
  pr3-planner-r3 re-planning: find reachable engine site, reuse §7/§8/§9, re-establish soundness
  arg for the new site, redo Step-0 at the real path. ESCALATE-TO-USER if the only reachable site
  can't host a sound engine-owned shortcut.

## 2026-06-26 — MSH-F COMMITTED (local); Line 1 → next MSH cluster (MSH-E modal)
- MSH-F DONE: re-review VERDICT A CLEAN (reviewer's own revert-proof: Attacking-filter swap → Some(4) reject assertion failed → restored byte-exact). Full suite 15592 pass/0 fail, clippy+fmt clean.
  Committed on branch feat/msh-f-cosmic-cube-hawkeye (in wt-msh-f), 2 pathspec commits:
  - 906cb1ee1 feat(parser): Cosmic Cube (oracle_effect/mod.rs + casting.rs finalize test)
  - a587f9178 feat(engine): Hawkeye, Young Avenger (8 files: ability/replacement/add_target_replacement/oracle_replacement/oracle_trigger + 3 integration tests)
  NOT pushed (held for user's word, like the standard boundary). Ready to ship via /ship-commits.
  Main-side follow-ups (post-merge): cargo engine-inventory regen (Plus u32→QuantityExpr); cargo coverage/semantic-audit to confirm the 2 DynamicQty gaps close.
- Line 1 next: per user "complete any remaining MSH work". Remaining 5: Baron Helmut Zemo, Loki,
  The Incredible Hulk, Hawkeye Master Marksman, The Ruinous Wrecking Crew. Started MSH-E modal
  cluster (Hawkeye Master Marksman + Ruinous audit) on new branch feat/msh-e-marksman-ruinous
  (off main, in wt-msh-f; MSH-F commits preserved on their branch). Both resolver-flagged
  (choose-up-to-dynamic modal). mshe-planner running.

## 2026-06-26 — PR-3 ESCALATION resolved: user chose Option C (GameState detection ring)
- Round-3 re-plan (pr3-planner-r3) measured the deeper contradiction: PR-3's "engine halts these
  loops" premise is MEASURED-FALSE — at realistic life idx 17/18 already end correctly via the
  0-life SBA (CR 704.5a). Neither engine loop reaches them: run_auto_pass_loop dies on session
  interrupts; resolve_all_fast_forward (engine-owned, sound) only engages at stack depth ≥10 but
  these stay depth 1; real drive = frontend per-beat single apply(PassPriority), one resolution/
  call, NO cross-call window. Escalated A/B/C/D to user.
- USER CHOSE C: persist a bounded loop-detection ring on GameState so detection accumulates across
  apply() calls (driver-independent). High scope: serialized-surface, per-resolution maintenance,
  SBA-ordering, soundness re-derivation for the per-beat drive. Its own PR (≈PR-7).
- pr3-planner-c designing full C: compact ring (minimize serde surface — prefer transient/skip),
  maintenance at single-resolution site, detection via reused §7 canon + §8 winner + §9 gate,
  GameOver emit, SBA-ordering (CR 704), Step-0 non-vacuity driving the REAL per-beat loop, dispose
  dead §10. §7/§8/§9 building blocks reused verbatim (still uncommitted in wt-combo-pr3).

## 2026-06-27 — MSH-F SHIPPED (PR #4471); MSH-E rebased + r2 planning; PR-3 r3 review
- MSH-F verified GREEN on v0.8.0 base: targeted unit tests (cosmic_cube 4/0, plus_dynamic_source_power
  2/0, plus_legacy 1/0) + full lib suite 13812 passed / 0 failed (no v0.8.0 regression). Cherry-picked
  906cb1ee1+a587f9178 onto upstream/main ac1917cf5 → ship/cosmic-cube-hawkeye-msh (fbb0c72cf+aa4e88ec4),
  clean. Pushed to origin fork; PR phase-rs/phase#4471 open; CI engaged (WASM/Tauri/lobby pass, rust
  lint/tests/card-data pending).
- MEASURED CORRECTION: lgray CANNOT self-enqueue on phase-rs/phase. `gh pr merge --auto` → 403
  EnablePullRequestAutoMerge; collaborator-permission API → 403; org-membership → 404. lgray is an
  EXTERNAL contributor; prior fork PRs (#4186 etc.) merged via MAINTAINER review + `status:ready-to-merge`
  label, not self-enqueue. Ship endpoint for me = push → PR create → confirm CI engaged → DONE; then
  monitor `gh pr checks` for failures (reserved CI line). Memory ship-finished-prs-as-completed updated.
- MSH-E (Hawkeye Master Marksman + Ruinous coverage) branch feat/msh-e-marksman-ruinous had NO own
  commits (tip 5eca83b8c is an ancestor of aa4e88ec4 via v0.7.0→v0.8.0 lineage), so clean
  fast-forward onto aa4e88ec4 (= v0.8.0 + MSH-F) — shared ability.rs edits now stack on MSH-F.
- mshe-planner-r2 (xhigh) producing MSH-E-PLAN-r2.md: fold MSH-E-REVIEW-r1 (BLOCKER A1 = detector
  needs `"modal":{` node gate to avoid regressing Temporal Firestorm + Heroic Feast; A2 key on absence
  of `"dynamic_max_choices":{`; B2 re-point seams to up-front optional gate + repeated_full_chain branch,
  scope NEW Fixed-count per-iteration-optional+early-stop driver first-class; B3 flag-clear at new loop)
  + RE-VERIFY all line numbers on aa4e88ec4 (old cites from 5eca83b8c are stale).
- Line state: L1 = MSH-E r2 plan; L2 = PR-3 Option C plan review r3 (pr3-plan-reviewer-c, stub log,
  pinged for status); L3 = reserved (CI/PR comments). Within 3-worker cap.

## 2026-06-27 (cont.) — MSH-F APPROVED; MSH-E r2 in review; PR-3 STOP#2 (two measured defects)
- MSH-F PR #4471: maintainer-APPROVED + `status:ready-to-merge` + mergeStateStatus CLEAN; will merge via queue. CI fully green (lint/clippy/parser-gate, both test shards, card-data coverage-gate, frontend, WASM, Tauri). I cannot self-enqueue (external contributor) — maintainer/queue handles merge. Ship worktree auto-prunes on next ship run.
- MSH-E-PLAN-r2 written (mshe-planner-r2, culled): A1 BLOCKER resolved via measured `"modal":{` gate (Heroic Feast + Temporal Firestorm have NO modal node → stay green, zero regressions across the 10-card "choose up to (x|that many)" class); A2/A3 confirmed; all B seams relocated on aa4e88ec4 (r1's drive_repeat_for_outermost cite was wrong — its gate needs player_scope||unless_pay which Hawkeye lacks; real path = resolve_chain_body up-front gate 5403 → resolve_optional_effect_decision 1733 → repeated_full_chain 5800-5922); NEW Fixed-count per-iteration-optional driver scoped first-class; B4 parser parameterized DynamicCostX→Dynamic{qty}; planner caught r1 CR error (700.2e is "other-than-controller chooses", not "illegal mode" → use 700.2b). In adversarial r2 review (mshe-plan-reviewer-r2, high).
- PR-3 Option C STOP-AND-RETURN #2 (pr3-executor-c1, pane-killed). LOW-3 Step-0 gate found TWO architectural defects all 3 reviews missed: **Defect-1** §2 maintenance at handle_priority_pass_with_limit is INERT (refill trigger placed by run_post_action_pipeline AFTER seam returns → stack shrunk at sample → gate never fires → ring 0/200 beats, P1 drained 200→150); **Defect-2** §3 detection at reconcile_terminal_result + §9 gate recurse via SimulationFilter's nested apply_as_current (legal_actions) → SIGABRT once ring populates. Executor shipped plan-as-written INERT+safe (suite 13666/0, clippy clean, serialized-surface ZERO), PROVED Defect-1 fix (relocate to pass_priority_once_with_pipeline after run_post_action_pipeline) makes ring accumulate + find winner P0, did NOT promote DRIVEN_ROW_INDICES, did NOT ship vacuous C-tests. This validates the hard gate. Re-plan in flight (pr3-replanner-defectfix, xhigh): Defect-1 proven fix + resolution-occurred gate refinement; Defect-2 decision among re-entrancy-guard/probe-suppress (MUST keep reconcile-seam detection so per-beat catches it; option (b) site-move likely reintroduces the per-beat wall). Win PROVEN to work → re-plan, not pivot. PR-4 still gated.
- Lines: L1 = MSH-E r2 review; L2 = PR-3 defect re-plan; L3 = reserved. Roster clean (team-lead, mshe-plan-reviewer-r2, pr3-replanner-defectfix).

## 2026-06-27 (cont.2) — MSH-E impl A+B running; PR-3 defect-fix approved → re-implementing
- MSH-E: r2 review CHANGES-REQUIRED (0 blocker/2 high/3 med/2 low), all addressed — A1 BLOCKER confirmed fixed (modal-node gate, zero green regressions); HIGH-1 B3 RESOLVED (reflexive sub effect=GenericEffect → WhenYouDo gate :6885 returns true unconditionally → :6886 UNCHANGED, K==0 skip sole authority, direct-sub resolution at depth≥1 — both lead-verified); HIGH-2 B-i test re-targeted (flag-clear discriminates K-accounting not reflexive); MED-1 parser arm must match "choose up to that many." PERIOD form (lead-verified, NOT em-dash, w/ not-noun guard vs selection clauses); MED-2 serde rationale = AbilityDefinitionRepr proxy skip_serializing_if ability.rs:13381; MED-3 new driver first-class + reflexive depth≥1; LOW-1 drop ModalCountSpec Copy. Decisions committed to MSH-E-PLAN-r2 authoritative addendum. mshe-executor-r2 (high) implementing A (greens Ruinous, independently shippable) then B; if B contradicts, stop B keep A. Report → MSH-E-IMPL-REPORT.md.
- PR-3 defect-fix: re-plan (pr3-replanner-defectfix) → review (pr3-defectfix-reviewer) CHANGES-REQUIRED (0 blocker/0 HIGH/2 med/1 low) — BOTH make-or-break decisions SOUND: (1) SimulationFilter::accept filter.rs:113 is the ONLY nested-apply re-entry to reconcile (callers 196/200, exhaustive scan) → RAII guard sufficient; (2) resolved_this_beat gate correct — fresh monotonic ObjectIds (triggers.rs:3894) so resolution+refill changes top id, handoff never does. Corrections: MED-1 (priority.rs revert MUST restore trailing if/else expr or clippy::let_and_return — lead-verified via git diff HEAD), MED-2 (§3 accel scoped to per-beat drive only, run_auto_pass_loop still 704.5a), LOW-1 (§7 canon resource.rs:751). Committed to PR3-PLAN-DEFECTFIX addendum. pr3-executor-c2 (high) re-implementing: Defect-1 relocate+gate, Defect-2 thread-local guard. HARD GATE: prove live GameOver on idx18 (~beat6) before promoting DRIVEN_ROW_INDICES. Report → PR3-DEFECTFIX-IMPL-REPORT.md.
- Lines: L1 = MSH-E impl A+B; L2 = PR-3 defect-fix re-impl; L3 = reserved. Roster clean (team-lead, mshe-executor-r2, pr3-executor-c2).

## 2026-06-27 (cont.3) — SendMessage skill/agent update SHIPPED (PR #4478)
- User asked (mid-run) to add missing SendMessage to skills for easier sub-agent management. Mapped surface via background workflow (sendmessage-skill-audit, 4 agents): all 4 .claude/agents/*.md lacked SendMessage in tools → could not ack shutdown_request (pane-kill+zombies) nor report-back (disk-only). Mirror nuance MEASURED: .agents/skills + .codex/skills are SYMLINKS to .claude/skills (same inode) so skill edits propagate automatically (my earlier "copies" read was wrong — ls -ld resolved through the symlink); .codex/agents/*.toml are thin wrappers delegating to .md source-of-truth (no tools key) → no Codex change.
- Applied 11 additive edits: SendMessage added to tools of all 4 agent defs + a return-point note each (preserving "final text IS return value" + durable .planning disk reports); graceful-cull flow (shutdown_request→shutdown_response) documented in batch-mechanics / engine-implementer / bug-triage. Verified all landed + propagated.
- User chose Commit+PR. Shipped from worktree off upstream/main (7 config files identical between local main and upstream/main → safe copy, zero clobber). Commit 9e4b63e17 → PR phase-rs/phase#4478 (docs-only; CI engaged; awaiting maintainer ready-to-merge — cannot self-enqueue). Worktree forge.rs-ship-sendmessage-agents.
- Memory cull-finished-subagents updated: change affects FUTURE spawns only; the two in-flight executors (mshe-executor-r2, pr3-executor-c2) predate it → still pane-kill.

## 2026-06-27 (cont.4) — both executors DONE+reviewed; PR-3 SHIPPED (#4480); MSH-E in fix round
- PR #4478 (SendMessage skills): CI fully green, OPEN+MERGEABLE, awaiting maintainer label.
- Both pre-grant executors finished and were pane-killed after reports landed on disk (%23 mshe, %25 pr3); panes identified via `tmux capture-pane` scrollback (both showed main-worktree cwd, identical titles — scrollback was the disambiguator). Lead + reviewers preserved.
- MSH-E (executor report MSH-E-IMPL-REPORT.md): A+B COMPLETE, suite 13827/0, 13 tests w/ 4 measured revert-discriminators, CR 8/8, ai-gate 0 FAIL, no stop-and-return. Independent review (mshe-impl-reviewer, opus, gracefully culled) = CHANGES REQUIRED: **HIGH** = K (`optional_cost_payments_this_resolution`) was `#[serde(skip)]`+eq-excluded but K is nonzero AT the per-iteration OptionalEffectChoice pause (a serde boundary across apply() calls) while the paired `pending_repeated_optional_payment` IS serialized+eq-included → save/restore mid-loop collapses reflexive cap below payments made (CR 700.2d). **I verified the driver code + sibling serde myself** (effects/mod.rs:4053 increment-then-pause; exiled_from_hand_this_resolution 6808 = the correct eq-included analog). MED = predicate admits any PayCost but driver is synchronous-only. 2 LOW. Fresh mshe-fix-executor (post-grant → SendMessage) applying: Fix1 K→`skip_serializing_if=is_zero_u32`+eq-include+doc rewrite; Fix2 narrow via `is_synchronous_mana_pay_cost`; Fix3 Explosive targeted-mode apply test; Fix4 document conservative-RED. Build clean, verifying.
- PR-3 (executor report PR3-DEFECTFIX-IMPL-REPORT.md): HARD GATE PASSED — idx18 wins LIVE GameOver{P0} at beat6 via real apply(PassPriority), no SIGABRT; idx17 also live (target opponent = sole legal target). Suite 13671/0, zero serde-surface delta. Surfaced FINDING: Defect-2 thread-local guard is defensive-depth (recursion doesn't reproduce, measured ×3) — kept guard + honest bounded-termination test. Independent review (pr3-impl-reviewer, opus, culled) = VERDICT CLEAN (0/0/0/1 LOW), all 7 obligations re-measured cargo-direct (spot-reverted §2/§3 discriminators → FAIL confirmed); Defect-2 verdict KEEP (reviewer independently reproduced no-recursion); idx17 MORE robust than reported. LOW (doc "target player"→"target opponent") applied inline (card-data verified: "target opponent loses that much life").
- SHIP PR-3: committed 2d7bcb514 on feat/combo-detect-pr3, but base 5eca83b8c was **53 commits behind** upstream (a6723026c). Cherry-pick onto upstream/main → ONE composable conflict in ai_support/filter.rs: PR-3's SimulationProbeGuard wrap vs #4479's `apply_as_current_for_legality` perf refactor (same legality-probe path). Resolved by composing both (perf fn UNDER the guard); engine.rs/game_state.rs auto-merged clean (verified in_simulation_probe 236 + SimulationProbeGuard 243 + apply_as_current_for_legality 407 all coexist; §3@297 §2@581 read the flag). Re-verified in isolated ship worktree: build clean, clippy -D warnings clean, drive_drain_idx18_wins_live + idx17 + bounded-term + victim-with-out PASS, full analysis:: 96/0. Pushed ship/combo-detect-pr3 → **PR phase-rs/phase#4480** (cross-repo, OPEN+MERGEABLE, CI engaged, watcher bgb1t8jkt). No self-enqueue. PR-4 still GATED.
- Lines: L1 = MSH-E fix round (mshe-fix-executor verifying); L2 = PR-3 shipped #4480 (CI watch only — effectively free); L3 = reserved. Within cap.

## 2026-06-27 (cont.5) — PR-3 merging; PR-4 GATE LIFTED (conditional); MSH-E in re-review
- USER lifted the combo PR-4 gate, with explicit sequencing: "wait for PR-3 (#4480) to get MERGED, then launch PR-4 based on the latest upstream/main so it contains PR-3." → Do NOT launch PR-4 (incl. planning) until #4480 merges. Merge poller armed (b0ze56evo, polls gh pr view 4480 state until MERGED).
- #4480 state: status:ready-to-merge + mergeStateStatus CLEAN, in merge queue (mergedAt null). Also: earlier "CI green" watcher exits were FALSE POSITIVES — the AI-CONTRIBUTOR triage label fires a `labeled` pull_request event that cancels the in-flight CI run (concurrency group) and starts a fresh one; `gh run watch` also died once on a transient HTTP 401. Authoritative CI confirmation = poll `gh run view <id> --json status,conclusion` until completed/success (run 28295701617 = completed/success, all jobs green). LESSON: don't trust `gh pr checks --watch`/`gh run watch` exit codes here; poll the run conclusion directly (bash syntax in background tasks — they run under /bin/bash, NOT fish).
- PR-3 #4480 series-link fix (user-requested): cross-verified the combo series via a 4-agent workflow (planning docs + git + gh): PR-0 #4092, PR-1 #4097, **PR-2 #4119** (the one I'd missed), PR-3 #4480. Updated #4480 title→"feat(analysis): combo-detection PR-3 — …" and body with a Combo-detector series section (table + predecessor links). All 4 links verified to exist before publishing.
- PR-4 SCOPE (decided, stated to user for confirmation window): FEASIBILITY-AND-PLAN.md:130 PR-4 = **Static ability-graph extractor (Engine B)** — analysis::ability_graph, coverage.rs traversal → node/edge graph + per-Effect ResourceVector, Tarjan SCC + net-vector coverability → candidate cycles; Effect→resource mapping incremental (mana/counter/damage/untap/cast first, rest "unmodeled"); file analysis/ability_graph.rs; offline, no gameplay change; depends on PR-0 (already in main). PROGRESS.md status board stops at PR-3 (no PR-4 row yet) — roadmap is the sole PR-4 def. Numbering verified consistent (PR-0/1/2/3 all match shipped). Will launch engine-planner on post-merge main when b0ze56evo reports MERGED.
- MSH-E: fix round DONE (mshe-fix-executor, gracefully culled — first engine-implementation-executor to cull via SendMessage, validating the #4478 grant). All 4 fixes verified in code (K serde+eq game_state.rs:6837/8175, is_synchronous_mana_pay_cost effects/mod.rs:4000, 2 new tests). Fresh re-review in flight (mshe-rereviewer, opus) re-running non-vacuity reverts. On CLEAN → commit + ship (check base aa4e88ec4 vs upstream; rebase if behind, as PR-3 needed).
- Lines: L1 = MSH-E re-review; L2 = PR-3 merging→then PR-4 (gated on merge); L3 = reserved. PR-4 will occupy L2 post-merge.

## 2026-06-27 (cont.6) — housekeeping: #4471 + #4478 MERGED (confirmed)
- **#4471 (MSH-F: Cosmic Cube + Hawkeye, Young Avenger) MERGED** 14:10Z → `upstream/main` `a78725e7f`. Those 2 MSH cards expected cleared (parser DynamicQty support); confirm at next coverage re-measure (don't assume — cf. Ruinous false-prediction).
- **#4478 (SendMessage skills) MERGED** 16:04Z.
- **#4480 (PR-3) still OPEN** in merge queue → PR-4 still gated (poller b0ze56evo running).
- Ship-base note: MSH-E worktree base aa4e88ec4 carries MSH-F as LOCAL commits (fbb0c72cf + aa4e88ec4); upstream now has it squashed as a78725e7f. MSH-E's diff vs aa4e88ec4 is ONLY Ruinous+Hawkeye-Master-Marksman (no MSH-F), so cherry-pick onto current upstream won't re-introduce MSH-F — resolve any drift conflicts at ship time (as PR-3 needed).
- Corrected MSH remaining-work picture: MSH-F DONE (merged). Left for MSH = MSH-E (shipping) + 3 bespoke cards (Baron Helmut Zemo: anaphoric "those exiled cards"; Loki God of Mischief: new became-target-of-ability trigger matcher; The Incredible Hulk: conditional Enrage "if he's attacking").

## 2026-06-27 (cont.7) — MSH-E SHIPPED (#4482); PR-4 plan rev round; Loki planning launched
- Local main advanced 5eca83b8c → c1b61ded5 (55-commit ff, PR-3 #4480 merged) BEFORE launching new review/exec (per user). #4478 skill edits reset (byte-identical to upstream); 2 data-file mods + tilt.log preserved (pre-existing, not mine).
- **MSH-E SHIPPED → PR phase-rs/phase#4482** (The Ruinous Wrecking Crew + Hawkeye, Master Marksman). Re-review CLEAN (4 fixes resolved); 2 new LOW resolved pre-commit: LOW-1 CR mis-cite fixed (117.1→118.1 effects/mod.rs:4082; 117.1→118.3a marksman_tests.rs:183, both grep-verified), LOW-2 known-gap doc note (oracle_modal.rs ModalCountSpec). Committed f2f3af4ee; cherry-picked CLEAN onto upstream as 3ed3345c9 (9/11 files drifted, no conflict). Verified green in isolated ship worktree (b503z28r6): cold build ok, clippy -D warnings clean (b1q6jd13x: Finished 0.15s, zero error/warning), discriminating set 15/0, full engine suite 13857/0 (6 ignored). Pushed ship/msh-e-marksman-ruinous → #4482. CI engaged (run 28297122179 in_progress, pull_request event; auto-label triage already completed/success so concurrency cancel-restart settled); mergeable MERGEABLE, mergeStateStatus BLOCKED (awaiting maintainer ready-to-merge label). No self-enqueue.
- **PR-4 plan (wx9ayxxfr) review verdict: CHANGES REQUIRED, 0 blocker / 2 high / 2 med / 4 low** — architecture SOUND, two-slice split (PR-4a/PR-4b) ENDORSED. Plan at .planning/combo-detection/PR4-PLAN.md. Lead decisions on judgment-calls: HIGH-1 = adopt per-axis magnitude (Fixed(n) vs Unbounded-up for dynamic production; unit for cost) so coverability is recall-correct + Priest+Mantle (trigger-free) becomes a genuine PR-4a green; LOW-1 = adopt Projection enum over modeled:bool (CLAUDE.md hard rule); MEDIUM-1 = trigger_axis exhaustive-no-wildcard over 169 TriggerMode variants (matches Effect drift gate). Revise→re-review workflow w408u34i9 IN FLIGHT (revise applies all 8 findings + reconciles HIGH-1/LOW-1/LOW-2 into one Projection type; xhigh adversarial re-review audits convergence).
- **Loki planning launched (wsdtl58d0)** on Line 1 (MSH bespoke trio, hardest last: Loki→Hulk→Zemo). Worktree wt-msh-loki (feat/msh-loki off c1b61ded5). understand(trigger-infra + target-event-flow)→design(MSH-LOKI-PLAN.md)→xhigh adversarial review. Key risk flagged to planner: the "becomes the target of an ability you control" SEAM may not cleanly exist — plan must pinpoint it or introduce minimally + CR-ground.
- Oracle text captured (data/card-data.json): Loki = "Whenever a player or permanent becomes the target of an ability you control, draw a card. This ability triggers only once each turn." Incredible Hulk = "Reach, trample / Enrage — Whenever ~ is dealt damage, put a +1/+1 counter on him. If he's attacking, untap him and there is an additional combat phase after this phase." Baron Zemo = connive-on-cast-black trigger + Boast "Exile any number of black cards from your graveyard with fifteen or more black mana symbols among their mana costs: Copy those exiled cards. You may cast up to three of the copies without paying their mana costs."
- Lines: L1 = Loki planning (wsdtl58d0); L2 = PR-4 plan revise/re-review (w408u34i9) [PR-4 impl still gated behind a CLEAN plan]; L3 = reserved (failed CI + PR comments). MSH-E #4482 = shipped, only re-engage on CI failure. Standard rollout GATED pending explicit user word.

## 2026-06-27 (cont.8) — reclustering @ c1b61ded5; #4482 review fix
- USER asked to rerun clustering for MSH / standard / modern∩commander vs current main. Data already FRESH at c1b61ded5 (card-data-meta.json: commit c1b61ded5, generated 18:53Z; Tilt card-data resource regenerated post-ff). No cargo regen needed — clustering scripts are pure jq/awk. Ran coverage-breakdown.sh + cluster-assign.sh for all 3 pools.
- **Results (3-agent workflow wj5q84dbo, all plans surgically updated, ZERO regressions across all pools — each agent spot-checked named cleared cards, none reappeared across the 55-commit jump):**
  - MSH: 7→**5** unsup (97.55%→98.25%). −2 = Cosmic Cube + Hawkeye Young Avenger cleared via MSH-F #4471. Remaining 5 = Ruinous + Hawkeye Master Marksman (MSH-E, in #4482) + Loki/Zemo/Hulk. **Incredible Hulk reclassified S01-reflexive-if-rider** (known building-block class → more tractable than "bespoke"). Forward: #4482 merge → MSH = 3 unsup (98.95%).
  - standard: 273→**270** (94.46%→94.52%). −3 (parser-gap −3, resolver flat 32). DynamicQty bucket 37→34. No cluster grew.
  - modern∩commander: 1556→**1505** (93.22%→93.44%). −51 (parser-gap −50, resolver −1). Biggest delta — pool had SKIPPED the v0.7.0 remeasure (baseline was 2026-06-23 ae663ee8c); this closes that gap. Condition_If −11, DynamicQty −11.
- **cluster-assign.sh ruleset gap FIXED:** modern∩commander had 1 S99-UNCLUSTERED = Nephalia Academy (handler Swallow:Replacement_Instead — no bucket). Added AWK rule `if (h ~ /Replacement_Instead/) return "S28-replacement-instead"` (reusable replacement-"instead" class). Re-ran → S99=0 restored, Nephalia Academy→S28.
- **#4482 PR review addressed (Line 3):** maintainer matthewevans CHANGES_REQUESTED — runtime blocker: `resolve_repeated_optional_payment_choice` (effects/mod.rs:4091) offered another "pay {1}" prompt on `remaining>0` EVEN when the accepted payment FAILED (cost_payment_failed_flag). Verified his reading of the code + CR myself: CR 118.3 (docs:972, can't pay without resources) + CR 603.12a (docs:2659, reflexive tied to payments made) → a failed payment must END the sequence. FIX: nested the next-prompt offer inside the `!cost_payment_failed_flag` success branch → failed payment falls through to finish_repeated_optional_payment (reflexive once iff K≥1). Gemini MEDIUM: added `payment_unit.condition = None` (defensive; field exists ResolvedAbility @ ability.rs ~16950). Rewrote test → `failed_payment_ends_sequence_and_offers_reflexive_once` (asserts pending None + modal (0,1) after failed payment; no third prompt). Verified isolated worktree: clippy -D clean, marksman 8/8. Committed a996bbf8d, pushed → #4482 updated (CI re-runs). Discrimination revert-test bgaxxtqb3 in flight (revert code fix, keep new test, expect FAIL).
- **PR-4 plan round 2 (wzdy97jzh) launched:** round-1 revise (w408u34i9) resolved 6/8; re-review caught residual HIGH-1 (untap-COST edge gap — AbilityCost::Untap→produces:Tap missing, Priest+Mantle still 0 candidates) + LOW anchor. Round 2: add symmetric untap-cost arm + evidence-gate canonical green (Mantle if its untap is AbilityCost::Untap, else Devoted Druid; demote Mantle to PR-4b if parser can't see its untap as a cost).
- Lines: L1 = Loki planning (wsdtl58d0) + reclustering DONE; L2 = PR-4 round 2 (wzdy97jzh); L3 = #4482 fix (pushed, discrim verifying). MSH-E #4482 CI was GREEN before the fix (run 28297122179); fix re-triggers CI.

## 2026-06-27 (cont.9) — USER scope gate: finish only PR-4 + rest of MSH
- USER: "Do not proceed to PR-5 or implementing the standard clusters without my authorization. Finish only PR-4 and the rest of MSH."
- GATES (hard): (1) PR-5 combo-detector GATED — PR-4 is the last combo PR to complete autonomously; (2) standard clusters GATED (reaffirmed). Modern/commander were never in implementation scope (measurement only).
- IN-SCOPE autonomous: finish PR-4 (Line 2: plan round 3 wr8jonlzf → impl → ship) + rest of MSH (Line 1: Loki [impl loki-impl in flight] → Incredible Hulk → Baron Zemo). After #4482 merges + the trio ships, MSH = 0 unsupported; Line 1 then has nothing left.
- STOP BEHAVIOR: when PR-4 ships AND the MSH trio is done, STOP and report — do not pick up PR-5, standard, or any new line. Line 3 stays reserved for PR comments + failed CI throughout.
- Persisted to memory: scope-gate-pr4-msh-only.md (+ MEMORY.md pointer; corrected stale ship-finished line PR-4→PR-5).

## 2026-06-28 — post-merge rescan @ 7c1c1cf67 (upstream/main HEAD)
- USER: update local upstream/main, let Tilt rebuild local main, rescan clusters, validate 100% MSH, refresh standard + modern∩commander, report card-coverage + combo-detector.
- SYNC: `git fetch upstream main` → local main was 0 AHEAD / 39 BEHIND upstream/main; merge-tree content-containment test = upstream tree exactly (all local work squash-merged). `git reset --hard upstream/main` → c1b61ded5 → **7c1c1cf67** (`feat: Alchemy perpetual keyword grant #4529`). Only tracked drift was generated `known-tokens.toml` (regen by gen-card-data.sh) — cleared, tilt regenerated. Tilt card-data resource finished 15:06:28Z → coverage-data.json fresh.
- MSH/combo PRs confirmed IN main: #4534 (PR-4b), #4533 (Zemo), #4526 (Hulk), #4493 (PR-4a), #4491 (Loki), #4482 (Ruinous+Marksman), #4480 (PR-3), #4471 (Cosmic Cube+Hawkeye YA). No open PRs.
- **Coverage rescan (members stable, so deltas = real support gains):**
  - **MSH: 5→0 unsup (98.25%→100.00%, 286/286).** −5 = MSH trio (Loki/Hulk/Zemo) + Ruinous Wrecking Crew + Hawkeye Master Marksman all merged. **MSH FULLY SUPPORTED.**
  - standard: 270→**263** unsup (94.52%→**94.66%**, 4661/4924). −7 (parser-gap 233, resolver 30). Biggest buckets: S25-effect-verb-bespoke 40, S07-condition-if-bespoke 29, S19-new-trigger-matcher 22, S10-dynamic-qty 21.
  - modern∩commander: 1505→**1492** unsup (93.44%→**93.50%**, 21451/22943). −13 (parser-gap 1320, resolver 172). Biggest: S25 518, S07 127, S19 94, R2-aslongas 80.
- **cluster-assign.sh ruleset gap FIXED:** modern∩commander had 1 new S99-UNCLUSTERED = **Tranquil Frillback** (handler `Swallow:Modal_DynamicMaxDropped` — repeatable-pay modal w/ dynamic mode count, no bucket). Added AWK rule `if (h ~ /Modal_/) return "S29-modal-dynamic-choose"` (reusable modal-dynamic-max class, mirrors the cont.8 S28 fix). Re-ran → **S99=0 restored** for all 3 pools.
- **Combo-detector status (all on main, test-engine green):** PR-0/1/2 (ResourceVector + sim + net-progress detector), PR-3 #4480 (mandatory-loop drain-cascade winner, CR 704.5a), PR-4a #4493 (static ability-graph combo-candidate extractor, Engine B), PR-4b #4534 (effect/trigger breadth + life-symmetry cost) all merged. Corpus = 53 canonical combos (4 gated on unimplemented cards: Doc Aurlock / Professor Onyx / Animate Dead / Grindstone), ~11 driven-live. **PR-5 (`cargo combo-verify` CLI), PR-6/7/8 = NOT STARTED, GATED.** PROGRESS.md status board is STALE (still shows PR-2 open / PR-3 "starts here") — reality is PR-3/4a/4b merged.
- SCOPE: rescan + report only. No engine impl (PR-5 + standard clusters remain GATED per scope-gate-pr4-msh-only). Only edit = the gitignored planning tool's S29 rule.

## 2026-06-28 (cont) — USER lifted gate for 3 lines: S01, PR-5, S21
- USER authorized: Line 1 = largest-ROI standard cluster (picked S01-reflexive-if-rider, 17 cards, evidence: only cluster with a shared parser structure; S19's 22 are ~18 distinct triggers = low ROI); Line 2 = combo PR-5 `cargo combo-verify` CLI; then a DEDICATED Line 4 = S21-static-ability standard cluster (8 cards). Line 3 reserved for PR comments/CI. STOP after these; await authorization for more.
- Worktrees off upstream/main 7c1c1cf67: wt-std-reflexive (feat/std-reflexive-if-rider), wt-combo-pr5 (feat/combo-detect-pr5), wt-std-s21 (feat/std-s21-static-ability).
- PLANNING (2 workflows, xhigh plan → adversarial review): wv90y6kux (S01+PR5), wvktebwpt (S21). ALL THREE plans came back changes_required — gate caught real defects:
  - **S01:** [HIGH] Orbital Plunge/Torch the Witness map to PreviousEffectAmount{GT,0} but previous_effect_amount_from_events (effects/mod.rs:4754) sums TOTAL damage for DealDamage (only Fight sums excess) → misparse; DROPPED both + the excess recognizer (clean set=5: Consuming Ashes/Brackish Blunder/Sold Out/Driftgloom/Wisecrack +Faller's via Part B). [MED] add ` had ` arm to parse_target_anaphoric_tense_polarity. [LOW] CR 508.1b not 510.1c for "is attacking". Root-cause confirmed: rider sub_ability parses with condition:null (fires unconditionally — correctness bug). Chokepoint=lower_effect_chain_ir; single registration=try_nom_condition_as_ability_condition (covers both swallow paths).
  - **PR-5:** [HIGH] gated array idx = 2/21/38/51 NOT 19/36/49 (idx49 is a DRIVER) — verified against CORPUS. [HIGH] DeferralBucket → declarative ComboRow field + partition shape-lock (not match idx). [MED] ComboBoard/BeatTrace pub(crate). [MED] JSON via string DTO (no Serialize on ResourceAxis/WinKind). Design: extract corpus+driver to feature-gated analysis/corpus.rs, parameterize on db:&CardDatabase, thin bin.
  - **S21:** [HIGH] B3 hexproof scoping multiplayer-WRONG (Nowhere to Run has NO "you control"; scope by affected filter, ANY targeting player) — DEFERRED to B3 pass. [LOW] B2 "costs"/"cost" both verb forms. Decomposed into 5 building blocks (B1 Agatha dyn cost-red, B2 plot/unlock special-action cost-red, B3 hexproof+ward, B4 grant-from-exiled, B5 play-from-top) + 2 re-homes (Sandswirl Effect:can't, Koh Effect:choose — OUT of static scope).
- Consolidated plan+amendments docs: .planning/coverage-analysis/{S01,S21}-PLAN-FINAL.md, .planning/combo-detection/PR5-PLAN-FINAL.md (amendments OVERRIDE plan body).
- DISPATCHED 3 background executors (engine-implementation-executor) in their worktrees — CRITICAL: worktrees NOT watched by Tilt, so they run cargo DIRECTLY (own target dir, no lock contention): l1-exec (S01 5 cards), l2-exec (PR-5 CLI), l4-exec (S21 B2+B1 only this pass — Doc Aurlock/Inquisitive/Agatha). Each: no commit (lead ships), report diff+coverage-delta+verification via SendMessage. NEXT: review-impl loop (lead-spawned reviewer, ≤3 rounds) → ship via merge queue.
- **PR-5 SHIPPED → phase-rs/phase#4547** (feat/combo-detect-pr5). l2-exec impl → l2-review APPROVED (no blocking; 1 LOW = classify_live revert-probe, fixed: 27 corpus tests pass). Acceptance `cargo combo-verify` = 12 confirmed/4 gated/37 deferred/0 failed, exit 0. Gated array idx 2/21/38/51 (idx49 is a driver). Feature-gated (combo-verify) so default lib/WASM surface unchanged; data/card-data.json symlink gitignored (not committed). upstream/main still 7c1c1cf67 (no drift) → no rebase. Auto-merge enqueue REJECTED (external contributor lacks EnablePullRequestAutoMerge perms — expected; maintainer squash-merges). l2-exec + l2-review culled. S01 under l1-review; S21 (l4-exec) finalizing B2+B1.
- **S21 PASS 1 (B2+B1) SHIPPED → phase-rs/phase#4551** (feat/std-s21-static-ability). l4-exec impl → l4-review APPROVED (no blocking; full workspace clippy --workspace exit0 cleared the new-enum-variant downstream-breakage risk; B1 dynamic_count:None path behaviorally unchanged). +3 flips (Doc Aurlock→ReduceActionCost{Plot,2}, Inquisitive→ReduceActionCost{UnlockDoor,1}, Agatha→dynamic ReduceAbilityCost Power{Source}), 0 regressions over 35396 cards. New variants SpecialAction::Plot + StaticMode::ReduceActionCost. Single-authority apply_special_action_cost_reduction (plot+unlock). Main DRIFTED 7c1c1cf67→4613c08e5 (#4546 attachment cycles) during review → rebased + RE-RAN full CI-equiv green (fmt + clippy --workspace 2m58s + test -p engine all pass). swallow_check.rs untouched (empirically unneeded). l4-review culled. NEXT for this line: B3 (Nowhere to Run hexproof+ward — multiplayer-correct scoping per amendment), then B4 (Locus grant-from-exiled), B5 (Fblthp play-from-top); Sandswirl/Koh need re-homed non-static work (out of scope, cards stay gated). Pass-2 branch feat/std-s21-b3 off upstream/main in wt-std-s21.
- **S01 SHIPPED → phase-rs/phase#4553** (feat/std-reflexive-if-rider). l1-exec impl → l1-review APPROVED (independently reproduced the 11-card clean diff: every change condition:null→correct, 0 swallows, 0 unrelated; filter.rs blast radius confined to the new LKI arm). Flips 11 cards (Consuming Ashes/Sold Out/Driftgloom/Wisecrack + serum snare/sewers of estark/battlefield improvisation/sweep away/will of the all-hunter/grave choice), swallows −11, 0 regressions. No new enum variants. Drifted main (4613c08e5) → rebased + full CI-equiv re-run green (fmt + clippy --workspace 4m18s + test -p engine 120 ok / reflexive_if_rider 8/8). Documented follow-ups (no regression): Brackish LKI-tapped plumbing (own cluster); Faller's Part B optional-decline guard. l1-exec + l1-review culled.
- **ALL 3 AUTHORIZED FIRST-DELIVERABLES SHIPPED (open PRs, maintainer squash-merges — external contributor can't enqueue): S01 #4553, PR-5 #4547, S21 pass1 #4551.** S01 line = the reflexive-if-rider building block (remaining S01-bucket cards need DIFFERENT building blocks = separate clusters, need fresh user authorization — do NOT pursue unprompted). 
- S21 LINE CONTINUES (dedicated, user said implement S21): l4-exec on B3 (feat/std-s21-b3, Nowhere to Run hexproof+ward, multiplayer-correct). Then B4 (Locus grant-from-exiled), B5 (Fblthp play-from-top). Sandswirl/Koh stay gated (re-homed non-static gaps, out of scope). After S21 cluster done → STOP, await authorization.

##  — S21 B3 shipped (PR #4557)
- Nowhere to Run scoped hexproof-bypass + ward-suppression. Reviewed (b3-review opus, all 7 items PASS; CR 603.2g→611.3+613.11 fixed; stale deferral test→positive ship test). Lead re-verified all CR citations (grep) + test replacement.
- Rebased onto upstream/main 752ed3058 (2 new commits: Namor #4552, auto-pass timers #4540 — no file overlap). Post-rebase CI-equiv GREEN: fmt=0, workspace clippy=0, cargo test -p engine=0; reviewer nextest 17753 passed.
- Shipped via FORK + cross-fork PR #4557 (head lgray:ship/nowhere-to-run-hexproof-bypass, base main). Reviewer had wrongly tried upstream push (no write access) + cited a stale scope-gate memory (now corrected: full S21 cluster authorized).
- Worktree forge.rs-ship-nowhere-to-run @85245ddd1.
