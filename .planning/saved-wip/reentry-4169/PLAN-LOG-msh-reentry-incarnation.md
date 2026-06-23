# Plan log: ETB intervening-if re-entry incarnation discriminator

## Confirmed facts (re-grepped after rebase on f4053d0e3)
- filter.rs:1311-1314 still_on_battlefield = objects.get(id).zone==Battlefield (ObjectId only). Battlefield branch 1298-1331.
- filter.rs:1232-1268 matches_target_filter_on_lki_snapshot reconstructs a ZoneChangeRecord from LKISnapshot — MUST carry incarnation if both gain the field.
- game_state.rs:162 LKISnapshot (no incarnation), :362 ZoneChangeRecord (no incarnation), :515 #[cfg(test)] impl test_minimal (:522).
- game_object.rs:524-532 incarnation field (#[serde(default)]); :1071 snapshot_for_zone_change builds record from live obj; :1307 snapshot_public_characteristics builds LKISnapshot; :1350-1354 reset_for_battlefield_entry bumps incarnation.
- zones.rs:620 record built PRE-move (OLD incarnation); :676 reset_for_battlefield_entry bumps; :738 event emitted. zones.rs:152-173 exit-LKI cache builds LKISnapshot literal INLINE (does NOT call snapshot_public_characteristics).
- create_object (zones.rs:465) does NOT call reset_for_battlefield_entry -> incarnation stays 0. So a test obj on bf has incarnation 0, matching test_minimal record incarnation 0. Prior test stays green.
- ZoneChangeRecord literal sites: most are #[cfg(test)] or use ..test_minimal()/..Default spread. Full-literal PRODUCTION sites needing manual incarnation: synthesis.rs:11529, derived_views.rs:1064, stack.rs:2041 (zone_change_record_from_spec, token probe), effects/mod.rs:15159 & 15505, restrictions.rs:2760/2879/2900/2954, attach.rs:1671/1679, zones.rs:1653 (test). LKISnapshot inline literal sites: zones.rs:152 (PROD exit-LKI), game_object.rs:1308 (method), plus many test literals.
- CR verified in docs/MagicCompRules.txt: 400.7 (L1948), 608.2h (L2802), 603.4 (L2588). 603.10a present.

## Open question resolution: lki_cache clobber
- lki_cache keyed by ObjectId only, no incarnation tag. After original leaves+caches, a re-enter/re-exit clobbers it.
- DECISION: option (i) — add incarnation to BOTH ZoneChangeRecord and LKISnapshot.
- Dispatch in filter.rs Battlefield branch:
  (a) obj present, zone==Battlefield, obj.incarnation==record.incarnation -> live matches_target_filter (current info, CR 608.2h in-zone).
  (b) else if lki_cache holds snapshot AND lki.incarnation==record.incarnation -> exit-LKI (preserves prior fix's pumped-then-died exit-time P/T).
  (c) else -> event record (matches_target_filter_on_zone_change_record): covers absent obj, different live incarnation (re-entry, original gone), and clobbered/mismatched lki_cache.
- This keeps zone_change_object_condition_entering_uses_exit_lki_after_leaving_battlefield green: entrant created via create_object (incarnation 0), record from zone_changed_event uses test_minimal (incarnation 0), exit LKI now stamped incarnation 0 -> (b) fires -> exit-LKI 3/3. 
