MODEL: claude-sonnet-5-5
Review Head: e30f20af90a4a5b29eb8ae20a260503c9d3d3df8 (PHASE_BASE bbce5c6bdab3c24a727d5d5f776cda093c46b15a; range = fb7fd960e6 + e30f20af90; delta == scope.nul, 25/25 paths, `comm -3` = 0)
Mode: phase mode, Phase 16 (deferral allowlist: none). Skill read at skills/review-scope-charter-addenda tip c51cf335d0.

## Verdict
No blocking findings. RE-CHARTER: not needed; every charter decision stands (engine publishes holder; client only selects; shared pile rendered once; ExilePile per seat; scope additions covered by addenda/phase-16).

Counts: HIGH 0, MED 0, LOW 2 (1 behavior, 1 text), blocking 0.

## Findings

**[LOW]** `shared_piles()` restates the zone-to-holder mapping that `GameState::shared_zone_holder` already owns. Evidence: `crates/engine/src/game/derived_views.rs` fn `shared_piles` (match on `ZoneScope` -> `canonical_seat()`) vs `crates/engine/src/types/game_state.rs` fn `shared_zone_holder` (same match, private). Why it matters: a third shared zone would need both edited; E1 pins agreement only for the library (graveyard agreement is not asserted). Suggested fix: make `shared_zone_holder` `pub(crate)` and build the view from `state.shared_zone_holder(Zone::Library)` / `(Zone::Graveyard)`, dropping the local closure and the `ZoneScope` import. [behavior] (plan.md 3.x chose the restated form and plan review r1 accepted it, so this rides along and does not block.)

**[LOW]** Doc comments on the touched selectors now overstate per-owner piles. Evidence: `client/src/components/animation/cardVfx/cardAnchors.ts` above `ZONE_SURFACES`: "hidden library card shows as its owner's pile"; `client/src/components/animation/cardVfx/cardFlightSpecs.ts` `CardFlightRoute` doc: "`ownerId` locates per-player surfaces (hand, library, graveyard)". Why it matters: under a shared format the node is the holder's pile. Suggested fix: [text] cardAnchors.ts old "hidden library card shows as its owner's pile." -> "hidden library card shows as its pile (the holder's, in a shared-zone format)."; cardFlightSpecs.ts old "(hand, library, graveyard)" -> "(hand, exile; library and graveyard unless the format shares them)". Optional: delete rather than reword if the project's comment rule prefers.

## Lens results (what was measured)

1. Display-layer purity: PASS. `getSharedPileHolder` = `derived?.shared_piles?.[zone] ?? null` (`??`, so holder seat 0 is not lost); `resolvePileSeat` = `holder ?? seat`; `getZoneViewerPile` selects a group key. No format name, seat count, or rule in the client; GamePage guard is `getSharedPileHolder(...) == null`. All `derived` consumers already depend on the adapters attaching `derived` (wasm/ws/p2p paths), unchanged here.

2. Client routing completeness (class: any reader of `players[x].library|graveyard` or seat-keyed pile DOM/anchor/group key).
 Instrument: `git grep -n -E 'data-(library|graveyard)-pile|\.graveyard\b|\.library\b|getPlayerZoneIds|zone:\$\{' -- client/src ':!*__tests__*' ':!*.test.*' ':!client/src/wasm' ':!client/src/i18n'` (positive control: returned 40+ lines including the candidate's own `pileSeat`/`resolvePileSeat` sites). Widened with `data-[a-z-]*pile`, `<(Library|Graveyard)Pile`, `viewingZone|handleViewZone|setViewingZone`, `players[...]`-reads, `"(Graveyard|Library)"` string keys. Population walked: all non-test client/src files (wasm typings, i18n, generated excluded).
 Classification:
 - routed: GraveyardPile (4 reads + `data-graveyard-pile`), LibraryPile (count, top, `data-library-pile`, `isMyLibrary`), `getPlayerZoneIds` (graveyard, library; feeds ZoneViewer), `useCastableZoneObjects` (graveyard), AnimationOverlay (2 mill anchors + CardsRevealed anchor), cardAnchors (ZONE_SURFACES.Library, ZONE_SURFACES.Graveyard, PROVISIONAL_SURFACES.Graveyard), GamePage (opponent widget guards, `getZoneViewerPile` auto-open group, local piles via components), ZoneViewer (seat gate deleted).
 - exile (kept per seat by charter): ExilePile, `data-exile-pile`, `ZONE_SURFACES.Exile`, `getZoneViewerPile` Exile arm, `useCastableZoneObjects("exile")`.
 - unaffected: RevealOverlay and BattlefieldBackground (all-players union loops, verify-only per charter), OpponentSeatPane (Dandan `max_players: 2`, so `isSplitBoardActive` unreachable), CardVfxGallery/galleryScenarios (synthetic state), DebugLibraryViewer (reads `derived.debug_library_cards`, engine via `library_of`), eventNormalizer/cardFlightSpecs (no DOM key).
 Constructed extra site the class should catch: a seat-keyed `[data-graveyard-pile]` selector built outside the three censused files (e.g. a new overlay using `ownerId`); the widened instruments above match every `data-*-pile` attribute string and returned no site outside the classified list. Could not construct an unrouted one.

3. Engine descriptor: PASS. `shared_piles` populated only when `format.shared_zones()` has a shared scope (exhaustive `ZoneScope` match, no wildcard); `skip_serializing_if` on both fields and on `DerivedViews.shared_piles`; holder = `state.canonical_seat()` (lowest `PlayerId`, the same authority `zone_storage_seat` reads), not `seat_order`. Hidden-information path: `derive_filtered_views` -> `derive_views(filtered_state)`; the field reads only format+seat set, so it is viewer-independent; E3 drives `ClientGameStateRef::wrap` and `wrap_filtered` for viewers P0/P1/None. CR: only `CR 400.1` added (doc on `SharedPilesView`); `grep -nE '^400\.1[^0-9]' docs/MagicCompRules.txt` -> "Each player has their own library, hand, and graveyard", subject matches ("as modified by the format's shared-zone axis"). 401.1/404.1 not cited, none needed.

4. Protocol: PASS. `node scripts/check-protocol-version.mjs` rc 0 at candidate. Live control: with `PROTOCOL_VERSION` temporarily 113 in ws-adapter.ts the script printed "Protocol version mismatch: Rust=114, client=113"; restored. Constants: `UPSTREAM(71)+43 = 114`, `PHASE_TWO_BASE(54)+42 = 96`, lobby 16/ack 5/scoring 6/draft 30/directory 1 unchanged. Moved in lockstep: lobby-broker `PROTOCOL_VERSION` + pin test (`MIN_SUPPORTED_PROTOCOL = PROTOCOL_VERSION.saturating_sub(1)` so 113 is derived) + doc entry, server-core pin test renamed `protocol_version_is_114_for_shared_piles_view` (script requires numeral in the name; `git grep 'is_11[0-9]_for'` shows only the 114 name), ws-adapter, network/protocol.ts `WIRE_PROTOCOL_VERSION` 96 + entry, protocol.test.ts, p2p-adapter-multiplayer.test.ts (title + literals 95/96). Residual `\b113\b` / `v95` hits are history comments or unrelated literals. Bump justified: new `DerivedViews` key the v113 client does not read and whose absence shows empty per-seat piles.

5. Tests discriminating and non-vacuous. Baseline: 8 touched client test files green (155/155). Single-line reverts run in place on wt-dandan, each restored with `git checkout -- <file>` + `touch`:
 - A `resolvePileSeat` returns `seat`: 12 tests red across 7 files (gameStateView x3, PileAnchors, LibraryPile x2, useCastableZoneObjects, AnimationOverlay x2, GamePage x2, cardAnchors); ZoneViewer row correctly stays green.
 - B ZoneViewer `playerId === viewerId` clause restored: only "offers the engine's delve action to the seat that is not the holder" red (so the Delve non-holder row is red at base behavior).
 - C GamePage opponent-widget guards forced `true`: both "renders ... once, for viewer seat 0/1" red; the non-shared sibling stays green and asserts both seats' anchors (reach-guard).
 - D `isMyLibrary = playerId === myId`: only "lets the non-holder seat play from the shared top" red; the non-shared refusal sibling is the reach-guard.
 - E `getZoneViewerPile` owner-keyed: gameStateView grouping row and GamePage auto-open row red; non-shared siblings green (prompt-rendered reach-guard).
 - F cardAnchors `pileSeat` returns `ownerId`: V16-r2 row red; per-player sibling green.
 - Engine: `shared_piles` forced to `None` in `derive_views` (nextest, warm target-dandan): 4 of 5 new rows FAIL (holder-names, absent-for-per-player via its `is_some` reach-guard leg, survives-every-viewer, lowest-seat); `debug_library_projection_reads_the_shared_pile_for_the_non_holder_seat` PASS (green guard; Phase 8 landed the `library_of` read, as the executor stated, not claimed red-at-base).
 Probe records: porcelain empty before and after every probe; HEAD == e30f20af90a4a5b29eb8ae20a260503c9d3d3df8 at the end; engine mutation restored and `touch`ed (next cargo build recompiles derived_views.rs).
 Not run: E2 under an unconditional descriptor (reasoned only: `None` asserts on standard/commander would fail).

6. Locale parity: PASS. No i18n/locale file in the delta; added-line grep for `t("`, `aria-label`, `title=`, `placeholder` over non-test client code matches only the three `data-*-pile` selector template strings.

7. Charter: stands. Scope rule paths all present; BattlefieldBackground and RevealOverlay verify-only and unedited; additions (ZoneViewer, test files, cardAnchors + test) are in addenda/phase-16. The stop-and-return was closed by addendum, correctly sized as a same-unit routing site.

## Pre-existing / environment (untagged, non-blocking)
- Orchestrator's full vitest run (`fe-p16-vitest.log`): 3 failed / 600 passed files. All three are 5-8 s timeouts under full-suite load in files the change does not touch (GameLogPanel.test.tsx, turnCredentials.runtime.test.ts, DraftPodPage.lobbyListing.test.tsx); re-run in isolation at the candidate: 3/3 files, 47/47 tests pass.
- Local `git status` at session start showed unrelated modified `client/src/wasm/engine_wasm.d.ts` and `client/src-tauri/gen/schemas/linux-schema.json` in the main checkout (not wt-dandan, not in the delta).
