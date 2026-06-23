I have everything. The `obj_mut` borrow at 672 stays alive through 677, so after the `reset_for_battlefield_entry` call I can read `obj_mut.incarnation` into `zone_change_record.incarnation` within the same block. Let me write the final plan.

---

# Implementation Plan: ETB intervening-if recheck must discriminate the original entrant by incarnation, not storage ObjectId

## Problem statement (verified against current code)

`matches_zone_change_event_object_filter` (`/private/tmp/wt-msh-intervening-if/crates/engine/src/game/filter.rs:1276-1331`) rechecks an ETB intervening-if at resolution (CR 603.4). The prior fix (95c8cd8f7) routes the recheck to exit-LKI once the entrant has left the battlefield. But its "still on battlefield" test at `filter.rs:1311-1314` keys on `ObjectId` **only**:

```rust
let still_on_battlefield = state
    .objects
    .get(object_id)
    .is_some_and(|obj| obj.zone == Zone::Battlefield);
```

`ObjectId` is **storage identity** that persists across zone changes (`zones.rs:120-126`). When the original entrant leaves **and a new object re-enters reusing that storage `ObjectId`** before the trigger resolves, this test passes against the **new** object and the recheck reads the **new** object's characteristics — violating CR 608.2h (the effect must use the original object's current-or-last-known information, never an unrelated object that happens to share storage). The monotonic `GameObject.incarnation` (`game_object.rs:524-532`, bumped on every battlefield entry by `reset_for_battlefield_entry` at `game_object.rs:1350-1354`) is the discriminator that exists precisely "to distinguish the new object from the old one at the same id," but it is absent from `ZoneChangeRecord`, `LKISnapshot`, and the `ZoneChanged` event, so the filter cannot use it.

## CR basis (grep-verified in `/private/tmp/wt-msh-intervening-if/docs/MagicCompRules.txt`)

- **CR 400.7** (line 1948): "An object that moves from one zone to another becomes a new object with no memory of, or relation to, its previous existence." — the re-entrant is a *different* object even at the same storage id.
- **CR 608.2h** (line 2802): an effect reading a specific object uses its current info if it is in the public zone it was expected in; otherwise its last known information. — never the characteristics of an unrelated object.
- **CR 603.4** (line 2588): the intervening-if is rechecked as the ability resolves. — this is the recheck site.
- **CR 603.10a**: an ETB trigger is not a look-back trigger, so the normal CR 608.2h rule applies (already annotated in the function).

## Analogous Trace

Traced the **prior fix's own mechanism** (the exit-LKI ETB recheck, 95c8cd8f7) end-to-end, plus the **per-turn attack-declaration LKI snapshot** as the canonical "snapshot a struct field at event time, read it back through a synthesized object" pattern:

- Condition dispatch: `TriggerCondition::ZoneChangeObjectMatchesFilter` arm at `game/triggers.rs:5448-5461` → calls `super::filter::matches_zone_change_event_object_filter(state, event, ...)`. The `event` is the immutable `GameEvent::ZoneChanged { record, .. }` carried from the original ETB.
- Filter dispatch: `game/filter.rs:1276-1331` (`matches_zone_change_event_object_filter`) → Battlefield branch → live `matches_target_filter` / `matches_target_filter_on_lki_snapshot` (`filter.rs:1232-1268`) / `matches_target_filter_on_zone_change_record` (`filter.rs:1079-1093`).
- Snapshot construction: `GameObject::snapshot_for_zone_change` (`game/game_object.rs:1071-1114`) builds the record at `zones.rs:620` (pre-move); exit-LKI literal built at `zones.rs:152-173`; `LKISnapshot → ZoneChangeRecord` reconstruction at `filter.rs:1239-1267`; `ZoneChangeRecord → LKISnapshot` reverse converter at `effects/mod.rs:1235-1256`.
- Snapshot-field-on-record precedent: `AttackDeclarationRecord` (`game_state.rs:482-492`) carries an `LKISnapshot` snapshotted at declaration and read back via a synthesized `GameObject` in `matches_target_filter_on_attack_declaration_record` (`filter.rs:1137-1177`). Adding `incarnation` to `ZoneChangeRecord`/`LKISnapshot` mirrors how `base_power`/`base_toughness`/`is_token` were threaded onto these records.

## add-engine-variant gate result: **N/A (no enum variant added)**

Ran the `/add-engine-variant` checklist against this change. The change adds a `u64` **struct field** (`incarnation`) to two existing structs (`ZoneChangeRecord`, `LKISnapshot`). It adds **no** variant to any of the gated enums (`QuantityRef`, `QuantityExpr`, `FilterProp`, `TargetFilter`, `ReplacementCondition`, `AbilityCondition`, `TriggerCondition`, `StaticCondition`, `ChoiceType`, `DelayedTriggerCondition`, `Effect`, `Keyword`, `ContinuousModification`, etc.). It reuses the already-existing `GameObject.incarnation` value (`game_object.rs:532`) — no new concept, no new parameterization axis, no sibling cluster. The parameterization filter, categorical-boundary check, and existence-verification steps therefore return "not applicable; this is a snapshot-field threading change, identical in shape to the existing `base_power`/`base_toughness`/`is_token`/`combat_status` fields on `ZoneChangeRecord`." No `cargo engine-inventory` variant proposal is required.

## Definitive resolution of the lki_cache-clobber dispatch: **Option (i) — tag both `ZoneChangeRecord` and `LKISnapshot` with `incarnation`**

### Why option (i), with evidence

The dispatch must distinguish three cases at `filter.rs:1298-1330`:

- **(a)** original entrant is the live battlefield object → live current info (CR 608.2h in-zone).
- **(b)** original entrant has left and `lki_cache` still holds **its** exit snapshot → exit-LKI (the prior fix's pumped-then-died semantics: exit-time P/T per CR 608.2h "most recently existed").
- **(c)** original entrant has left and `lki_cache` is **absent** or holds a **different** incarnation's snapshot (clobbered by a re-entry/re-exit at the same storage id) → fall to the **event's own `record`** (immutable, carried inside `GameEvent::ZoneChanged`, the entry-time snapshot of the original entrant).

`lki_cache` is keyed by `ObjectId` only and carries no incarnation tag, so **(b) cannot be distinguished from (c) by reading `lki_cache` alone.** The live object's incarnation tells us the original is gone but cannot validate which incarnation produced the cached snapshot. The only sound discriminator is to tag the cached snapshot with the incarnation it was taken from and compare it to the record's incarnation.

**Option (ii) ("always use the event record for the left case") is rejected** — proven by tracing the prior fix's test. `zone_change_object_condition_entering_uses_exit_lki_after_leaving_battlefield` (`triggers.rs:9199-9349`) builds its ETB event via the `zone_changed_event` helper (`triggers.rs:6467-6485`), whose record is `ZoneChangeRecord::test_minimal(...)` → `power: None, toughness: None`. The test's positive assertion requires reading **3/3** (the exit-LKI value at `lki_cache`), not the record's `None`. Routing the left-path to the record would read `None` for P/T and the positive assertion (`exit LKI 3/3 > source 2/2`) would fail. Option (i) preserves this test because case (b) still fires: the original entrant was created via `create_object` (which does **not** call `reset_for_battlefield_entry`, so its `incarnation == 0`), the exit-LKI snapshot is stamped `incarnation == 0`, the record from `test_minimal` is `incarnation == 0`, so `lki.incarnation == record.incarnation` → exit-LKI path → reads 3/3 → green.

### Resulting dispatch (replaces `filter.rs:1311-1327`)

```rust
// CR 400.7 + CR 608.2h: A zone change makes a NEW object even when the engine
// reuses the storage ObjectId. The recheck must read the ORIGINAL entrant's
// info, identified by (id, incarnation) — never an unrelated re-entrant that
// happens to occupy the same storage id. `incarnation` is the discriminator
// (game_object.rs reset_for_battlefield_entry).
let live_is_original_entrant = state.objects.get(object_id).is_some_and(|obj| {
    obj.zone == Zone::Battlefield && obj.incarnation == record.incarnation
});
if live_is_original_entrant {
    // (a) Original entrant still on the battlefield → current info.
    matches_target_filter(state, *object_id, filter, ctx)
} else if let Some(lki) = state
    .lki_cache
    .get(object_id)
    .filter(|lki| lki.incarnation == record.incarnation)
{
    // (b) Original entrant left and its OWN exit-LKI is still cached (not
    // clobbered by a re-entry) → exit-time last-known information
    // (CR 608.2h "most recently existed"; preserves pumped-then-died P/T).
    matches_target_filter_on_lki_snapshot(state, *object_id, lki, filter, ctx)
} else {
    // (c) Original entrant absent, replaced by a different incarnation, or its
    // exit-LKI was clobbered by a re-entry/re-exit at the same storage id.
    // The event's own immutable `record` is the entry-time snapshot of the
    // original entrant and the only surviving authority.
    matches_target_filter_on_zone_change_record(state, record, filter, ctx)
}
```

(The existing CR comment block at `filter.rs:1298-1310` is retained and extended with the CR 400.7 incarnation note.)

## Identity / Provenance Contract

- **Source concept:** "the object that entered the battlefield" (CR 603.4 recheck subject), bound at ETB.
- **Authority type/id:** `(ObjectId, incarnation: u64)` — storage id plus the monotonic per-incarnation counter.
- **Binding time:** the `record.incarnation` is latched at the moment the entrant finishes entering the battlefield (`zones.rs`, immediately after `reset_for_battlefield_entry` bumps the live incarnation). The `lki.incarnation` is latched at the entrant's battlefield **exit** (`zones.rs:152-173`).
- **Live vs snapshotted:** the record's incarnation is a snapshot frozen inside the immutable `GameEvent::ZoneChanged` (immune to later state mutation). The live-object incarnation is read live to detect re-entry. The exit-LKI incarnation is a snapshot.
- **Storage location:** `ZoneChangeRecord.incarnation`, `LKISnapshot.incarnation` (both `#[serde(default)]`).
- **Consuming function:** `matches_zone_change_event_object_filter` (`filter.rs:1276`).
- **Invalidation/expiration:** a re-entry bumps the live incarnation (record mismatch → not case (a)); a re-exit overwrites `lki_cache` with the new incarnation (lki mismatch → not case (b)); both correctly route to the immutable record (case (c)).
- **Multi-authority hostile fixture (proves the binding):** original entrant (incarnation N, layered 3/3 on battlefield, base 1/1) leaves → a **different** object re-enters at the same storage `ObjectId` (incarnation N+1, live 1/1). The recheck against the **original** ETB event must read the original entrant's authority (3/3), not the re-entrant's 1/1. See the discriminating test below.

## Pattern Coverage

This covers the **entire class of ETB / zone-change intervening-if and look-back filters that read a specific entrant's characteristics across a blink/flicker or re-entry at the same storage id** — not one card. Every card whose ETB trigger has an intervening-if comparing the entrant to a dynamic value is exposed to the bug whenever the entrant leaves and any object re-enters at the same `ObjectId` before resolution. Concretely the `ZoneChangeObjectMatchesFilter` condition (`triggers.rs:5448`) is used by Hulkling-class P/T-vs-source ETB checks, "if it's a [type]" ETB checks, "if it entered with counters" checks, etc. The fix is at the shared filter dispatch and the shared snapshot structs, so it benefits every present and future consumer of `matches_zone_change_event_object_filter` and every `matches_target_filter_on_lki_snapshot` caller (the exit-LKI path is also used by leaves-battlefield/dies look-back filters). Estimate: structurally unbounded (every blink/flicker interaction with an ETB or LTB intervening-if), not a single card.

## Building Blocks

Composes entirely from existing primitives — no new helper:
- `GameObject.incarnation` (`game_object.rs:532`) — reused as the discriminator; no new field on `GameObject`.
- `GameObject::snapshot_for_zone_change` (`game_object.rs:1071`) — captures `self.incarnation` into the record (live value at snapshot time).
- `GameObject::reset_for_battlefield_entry` (`game_object.rs:1350`) — already bumps incarnation; unchanged.
- `matches_target_filter` / `matches_target_filter_on_lki_snapshot` / `matches_target_filter_on_zone_change_record` (`filter.rs:1316/1232/1079`) — the three existing dispatch targets; reused unchanged.
- `ZoneChangeRecord::test_minimal` (`game_state.rs:522`) — the canonical test constructor; gains `incarnation: 0` so all `..test_minimal()` spread sites stay correct with zero per-site churn.
- `Option::filter` (std) — used in the dispatch to gate the `lki_cache` hit on incarnation equality.

## Logic Placement

- **`types/game_state.rs`** — the two `incarnation` struct fields (data model: a snapshot of object identity at event time, exactly where `base_power`/`is_token`/`combat_status` already live).
- **`game/game_object.rs`** — populate `incarnation` in `snapshot_for_zone_change` and in `snapshot_public_characteristics` (the live→snapshot capture is the object's responsibility).
- **`game/zones.rs`** — overwrite the ETB record's incarnation with the post-entry value (the move pipeline owns the entry-time invariant) and stamp the exit-LKI literal with the live incarnation.
- **`game/effects/mod.rs`** — carry `incarnation` through the `ZoneChangeRecord → LKISnapshot` reverse converter (`lki_snapshot_from_zone_change_record`).
- **`game/filter.rs`** — the dispatch (the only behavioral change; all game logic stays in the engine).

No frontend, parser, AI, or transport change — this is a pure engine snapshot/dispatch fix.

## Rust Idioms

- Typed `u64` mirroring `GameObject.incarnation`, not a bool flag.
- `#[serde(default)]` on both fields for serialized back-compat (snapshots predating this field deserialize to `incarnation == 0`).
- The dispatch keeps the existing `if / else if let / else` shape; adds `&& obj.incarnation == record.incarnation` to the live guard and `.filter(|lki| lki.incarnation == record.incarnation)` to the cache hit — composing `Option::filter` rather than nesting a second `if`.
- All `..ZoneChangeRecord::test_minimal()` spread sites are auto-covered by adding the field to the helper — no per-site churn for those.

## Serde back-compat note

Both fields are annotated `#[serde(default)]`. A `GameState` serialized before this change deserializes with `incarnation == 0` on every `ZoneChangeRecord` / `LKISnapshot`. For an in-flight ETB whose serialized record predates the field, the record reads `0`; if the live entrant is still on the battlefield it will (in the dominant case) have `incarnation` equal to whatever its real entry bumped it to. **This is a benign, conservative skew:** a mismatch routes to case (c) (the event record), which for a pre-field serialized record carries the entry-time public characteristics — still the original entrant's info, never an unrelated object's. No save-migration is required; the only observable effect on a legacy save mid-ETB is a possible exit-LKI→record demotion, which is itself rules-correct (entry-time vs exit-time both describe the original entrant). New saves carry exact incarnations.

## Step-by-step implementation

### Step 1 — `crates/engine/src/types/game_state.rs`

1a. Add to `ZoneChangeRecord` (after `object_id`, near the top of the struct at `:363`), with a CR-annotated doc comment:
```rust
/// CR 400.7 + CR 608.2h: The entrant's incarnation as it entered the
/// destination zone. Pairs with `object_id` (storage identity) so an
/// intervening-if recheck can tell the ORIGINAL entrant from a different
/// object that later re-entered reusing the same storage ObjectId
/// (blink/flicker). `#[serde(default)]` yields 0 for snapshots predating
/// this field.
#[serde(default)]
pub incarnation: u64,
```

1b. Add the same field to `LKISnapshot` (after `name` at `:163`), CR-annotated, noting it is the exit-time incarnation.

1c. Add `incarnation: 0` to `ZoneChangeRecord::test_minimal` (`:522-548`). This auto-covers every `..ZoneChangeRecord::test_minimal()` spread site (synthesis.rs:11529, events.rs:903, restrictions.rs:2760/2879/2900/2954, attach.rs:1671/1679, effects/mod.rs:9560/15159/15505, quantity.rs:6078/6083/6088/6122/6128/6134/6699, filter.rs:8553/8565/8579/8591/8633/8848/8860/8876/8937/8948/9017, derived_views — verify each is spread, zones.rs:1653, and all others).

### Step 2 — `crates/engine/src/game/game_object.rs`

2a. In `snapshot_for_zone_change` (`:1077`), add `incarnation: self.incarnation,` (captures the live value at snapshot time — the OLD incarnation for an ETB, corrected in Step 3a).

2b. In `snapshot_public_characteristics` (`:1308`), add `incarnation: self.incarnation,`.

2c. `reset_for_battlefield_entry` (`:1350`) — **no change** (already bumps `incarnation`).

### Step 3 — `crates/engine/src/game/zones.rs`

3a. ETB record correction. After `reset_for_battlefield_entry` at `:676`, inside the existing `if to == Zone::Battlefield { ... }` block where `obj_mut` is still borrowed (`:672-677`), set the record's incarnation to the **post-entry** value:
```rust
if to == Zone::Battlefield {
    obj_mut.reset_for_battlefield_entry(state.turn_number);
    // CR 400.7: the record built pre-move (line ~620) captured the OLD
    // incarnation; the entry above bumped it. The ETB event's record must
    // carry the entrant's incarnation AS IT ENTERED the battlefield, so the
    // resolution-time recheck (filter.rs) can tell this entrant from a later
    // object that re-enters at the same storage id.
    zone_change_record.incarnation = obj_mut.incarnation;
}
```
*Invariant:* the ETB event's `record.incarnation == the entrant's incarnation as it entered the battlefield.*

3b. Exit-LKI literal (`:152-173`). Add `incarnation: obj.incarnation,` to the `LKISnapshot { .. }` (the live obj still holds its on-battlefield incarnation at exit-cleanup time, which runs at `:648` before the zone field is reset — confirm `apply_zone_exit_cleanup` reads the pre-move obj).

3c. Verify all `GameEvent::ZoneChanged` emission sites carry the populated record: `zones.rs:738` (the main path — covered by 3a), `zones.rs:1653` (test, uses `test_minimal` — covered by 1c). Grep-confirm no other production `move`-path emission bypasses `snapshot_for_zone_change`.

### Step 4 — `crates/engine/src/game/effects/mod.rs`

4a. `lki_snapshot_from_zone_change_record` (`:1235-1256`) — add `incarnation: record.incarnation,` so a record-derived LKI carries the incarnation forward.

### Step 5 — `crates/engine/src/game/filter.rs`

5a. `matches_target_filter_on_lki_snapshot` (`:1239-1267`) — the synthesized `ZoneChangeRecord` must carry `incarnation: lki.incarnation,` so a downstream re-evaluation is consistent.

5b. Replace the Battlefield-branch dispatch (`:1311-1327`) with the three-case dispatch shown in the resolution section above. Extend the existing CR comment block (`:1298-1310`) with the CR 400.7 incarnation rationale.

### Step 6 — Remaining full-literal sites (compile-completeness)

Add `incarnation: <value>` to every `ZoneChangeRecord {` / `LKISnapshot {` literal that does **not** spread from `test_minimal`/another base. Confirmed non-spread sites:
- `ZoneChangeRecord`: `stack.rs:2041` (`zone_change_record_from_spec` — fresh token, set `incarnation: 1` to match a token's post-`reset_for_battlefield_entry` value; or `0` since it is a synthetic probe record keyed by `PROBE_ID` used only for trigger-key probing — set `0` and annotate that the probe record's incarnation is never compared), `derived_views.rs:1064` (test → `0`).
- `LKISnapshot`: all ~36 literal sites that are full literals — add `incarnation: 0` (test snapshots) or the appropriate live value (production: none remain beyond the method/converter/zones already handled). Enumerate via `grep -rn "LKISnapshot {" crates/engine/src/ | grep -v "pub struct"` and add the field to each that the compiler flags. Most are test literals → `incarnation: 0`. specialize.rs:172 `empty_lki()` → `0`.

(The compiler is the gate: after Steps 1-5, `cargo`/Tilt `check` will flag every missing-field literal precisely. Add `incarnation: 0` to each test literal and the documented value to each production literal.)

## Discriminating Test

Add to the `#[cfg(test)] mod tests` in `crates/engine/src/game/triggers.rs`, adjacent to `zone_change_object_condition_entering_uses_exit_lki_after_leaving_battlefield` (`:9199`), modeled on it and on `zone_change_object_condition_entering_greater_pt_than_source` (`:9081`). Reuse the same `condition` literal (Hulkling AnyOf power/toughness > source), `setup()`, `create_object`, `zone_changed_event`, and `move_to_zone`.

**Test name:** `zone_change_object_condition_entering_reads_original_entrant_not_reentrant_at_same_id`

**Changed seam:** `matches_zone_change_event_object_filter` Battlefield branch (`filter.rs:1298-1330`).
**Production entry point:** `check_trigger_condition(state, &condition, controller, Some(source), Some(&etb_event))` → `triggers.rs:5448` → `filter.rs:1276`. Re-entry driven through the production `zones::move_to_zone` path (bumps incarnation via `reset_for_battlefield_entry`).

**Body:**
1. Source = Hulkling 2/2 (creature) on battlefield, as in the model tests.
2. Original entrant: creature, `base_power/toughness = Some(1)`, layered `power/toughness = Some(3)` on battlefield (> source). Capture its incarnation `n = state.objects[&entrant].incarnation` (will be 0 via `create_object`).
3. Build the original ETB event. **The record's incarnation must equal `n`.** Use `zone_changed_event(entrant, Hand, Battlefield, vec![Creature], vec![])` — its `test_minimal` record has `incarnation: 0 == n`. **Sanity-assert** `if let GameEvent::ZoneChanged{record,..} = &etb_event { assert_eq!(record.incarnation, n); }` so the test fails loudly if `create_object`/`test_minimal` incarnation conventions drift.
4. Move the entrant OFF the battlefield via `move_to_zone(entrant, Graveyard)` (caches exit-LKI 3/3 at incarnation `n`), THEN re-enter at the SAME storage `ObjectId` via `move_to_zone(entrant, Battlefield)` (bumps incarnation to `n+1`). Reset the re-entrant's live P/T to 1/1 (`base 1/1`, `power/toughness Some(1)`).
   - **Non-vacuity / sanity asserts:** `assert_eq!(state.objects[&entrant].incarnation, n + 1)`; `assert_ne!(state.objects[&entrant].incarnation, n)`; `assert_eq!(state.objects[&entrant].power, Some(1))` (live re-entrant is 1/1).
5. **POSITIVE:** `check_trigger_condition(&state, &condition, PlayerId(0), Some(source), Some(&etb_event))` MUST be `true` — the original entrant's authority (3/3, via case (b) exit-LKI at incarnation `n`, or case (c) record if the re-exit clobbered the cache; either yields the original's 3/3 here because the exit-LKI was stamped at incarnation `n` and the re-entrant has not exited) is > source 2/2.
   - **Revert-probe (fail-before / pass-after):** under the current prior-fix code (`still_on_battlefield` by id only), the live re-entrant is on the battlefield at the shared id → reads the re-entrant's live 1/1 → `1 > 2` false → assertion **fails before** the fix. After the fix, the live guard's `obj.incarnation == record.incarnation` is `n+1 == n` → false → falls to exit-LKI/record → reads 3/3 → **passes after**. This is the discriminating assertion.
6. **NEGATIVE sub-case (proves it reads the ORIGINAL entrant, not the re-entrant — guards an over-broad fix that would always use the record/LKI regardless of which is greater):** a second scenario where the original entrant's authority is NOT greater (original layered 1/1, exit-LKI 1/1, record incarnation `m`) but the re-entrant IS 3/3 (live). `check_trigger_condition` with the ORIGINAL ETB event MUST be `false` (1/1 ≯ 2/2). A naive "if a re-entry happened, just read the live object" fix would read the re-entrant 3/3 → true → this negative catches it. (A naive "always read the record" fix reads 1/1 → false → correct here; this negative is paired with the positive's revert-probe, which a record-only fix passes but the prior-fix exit-LKI test would still constrain.)
7. **Keep-green assertions (regression guard, asserted in this test file by the existing tests, re-run together):**
   - `zone_change_object_condition_entering_uses_exit_lki_after_leaving_battlefield` (`:9199`) — stays-out exit-LKI path; case (b) preserved because `lki.incarnation == record.incarnation == 0`.
   - `zone_change_object_condition_entering_greater_pt_than_source` (`:9081`) — still-present path; case (a) preserved because the on-battlefield obj created via `create_object` has `incarnation == 0 == record.incarnation`.

**Fail-before / pass-after methodology (explicit):**
- *Fail-before:* revert only Step 5b (restore the id-only `still_on_battlefield`) while keeping the new struct fields → the POSITIVE assertion in the new test fails (reads re-entrant 1/1 → false). Confirm by temporarily commenting the `&& obj.incarnation == record.incarnation` clause.
- *Pass-after:* with the full dispatch, POSITIVE passes, NEGATIVE passes, and both keep-green tests pass.
- *Non-vacuity:* the explicit `assert_ne!(incarnation, n)` and `assert_eq!(live power, Some(1))` sanity asserts guarantee the live re-entrant genuinely differs from the original's authority, so the positive/negative discrimination is not coincidental.

## Verification Matrix

| Claim | Changed seam | Production entry | Test | Revert-failing assertion | Negative / hostile fixture | First prod branch reached | Coverage impact |
|---|---|---|---|---|---|---|---|
| ETB recheck reads the original entrant, not a re-entrant at the same id | `filter.rs:1298-1330` dispatch | `check_trigger_condition` → `matches_zone_change_event_object_filter` | `..._reads_original_entrant_not_reentrant_at_same_id` (new) | POSITIVE 3/3>2/2 true (pre-fix reads re-entrant 1/1 → false) | re-entrant 3/3 but original 1/1 → false (negative); empty `lki_cache` after clobber → case (c) record | `live_is_original_entrant` guard `obj.incarnation == record.incarnation` | no parser coverage change (engine-internal) |
| Stays-out exit-LKI path unchanged | `filter.rs` case (b) | same | `..._uses_exit_lki_after_leaving_battlefield` (existing, keep green) | exit-LKI 3/3 > 2/2 true; weak 1/1 → false | weak entrant exit-LKI 1/1 → false | `lki_cache.get(...).filter(incarnation==)` | unchanged |
| Still-present live path unchanged | `filter.rs` case (a) | same | `..._entering_greater_pt_than_source` (existing, keep green) | 3/3 true, 2/2 false, 1/2 false | 1/2,2/2,1/1 negatives present | live guard | unchanged |
| Record carries entry-time incarnation | `zones.rs:676` + `game_object.rs:1077` | `move_to_zone(_, Battlefield)` | new test Step 3 sanity-assert `record.incarnation == n` | assert_eq fails if record incarnation wrong | (binding fixture) | record overwrite after `reset_for_battlefield_entry` | n/a |
| Exit-LKI carries exit-time incarnation | `zones.rs:152-173` | `move_to_zone(_, leaving bf)` | covered transitively by case (b) keep-green + new test re-entry | case (b) selects only matching incarnation | clobbered cache (different incarnation) routes to (c) | exit-LKI literal stamp | n/a |
| serde back-compat | `#[serde(default)]` both fields | deserialize legacy `GameState` | (no new test; documented) | n/a — legacy `incarnation==0` routes conservatively | legacy mid-ETB record → case (c) record (still original entrant) | `#[serde(default)]` | n/a |

## Scope confirmation

In bounds (all under `/private/tmp/wt-msh-intervening-if/`): `crates/engine/src/types/game_state.rs`, `crates/engine/src/game/game_object.rs`, `crates/engine/src/game/zones.rs`, `crates/engine/src/game/effects/mod.rs`, `crates/engine/src/game/filter.rs`, `crates/engine/src/game/triggers.rs` (test), plus mechanical `incarnation:` additions to non-spread literal sites the compiler flags (`stack.rs:2041`, `derived_views.rs:1064` test, ~36 `LKISnapshot` literals, mostly test). Out of bounds: parser, `types/ability.rs` (no enum variant), `game/effects/*` beyond the one converter, any `mtgish/`, and the authoring agent's untracked `FIX-LOG`/`INVESTIGATION`/`PLAN-LOG`/`PR-BODY`/`.patch`/`.md` docs.

## Verification cadence (Tilt-first per CLAUDE.md)

`cargo fmt --all` directly. Then Tilt `clippy` + `test-engine` (worktree note says Tilt is down → fall back to direct `cargo` with `export PATH="$HOME/.cargo/bin:$PATH"`): run the new test plus the two keep-green tests (`cargo test -p engine zone_change_object_condition_entering`), then the full engine suite. Run `/validate-cr-annotations` over the new `// CR 400.7`/`// CR 608.2h` comments.

---

This plan is ready for `/review-engine-plan`. Key decisions: **option (i)** for the lki_cache-clobber dispatch (the only sound way to distinguish the original entrant's cached snapshot from a clobbered one, and the only option that keeps the prior fix's exit-LKI test green — proven by tracing that test's synthetic `None`-P/T record); the `incarnation` discriminator reuses the existing `GameObject.incarnation` so the add-engine-variant gate is **N/A (struct field, no enum variant)**; `#[serde(default)]` on both fields gives benign legacy back-compat; and the discriminating test fails-before (id-only check reads the re-entrant 1/1) and passes-after (incarnation mismatch routes to the original's 3/3) with explicit non-vacuity sanity asserts and a negative sub-case proving it reads the original, not the re-entrant.

Let me notify the team lead that the plan is complete.