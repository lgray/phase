# Charter review, round 4 (charter mode) — run dandan-5169

Reviewer model: claude-sonnet-5-5. Artifact: whole charter `.planning/dandan-5169/phase-charter` (17 phases, revision r3; byte-identical to `phase-charter.r3`, `cmp` confirms), diffed against `phase-charter.r2`. Measured at `BASE_SHA = b9ba9360` (`git rev-parse --short HEAD`; clean tree). No cargo was run (nothing here needed a build). Every claim below rests on `diff`, `git grep`, `grep`, reading code, `client/public/card-data.json` (35,864 cards) and `docs/MagicCompRules.txt`.

**Verdict: CLEAN (corrections only).** 0 decision revisions, 0 review-only revisions, 2 corrections. Both round-3 decision revisions and both round-3 corrections are closed. Corrections are applied by the orchestrator; no further charter round is required.

## Diff scope check (r2 -> r3)

`diff phase-charter.r2 phase-charter` touches exactly lines 5, 20, 25, 37, 63, 65, 69, 75, 79, 81, 92, 163, 171, 173, 179-180, 182, 186, 188, 224 — every hunk is F1, F2, C1, C2 or their LOC/seam/T2 consequences. No phase boundary, ordering or deferral attribution moved. LOC: phase-list figures sum to 10,140 (`python3 -c 'print(sum([150,900,100,80,1300,960,300,900,350,600,550,900,700,750,550,450,600]))'` = 10140); Phase 2 -110, Phase 7 +150 = +40 over r2's 10,100, matching lines 5 and 37.

## Round-3 findings: closure check

| ID | Status | Evidence |
|---|---|---|
| F1 (stored ceiling field -> axis) | closed | Phase 2 Goal/Scope/Claims/Verification now carry a sixth axis method returning `MatchType`, `Custom(_)` = Bo3, no `FormatConfig` field, no serde default, no verdict row, no client mirror; `git grep`/`grep -n ceiling phase-charter` shows no residual "field", "mirror", "registry helper" or "deserializes to Bo3" text (every hit is the axis form, or the unrelated ~30,000-line PR ceiling). Measured feasibility: `FormatConfig.format: GameFormat` exists (format.rs:620); `MatchType { Bo1, Bo3 }` exists (types/match_config.rs:7); the coercion site is `start_game_with_starting_player` (engine.rs:18431, `Bo3 && players.len() != 2 && archenemy(state).is_none()`), which can read `state.format_config.format`; the wasm precedent `isCardCommanderEligibleForFormat` exists (engine-wasm lib.rs:1105, wrapper engineRuntime.ts:251, binding `client/src/wasm/engine_wasm.d.ts` tracked, last touched by upstream export commits). `no_format_axis_key_reaches_the_client_mirror` scans only `client/src/adapter/types.ts` and `client/src/data/formatRegistry.ts` for axis key names; the export lives in `engineRuntime.ts` + the `.d.ts`, so the census stays green as Phase 2 claims. T2 recount: Phase 2 scope rule lists 18 paths (counted by hand: format.rs, deck_validation.rs, custom_format.rs, engine-wasm lib.rs, custom_format_schema.rs, format_axis_census.rs, types.ts, formatRegistry.ts, ws-adapter.ts, network/protocol.ts, protocol.test.ts, p2p-adapter-multiplayer.test.ts, deckUrlImport.test.ts, lobby-broker protocol.rs, server-core protocol.rs, check-protocol-version.mjs, card-bot formats.ts, manabrew-compat lib.rs) = the stated 18; T1 = 1 stands (an axis method is not a second lockstep mechanic). |
| F2 (`GameSetupPage.tsx` second Bo3 control) | closed | Phase 7 scope, Goal, claim 5, Verification and T2 (9 paths; 10 ungrouped, both < 13) carry it. Measured: `git grep -n 'setMatchType(' -- client/src/components client/src/pages ':!*__tests__*'` returns HostSetup.tsx (518, 615, 1004, 1009), GameSetupPage.tsx (137, 149, 538, 550, 561) and `tournament/CreateTournamentForm.tsx:223` (display metadata, correctly excluded); both game-setup controls gate on `playerCount !== 2` only (HostSetup 1011, GameSetupPage 562); `effectiveMatchType` (HostSetup 471) and the `lastMatchType` restore (GameSetupPage 137) and `match=` navigation (GameSetupPage 222) exist as named. Every other production `match=` / `match_type:` producer was enumerated: `MultiplayerPage.tsx:628` consumes HostSetup's `effectiveMatchType` through `action.settings.matchType`; `GamePage.tsx:323` only decodes the URL param; `multiplayerStore.ts` reads/rehydrates `settings.matchType`. No third game-setup control exists, and the engine coercion is the backstop for any that did. Both test files exist (`ls`). |
| C1 (`evaluate` not `score`) | closed | Phase 3 claim 1 reads `FixedDeckKeepMulligan::evaluate`. |
| C2 (struct-literal snapshot) | closed | The bullet is replaced by the axis form with a durable buy (`no_format_axis_key_reaches_the_client_mirror` green; `built_in_axes_no_looser_than_rules` unchanged); no `FormatConfig {` count remains. |

## Whole-charter fresh pass (emphasis on r3 text)

Held on measurement, not raised:
- Every CR cited (103.5, 103.5c, 104.4a, 121.2, 121.2a, 121.6b, 612, 612.2, 613.1c) grep-verified in `docs/MagicCompRules.txt`; characterizations match.
- Card premises against `client/public/card-data.json`: Day's Undoing, Haunted Fengraf, Mystic Sanctuary (an ETB "may put target instant or sorcery card from your graveyard on top of your library" — Phase 10's "targets an instant in the shared graveyard" is consistent), The Surgical Bay, Wheel of Fortune, Prosperity, Metamorphose, Memory Lapse, Hinder all read as the charter uses them.
- Settled decisions are honored: free-reveal predicate (Phase 13), Day's Undoing wheel (Phase 15), full list incl. PREREQ-0/1/2/3 (Phases 4, 5, 15), architecture scope note.
- Card-bot: `scripts/card-bot/formats.ts` and `formatRegistry.ts` each hold 25 entries at base, so "26 with Dandan" and Phase 1's green-at-25 hold.
- Seam notes after the r3 edit agree with the scope rules of Phases 2, 6, 7 and 9 for `engine-wasm/src/lib.rs` (Phase 9's edit there is conditional and is named in Phase 9's own seam).
- Recursive T1∧T2: Phase 2 (T1 = 1, T2 = 18) and Phase 7 (T1 = 1, T2 = 9) do not fire the conjunction.

## Corrections

### C1 — Charter title carries a stale revision label
Coordinate (line 1), old text: `# Phase charter — Dandan format (issue #5169), run dandan-5169 (revision r2)`
Replacement: `# Phase charter — Dandan format (issue #5169), run dandan-5169 (revision r3)`

### C2 — Run-level note restates the LOC estimate as a snapshot that the r3 edit falsified
Coordinate (Run-level notes, Settled decision 3 bullet), old text: `(one PR unless total exceeds ~30,000 lines; the estimate is ~10,100)`
Measurement: the phase-list sum is 10,140 (line 5 and line 37 say so).
Replacement (durable form, so the next edit cannot re-stale it): `(one PR unless total exceeds ~30,000 lines; the estimate is the phase-list sum stated in the Deliverable line)`

## Observations for the phase plans (not counted, no charter text changes)

- Phase 7's export wrapper should not copy the precedent's `await ensureCardDatabase()` (`engineRuntime.ts:251-258`): the ceiling needs no card database. The precedent also answers `false` on an undeserializable format (lib.rs:1107-1109); for the ceiling the fail-closed answer is the phase plan's to state (Bo3 must not be offered on a decode failure for a shared-zone format), consistent with the charter's "neither control offers Bo3 before the answer has resolved".
- Phase 7's coercion needs a typed "exceeds the ceiling" comparison; `MatchType` derives no `Ord` (types/match_config.rs:6-11), so the phase plan picks a method on `MatchType` rather than an inline pair of comparisons. Keep the existing `archenemy(state).is_none()` structure in the same condition.
- `GameSetupPage.tsx` persists the choice through `setLastMatchType`; the phase plan should decide that a forced Bo1 for Dandan does not overwrite the remembered Bo3 for other formats (the Phase 7 test row reads the remembered value, so it already discriminates).
- `multiplayerStore.ts:2479` rehydrates a persisted host session's `matchType`; a Dandan session can only have been persisted as Bo1 once Phase 7's controls land, and the engine coercion covers gameplay regardless.
