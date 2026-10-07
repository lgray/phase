MODEL: claude-sonnet-5-5
Review Head: 1370f485471706db195f5a33b8cdf6dcbf670216 (delta e30f20af90..1370f48547, phase range bbce5c6bda..1370f48547). Phase mode, Phase 16 round 2 (delta round + one whole-artifact pass). Skill read at skills/review-scope-charter-addenda tip c51cf335d0 (`rev-parse HEAD skills/...` printed one SHA twice).

## Verdict
No findings. RE-CHARTER: not needed. Counts: HIGH 0, MED 0, LOW 0; blocking 0 (behavior 0, text 0, machinery 0). Both r1 LOWs closed.

## Checks
(a) Behavior LOW closed as a class. `shared_piles` now builds both fields from `state.shared_zone_holder(Zone::Library|Graveyard)`; `ZoneScope` import removed and `grep -n ZoneScope derived_views.rs` has no hit. `shared_zone_holder` body is the exact match the old closure restated (Shared -> `canonical_seat()`, PerPlayer -> None; Library/Graveyard arms read `shared_zones().library/.graveyard`), so Dandan output is {library:0, graveyard:0} and per-player formats still yield None. Change to game_state.rs is 2 changed lines (visibility only). Sweep over `git diff bbce5c6bda 1370f48547 -U0 -- 'crates/*.rs'` for `shared_zones()|canonical_seat()|shared_zone_holder|ZoneScope`: only the two calls and the visibility edit (positive control: instrument returned them). Other `shared_zones()` readers are outside the phase diff (executor sweep: mulligan, draw, effects/mod). Executor nextest `shared_piles|census|debug_library_projection|storage_seat` 72/72, rc 0 (nextest-r3.log); not re-run (no-build rule).
(b) Comment edits are exactly the r1 supplied text: cardAnchors.ts "hidden library card shows as its pile (the holder's, in a shared-zone format)."; cardFlightSpecs.ts "(hand, exile; library and graveyard unless the format shares them)". True: ZONE_SURFACES.Library/Graveyard route through `pileSeat` -> `resolvePileSeat`; Exile keys by `ownerId`.
(c) Non-comment lines in the two client files: `git diff -U0 e30f20af90 1370f48547 -- <file> | grep -E '^[+-]' | grep -v '^(+++|---)' | grep -vE '^[+-]\s*(//|/\*\*|\*)'` -> empty for both (rc 1). Control: same predicate over bbce5c6bda..1370f48547 on cardAnchors.ts returns the 3 phase-added code lines (imports, `pileSeat`).
(d) Scope: delta paths = {cardAnchors.ts, cardFlightSpecs.ts, derived_views.rs, game_state.rs}, all in scope.nul; phase range 27 paths vs scope.nul 27 entries, `comm -3` empty. addenda/phase-16 last two lines name game_state.rs (visibility only) + cardFlightSpecs.ts (doc-comment only) and cardAnchors.ts + its test; they cover the delta.

## Whole-artifact pass (bbce5c6bda..1370f48547)
Only touched surface is `shared_piles`/`SharedPilesView`; r1 lens results (display-layer purity, routing completeness, hidden-info projection, protocol constants, discriminating reverts) are unaffected by the delta: the only code edit is behaviour-preserving and engine tests E1-E4 still reach it via `derive_views`. Added-line sweep of non-test client code for "owner's|per-player|per-seat": remaining hits are accurate (exile stays per-seat; `getZoneViewerPile` doc; v113-peer protocol notes). Nothing r1 missed.

## Pre-existing
None new. (r1 list stands: three full-suite vitest timeouts pass in isolation; unrelated main-checkout modifications.)
