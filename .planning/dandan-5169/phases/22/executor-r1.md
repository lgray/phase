# Phase 22 executor r1 — S9b host-supplied Dandan pile, client flows

Mode: implementation/fix (phase mode). BASE_SHA = START_SHA = 91c34cdc967376c7800904b3cce6f8f58e00162a. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Start: clean, HEAD == START_SHA, nothing staged. End: HEAD == START_SHA, nothing staged, delta = 28 paths, all inside scope.nul (`comm -23 changed scope` empty). Not committed. Everything below is PREPARATORY, not completion evidence.

## Diff
- `services/engineRuntime.ts`: `DeckSupply` type + `deckSupplyForFormat` wrapper (ceiling-wrapper shape).
- `services/pileSource.ts` (new): `PileSource`, `PileChoice`/`DEFAULT_PILE_CHOICE`, `pileSourceFromParam`/`pileSourceParam`, `emptySeatDeck`, `suppliedAiDeckChoice`, `pileSeatDeck` (engine asked only for a SavedDeck; HostPile → saved expansion or null; else empty).
- `components/menu/PileSourceChoice.tsx` (new): component-local `useDeckSupply`; renders only for `HostPile`; native `<select>` Default + `listSavedDeckNames`; named deck → `evaluateDeckCompatibility`, shows `selected_format_reasons`, reports `legal` false while pending/incompatible.
- `menu.json` ×8: `pileSource.{label,default}` (translated).
- `GameSetupPage.tsx`: picker, Start blocked on `!pile.legal`, `&pile=` on the URL, pile reset on any format change (adjust-state-on-change, so A→B→A also resets).
- `GamePage.tsx`: `pile` URL param → `GameProvider` prop (rematch copies searchParams).
- `GameProvider.tsx`: `pile` prop (effect dep); local-AI supply branch takes the pile seat from `pileSeatDeck` (null → throws `multiplayer:page.couldNotLoadDeck`, surfaced by every caller's existing `onNoDeck(message)`); P2P: supplied format never reads the active deck, fresh host submits `pileSeatDeck`, guest submits empty, the no-deck gate is host-only; online join with a supplied URL format submits empty; every empty literal → `emptySeatDeck()` (`rg -c 'main_deck: \[\]' GameProvider.tsx` prints nothing).
- `MultiplayerPage.tsx`: `isSuppliedFormat`; `hostDeck` (one `pileSeatDeck` call per host-deck site); `executeAction` skips active-deck require/validate for supplied formats, validates a named pile via `loadSavedDeck` + `evaluateDeckCompatibility` (refusal toasts, no deck-select); null host deck toasts at both sites; all-AI navigate carries `pile`; join URLs carry `format` iff supplied (`navigateDirectP2PJoin` shared by `joinP2PRoom`, direct code, raw-code shortcut); lookup shortcut; raw-code deckless dial; supplied re-entry re-dials; banner/chip/warning hidden and live check idle for a supplied store format.
- `HostSetup.tsx`: AI default deck = `suppliedAiDeckChoice().choice` for a supplied format; picker; submit blocked on `!pile.legal`; `settings.pile`.
- `HostControlTile.tsx`: one cut at `deckChoices` (supplied → `[suppliedAiDeckChoice()]`), deck dropdown hidden.
- `multiplayerStore.ts`: `HostingSettings.pile?: PileSource` (type only).
- `crates/engine/src/types/custom_format.rs`: `from_lobby_config` doc sentence now names Momir and Dandân (addendum line 4).

Primary lines: +261/−132 in edited files + 153 new ≈ 414 added (plan ≈ 420).

## Addenda
- L1 (EngineFixed submits empty on every route): `pileSeatDeck` returns empty for non-HostPile; Momir lobby-host row and C4 EngineFixed row.
- L2 (`deck_supply` out of types.ts/formatRegistry.ts): `grep -c deck_supply` = 0 / 0 (census file control: 6). Neither file touched, so no census run.
- L3 (lib.rs/d.ts sentence "the lobby offers a pile choice only for HostPile"): made true in client code — HostSetup mounts `PileSourceChoice`, which renders only for HostPile (C5 + C3 Standard sibling). No d.ts regeneration, no engine-wasm edit.
- L4: fixed in this phase (comment only; `rustfmt --check` clean).

## Preparatory checks (final tree)
- `npx tsc -b --noEmit --force` rc=0 (`pnpm run type-check` lacks `--force`).
- `pnpm lint` rc=0, 0 errors; warnings on the four touched files identical to base (9 = 9, same rules).
- Focused vitest, 41 files / 786 tests passed: every MultiplayerPage*, GamePage*, GameSetupPage*, GameProvider.* (incl. joinOrigin, nativeEngine, p2pHostLifecycle), HostSetup*, HostControlTile*, components/menu/*, pileSource.test, engineRuntime.test, i18n resources + localeParity.
- U1 (local gate, real wasm + card-data): `npx vitest run --config vitest.integration.config.ts --coverage.enabled=false src/services/__tests__/pileSource.integration.test.ts` → 3 passed, none skipped.
- CR gate: 0 CR numbers added (instrument control on custom_format.rs: 26).

## Test placement (Sizing list correction for the orchestrator; plan.md is outside scope)
P1 `services/__tests__/pileSource.test.ts`; U1 `services/__tests__/pileSource.integration.test.ts`; C5 `components/menu/__tests__/PileSourceChoice.test.tsx`; C1 extends `pages/__tests__/GameSetupPage.test.tsx`; C2a/C2b/C2c/C2d all in new `pages/__tests__/MultiplayerPage.dandanPile.test.tsx`; C3 in new `components/lobby/__tests__/HostSetup.dandanPile.test.tsx` (own `useAiDeckCatalog`/`deckSupplyForFormat` mocks, so HostSetup.test.tsx is untouched); C3b extends `components/chrome/__tests__/HostControlTile.test.tsx`; C4 new `providers/__tests__/GameProvider.dandanPile.test.tsx`; C6 extends `pages/__tests__/GamePage.bracketViolation.test.tsx` (no GamePage.pile file). Unused scope test paths: HostControlTile.dandanPile, HostSetup.test, GamePage.pile, GameSetupPage.dandanPile, MultiplayerPage.dandanJoin.

## Rows: red at base, green at head
Base instrument: scratch copy `dandan-run/scratch/p22exec/client` with the 7 consumer files reverted to `git show HEAD:` (additive modules kept), same test files. 40 of 123 tests red at base.

| Row | Head | Base | Discriminating assertion |
|---|---|---|---|
| P1 | 5/5 | n/a (module new) | HostPile named → 80-card main; EngineFixed named → `emptySeatDeck()`; Default → zero engine calls; gone → null |
| U1 | 3/3 real wasm | n/a | export_game_state_json object-name multiset == pile, no `Dandân`; Default has `Dandân`; Momir+named loads (`error` not true) while Momir+80 list returns `{error:true, reasons}` |
| C1 | 6 rows | all red | URL lacks/has `pile=Pile%20A`; Start disabled on refusal; A→Momir→Dandan resets to Default; Momir/Standard no picker after the supply mock settles (red at base only because the settle guard waits on a supply call base never makes) |
| C2a | 8 rows | 7 red, Standard sibling green | submitted decks `[[]]`/`[PILE]`; Dandan+Momir illegal active deck → `[[],[]]`, no toast; refusal toast + no start + no deck-select; continue-without-lobby same pile; all-AI `pile=` present/absent |
| C2d | 3 legs | red | `couldNotLoadDeck` toast, no start; `pileSeatDeck` call count 1 (executeAction) / 2 (continue, sequenced mock, button present before click); absent-from-storage via the real `pileSeatDeck` |
| C2b | 7 rows | 4 red, 3 Standard/with-deck siblings green | navigate URL `…&format=Dandan` (p2p via `joinP2PRoom`, server `mode=join`), raw code `mode=p2p-join&code=ABCDE$`, re-entry re-dial no toast; siblings reach deck-select / toast |
| C2c | 3 rows | 2 red, Standard green | no "Active Deck"/chip, `hostDisabled === false`; no warning; Standard shows banner, "Not legal in Standard", `hostDisabled === true`, warning |
| C3 | 4 rows | red (Standard sibling red at base only via the same settle guard) | AI seat deck `{type:"DeckList", data: emptySeatDeck()}`, `pile:{type:"Default"}`; refusal disables Host; `pile` SavedDeck in settings; Standard + empty catalog disabled |
| C3b | 7 rows | 5 red, 2 Standard green | each consumer's `SetKind` carries the empty DeckList; 1 combobox on a Dandan AI row vs 2 for Standard+catalog |
| C4 | 13 rows | 7 red; Default, EngineFixed and PlayerBuilt siblings green | adapter constructor deck / `initGame` deckList player main = PILE / [] / ACTIVE; native `aiSeats` decks `[[]]`; vanished → `onNoDeck("Could not load deck…")`, no `initGame`; deckless p2p-join builds guest, no `onNoDeck`; PlayerBuilt p2p-host no deck → `onNoDeck` |
| C5 | 6 rows | n/a | empty DOM for EngineFixed/PlayerBuilt/pending/failed after settle; last `onChange` legal false pending then false/true by verdict; reasons rendered |
| C6 | 1 row (2 legs) | red | `pile === "Pile A"`, absent → undefined |
| L1 | green | — | dropping `de` `pileSource.default` reds both resources.test and localeParity (restored sha256-identical) |

## Mutations (script `dandan-run/scratch/p22exec/mut.py`; each applied alone in W, named rows run, file restored and sha256-verified; logs in `scratch/p22exec/mut/`)
| Mutation | Red tests |
|---|---|
| M1 SavedDeck → empty | P1 HostPile row; U1 named-pile + Momir-control rows; C4 local AI, native, fresh p2p-host pile; C2a named pile, continue |
| M2 picker ignores supply | C5 EngineFixed, PlayerBuilt, pending/failed; C1 Momir, Standard (+2 existing cEDH-chip rows) |
| M2-pending (select reports legal true) | C5 pending row |
| M3 p2p-join keeps active deck | C4 supplied P2P guest |
| M3-online | C4 supplied online guest |
| M4 skip supply check | P1 EngineFixed row; U1 Momir row |
| M5 `deckChoices` ignores supply | C3b Add, Replace, Fill, promotion (one test each) |
| M5-dropdown | C3b no-dropdown |
| M6 keep active-deck requirement | 9 C2a/C2d rows |
| M6-over (requirement dropped for all) | C2a Standard sibling only |
| M7-p2p / M7-online (empty for all formats) | C4 PlayerBuilt p2p guest + p2p host (+supplied host pile) / C4 PlayerBuilt online guest |
| gate kept for p2p-join / gate removed for p2p-host | C4 deckless guest / C4 PlayerBuilt host onNoDeck |
| M8 raw code → deck-select | C2b raw-code row |
| M9 re-entry always toasts | C2b Dandan re-entry |
| lookup shortcut removed | C2b both listed-Dandan joins |
| join `format` dropped | C2b both listed joins + re-entry |
| named-pile validation removed | C2a named pile (compat call) + refusal |
| M10a / M10b | C2d executeAction + absent legs / C2d continue leg only |
| banner / warning / live-check gates | C2c Dandan banner row / warning row / banner row (`hostDisabled`) |
| M11 | C6 |
| C1 reset / legal gate / `pile=` param | format-switch / refusal / compatible rows |
| C3 legal gate / `settings.pile` / AI deck | refusal / both submit rows / both submit rows |

## Route census accounting
Every route-table row lands in a file in the set: GameSetupPage (C1), GamePage (C6), GameProvider local AI wasm-fresh and native (C4; the resume-fail fallback calls the same `buildLocalAiDeckList` with `pileSourceFromParam(pile)`, not separately driven), p2p-host fresh (C4), p2p-join (C4), online join (C4), `handleHostSetupComplete` (C2a, M6), `executeAction` host server/P2P/continue/all-AI (C2a, C2d), join + `joinP2PRoom` + direct code + server params (C2b), lookup shortcut (C2b), raw code (C2b, M8), re-entry (C2b, M9), banner/chip/warning/live check (C2c), HostSetup (C3), HostControlTile four consumers + dropdown (C3b), multiplayerStore type field (threaded below). Verify-only rows untouched: AiOpponentConfig, p2p-adapter, brokerClient, multiplayerDraftStore, tournament/draft. `useResumables` guest resume: covered by the p2p-join gate change (a resume sends `reconnect`).

## New-field threading
- `HostingSettings.pile`: constructed in `HostSetup.handleHost` (threads); read by `MultiplayerPage` `hostDeck` (both host-deck sites), named-pile validation, all-AI navigate (threads); `multiplayerStore.startHosting`/`startP2PHostingSession` defaults intentionally because they receive the already-resolved deck; `brokerClient` has its own request type (not HostingSettings).
- `GameProvider` `pile` prop: one producer (`GamePage`, threads); consumed at the three local-AI call sites and the fresh p2p-host branch (threads); in effect deps.

## Maintainer matrix (condensed)
| Seam | Authority / bound value | Bound when | Storage → consumer | Invalidation |
|---|---|---|---|---|
| pile pick | saved-deck name + format | picker change; verdict at pick | page state `PileChoice` → URL `pile` / `settings.pile` | format change resets to Default; vanished deck → null at start → toast/onNoDeck |
| pile legality | engine `evaluateDeckCompatibility` | picker (display/gate) and again in `executeAction` | `legal` flag; not stored past start | pending = illegal |
| supply axis | engine `deckSupplyForFormat` | at pick and at seat build | not stored (asked per call) | failure → PlayerBuilt (no picker) |
| guest submission | `formatSuppliesDeck(URL format)` | route mount | adapter constructor deck | host gate (`validateGuestDeck`) decides; refusal → re-entry with `kick.format` |
No serde/protocol/wire change; no engine export change.

## Judgement calls
- Pile reset uses React's adjust-state-during-render pattern: a format-keyed derived state would resurrect a named pile on A→B→A.
- `PileChoice`/`DEFAULT_PILE_CHOICE` live in `pileSource.ts` (exporting them from the component file added a react-refresh warning).
- C3 placed in a new `HostSetup.dandanPile.test.tsx` rather than mocking `useAiDeckCatalog` across the 1676-line HostSetup.test.tsx.
- The P2P deck construction moved inside `setupP2P`'s `try` because it now awaits; an abort or wasm failure there converges on the existing catch instead of an unhandled rejection.

## Risks
- C1 Momir/Standard and C3 Standard siblings are red at base only because their settle guard waits for the new supply call; their absence assertions are trivially true at base. M2 shows they discriminate at head.
- Fresh p2p-host with a supplied format has no production URL producer (formatConfig synthetic in C4, as the plan states).
- Server-side handling of empty guest lists/AI DeckLists remains read-only evidence (plan's Unprobed).

## Stop-and-return items
None.

Scratch: the base tree is deleted; `dandan-run/scratch/p22exec` keeps the mutation script, logs and base-run log (680K), plus `scratch/p22-c*.log`. The orchestrator may delete them.
