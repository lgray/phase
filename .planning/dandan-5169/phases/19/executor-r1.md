MODEL: claude-sonnet-5-5

# Phase 19 executor r1 (implementation/fix, phase mode)

Mode implementation/fix. BASE_SHA = START_SHA = 61f66c92e98fd3d8cdde115c29c636ca46400730. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.

## Verdict
Delivered per plan; no stop-and-return item. Preparatory evidence only (not completion evidence). Raw lines: `phases/19/executor-r1-evidence.log`.

## Worktree record
- Start: HEAD == START_SHA, clean, nothing staged. End: HEAD == START_SHA, nothing staged, unstaged delta = exactly the 3 scope paths.
- Final sha256 (restored byte-identical after every mutation): layers.rs 67511bf2..., text_substitution.rs b01fdb6a..., text_substitution_cr612.rs 3a3ccdaf... (full values in the evidence log).

## Diff summary (+~190/-~180 primary, tests excluded)
- `layers.rs`: `select_next_effect` extracted from the ability-layer selector (called there unchanged in behavior) and `pub(crate)`; `order_by_timestamp` `pub(crate)`; `SubstituteTextWord` arm of `depends_on` deleted; top-level `TextSubstitution`/`TextSubstitutionSpec` imports moved into the inline test module; two inline tests that pinned the arm replaced by `fixed_graph_no_longer_orders_text_word_effects` (V6) and `select_next_effect_follows_the_cr_613_8b_loop_rule` (V4 kernel).
- `text_substitution.rs`: counting through the one rewrite authority (`rewrite_counted`, `rewrite_in_place`, `rewrite_each`, `apply_to_permanent_text`, `rewrite_resolved_ability` return occurrence counts; its params reordered to `(ability, substitution)`); `dependency_edges` + `apply_in_dependency_order` (one routine, both consumers); subtype dedupe moved to `collapse_subtype_repeats`, run once after the routine and only when it replaced something; `active_text_substitutions` returns timestamp order via `order_by_timestamp`.
- `text_substitution_cr612.rs`: 8 new rows, grouping row rewritten, one assertion message corrected.

## Constraint record
1. First step: lib-target compile `cargo nextest run -p phase-engine --features phase-engine/test-support --lib --no-run` after the import removal and param reorder: rc=0, 0 warnings (the layers.rs top-level import removal compiles; the inline `TextSubstitutionSpec::Chosen` fixtures needed the imports added to the test module, done).
2. V1x discriminates M2 only through Swamp CAB (plus Island ABC from V1): measured below (M2 red rows: Island ABC, Swamp CAB; Forest rows and Swamp ABC stay green).
3. M1-M4 measured by me, each applied alone to the final source, each red, file restored and sha256-checked each time. Two extras: M5 (kernel without the older-loop-member clause) and M6 (the deleted `depends_on` arm re-added).
4. All new rows run the production `GameScenario` cast/resolve pipeline with real cards from the shared DB. The permanent-recipient rows and Terror row use real Magical Hack x2, Crystal Spray, Sleight of Mind, Terror, Sunken Hollow, Bog Wraith, Bad Moon; V2s uses Sirocco through `restamp_resolving_spell_text` (the unit-seam row the plan keeps).
5. Base-red: new rows run with only the production arms of `layers.rs` and `text_substitution.rs` restored from START_SHA (tests unchanged): see table.

## Discriminating-test gate (production-path coverage map)
Seam: `apply_in_dependency_order` (+ `dependency_edges`, counting) via `apply_battlefield_text_substitutions` (Layer 3 pre-pass inside `evaluate_layers`) and `restamp_resolving_spell_text` (`stack.rs::resolve_top`); kernel `select_next_effect`.

| row (test fn) | claim | entry | base (START_SHA arms) | fix |
|---|---|---|---|---|
| V1 `land_type_cycle_on_a_type_line_orders_by_the_recipients_current_text` | Island, A/B/C cycle ends Island/{U} in ABC and CAB; guard AB Swamp/{B} | cast pipeline, 2 Hack + Spray | RED: ABC Forest/{G}, CAB Swamp/{B}; guard green | green |
| V1x `cycle_verdict_depends_on_the_recipients_starting_land` | Forest ABC Forest/{G}, CAB Swamp/{B}; Swamp ABC Island/{U}, CAB Swamp/{B}; guards Forest AB Swamp, Swamp CB Island | same | RED all four cycle rows (Forest ABC Island, CAB Island; Swamp ABC Forest, CAB Island); both guards green | green |
| V1L `type_line_loop_applies_in_timestamp_order` | AD Swamp/{B}, DA Forest/{G}, ADC Plains/{W} | same | green (preservation: CR 613.8b timestamp fallback on a type-line recipient) | green |
| V1d `type_line_repeats_count_until_every_change_has_applied` | Sunken Hollow AB and BA ["Forest"]; guards A ["Swamp"], B ["Island","Forest"] | same | green (preservation; discriminating by M4) | green |
| V2 `land_cycle_on_a_keyword_orders_by_the_recipients_current_text` | Bog Wraith ABC, CAB walk Swamp; guard AB Forest | same | RED: ABC Plains, CAB Forest; guard green | green |
| V3 `color_cycle_on_a_static_orders_by_the_recipients_current_text` | Bad Moon: ABC black 3/white 2/red 2; guard AB red buffed | same | RED ABC (white buffed [2,3,2]); guard green | green |
| V2p `color_cycle_on_a_stack_spell_orders_by_the_recipients_current_text` | Terror at white creature, 3 Sleights answered while Terror stays on the stack: destroyed; guards none, AB destroyed | cast pipeline + `GameAction::ChooseOption` at `NamedChoice`, passes to resolve Terror | RED ABC (Battlefield); guards green | green |
| V2s `restamp_applies_a_color_cycle_in_dependency_order` | restamp of Sirocco with A,B,C leaves JSON equal; guard A,B reads Red | `restamp_resolving_spell_text` direct | RED (repeat_for reads White) | green |
| V5 `active_substitutions_are_grouped_and_ordered_per_recipient` (rewritten) | one list per recipient in timestamp order | `active_text_substitutions` | RED (old dependency order) | green |
| V4 kernel `select_next_effect_follows_the_cr_613_8b_loop_rule` | literal edge lists: no edges, chain, loop, loop with leaving edge | unit | n/a (kernel new) | green; RED under M5 (left 1, right 2) |
| V6 `fixed_graph_no_longer_orders_text_word_effects` | `depends_on` false for former chain/loop pairs; positive control type writer/reader true | unit | RED under M6 (arm re-added) | green |
| V4 preserved: `chain_of_text_changes_*`, `loop_of_text_changes_*`, `chain_plus_loop_*`, `same_from_*`, loop_only_dependency_fallback | unchanged | | green | green; chain/chain_plus_loop RED under M3 |

Every red row fails on its discriminating assertion, not on a guard (first panics at base: Wraith ABC Plains vs Swamp; Forest ABC Island vs Forest; Island ABC Forest vs Island; Moon [2,3,2] vs [3,2,2]; Terror Battlefield vs Graveyard). Per-row base values (probe, also per CAB row): Island ABC Forest/{G}, CAB Swamp/{B}, AB Swamp; Forest ABC Island, CAB Island, AB Swamp; Swamp ABC Forest, CAB Island, CB Island; Wraith ABC Plains, CAB Forest, AB Forest; Moon ABC [2,3,2], CAB [2,2,3], AB [2,2,3]; Terror ABC Battlefield (none, AB, ACB, CAB Graveyard). Under the fix: Island ABC/CAB Island/{U}; Forest ABC Forest/{G}, CAB Swamp/{B}; Swamp ABC Island/{U}, CAB Swamp/{B}; Wraith ABC/CAB Swamp; Moon ABC/CAB [3,2,2]; Terror all Graveyard.

Mutations (final source, each alone; red set among text_substitution_cr612 rows):
- M1 (word-pair predicate in `dependency_edges`): cycle_verdict (Forest ABC Island), V1 (CAB Swamp), V2 (CAB Forest) red; V3, V2p, V2s, Swamp rows green (V2p not M1-discriminating, as the plan says).
- M2 (edges once before the loop): V1 (ABC Forest), V1x (only Swamp CAB Island), V2 (ABC Plains), V3 (ABC), V2p (ABC), V2s red; Forest rows and Swamp ABC green.
- M3 (timestamp order, no kernel): V1, V1x (Forest ABC/CAB and guard, Swamp ABC/CAB), V2, V3, V2p, V2s, V1d (BA `Swamp+Forest`), `chain_of_text_changes_*`, `chain_plus_loop_*` red.
- M4 (collapse inside `apply_to_permanent_text`): only V1d red (BA `Swamp+Forest`; AB stays Forest).
- M5 (kernel without older-loop-member clause): `select_next_effect_*` red.
- M6 (word-pair arm re-added): `fixed_graph_no_longer_*` red.
Controls: the unmutated final source ran the identical probe (33 tests, all pass); guards equal base values in every leg, so each leg reached the code.

Focused run (final): `cargo nextest run -p phase-engine --features phase-engine/test-support -E 'test(/^game::layers::/) | test(/game::text_substitution::/) | test(/text_substitution_cr612/) | test(/loop_only_dependency_fallback/) | test(/census/)'`: 422 passed, 0 failed, 0 warnings. Packages: phase-engine only (census tests holding source scans live there; phase-ai has none). `rustfmt --check` on the 3 paths clean. No clippy/ai-gate/full suite (per brief).

## Parser gate
No file under `parser/` changed: not applicable.

## New-field threading sweep
No field added to any variant or struct. `apply_to_permanent_text` / `rewrite_resolved_ability` / `rewrite_in_place` / `rewrite_each` changed return type only; every caller in the workspace is in `text_substitution.rs` (`git grep -n` of the four names: only that file).

## Maintainer-simulation matrix
| seam | entry / first branch | selected authority / bound value | binding | storage | consumer | invalidation | hostile fixtures |
|---|---|---|---|---|---|---|---|
| `apply_in_dependency_order` on a permanent | `evaluate_layers` -> `apply_battlefield_text_substitutions`; `pending.len()==1` skips edges, `>=2` builds clones | recipient = `SpecificObject` id of the transient effect; each word pair latched at the spell's resolution | live: relation recomputed from the recipient's current text after every application (CR 613.8c), nothing stored | transient continuous effects (`gather_transient_continuous_effects`) | `select_next_effect`, then `apply_to_permanent_text` | effect gone when its duration ends (Crystal Spray at cleanup, CR 514.2) or the permanent leaves (new object, CR 400.7) | V1/V1x/V1L/V1d/V2/V3 + existing chain/loop/different-recipient |
| same on a stack spell | `stack.rs::resolve_top` -> `restamp_resolving_spell_text`; first branch `remove(&object_id)` Some | recipient = the resolving object id | live per resolution from the spell entry's ability | same | `rewrite_resolved_ability` (field-exhaustive destructure kept) | the change lasts indefinitely (CR 611.2a) and the stack entry ends with the spell | V2p production, V2s seam |
| kernel `select_next_effect` | both layers' selectors; loop-rule branch (`dependency_path_exists`) | edges over timestamp-sorted pending | per call | none | ability-layer selector and text routine | n/a | V4 literal rows, referenced-grant suites in `game::layers::` (green) |
| subtype collapse | after the routine, only when occurrences > 0 | n/a | n/a | `card_types.subtypes` | later layers read a repeat-free list | n/a | V1d (+M4) |
No serde/protocol/card-data shape changed (no enum, action, state or export change).

## CR annotations
Added/kept: 613.8a, 613.8b (also cited as 613.8a-c), 613.7, 612.1, 612.2, 205.3i, 608.2b, 305.6 (kept). Gate command from executor.md: zero `UNVERIFIED`; separately grepped 613.8c, 613.1c, 514.2 (cited in tests/doc by prefix range): all exist. `collapse_subtype_repeats` carries no CR number (plan constraint).

## Judgement calls / deviations
- Mutations ran in place in W (the brief says "restored byte-identical (sha256)") rather than in a scratch tree as plan step 4 reads; originals were copied to scratch first, every restore sha-matched the final.
- `apply_to_permanent_text` returns the count and the collapse runs only if it is > 0, so objects whose subtypes never changed are never deduped (keeps the old behavior exactly).
- Added row names differ from plan labels (V-numbers are in the table); V1/V2/V3 each assert both orders where the plan says both.
- `dependency_edges` clones the post-provider recipient once per provider and again per pair so `apply` never mutates the shared clone.

## Risks
- Cost: a recipient with k live text changes pays O(k^2) recipient clones per selection and O(k^3) over the loop; fine for the 2-4 effects real games produce, unmeasured at large k.
- The M2 note: V1x's M2 discrimination rests on Swamp CAB alone.
