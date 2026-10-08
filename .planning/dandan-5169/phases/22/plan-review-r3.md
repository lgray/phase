# Phase 22 plan review r3 (phase-plan mode; phase-fit context, so Sizing is blocking)

Verdict: NOT clean. There is 1 blocking finding, F6 (`behavior`), with replacement text supplied. F5 is closed. Claim 7 is now scoped correctly. The `suppliedAiDeckChoice` shape is right. Sizing is unchanged by the delta and consistent: 1 unit, 12 non-test paths, so T1 and T2 do not fire. This was a delta-only round (`diff plan.r2.md plan.md`); r2 already did the whole-artifact pass.

## Findings

### F6 `behavior`: the C2d continue-without-lobby leg cannot reach its site under the mock as written, so M10 at that site stays green

**The ordering.** In `executeAction`, the host-deck construction runs before the broker probe and before `setBrokerOfflinePrompt`. Today that construction is `expandDeck()`; the plan replaces it with `pileSeatDeck` "at both host-deck sites".

**Probe, driven at base.** I used a scratch copy with the `MultiplayerPage.hostServer` harness. The host deck was forced to `null`, and the setup was P2P with the official broker unavailable. Results:
- The only toast was `page.couldNotLoadDeck`.
- `ensureSubscriptionSocket` was never called.
- The "Continue without lobby" button never rendered.

**Why the leg is vacuous.** A `pileSeatDeck` mock that always resolves `null` (what C2d says) stops at the first site. The continue-callback leg then sees the toast and no start, which are exactly its assertions, even with that site's null-handling deleted. So M10 at the continue site cannot redden the row.

**A second gap in M10.** M10 says "either host-deck site", which lets the executor run only one of the two mutations.

**Replacement for the C2d row, test column:** "`pileSeatDeck` mocked to resolve `null` for a named pile: the `multiplayer:page.couldNotLoadDeck` text is toasted and neither start is called. At the `executeAction` site the first call resolves `null`. At the continue-without-lobby callback (P2P, broker not `LobbyOnly`), the first call resolves the pile and the second resolves `null`, and the leg clicks "Continue without lobby"."

**Replacement for the C2d row, reach column:** "The same flows with a resolving pile reach the start. On the continue leg, the "Continue without lobby" button is present before the click."

**Replacement for M10:** "M10a: the `executeAction` site ignores a `null` pile. M10b: the continue-without-lobby callback ignores a `null` pile. Each, run alone, reddens its own C2d leg."

## Delta checks
- **F5 closed.** The C2a Standard-sibling text matches the r2 replacement word for word. I drove it in the scratch copy: `onHost` with Standard and no `active-deck` key.
  - At base it passes: host-setup unmounts, `toasts` is empty, and neither `startHosting` nor `startP2PHostingSession` is called.
  - Under the M6 over-removal mutation (`if (activeDeckName || true)` in `handleHostSetupComplete`), the row goes red. The first failing assertion is `queryByTestId("host-setup")).not.toBeInTheDocument()`, because `executeAction` toasts `selectDeckFirst` and the form stays up.
  - The mutation was restored (`diff` came back empty).
- **C2d sites exist.**
  - `rg -n 'couldNotLoadDeck' client/src/pages/MultiplayerPage.tsx` finds both host-deck null branches: inside `executeAction`'s host branch, and inside the `BrokerOfflinePrompt` `onContinueWithoutLobby` callback. It also finds the active-deck load at the top of `executeAction`.
  - The key exists in all 8 `multiplayer.json` locales.
  - The continue site is drivable: the existing `MultiplayerPage.hostServer` and `MultiplayerPage.offline` suites click "Continue without lobby". The `executeAction`-site leg plus its M10a mutation discriminates as written.
- **Claim 7 is scoped correctly.** `p2p-adapter.ts` `handleNewGuest` calls `validateGuestDeck` only `if (this.formatConfig)`. The new sentence says exactly that, and correctly notes that GameProvider's fresh p2p-host has no `formatConfig`.
- **`suppliedAiDeckChoice` shape is right.** `HostControlTile` types `deckChoices` as `Array<{ id: string; label: string; choice: DeckChoice }>`. The planned `{id, label, choice}` with `choice` an empty `DeckList` fits that type. The promote effect skips seats whose deck is already `DeckList`, so the empty choice is never re-promoted.

## Constraints for the executor (exit-round material)
- The `HostSetup` route-table row says the AI-seat deck "is `suppliedAiDeckChoice()`". `HostSetup`'s `defaultAiDeck` is a bare `DeckChoice`, so `HostSetup` must use `suppliedAiDeckChoice().choice`. `tsc` enforces this.

## Probe hygiene
- The scratch tree was `dandan-run/scratch/p22r3`: a copy of `client/` (without `public`, `coverage` or `src-tauri`) with `node_modules` symlinked to W's.
- The probe file was `MultiplayerPage.p22r3probe.test.tsx`, built from the `hostServer` header.
- The base run passed 2/2. Its rc=1 came only from the global coverage threshold.
- The tree was deleted afterwards. There was no Rust build, and W was not modified except for this file.
