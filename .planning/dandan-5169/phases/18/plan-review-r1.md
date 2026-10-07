# Phase 18 plan review — round 1 (phase-plan mode, phase-fit context declared: Sizing consistency blocking)

Reviewer: Sonnet 5.5. Skill: review-engine-plan at 1ecbc840cf. Plan base `303d76fa69`.

## Verdict

No behavior or machinery findings. Three `text` findings, each with replacement text; all are wording/citation fixes, so per the skill's review loop this round closes by applying them, no re-review. The design (pool-name domain at the shared-library arm, gated by `shared_zone_holder`, per-seat path unchanged) is sound and the plan's central premise was driven, not only read.

## Probes (scratch tree rebuilt from W at `303d76fa69` + `scratch/p18-probe.patch`, own target dir, deleted after)

Same fixtures as the plan's rows (Dandan, real Predict cast via `GameScenario`). Results:

| Row | With the fix (patch applied) | Generator block reverted to base (mutation) |
|---|---|---|
| V1 `predict_card_name_domain_is_constant_across_determinized_worlds` | PASS | FAIL: `["Predict","Brainstorm","Control Magic"]` vs `["Predict","Brainstorm","Memory Lapse","Control Magic"]` |
| V2 `predict_card_name_choice_is_issued_under_k2` (final form: non-empty ensemble, every action contract-admitted, chosen action admitted) | PASS | FAIL: `finalize_mean` support drift, `left: 1 right: 2` |
| V3 per-seat twin | PASS | PASS (green in both, as the plan states) |
| V4 `shared_pile_card_name_domain_is_the_registered_pool_whatever_the_pile_holds` | PASS | FAIL: `[Pool 3..]` vs `[Pool 0..]` |
| v8 repaired (`[Memory Lapse, Control Magic, Island, Brainstorm]` shared / Standard unchanged) | PASS | FAIL (`Control Magic` absent: pool-only name, so a library read cannot satisfy it) |

Controls: V1/V2/V4/v8 red at base and green with the fix (discriminating); V3 green in both; V1/V2 reach guards (domain >1, pile name set moves) held. The plan's V2 final form had not been run green before this round (the surviving `p18-fix.log` is the earlier, stricter V2); it is now measured.

Other premises checked:
- Oracle text of Predict (`client/public/card-data.json`) matches the plan's quote; Predict is in the Dandan list (`deck_loading.rs`, `("Predict", 2)`).
- `shared_zone_holder` is `pub(crate)` and exhaustive over `Zone`; Dandan shares library and graveyard (`types/format.rs`); `deck_pool_of` resolves the pile holder's pool via `zone_storage_seat`.
- `current_main =` writers: `match_flow.rs` (between games), `visibility.rs` (redaction on the filtered clone), tests, `session.rs` test setup. Production AI entry points (`engine-wasm` `get_ai_action_proposal*` via `with_state_mut`; worker path restores the `TrustedGameStateEnvelope` full export) read the authoritative state, so the redaction does not empty the pool the generator reads.
- Remaining hand reads in `ai_support` candidate/validation code index the deciding seat (`player` / `entry.player` for mulligan prompts); the `Hand|Library|...` object sweep in `mod.rs` (spell-cost display map) keys by id and is gated by `display_spell_cost` eligibility. Searched `ai_support/` non-test for `Zone::Library|Zone::Hand|zone_object_ids|hand_of|\.hand\b|library_of|\.library\b`: single candidate-domain identity read of a resampled zone (the one in `card_name_choice_candidates`). Claim 1 stands over that population.
- Sizing: one unit; four counted paths (`candidates.rs`, `search.rs`, the integration row file, `determinize.rs` comment); no variant, serialized surface, `WaitingFor`/`GameAction` change; consistent with the body (4 new rows + one repair + one comment, all one rule). Small-change lane eligible. Addenda file admits exactly the two extension paths; every charter decision stands; deferral list empty and the plan forecloses nothing.
- Nom compliance, new-variant discoverability: not applicable (no parser or enum change). CR 400.2 and 401.2 exist and say what Step 0 states.

## Findings

### T1 (text) — wrong CR in Reference Readings

Old (Reference Readings, V3 bullet): `CR 201.2 allows any card name, so the generator's bounded domain is a heuristic, not a rules reading`
CR 201.2 is "a card's name is always the English version" (`grep -n "^201.2" docs/MagicCompRules.txt`); choosing a card name is CR 201.4.
Replace with: `CR 201.4 admits any card name in the Oracle card reference, so the generator's bounded domain is a heuristic, not a rules reading`

### T2 (text) — CR annotation sentence states an engine fact under a CR whose subject is different

Old (Fix, "CR annotation at the shared arm"): `// CR 400.2: a shared library is resampled by the determinizer; its registered pool names, not its live identities, are the stable domain.`
CR 400.2 says library and hand are hidden zones; "resampled by the determinizer" is not its subject, and the project rule requires the annotation to describe the rule.
Replace with: `// CR 400.2: the library is a hidden zone, so the chooser's domain is the pile's registered pool names, not its live identities.`
The same replacement applies to the "Step 0" sentence only if it quotes the old comment; it currently does not.

### T3 (text) — measurement ledger and snapshot figures in the plan

Delete the paragraph beginning `Discrimination evidence already gathered` through `...reference diff kept at ... non-authoritative)`, keeping only its regenerate commands and the sentence on the executor showing each red leg by reverting the shared arm (the V3 mutation sentence stays). In `Pattern Coverage` replace `(exported cards whose Oracle text contains the phrase; 50 at plan time; Predict is in the Dandan list)` with `(exported cards whose Oracle text contains the phrase; Predict is in the Dandan list)`. In claim 2 and claim 4 drop the quoted panic/`left`/`right` strings (the commands regenerate them). Probe-result strings and counts in a plan are figures a later edit falsifies.

## Constraints for the implementer (exit-round, no decision changes)

- V3/V4 live in the engine crate's `candidates.rs` test module and may construct the `NamedChoice { choice_type: CardName, options: [] }` prompt directly and call `candidate_actions`; the cast-route proof for the generator is carried by V1/V2 and the v8 rows (v8 runs real `Predict` casts for both actors, so the pile-holder-seat resolution for P1 is covered there). Say so in the Verification Matrix row header (the header currently says every row reaches the prompt by a real cast).
- `determinize.rs` doc sentence: place it so it qualifies the paragraph's own `card_name_choice_candidates` example and the "AI player's OWN hand/library byte-identical" claim (false for a shared pile, which is resampled for the opponent); do not copy the stale `candidates.rs:NNNN` coordinate already in that paragraph.
- The probe tests carry no comments beyond one sentence; keep the CR comment to the replacement above.

## Pre-existing (untagged, not blocking, not repeated later)

- The K-sample scorer drops the source's own name that the contract issues (plan already lists; V2 is written so it does not depend on it).
- The per-seat branch reads the deciding seat's own face-down library by identity (CR 401.2: players cannot look at a library). The determinize header accepts this by design; no row the plan asserts depends on it.
