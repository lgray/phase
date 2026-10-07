# Final review-impl, slice B (tests): head b0ca259fbf, base 4ab8245808

Slice: `git diff 4ab8245808 b0ca259fbf -- crates/engine/tests crates/phase-ai/tests` (28 files, +9459/-9; the 16 new modules each have a `mod` line in `integration/main.rs`; no new top-level test binary). Whole slice read. Mode: ordinary, tests only.

Verdict: no HIGH. 1 MED behavior, 3 MED text, 0 machinery; 4 LOW ride along (3 behavior-class, 1 text). No wrong product behavior found.

## Findings

**[MED][behavior]** `crystal_spray_prompts_both_domains_then_draws` cannot fail on the two things its doc claims ("folds both domains and draws after the change"). Evidence: text_substitution_cr612.rs, fn `crystal_spray_prompts_both_domains_then_draws`; its assertions are `options.len()==40`, pair-shape, two `contains`, and `last_named_choice.is_none()`. None reads hand size / `CardDrawn`, and none checks that "Black -> Red" changed Bad Moon (`awk` over the function body: no `hand`, `Draw`, `draw` token outside the doc line). It also stages a second card, `sleight`, and discards it with `let _ = sleight;`. Why it matters: Crystal Spray is a decklist card; its `Draw a card.` tail and the latch-then-apply order are unguarded. Suggested fix: assert the Spray caster's hand grew by one after resolution and Bad Moon's pump flipped (or a post-change `walks`), delete `sleight`/`let _`; closing test = remove the draw from Spray's chain and see red.

**[MED][text]** CR subject mismatch. Old: `/// The non-`effect` fields of a resolving ability are rewritten while its runtime state stays bit-identical (CR 601.2b).` CR 601.2b (grep: "If the spell is modal, the player announces the mode choice ... alternative or additional costs") is cast-time announcement, not resolution-time text rewrite. Replace `(CR 601.2b)` with `(CR 612.1)`.

**[MED][text]** Plan/history jargon in durable comments (USER rule: substance, not pointers). Sites, old → new:
- dandan_read_sweep.rs: `(red before the\n/// sweep), by `P1`` → delete `(red before the sweep)`.
- dandan_read_sweep.rs: `// V3 pile-side positive controls (Phase 6 code; green before the sweep)` → `// V3 pile-side positive controls`.
- dandan_simultaneous_draw.rs: `// V6 / V7: deck-out under the dealer (S6, verification only)` → `// V6 / V7: deck-out under the dealer`.
- text_substitution_cr612.rs: `"the answer was consumed by the latch (F7)"` → `"the answer was consumed by the latch"`.
Sweep boundary: added lines of slice B matching `Phase [0-9]|red before|green before|before the sweep|\(F[0-9]+\)|S6|round [0-9]|lane|charter|reviewer|addend` (5 matches, all listed; the 6th is the header below).

**[MED][text]** False claim about what V7 pins. Old (dandan_simultaneous_draw.rs header): `//! S6 scope limit: no card on the Dandân decklist has a draw replacement, so a\n//! prompt between two empty-pile attempts cannot occur for that list. V7 pins\n//! that for the list only; it is not a general pause-safety proof, and V5 is the\n//! positive control that a prompt is reachable inside the dealer.` V7 (`v7_no_prompt_opens_between_the_two_empty_draws`) builds Prosperity over Islands (`prosperity_game`); it never touches the decklist, and its message says "the Dandân list has no draw replacement". The premise itself is true (jq over the fixture for the 23 names: no oracle text with a draw replacement), but this test does not pin it. Replace header with `//! A prompt between two empty-pile attempts needs a draw replacement; V5 is the\n//! positive control that a prompt is reachable inside the dealer, and V7 covers a\n//! board with none.` and the assert message with `"no draw replacement is on the board"`.

**[LOW][text]** `w1e_end_the_turn_follows_the_last_dealt_card` message `"CR 724.1: the turn ends after the draw"`: 724.1 describes the end-the-turn procedure; the ordering claim is instruction order, CR 608.2c ("follows its instructions in the order written"). Replace `CR 724.1` with `CR 608.2c`.

**[LOW][behavior-class, no wrong behavior]** `v6_all_graveyards_types_count_the_pile_once` (Tarmogoyf) and the v5 delirium rows read a card-type *set*, so double counting cannot change their result. Measured: with `distinct_zone_holders`' dedup disabled only `v11_instants_in_all_graveyards_count_the_pile_once` went red. Dedup is guarded by v11 and v9; the name overclaims.

**[LOW][behavior-class]** `curse_newer_than_the_loop_applies_after_it` is a control, not a discriminator: it stayed green under both ordering mutations below (the Curse is newest, so any timestamp-respecting order yields the same board). The older-Curse and Ultima rows carry the discrimination.

**[LOW][behavior-class]** Claim-to-test gaps on list cards, probed for behavior (correct): no committed test casts Memory Lapse (8 copies) in Dandân or exercises Dandân itself under Magical Hack (the format's headline interaction; text_substitution rows use Bog Wraith/Merfolk/Bad Moon). Throwaway probes (deleted): Memory Lapse counters P0's spell and puts it on the pile top, P1's own container empty, same as Standard; Dandân + Hack(Island->Swamp) with P0 holding a Swamp: attack at a lone-Island defender `Err("... can't attack ... (CR 508.1c/d attack restriction)")`, at a Swamp defender `Ok`; with no P0 Swamp it is sacrificed; baseline lone-Swamp defender `Err`, Island defender `Ok`. Suggested: one test pinning those four rows.

**[LOW][machinery-adjacent, no behavior]** Duplicated test helpers: `scenario/stage/start/plenty_of_mana` in dandan_read_sweep.rs are the same bodies as the `pub(super)` ones in dandan_filter_owner_axis.rs that six sibling modules already import; `other()`, `hand()`, `pile()`, `act()` repeat across declare_round/free_reveal/hand_entry. Cost-ability lookup by `format!("{:?}", ability.cost).contains("Sacrifice"|"CollectEvidence"|"ExileMaterials"|"ExileWithAggregate"|"-7")` at 6 sites (and `contains("[Green, White]")`, `contains("blight")`) matches Debug text where `matches!` on the typed cost exists.

## §5 checklist (slice B)
(a) seam: tests sit in `tests/integration/` per CLAUDE.md; production seams are slices A/C. (b) no new bool fields; test helpers take `shared: bool` selectors (see LOW above). (c) nom-mandate grep over added lines for `.contains("`, `.find("`, `.rfind(`, `.split(`, `.split_once`, `.splitn`, chained `if let Ok(..) = tag(`: 15 matches, all test assertions or source-census helpers (positive control: the same grep prints them), zero in parser code; no string-literal `match` arms. (d) 33 distinct CR numbers in the slice grepped in docs/MagicCompRules.txt; all resolve; one subject mismatch (601.2b), one weak (724.1). `CR 400.7a` on the incremental-flush row checked and kept (permanent-spell carry-over is the case it names). (e) the new tests are class-level (`CASES` x seats, Standard controls). (f) no frontend in slice. (g)/(h) LOW above. (i) discrimination probes below.

## Discrimination probes (target-dandan, empty porcelain at start; baseline 225/225 green for the slice's filters)
The first six mutations (rows 1-4, 7, 8) ran in one build, so their red sets are attributed by test subject; the rest ran alone. Mutations on production, run against the slice's modules (`dandan_`, best_of_three_ceiling, loop_only_dependency_fallback, mulligan_serum_powder_scope, text_substitution_cr612) and phase-ai `dandan_shared_pile_ai_pair_both_act_and_draw`:

| Mutation | Red tests |
|---|---|
| `hand_entry_receiver`: `ReceiverOwns => return None` | 10 hand_entry_ownership rows (every row asserting the receiver) |
| `free_reveal_offered`: `lands <=` | v1 predicate table, v1 land-type row, free-reveal close rows |
| `seat_walk_from_active` start forced to seat 0 | opening-deal rows (P1 starts), declare-round close order, simultaneous-draw v1/v1b/v4/v5/v12, wheel rows |
| `restamp_resolving_spell_text` made inert | spell-on-stack, Acid Rain, restamp row |
| `dependency_application_order`: whole-layer timestamp fallback | `curse_older...`, chain+loop row |
| same, stall-only ordering (pre-fix shape) | `curse_older...`, `newer_independent_effect_...`, chain+loop row |
| `shared_library_wheel_split` disabled | w1, w1b, w3, w4, w5, w6 |
| session `pool_seats` chain removed | phase-ai AI-pair test |
| text-sub escalation disjunct in incremental flush disabled | `incremental_flush_matches_...`, permanent-spell carry row |
| `distinct_zone_holders` dedup disabled | only `v11_instants_...` (see LOW) |
| `counted_containers` dedup disabled | `v9_each_graveyard_with_seven_cards_counts_the_shared_pile_once` |
| `prune_affected_object_left_effects` no-op | `indefinite_text_change_ends_...`, `text_change_on_a_non_permanent_spell_...` |

Each mutated test set was non-empty-red where expected, so the instrument was live; mutated lines were reverted with `git checkout -- <file>` on files this probe alone changed, then `touch`ed. Final state: `git status --porcelain` empty, `HEAD == b0ca259fbf3e4f6ec1af5132daaf258708ebba4c`. ai-gate.log showed `done rc=0` before any probe started.

## Pre-existing (reproduce at base, untagged, never blocking)
None found in this slice. (The CR 613.8b loop-confinement change in layers.rs is a defect fix the PR's own chain+loop text-change rows depend on: they go red without it.)
