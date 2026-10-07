# Phase 16 executor report r1

Mode: implementation/fix (phase mode). BASE_SHA = START_SHA = bbce5c6bdab3c24a727d5d5f776cda093c46b15a. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.

## Verdict
Implemented. One stop-and-return item (out-of-scope pile anchor site, below). All evidence is PREPARATORY, not completion evidence.

## 1. Diff summary (23 scope paths, delta == scope.nul exactly, nothing outside)
- Engine: `crates/engine/src/game/derived_views.rs` adds `SharedPilesView`, `DerivedViews.shared_piles`, `shared_piles()` (exhaustive `ZoneScope` match; holder = `canonical_seat()`), 5 inline tests.
- Protocol 113 -> 114 / wire 95 -> 96 / lobby 16 unchanged: `lobby-broker/src/protocol.rs`, `server-core/src/protocol.rs` (pin test renamed `protocol_version_is_114_for_shared_piles_view`), `ws-adapter.ts`, `network/protocol.ts`, `protocol.test.ts`, `p2p-adapter-multiplayer.test.ts` (literals, title, comments), `scripts/check-protocol-version.mjs` (+43 / +42, new `// +43:` / `// +42:` lines, header line). `RESOLUTION_STATE_WIRE_VERSION` untouched.
- Client authority: `adapter/types.ts` (`SharedPilesView`, `shared_piles?`), `viewmodel/gameStateView.ts` (`getSharedPileHolder`, `resolvePileSeat`, `getZoneViewerPile`; `getPlayerZoneIds` routed).
- Consumers: `GraveyardPile.tsx`, `LibraryPile.tsx` (`pileSeat`; `isMyLibrary = pileSeat === myPileSeat`), `useCastableZoneObjects.ts`, `AnimationOverlay.tsx` (2 mill anchors + CardsRevealed anchor), `GamePage.tsx` (opponent widget renders Library/Graveyard only when no holder; auto-open loop via `getZoneViewerPile`), `ZoneViewer.tsx` (seat-identity delve clause deleted).
- Tests: engine rows E1-E5 in `derived_views.rs`; client `gameStateView.test.ts`, `PileAnchors.test.tsx`, `LibraryPile.test.tsx`, `ZoneViewer.test.tsx`, new `useCastableZoneObjects.test.tsx`, `AnimationOverlay.sharedPileAnchors.test.tsx`, `GamePage.sharedPiles.test.tsx`.

## 2. Worktree record
Start: HEAD == START_SHA, tree clean, no staged entries (verified before editing). End: HEAD == bbce5c6bda, 0 staged entries, delta == the 23 scope paths (`comm` against scope.nul: nothing outside, nothing untouched). Preparatory evidence is not completion evidence.

## 3. PREPARATORY verification (START_SHA bbce5c6bda -> same HEAD, uncommitted)
- `cargo fmt --all -- --check` rc=0 (applied to the three changed .rs paths only).
- Engine focused: `cargo nextest run -p phase-engine --features test-support --lib -E 'test(shared_piles)|test(debug_library_projection)|test(census)'` 34/34 pass.
- Pins: `cargo nextest run -p lobby-broker -p server-core -E 'test(protocol_version)'` 3/3 pass. `node scripts/check-protocol-version.mjs` red after bumping only `PROTOCOL_VERSION` ("Rust=114, client=113"), green rc=0 after the pins.
- `scripts/check-interaction-bindings.sh --check` rc=0 (no diff).
- Client: `pnpm exec tsc -b --noEmit --force` clean; eslint on the 19 touched client files: 0 errors (2 warning classes pre-existing: `ZoneViewer` useMemo dep `playerId`, `GamePage` react-refresh exports); vitest on the 7 new/edited test files and `localeParity.test.ts` green. No `t(` added, no i18n file touched. Full vitest / full lint / clippy / full nextest not run (per coordinator correction: orchestrator runs at acceptance).
- Protocol numeral sweep: `grep -rnE '\b113\b'` over crates/client/scripts: only comments/history and unrelated literals remain.

## 4. Parser gate
Not applicable (no file under `parser/`).

## 5. Production-path coverage map
All client rows drive real components/hooks/pages reading a serialized `derived` object (the same JSON shape E1 asserts). Revert = the named single-line change, run against the 7 client test files; engine revert = `shared_piles` returns `None`.

| Claim | Seam | Entry | Test | Fails on revert |
|---|---|---|---|---|
| E1 descriptor names holder (Dandan) | `shared_piles` | `derive_views` + JSON | `shared_piles_names_the_holder_for_a_shared_zone_format` | revert -> None: FAIL (4/6 rows fail, run shown) |
| E2 absent for per-player | same | standard/commander | `shared_piles_is_absent_for_per_player_formats` | unconditional descriptor would fail the `None` asserts; the paired `is_some` leg fails under None-revert |
| E3 every projection carries it | `derive_filtered_views`, `wrap`, `wrap_filtered` | viewers P0/P1/None | `shared_piles_survives_every_viewer_projection` | FAIL under None-revert |
| E4 holder = lowest seat | `canonical_seat` | `seat_order=[P1,P0]` | `shared_piles_holder_is_the_lowest_seat_not_the_first_in_turn_order` | FAIL under None-revert |
| E5 debug library for non-holder | `debug_library_cards` | `derive_views(Some(P1))` | `debug_library_projection_reads_the_shared_pile_for_the_non_holder_seat` | green guard: Phase 8 already landed the `library_of` read (stays PASS under revert, as predicted) |
| C1 graveyard pile anchor | `GraveyardPile` | render `playerId=1` shared | `PileAnchors` shared + per-seat sibling | `resolvePileSeat -> seat`: FAIL |
| C2 play from shared top, non-holder | `LibraryPile.isMyLibrary` | viewer seat 1 (`online`, `activePlayerId` 1) | `LibraryPile shared library` (3 rows incl. non-shared refusal sibling) | `isMyLibrary = playerId === myId`: FAIL (only that row) |
| C2b library pile resolution | `LibraryPile` count/top/anchor | render `playerId=1` | same file | `resolvePileSeat -> seat`: FAIL |
| C3 anchors (mill, reveal) | `AnimationOverlay` | `ZoneChanged` Library->Graveyard owner 0 AND 1, `CardsRevealed` player 1 | `AnimationOverlay.sharedPileAnchors` | `resolvePileSeat -> seat`: owner-1 mill row and reveal row FAIL; owner-0 row + non-shared sibling pass (reach-guards) |
| C4 render once | `GamePage` opponent widget | `GamePage` shared state, viewer 0 and 1 | `GamePage.sharedPiles` render-once rows | widget guards forced `true`: both rows FAIL; non-shared sibling asserts both seats' anchors |
| C5 one auto-open group | `getZoneViewerPile` in the GamePage loop + helper | TargetSelection over graveyard cards of both owners, viewer seat 1 | `GamePage.sharedPiles` auto-open row; `gameStateView` rows | `getZoneViewerPile` owner-keyed: both FAIL; non-shared sibling: no dialog, prompt rendered (reach-guard) |
| C6 delve for non-holder | `ZoneViewer.canDelveFromGraveyard` | `ManaPayment` Delve, seat 1, shared graveyard | `ZoneViewer delve from a shared graveyard` | clause restored: FAIL; no-action sibling asserts nothing dispatched |
| C7 castable wings | `useCastableZoneObjects` | seat-1 hook, shared graveyard | `useCastableZoneObjects.test.tsx` | `resolvePileSeat -> seat`: FAIL; non-shared sibling |
| P1 protocol | pins/script | script + pin tests | red then green shown above | n/a |

Every changed behavioral seam has a mapped production-path test. No shape-only test.

## 6. Maintainer-simulation matrix
Single authority: `DerivedViews.shared_piles` (holder `Option<PlayerId>` per zone). Bound at every snapshot from `format.shared_zones()` + `canonical_seat()` (live; no stored copy), stored in the serialized `derived`, consumed by `resolvePileSeat` / `getSharedPileHolder` / `getZoneViewerPile` in `viewmodel/gameStateView.ts` (consumers: the rows above). Invalidation: none (format and seat set fixed). Hostile rows: viewer is the non-holder (C2, C3, C4, C5, C6, C7); milled card owned by the non-holder (C3); graveyard holding cards of both owners (C5); starting player is not the holder (E4). Serde impact: one optional key, absent for non-shared formats (E2); protocol 114 / wire 96. No incomplete row.

## 7. CR-annotation gate
Only added citation: `CR 400.1` (doc comment on `SharedPilesView`), grepped: line "Each player has their own library, hand, and graveyard." matches the annotation's subject. Zero UNVERIFIED.

## 8. Judgement calls
- E3 uses `wrap_filtered` (viewer path) plus `wrap` for P1 rather than `wrap(None)` on its own.
- AnimationOverlay reads the holder from `useGameStore.getState().gameState` (constant for the game, so pre/post state agree).
- `GamePage` auto-open effect gained `gameState` as a dependency (same churn as the existing `objects` dependency).

## 9. Stop-and-return items
1. **Out-of-scope owner-keyed pile anchor: `client/src/components/animation/cardVfx/cardAnchors.ts`** (not in scope.nul; found by `git grep -n -E 'data-(library|graveyard)-pile' -- client/src ':!*__tests__*'`, which the plan's census greps (`\.(graveyard|library)\b`, `<(Library|Graveyard)Pile`) did not cover). `ZONE_SURFACES.Graveyard` selects `[data-graveyard-pile="${ownerId}"][data-grouped-ids~=id]`, `ZONE_SURFACES.Library` falls back to `[data-library-pile="${ownerId}"]`, and `PROVISIONAL_SURFACES.Graveyard` selects `[data-graveyard-pile="${ownerId}"]`, with `ownerId` = `route.ownerId` (`cardFlightSpecs.ts` builds it from `obj.owner`, `CardVfxLayer.tsx` also passes `origin.ownerId`). Under a shared graveyard a flight of a non-holder-owned card finds no pile node (aim -> hold/Classic). Smallest fix: resolve through `resolvePileSeat` at the three selector sites (or where `route.ownerId` is built); needs `cardAnchors.ts` (+ a test beside `cardAnchors.test.ts` / `CardVfxLayer.test.tsx`) added to scope. Not edited. Also `client/src/components/animation/cardVfx/gallery/galleryScenarios.ts` reads `player.library/graveyard` on synthetic gallery state (dev gallery, not game state; no fix needed).
2. None other.

## 10. CR annotations added
`CR 400.1` verified with `grep -nE "^400\.1[^0-9]" docs/MagicCompRules.txt`.

## 11. Deviations from the plan (premises measured false at the base)
- `getCastableZoneViewerTarget` no longer exists (removed by #9459); the auto-open loop in `GamePage.tsx` is the only owner-keyed grouping left, so item 3.3.6(b) reduces to that loop and `gameStateView.test.ts` has no compiler-forced call-site edits.
- Plan protocol numerals 98/80 stale: executed 113->114 / 95->96 (script offsets +43/+42).
- `debug_library_cards` already reads `library_of` (Phase 8 landed): E5 is a green guard, not red-at-base.
- Plan's "two Rust constants" = `lobby_broker::PROTOCOL_VERSION` only; the wire numeral lives in client TS.

## 12. Risks
- Item 9.1: card-flight VFX for non-holder-owned cards into/out of the shared piles still aims at a non-existent owner pile.
- `ZoneViewer` Delve is now gated only by the engine's `legalActionsByObject`; the GraveyardPile glow was already engine-gated.
- #9377 id-tracking note (plan 4.2): the shared library's id list reaches the non-holder viewer exactly as an opponent's library already did; no new id class, not fixed here.
