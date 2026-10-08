# Phase 22 plan review r1 (phase-plan mode, phase-fit context: Sizing blocking)

Verdict: NOT clean. 4 blocking findings, all `behavior`. Sizing is present and consistent (1 unit, 12 non-test paths; none of the repairs below adds a path). The plan's own claims 1, 4, 5 and 6 check out. The `deck_supply` boundary and addenda lines 2–4 are honoured. Line 1 is honoured in the design but missing one test row (F3).

## Findings

### F1 `behavior`: the route census predicate misses active-deck gates and join routes
The census command (`rg -n 'loadActiveDeck|formatSuppliesDeck|…'`) does not match `activeDeckName`, `ACTIVE_DECK_KEY`, `setView("deck-select")` or the join navigations. Census to run instead: `rg -n 'setView\("deck-select"\)|mode=p2p-join|mode: "join"|if \(activeDeckName\)|activeDeckName\b.*&&' client/src/pages/MultiplayerPage.tsx`, plus `rg -l 'ACTIVE_DECK_KEY|activeDeckName|loadActiveDeck\(' client/src` (non-test). The second command returns GameProvider, GameSetupPage, MultiplayerPage, MyDecks, HomeDashboard (display only), feedService, backup and storage. Each MultiplayerPage site, with what the plan says about it:
- `handleHostSetupComplete` (`if (activeDeckName) … else setView("deck-select")`): **undecided.** Every HostSetup submit, including the Discord bot-link host, passes through it before `executeAction`. A host with no active deck is sent to deck-select, so C2's "Default host proceeds with no active deck" cannot pass unless this changes. Decide: skip the deck requirement for supplied formats.
- The `location.state.deckRejected` re-entry effect (`setPendingAction({type:"join", format}) ; setView("deck-select")`): **undecided.** For a supplied format it should run the join without deck-select.
- `handleJoinGame` direct 5-character P2P code branch (`!format && !context && directP2PCode` → deck-select): **undecided.** The format is unknown before dialling. A guest with no saved deck cannot join a Dandan room. A guest whose active deck is illegal is kicked once by the host gate (`validateGuestDeck` → `kick … format`) and only then routed through the re-entry above. The charter goal ("joining guests … need no deck") is false for this route. Decide it, for example: with no active deck, dial with an empty submission and let the host gate's `deckRejected{format}` answer a PlayerBuilt room; then route a supplied-format re-entry straight to the join. Add a C2 row.
- `joinP2PRoom` (`navigate(…mode=p2p-join&code=…)`, no format parameter): the plan's generic "join URL carries `format`" has to thread the format into this function. This is the P2P lobby join, the route whose guest gate kicks an illegal deck. A server `mode=join` guest is never format-validated (`resolve_deck` only; `resolve_deck_empty_deck_is_ok` in `server-core/src/deck_resolve.rs`). C2 must assert the P2P lobby join, not only "join … navigates with `format`".
- The direct-code navigate (`mode=p2p-join&code=${p2pCode}`) must also carry the format when it is known (the re-entry case).
- The host-setup active-deck chip / Change, `DeckLegalityChip`, and the `noDeckWarning` "Pick deck" prompt: the plan decides only the live-check chip ("stays for PlayerBuilt"). Decide the chip and the warning for supplied formats. For Dandan the warning asks for a deck the flow no longer needs.
Required revision: re-run the census by the command above, add every site to the route table with a decision, and add C2 rows for `handleHostSetupComplete`, the P2P lobby join and the direct-code or re-entry join.

### F2 `behavior`: HostControlTile has four catalog-gated consumers, and the plan decides two
Command: `rg -n 'deckChoices|pickRandomAiDeck|haveAnyDeck' client/src/components/chrome/HostControlTile.tsx`. Consumers:
- Add AI on a WaitingHuman row (`disabled={deckChoices.length === 0}`)
- Replace with AI on a JoinedHuman row (same predicate)
- The promote-to-DeckList effect (`if (!canEditSeats || !haveAnyDeck) return`)
- **Fill with AI** (`fillWithAiAndStart`, `disabled={!haveAnyDeck}`)

The plan names Add/Replace and C3b asserts Add only. As written, Fill with AI stays disabled for Dandan and Momir lobbies. Required revision: make the cut at `deckChoices`/`pickRandomAiDeck`. For a supplied format, return one empty `DeckList` choice so all four consumers follow it. C3b must also assert that Fill with AI is enabled and that its `SetKind` carries the empty `DeckList`, with a PlayerBuilt sibling that stays disabled. Red at base by construction: the predicate is a literal over an empty catalog.

### F3 `behavior`: C4's PlayerBuilt siblings depend on suites that assert no deck
C4 says the routes with `formatSuppliesDeck` false "keep the active deck (existing suites unchanged and green)". The suites do not check that:
- `GameProvider.p2pHostLifecycle.test.tsx` mocks `P2PGuestAdapter: class {}` and never inspects the `P2PHostAdapter` deck argument.
- `GameProvider.joinOrigin.test.tsx` never asserts the `WebSocketAdapter` deck argument (`rg -n 'main_deck|mock.calls' …` finds only mock factories).

A mutation that makes p2p-join, online join or the p2p-host fresh branch submit empty for every format stays green. Required revision:
- In `GameProvider.dandanPile.test.tsx`, add per-route PlayerBuilt rows: with `formatSuppliesDeck` false, the active deck's main reaches the adapter on p2p-join, online join and p2p-host fresh.
- Addendum line 1 also names the MultiplayerPage server and P2P host, but no row covers EngineFixed there. Add a C2 Momir host row: an illegal active deck is present, `startHosting` / `startP2PHostingSession` receive the empty deck, and no deck-select or toast appears. This is red at base, where `executeAction` validates the active deck for Momir.

### F4 `behavior`: C1's sibling rows and M2's C1 target cannot fail
Decision 3 puts the HostPile gate inside `PileSourceChoice`, which "renders nothing unless `HostPile`"; GameSetupPage reads only the sync `formatSuppliesDeck`. C1 mocks `PileSourceChoice` as a props recorder, and a recorder has no gate. So "PlayerBuilt and EngineFixed: no picker" in C1 cannot fail, and M2 ("always renders") cannot redden C1, contrary to the mutation list. Required revision, either:
- C1 asserts only GameSetupPage's own logic (the start gate from the reported `{source, legal}`, `pile` on the URL, Default with no saved deck) and M2 targets C5 alone; or
- C1 renders the real `PileSourceChoice` with `deckSupplyForFormat` mocked per format.

## Checks that pass (evidence)
- **Claim 1 / census of `main_deck: []`:** 11 non-test literals (brokerClient ×2, multiplayerDraftStore, multiplayerStore, GameProvider ×5, p2p-adapter ×2), matching the plan.
- **Claim 4 / the three GameProvider mocks (g), driven:**
  - Setup: a scratch copy of `client/` at d12daf5793, with GameProvider importing a probe module shaped like `pileSource`. The probe imports `expandParsedDeck`, `loadSavedDeck` and an `engineRuntime` export, and does not import `formatRegistry`.
  - Result: `joinOrigin`, `nativeEngine` and `p2pHostLifecycle` pass 3 files / 120 tests.
  - Control: making the probe module throw fails all 3 files with the probe error, so the import was reached.
- **C3 red at base (driven):** HostSetup with a restored AI seat 1 and the test-environment AI catalog has Host Game disabled for Dandan, Momir and Standard. So the row is red at base, and the PlayerBuilt sibling ("empty catalog stays disabled") holds.
- **(b) purity and binding line 2:** `rg -c deck_supply` returns 0 hits across types.ts, formatRegistry.ts and engineRuntime.ts. Control: `rg -c supplies_fixed_deck client/src/adapter/types.ts` returns 2. `format_axis_census.rs` lists `deck_supply` for both client files. The plan puts `DeckSupply` and the wrapper in engineRuntime.ts. No format literal, card count or legality derivation appears in the plan; verdicts come from `evaluateDeckCompatibility`, and the supply axis comes from `deckSupplyForFormat`.
- **(c) EngineFixed:** `pileSeatDeck` returns empty for Momir on every route, and no picker appears for Momir. The Momir lobby change (no active deck needed, AI seats allowed) is sanctioned by the charter's "supply is not PlayerBuilt" rule and by addendum line 1. The missing tests are in F3.
- **(d) U1 placement:** `.github/workflows/ci.yml` runs `vitest.integration.config.ts` on exactly two named files (the set and cube draft proofs). Those build synthetic card databases from `src/test/fixtures/draftMatchProof.json`, and that job never produces `public/card-data.json`. Any placement in CI therefore needs a workflow edit (out of phase scope), so a local executor gate is correct. U1 discriminates through M1 and M4. Because of `describe.skipIf`, the executor's report must show U1 executed (pass count, not skipped).
- **(e) i18n, driven at base:** `vitest run src/i18n` passes 4 files / 231 tests. Deleting `backButton.label` from `de/menu.json` fails both `resources.test.ts` ("de has exactly the same keys as en") and `__tests__/localeParity.test.ts` (menu.json key set). Only the `menu` namespace is needed (PileSourceChoice lives in `components/menu`; HostSetup's `useTranslation(["multiplayer","menu"])` resolves it).
- **(f) Sizing:** 1 unit, so T1 fails and the conjunction cannot fire. Even if a new `game.json` key for GameProvider's missing-pile message pushed T2 to 13, the conjunction would still not fire. AiOpponentConfig needs no change: `hideDeckPicker={suppliesDeck}`, and its no-legal-decks warning and catalog controls are gated on `!suppliesDeck`.
- **(h) Addenda:**
  - Line 3: the `deck_supply_for_format` sentence ("the lobby offers a pile choice only for `HostPile`", in `lib.rs` and the d.ts) stays true when the picker self-gates on HostPile, which C5 guards.
  - Line 4: the file is `crates/engine/src/types/custom_format.rs`; the addendum's `game/` path is wrong and the plan has it right. The doc says "the only built-in that sets it (Momir)", while the body rejects both Momir and Dandân through `has_unrepresentable_auxiliary_deck_component`, so the plan's correction is right.
- **Claim 5 (server/native empty guest):**
  - Server join resolves names only; `resolve_deck_empty_deck_is_ok` is an existing driven test.
  - The P2P guest gate admits an empty Dandan submission (`e2_the_p2p_guest_gate_admits_an_empty_dandan_submission`).
  - The loader ignores non-pile seats (`e1a`/`e1d`).
  - Live server seat mutations go through `ServerDeckResolver`, which only name-resolves.
  - Rematch copies `searchParams`, so `pile` survives it.

## Constraints for the next plan / executor (exit-round material)
- L1: name both parity suites (`resources.test.ts` and `__tests__/localeParity.test.ts`).
- Name the GameProvider message key used when a named pile has vanished (existing or new, and its namespace).
- The p2p-host fresh branch has no URL producer that sets a format: `mode=p2p-host` routes carry no `format`, and `startActiveP2PHostGame`'s `saveActiveGame` stores no `formatConfig`. Keep the decision (binding line 1) and label the C4 row as one that fixes `formatConfig` synthetically.
- The applicable skill is `add-frontend-component` (Phase 2.5 i18n, Phase 7 tests). Locale parity tests override its "do not edit es/fr/…" line.

## Pre-existing
None reported. The direct-code join gap also affects Momir at base, but the change's own goal covers it, so it sits under F1.

## Probe hygiene
The scratch tree `dandan-run/scratch/p22r` was deleted. No Rust build was needed. W was not modified except for this file.
