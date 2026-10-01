# Phase 7 plan review, round 2 (phase-plan mode, phase-fit context declared: Sizing check blocking)

MODEL: claude-sonnet-5-5
Reviewed: whole revised `phases/7/plan.md` against the charter Phase 7 entry and code at HEAD 67cd221e. Static review; no cargo run. "Measured" = read or grepped at HEAD.

## Verdict: APPROVE (0 blocking findings; 3 non-blocking notes)

## Closure of round-1 findings
- F1 CLOSED. Plan 5.3 now routes the wrapper call through an async try/catch (rejection, missing mocked export and synchronous throw all resolve to Bo1). The 8.2 census sentence now names `offlineLocalPlay.integration.test.tsx`, which is in the vitest list (8.2 and step 8). H5 is the synchronous-throw reach-guard. Measured: `GameSetupPage` has no early return (first `return` is line 269), so the hook placement is rules-of-hooks safe. `sideboardPolicyForFormat` (engineRuntime.ts:362) is precedent for a wrapper that calls `ensureWasmInit()` alone.
- F2 CLOSED. `match_type_within` is a private exhaustive helper in `engine.rs`; `match_config.rs` is out of scope. `MatchType` derives `Copy, PartialEq, Eq` and no `Ord` (match_config.rs:6), so the by-value two-axis match compiles. R5 is now behavior legs (Bo1/Bo1, Bo1/Bo3) plus R1 (Bo3/Bo1) and R3 (Bo3/Bo3). The variant-discoverability grep is corrected.
- F3 CLOSED. The hook and `cappedMatchType` are exported from `HostSetup.tsx`. Section 10 lists 10 rows, 9 grouped paths, and matches the charter's 9 paths exactly. Section 11 counts agree (T1 = 1, T2 not reached).
- F4 CLOSED. 5.4 stores the raw `matchType` in `rememberHostConfig` (HostSetup.tsx:726 at HEAD is `matchType: effectiveMatchType`, line 744 is the `onHost` entry; both line references verified). H1 asserts the remembered value stays Bo3, H3 asserts the same for the pre-resolve host, and H2 is the paired guard.

## Fresh review: no blocking findings
Checked and passed:
- Engine clamp: lines 18431-18436 hold the seat-count/Archenemy block, and the clamp follows it. The ceiling is read from `state.format_config.format` (`best_of_three_ceiling`, format.rs:2329, Dandan = Bo1). Only `match_type` is written, so `loop_detection` is untouched (R6).
- C2: wasm `initialize_game_impl` (lib.rs:1784 sets the config before the starts at 1923-1925); `session.rs:923-936` and `:2628-2638` set the config in a fresh pre-start state; `match_flow.rs:498` sets it before the restart at 569; `replay.rs:85` sets it before the start. No post-start setter was found.
- Client: the two Bo3 controls are as described. HostSetup's Bo1/Bo3 buttons are at lines 1004-1011. GameSetupPage reads `matchType` at lines 222, 342, 552, 564 and 641, all covered by 5.5. The format-change handlers reset `matchType` only for `min_players !== 2`, so H4's "Bo3 choice preserved across formats" holds.
- No new string is added, so locale parity and `add-frontend-component` Phase 2.5 hold. The `"Dandan"` grep guard keeps the frontend from deriving the rule.
- MultiplayerPage and GameSetupPage test census: no `engineRuntime` mock exists in those files (measured), so they take the real-wrapper fail-closed path as the plan says.

## Non-blocking notes
- N1 [note] `eslint.config.js` sets `react-refresh/only-export-components` to `warn`. Exporting `useBestOfThreeCeiling` and `cappedMatchType` from `HostSetup.tsx`, which today exports only types and the component, will probably add two lint warnings. There is no `--max-warnings` anywhere (measured), so it is not a gate. The executor should compare the lint warning count before and after, and may add a one-line `eslint-disable-next-line react-refresh/only-export-components` with the reason (shared hook; `engineRuntime.ts` cannot host it) if the repo is warning-clean.
- N2 [note] C2's entry list omits `server-core/draft_session.rs:663`, which builds a match config for draft pods. It still reaches the engine via `GameSession::start_game`, and its format is Limited (ceiling Bo3), so nothing changes. The executor's C2 re-grep will surface it, and it is not a finding.
- N3 [note] Carried from round 1 (N3/N4), unchanged: the first-mount double compat pass on `GameSetupPage`, and the Dandan-labelled tournament form keeping a Bo3 default. The plan's PR-description note should say both explicitly.
