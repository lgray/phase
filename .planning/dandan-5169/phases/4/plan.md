# Phase 4 plan — PREREQ-0: Memory Lapse swallow-check false positive

Planned at HEAD 41b2e353 (Phase 3's uncommitted `phase-ai/.../fixed_deck_keepables.rs` ignored; disjoint from this scope).
Skills applied: `oracle-parser` (parser gates), `engine-planner` phase-plan mode. No enum variant, serialized surface or pipeline-shape change, so `add-engine-variant`, `add-card-data-pipeline` and `card-test` checklists are N/A (detector-only change; the coverage pipeline output is regenerated, not edited).

## Premise verification (Step 0)
Oracle text read verbatim from `client/public/card-data.json` (`.oracle_text`):
- Memory Lapse: "Counter target spell. If that spell is countered this way, put it on top of its owner's library instead of into that player's graveyard."
- Lapse of Certainty: identical text to Memory Lapse.
- Remand: "Counter target spell. If that spell is countered this way, put it into its owner's hand instead of into that player's graveyard.\nDraw a card."
- Spell Crumple: "Counter target spell. If that spell is countered this way, put it on the bottom of its owner's library instead of into that player's graveyard. Put Spell Crumple on the bottom of its owner's library."
- Hinder: "Counter target spell. If that spell is countered this way, put that card on your choice of the top or bottom of its owner's library instead of into that player's graveyard."

## Measured claims (commands run; no cargo was run)
1. `effect_is_replacement_carrier` (`swallow_check.rs`) matches only `CreateDrawReplacement | CreatePlaneswalkReplacement | ExileResolvingSpellInsteadOfGraveyard => true, _ => false`; no `Effect::Counter` arm. Read at HEAD.
2. `detect_replacement_instead` fires when cleaned text contains " instead", `parsed.replacements` is empty, and none of the earlier exemptions hold; the last exemption is `any_ability_has_replacement_carrier`, which walks `def_tree_has_replacement_carrier` (effect, sub, else, delayed-trigger, mode abilities). Read at HEAD.
3. Parsed AST per member (`jq` over `.abilities[]` of `card-data.json`), `Effect::Counter.countered_spell_zone`:
   - Memory Lapse, Lapse of Certainty: `Library { Top }`, no sub_ability.
   - Remand: `Hand`, sub_ability `Draw`.
   - Spell Crumple: `Library { Bottom }`, sub_ability `PutAtLibraryPosition`.
   - Hinder: `null`; sub_ability `ChangeZone` to Graveyard with `ZoneChangedThisWay` condition and no `else_ability` (not a represented branch).
4. `coverage-data.json` (run base): all five are `supported:false`, `gap_count:1`, sole gap `Swallow:Replacement_Instead`. So removing that one gap flips exactly the four zone-carrying members to supported; Hinder keeps its gap.
5. Pool-wide: 48 cards carry `Swallow:Replacement_Instead`; only 4 cards in `card-data.json` have any `countered_spell_zone != null` (lapse of certainty, memory lapse, remand, spell crumple), and all 4 are among the 48. The accepted arm therefore cannot silence any other card in the current pool.
6. Runtime honours the zone: `game/effects/counter.rs` reads `countered_spell_zone`; integration test `tests/integration/counter_spell_zone_redirect.rs` exists (both grepped as referencing the field/cards). So the swallow flag is a detector false positive, not a dropped clause, for the four members.
7. CRs grepped in `docs/MagicCompRules.txt`: 701.6a (countered spell goes to graveyard), 614.1a ("instead" effects are replacement effects), 608.2c (its worked example is exactly "Counter target spell. If that spell is countered this way, put it on top of its owner's library instead of into its owner's graveyard"), 614.15 (self-replacement effects of a resolving spell).

## Design (one unit)
Choice between the brief's two options: extend `effect_is_replacement_carrier` with a typed `Effect::Counter { countered_spell_zone: Some(_), .. }` arm, rather than reusing `evidence.has_slot("countered_spell_zone")`. Reason: the `Replacement_Instead` detector already routes every effect-borne replacement through `effect_is_replacement_carrier` via `any_ability_has_replacement_carrier`; the typed arm is tree-walked (covers a Counter nested in a sub_ability/trigger) and needs no new call site, whereas the slot gate is a string-keyed evidence lookup in a different detector (`Condition_If`) and would add a second exemption path to `detect_replacement_instead`. The arm is folded into the existing or-pattern of `=> true` arms with a one-line CR comment. `Some(_)` (not a variant list) per charter: the field's type IS the redirect destination (`SpellStackToGraveyardReplacement`), and its presence is the replacement.

Edit (the only production change), in `effect_is_replacement_carrier`, add to the `=> true` or-pattern:
```
// CR 614.1a + CR 701.6a + CR 608.2c: "if that spell is countered this way, put it <zone> instead of into the graveyard" — the destination rides on the Counter effect (Memory Lapse, Remand).
| Effect::Counter { countered_spell_zone: Some(_), .. }
```
The edit is placed so the existing `ExileResolvingSpellInsteadOfGraveyard { .. } => true` arm terminator stays last. Do not touch the function's doc comment or the `_ => false` arm.

## Scope matrix (literal paths)
| Path | Change |
|---|---|
| `crates/engine/src/parser/swallow_check.rs` | production: one arm in `effect_is_replacement_carrier`; tests: one `#[test]` in the inline `mod tests` |

Scope-path count: 1. No committed generated artifacts (coverage/card-data output is gitignored and only used as the closing measurement). Nothing else (no `ability.rs`, no parser dispatch, no tests/integration file).

## Sizing
- Units: 1 (accept the countered-spell-redirect carrier in the `Replacement_Instead` swallow check). No inter-unit edges.
- Counted scope paths: 1 (≈15 LOC test + 2 LOC production; well under the charter's ~80).
- No new enum variant, serialized surface, or `WaitingFor`/`GameAction` change.
- T1 fails, T2 fails (consistent with charter entry).

## Pattern Coverage
Class: every card whose parse carries `Effect::Counter.countered_spell_zone: Some(_)`. Measured pool: 4 cards today (Memory Lapse, Lapse of Certainty, Remand, Spell Crumple), and any future "counter ... put it <zone> instead" card the parser already lifts into the field. Charter-class attribution: brief's 5 named members, of which 4 are this class; Hinder is not (claim 3).

## Building Blocks / Logic Placement / Rust Idioms / Extension vs Creation
Reuses `effect_is_replacement_carrier` and its tree walk (`def_tree_has_replacement_carrier`, `any_ability_has_replacement_carrier`); no new helper, type, or detector. Logic stays in the parser diagnostic layer where the detector lives; engine runtime is untouched. Typed-pattern match on the existing `Option<SpellStackToGraveyardReplacement>`, no string matching, no bool. Extends an existing pattern (same or-pattern as `ExileResolvingSpellInsteadOfGraveyard`).

## Nom Compliance
No new parsing, detection or dispatch: the change matches on an already-parsed typed AST field. No `contains`/`starts_with`/`find` is added. Oracle text is never matched.

## Analogous Trace
Traced the `ExileResolvingSpellInsteadOfGraveyard` carrier: `crates/engine/src/parser/swallow_check.rs` `effect_is_replacement_carrier` -> `def_tree_has_replacement_carrier` -> `any_ability_has_replacement_carrier` -> `detect_replacement_instead` (early return), and `Condition_If`'s `has_slot("countered_spell_zone")` gate for the sibling detector.

## Variant Discoverability
No variant added.

## Verification Matrix
Test file: inline `mod tests` of `crates/engine/src/parser/swallow_check.rs`, using the existing `parse_named`, `has_swallowed_detector`, `only_swallow` helpers and verbatim Oracle text with real names.

| Claim | Seam | Test | Revert-failing assertion | Negative / reach-guard |
|---|---|---|---|---|
| Counter-with-zone is a carrier | `effect_is_replacement_carrier` | `replacement_instead_accepts_countered_spell_redirect_carrier`: for Memory Lapse (Library Top), Remand (Hand, with its `\nDraw a card.`), Spell Crumple (Library Bottom), Lapse of Certainty, assert `!has_swallowed_detector(.., "Replacement_Instead")` | removing the new arm makes each report `Replacement_Instead` (executor shows red by reverting the arm, then green) | Positive reach-guard per card BEFORE the negative assertion: the first ability's effect is `Effect::Counter { countered_spell_zone: Some(expected), .. }` with the expected zone, so an upstream `Effect::Unimplemented` (which makes `check_swallowed_clauses` skip the unit) or a dropped zone cannot satisfy the assertion vacuously. The text contains " instead", proving the detector's marker precondition holds. |
| A Counter without the zone is still flagged | same | in the same file, `replacement_instead_keeps_flagging_counter_without_zone`: Hinder verbatim; `only_swallow(&parsed, "Replacement_Instead")` (exactly one warning) | passes before and after the arm (guards over-acceptance: a `Some(_)` widened to any `Counter` would turn it red) | Reach-guard: first ability's effect is `Effect::Counter { countered_spell_zone: None, .. }` and `only_swallow` itself proves the detector fired (so the unit was not skipped as unimplemented). Non-replacement-clause flagging is already pinned by the existing Lava Burst test `replacement_instead_swallow_carries_the_event_antecedent`; it must stay green. |
| Existing suites | - | `cargo nextest run -p phase-engine swallow_check` (whole module green, incl. `condition_if_accepts_countered_spell_redirect`) | - | - |

Hostile-fixture rows: multi-authority / owner-vs-controller / decline paths are not reachable here (a pure diagnostic predicate over static parse output; no game state). First production branch reached by the fixtures: `detect_replacement_instead`'s `any_ability_has_replacement_carrier` early return (positive) and the diagnostic push (Hinder). Hinder's parse has a Graveyard `ChangeZone` sub_ability with `ZoneChangedThisWay` and no `else_ability`, so neither `def_is_represented_instead_branch` nor any earlier exemption rescues it.

Parser-change statement: no Oracle text is newly accepted by the parser; only the diagnostic changes. Coverage honesty: the four members flip to supported because their AST is correct and runtime-honoured (claim 6); Hinder stays `Swallow:Replacement_Instead`, left red (its parse is semantically wrong, sending the card to the graveyard; fixing it is outside this scope and the charter).

### Closing measurement (executor, after the code is green)
Regenerate per the brief (`./scripts/gen-card-data.sh`, then the brief's jq over `client/public/coverage-data.json` with `brief/scripts/names.txt`). Expected: Memory Lapse `supported=true, gap_count=0`; Lapse of Certainty, Remand, Spell Crumple flip likewise; Hinder stays `false`/1 `Swallow:Replacement_Instead`. Also re-run the pool census: `Replacement_Instead` card count 48 -> 44. If the regeneration is too costly under the box limits, the executor reports it as not run rather than claiming it; the unit tests are the gating evidence. A member whose regenerated `gap_count` stays >0 for another detector is reported, not fixed here.

## Reference Readings
Hinder's current parse (Counter + Graveyard ChangeZone) disagrees with the derived reading (owner's choice of top or bottom of library instead of graveyard). This is a pre-existing parse defect; no row here asserts parity with or preservation of it - the Hinder row asserts only that the detector flag stays. Other rows use derived values from verbatim Oracle text (zones: top, hand, bottom, top), each confirmed against the parsed AST by jq.

## Identity / Provenance Contract
N/A: "this way" is resolved by the parser/runtime already (`Effect::Counter` zone); this change neither binds nor reads an object identity.

## Deferral
None (charter deferral list: None). CR annotation: the new arm carries `CR 614.1a + CR 701.6a + CR 608.2c` (all grepped above); the test docstring cites CR 608.2c's Memory Lapse example.

## Executor notes
- Edit only `crates/engine/src/parser/swallow_check.rs`; re-read the file before editing (other agents' work may be present); use targeted Edit, never whole-file writes.
- Run `cargo fmt --all` directly; other checks per worker-env (prefixed `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0`, one cargo at a time, wait for `pgrep -f 'cargo|rustc'` idle).
- Show red: temporarily remove the new arm (Edit), run the new tests to see the positive test fail, restore it, then green.
- Comments: one sentence, no history, no file:line pins.
