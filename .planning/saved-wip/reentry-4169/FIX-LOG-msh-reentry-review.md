# PR #4169 review-fix log (card/msh-intervening-if) — [MED] entrant re-enters as new object

Rebased onto upstream/main (3a844e56d..f4053d0e3). 2 commits on branch: Hulkling feature (59373bcd6) + prior LKI fix (95c8cd8f7). Worktree clean except authoring agent's untracked docs.

## Finding (matthewevans MED, PARTIAL) — re-entry as new object reuses ObjectId, recheck reads the NEW object

VERIFIED in code:
- The prior fix (95c8cd8f7) added still_on_battlefield at filter.rs:1311-1314:
  `state.objects.get(object_id).is_some_and(|obj| obj.zone == Zone::Battlefield)`.
  If true → matches_target_filter (live); else → exit LKI.
- ObjectId is STORAGE identity that persists across a zone change (zones.rs:120-126:
  "An object that changes zones becomes a new object... ObjectId here is storage identity and
  persists across the zone change"). So if the original entrant leaves AND a new object re-enters
  the battlefield reusing the same storage ObjectId, still_on_battlefield is TRUE for the NEW object,
  and the recheck reads the NEW object's P/T via matches_target_filter — NOT the original entrant's
  exit LKI. Per CR 400.7 the re-entered permanent is a NEW object; per CR 608.2h the recheck needs the
  ORIGINAL entrant's last-known info.

EXISTING BUILDING BLOCK (REUSE — no new concept):
- GameObject.incarnation (game_object.rs:524-532): "Monotonic per-object incarnation, bumped on every
  battlefield entry (reset_for_battlefield_entry). A permanent that leaves and re-enters becomes a new
  object even though the engine reuses its ObjectId... Pairing the id with this counter distinguishes
  the new object from the old one at the same id, so a pending ability that captured the previous
  incarnation no longer resolves its self-reference against the re-entered permanent (blink/flicker)."
  This is EXACTLY the discriminator needed. reset_for_battlefield_entry (game_object.rs:1350-1354)
  does self.incarnation += 1 on every battlefield entry.
- incarnation is NOT currently carried in ZoneChangeRecord, the ZoneChanged event, or LKISnapshot
  (grep confirms absent). So the fix must propagate it into the event-time record.

CR basis (grep-verify in docs/MagicCompRules.txt):
- CR 400.7: an object that changes zones is a NEW object with no memory of its previous existence.
- CR 608.2h: info from a specific object uses current info if still in the expected public zone, else LKI.
- CR 603.4: intervening-if rechecked at resolution.

## FIX
1. Add `incarnation: u64` field to ZoneChangeRecord (types/game_state.rs:362, #[serde(default)]).
   This is a STRUCT field, NOT an enum variant — add-engine-variant gate is N/A (let planner confirm).
   Reuses the existing GameObject.incarnation concept; no new semantic.
2. Populate it in zones.rs:
   - zone_change_record built at zones.rs:620 from the PRE-move obj (so it has the OLD incarnation).
   - For to == Battlefield, reset_for_battlefield_entry (zones.rs:676) bumps incarnation AFTER the
     snapshot. So after line 676 (ETB case), set zone_change_record.incarnation = obj_mut.incarnation
     (the NEW post-entry incarnation). For non-Battlefield destinations the snapshot's pre-move
     incarnation is correct (the object's incarnation at the moment it left).
   - Verify ALL ZoneChanged event-emission sites (zones.rs:738, 1002, 1653) carry the populated record.
3. In matches_zone_change_event_object_filter (filter.rs:1311), tighten still_on_battlefield: require
   BOTH obj.zone == Battlefield AND obj.incarnation == record.incarnation. If the incarnation differs,
   the original entrant has left (and a new object may occupy the id) → fall through to the LKI/record
   path (exit LKI of the ORIGINAL incarnation, keyed by ObjectId in lki_cache).
   NOTE: lki_cache is keyed by ObjectId only; on re-entry-then-re-exit it could be overwritten by a
   later incarnation. Planner must check: does lki_cache hold the ORIGINAL entrant's exit snapshot at
   recheck time, or could a re-entry+re-exit have clobbered it? If clobbered, the `record` (carried in
   the ETB event itself, immune to later mutation) is the authoritative fallback — verify the record
   path (matches_target_filter_on_zone_change_record) reads the right entrant snapshot. The ETB event's
   `record` is the entry-time snapshot of the ORIGINAL entrant, which is the most robust authority here.

## DISCRIMINATING TEST
Runtime, triggers.rs test module near the existing exit-LKI test:
- Source (Hulkling 2/2) on battlefield. Original entrant with on-battlefield P/T GREATER than source
  (3/3), base 1/1; build its ETB ZoneChanged event (capturing its incarnation N).
- Move the entrant OFF the battlefield (battlefield→graveyard via move_to_zone), THEN re-enter a permanent
  at the SAME storage ObjectId with P/T NOT greater than source (e.g. 1/1) via the production entry path
  (so incarnation bumps to N+1). Confirm state.objects[id].incarnation == N+1 != N (event's N), and the
  re-entered live obj is 1/1.
- Recheck check_trigger_condition with the ORIGINAL ETB event → MUST be true (original entrant's LKI 3/3 >
  2/2). Revert-probe: the CURRENT prior-fix code reads the NEW object (1/1, incarnation matches by id only,
  still_on_battlefield true) → 1/1 not > 2/2 → false → test FAILS on pre-fix.
- Negatives: (a) entrant still on battlefield same incarnation → live path unchanged (existing tests stay
  green); (b) original entrant left, re-entrant is GREATER (3/3) but original LKI was 1/1 → must be false
  (proves it's the ORIGINAL entrant's value via LKI, not the re-entrant's).
Provide fail-before/pass-after evidence.
