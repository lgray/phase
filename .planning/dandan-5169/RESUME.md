# RESUME — Dandan format run (phase-rs/phase#5169), parked 2026-10-01

The user parked this lane. Resume from origin's two branches only:
- `feat/dandan-format` holds the accepted code commits.
- `dandan-run-state` (orphan) holds the run records. They live under `.planning/dandan-5169/`, with this file at the branch root.

All paths below are relative to a checkout root (`$REPO`). No absolute path is assumed.

## 1. Git state

| Item | Value |
|---|---|
| Run base (`BASE_SHA`) | `b9ba93609631d6d4bfab7169c00a9e57cec58ae0` (upstream/main at run start) |
| `feat/dandan-format` head | `91ad0dff88426ed6d983e8e060d67cd0a030c926` (Phase 4b accepted) |
| upstream/main at park | `b2d35b6e` (39 commits past BASE_SHA) |

**Resume step 0:**
1. Merge upstream/main into `feat/dandan-format`. Never rebase it: the branch is pushed. Re-run the Phase 1–4b checks on the merge.
2. Re-apply the Phase 5 WIP patch against the head named in its file name.
3. If upstream touched the same files, re-measure Phase 5's base claims before continuing.

To restore the records into a checkout:

```bash
git fetch origin feat/dandan-format dandan-run-state
git checkout -B feat/dandan-format origin/feat/dandan-format
git archive origin/dandan-run-state .planning | tar -x
```

Afterwards the records sit in `.planning/dandan-5169/`, which is gitignored on the code branch.

## 2. Phases

The charter is `.planning/dandan-5169/phase-charter`, revision r5 plus a seam fix. Prior revisions are kept as `phase-charter.r0` to `phase-charter.r5`. The audit trail is `phase-fit`, append-only, sections 1–23. Phase k's directory is `.planning/dandan-5169/phases/<k>/`. It holds `plan.md` (the current plan), `plan.rN.md` (prior versions), `plan-review-rN.md`, `executor-rN.md` and `impl-review-rN.md`. Short summaries of the accepted phases are in `phases/summaries.md`.

| # | Phase | State | Commit / plan |
|---|---|---|---|
| 1 | card-bot `/lfg` format autocomplete | ACCEPTED | `2cf3d3aa` |
| 2 | S1 format registration + axes + protocol bump | ACCEPTED | `b96d6ef2`, `41b2e353` |
| 3 | AI force-keep re-gate | ACCEPTED | `06554d0e` |
| 4 | PREREQ-0 Memory Lapse swallow check | ACCEPTED | `67cd221e` |
| 4b | Fix-first: CR 613.8b loop-only dependency ordering (defective-reference route) | ACCEPTED | `c7dfc79e`, `695933e5`, `91ad0dff` |
| 5 | CR 612 text-changing primitive | PLAN CLEAN; IMPLEMENTATION IN PROGRESS (WIP patch, unverified) | `phases/5/plan.md`; `wip/phase5-on-91ad0dff….patch` |
| 6 | S2a canonical-seat storage + pool resolver | PLAN CLEAN (amended for the engine_resolution_choices reads; re-reviewed r3) | `phases/6/plan.md` |
| 7 | S5 best-of-three ceiling | PLAN CLEAN | `phases/7/plan.md` |
| 8 | S2b-1 read sweep `game/` | PLAN CLEAN | `phases/8/plan.md` |
| 9 | S2b-2 read sweep analysis/ai_support/cross-crate | PLAN CLEAN | `phases/9/plan.md` |
| 10 | S2c filter collapse + count dedup | PLAN CLEAN | `phases/10/plan.md` |
| 11 | S3 hand-entry ownership rebind | PLAN CLEAN (charter r5 admitted `engine_resolution_choices.rs` DigChoice kept map) | `phases/11/plan.md` |
| 12 | S4a declare round + pregame dealer | PLAN CLEAN, pending a charter scope addition (see §4) | `phases/12/plan.md` |
| 13 | S4b FreeReveal | PLAN CLEAN | `phases/13/plan.md` |
| 14 | S4c in-game dealer + S6 | PLAN CLEAN, pending a charter scope addition (see §4) | `phases/14/plan.md` |
| 15 | S4d Day's Undoing wheel split | PLAN CLEAN | `phases/15/plan.md` |
| 16 | S7 frontend shared-pile display | PLAN CLEAN, pending charter scope additions (see §4) | `phases/16/plan.md` |
| 17 | S8 AI consumer rerouting (+ run-level `cargo ai-gate`) | PLAN WRITTEN, NOT YET REVIEWED | `phases/17/plan.md` |

Every plan for Phases 6–17 was written against code that does not yet contain the earlier phases. Each phase's executor runs its plan's "step 0" re-measurement at its own `PHASE_BASE` before editing.

## 3. Exact next steps

1. **Merge upstream** (see §1).
2. **Phase 5, implementation round 1 (resume).**
   - From the post-merge head, apply the WIP patch:
     `git apply --check .planning/dandan-5169/wip/phase5-on-91ad0dff88426ed6d983e8e060d67cd0a030c926.patch`, then `git apply` it.
   - If the merge touched those files, apply it on `91ad0dff` in a scratch worktree and port the changes by hand.
   - The patch is an executor's unfinished, unverified edit set. Its last note was "edits: layers.rs intrinsic-ability authority, text_substitution rewiring, and test fixes". Treat it as a starting point. A fresh executor must re-verify everything in `phases/5/plan.md`: red/green, probe gates, workspace clippy, the full `phase-engine` suite, the protocol check, the bindings check, the parser gates and the fixture re-slice report.
   - Scope is `phases/5/scope.txt` (31 paths).
   - Commit only after the checks pass. Then run the Step-6 implementation review and the parser-output measurement (regenerate card data and diff `client/public/card-data.json` against a base copy).
3. **Charter revision batch (USER-authorized).** Before Phase 12, run one charter-mode planner plus a whole-charter review that adds:
   - Phase 12: `crates/engine/src/game/elimination.rs` (`prune_mulligan_pending`, compiler-forced plus behaviour), `client/src/adapter/types.ts` (mirror), and the Phase 6 and Phase 11 Dandan integration test files (lone-Mulligan rows need the other seat's Keep).
   - Phase 14: `crates/engine/tests/integration/deterministic_game_state_serde.rs` (test-forced census row).
   - Phase 16: `client/src/components/.../ZoneViewer.tsx` (delete the seat gate) and the client test files named in `phases/16/plan.md` "Charter scope additions".
   - Phase 17: `crates/phase-ai/src/policies/tests/mill_payoff.rs` (test-only).
4. **Phase 17 plan review** (`review-engine-plan`, phase-plan mode) before its turn.
5. **Implement** Phases 6–17 in order, each running Steps 3–7 of `.claude/skills/engine-implementer/SKILL.md` (chartered mode). Push after each accepted phase.
6. **Run-level acceptance.** Check chain integrity, then run the integration review in `review-engine-impl` integration mode. Then `cargo ai-gate` without `--refresh-baseline` (owed since Phase 3), `cargo coverage`, `cargo semantic-audit`, and the full workspace suites.
7. **PR handoff.** Follow `.claude/skills/engine-implementer/pr-handoff.md` and `docs/AI-CONTRIBUTOR.md` §5–7, then finish `.planning/dandan-5169/pr-body.md`.

## 4. Open decisions

- **Charter scope additions above.** The USER already authorised charter revisions ("revising charter is fine", 2026-10-01) and continuation past loop limits when findings are narrow (phase-fit §18).
- **Pre-existing defects found and left out of scope** (recorded in `pr-notes.md`):
  - Shape-based `depends_on` false loops.
  - Record-door "your graveyard" triggers stay owner-based.
  - `gen-test-fixture.py --check` reports 76 uncovered cards at base, none from this run.
- **No user decision is pending.**

## 5. Environment

- **Network:** allow `magic.wizards.com`, `media.wizards.com`, `mtgjson.com` and `api.scryfall.com`.
  - `./scripts/fetch-comp-rules.sh` writes `docs/MagicCompRules.txt`.
  - `./scripts/gen-card-data.sh` writes `client/public/card-data.json` and `coverage-data.json`. A cold run takes about 15–20 minutes.
- **Machine:** the run used 4 cores, 15 GB RAM and about 38 GB of disk. Source `.planning/dandan-5169/cargo-env.sh` before every cargo command. It sets `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0` and no debuginfo; the `phase-engine` test crate peaks at about 13 GB RSS and is OOM-killed at -j4.
  - Run one cargo command at a time.
  - The full `phase-engine` nextest takes about 32 minutes.
  - When disk runs low, prune stale per-crate artifact hashes in `target/*/deps`.
- **Client:** run `cd client && pnpm install --frozen-lockfile --ignore-scripts` once.
- **Workers:** run Sonnet (`claude-sonnet-5-5` or newer) with effort high. Executors use the `engine-implementation-executor` agent type with `model: sonnet`. Role briefs are in `.planning/dandan-5169/roles.md`; worker rules are in `worker-env.md`. Update the paths in both to the new checkout root.
- **Tooling:** `cargo-nextest` is needed. Tilt is not used.

## 6. PR route

From the cloud environment, `api.github.com` returned 403 and `gh` was not installed, as re-tested in journal J3, J5 and J6. Open the PR outside the cloud from `.planning/dandan-5169/pr-body.md`, with head `lgray:feat/dandan-format` and base `phase-rs/phase:main`. Do not open it before run-level acceptance.
