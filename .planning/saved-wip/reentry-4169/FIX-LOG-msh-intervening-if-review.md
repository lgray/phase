# PR #4169 review-fix log (card/msh-intervening-if, Hulkling)

Rebased onto upstream/main (c8c1f6855..2bc9b25f5). Worktree clean except authoring agent's untracked docs (untouched).

## Finding (matthewevans MED) — resolution-time recheck compares against reverted/baseline P/T after entrant leaves

VERIFIED in code:
- Hulkling condition = TriggerCondition::ZoneChangeObjectMatchesFilter { destination: Battlefield,
  filter: AnyOf[ PtComparison{stat, scope:Current, GT, Ref(Power/Toughness{scope:Source})} ] }
  (oracle_trigger.rs feature commit).
- triggers.rs:5448-5461 evaluates it via matches_zone_change_event_object_filter(state, event, origin, dest, filter, from_source).
- filter.rs:1298-1302: for `destination == Zone::Battlefield`, calls matches_target_filter(state, *object_id, filter, ctx)
  → reads the LIVE GameObject. For non-Battlefield it uses matches_target_filter_on_zone_change_record (record = LKI).
- filter.rs PtComparison eval (3438-3446): object_pt_value(obj, stat, Current) reads obj.power/obj.toughness
  (filter.rs:2921-2926 — Current → obj.power; Base → obj.base_power).
- zones.rs: an object is NOT removed from state.objects on zone change (kept, zone field updated).
  zones.rs:147-173 snapshots exit LKI into state.lki_cache (power, toughness, base_power, base_toughness, counters)
  on from==Battlefield, keyed by ObjectId.
  zones.rs:428-429 calls obj.revert_layered_characteristics_to_base() on battlefield exit.
- game_object.rs:1435-1456 revert_layered_characteristics_to_base: power = base_power, toughness = base_toughness
  — DISCARDS all layer-7 modifications (boosts, counters' P/T effect).

CONCLUSION: if the entrant leaves the battlefield before the Hulkling trigger resolves, obj.power/obj.toughness
are reverted to printed base, so object_pt_value(obj, Current) returns BASELINE, not the entrant's last-known
on-battlefield P/T. The recheck thus compares wrong values. (If the object's zone is now Graveyard etc., the
filter still reads the live-but-reverted obj — it is NOT removed from state.objects.) Maintainer is CORRECT.

CR basis (grep-verified docs/MagicCompRules.txt):
- 608.2h (line 2802): "If the effect requires information from a specific object ... uses the current
  information of that object if it's in the public zone it was expected to be in; if it's no longer in that
  zone ... uses the object's last known information." → recheck of "if it has greater power/toughness than ~"
  requires the entrant's P/T; entrant gone from battlefield → MUST use its LKI.
- 603.4 (2588): intervening-if rechecked at resolution.
- 603.6 (2593): zone-change triggers look for the object in the zone it moved to; if not found (left before
  resolution), the part acting on it fails. (The EFFECT "put counter on ~" targets the SOURCE, not the
  entrant, so it still resolves; only the intervening-if INFO lookup on the entrant needs LKI.)
- 603.10a (2634): ETB is NOT a look-back trigger → normal 608.2h (current-else-LKI) applies, not 608.2i.

## Existing building block (REUSE, no new variant)
- filter.rs:1232 matches_target_filter_on_lki_snapshot(state, object_id, lki, filter, ctx): synthesizes a
  ZoneChangeRecord from an LKISnapshot and evaluates the filter against it (CR 400.7 + CR 608.2h annotated).
  It already carries power/toughness/base_power/base_toughness/counters → PtComparison{Current|Base} reads them.
- Canonical LKI-fallback pattern: effective_controller (filter.rs:686-689) — when obj.zone not in
  {Battlefield, Stack} AND lki_cache has a snapshot, use the LKI. Mirror this gating.

## FIX (single shared building-block point)
In matches_zone_change_event_object_filter (filter.rs:1298-1302), the `destination == Zone::Battlefield`
branch: if the live entering object is still on the battlefield (obj.zone == Battlefield), use the live
matches_target_filter (current behavior). Otherwise (entrant has left) prefer state.lki_cache.get(object_id)
→ matches_target_filter_on_lki_snapshot; if no LKI is cached, fall back to the ETB `record`
(matches_target_filter_on_zone_change_record) so we never silently regress to the reverted live obj.
Annotate CR 608.2h + CR 400.7 + CR 603.4.

This fixes BOTH callers of the shared fn:
- triggers.rs:5453 TriggerCondition::ZoneChangeObjectMatchesFilter (Hulkling, the report)
- effects/mod.rs:7018 AbilityCondition::ZoneChangeObjectMatchesFilter (same class)
Covers the whole class of ETB intervening-if filters reading entrant characteristics, not just P/T.

## DISCRIMINATING TEST
Runtime: source (Hulkling, P/T e.g. 2/2) on battlefield; an entering creature with on-battlefield P/T
GREATER than source (e.g. 3/3) generates the ZoneChanged event; then MOVE the entrant off the battlefield
(battlefield→graveyard) so it reverts to a baseline that is NOT greater than source (e.g. base 1/1).
Recheck check_trigger_condition with the ETB event → MUST be true (uses entrant's exit LKI 3/3 > 2/2).
Revert-probe: pre-fix reads reverted live obj (1/1) → 1/1 NOT > 2/2 → false → test fails on pre-fix.
Also assert a negative: entrant whose LKI is NOT greater (e.g. left at 1/1) → false (no spurious-true).
Model on the existing runtime gating test in the feature commit (triggers.rs ~9075+).
