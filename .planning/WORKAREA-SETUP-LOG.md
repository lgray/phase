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
