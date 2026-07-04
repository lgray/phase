# S25-P2a — Multi-zone name-matched search-and-exile (Deadly Cover-Up, The End)

**Increment:** P2a (FINAL classification B1 — genuine creation, ≥2-card class)
**Worktree:** `/home/lgray/vibe-coding/s25-impl-wt` · branch `feat/std-s25-completion` · HEAD `3abcfef7b`
**Skills applied:** `/add-engine-effect`, `/add-engine-variant`, `oracle-parser`, `/card-test`
**Headline:** This is NOT a green-field effect. **Every building block already exists.** Both cards already
parse ~95%; the *only* gap is one clause that lowers to `Effect::Unimplemented { name: "search" }`.
The work is **parser routing + a small resolver-scoping extension + two pre-existing lowering-bug fixes** —
**zero new engine enum variants.**

---

## 1. Exact Oracle text (fetched from Scryfall) + shared/per-card decomposition

**Deadly Cover-Up** — `{3}{B}{B}` Sorcery (MKM):
> As an additional cost to cast this spell, you may collect evidence 6.
> Destroy all creatures. If evidence was collected, exile a card from an opponent's graveyard. Then search its owner's graveyard, hand, and library for any number of cards with that name and exile them. That player shuffles, then draws a card for each card exiled from their hand this way.

**The End** — `{2}{B}{B}` Instant (OTJ):
> This spell costs {2} less to cast if your life total is 5 or less.
> Exile target creature or planeswalker. Search its controller's graveyard, hand, and library for any number of cards with the same name as that permanent and exile them. That player shuffles, then draws a card for each card exiled from their hand this way.

> **Measured correction to the brief:** The End targets **"creature or planeswalker"** — there is **NO legendary
> restriction** (the brief guessed "legendary targeting"). Do not add one. Measured text wins.

**Shared core primitive (identical in both, byte-for-byte in the tail):**
`search <player>'s graveyard, hand, and library for any number of cards with <same-name-ref> and exile them.
That player shuffles, then draws a card for each card exiled from their hand this way.`

| Axis | Deadly Cover-Up | The End |
|---|---|---|
| Same-name reference source | `that name` — a card **exiled during resolution** from an opponent's graveyard (**non-targeted choice**) | `the same name as that permanent` — the **targeted** exiled creature/planeswalker |
| Searched player | **its owner's** → `ControllerRef::ParentTargetOwner` (CR 108.3 — GY card has no controller, CR 109.4) | **its controller's** → `ControllerRef::ParentTargetController` |
| Per-card head | additional cost `collect evidence 6` (may); `Destroy all creatures`; conditional `if evidence was collected` seed-exile | cost reduction `{2} less if life ≤ 5`; `exile target creature or planeswalker` |
| Draw rider | present (shared) | present (shared) |

**Per-card heads already parse correctly** (probe-verified, §7). The shared search clause is the sole gap.

---

## 2. Existing building blocks traced (file:line) + the precise gap

### 2a. Ground-truth parse (probe of the real Oracle text — the decisive measurement)
A scratch harness calling `engine::parser::parse_oracle_text` on both exact strings shows:

**The End** →
`ChangeZone{dest:Exile, target:Or[Creature,Planeswalker]}`  ✅ (exile-target; seed enters `ability.targets`)
→ sub: **`Unimplemented{ name:"search", desc:"Search its controller's graveyard, hand, and library for any number of cards with the same name as that permanent and exile them" }`  ⛔ THE GAP**
→ sub: `Shuffle{ target: ParentTargetController }`  ✅
→ sub: `Draw{ count: Ref(ExiledFromHandThisResolution), target: Controller }`  ⚠️ (target bug, §4b)
+ static `ModifyCost{Reduce {2}}` gated `LifeTotal(Controller) LE 5`  ✅

**Deadly Cover-Up** →
`DestroyAll{Creature}`  ✅
→ sub (cond `AdditionalCostPaid`): `ChangeZone{dest:Exile, target:Card[Owned(Opponent),InZone(Graveyard)], forward_result:false}`  ✅ effect / ⚠️ `forward_result` (§4c)
→ sub: **`Unimplemented{ name:"search", desc:"search its owner's graveyard, hand, and library for any number of cards with that name and exile them" }`  ⛔ THE GAP**
→ sub: `Shuffle{ target: ParentTargetController }`  ⚠️ (owner-axis bug, §4b)
→ sub: `Draw{ count: Ref(ExiledFromHandThisResolution), target: Controller }`  ⚠️ (§4b)
+ `additionalCost: Optional(CollectEvidence{amount:6})`  ✅

**Conclusion: the ONLY missing effect is the search-and-exile clause. Everything else is wired.**

### 2b. Building blocks that already exist (reuse these — do NOT reinvent)
| Capability | Location | Notes |
|---|---|---|
| Multi-zone search effect | `Effect::SearchLibrary{ source_zones:Vec<Zone>, filter, count, target_player, .. }` — `types/ability.rs:9545` | `source_zones` already supports `[Graveyard,Hand,Library]` (God-Pharaoh's-Gift class, doc:9548) |
| "any number of" count | `QuantityExpr::UpTo{max}` — resolver peels it at `search_library.rs:342` → interactive `SearchChoice` with `up_to`/fail-to-find | CR 701.23b |
| Same-name-as-referent filter | `FilterProp::SameNameAsParentTarget` — `filter.rs:3462` → `parent_target_name()` `filter.rs:3034` (live obj → `lki_cache` fallback) | Snapshot-safe after seed leaves its zone |
| Multi-zone origin filter | `FilterProp::InAnyZone{ zones }` | |
| Searched-player scope | `ControllerRef::ParentTargetController` / `ParentTargetOwner`; resolved via `controller_ref_player` `filter.rs:730`, `parent_target_controller/owner` `filter.rs:662/675` | |
| Search resolver (multi-zone, target_player, candidate build, fail-to-find) | `game/effects/search_library.rs:325-500` — builds candidates from `library_owner`'s `source_zones`, filters, raises `WaitingFor::SearchChoice` | Fully handles multi-zone + `SameNameAsParentTarget` |
| Found→destination(exile)+shuffle | `SearchChoice` completion `engine_resolution_choices.rs:1845-1969` → `cont.chain` = `SearchDestination`→`ChangeZone{Exile}` + `Shuffle`; `propagate_targets_through_search_shuffle` `:4411` | |
| SearchDestination attach (intrinsic) | `parse_intrinsic_continuation_ast` `sequence.rs:4071` — attaches `SearchDestination{ parse_search_destination(full_lower) }` for `Effect::SearchLibrary`; suppressed only for the "all cards" ChangeZoneAll recognizer (`:4087`) | Will fire for our SearchLibrary (recognizer returns None on "any number") |
| Hand-exile counter | `QuantityRef::ExiledFromHandThisResolution` `types/ability.rs:4620`; field `state.exiled_from_hand_this_resolution` `game_state.rs:7181`; reset `engine.rs:220`,`effects/mod.rs:5174` | **Populated ONLY in the `ChangeZoneAll` mass-move loop** `change_zone.rs:1419-1425` — the single-`ChangeZone` SearchDestination path does NOT feed it (§4a) |
| Draw N (dynamic) | `Effect::Draw{ count, target }` | Already lowered with `Ref(ExiledFromHandThisResolution)` — probe-confirmed |
| Non-targeted seed → parent target | `forward_result:true` on a `ChangeZone` → "moved object becomes sub's source" & "binds ParentTarget to the moved object" `effects/mod.rs:7196,7261`; tests `:11175,:11266` | Deadly Cover-Up's seed threading (§4c) |
| collect evidence / destroy all / cost reduction / exile-target | all already parse (probe §2a) | `CollectEvidence` CR 701.59 |

### 2c. The precise gap (root cause)
The mid-chain clause dispatch tries **only** the "all cards" recognizer and then falls through to `Unimplemented`:
- `parser/oracle_effect/mod.rs:6148` (standalone-clause imperative dispatch) and
- `parser/oracle_effect/imperative.rs:2348` (`parse_search_creation_imperative_ast`)
both call `try_parse_multi_zone_same_name_exile` (`imperative.rs:2170`), which hard-requires `tag("all cards")`
(`:2201`) and **declines on "any number of cards"**. No other branch claims the clause → `Effect::unimplemented("search", …)`.

**The interactive `SearchLibrary` path is NEVER reached for this mid-chain clause** because
`parse_search_library_details` runs from the top-level line classifier, not the mid-chain imperative dispatch;
and even if reached, `parse_search_target_player` (`search.rs:613`) only recognizes
`target opponent's / target player's / an opponent's` — **not** the possessive-pronoun owner `its owner's` /
`its controller's`.

### 2d. Test-enforced design boundary (CRITICAL — do not fight it)
`name_hate_any_number_spells_do_not_auto_exile` (`tests.rs:31570`) **asserts Crumble to Dust & Surgical
Extraction ("any number of cards") must NOT lower to `ChangeZoneAll`.** CR 701.23b (name = stated quality →
searcher may fail to find) makes the interactive model rules-correct and `ChangeZoneAll` (mandatory exile-all)
**wrong** for "any number". → **We MUST route to interactive `SearchLibrary`, never `ChangeZoneAll`.**
(The `ChangeZoneAll` path stays reserved for the "all cards" class: Eradicate/Quash/Counterbore — `tests.rs:31523`.)

---

## 3. Proposed lowered AST/chain (target)

Both cards keep every already-correct node; we replace the `Unimplemented{"search"}` node with:

```
Effect::SearchLibrary {
    source_zones: vec![Zone::Graveyard, Zone::Hand, Zone::Library],
    filter: TargetFilter::Typed(TypedFilter::default()
                .properties(vec![FilterProp::SameNameAsParentTarget])),
    count: QuantityExpr::UpTo { max: Box::new(QuantityExpr::Fixed { value: i32::MAX }) }, // "any number"
    reveal: false,
    target_player: Some(TargetFilter::Typed(TypedFilter::default()
                .controller(<AXIS>))),   // The End: ParentTargetController · Deadly: ParentTargetOwner
    selection_constraint: SearchSelectionConstraint::None,
    split: None,
}
// then, via the existing intrinsic SearchDestination continuation (sequence.rs:4071):
//   → ChangeZone { origin:Library-per-object, destination: Exile }   (found set → exile)
//   → Shuffle { target: <AXIS player> }
//   → Draw   { count: Ref(ExiledFromHandThisResolution), target: <AXIS player> }
```

**Why `target_player` is a `Typed(controller=<axis>)` and NOT the bare `ParentTargetController` variant:**
the bare `TargetFilter::ParentTargetController` is **load-bearing for Assassin's Trophy self-search**
(`tests.rs:9861,9948,10029`), where `searcher_is_library_owner(ParentTargetController)==true`
(`search_library.rs:54-68`) makes the *controller* the searcher. For our cards the **caster** searches the
opponent's zones (CR 701.23a asymmetric). Encoding the axis on a `Typed` filter's `.controller` field:
- `searcher_is_library_owner(Typed{..})` → **false** ⇒ searcher = caster ✅ (caster makes the fail-to-find choice)
- `resolve_library_owner` resolves the Typed filter's controller-ref → the parent target's owner/controller ✅
- Assassin's Trophy (bare variant) untouched ✅

The End: seed = the targeted exiled permanent (already in `ability.targets`) → `SameNameAsParentTarget` /
`ParentTargetController` resolve directly (via `lki_cache` after exile).
Deadly Cover-Up: seed = the non-targeted exiled GY card → threaded into the parent slot by `forward_result:true`
on the preceding seed-exile `ChangeZone` (§4c).

---

## 4. `/add-engine-variant` gate verdict + the concrete work items

### `/add-engine-variant` — Stage-1 existence verification for every element:
| Proposed element | Verdict | Evidence |
|---|---|---|
| Multi-zone search effect | **EXISTS_SAME_NAME** | `Effect::SearchLibrary` `types/ability.rs:9545` (`source_zones`) |
| "any number" count | **EXISTS_SAME_NAME** | `QuantityExpr::UpTo` |
| same-name filter | **EXISTS_SAME_NAME** | `FilterProp::SameNameAsParentTarget` (inventory ✓) |
| multi-zone origin | **EXISTS_SAME_NAME** | `FilterProp::InAnyZone` (inventory ✓) |
| searched-player axes | **EXISTS_SAME_NAME** | `ControllerRef::ParentTargetOwner`/`ParentTargetController` (inventory ✓) |
| hand-exile count | **EXISTS_SAME_NAME** | `QuantityRef::ExiledFromHandThisResolution` (inventory ✓) |
| seed threading | **EXISTS_SAME_NAME** | `AbilityDefinition.forward_result` |

**Gate verdict: ZERO new variants — composition + routing only.** (Confirmed against `data/engine-inventory.json`,
read-only; not regenerated.) No Stage-2/Stage-3 needed.

### Work items (all reuse; ordered in §5)
- **W1 (parser, new recognizer):** interactive sibling to `try_parse_multi_zone_same_name_exile`, matching the
  same grammar with the **"any number of cards"** (also "up to N cards") quantifier, returning the owner axis.
- **W2 (parser, lowering):** new `SearchCreationImperativeAst` arm → the `Effect::SearchLibrary` of §3.
- **W3 (parser, dispatch):** call W1 right after the "all cards" recognizer at `mod.rs:6148` and `imperative.rs:2348`.
- **W4 (resolver, `search_library.rs` — NOT frozen):** extend `resolve_library_owner` (`:25`) to resolve a
  player-scoped `Typed` `target_player` whose `.controller` is `ParentTargetController`/`ParentTargetOwner`
  → concrete player via `controller_ref_player` (delegating to `parent_target_controller/owner`). Leave
  `searcher_is_library_owner` unchanged (Typed ⇒ caster searches).
- **W5 (counter, §4a):** populate `exiled_from_hand_this_resolution` in the SearchDestination→exile path.
- **W6 (parser lowering fix, §4b):** `Draw.target` for "that player … then draws" must inherit the searched
  player (rebind to the same axis as the shuffle), not `Controller`; and the owner-axis shuffle/draw must use
  `ParentTargetOwner` for Deadly Cover-Up.
- **W7 (parser lowering, §4c):** set `forward_result:true` on Deadly Cover-Up's seed-exile `ChangeZone` so the
  non-targeted chosen card seeds the parent slot.

### 4a. Hand-exile counter integration (W5 — the make-or-break)
`state.exiled_from_hand_this_resolution` is incremented **only** inside the `ChangeZoneAll` mass-move loop
(`change_zone.rs:1419-1425`, guard `per_object_origin==Hand && dest==Exile`). The interactive path exiles the
found set via a **single `ChangeZone`** (SearchDestination lowering — see `sequence.rs:6774,6828`), which does
**not** hit that write. Without W5 the shared draw rider counts 0 on **both** cards.
**Fix:** mirror the guard in the found-set exile path. Cleanest site: the `SearchChoice` completion in
`engine_resolution_choices.rs:1874-1968` (the `chosen` ids and their pre-move zones are known there before the
continuation drains), or the single-`ChangeZone` move helper. Increment once per chosen card whose pre-move
zone == `Hand` and destination == `Exile`. **Verify** the counter survives the `WaitingFor::SearchChoice`
pause (it is a `GameState` field, preserved by save/restore per `game_state.rs:8617`) and is read by the later
`Draw` in the same resolution (reset only at action/chain top — `engine.rs:220`, `effects/mod.rs:5174`).

### 4b. Draw/Shuffle "that player" subject (W6 — pre-existing lowering bug, probe-proven)
`"that player shuffles"` lowers to `Shuffle{target:Player}` (`imperative.rs:6028`) then is rebound to
`ParentTargetController` post-lowering (probe §2a). The `", then draws a card for each card exiled from their
hand this way"` clause lowers to `Draw{ count:Ref(ExiledFromHandThisResolution), target: Controller }` — the
`target` did **not** inherit the rebind and defaulted to `Controller` (the caster). Rules: "That player …
draws" ⇒ draw goes to the **searched player**. Fix so the draw inherits the shuffle's parent-target axis.
For Deadly Cover-Up the axis is **owner** (`ParentTargetOwner`); ensure both shuffle and draw resolve to the
same axis the search used (compare `name_hate_owner_axis_shuffle_inherits_parent_target_owner` `tests.rs:31593`
for the ChangeZoneAll analogue — replicate that owner-axis carry on the SearchLibrary path).

### 4c. Deadly Cover-Up seed threading (W7)
The seed is exiled by a **non-targeted** `ChangeZone{ target:Card[Owned(Opponent),InZone(Graveyard)] }` — it is
NOT in `ability.targets`, so `SameNameAsParentTarget` / `ParentTargetOwner` would resolve against nothing.
Set `forward_result:true` on that `ChangeZone` (probe showed `false`) so the chosen exiled card binds the
parent slot for the downstream search/shuffle/draw. The End needs no change here (it targets its permanent).

---

## 5. Files to touch (dependency order — all NON-frozen)

1. `crates/engine/src/parser/oracle_effect/imperative.rs` — **W1** new `try_parse_multi_zone_same_name_search`
   (compose with the existing owner-axis `alt()` at `:2174-2188`); **W2** new `SearchCreationImperativeAst`
   arm + lowering (mirror `:2834`); **W3** dispatch at `:2348`; **W7** `forward_result` on the seed-exile
   `ChangeZone` lowering.
2. `crates/engine/src/parser/oracle_effect/mod.rs` — **W3** dispatch at `:6148`.
3. `crates/engine/src/parser/oracle_effect/sequence.rs` — verify intrinsic `SearchDestination` attaches Exile
   for the new SearchLibrary (suppression at `:4087` must NOT fire — it keys on the "all cards" recognizer,
   which returns None here); **W6** draw-subject carry if it lives in the shuffle/draw continuation lowering.
4. `crates/engine/src/game/effects/search_library.rs` — **W4** `resolve_library_owner` Typed-controller
   resolution. (NOT frozen.)
5. `crates/engine/src/game/effects/change_zone.rs` **or** `crates/engine/src/game/engine_resolution_choices.rs`
   — **W5** hand-exile counter in the found-set exile path. (change_zone.rs NOT frozen.)
6. Tests colocated in the above modules (§7).

**Frozen-file check (do NOT edit):** `game/effects/mod.rs`, `game/filter.rs`, `game/effects/delayed_trigger.rs`.
- No design edit is required in any of them. `filter.rs::SameNameAsParentTarget` / `controller_ref_player` are
  **read/reused as-is**. `effects/mod.rs::forward_result` machinery is **reused as-is** (W7 only sets a flag in
  the parser lowering; no `mod.rs` edit).
- **Frozen-pressure flag (low):** W5 ideally lives in `change_zone.rs` (not frozen). If the reviewer prefers the
  counter write in the shared `execute_zone_move` and that turns out to sit in a frozen module, fall back to the
  `SearchChoice`-completion site in `engine_resolution_choices.rs` (not frozen). **Reuse path preferred; no
  frozen edit anticipated.**

**nom mandate:** W1/W2 use `tag()`/`alt()`/`value()` composed with the existing owner-axis and zone combinators
— no `find`/`split_once`/`contains`/`starts_with` for dispatch. Add the "any number of cards" quantifier as one
`alt()` axis; do not enumerate permutations.

---

## 6. CR annotations (all grep-verified against `docs/MagicCompRules.txt`)
| CR | Text (verified) | Applies to |
|---|---|---|
| **701.23** | "Search" | W1/W2/W4 search primitive |
| **701.23a** | search a zone; asymmetric-library searches (target player's library searched by caster) | W4 searcher=caster / owner=parent-target |
| **701.23b** | searching a hidden zone for a **stated quality** — player **isn't required to find** | interactive `UpTo`; forbids ChangeZoneAll (§2d) |
| **701.24** | "Shuffle" (701.24a randomize) | shuffle step. ⚠️ **Codebase mis-annotates shuffle as `CR 701.18a` in places — 701.18 is "Play". Use CR 701.24 in all new annotations.** |
| **201.2 / 201.2a** | two objects "have the same name" iff ≥1 name in common | `SameNameAsParentTarget` |
| **400.7** | zone change → new object, no memory | seed name via `lki_cache`; per-object origin |
| **108.3** | owner = player who started with the card | Deadly "its owner's" → `ParentTargetOwner` |
| **109.4** | only stack/battlefield objects have a controller | why Deadly uses owner (GY card), The End uses controller (battlefield) |
| **701.59 / 701.59a-c** | Collect Evidence N; 701.59c linked "evidence was collected" | Deadly additional cost + `AdditionalCostPaid` (already parsed) |

---

## 7. Test plan (`/card-test` — discriminating, revert-to-red)

**Foot-gun compliance:** use `GameScenario` + `GameRunner::cast(...).resolve()` and assert on `CastOutcome`
deltas / observed zones — never hand-write `TargetRef` vectors, never assert AST-internal flags, submit the full
`SearchChoice` selection, take the hand baseline at the right point, and make every negative assertion
non-vacuous (a same-named card in a decoy zone that MUST survive).

### 7a. Parser tests (fast loop) — colocated in `imperative.rs`/`tests.rs`
1. **The End** — `parse_effect_chain(full_text)` must contain **`Effect::SearchLibrary`** with
   `source_zones==[Graveyard,Hand,Library]`, `count` matches `QuantityExpr::UpTo{..}`, `filter` carries
   `FilterProp::SameNameAsParentTarget`, `target_player` is `Some(Typed(controller==ParentTargetController))`;
   chain contains `ChangeZone{dest:Exile}`, `Shuffle{ParentTargetController}`,
   `Draw{Ref(ExiledFromHandThisResolution), target==ParentTargetController}`; **root effect is NOT `Unimplemented`.**
2. **Deadly Cover-Up** — same, with `target_player`/shuffle/draw axis == **`ParentTargetOwner`**, seed-exile
   `ChangeZone` has `forward_result==true`, additional cost `CollectEvidence{6}` present.
3. **Discrimination / no-collateral-damage (revert-to-red):**
   - Crumble to Dust & Surgical Extraction still route interactive (`SearchLibrary`, **not** `ChangeZoneAll`)
     — keep `name_hate_any_number_spells_do_not_auto_exile` green.
   - Eradicate/Quash/Counterbore ("all cards") still lower to `ChangeZoneAll`
     — keep `name_hate_spells_parse_multi_zone_same_name_exile_chain` green.
   - Assassin's Trophy still lowers to `SearchLibrary{ target_player: ParentTargetController }` self-search
     (bare variant, searcher=owner) — unchanged by W4.
   - **Non-vacuity evidence:** revert W1 → assertion 1/2 flips to `Unimplemented{"search"}` (the current state,
     probe-proven). Revert W4 → runtime test 7b fails (searches caster's zones). Revert W5 → draw count = 0.

### 7b. Runtime cast tests (`GameScenario`/`GameRunner`) — the anti-hollow-win observables
**The End (controller axis):**
- Setup: caster P0; P1 controls a creature `Grizzly Bears` on battlefield; P1 has same-named `Grizzly Bears` in
  **each** of GY, hand, library; P1 also has a **decoy `Llanowar Elves`** in GY/hand/lib; **P0** has its own
  `Grizzly Bears` in hand (the multiplayer-scoping decoy).
- Cast The End targeting P1's battlefield Bears; drive the `SearchChoice` selecting **all** offered cards.
- **Assert (positive):** the battlefield Bears is exiled; **all three** P1 `Grizzly Bears` (GY, hand, lib) are
  exiled; P1's library was shuffled.
- **Assert (draw rider):** P1 (the searched player) drew **exactly 1** (one Bears exiled from P1's **hand**),
  and drew to **P1's** hand, not P0's.
- **Assert (discriminating negatives — must all hold or the test is hollow):** P1's `Llanowar Elves` in every
  zone survive; **P0's own `Grizzly Bears` in hand survives** (proves owner/searcher scoping — the search hit
  P1's zones, not the caster's).

**Deadly Cover-Up (owner axis + non-targeted seed + evidence gating):**
- Setup mirrors §change_zone.rs:6084 but **through the real cast pipeline** (not a hand-built chain): P0 caster;
  destroy-all fodder on battlefield; P1 owns a `Grizzly Bears` in GY (seed) + same-named copies across GY/hand/lib
  + a decoy; P0 owns a `Grizzly Bears` in hand (scoping decoy); both players have libraries.
- **Case A (no evidence):** cast without paying collect-evidence → only `Destroy all creatures` happens; seed
  stays in GY; no Bears exiled from hand/lib; `exiled_from_hand_this_resolution == 0`; P1 draws 0.
- **Case B (evidence paid):** collect evidence 6, choose the P1-GY seed, drive the `SearchChoice` selecting all →
  seed + all P1 Bears exiled across all three zones; P1 shuffles; P1 drew exactly `#(Bears exiled from P1's hand)`;
  P0's `Grizzly Bears` in hand survives; the decoy survives.
- **Non-vacuity:** Case A vs Case B is the discriminating pair; reverting W7 makes Case B find nothing (seed not
  in the parent slot); reverting W5 makes Case B's draw = 0.

---

## 8. Risks / open questions for `/review-engine-plan`

1. **[HIGH] Hand-exile counter on the interactive path (W5).** The draw rider on *both* cards depends on
   `exiled_from_hand_this_resolution`, which today is written only by `ChangeZoneAll`. Confirm the chosen site
   (SearchChoice completion vs single-`ChangeZone` move) writes it, that it survives the `SearchChoice` pause,
   and that no *other* interactive search (e.g. a hidden-zone tutor into hand) accidentally starts counting.
   Prefer the guard `pre-move zone==Hand && dest==Exile` at the found-set move so it stays exile-specific.

2. **[HIGH] Searcher/owner asymmetry (W4).** The plan encodes the axis on `Typed(controller=…)` specifically to
   keep the caster as searcher while reusing the parent-target-owner resolution, WITHOUT disturbing Assassin's
   Trophy's bare-`ParentTargetController` self-search. Reviewer to confirm: (a) `resolve_library_owner`'s new
   Typed arm resolves via `controller_ref_player` correctly for both axes; (b) `searcher_is_library_owner`
   genuinely returns false for `Typed{..}`; (c) `matches_target_filter_in_owner_zone` evaluates
   `SameNameAsParentTarget` against the **library-owner's** candidate cards (ownership-substituted, CR 109.5).

3. **[MED] Name-snapshot ownership.** `parent_target_name` (`filter.rs:3034`) reads live object then
   `lki_cache`. The End's targeted permanent is exiled *before* the search resolves — confirm the exile writes
   the LKI name (it should, via `change_zone`), and that the parent slot still points to the (now-exiled) id.
   Deadly's seed relies on `forward_result` binding (W7) — confirm `forward_result` populates the sub-chain
   `targets` (so `first_object_target` finds it), not merely `source_id`.

4. **[MED] Coverage-regression on the shared parse path.** W1/W3 add a recognizer ahead of the mid-chain
   fall-through. Risk: it over-claims other "search … and exile them" clauses. Mitigation: gate strictly on the
   full grammar (possessive + `graveyard, hand, and library` permutation + `any number of cards` + same-name
   suffix + `exile them`), and run the card-data coverage-regression check (CI) — a broadened `alt()` here can
   silently swallow clauses on unrelated cards (see MEMORY: "parser-coverage-regression-ci-only").

5. **[MED] "any number" vs "all cards" boundary must stay intact.** Do not let W1 match the "all cards" form
   (that stays ChangeZoneAll) nor let it break `name_hate_any_number_spells_do_not_auto_exile`. The two
   recognizers are siblings keyed on the quantifier `alt()` only.

6. **[LOW] Multiplayer routing.** `SearchLibrary` interactive prompts already route to `searcher_id` (the
   caster). Confirm visibility (`visibility.rs:93,408`) shows the searcher the candidate cards and hides them
   from others, unchanged by this work.

7. **[LOW] `/add-engine-effect` lifecycle** is mostly pre-satisfied because we reuse `Effect::SearchLibrary`
   (targeting, multiplayer filter, frontend `SearchChoice` UI, AI `search_pick` enumerator all already wired).
   The only lifecycle touch-points are parser (W1-3,6-7), resolver scoping (W4), and the counter (W5) — verify
   the AI `SearchChoice` enumerator picks a sensible "all" for these (it should, given `up_to`).

---
_Probe crate used for §2a is `scratch-p2a-probe/` (removed after planning); target dir was in scratchpad. No
repo code was edited during planning._
