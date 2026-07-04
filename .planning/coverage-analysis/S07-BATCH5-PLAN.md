# S07-BATCH5-PLAN — final Condition_If tranche (3 Standard cards)

Worktree `/home/lgray/vibe-coding/s07-impl-wt`, branch `feat/std-s07-condition-if`, HEAD e259f2e45. All line numbers verified against live code this session. All CR numbers grep-verified (appendix at bottom).

## TL;DR verdict per card

| Card | Effort | New variant? | Parser change? | Swallow clears via |
|---|---|---|---|---|
| Sonic Shrieker | **Trivial** (detector-only) | No | **No** | structural exemption (mirror Screaming Nemesis) |
| Slumbering Trudge | Small | No (reuse `ReplacementCondition::OnlyIfQuantity`) | Yes (1 new dispatch arm) | `"condition":{` AST marker |
| Avatar Aang | Medium | **Yes**: bare `QuantityRef::BendTypesThisTurn` | Yes (1 condition arm; attach is already wired) | `"condition":{` + new Duration marker |

**No deferrals. Aang's "per-turn bend tracking" infra ALREADY EXISTS** (`Player::bending_types_this_turn: HashSet<BendingType>`, populated by `game/bending.rs:record_bending`, cleared `game/turns.rs:701`). The only gap is a `QuantityRef` leaf that reads its `.len()`. This is NOT multi-day infra — it is one variant + one resolver arm + one parse phrase.

---

## CARD 1 — Sonic Shrieker  (detector-only fix)

Oracle: "Flying\nWhen this creature enters, it deals 2 damage to any target and you gain 2 life. If a player is dealt damage this way, they discard a card."

### Trace-before-build
Analogous shipped card: **Screaming Nemesis** ("If a player is dealt damage this way, they can't gain life…"). Traced end-to-end:
- Parser: `oracle_effect/subject.rs:4600 strip_dealt_damage_this_way_player_anaphor` recognizes "if a player is dealt damage this way, they " and (caller `subject.rs:1157`) binds the follow-up `affected: TargetFilter::ParentTarget`. The `ParentTarget` binding IS the CR 608.2c "this way" back-reference (it resolves to the damage target only when that target is a player).
- Swallow: `swallow_check.rs:900 def_tree_has_parent_target_cant_gain_life` + `:933 any_ability_has_dealt_damage_this_way_life_lock`, invoked from `detect_condition_if` at `:2158`, exempts it because the anaphor is *structurally represented*, not swallowed.

**Key finding — Sonic Shrieker is ALREADY parsed correctly.** `jq '.["sonic shrieker"]'` shows the trigger chain `DealDamage{2, Any}` → `GainLife{2}` → `Discard{count:1, target: ParentTarget}` (SequentialSibling). I traced the discard resolver (`game/effects/discard.rs`):
- Player-target case: `ParentTarget` yields no `TargetRef::Object`, so `specific_targets` is empty → falls to the else-branch (`discard.rs:281`) → `resolve_player_for_context_ref` (`effects/mod.rs:4587`, `ParentTarget→Player` arm at `:4628`) correctly makes **that damaged player** discard a card of their choice. Correct.
- Creature/planeswalker-target case: `ParentTarget` is an Object, `object_bound_discard` true (`discard.rs:187`), loop hits `if obj.zone != Zone::Hand { continue }` (`:208`) → no-op. Correct ("if a *player*…").

So the ONLY defect is the spurious `Condition_If` swallow warning. The follow-up discard already resolves right.

### New-variant decision
None. No `Effect`/`AbilityCondition`/`QuantityRef` change. This is the same architecture Screaming Nemesis ships (`review-engine-plan` already accepted the structural-anaphor representation for the sibling; adding a `condition` here while the sibling has none would be inconsistent).

### Swallow-clearing mechanism  [REVISED per review FIX-B — do NOT touch the :2158 CantGainLife helper]
**REVIEW CORRECTION (FIX-B):** the original plan proposed generalizing `def_tree_has_parent_target_cant_gain_life` (called at `:2158`) to also match `Discard{ParentTarget}`. That is SELF-INCONSISTENT: the `:2158` helper runs BEFORE `stripped` is computed at `:2187`, so the intended `stripped.contains("dealt damage this way")` text-gate CANNOT apply there — a structural generalization at `:2158` would over-suppress Condition_If for ANY card with a `Discard{ParentTarget}` sub-ability + a bare " if " (Dread Fugue-class collateral). **Leave the CantGainLife helper and its `:2158` call site UNTOUCHED.**

Instead add a NEW text-gated exemption branch AFTER `stripped` is computed (`swallow_check.rs:2187`), mirroring the existing "lost/gained life this way" branch at `:2259-2264`:
```
// CR 608.2c: "if a player is dealt damage this way, they discard" — the ParentTarget
// discard rider is structurally represented (Effect::Discard{target:ParentTarget}); the
// leading "if" is the CR 608.2c back-reference, not a swallowed game-state condition.
// allow-noncombinator: swallow detector marker scan on classified text
if stripped.contains("dealt damage this way") && any_ability_has_parent_target_discard(parsed) {
    return;
}
```
Add the small walker `any_ability_has_parent_target_discard(parsed)` — returns true when any ability/trigger def-tree contains `Effect::Discard{ target: TargetFilter::ParentTarget, .. }` in a sub-ability (recurse `sub_ability`/`else_ability`/`mode_abilities`, mirror the existing `any_ability_has_dealt_damage_this_way_life_lock` walker at `:933` for structure but keyed on Discard). The `stripped.contains("dealt damage this way")` gate + the structural Discard{ParentTarget} check TOGETHER prevent any over-suppression (a bare Discard{ParentTarget} without the damage anaphor never matches).
Class covered: every "if a player is dealt damage this way, they [ParentTarget-scoped discard]" rider (Sonic Shrieker today; extend the walker for lose-life / mill riders later).

### CR sections
- CR 608.2c (`docs/MagicCompRules.txt:2793`) — "later text… may modify the meaning of earlier text"; the "this way" back-reference.
- CR 119.7 (`:1077`) — governs the sibling life-lock (cite in the shared helper doc-comment for continuity).
- CR 701.9b (`:3331`) — affected player chooses which card to discard (why the player-target case is a choose-discard, not a random one).
- CR 615.5 (`:3149`) / CR 120.3 (`:1097`) — the prevented-damage edge (see below).

### Rules-fidelity note (precedent-consistent simplification, NOT a deferral)
If the 2 damage to a player is fully *prevented* (CR 615.5), the player was not "dealt damage this way" and should not discard; the structural (unconditional) representation would still discard. This is the **identical** fidelity ceiling Screaming Nemesis already ships (its CantGainLife also grants regardless of prevention). Mark with a `// ponytail:` comment naming the ceiling: "prevented-damage edge not gated; upgrade path = damage-recipient `AbilityCondition` when a card forces it — track with Screaming Nemesis." Shipping this matches accepted precedent; it is `supported:true, gap_count:0` for the printed behavior.

### Discriminating tests (`/card-test`, revert-fails)
1. `sonic_shrieker_etb_damages_player_forces_discard`: opponent hand=2. Cast/ETB Sonic Shrieker, `DealDamage` target = opponent (player). Resolve. Assert opponent hand delta = −1 (discarded), controller life +2, opponent life −2. Reverting the exemption does NOT fail this (behavior already works) — so **this test's real teeth are on the warning**; add the assertion below.
2. `sonic_shrieker_no_condition_if_swallow`: parse the verbatim Oracle text; assert `!has_swallowed_detector(&parsed, "Condition_If")`. This FAILS on revert of the detector change — the discriminating regression guard.
3. `sonic_shrieker_creature_target_no_discard` (reach-guard for the negative): ETB damage target = a creature; assert no `Discarded` event / opponent hand unchanged. Pairs with test 1 so the negative isn't vacuous.

### Files (Sonic Shrieker)
- `crates/engine/src/parser/swallow_check.rs` (`:900`, `:928-931`, `:933`, `:2150-2160`) — generalize helper + call.
- `crates/engine/src/parser/swallow_check.rs` inline `#[test]` module — add tests 2/3 shape.
- Runtime test file (see Aang §Files for the harness module) — test 1.
No parser/resolver/type changes.

---

## CARD 2 — Slumbering Trudge  (reuse `OnlyIfQuantity`)

Oracle: "This creature enters with a number of stun counters on it equal to three minus X. If X is 2 or less, it enters tapped. (…)"

### Trace-before-build
Current `jq` shows TWO `Moved` replacements: (1) `PutCounter{stun, Offset(3, Multiply(-1, Ref(CostXPaid)))}` ✓ correct = 3−X; (2) `SetTapState{Tap, SelfRef}` **with `condition: null`** — taps UNCONDITIONALLY. That is the bug + the swallow source.

Sentence plumbing: `oracle.rs:199 parse_replacement_sentence_sequence` → `:207 parse_replacement_sentences` (nom `all_consuming(many1)`) → per-sentence `:217 parse_replacement_line`. Sentence 2 "If X is 2 or less, it enters tapped." reaches `oracle_replacement.rs:53 parse_replacement_line`; today it falls through every conditional arm to the **unconditional** enters-tapped guard at `oracle_replacement.rs:202-225` (guard only excludes `" unless "` and `"if you control"`, so the X-comparison slips through and the condition is dropped).

Analogous shipped path: `oracle_replacement.rs:2303 parse_enters_tapped_if_controls` → builds `SetTapState` replacement `.condition(ReplacementCondition::IfControlsMatching{..})`, dispatched at `:163` **before** the unconditional guard. This is the exact sibling to copy. The typed replacement gate `ReplacementCondition::OnlyIfQuantity { lhs, comparator, rhs, active_player_req }` already exists (`types/ability.rs:15928`) and is evaluated at `game/replacement.rs:3953` via `resolve_quantity(state, lhs, controller, source_id)`. `QuantityRef::CostXPaid` resolves in this exact ETB-replacement context (`game/quantity.rs:2885`, reads `obj.cost_x_paid`) — proven already-working by replacement (1)'s counter count on the same `Moved` event.

### New-variant decision
**Reuse, no new variant.** `ReplacementCondition::OnlyIfQuantity` + `QuantityRef::CostXPaid` + `Comparator::LE` + `QuantityExpr::Fixed` are all existing typed building blocks. (add-engine-variant gate: not triggered — extending an existing enum's *use*, not adding a variant.)

### Implementation
1. New helper `oracle_replacement.rs::parse_enters_tapped_if_x_comparison(norm_lower, original) -> Option<ReplacementDefinition>`, modeled byte-for-byte on `parse_enters_tapped_if_controls` (`:2303`):
   - Guard on `scan_contains("enters tapped")` / `"enters the battlefield tapped"`.
   - nom: `tag("if x is ")` then a typed suffix-comparator combinator (reuse the shipped pattern at `oracle_nom/condition.rs:3468-3469`: `value(Comparator::GE, alt((tag(" or greater"), tag(" or more"))))`, `value(Comparator::LE, alt((tag(" or less"), tag(" or fewer"))))`) parsed around `nom_primitives::parse_number`. Order: number then comparator suffix ("2 or less").
   - Confirm the tail is `[it|~|this creature] enters tapped`.
   - Build `.execute(SetTapState{SelfRef, Single, Tap})`, `.valid_card(SelfRef)`, `.destination_zone(Zone::Battlefield)`, `.description(original)`, `.condition(ReplacementCondition::OnlyIfQuantity{ lhs: QuantityExpr::Ref{qty: QuantityRef::CostXPaid}, comparator, rhs: QuantityExpr::Fixed{value: n as i32}, active_player_req: None })`.
2. Dispatch: call it in `parse_replacement_line` immediately **after** `parse_enters_tapped_if_controls` (`:163-165`) and before the unconditional guard.
3. Belt-and-suspenders: extend the unconditional guard (`:204-205`) with `&& !nom_primitives::scan_contains(&norm_lower, "if x")` so no future path re-drops it. (Ordering already prevents it; this documents intent.)

Rules note: this is a *replacement gate*, so the domain authority is `ReplacementCondition` (as with every enters-tapped-conditional in this file), not `parse_inner_condition` — which is the game-state-condition authority for ability/trigger conditions (Aang). Building `OnlyIfQuantity` directly mirrors the accepted `IfControlsMatching` sibling exactly.

### Swallow-clearing mechanism
Once replacement (2) carries `condition: Some(OnlyIfQuantity{..})`, the serialized `ast_json` contains `"condition":{"type":"OnlyIfQuantity"…`. `detect_condition_if` (`swallow_check.rs`) hits the `" if "` `has_marker` gate (`:2287`), then the `cond_markers` scan finds `"condition":{` (`:2294`) → returns, no warning.

### CR sections
- CR 107.3 (`:464`) — X placeholder / value determination.
- CR 614.1c (`:3060`) — "[This permanent] enters with…" / "enters as…" are replacement effects (the enters-tapped + enters-with-counters shapes).
- CR 614.1d (`:3062`) — continuous "enters…" replacement, applicability gate for the conditional tap.
- CR 122.1 (`:1178`) — stun counters are counters (replacement 1, already shipped; cite for continuity).
Ruling captured (card `rulings`): "If it enters without being cast, X is 0" ⇒ `CostXPaid` defaults 0 (`quantity.rs:2891 unwrap_or(0)`), so X≤2 is true and it enters tapped with 3 counters. `OnlyIfQuantity{CostXPaid ≤ 2}` reproduces this automatically. Add a test.

### Discriminating tests (`/card-test`, revert-fails)
1. `slumbering_trudge_x1_enters_tapped_two_counters`: cast with X=1 (pay {G}). Assert on the battlefield object: 2 stun counters (3−1) AND tapped. (X≤2 ⇒ tapped.)
2. `slumbering_trudge_x3_enters_untapped_zero_counters`: cast with X=3. Assert 0 stun counters AND **untapped**. This is the discriminating case — on revert (unconditional tap) the untapped assertion FAILS.
3. `slumbering_trudge_no_condition_if_swallow`: parse verbatim; assert `!has_swallowed_detector("Condition_If")`.
Class covered: general "if X is N or less/greater/more/fewer, it enters tapped" cast-X-comparison enters-modifier (any future X-gated ETB tap).

### Files (Slumbering Trudge)
- `crates/engine/src/parser/oracle_replacement.rs` — new `parse_enters_tapped_if_x_comparison` (near `:2303`), dispatch at `:163-165`, guard tweak `:204-205`.
- Inline parser `#[test]` in same file — assert the `ReplacementDefinition.condition` shape.
- Runtime test module — tests 1/2.
- Snapshot: `crates/engine/tests/oracle_parser.rs` insta if Trudge is snapshotted (check; update if so).

---

## CARD 3 — Avatar Aang  (one bare QuantityRef; attach path already wired)

Oracle: "Flying, firebending 2\nWhenever you waterbend, earthbend, firebend, or airbend, draw a card. Then if you've done all four this turn, transform Avatar Aang."

### Trace-before-build
Current `jq`: `TriggerMode::ElementalBend` → `Draw{1, Controller}` → `Transform{SelfRef}` (SequentialSibling) with **no condition**, so it transforms on *every* bend. Two swallows: `Condition_If` + `Duration_ThisTurn`.

Per-turn bend tracking — **already exists**:
- `types/player.rs:167 pub bending_types_this_turn: HashSet<BendingType>` (init `:228`).
- `game/bending.rs:6 record_bending` inserts the `BendingType` (`:38`) and emits the Firebend/Airbend/Earthbend/Waterbend `GameEvent`.
- `game/turns.rs:701` clears it each turn.
"you've done all four this turn" ⇔ `bending_types_this_turn.len() >= 4` (exactly 4 possible types ⇒ `>=4` == all four).

Condition-attach wiring — **already exists** and already runs on Aang's second sentence:
- `oracle_effect/conditions.rs:236 strip_leading_general_conditional` handles the `"then if "` prefix (`:247`), strips it, and calls `try_nom_condition_as_ability_condition` (`:259`). Today that returns `None` (`parse_inner_condition` can't parse "done all four this turn"), so the condition is dropped but `Transform` still parses — exactly the current AST. Make `parse_inner_condition` recognize the phrase and the condition auto-attaches to the `Transform` sub-ability.
- Bridge: `conditions.rs:3688 static_condition_to_ability_condition` maps `StaticCondition::QuantityComparison → AbilityCondition::QuantityCheck` (`:3694-3702`); tail fallback `:5065-5069` routes any fully-consumed `parse_inner_condition` result through it.
- Runtime gate: `AbilityCondition::QuantityCheck` evaluated at `game/effects/mod.rs:7812` via `resolve_quantity_for_ability_condition` — a sub-ability with `condition` that evaluates false is skipped. CR 603.4 intervening-if semantics.

### New-variant decision (add-engine-variant gate — RAN)
- Existence check: grepped `data/engine-inventory.json` — `BendingType`, `bending`, `PlayerActionsThisTurn`, `PlayerActionKind` present; **no** bend-count `QuantityRef`.
- Parameterize-first: `QuantityRef::PlayerActionsThisTurn { action: PlayerActionKind }` (`ability.rs:4884`) is the nearest sibling but is WRONG — `PlayerActionKind` (`events.rs:96`) has no bend arm, and it counts *occurrences* not *distinct types* ("done all four" needs 4 distinct types; four firebends must not qualify). Forcing bending into `PlayerActionKind` mismodels the mechanic (bending is a keyword action with its own `GameEvent` + dedicated set, not a `PlayerActionKind`).
- Decision: **add bare `QuantityRef::BendTypesThisTurn`** (Controller-scoped, no field), reading `player.bending_types_this_turn.len()`. This exactly mirrors the existing bare sibling variants `CrimesCommittedThisTurn` (`ability.rs:4699`, resolver `quantity.rs:2565`) and `DescendedThisTurn` (`:4826`) — both bare, Controller-scoped reads of a dedicated per-player/state counter. It is the distinct-type-cardinality axis, categorically distinct from occurrence counts. Justified.

### Implementation
1. `types/ability.rs` `enum QuantityRef` (near `:4699`): add `/// CR 701.65b/701.66b/701.67b/702.189b: distinct bend types (water/earth/fire/air) the controller has performed this turn. Reads Player::bending_types_this_turn.len(). BendTypesThisTurn`.
2. `game/quantity.rs`: (a) resolver arm near `:2565` (model on `CrimesCommittedThisTurn`): `QuantityRef::BendTypesThisTurn => player.map_or(0, |p| crate::game::arithmetic::usize_to_i32_saturating(p.bending_types_this_turn.len()))`. (b) Add `| QuantityRef::BendTypesThisTurn` to the three exhaustive classification match groups at `:386/:406`, `:638/:658`, `:823/:843` (turn-history/no-recipient group — same arm as `CrimesCommittedThisTurn`). The exhaustive matches will fail to compile until all are updated — that is the safety net; grep every `QuantityRef::CrimesCommittedThisTurn` site and co-locate.
   **[REVIEW FIX-A] TWO ADDITIONAL EXHAUSTIVE SITES the original site list MISSED (both no-wildcard compile-error nets — triangulated from CrimesCommittedThisTurn/DescendedThisTurn/TurnsTaken/DungeonsCompleted):**
   - `game/triggers.rs` — `ability_condition_refs_cost_paid_object` quantity walker: `CrimesCommittedThisTurn` sits in the `=> false` non-object group near `:6804` (closes `:6825`; lines `:6773-6777` document "no wildcard … caught by the compiler"). Add `| QuantityRef::BendTypesThisTurn` to that `=> false` group.
   - `game/ability_scan.rs` — `scan_quantity_ref` (exhaustive, closes `:1959`, no wildcard); `CrimesCommittedThisTurn => Axes::NONE` at `:1764`, `Descended` at `:1853`. Add `QuantityRef::BendTypesThisTurn => Axes::NONE` (near `:1958`) — a controller turn-accumulator leaf: no event/sibling/projected axis, matches the Crimes/Descended arm. This is the fail-closed growing-cascade walker; DO NOT default it to a wildcard.
   mtgish-import has only construction sites (no exhaustive consuming match) → will NOT break.
3. `game/coverage.rs`: add describe arm near `:1432/:1505` ("distinct bend types this turn") and coverage_kind `("BendTypesThisTurn", Handled)` near `:6545`.
4. Parser `oracle_nom/condition.rs`: add `parse_youve_bending_history_condition` (model on `parse_youve_player_action_history_condition:4161`): `value(make_quantity_ge(QuantityRef::BendTypesThisTurn, 4), tag("done all four this turn"))`. Register it inside `parse_youve_this_turn`'s `alt` (`:3986-4001`). This yields `StaticCondition::QuantityComparison{Ref(BendTypesThisTurn), GE, Fixed(4)}` via `make_quantity_ge` (`:2746`). No literal string dispatch outside the nom `tag`.

### Swallow-clearing mechanism
Attaching the condition to `Transform` puts `"condition":{"type":"QuantityCheck"…}` with `BendTypesThisTurn` in the AST.
- `Condition_If`: `detect_condition_if` marker scan finds `"condition":{` (`swallow_check.rs:2294`) → cleared.
- `Duration_ThisTurn`: `detect_duration_this_turn` (`:2705`) clears via AST marker list — **add `"BendTypesThisTurn"` to the `markers` slice (`:2825-2951`)**, alongside the existing `PlayerActionsThisTurn`/`CardsDrawnThisTurn`/etc. turn-history markers. (The "done all four this turn" phrase is not in `QUANTITY_CONTEXT_SUFFIXES`, so the marker path is the correct clear.)

### CR sections
- CR 603.4 (`:2592`) — intervening-if ("Then if you've done all four this turn, transform"): condition re-checked on resolution; the ability does nothing if false. Exactly `AbilityCondition::QuantityCheck` on the sub-ability.
- CR 608.2c (`:2793`) — "done all four this turn" back-references the four bend verbs in the trigger head.
- CR 701.65b / 701.66b / 701.67b (`:3841/:3847/:3853`) — Airbend/Earthbend/Waterbend "triggers whenever a player [bends]" (what `record_bending` records).
- CR 702.189b (`:5403`) — firebends "whenever a firebending ability they control resolves".
- CR 701.27a (`:3526`) + CR 712.13 (`:5896`) — Transform action (Aang is a transforming DFC).

### Discriminating tests (`/card-test`, revert-fails)
1. `avatar_aang_transforms_after_all_four_bends`: put Aang on battlefield; drive four bends of distinct types this turn (or seed `bending_types_this_turn` with all four then fire one `ElementalBend`). Assert after the fourth resolution: a `Draw` occurred each time AND Aang is now back-face (transformed). Revert (condition dropped) → still transforms early, but this passes; teeth are in test 2.
2. `avatar_aang_no_transform_on_partial_bends`: only 2 distinct bend types this turn; fire `ElementalBend`; assert Draw happened but Aang did NOT transform. On revert (unconditional Transform) this FAILS — the discriminating guard. Pair with test 1 as the positive reach-guard.
3. `avatar_aang_no_swallow`: parse verbatim; assert neither `Condition_If` nor `Duration_ThisTurn` swallow present.
4. (parser) inline: `parse_inner_condition("done all four this turn")` (or via `parse_condition("if you've done all four this turn")`) == `QuantityComparison{Ref(BendTypesThisTurn), GE, Fixed(4)}`.

### Files (Avatar Aang)
- `crates/engine/src/types/ability.rs` (`enum QuantityRef` near `:4699`).
- `crates/engine/src/game/quantity.rs` (resolver `:2565` area; 3 classification groups `:386/:638/:823`).
- `crates/engine/src/game/triggers.rs` **[FIX-A]** (`ability_condition_refs_cost_paid_object` `=> false` group near `:6804`).
- `crates/engine/src/game/ability_scan.rs` **[FIX-A]** (`scan_quantity_ref` near `:1958`, `=> Axes::NONE`).
- `crates/engine/src/game/coverage.rs` (`:1432`, `:6545`).
- `crates/engine/src/parser/oracle_nom/condition.rs` (`parse_youve_this_turn:3986`; new arm).
- `crates/engine/src/parser/swallow_check.rs` (`Duration_ThisTurn` markers `:2825`).
- Runtime tests — new module or existing S07 batch test file.

**NOTE:** Aang's `swallow_check.rs` edit (Duration marker `:2825`) is in a DIFFERENT region from Increment-A/Sonic Shrieker's `swallow_check.rs` edit (new text-gated branch after `:2187` + `any_ability_has_parent_target_discard` walker). If Increment A commits first, Increment B's executor must re-read `swallow_check.rs` before editing (multi-agent safety) — the regions don't overlap but the file will be dirty.

### STOP/escalation flag
None. The "deep infra" candidate (per-turn distinct-bend tracking) already ships; this is one leaf + one arm + one phrase. If test 1's ParentTarget/transform chain surprises, that is orthogonal and already hardened by HEAD's `a410d2d74`/`02c105602`.

---

## Sequencing (executor increments)

Shared-file collision map — the three cards are almost fully disjoint:
- Sonic Shrieker: `swallow_check.rs` only.
- Slumbering Trudge: `oracle_replacement.rs` only (+ maybe snapshot).
- Avatar Aang: `types/ability.rs`, `game/quantity.rs`, `game/coverage.rs`, `oracle_nom/condition.rs`, `swallow_check.rs`.
Only `swallow_check.rs` is touched by both Sonic Shrieker (helper `:900/:933/:2158`) and Aang (Duration marker `:2825`) — **different, non-overlapping regions**, no real conflict.

Recommended: **two increments**, one commit each (cargo fmt + clippy + `cargo test -p engine` + `cargo coverage` between).
- **Increment A — Sonic Shrieker + Slumbering Trudge together** (disjoint files, both small, both detector/replacement-local). Fastest path to 2 cards green.
- **Increment B — Avatar Aang** (the only card touching the type system + resolver + coverage; the new `QuantityRef` forces exhaustive-match edits across `quantity.rs`, so isolate it to keep the diff auditable).
Aang does NOT need event-tracking infra (already present), so it need not be first or specially staged — but keep it separate purely for the multi-file `QuantityRef` blast radius.

---

## CR-verification appendix (every number grep'd against docs/MagicCompRules.txt this session)

| CR | line | text (head) |
|---|---|---|
| 107.3 | 464 | "Many objects use the letter X as a placeholder…" |
| 119.4 | 1067 | pay life = lose that much life |
| 119.7 | 1077 | "If an effect says that a player can't gain life…" |
| 120.3 | 1097 | damage results depend on recipient |
| 120.6 | 1130 | marked damage remains until cleanup |
| 122.1 | 1178 | "A counter is a marker placed on an object or player…" |
| 400.7 | 1950 | new object, no memory (LKI) |
| 603.4 | 2592 | intervening-"if" clause rule |
| 608.2c | 2793 | later text modifies earlier text / "this way" |
| 614.1a | 3056 | "instead" = replacement |
| 614.1c | 3060 | "[permanent] enters with…/as…" = replacement |
| 614.1d | 3062 | continuous "enters…" = replacement |
| 615.1 | 3138 | prevention effects |
| 615.5 | 3149 | prevention + additional effect |
| 701.9 | 3327 | Discard |
| 701.9b | 3331 | affected player chooses which card to discard |
| 701.27 | 3524 | Transform |
| 701.27a | 3526 | "turn it over so other face is up" |
| 701.65b | 3841 | Airbend trigger |
| 701.66b | 3847 | Earthbend trigger |
| 701.67b | 3853 | Waterbend trigger |
| 702.189 | 5399 | Firebending keyword |
| 702.189b | 5403 | firebends trigger |
| 712.13 | 5896 | DFC face-up on resolution |

NOT verified / do not cite: `603.4c` (absent — use `603.4`). No other cited number is unverified.

Ready for review-engine-plan. All three ship `supported:true, gap_count:0` with no deferral.
