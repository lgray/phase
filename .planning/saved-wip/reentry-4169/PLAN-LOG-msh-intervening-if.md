# Plan Log — ETB intervening-if recheck reads reverted entrant P/T

## Verified facts (rebased worktree /private/tmp/wt-msh-intervening-if @ 8f6a1c631)

- Hulkling commit `8f6a1c631` already on branch (entering-creature-vs-source P/T intervening-if).
- `matches_zone_change_event_object_filter` at filter.rs:1276; target branch at **1298-1302**.
  - `destination == Zone::Battlefield` => `matches_target_filter(state, *object_id, filter, ctx)` (reads LIVE obj).
  - else => `matches_target_filter_on_zone_change_record(state, record, filter, ctx)`.
- `matches_target_filter_on_lki_snapshot` at filter.rs:1232-1268 — synthesizes ZoneChangeRecord from LKISnapshot, carries power/toughness/base_power/base_toughness/types/subtypes/supertypes/keywords/colors/mana_value/controller/owner; delegates to `matches_target_filter_on_zone_change_record`. (Design check 3: non-P/T fields present.)
- `effective_controller` gating model at filter.rs:680-694: `LiveOrLki && !matches!(obj.zone, Battlefield|Stack) && lki_cache.get(id).is_some()`.
- `object_pt_value` (filter.rs:2921-2926) Current scope reads `obj.power/obj.toughness` => after revert, base. Bug confirmed.
- zones.rs:147-175 snapshots LKI (incl base_power/base_toughness/counters) on `from==Battlefield||from==Exile`.
- zones.rs:395 & 429 call `revert_layered_characteristics_to_base()`.
- LKISnapshot struct: game_state.rs:162-204 — has all needed fields.
- Two callers, both via the single function: triggers.rs:5448-5461 (TriggerCondition) + effects/mod.rs:7013-7026 (AbilityCondition). Single-point fix covers both.
- Production zone-change entry point: `move_to_zone` (zones.rs:522), signature `(state, object_id, to, events: &mut Vec<GameEvent>)`.
- Existing PR test: `zone_change_object_condition_entering_greater_pt_than_source` (triggers.rs:9081-9184) — asserts 3/3→true, 2/2→false, 1/1→false, 1/3→true, 1/2→false (entrant STILL on battlefield).
- LKI-cache test pattern model: `zone_change_object_condition_checks_dead_object_snapshot` (triggers.rs:9187-9252) — but populates lki_cache manually; new test must use the PRODUCTION path (`move_to_zone`) instead.
- Test helpers: `setup()` (6458), `create_object` (zones.rs:465), `make_creature` (triggers.rs:6503 — sets base_power/base_toughness AND power/toughness), `zone_changed_event` (6467), `check_trigger_condition` (5119).

## CR rules verified in docs/MagicCompRules.txt
- 608.2h (2802): specific-object info uses current if in expected public zone, else LKI.
- 603.4 (2588): intervening-if rechecked at resolution.
- 603.6 (2593): zone-change triggers look for object in zone it moved to; may have left.
- 603.10a (2634): ETB is NOT a look-back trigger (look-back = LTB/sacrifice/leaves-graveyard/public-to-hidden) => normal 608.2h, not 608.2i.
- 208.4b (1525): base P/T scope.
- 400.7 (1948): new object, no memory; 400.7e public-zone find.

## Fix
filter.rs:1298-1302 Battlefield branch:
```
if destination == Zone::Battlefield {
    // entrant still live on battlefield => current info (CR 608.2h)
    let live_on_bf = state.objects.get(object_id)
        .is_some_and(|o| o.zone == Zone::Battlefield);
    if live_on_bf {
        matches_target_filter(state, *object_id, filter, ctx)
    } else if let Some(lki) = state.lki_cache.get(object_id) {
        // entrant left => LKI (CR 608.2h "most recently existed")
        matches_target_filter_on_lki_snapshot(state, *object_id, lki, filter, ctx)
    } else {
        // no exit LKI => ETB record fallback (never regress to reverted live obj)
        matches_target_filter_on_zone_change_record(state, record, filter, ctx)
    }
} else { ... unchanged ... }
```
Annotate CR 608.2h + CR 400.7 + CR 603.4. Missing-object entrant also routes to LKI (live_on_bf=false when objects.get is None).
