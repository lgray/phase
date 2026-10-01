# Phase 7 plan review, round 1 (phase-plan mode, phase-fit context declared: Sizing check blocking)

MODEL: claude-sonnet-5-5
Reviewed: `phases/7/plan.md` against charter Phase 7 entry and code at HEAD 67cd221e. Static review plus client-side reads; no cargo run (constraint). Nothing below was compiled; "measured" means read or grepped at HEAD.

## Verdict: REVISE (4 blocking findings, all `text`/`behavior` repairs with replacement text; no machinery finding, no design change needed)

The design is sound on the lenses asked about: the engine is the only ceiling authority, the wasm helper fails closed, the export needs no card database, the Phase 6 seam is clean, and the Sizing section is consistent with the body. The findings are scope homing, one missed mocked-engineRuntime consumer, and one remembered-choice leak in HostSetup.

## Scope adjudication (the two paths outside the charter's Phase 7 scope rule)

### `crates/engine/src/types/match_config.rs` (`MatchType::exceeds`): YES, homable in a chartered path, no harm (finding F2)
The charter chartered `engine.rs` for "`start_game_with_starting_player`, the match-structure normalization only". A private exhaustive helper beside that function is part of that normalization. `MatchType` derives `PartialEq, Eq` but no `Ord` (read at `match_config.rs:6`), so the typed comparison is still needed; it does not have to be a public method on the type. A charter decision revision is not needed. Concrete relocation in F2.

### `client/src/hooks/useBestOfThreeCeiling.ts` (new hook): YES, homable in a chartered path, with one harm-avoiding constraint (finding F3)
- `engineRuntime.ts` is NOT a viable home for the hook. Tests mock that module with a bare factory and no `importActual` (measured: `client/src/__tests__/offlineLocalPlay.integration.test.tsx:251-258`, which renders `GameSetupPage`). A hook exported from there would be `undefined` under that factory, so the page crashes at render (vitest throws on access to a missing factory export). Keep only the async wrapper there, as the plan has it.
- The viable home is `HostSetup.tsx`: export `useBestOfThreeCeiling` and `cappedMatchType` from it, and import them in `GameSetupPage.tsx`. Both files are chartered, nothing is duplicated, and no new file is created. Cost: the page imports a hook from a lobby component file (a mild layering smell, not a correctness problem). `GameSetupPage.test.tsx` and the other test files already mock `wasm-adapter` by factory, and `HostSetup` uses `getHostAdapter` only on access, so the extra import should not add load-time failures. The executor confirms by running the listed vitest files.
- If the lead judges the page-to-lobby-file import unacceptable, the only alternative is a charter decision revision adding the hook file. I do not recommend it: the added path buys layering tidiness only.

## Blocking findings

### F1 [text] Census of tests that render the controls misses a factory-mocked engineRuntime consumer (plan section 8.2, last paragraph)
Plan text (old string): "Other test files that render these components (`MultiplayerPage.*`, `GameSetupPage.startingLife/.visualPacks/.deckPlayed`) do not mock the wrapper: the stubbed `@wasm/engine` has no export, the wrapper rejects, the hook fails closed to Bo1, and none of them selects Bo3"

The census is incomplete. `client/src/__tests__/offlineLocalPlay.integration.test.tsx` mocks `../services/engineRuntime` by a factory that lists six exports and not the new one (lines 251-258), then renders `GameSetupPage` and clicks Start Match (lines 533, 595). In that test the wrapper is not "a wrapper that rejects": accessing the missing export throws synchronously, inside the effect body, unless the hook routes the call through an async boundary. The plan's "a rejection resolves to Bo1" does not cover a synchronous throw.

Replacement text, appended to section 5.3's hook bullet and replacing the 8.2 sentence:
- Hook bullet: "The effect invokes the wrapper inside an async function with try/catch (`void (async () => { try { ceiling = await bestOfThreeCeilingForFormat(format) } catch { ceiling = 'Bo1' } ... })()`), so a rejection, a missing export on a mocked module, and a synchronous throw all resolve to Bo1."
- 8.2 sentence: "Test files that render these components without mocking the wrapper (`MultiplayerPage.*`, `GameSetupPage.startingLife/.visualPacks/.deckPlayed`) and `src/__tests__/offlineLocalPlay.integration.test.tsx`, whose factory mock of `engineRuntime` omits the export, exercise the fail-closed path; the executor runs all of them (added to the step 8 vitest list) and confirms no unhandled error and no changed assertion. The census is `git grep -ln 'services/engineRuntime' -- 'client/src/**/__tests__/*'` intersected with the test files that render `GameSetupPage` or `HostSetup`, re-run at PHASE_BASE."
- Add `src/__tests__/offlineLocalPlay.integration.test.tsx` to the client vitest command in 8.2 and step 8, and add a row H5: a render of `HostSetup` (or the page) under a factory mock of `engineRuntime` that omits the export shows Bo3 disabled and submits Bo1 (revert: call the wrapper synchronously in the effect body and the render throws). This is the reach-guard for the fail-closed claim, which today is asserted only with a rejecting mock.

### F2 [text] Relocate `MatchType::exceeds` into `engine.rs` and drop `match_config.rs` from scope (sections 5.1, 6, 7, 8.1 R5, 10, 11)
Replace the 5.1 method with a private helper in `crates/engine/src/game/engine.rs` directly above `start_game_with_starting_player`:

```rust
/// CR 100.6a + CR 100.4: the structure actually played; never longer than `ceiling` allows.
fn match_type_within(configured: MatchType, ceiling: MatchType) -> MatchType {
    match (configured, ceiling) {
        (MatchType::Bo3, MatchType::Bo3) => MatchType::Bo3,
        (MatchType::Bo3, MatchType::Bo1) | (MatchType::Bo1, MatchType::Bo1 | MatchType::Bo3) => MatchType::Bo1,
    }
}
```

and the coercion becomes: keep the existing seat-count condition as is, then add `state.match_config.match_type = match_type_within(state.match_config.match_type, state.format_config.format.best_of_three_ceiling());` (only `match_type` is written, exactly as today; `loop_detection` untouched). The exhaustive match keeps the new-variant compile guarantee. Other edits:
- Section 6 and 7 text ("one method added", "`MatchType::exceeds`"): replace with "one private helper in `engine.rs`".
- Section 7 "Variant discoverability" grep: replace `git grep -n 'fn exceeds\|fn capped' -- crates/engine/src/types` with `git grep -n 'fn match_type_within\|fn exceeds' -- crates/engine/src`.
- Row R5: a private function cannot be called from `tests/integration`, and the "all four pairs" property is already covered by behavior. Replace R5 with legs inside R1 to R3: Dandan configured Bo1 stays Bo1 (pair Bo1/Bo1), Standard configured Bo1 stays Bo1 (pair Bo1/Bo3), plus R1 (Bo3/Bo1) and R3 (Bo3/Bo3). Each leg keeps its reach-guard (the state started). Revert: make the `(Bo1, _)` arm return `Bo3` and the Bo1 legs fail.
- Section 10: delete the `match_config.rs` row. Section 11: scope-path counts become 11 rows / 10 grouped paths / 6 authored source paths after F3 also removes the hook file (see F3 for the joint count); the closing sentence "The charter counted 9; the delta is the two added paths" becomes "The charter counted 9; the delta is the `main.rs` mod line already counted, so the plan adds no path outside the charter rule."
- Section 12 seam note for `engine.rs` is unchanged (the helper is outside `start_game_with_starting_player` but adjacent; the executor's `git diff HEAD PHASE_BASE` check applies to both).

### F3 [text] Home the hook in `HostSetup.tsx` and drop the new file from scope (sections 5.3, 5.5, 10, 11)
- Replace "New `client/src/hooks/useBestOfThreeCeiling.ts` (a path outside the charter's scope rule; justification in section 10)" with "`HostSetup.tsx` exports `useBestOfThreeCeiling` and `cappedMatchType` (both files that use them are chartered; `engineRuntime.ts` cannot host the hook because tests replace that module with a bare factory)". In 5.5 import them from `../components/lobby/HostSetup`.
- Section 7's grep path list: replace `client/src/hooks/useBestOfThreeCeiling.ts` with nothing (the two-file grep stays).
- Section 10: delete the hook row; `HostSetup.tsx` role becomes "control, `effectiveMatchType`, exported `useBestOfThreeCeiling` and `cappedMatchType`". Section 11 scope counts: 10 rows (removing both `match_config.rs` and the hook), 9 grouped paths (d.ts with `lib.rs`), 5 authored source paths: `engine.rs`, `lib.rs` (with binding), `engineRuntime.ts`, `HostSetup.tsx`, `GameSetupPage.tsx`. All counts below T2's 13; T1 = 1; the conjunction does not fire. The charter counted 9 (it listed `HostSetup.test.tsx` and `GameSetupPage.test.tsx` and the integration test plus `mod` line, which the plan also has); reconcile the plan's row count to the charter's by listing the same 9 paths, which now match exactly.
- If the lead rejects F3's relocation, this finding is withdrawn in favor of a charter decision revision adding the hook file; the rest of the plan is unaffected.

### F4 [behavior] `rememberHostConfig` stores the ceiling-capped match type, overwriting the remembered Bo3 (section 5.4)
Plan text (old string): "It flows unchanged to `useAiDeckCatalog({ selectedMatchType })`, `rememberHostConfig({ matchType })` and `onHost({ matchType })`, so a remembered Bo3 on a Dandan config (mount `useState(remembered?.matchType)`) submits Bo1."

Measured at HEAD: `HostSetup.tsx:724-744` stores `matchType: effectiveMatchType` in `rememberHostConfig`, and the mount state is `useState(remembered?.matchType ?? "Bo1")`. With the ceiling folded into `effectiveMatchType`, hosting one Dandan room writes Bo1 into the remembered host config, and so does hosting a Standard room before the ceiling answer resolves (the plan's own H3 case). The user's Bo3 choice is then lost on the next format, the exact leak the plan forbids for `GameSetupPage` (5.5 "the forced Bo1 is derived, never stored"). Today the stored value equals the raw one whenever it matters, because the seat-count resets already call `setMatchType("Bo1")` when `playerCount !== 2`.

Replacement: "`effectiveMatchType` flows to `useAiDeckCatalog({ selectedMatchType })` and `onHost({ matchType })`; `rememberHostConfig` stores the user's raw `matchType` (the seat-count resets already keep it Bo1 for a non-two-seat table), so a forced Bo1 is derived and never remembered." Add to H1 an assertion that `useMultiplayerStore.getState().lastHostConfig?.matchType` is still `"Bo3"` after hosting Dandan (revert: store `effectiveMatchType` and it reads `"Bo1"`), and to H3 that hosting before the answer resolves leaves the remembered value `"Bo3"`. Paired guard: H2, same seed on Standard, stored `"Bo3"`.

## Non-blocking notes (no plan change required; fix if convenient)

- N1 [note] R4's "reuse the archenemy fixture from `engine_mdfc_land_tests.rs`" cannot literally reuse it: that file is an inline `#[path]` test module of `engine.rs` (line 19016), not reachable from `tests/integration`. Its fixture is public-API only (`FormatConfig::archenemy()`, `config.archenemy_player = Some(PlayerId(2))`, `GameState::new(config, 4, 7)`), so the integration file re-creates those three lines. Replace "reuse the archenemy fixture from" with "build the same state as".
- N2 [note] Section 8.1 R6 asserts `loop_detection` survives; the authority claim "the host cannot raise the ceiling" is carried by R1 and R6 together, which is fine. `LoopDetectionMode::Interactive` exists (`game_state.rs:17857`), so the fixture compiles as planned.
- N3 [note] Before the ceiling resolves, `GameSetupPage`'s derived `effectiveMatchType` is Bo1 even for formats that admit Bo3, so a remembered Bo3 makes `MyDecks`/`AiOpponentConfig`/`useAiDeckCatalog` see Bo1 then Bo3 on first mount (two compat passes). Correct and fail-closed; the executor may note it in the report. No change.
- N4 [note] `CreateTournamentForm.tsx` offers every `FORMAT_REGISTRY` entry including Dandan with a Bo3 default (measured: lines 65, 204, 221-227), so a Dandan-labelled two-seat tournament can be created as Bo3 while every game plays as Bo1 (engine coercion). The plan already leaves it unedited and calls it display metadata, consistent with the charter; the PR description note the plan promises should say this explicitly. Not a finding against this phase.
- N5 [note] `validate_deck_list_seats(..., Some(state.match_config.match_type), ...)` in `initialize_game_impl` (lib.rs:1833) runs before the coercion, and the replay header records the uncoerced config (lib.rs:1934); replay reconstruction re-runs the same coercion, so the two agree. No change.

## Checks the lead asked for, with results
- Engine is the only authority: the coercion reads `state.format_config.format.best_of_three_ceiling()` (`format.rs:2329`); the export reads the `GameFormat` passed; the frontend compares the answer to `"Bo3"` and derives no rule. The plan's grep for `"Dandan"` in the three client files is a good guard. PASS.
- Fail-closed decode: helper answers Bo1 on `None`; `cappedMatchType` admits Bo3 only on `=== "Bo3"`, so an `undefined` or malformed export answer is also closed. `GameFormat` is a string on the client (`"Custom:N"` for custom formats, decoded by the engine's own `FromStr`), so the hook's format equality check is a plain string compare and Custom formats reach the engine decode. PASS, with F1 covering the synchronous-throw gap.
- No `ensureCardDatabase`: wrapper calls `ensureWasmInit()` only. PASS.
- Remembered `lastMatchType`: PASS for `GameSetupPage` (no setter added), FAIL for HostSetup's remembered host config (F4).
- Real-flow red row: `handle_game_over_transition` for Bo3 two-seat reaches `BetweenGames` (read, `match_flow.rs:234-293`), and R1 states the base red as an executor measurement. The Momir driver precedent exists (`momir_basic_emblem.rs:806`). PASS; the plan correctly labels it unprobed.
- Reach-guards: R1/R2, R3, R4, H1/H2, H3, H4, G1/G2 each carry a paired positive; H3's "BO3 button exists in the DOM" is the weakest and is strengthened by F1's H5. PASS with F1.
- Locale parity: no string added, no locale catalog edited. Dandan Bo3 is disabled with no explanatory text (recorded decision). PASS; `add-frontend-component` Phase 2.5 is satisfied by adding none, and its dispatch/WaitingFor phases are correctly marked N/A.
- Phase 6 seam (`engine-wasm/src/lib.rs`): the export is a free function beside `is_card_commander_eligible_for_format` (lib.rs:1105); `initialize_game_impl` and the `// CR 103.1: Start the game` marker are untouched. PASS. The d.ts is regenerated, not hand-edited.
- Sizing (blocking in this mode): the section is consistent with the body (one mechanic in lockstep layers); counts need the F2/F3 adjustment above.
- CR: 100.6a and 100.4 quoted in section 0 are real rules in `docs/MagicCompRules.txt` per the plan; the plan uses them as context only and says the Bo1 choice is product policy, which is accurate.
