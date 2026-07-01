# MSH-F implementation review — round 1 (mshf-impl-reviewer-r1)

VERDICT A (Cosmic Cube): CHANGES REQUIRED (1 HIGH). VERDICT B (Hawkeye): CLEAN (1 LOW optional).

## Evidence gathered
- Full diff read (9 files, +603/−72). All 9 discriminating tests RUN & PASS (cargo test -p engine --lib). clippy -p engine --all-targets -D warnings → exit 0. casting.rs + casting_costs.rs UNTOUCHED (runtime enforcement genuinely unchanged). CRs 107.1b/107.3a/109.4/202.3/601.2e/601.2f/120.1/614.1a grep-resolve.

## VERDICT A — CHANGES REQUIRED (one HIGH)
**[HIGH] casting.rs:1592-1598 — A2 deferral NOT acceptable; a focused unit test on `cast_permission_constraint_allows_cast` is REQUIRED (heavy end-to-end harness is NOT).**
Traced every `CastPermissionConstraint::ManaValue` producer: parser `parse_beseech_mv_constraint`→Fixed and the NEW `parse_with_mana_value_constraint`→can be Ref; engine-internal search_library.rs:1326 Fixed{4}, casting_costs.rs:11846 Fixed{source_mv} (Cascade), engine_resolution_choices.rs:549 Fixed{discover_value} (Discover); cast_from_zone.rs:379 clones the parser value. ⇒ **Every pre-existing dynamic ceiling is a computed Fixed. Cosmic Cube is the FIRST card to route `Ref(Aggregate{Max,Power,filter})` through `cast_permission_constraint_allows_cast` at finalize.** Executor's "already-covered by other dynamic-constraint cards" is factually wrong (those are Fixed, not Ref). The `resolve_quantity(Aggregate)`→`comparator.evaluate` composition at this seam has ZERO direct coverage. A1 proves only AST shape, never the runtime numeric ceiling / accept-below-reject-above / non-attacker exclusion. The stale-symlink + heavy-harness excuses are valid ONLY for end-to-end; they do NOT cover a focused unit test — `cast_permission_constraint_allows_cast` is `pub(super)`, callable from casting.rs's own `#[cfg(test)] mod tests` with no card-data/harness. HIGH not BLOCKER (code unchanged, machinery independently tested) but the headline behavior must not ship with zero runtime coverage when the cheap test is right there.

Exact required test (crates/engine/src/game/casting.rs tests module):
```
#[test] fn cosmic_cube_dynamic_mv_ceiling_enforced_at_finalize() {
  // Board: 2 attacking creatures you control, power 2 and 3 (ceiling=3),
  //        + 1 NON-attacking power-7 creature you control (must NOT raise ceiling).
  // obj = spell on stack, controller = caster (P0).
  // c = Some(ManaValue{ LE, Ref(Aggregate{Max, Power, attacking creatures you control}) }).
  assert!( cast_permission_constraint_allows_cast(&state, obj, &c, Some(3)));   // 3<=3 accept
  assert!(!cast_permission_constraint_allows_cast(&state, obj, &c, Some(4)));   // 4>3 reject (non-attacker didn't lift ceiling)
  assert!( cast_permission_constraint_allows_cast(&state, obj, &c, None));      // offer-time permissive
}
```
Non-vacuous (Some(4)=false fails if Aggregate mis-resolves or non-attacker leaks) + discriminating (Some(3)/Some(4) split fails if resolve/compare wrong). Do NOT mark Sub-Plan A fixed-unreleased until it lands.
Everything else in A clean: nom-pure (grep .contains/.split_once/.starts_with/.find on added lines → empty); comparator longest-match LE-before-LT verified; Beseech regression-guarded; full-line parse test revert-discriminating (constraint null in card-data today). CR 202.3 + 601.2e verified.

## VERDICT B — CLEAN (one LOW optional)
Field-lift Plus{u32}→{QuantityExpr} correct & complete: all Plus sites migrated (no bare `Plus{value:<int>}` remains); `Minus{u32::MAX}` continuous-prevent sentinel INTACT (Minus still u32, replacement.rs:986); QuantityModification NOT in diff (NON-GOAL honored); SetTo/SetToSourcePower untouched. Resolver Plus arm: controller discriminator mirrors `damage_modification_for_rid`'s `rid.source==ObjectId(0)` branch exactly; pending source_controller is Option<PlayerId> (ability.rs:16151); `resolve_quantity(state,&value,controller,rid.source)` param order matches quantity.rs:68; `.max(0) as u32` = CR 107.1b. Behavior-identical for all Fixed (controller/source_id ignored by Fixed); only Ref-on-pending-sentinel sensitivity has no card today (Hawkeye object-hosted, rid.source=Hawkeye). B2 drives real replace_event pipeline, proves live re-read (power 2→5, 4→7), covers negatives (combat not amplified, own-permanent not amplified), revert-discriminating (frozen→3). B1 revert-discriminating; B3 serde incl. non-int rejection. 7 Fixed-wrap migrations preserve intent.
**[LOW optional]** oracle_replacement.rs:~4948 — inline CR 107.3a on the *dynamic* arm is a stretch (Hawkeye's X is card-text-defined, not announced/undefined X). Function-level doc legitimately needs 107.3a for the bare-"plus x" freeze arm (Taii Wakeen). Defensible as-is; if tightening, scope 107.3a to the freeze arm. Not blocking.

## Main-side follow-ups (unaffected by findings)
#2 engine-inventory regen (Plus field u32→QuantityExpr); #3 cargo coverage/semantic-audit post card-data flip.
