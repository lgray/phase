# Phase 19 plan review, round 2 (phase-plan mode, phase-fit context declared; Sizing consistency blocking)

MODEL: claude-sonnet-5-5. Plan base: W acf5422906 (layers.rs, text_substitution.rs, stack.rs and the cr612 test file are byte-identical at W HEAD 61f66c92e9). Reviewed: delta `scratch/plan19-r2.diff` plus what it references, then the new coordinator requirement (review-9669-3.md HIGH).

**Verdict: REJECT — 1 `behavior` finding (F1: missing real-card controls for different starting recipient words), 1 `text` finding (F2: CR 205.3i does not support the collapse claim). All four r1 findings are closed. Sizing present and consistent (1 unit, 3 paths, small-change lane eligible).**

## Instruments (scratch tree under dandan-run/scratch, deleted; own target dir, deleted)
Prototype of plan items 1-5 built from the plan text (about +136/-114 primary lines by `git diff --no-index --numstat`); probe rows through `GameScenario` + `GameRunner::cast(..)` with real cards from the fixture. Every probe prints, the table below is the printed output. Base = unmodified source; the Island/Terror rows are the positive controls (a base value that differs from the design value proves the driver reaches the code).

| row | base | design prototype |
|---|---|---|
| Island ABC (A=Island->Swamp, B=Forest->Island, C=Swamp->Forest; two Hack + Spray, same turn) | Forest/{G} | Island/{U} |
| Island CAB / ACB / guard AB | Swamp/{B}; Island/{U}; Swamp/{B} | Island; Island; Swamp (guard unchanged) |
| V1d Sunken Hollow `[Island, Swamp]`: AB / BA / A only / B only | `[Forest]` / `[Forest]` / `[Swamp]` / `[Island, Forest]` | identical (preservation row, green at base) |
| V2p Terror (white target) + 3 Sleight of Mind on the stack: none / AB / ABC / ACB / CAB (destroyed?) | true / true / **false** / true / true | true in all five |
| Mountain start, A=Mountain->Plains, B=Island->Mountain, C=Plains->Island: ABC / CAB | Island / Plains | Mountain / Mountain |
| Genuine type-line loop on an Island, A=Island->Swamp, D=Island->Forest: AD / DA / then C=Swamp->Plains cast third | Swamp/{B}; Forest/{G}; Plains | identical (timestamp fallback kept) |
| Forest start, plan cycle: ABC / CAB / BCA | Island / Island / Swamp | Forest / Swamp / Forest |
| Swamp start, plan cycle: ABC / CAB / BCA | Forest / Island / Forest | Island / Swamp / Swamp |

Existing `text_substitution_cr612` rows (28 incl. the chain/loop/chain+loop/different-recipient rows) and `loop_only_dependency_fallback` pass under the prototype except `active_substitutions_are_grouped_and_ordered_per_recipient`, the row the plan rewrites.

Mutations (each applied alone to the prototype, V-rows read from the same probe output):
- M4 (collapse the subtype set inside `apply_to_permanent_text`, count after the dedupe): Hollow BA becomes `["Swamp", "Forest"]`; AB stays `["Forest"]`. Discriminating in exactly the order the plan says.
- M1 (word-pair predicate in `dependency_edges`, one-at-a-time selection kept): Island CAB `Swamp/{B}` and Mountain CAB `Plains` red; Island ABC, Terror ABC stay green. The plan's "M1: V1 and V2 red" holds only through V1/V2's second cast order (C, A, B); V2p is not M1-discriminating and the plan does not claim it is.
- M2 (edges computed once before the loop): Island ABC `Forest/{G}`, Terror ABC survives (red), Mountain ABC red.
- M3 (apply in timestamp order, no `select_next_effect`): Island ABC red, Terror ABC red, Hollow BA `["Swamp","Forest"]`, and `chain_of_text_changes_applies_dependency_order_in_both_casting_orders` + `chain_plus_loop_on_one_object_and_unrelated_loop_on_another` red.

## USER requirement check
- Reuse the existing state-aware selection authority: satisfied in kind. `select_next_effect` is the single select-next kernel (CR 613.8b loop rule, oldest-loop-member tiebreak) called by the ability-layer selector and by the text routine; the word-pair `depends_on` arm is deleted (the prototype compiled with it gone and the fixed graph no longer sees `SubstituteTextWord`, which `bucket_effects_by_layer` already drops). Residual: the select/apply/repeat loop (about ten lines) is restated per layer because the edge producer differs (cloned recipient text vs cloned `GameState`); that is the one repeated shape and is justified. It requires `pending` timestamp-sorted on entry (the kernel's `(0..i)` tiebreak reads index order); `active_text_substitutions` supplies that through `order_by_timestamp`.
- One effect at a time against current text (CR 613.8a-c): holds; M2 shows stale edges are wrong, M3 shows timestamp order is wrong.
- Failing-first production regression for the maintainer example: Island ABC red at base (Forest/{G}) and CAB red (Swamp/{B}); the rules-text variants: Terror production row red at base (ABC), Bog Wraith/Bad Moon rows per r1.
- Oracle text and CRs: Magical Hack, Crystal Spray, Sleight of Mind verbatim per r1 jq; Sunken Hollow `["Island","Swamp"]` and Terror loaded from the fixture. CRs 613.8a/b/c, 613.7, 613.1c, 612.1, 612.2, 305.6, 514.2, 611.2a grepped, subjects match; 205.3i see F2.

## r1 findings, closure
- F1 (type-line collapse) closed: the delta moves the dedupe after the routine (`collapse_subtype_repeats`), V1d and M4 added; measured above (BA `["Forest"]` under the design, `["Swamp","Forest"]` under M4). Single-member claim re-measured: `git grep -nE 'retain|dedup|HashSet|BTreeSet' -- crates/engine/src/game/text_substitution.rs` lists the import, the subtype `seen` HashSet/`retain`, and the intrinsic-ability `retain`; `git grep -nE 'Set<[^>]*(ManaColor|BasicLandType)' -- crates/engine/src/types` is empty, with the same pattern shape matching `HashSet<(ObjectId, ...CoreType)>` in `game_state.rs` as control.
- F2 (stack-spell production row) closed: V2p measured red at base for ABC, with both reach-guards (none, AB) green and ACB/CAB green.
- F3, F4 closed: `bucket_effects_by_layer` drops only `SubstituteTextWord`; `SetTextName`/`SetChosenName` are `Layer::Text` (`types/layers.rs`); the chain+loop derivation and M1 wording are in the plan.

## Findings

### F1 [behavior] No real-card control where the recipient starts on a word other than the first effect's `from`; the maintainer's third review asks for it
Maintainer ask (review-9669-3.md, HIGH): "real-card cast/resolve controls for this false three-cycle, a genuine loop and different starting recipient words". Coverage by row name:
1. False three-cycle: covered by V1 (Island, orders ABC and CAB, guard C absent; the result is read the same turn Crystal Spray is live).
2. Genuine loop with timestamp fallback: covered literally by V4's preserved rows `loop_of_text_changes_applies_timestamp_order` and `chain_plus_loop_on_one_object_and_unrelated_loop_on_another` (real Magical Hack and Bog Wraith through `cast_text_change`, not synthetic), but only on a landwalk keyword; there is no loop row on the type-line recipient where the maintainer's witness lives.
3. Different starting recipient words: not covered. Every cycle row starts on `A.from` (V1 Island, V2 Swamp walk, V2p/V3 black). V3's three creatures are recipients of Bad Moon's static, not text recipients, so the starting word of the changed text is always Black. Starting on `B.from` or `C.from` changes the verdict and is cast-order dependent, so it is the case an order-independent shortcut would get wrong.
Required rows (production cast/resolve, real cards; each red at base as measured above; derived by hand from CR 613.8a-c before measurement and equal to the prototype):
- V1x: the plan's cycle (A=Island->Swamp, B=Forest->Island, C=Swamp->Forest) on a real Forest: ABC `["Forest"]`/{G}, CAB `["Swamp"]`/{B}; on a real Swamp: ABC `["Island"]`/{U}, CAB `["Swamp"]`/{B}. Derivation (Forest, CAB): C is a no-op on Forest and nothing depends on it, so timestamp applies it first; then A depends on B (B creates the Island A rewrites), B applies, A applies: Swamp. (Forest, ABC): B is independent (A depends on it), so B, then C depends on A, A, C: Forest. Swamp start mirrors it (ABC: A no-op first, C before B, then B: Island; CAB: C first, then B before A: Swamp). Base: Forest ABC/CAB `Island`/`Island`; Swamp ABC/CAB `Forest`/`Island`. Discriminating by M2 and M3 as for V1.
- V1L: a genuine loop on the type-line recipient: Island with A=Island->Swamp and D=Island->Forest (same `from`, each removes the Island the other rewrites): AD ends `["Swamp"]`/{B}, DA ends `["Forest"]`/{G}, with a third effect C=Swamp->Plains cast after A and D ending `["Plains"]`/{W} (A, D loop; C depends on A). Preservation row (green at base and under the design, so not a discriminator); it exercises the timestamp fallback with the mana readout on the type-line recipient.
Add all rows to the Verification Matrix and Reference Readings (the V1x derivations belong to Reference Readings; no figures beside a list).

### F2 [text] CR 205.3i cited for a rule it does not state
Plan, Design principle: "the set collapse (CR 205.3i) runs once after the last change applies". CR 205.3i (grep, `docs/MagicCompRules.txt`) only lists the land types ("Lands have their own unique set of subtypes ... See rule 305.6"); it says nothing about repeated subtypes collapsing, so it supports "land-type words are on the type line" (the claim at the pre-existing comment), not the collapse. Replacement: "the engine's subtype list holds no repeats (a representation fact, not a rule), so the collapse runs once after the last change applies"; the executor's `collapse_subtype_repeats` doc carries no CR number for the dedupe and keeps CR 205.3i + CR 612.1 only on the type-line rewrite.

## Constraints for the executor (exit-round material, no re-review)
- `apply_in_dependency_order`'s `apply` parameter is `fn(&mut R, &TextSubstitution) -> usize`, while `rewrite_resolved_ability` takes `(substitution, ability)`: reorder its parameters (and its recursion) to `(ability, substitution)`; compile-forced (measured).
- `layers.rs` inline tests (e.g. the `TextSubstitutionSpec::Chosen` rows) may reach `TextSubstitution`/`TextSubstitutionSpec` through the file's imports: drop the top-level imports only if the lib test target still compiles (unmeasured; my prototype built the integration target only).

## Pre-existing (untagged, non-blocking, not repeated)
- None new. (r1: wrong assertion message in `chain_plus_loop_on_one_object_and_unrelated_loop_on_another`, carried by plan step 3.)
