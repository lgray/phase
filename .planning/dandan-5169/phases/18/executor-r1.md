MODEL: claude-sonnet-5-5

# Phase 18 executor report r1

Mode: implementation/fix (phase mode, small-change lane). BASE_SHA = START_SHA = 303d76fa692aa09fd443916b2653d95c9329b8ac. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.

## Verdict
Delivered; no stop-and-return items. All evidence below is PREPARATORY, not completion evidence.

## 1. Diff summary
- `crates/engine/src/ai_support/candidates.rs`: `card_name_choice_candidates` uses one capped `&str` pusher (`push_candidate`) plus a local `object_names` iterator; library term is `match state.shared_zone_holder(Zone::Library)`: `Some(_)` pushes `deck_pool_of(controller.id)` `current_main` names in registered order, `None` pushes `library_of(controller.id)` names; CR 400.2 comment per T2. Inline rows V3 (`card_name_candidates_in_a_per_seat_format_read_the_own_library`) and V4 (`shared_pile_card_name_domain_is_the_registered_pool_whatever_the_pile_holds`).
- `crates/phase-ai/src/search.rs`: inline rows V1 (`predict_card_name_domain_is_constant_across_determinized_worlds`) and V2 (`predict_card_name_choice_is_issued_under_k2`) plus fixture helpers.
- `crates/engine/tests/integration/dandan_analysis_ai_support_reads.rs`: v8 repaired (shared rows register a four-name pool; expected lists literal per row).
- `crates/phase-ai/src/determinize.rs`: one doc sentence qualifying the paragraph's `card_name_choice_candidates` example and the OWN-zones byte-identical claim; no coordinate copied.

## 2. Worktree record
Start: HEAD == START_SHA, `git status --porcelain` clean, no staged entries. End: HEAD == START_SHA, 0 staged, unstaged delta == scope.nul exactly (diff of sorted names empty). Mutations were applied to `candidates.rs` and restored from a saved copy; sha256 after restore equals the pre-mutation sha (88659571...a1aa).

## 3. PREPARATORY verification
- Format: `rustfmt --edition 2021 --config skip_children=true` on the four scoped .rs paths, then `--check` clean.
- Focused: `cargo nextest run -p phase-engine -p phase-ai --features phase-engine/test-support -E '(package(phase-ai) & (test(predict_card_name) | test(/search::tests/) | test(/determinize/))) | (package(phase-engine) & (test(/ai_support::candidates/) | test(v8_card_name) | test(/census/) | test(/dandan_analysis/)))'`: 349 run, 347 passed, 2 failed (`search::tests::self_destruct_target_selection_prefers_lethal_over_nonlethal_body`, `search::tests::prospective_fetch_choice_survives_to_the_real_search_prompt`); both PASS in an isolated re-run (2 of 2), i.e. load-sensitive deadline flakes, not caused by the diff (neither touches the card-name path). Logs: `/home/lgray/vibe-coding/dandan-run/scratch/p18-exec-focused.log`, `p18-exec-rerun.log`.
- Not run per brief: clippy, ai-gate, full suites.

## 4. Parser gate
Not applicable: no file under `crates/engine/src/parser/` changed.

## 5. Discriminating-test gate / production-path coverage map
Mutation A = revert only the shared arm (always read live `library_of`); mutation B = apply pool domain with no `shared_zone_holder` gate (V3 mutation). Both measured (`p18-exec-mutA.log`, `p18-exec-mutB.log`).

| Claim | Seam | Entry | Test | Flips under | Sibling/negative |
|---|---|---|---|---|---|
| Shared-pile domain constant across worlds | `card_name_choice_candidates` shared arm | real Predict cast via `GameRunner::cast` to `NamedChoice{CardName}`, then `candidate_actions` | V1 | A: FAIL "actual states agree" (`[Predict,Brainstorm,Control Magic]` vs `[Predict,Brainstorm,Memory Lapse,Control Magic]`); B: PASS | reach guards: list >1 and contains Brainstorm; `redistributed > 0` over seeds 0..16 (only evaluated when the actual-state leg passes, i.e. under the fix) |
| K=2 ensemble no support drift, contract-admitted pick | same, via `score_candidates_with_session`/`choose_action_with_session` | same | V2 | A: FAIL `finalize_mean` "observed in 1/2 samples (support drift)" left 1 right 2; B: PASS | reach: contract domain >1, ensemble non-empty |
| Pool domain, capped, placement-independent | shared arm + cap | `candidate_actions` on directly built prompt | V4 | A: FAIL left `[Pool 3,4,5,6]` right `[Pool 0,1,2,6]`; B: PASS | reach: pool 30 > cap 24, pile name sets differ, pile shared (`players[1].library` empty) |
| Per-seat format unchanged (literal base list) | `None` arm | `candidate_actions` on directly built prompt | V3 | A: PASS; B: FAIL `[Field,Held,Grave,Exiled]` vs `[...,Deep,...]` | reach: library-only name `Deep` present, opponent `Theirs` absent; green at base and after |
| Shared rows read the pool; Standard row keeps library read | both arms, three actor/format cases | real Predict casts, `candidate_actions` | v8 | A: FAIL shared P1 missing `Control Magic` (pool-only name); B: FAIL Standard row got `[Memory Lapse]` | covers shared P0, shared P1 (pile-holder resolution), Standard P1 |

No test is shape-only. Observation: under mutation A, V1 trips on the actual-state equality before the seed loop, so the seed loop's own red is not separately shown; V2 carries the K=2 sampled-world red.

## 6. Maintainer-simulation matrix
V3/V4 construct the `NamedChoice { CardName, options: [] }` prompt directly and call `candidate_actions` (cast-route proof for the generator is V1/V2 and v8).

| Row | Entry / first branch | Authority | Bound value, when | Mode | Storage | Consumers | Invalidation | Hostile fixture | Serde impact |
|---|---|---|---|---|---|---|---|---|---|
| Shared library term | `candidate_actions` NamedChoice CardName -> `shared_zone_holder(Library)` `Some(_)` (V1/V4/v8-shared) | pile holder's registered pool (`deck_pool_of`) | pool names, read per call at generation | live read of a registered, game-constant pool | `GameState.deck_pools[].current_main` | `candidate_actions`, `AiDecisionContract::issue`, `score_candidates_core` per sample | pool written only at load / between games; missing pool -> no library names (unreachable in production, no row per plan) | V1 two-copy name across hand/pile; V4 two placements | none |
| Per-seat library term | same -> `None` (V3, v8 Standard) | deciding seat's own library | identities, per call | live, never resampled | `Player.library` | same | n/a | V3 library-only name vs opponent library name | none |
No rows incomplete.

## 7. CR gate
Added CR numbers: `CR 400.2` only; `grep -nE "^400\.2([^0-9]|$)" docs/MagicCompRules.txt` finds it (Library and hand are hidden zones). Zero UNVERIFIED. Cited rule describes the annotated claim.

## 8. Judgement calls
- Used the reviewer's `scratch/p18-probe.patch` test bodies for V1/V2/V3/V4/v8 (already measured by the plan review) and wrote the generator body myself; dropped the probe's pool-absent shared row (plan: no row, unreachable). Dropped the probe's CR 401.2 comment in favor of the T2 text.
- Wrapped the T2 comment onto two lines (text unchanged).

## 9. Stop-and-return items
None.

## 10. CR annotations
CR 400.2 (candidates.rs shared arm), verified as in section 7.

## 11. Deviations from the plan
None. No new-field threading applies (no field added).

## 12. Risks
- The two phase-ai flakes above reproduce only under the parallel focused run; the acceptance run should treat them per the known deadline-flake rule.
- `determinize.rs` paragraph keeps its pre-existing stale `candidates.rs:4053` coordinate (not copied, not edited).
- `cargo ai-gate` and clippy not run, per brief/plan.
