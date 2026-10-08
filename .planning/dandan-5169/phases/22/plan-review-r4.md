# Phase 22 plan review r4 (phase-plan mode; phase-fit context, so Sizing is blocking)

Verdict: clean. There are no blocking findings: 0 behavior, 0 text, 0 machinery. No row-reachability or vacuity finding came back, so the non-convergence split rule does not fire. F6 is closed.

Sizing is consistent. There is 1 unit and 12 non-test paths, so T1 and T2 do not fire. The C6 row adds only a test path; `GamePage.tsx` was already counted.

This was a delta round (`diff plan.r3.md plan.md`), plus an independent re-drive of seven rows on my own stand-ins.

## Delta checks (each driven)

Instrument: a scratch copy of `client/` at `dandan-run/scratch/p22r4rev`, with `node_modules`, `public` and `src/wasm` symlinked. I wrote my own minimal stand-ins:
- `pileSource.ts` and `engineRuntime.deckSupplyForFormat`
- `PileSourceChoice.tsx`
- the MultiplayerPage host sites
- `HostControlTile`, the `GamePage` `pile` prop, the GameProvider guest routes and `GameSetupPage`

Runs:
- Each row ran three ways: BASE (W's production file swapped back in), STAND-IN, and each mutation, applied one at a time and restored by copy.
- Command: `npx vitest run --coverage.enabled=false <file>`. U1 added `--config vitest.integration.config.ts`.

Results:

- **F6 closed (C2d).** Probe file: `MultiplayerPage.hostServer` header, with `pileSeatDeck` wrapped as a sequenced mock that falls back to the real function.
  - The stand-in passed 9/9.
  - M10a reddened the executeAction leg and the absent-from-storage leg, and left the continue leg green.
  - M10b reddened only the sequenced continue leg.
  - An always-`null` continue variant stayed green under M10b. That is r3's vacuity, reproduced; the sequenced mock is what fixes it.
  - On the continue leg the "Continue without lobby" button is present before the click.
  - BASE: 8/9 red. The Standard sibling stays green, as its reach guard should.
  - M6 reddened 8 rows. M6-over reddened only the Standard sibling.
- **U1 rewrite holds (real wasm, `initialize_game` + `card-data.json`).** The run used a named pile of 40 Forest, 20 Island, 10 Llanowar Elves and 10 Grizzly Bears.
  - `get_game_state().state.objects` has 80 objects. 73 of them are `Hidden Card`: all 66 library cards, plus the opponent's 7-card hand.
  - `export_game_state_json` `.state.objects` has 80 names, and their multiset equals the pile exactly.
  - The default (all-empty) Dandan game has 80 objects with `Dandân` ×10, so the multiset row discriminates M1.
  - Momir with all seats empty loads (120 snow basics plus 2 `Emblem`). That row asserts only that it loads, not a multiset, so the emblems do not matter.
  - Momir with the 80-card list is refused.
- **C3b (one test per consumer) holds.**
  - The stand-in passed 7/7.
  - M5 reddened Add AI, Replace AI, Fill With AI, promotion and no-dropdown, each in its own test.
  - The dropdown-hide mutation reddened only the no-dropdown test.
  - BASE: the same 5 tests are red. The Standard empty-catalog and one-deck siblings are green.
- **C6 / M11 hold.** Probe: the `GamePage.bracketViolation` harness with `pile` captured.
  - The stand-in passed 2/2.
  - M11 reddened only the `pile` row.
  - BASE reddened the `pile` row.

## Independent re-drive (sample beyond the delta)

- **C1 + M2** (GameSetupPage with the real `PileSourceChoice` stand-in):
  - The stand-in passed 6/6.
  - M2 reddened the Momir and Standard no-picker siblings.
  - Removing the reset reddened the format-switch row.
  - Removing the `legal` gate reddened the incompatible row and the format-switch row.
  - Removing the `pile=` param reddened the compatible row.
  - BASE: all red.
- **C2a + M6**: covered in the C2d run above.
- **C4** (guest-route subset: supplied p2p-join and online join submit empty, deckless p2p-join, PlayerBuilt p2p-join and online join, p2p-host `onNoDeck`):
  - The stand-in passed 6/6.
  - M3, M3-online, M7 and M7-online each reddened only their own row.
  - Keeping the deckless p2p-join gate reddened that row.
  - Removing the gate for p2p-host too reddened the p2p-host `onNoDeck` row.
  - BASE: the 3 supplied/deckless rows are red.
  - Claim 4: with `pileSource` imported by GameProvider, the existing `joinOrigin`, `nativeEngine` and `p2pHostLifecycle` suites stay green (120/120).

Result: the journal's claims hold on every row I re-drove: U1, C1, C2a, C2d, C3b, C4 (subset) and C6.

## Constraints for the executor (exit-round material, no round)

- **U1:** the raw `initialize_game` refusal is a returned `{error: true, reasons: [...]}`, not a throw. Measured: the Momir 80-card call returned "Momir's Madness decks must have exactly 60 cards (found 80)" and did not throw.
  - Assert "loads" as a result without `error`, and "refused" on `error`/`reasons`. Alternatively, go through `WasmAdapter`'s `throwInitFailure`.
  - Otherwise the Momir "loads" leg cannot redden under M4.
- **C2d's absent-from-storage leg uses the real `pileSeatDeck`**, so the file must mock `engineRuntime.deckSupplyForFormat` (HostPile for Dandan). Otherwise the jsdom wasm load fails before the toast.
- **C6:** the existing `GamePage.bracketViolation.test.tsx` harness already records `GameProvider` props (`formatConfig`, `serverUrl`). Adding `pile` there is a few lines, against roughly 400 lines of mocks to rebuild for a new file.
- **Test file names:** the journal's stand-in files (`GameSetupPage.dandanPile`, `HostSetup.dandanPile`, `HostControlTile.dandanPile`, `MultiplayerPage.dandanJoin`) differ from the matrix and Sizing list (extensions of the existing suites; C2b in `MultiplayerPage.dandanPile`). Either placement is fine; keep the Sizing test-path list matching what lands.

## Probe hygiene

- The scratch tree, its logs and the stand-in copies were deleted afterwards.
- There was no Rust build.
- W was not modified apart from this file.
