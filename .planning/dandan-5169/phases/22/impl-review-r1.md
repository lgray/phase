# Phase 22 implementation review r1 (phase mode + checkpoint mode)

```text
Review Head: 4b59d1541153d571d8107869a45efd7fa83abbff
Completion Gate: FAIL (evidence incomplete at review time; no defect in the change)
Maintainer-Simulation Gate: PASS
```

Verdict: clean. There are 0 blocking findings (0 behavior, 0 text, 0 machinery) and 1 LOW text finding that does not block. The diff reproduces from exactly `91c34cdc..4b59d154`, one commit. Nothing is pre-existing to report: the formatless fresh p2p-host gap was already reported in plan-review r2.

**Completion Gate.** The gate fails only because the evidence was incomplete when this review was written; there is no code fault.
- `completion-p22/summary.log` shows fmt rc=0 and protocol-gate rc=0. At 12:35, clippy was still running, and there were no bindings, nextest or coverage logs yet.
- `completion-p22fe`: tsc passed and lint had 0 errors.
- Vitest had 1 failure in 611 files: `src/config/__tests__/turnCredentials.runtime.test.ts`, a 5000 ms timeout in a file outside the phase. It passes when run alone in W at the candidate (1/1). It is a load flake.
- The orchestrator should fold in the Rust legs when they finish. The phase's only Rust edit is a doc comment.

## Findings

**[LOW]** Two existing comments that the change made false. Evidence: `client/src/pages/MultiplayerPage.tsx` (the `executeAction` header and the `handleHostSetupComplete` comment) and `client/src/providers/GameProvider.tsx` (the `EMPTY_PARSED_DECK` comment). Why it matters: a supplied-deck format no longer uses the active deck. `EMPTY_PARSED_DECK`'s remaining uses are now only on PlayerBuilt paths, including the deckless `p2p-join`. Suggested fix, as text replacements: [text]
- Old: `// Execute a pending action (host or join) with the currently active deck.` New: `// Execute a pending action (host or join) with the active deck, or with none for a supplied-deck format.`
- Old: `// Host setup complete → execute immediately if deck exists, otherwise prompt` New: `// Host setup complete → execute immediately if a deck exists or the format supplies it, otherwise prompt`
- Old: `// Placeholder decklist for fixed-deck formats (Momir's Madness): the player\n// builds nothing, and the engine synthesizes the real deck for every seat. The\n// builders below ignore its contents for such formats.` New: `// The empty decklist a seat submits when it has no deck to send.`

## Judged items

1. **Purity.**
   - `grep -c deck_supply` returns 0 in `adapter/types.ts`, 0 in `data/formatRegistry.ts` and 0 in `engineRuntime.ts`. Control: 6 in `format_axis_census.rs`.
   - The production client diff has no format literal and no card counting. `HostPile` appears only as the mirrored wire string (6 lines).
   - The pile verdict is `evaluateDeckCompatibility`'s `selected_format_compatible`.
   - `isSuppliedFormat` is the sync `formatSuppliesDeck` flag, as the charter allows.
2. **Route census.** Every route listed in the brief is decided as the plan says:
   - the host-setup submit, `executeAction` host (server, P2P, continue-without-lobby, all-AI) and join;
   - `joinP2PRoom`, the direct code and the raw code;
   - the lookup shortcut and the `deckRejected` re-entry. A WS `deckRejected` carries no format, so it stays on the PlayerBuilt path, which is correct;
   - GameProvider: local AI (fresh, resume-fail fallback and native), fresh p2p-host, p2p-join and online join;
   - `HostSetup` AI seats and the four `HostControlTile` consumers;
   - rematch, which copies `pile`.

   Constructed attempts:
   - **Dashboard resume of a named-pile AI game whose restore fails.** `useResumables` builds `mode=ai&difficulty=…` with no `pile`, so the fresh restart uses the default pile. This falls under the charter's drop ("pile-source choice is not persisted across sessions: dropped, a default is the safe state"), so it is not a finding.
   - **Lobby-claimed p2p-host.** The new `await pileSeatDeck` runs before `takeActiveP2PHost`, but that URL never carries `pile`, so the Default source resolves with no engine call. That is a microtask gap only, and no PeerJS macrotask can interleave.

   I could not construct a route that the class misses.
3. **PlayerBuilt/EngineFixed.**
   - Momir submits empty on every route: `pileSeatDeck` returns empty for Default/EngineFixed, the AI seats get the empty `DeckList`, and C4's EngineFixed row and C2a's Momir row cover it.
   - Standard is still required: M6-over reddens only the Standard sibling.
   - **The p2p-join `onNoDeck` change is safe.** A deckless PlayerBuilt guest dials empty. A lobby host built with a `formatConfig` kicks it with `kick{format}`, and the re-entry for a non-supplied format then toasts and opens deck-select (C2b Standard sibling). A resume sends `reconnect`. The gate stays for `p2p-host` (C4 PlayerBuilt row).
4. **Re-drive.** All runs were in W, one probe at a time, restored from `dandan-run/scratch/ir22/orig` with sha256 verified.
   - **Head:** 8 files, 134/134 passed. U1 ran against the real wasm + card-data: 3 passed, none skipped.
   - **Base:** the 7 consumer files were reverted with `git show BASE:`. 40/123 tests red. That covers every C1, C2a (except the Standard sibling), C2b supplied, C2c Dandan, C2d, C3, C3b Dandan, C4 supplied/deckless and C6 row. Siblings stay green.
   - **Mutations:**

| Mutation | Tests that went red |
|---|---|
| M2 (picker ignores supply) | C5 EngineFixed/PlayerBuilt/pending, C1 Momir and Standard, plus 2 cEDH-chip rows |
| M6 | 9 C2a/C2d rows |
| M6-over | Standard sibling only |
| M10a | C2d executeAction + absent legs |
| M10b | C2d continue leg only |
| M5 | C3b Add, Replace, Fill and promotion, each alone |
| M11 | C6 |
| M3 | C4 supplied P2P guest |
| M3-online | C4 supplied online guest |
| M7 | C4 PlayerBuilt p2p guest + fresh host |
| gate kept for join | C4 deckless guest |
| U1 M4 | Momir row; the assertion fires on the returned `error` with the engine reasons ("exactly 60 cards (found 80)") |
| U1 M1 | named-pile multiset row + the Momir refusal control |

   The settle guards are not vacuous: M2 reddens the C1/C5 absence rows. C2c waits 300 ms against a 250 ms debounce, and the Standard sibling in the same harness shows the chip and `hostDisabled`.
5. **Scope and i18n.**
   - Changed paths ⊆ `scope.nul`, with nothing outside it.
   - The 5 unused scope paths are the test files the executor reported (placement corrections).
   - Only the `menu` namespace changed: `pileSource.{label,default}` in all 8 locales, translated.
6. **Comments.** New comments are one sentence each, apart from the LOW finding above. No logic duplicates the engine.
7. **`from_lobby_config` doc.** It is accurate. `has_unrepresentable_auxiliary_deck_component` rejects Momir and Dandan, and `deck_supply` is non-PlayerBuilt only for those two.

**Addenda and constraints.**
- **Addendum lines 1–5:** all honoured. Line 3's lib.rs/d.ts sentence holds because `PileSourceChoice` renders only for HostPile.
- **r1 constraints:** closed. Both parity suites and the `multiplayer:page.couldNotLoadDeck` key are in place.
- **r2 constraints:** closed. The addendum text matches the replacement, the `{id,label,choice}` shape is used, and null is handled at both host-deck sites.
- **r3 constraint:** closed. HostSetup uses `suppliedAiDeckChoice().choice`.
- **r4 constraints:** closed. U1 asserts on `{error,reasons}`, C2d mocks `deckSupplyForFormat`, and C6 sits in the `bracketViolation` harness.

**Probe hygiene.** Porcelain was empty at start and end, and HEAD == candidate. The scratch tree `dandan-run/scratch/ir22` was deleted. There was no cargo run.
