# Charter review r7 (charter-mode, whole charter, post-revision r5 + F1)

Reviewer model: claude-sonnet-5-5. Read-only; no cargo. Artifact: `phase-charter` vs `phase-charter.r5`.

## Verdict: CLEAN. 0 decision revisions, 0 review-only findings, 0 corrections.

## Edit check (F1)
- `diff phase-charter.r5 phase-charter` touches exactly two lines: the Phase 6 seam note and the Phase 11 seam note. No goal, scope rule, acceptance row, seam order, deferral or count moved.
- Measured in `engine_resolution_choices.rs` (the `(WaitingFor::DigChoice, GameAction::SelectCards)` arm, from ~4389): the raw writers (`player_state.library.retain/insert/push_back`, `allow-raw-zone`) sit inside `if kept_destination == Some(Zone::Library) { .. return .. }`. The kept-card map (`ZoneMoveRequest::effect(obj_id, kept_zone, ..)`, ~4589) sits in the following `if let Some(kept_zone) = kept_destination` branch of the same arm. The branches are disjoint (the first returns). So "a different branch of the same arm", with the `kept_destination == Some(Zone::Library)` qualifier, is exactly what the measurement shows. The replaced "different function" is gone from both notes (`grep "different function"` on the two seam notes: none left).
- Phase 6's scope still lists the file whole, so overlap-by-line ordering ("sequential") is the correct claim; Phase 11's "edits only the kept-card map" matches its scope-rule limiter.

## Whole-charter pass
- No new premise, ordering or seam problem found. Phase 6/8/11 seam notes remain mutually consistent; Phase 8 dig.rs note matches Phase 11's conditional dig.rs entry. Accepted phases 1-4 untouched. T1/T2 lines unchanged and still hold.
- Observation, not a finding (closed r6 ground): `ZoneMoveRequest` already has `mods.performed_by` and a `.performed_by(player)` builder (zone_pipeline.rs ~545), carried into the pending-batch form. Phase 11's claim "no recipient field and no producer supplies one" is a phase-start measurement, and the phase plan buys it; no decision rests on it.

No findings.
