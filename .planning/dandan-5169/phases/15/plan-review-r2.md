# Phase 15 plan review, round 2 (phase-plan mode, Sizing consistency blocking)

MODEL: claude-sonnet-5-5

Reviewed the whole revised `plan.md` (rev 1) against r1, Phase 14's `plan.md`, and code at HEAD. No cargo run; claims rest on reads/greps this session.

## Verdict: REVISE (1 blocking `text` finding; F1, F2 and the self-found fix verified closed)

## Closure checks (measured)
- F1 closed. Sections 3.2, 3.3, U1, M3/M6, P1 now re-tag only the shuffle; Draw keeps `ContinuationStep`. Verified `scoped_library_search.rs::is_plain_parent_target_delivery` ends with `delivery.sub_link == SubAbilityLink::ContinuationStep` and has `player_scope.is_none()` / `sub_ability.is_none()` (the two covered by Phase 14's four caller-specific fields / tail detach). `detach_after_player_scope_local_chain` (mod.rs ~6887-7000): `next_is_local_continuation = next.sub_link == ContinuationStep && is_player_scope_local_continuation(..)`; move2 -> shuffle(SequentialSibling) is returned as the tail, so only the shuffle re-tag matters. Draw in the tail keeps its own `player_scope` (the clear happens only in the local branch).
- F2 closed (W1 (d) rewritten, 724.1b annotation dropped; `end_the_turn.rs` uses `exile_nonresolving_stack_objects`).
- Self-found `detached_remainder` fix is right: `grep detached_remainder` shows it is read only by `is_sole_chain_producer` (mod.rs ~8090) and the stack.rs publish gates; it is absent from `is_plain_parent_target_delivery`, so it is not a dealer-gate input.
- Fresh traces that held: driver `after_scope` runs through `resolve_ability_chain` after the per-seat loop; parked path appends `after_scope` via `append_to_pending_continuation`; `ChangeZoneAll` `TerminalShuffle` bottom-places (`change_zone.rs` ~1985) so the detached shuffle is the single randomization; `effect_has_iteration_bound_recipient` ignores `ChangeZoneAll`/`Shuffle` so the re-tag, not the first rule, is the discriminator; CR 724.1d grepped (skip to cleanup).

## F1 (blocking, `text`): W1 positive reach-guard miscounts the hand population
Old string (W1 row, "Sibling / hostile / paired positive" column): "the move events cover all 7 hand cards and all 4 graveyard cards".
Measured against the same row's fixture ("P0 hand 3 real cards, P1 hand 2"): the hand population is 3 + 2 = 5 (W3 in the plan itself says "all 5 discards" for the identical fixture; the cast spell leaves the hand). An executor asserting 7 move events from Hand turns the guard permanently red.
Replacement: "the move events cover all 5 hand cards and all 4 graveyard cards (9 `ZoneChanged { to: Library }` events from Hand/Graveyard)".

## Non-findings
Sizing consistent (1 unit, 1 production path, tests excluded); no scope additions needed; no re-raise of r1 items.
