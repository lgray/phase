# Phase 22 plan (r0) — S9b host-supplied Dandan pile, client flows (PHASE_BASE_SHA d12daf57938f9d9db5762f20ed36ff1d4c76701e)

## Shape of the code (derived from one entry-point trace per route; no architecture file states the client's start-flow shape)

Four layers own the decisions. (1) Entry UIs choose what to start: `GameSetupPage` (local AI, navigates to `/game/<id>?mode=ai&format=…`), `MultiplayerPage.executeAction` (lobby host/join; P2P host via `multiplayerStore.startP2PHostingSession(settings, deck, …)`, server host via `startHosting(settings, deck, url)`, join via `/game/<id>?mode=join|p2p-join`), `HostSetup` (the host form) and `HostControlTile` (live AI-seat edits). (2) `GamePage` parses the URL into `formatConfig`/`matchConfig` and renders `GameProvider`. (3) `GameProvider` builds the `DeckListPayload` per route (local AI wasm, native AI, P2P host/guest, online host/join) and hands it to the adapter. (4) The engine decides legality (`evaluateDeckCompatibility`) and the supply axis (`deckSupplyForFormat`, wire strings `PlayerBuilt|EngineFixed|HostPile`, Phase 21). The client is a transport/display layer: it asks the engine and dispatches. `formatSuppliesDeck` (registry flag, sync, true for Momir and Dandan) answers "this player builds no deck" and stays the guest/AI/lobby-requirement predicate; the engine's `deckSupplyForFormat` distinguishes `HostPile` and is asked only where the pile is chosen or the pile seat is built.

## Claims re-measured at the base (command or driven probe; instrument = scratch copy of `client/` under `dandan-run/scratch/p22/`, real wasm + `card-data.json`, deleted after)

1. **"One helper builds every empty-seat deck" is false at base.** `rg -n 'main_deck: \[\]' client/src` (excl. tests) lists 11 literals: `GameProvider.tsx` `buildPlayerOnlyDeckList` (opponent), `buildLocalAiDeckList` (`emptySeat` closure), online `deck` fallback (two), native `deckList?.player ??`; `multiplayerStore.ts` (P2P lobby host `opponent`), `multiplayerDraftStore.ts`, `brokerClient.ts` (two registration placeholders), `p2p-adapter.ts` (two defaults). Positive control: the same pattern hits `buildLocalAiDeckList`. Premise corrected: the plan introduces the one builder (`services/pileSource.ts`) and routes every site that constructs a seat for a `formatSuppliesDeck` format through it; the sites that are not engine seats (broker registration, draft, adapter defaults, the lobby host's own `opponent` placeholder, which is empty for every format) stay, with verdicts in the route table.
2. **The legality verdict already exists and is the pile's gate.** Driven through the real `evaluateDeckCompatibility` over the real engine (scratch `verdict.integration.test.ts`): an 80-card Jund list for Dandan → `selected_format_compatible=true`, no reasons; 60 Forest → `false`, "Dandân deck must have exactly 80 cards (found 60)"; the empty deck → `false`, "…(found 0)" (so the Default pile must bypass the hint, never submit through it); the same 80 for Momir → `false` with the 60-card/snow-basic reasons. Consequence: the picker displays this verdict for a named deck; the client counts nothing.
3. **Engine accepts the pile end to end (U1 mechanism).** Driven with the real wasm (scratch `pileSource.integration.test.ts` over the prototype `pileSeatDeck`): Dandan with a named 80-card pile on the pile seat and `emptySeatDeck()` on the other → `initialize_game` ok, 80 library objects, the default list's `Dandân` absent; Dandan all-empty → ok, `Dandân` present (default pile). Momir all-empty → ok (122 objects); Momir with an 80-card list on the player → refused with the "exactly 60 cards"/"snow basics only" reasons, i.e. EngineFixed must submit empty (binding addendum line 1). The prototype's Momir branch (`deckSupplyForFormat` ≠ `HostPile` → empty) returned an empty list for a named source.
4. **Existing GameProvider suites cannot reach the new code.** `joinOrigin`, `nativeEngine`, `p2pHostLifecycle` mock `formatSuppliesDeck: () => false` (`p2pHostLifecycle` replaces `data/formatRegistry` wholesale with only that export), `services/deckParser` with only `expandParsedDeck`, and `constants/storage` partially (spread of the original). `pileSource.ts` imports only `expandParsedDeck`, `loadSavedDeck` (real, via the spread), `deckSupplyForFormat`, and no `formatRegistry` name, and calls the engine only for a named pile on a supplied format, so those suites stay green unchanged; the new suite sets `formatSuppliesDeck: () => true` itself.
5. **Server and P2P transports ignore non-pile seats (read, not driven).** P2P host adapter reads only `hostDeckData.player`; `validateGuestDeck` (`p2p-adapter.ts`) gates a guest through `evaluateDeckFormatGate` (Phase 21: empty accepted for supplying formats); server join/seat mutation only name-resolves decks. Rematch (`GamePage` `handleRematch`) copies the URL searchParams, so a `pile` param survives it.
6. **No tournament/draft route reaches a supplied format.** `grep -c format client/src/pages/TournamentLandingPage.tsx` = 0 (control: `GameSetupPage.tsx` > 0); `formatSuppliesDeck` has exactly three readers at base (`rg -n formatSuppliesDeck client/src` non-test: `GameSetupPage`, `GameProvider`, `AiOpponentConfig`).

## Route census (generated by `rg -n 'loadActiveDeck|formatSuppliesDeck|buildLocalAiDeckList|buildPlayerOnlyDeckList|startHosting|startP2PHostingSession|expandDeck' client/src` non-test) and decisions

| Route | Base behaviour | Decision |
|---|---|---|
| `GameSetupPage` local AI | Supplied formats bypass deck gates; submit empty everywhere (default pile only) | Shows `PileSourceChoice` for HostPile; a named pile rides the URL as `pile=<saved deck name>`; Start blocked while the picked pile's engine verdict is incompatible/pending; Default needs no deck |
| `GamePage` | Parses `format/players/match`; no pile | Reads `pile`, passes `pile` (string) to `GameProvider`; primitive prop keeps the provider's effect deps stable; rematch carries it |
| `GameProvider` local AI (resume-fail fallback, fresh WASM, native) | `buildLocalAiDeckList` supply branch → empty everywhere | Supply branch: player = `pileSeatDeck(format, source)` (null → `onNoDeck` with the missing-deck message), other seats `emptySeatDeck()`; native passes `deckList.player` unchanged (already) |
| `GameProvider` p2p-host fresh branch | `buildPlayerOnlyDeckList(active ?? EMPTY)` submits the active deck | Player = `pileSeatDeck(…)` for supplied formats; never the persisted active deck (binding line 1) |
| `GameProvider` p2p-join | Guest submits its active deck; host gate kicks an illegal one | Supplied format: guest submits `emptySeatDeck()` |
| `GameProvider` online join/host (`new WebSocketAdapter`) | Active deck or empty | Supplied format (format known from `formatConfig`): join submits `emptySeatDeck()`; host reconnect path unchanged |
| `MultiplayerPage.executeAction` host | Requires active deck + validates it for every format | Supplied: no active-deck requirement; deck = `pileSeatDeck(format, settings.pile)` (server and P2P and "continue without lobby"); named pile verdict re-checked via `evaluateDeckCompatibility`, refused with the engine reason; the `allOpponentsAreAi` branch navigates `mode=ai` carrying `pile` |
| `MultiplayerPage.executeAction` join, `handleJoinGame` deck-select | Always deck-select, validates | Supplied: skip deck-select and validation; join URL carries `format` so `GameProvider` knows to submit empty; the live-check banner/chip stays for PlayerBuilt |
| `HostSetup` | AI-seat default deck required (`submitDisabled`) and attached to every AI seat | Supplied: requirement bypassed, AI seats carry `{type:"DeckList", data: emptySeatDeck()}`; renders `PileSourceChoice` for HostPile and puts `pile` on `HostingSettings`; blocked while the pile verdict is not legal |
| `HostControlTile` Add/Replace AI | Disabled when the AI catalog has no choices | Supplied: enabled, seat deck empty |
| `multiplayerStore` (P2P/server host, AI seat mutation) | Takes `deck` param; `opponent` literal empty | Unchanged except the `HostingSettings.pile` type field; caller passes the pile deck |
| `AiOpponentConfig` | Already gates on `formatSuppliesDeck` | Verify-only (no change) |
| `p2p-adapter.ts`, `brokerClient.ts`, `multiplayerDraftStore.ts` | Defaults/placeholders | Verify-only |
| Tournament/draft | Unreachable (claim 6) | None |

## Decision

1. `engineRuntime.deckSupplyForFormat(format): Promise<DeckSupply>` beside `bestOfThreeCeilingForFormat` (same `ensureWasmInit` + `loadEngineModule` shape). The type `DeckSupply` lives there; the substring `deck_supply` is added to neither `adapter/types.ts` nor `data/formatRegistry.ts` (binding line 2; the Rust census file stays untouched).
2. New `services/pileSource.ts`: `PileSource = {type:"Default"} | {type:"SavedDeck"; name}`, `emptySeatDeck()`, `pileSeatDeck(format, source)`: `SavedDeck` asks the engine and returns the expanded saved deck only for `HostPile`, else (Default, EngineFixed) the empty deck without any engine call; a vanished saved deck returns `null`. It is the single builder named by the charter; every route above that submits for a supplied format goes through it.
3. New `components/menu/PileSourceChoice.tsx`: asks `deckSupplyForFormat` (hook local to the component, `useBestOfThreeCeiling` pattern: null until it resolves for exactly this format, failure answers `PlayerBuilt`), renders nothing unless `HostPile`, offers Default / saved deck (`listSavedDeckNames`), and for a named deck displays the engine's `selected_format_reasons` from `evaluateDeckCompatibility`. It reports `{source, legal}` upward (`legal` false while a named verdict is pending or incompatible). The hook is not exported from `HostSetup`; `GameSetupPage`, `HostSetup` and `MultiplayerPage` need only the sync `formatSuppliesDeck` for the requirement bypass.
4. Carriers: setup page and `allOpponentsAreAi` navigation put `pile=<name>` on the game URL (not persisted across sessions: dropped by charter); lobby hosts pass `settings.pile`.
5. Docs: the `deck_supply_for_format` doc sentence ("the lobby offers a pile choice only for `HostPile`") is true at this head (picker renders only for `HostPile`; row C5 guards it), so no d.ts regeneration. The `from_lobby_config` doc in `crates/engine/src/types/custom_format.rs` is corrected in this phase (one comment sentence: the built-ins that set `supplies_fixed_deck` are Momir and Dandân, both rejected below), binding line 4.

## Pattern Coverage

Every format whose engine supply is `HostPile` (Dandan today; any future custom pile format) gets the picker and pile seat; `EngineFixed` formats (Momir) and `PlayerBuilt` formats are unchanged except that supplied formats' guests/AI seats submit empty. No card text and no format literal in client code.

## Sizing

One unit: the client display of the engine's pile answer across the start flows. T1 (≥2 units) fails. Non-test scope paths: `engineRuntime.ts`, `pileSource.ts`, `PileSourceChoice.tsx`, `GameSetupPage.tsx`, `GamePage.tsx`, `GameProvider.tsx`, `MultiplayerPage.tsx`, `HostSetup.tsx`, `HostControlTile.tsx`, `multiplayerStore.ts` (type field), the `menu` locale group (en + 7 mirrors, one T2 path per namespace), `custom_format.rs` (comment) = 12 (charter 9: `AiOpponentConfig` dropped by claim 1/route table; `pileSource.ts`, `GamePage.tsx`, `HostControlTile.tsx`, `multiplayerStore.ts`, `custom_format.rs` added by the route census). T2 (≥13) does not fire at 12 either; the conjunction cannot fire. Test paths counted separately: `pileSource.test.ts`, `pileSource.integration.test.ts`, `PileSourceChoice.test.tsx`, extensions to `GameSetupPage.test.tsx` and `HostSetup.test.tsx`, `HostControlTile.test.tsx`, new `MultiplayerPage.dandanPile.test.tsx`, new `GameProvider.dandanPile.test.tsx`, `resources.test.ts` (existing, unchanged). Estimated primary lines ~340 added (pileSource ~45, PileSourceChoice ~110, GameProvider ~45, MultiplayerPage ~45, GameSetupPage ~30, HostSetup ~30, rest ~35) against the charter's ~390.

## Building Blocks

`bestOfThreeCeilingForFormat` + `useBestOfThreeCeiling` (engine answer → hook); `evaluateDeckCompatibility` (verdict); `loadSavedDeck`, `listSavedDeckNames`, `expandParsedDeck`; `formatSuppliesDeck`; `DeckChoice` `DeckList` for AI seats. Discoverability: `rg -n 'deckSupplyForFormat' client/src` (wasm d.ts and the Phase 21 wrapper-free export only at base; control `bestOfThreeCeilingForFormat` hits `engineRuntime.ts`).

## Logic Placement, Rust Idioms, Extension vs Creation, Nom Compliance

All legality and supply decisions stay in the engine (verdict and supply answer are displayed/dispatched, never recomputed). No Rust logic changes; the only Rust edit is a doc comment. No parser code (nom N/A). Typed `PileSource` union instead of a boolean/stringly flag; URL param is a name only because a URL is the carrier that survives rematch/refresh.

## Analogous Trace

`bestOfThreeCeilingForFormat`: `engine.rs` export → `engineRuntime.ts` wrapper → `useBestOfThreeCeiling` in `HostSetup`/`GameSetupPage` → capped `match_type` on the URL. The pile choice follows the same shape (export → wrapper → component-local hook → `pile` on URL/settings) with the consumer being `pileSeatDeck`.

## Variant Discoverability

No engine variant added. `rg -n 'DeckSupply' crates/engine/src/types/format.rs` is the contract; client mirrors the three wire strings in one type in `engineRuntime.ts` only.

## Steps

1. `engineRuntime.ts`: `DeckSupply` + `deckSupplyForFormat`.
2. `pileSource.ts` (+ `pileSource.test.ts`, `pileSource.integration.test.ts`).
3. `PileSourceChoice.tsx` + `menu` locale keys in en and the seven mirrors (same key paths; non-English strings translated, not copied).
4. `GameSetupPage` (picker, start gate, `pile` on URL), `GamePage` (parse and pass `pile`), `GameProvider` (route builders per table, `pile` prop added to the effect deps; `buildPlayerOnlyDeckList` takes the player seat deck).
5. `HostSetup`, `HostControlTile`, `multiplayerStore` (type field), `MultiplayerPage` (host/join/continue-without-lobby/all-AI branches, join URL `format`).
6. `custom_format.rs` comment sentence; confirm the lib.rs/d.ts sentence by row C5.
7. Targeted vitest per touched suite, then `cargo fmt --all` (comment-only Rust edit), `tsc -b --noEmit --force`, eslint on touched files, locale parity test; Tilt `check-frontend`/`test-frontend` as read-only signal where Tilt watches this checkout.

## Verification Matrix

Every negative row names its paired positive reach guard; red-at-base rows are exact consequences of the base behaviour in the route table (the picker and `pileSource` do not exist at base, so those rows fail to resolve/assert).

| Row | Seam / entry | Test (file) | Revert-failing assertion | Reach guard / siblings |
|---|---|---|---|---|
| P1 | `pileSeatDeck` | `services/__tests__/pileSource.test.ts` (engine mocked) | `HostPile`+SavedDeck returns the expanded saved main; Default returns empty with zero engine calls; `EngineFixed`+SavedDeck returns empty; missing deck → null | HostPile named row vs EngineFixed named row are each other's control |
| U1 | pile reaches the engine | `services/__tests__/pileSource.integration.test.ts` (real wasm, `describe.skipIf` inputs absent, header gives the `pnpm exec vitest run --config vitest.integration.config.ts` command) | Library multiset after `initialize_game` equals the named pile and lacks `Dandân`; Momir with the same named source loads (empty submission) | Default source → `Dandân` present (reach); Momir 80-list on the player is refused (hostile: proves empty is load-bearing) |
| C1 | `GameSetupPage` | `GameSetupPage.test.tsx` | HostPile: picker shown; Default starts with no saved deck and URL has no `pile`; named incompatible verdict disables Start; named compatible navigates with `pile=<name>` | PlayerBuilt and EngineFixed formats: no picker (`PileSourceChoice` mocked as a props recorder, engine supply mocked per format) |
| C2 | `MultiplayerPage` host/join | new `MultiplayerPage.dandanPile.test.tsx` | Default host proceeds with no active deck and passes the empty deck to `startHosting`/`startP2PHostingSession`; named incompatible refused with engine reason; join of a supplied format skips deck-select and navigates with `format`; all-AI host navigates with `pile` | PlayerBuilt format with no active deck still toasts "select a deck first" |
| C3 | `HostSetup` | `HostSetup.test.tsx` | Supplied format with AI seats and an empty AI catalog: submit enabled, every AI seat deck is the empty `DeckList`; incompatible named pile disables submit; settings carry `pile` | PlayerBuilt format with empty catalog stays disabled |
| C3b | `HostControlTile` | `HostControlTile.test.tsx` | Supplied format: Add AI enabled with no catalog choices | PlayerBuilt: disabled |
| C4 | `GameProvider` routes | new `GameProvider.dandanPile.test.tsx` (formatSuppliesDeck true, `pileSource` real, engine supply mocked) | One assertion per route: local AI wasm and native carry the named pile on the player seat and empty elsewhere; fresh p2p-host carries the pile and not the active deck; p2p-join and online join submit empty; EngineFixed named → empty player | Same routes with `formatSuppliesDeck` false keep the active deck (existing suites unchanged and green) |
| C5 | `PileSourceChoice` | `PileSourceChoice.test.tsx` | Renders nothing for PlayerBuilt/EngineFixed and while the answer is unresolved or fails; renders for HostPile; named verdict reasons shown | HostPile vs the two siblings |
| L1 | i18n | existing `src/i18n/resources.test.ts` | Parity across 8 locale dirs for the new `menu` keys | Plant check: a key removed from one mirror reds the test |

Mutations (each must red its row; run once, restore): M1 `pileSeatDeck` returns empty for SavedDeck → U1, P1, C4 red; M2 `PileSourceChoice` ignores the supply answer (always renders) → C5, C1 siblings red; M3 p2p-join keeps the active deck → C4 red; M4 `pileSeatDeck` skips the supply check (returns the named deck for EngineFixed) → P1, U1 Momir row red.

Closing measurement (non-gating): a browser run against the built wasm through the existing `run` skill starting one default and one custom-pile Dandan game against the AI; U1 is the gating form of the Phase 21 deferred row. U1 is not added to `ci.yml` (explicit file list there; a workflow edit is outside this phase) so it is a local executor gate; the unit rows P1/C4 are what CI runs.

## Reference Readings

Rows expecting another reading: C1/C2 negative rows use the engine verdict strings measured in claim 2 only as fixtures of the mocked `evaluateDeckCompatibility` answer; U1's expected library is the submitted pile's own multiset (independent of the loader), and the default row's reference is the printed default list already in `dandan_shared_pile_storage.rs`. No card ability is a reference.

## Identity / Provenance Contract

The pile seat is the engine's `canonical_seat()` (seat 0, Phase 21). The client's `pile` is a saved-deck name resolved at game start through `loadSavedDeck`; the picked deck's legality verdict is keyed to that name and format at picker time and re-checked in `executeAction`. Hostile fixture: C4's EngineFixed + named source, and C2's refusal of an incompatible named pile.

## Findings for the driver (not plan content)

- Charter premises corrected: no single empty-seat helper exists at base (claim 1); lobby and P2P guests submit the active deck today, not empty (route table); `AiOpponentConfig` needs no change; no `multiplayer.json`/`game.json` keys are needed (only `menu`); the hook cannot live in `HostSetup` (the new menu component would import from lobby), so it is component-local.
- Scope additions over the charter's list: `pileSource.ts`, `GamePage.tsx` (URL carrier), `HostControlTile.tsx`, `multiplayerStore.ts` (one type field), `custom_format.rs` (binding line 4). Charter T1/T2 count of 9 becomes 12; one addendum line on Phase 22's scope rule is the candidate (decision touched: Scope rule; drift: path additions; no design break).
- Behaviour changes for the PR body: supplied-format guests and AI seats submit empty on every transport; a non-legal active deck no longer reaches a Momir/Dandan seat.

## Unprobed (labelled)

HostSetup C3 and every UI row were read, not driven, before the executor's red run; server and native-route handling of an empty guest list and seat-0 pile (claim 5) read, not driven; rematch carrying `pile` read from `handleRematch`; the `from_lobby_config` rejection of Dandân (read at its `has_unrepresentable_auxiliary_deck_component` comment); GamePage passing `format` to a `mode=join` route leaves `formatConfig` otherwise unused by the online effect (read).
