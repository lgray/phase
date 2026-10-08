# Phase 22 plan review r2 (phase-plan mode, phase-fit context: Sizing blocking)

Verdict: NOT clean. 1 blocking finding (F5, `behavior`). F1–F4 are each closed by a design edit. The delta round found no design finding, so a whole-artifact pass followed, and F5 came from that pass. The C2a row with this defect was already in r1's C2. Sizing is consistent: 1 unit, 12 non-test paths, T1/T2 do not fire.

## Findings

### F5 `behavior`: C2a's PlayerBuilt sibling asserts the behaviour of the mutation it should catch
Old text (Verification Matrix, row C2a, Reach guard column): "Standard sibling with no active deck still toasts "select a deck first" and calls neither start".

Evidence (driven at base, scratch copy, `MultiplayerPage.hostServer` harness, `onHost` invoked with Standard and no `active-deck` key):
- `host-setup` unmounts (deck-select).
- `startHosting` and `startP2PHostingSession` are not called.
- `toasts` is `[]`.

`handleHostSetupComplete` routes a missing `activeDeckName` to deck-select and never reaches `executeAction`. The `selectDeckFirst` toast lives only in `executeAction`. The plan keeps that route for PlayerBuilt ("PlayerBuilt unchanged"), so the row as written cannot pass.

It goes green only under the over-removal mutation, where the deck requirement is dropped for every format. Then `executeAction` toasts `selectDeckFirst` and the form stays on host-setup. So the row inverts the guard M6 needs.

Replacement: "Standard sibling with no active deck: deck-select is reached (host-setup unmounts), no toast, neither start is called". With that wording, the over-removal mutation reddens the row on both the toast and the view.

## F1–F4 closure (delta)
- **F1, closed.** I re-ran the r1 census commands.
  - The `MultiplayerPage` hits are: the re-entry effect, `joinP2PRoom`'s navigate, the `executeAction` compat refusal, the direct-code navigate, the server-join params, `handleHostSetupComplete`, the raw-code branch, the post-lookup deck-select, the banner (Change), the chip and the warning. Each one has a decided route-table row.
  - The `ACTIVE_DECK_KEY|activeDeckName|loadActiveDeck(` file list is GameProvider, GameSetupPage, MultiplayerPage, MyDecks, HomeDashboard, feedService, backup and storage. It matches the plan's dispositions.
  - Handshake (claim 8), read in `p2p-adapter.ts`: the host refuses any first message other than `guest_deck`/`reconnect`. A guest with a `playerToken` sends `reconnect` (no deck), otherwise `guest_deck`. The format reaches the guest only as `kick.format`. `GameProvider` parses that into `deckRejected.format`, and `GamePage` hands it to the re-entry. So "no wire field" holds.
  - Driven base state for C2a/C2b, red where the plan inverts it:
    - Dandan host with no deck: deck-select, no `startHosting`.
    - Momir or Dandan host with an illegal active deck: the engine reason is toasted and no start is called.
    - Raw `ABCDE` code with no deck: deck-select, no `p2p-join` navigate.
    - Re-entry with `format: "Dandan"`: toast and deck-select.
    - Control: Standard with a deck reaches `startHosting`.
  - "Dial empty cannot admit a deckless guest into a PlayerBuilt room" holds for every host built with a `formatConfig`: the lobby `startP2PHostingSession` and both draft hosts. It does not hold for a host without one; see Pre-existing.
- **F2, closed.** `rg -n 'deckChoices|pickRandomAiDeck|haveAnyDeck' HostControlTile.tsx` shows that Add AI, Replace with AI, the promote effect (which skips seats already holding `DeckList`) and `fillWithAiAndStart` all read `deckChoices`. One cut there covers all four. C3b asserts all four, plus the Standard empty- and one-deck siblings.
  - Driven base: Dandan with an empty catalog has Add AI and Fill With AI disabled. Standard with an empty catalog has both disabled. Standard with a 1-deck catalog has both enabled. So C3b is red at base, and the control shows the predicate fires.
- **F3, closed.** C4's PlayerBuilt siblings assert the submitted deck equals the active deck's expansion on p2p-join, online join and fresh p2p-host, and that p2p-host with no deck still calls `onNoDeck`. C2a has the Momir lobby-host row, which is red at base (driven, above).
- **F4, closed.** C1 renders the real `PileSourceChoice` with `deckSupplyForFormat` mocked at `engineRuntime`, and runs absence assertions after the answer settles. M2 (the picker always renders) therefore reddens C1's Momir/Standard siblings.
  - Existing suites that mount these pages either spread the actual `engineRuntime` (`GameSetupPage.test`, `HostSetup.test`) or mock `HostSetup` out (`MultiplayerPage.*`). The `useBestOfThreeCeiling` failure path already runs there. `offlineLocalPlay.integration` mocks `engineRuntime` without either wrapper, but it is excluded from the unit config and from CI's named integration list.

## The deliberate PlayerBuilt change (p2p-join no-deck gate)
- **Base, driven** (scratch `GameProvider` harness, `p2p-join` with `joinCode`):
  - With no active deck, `onNoDeck` fires and `joinRoom` is never called.
  - Control: with a deck, `joinRoom` is called.
- **After the change:**
  - A deckless PlayerBuilt guest dials empty. The host gate kicks it with `kick{format}` (claim 7), and the re-entry for a non-supplied format toasts and opens deck-select (C2b Standard sibling). This is unchanged UX plus one round trip.
  - A deckless guest resume sends `reconnect` and now works. At base it was blocked for PlayerBuilt and Momir alike, which was a base defect.
  - `onNoDeck` is kept for PlayerBuilt `p2p-host`.
- **Coverage:**
  - C4 "no active deck and no format builds the guest adapter with an empty deck and no `onNoDeck`" is red at base (driven above).
  - C4 "p2p-host with no active deck still calls `onNoDeck`" catches removing the whole gate.
  - Together they discriminate.

## Sizing / addendum
- **Path count.** The 12 named non-test paths are each needed by a route-table row: `engineRuntime`, `pileSource`, `PileSourceChoice`, `GameSetupPage`, `GamePage`, `GameProvider`, `MultiplayerPage`, `HostSetup`, `HostControlTile`, `multiplayerStore`, the `menu` locale group (8 dirs) and `custom_format.rs`. `HostSettings` aliases `HostingSettings` in `multiplayerStore`.
- **`multiplayer` locale group.** It is correctly dropped: the only new message reuses `multiplayer:page.couldNotLoadDeck`, which exists.
- **Engine export.** `deckSupplyForFormat` is exported in `engine_wasm.d.ts` with the HostPile sentence, and it needs only `ensureWasmInit`, not the card DB.

## Constraints for the executor / orchestrator (exit-round material)
- **Addendum line.** The line `phase-22 plan r2 (decision: scope rule)` lists 5 additions to "the charter's 9 paths (12 non-test …)" but omits the two drops, so 9 + 5 ≠ 12. Replace it with: "phase-22 plan r2 (decision: scope rule): the plan adds pileSource.ts, GamePage.tsx, HostControlTile.tsx, multiplayerStore.ts (type field) and crates/engine/src/types/custom_format.rs, and drops AiOpponentConfig.tsx and the multiplayer locale group (no edit needed), from the charter's 9 paths: 12 non-test; 1 unit; T1/T2 do not fire."
- **Claim 7's consequence sentence** ("answered by `kick{format}` by a PlayerBuilt room") holds only for hosts constructed with a `formatConfig`. `handleNewGuest` gates with `if (this.formatConfig)`. GameProvider's fresh p2p-host branch never has one in production.
- **Claim 6.** The `formatSuppliesDeck` file list also hits `adapter/types.ts`, but only in a doc comment. No decision changes.
- **`suppliedAiDeckChoice()`.** It must also supply the `{id, label}` that `HostControlTile`'s `deckChoices` entries carry. The dropdown is hidden, so the label is never displayed.
- **`pileSeatDeck` null.** `pileSeatDeck` is async and may return `null`. Both lobby host-deck sites (`executeAction` and the continue-without-lobby callback) handle `null` with the existing `page.couldNotLoadDeck` toast.

## Pre-existing (untagged, not a finding)
- **A P2P host without a `formatConfig` never gates guest decks.** `handleNewGuest` validates only `if (this.formatConfig)`.
  - The only such host is `GameProvider`'s fresh p2p-host branch. Its `formatConfig` is `savedFormatConfig ?? FORMAT_DEFAULTS[formatParam]`, and neither producer sets it: `rg -n 'saveActiveGame\(\{[^}]*p2p'` finds `GameProvider` and `multiplayerStore`, both without `formatConfig`, and no `mode=p2p-host` URL carries `format`.
  - That branch runs only when the lobby adapter is gone and the saved state and session do not pair up. For example, a `useResumables` resume with state saved but no session clears both and opens a fresh, formatless room.
  - At base, such a room seats any illegal guest deck. After this change it also seats a deckless raw-code guest, who previously got `onNoDeck`.
  - The plan's claimed cases are unaffected. It is for the owner to triage.

## Probe hygiene
- **Setup.** The scratch tree was `dandan-run/scratch/p22r2`: a copy of `client/` with `node_modules` symlinked to W's. Probe files were `MultiplayerPage.p22probe`, `HostControlTile.p22probe` and `GameProvider.p22probe`, built from the existing harness headers.
- **Results.** All base expectations held; the passing runs are the evidence.
- **Cleanup.** The scratch tree was deleted. There was no Rust build. W was not modified except for this file.
