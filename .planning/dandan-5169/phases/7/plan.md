# Phase 7 plan — S5: best-of-three ceiling enforced at the engine's match-structure authority; Dandan is best-of-one

Planned against HEAD 67cd221e (phase-plan mode). PHASE_BASE will be Phase 6's accepted candidate; Phases 4b, 5 and 6 land first. Every statement below about code was read at HEAD with `git grep` / `sed`; nothing was compiled or run (the orchestrator forbids cargo for this role), so each behavior row below is labelled **unprobed** and its red state is an executor measurement, not a planner fact.

## 0. Premise check (Step 0)

No card Oracle text is load-bearing in this phase. The one real-card dependency is that the Phase 6 provisioning builds the Dandan pile from real cards (Phase 6 plan V1); this phase reuses that fixture and reads no card's ability. CR text grepped in `docs/MagicCompRules.txt`:

- CR 100.6a: "A two-player match usually involves playing until one player has won two games. A multiplayer match usually consists of only one game."
- CR 100.4: "Each player may also have a sideboard ... to modify their deck between games of a match."

Neither rule obliges a format to offer a best-of-three structure; the coercion is product policy for a format with a fixed sideboard-less pile, annotated with those two rules as the context, not as a mandate.

## 1. Skills applied

`engine-planner` (phase-plan mode), `add-frontend-component` (Phase 2.5 i18n, Phase 7 component tests; no overlay, no `WaitingFor`, no `GameAction`, so its Phase 1/3/4/5/6 steps are N/A: stated here, not skipped), `add-engine-variant` (no variant is added: the six axis methods landed in Phase 2 and `engine.rs` gains one private exhaustive helper), `card-test` (the engine rows drive a match-flow path, not a cast pipeline; its recipe does not apply and the Momir driver the repo already uses is followed instead).

## 2. Goal

`start_game_with_starting_player` already coerces a Bo3 config to Bo1 when the seat count cannot serve the structure (and Archenemy exempts itself). It additionally clamps the configured match type to `state.format_config.format.best_of_three_ceiling()` (Phase 2's axis: Bo1 for Dandan, Bo3 for every other built-in and for `Custom(_)`). The lobby and the local setup page read the same axis through one wasm export and disable Bo3 from that answer.

## 3. Claims and their measurements

| # | Claim | Measured at HEAD (command) | Status |
|---|---|---|---|
| C1 | The between-games path cannot serve a Dandan game | Read: `handle_game_over_transition` enters `BetweenGames` for any two-player Bo3 (`match_flow.rs`); `bo3_sideboard_players` returns `[P0, P1]` for a non-Archenemy game; Phase 6 gives Dandan one pool (Phase 6 plan V1: `deck_pools.len() == 1`, `players[1].library` empty) so `handle_submit_sideboard`'s per-seat pool lookup and `deck_payload_from_current_pools` have nothing for P1 | **unprobed**; the red row R1 below is the measurement. The executor records the exact stall (which `Err` string, for which seat) in `executor-r1.md`; the plan asserts only that the match reaches `BetweenGames` at base and does not at the candidate |
| C2 | Every game-start entry passes `start_game_with_starting_player` | `git grep -n 'start_game_with_starting_player\|start_game(\|start_game_skip_mulligan' -- crates/engine-wasm/src crates/server-core/src crates/phase-server/src crates/engine/src ':!*test*'`: wasm `initialize_game_impl` (lib.rs, `Some(0)`/`Some(1)`/`_` arms after `state.set_match_config`), `server-core/session.rs::GameSession::start_game` -> `start_game`, `match_flow.rs::restart_between_games_with_starting_player`, `replay.rs`. `start_game` delegates to it on every branch (the Archenemy, contest and empty-seat branches). `start_game_skip_mulligan` has callers in `ai_tune` and tests only (`git grep -n start_game_skip_mulligan -- crates`) and stays outside the gate by design. No entry sets `match_config` after the start: `set_match_config` call sites are wasm init (before start), `session.rs::rebuild_pregame_state` (a fresh state before `start_game`), `match_flow.rs` restart (before the restart's start) and `replay.rs` (before start) | read, complete; executor re-runs the grep at PHASE_BASE and treats a new post-start setter as a finding |
| C3 | Momir Bo3 is unaffected | `GameFormat::Momir` is in the Bo3 arm of `best_of_three_ceiling` (`format.rs`, read); `momir_basic_emblem.rs::momir_emblem_creates_token_in_game_two_of_bo3` already drives Momir Bo3 through `Concede` -> `BetweenGamesSideboard` x2 -> `ChoosePlayDraw` -> `game_number == 2` | the existing test is the instrument; R2 reuses its driver |
| C4 | No serialized shape changes | the export returns a bare `"Bo1"`/`"Bo3"` string through `to_js`; no `types/` file is edited at all (the clamp is a private function in `engine.rs`) | executor runs `node scripts/check-protocol-version.mjs` green with no pin-file edit |
| C5 | Exactly two Bo3 controls, both seat-count-gated only | `git grep -n 'setMatchType(' -- client/src/components client/src/pages ':!*__tests__*'` returns HostSetup.tsx (mount `useState`, `applyResolvedFormat`, `handlePlayerCountChange`, the Bo1/Bo3 buttons), GameSetupPage.tsx (restore effect, `applyFormat`, player-count slider, the two buttons) and `tournament/CreateTournamentForm.tsx` (display metadata only; its format is not a game-setup control, noted for the PR description: a Dandan-labelled head-to-head tournament keeps its Bo3 default) | read; a further hit at PHASE_BASE is a finding |
| C6 | Other readers of a remembered/selected match type do not create a third control | `MenuPage.tsx` passes `lastMatchType` to a deck-compatibility warm only (`selectedMatchType` into `useAiDeckCatalog`/`MyDecks` inputs); `MultiplayerPage.tsx` consumes `settings.matchType` produced by HostSetup's `effectiveMatchType`; `GamePage.tsx` only decodes the `match` URL param into `match_type` | read. `MenuPage` stays unedited: a fixed-deck format supplies its decks, so the compatibility warm does not read the match type for Dandan |
| C7 | The ceiling relates to the existing "BO3 requires a sideboard" rule without contradicting it | `deck_validation.rs` computes `bo3_ready = !sideboard.is_empty()` for submitted decks; fixed-deck formats bypass it (Momir is sideboard-`Forbidden` yet Bo3-capable). The two authorities answer different questions: deck readiness versus the format's match structure. Dandan never reaches the readiness check (supplies a fixed deck), and the ceiling is applied at game start regardless of deck payload | read |

## 4. Step 2: analogous trace

Traced the existing Bo3-to-Bo1 coercion and its consumers end to end: `engine.rs::start_game_with_starting_player` (coercion) -> `match_flow.rs::handle_game_over_transition` (`match_type != Bo3` -> `Completed`) -> `restart_between_games_with_starting_player` -> wasm `initialize_game_impl` -> `GamePage.tsx` (`match` param -> `match_type`). For the client export, traced `isCardCommanderEligibleForFormat`: `engine-wasm/src/lib.rs` (`#[wasm_bindgen(js_name = ...)]`, `serde_wasm_bindgen::from_value::<GameFormat>`) -> generated `client/src/wasm/engine_wasm.d.ts` -> `client/src/services/engineRuntime.ts` wrapper -> consumers (`useDeckBuilder.ts`, `ImportDeckModal.tsx`, `LimitedDeckBuilder.tsx`) with tests mocking the wrapper (`vi.fn(async ...)`). For the page controls, traced HostSetup's `effectiveMatchType` -> `onHost({ matchType })` -> `MultiplayerPage` -> `multiplayerStore`, and GameSetupPage's `matchType` -> `&match=` -> `GamePage`.

## 5. Design

### 5.1 Engine

`MatchType` derives `PartialEq, Eq` and no `Ord` (read at `match_config.rs:6`), so the clamp is a typed exhaustive function. It is a private helper in `crates/engine/src/game/engine.rs` directly above `start_game_with_starting_player` (line 18422 at HEAD), part of that function's match-structure normalization; `match_config.rs` is not edited.

```rust
/// CR 100.6a + CR 100.4: the structure actually played; never longer than `ceiling` allows.
fn match_type_within(configured: MatchType, ceiling: MatchType) -> MatchType {
    match (configured, ceiling) {
        (MatchType::Bo3, MatchType::Bo3) => MatchType::Bo3,
        (MatchType::Bo3, MatchType::Bo1) | (MatchType::Bo1, MatchType::Bo1 | MatchType::Bo3) => MatchType::Bo1,
    }
}
```

Exhaustive, no wildcard: a new `MatchType` variant fails to compile here. In `start_game_with_starting_player` the existing seat-count condition is kept exactly as is (the `if state.match_config.match_type == MatchType::Bo3 && state.players.len() != 2 && super::topology::archenemy(state).is_none()` block), and one statement follows it:

```rust
state.match_config.match_type = match_type_within(
    state.match_config.match_type,
    state.format_config.format.best_of_three_ceiling(),
);
```

Only `match_type` is written (not `set_match_config`), exactly as today, so `loop_detection` is untouched. The ceiling is read from the format itself, so no host-supplied match config can raise it. Comments at implementation time are the helper's CR doc line plus one sentence at the call.

Every entry inherits it (C2). The between-games machinery is not edited: a Dandan match is `Completed` at game end through the existing `match_type != Bo3` arm of `handle_game_over_transition`, and `match_flow.rs` is deliberately unedited (architecture-scope file).

### 5.2 Wasm export

`crates/engine-wasm/src/lib.rs`, placed beside `is_card_commander_eligible_for_format` (and not inside `initialize_game_impl`, which Phase 6 edits):

```rust
#[wasm_bindgen(js_name = bestOfThreeCeilingForFormat)]
pub fn best_of_three_ceiling_for_format(format: JsValue) -> JsValue {
    to_js(&best_of_three_ceiling_or_bo1(serde_wasm_bindgen::from_value::<GameFormat>(format).ok()))
}

/// An undecodable format identifies no format whose ceiling admits Bo3, so it answers Bo1.
fn best_of_three_ceiling_or_bo1(format: Option<GameFormat>) -> MatchType {
    format.map_or(MatchType::Bo1, GameFormat::best_of_three_ceiling)
}
```

Fail-closed on decode failure (r4): the existing precedent answers `false` on an undecodable format, which for a ceiling would read as "no restriction" if the polarity were inverted; here the failure answer is the restrictive one. The pure helper exists because the `JsValue` shell panics natively (the file's own note at the test modules); it carries the native unit test. The export needs no card database (r4: do not copy the precedent's `ensureCardDatabase`). Serialized `MatchType` is the bare string `"Bo1"`/`"Bo3"` (derives `Serialize` with unit variants), matching the client `MatchType` type.

`client/src/wasm/engine_wasm.d.ts` is regenerated by `./scripts/build-wasm.sh wasm-dev` (CI byte-compares it); it is never hand-edited. It adds `export function bestOfThreeCeilingForFormat(format: any): any;` with the doc comment.

**Seam (engine-wasm lib.rs):** Phase 2 edited this file for the `GameFormat` mirrors, Phase 6 edits `initialize_game_impl`'s boot guard (the `filter(|p| p.library.is_empty())` call) and its Phase 6 source-census test slices `initialize_game_impl` by the `// CR 103.1: Start the game` marker. This phase adds a free function and a helper above or below `is_card_commander_eligible_for_format` and one test in the existing inline test module; it touches neither `initialize_game_impl` nor that marker. **Seam (generated d.ts):** Phases 5 and 6 may regenerate it; this phase regenerates it once at PHASE_BASE, so its diff is one added declaration (verify with `git diff` that no other declaration moved).

### 5.3 Client wrapper and one shared hook

`client/src/services/engineRuntime.ts`:

```ts
/** The engine's longest match structure for `format`; Bo3 must never be offered above it. */
export async function bestOfThreeCeilingForFormat(format: GameFormat): Promise<MatchType> {
  await ensureWasmInit();
  const engine = await loadEngineModule();
  return engine.bestOfThreeCeilingForFormat(format) as MatchType;
}
```

It calls `ensureWasmInit()` (the engine module must be initialized) and not `ensureCardDatabase()`. `MatchType` joins the existing `import type { ... } from "../adapter/types"`.

`HostSetup.tsx` exports `useBestOfThreeCeiling` and `cappedMatchType` (both files that use them are chartered; `engineRuntime.ts` cannot host the hook because tests replace that module with a bare factory, e.g. `src/__tests__/offlineLocalPlay.integration.test.tsx:251-258`, which renders `GameSetupPage`). Both controls need the same resolve-on-format-change, stale-answer guard and fail-closed handling; one export used by both avoids a duplicated ~20-line effect.

```ts
export function useBestOfThreeCeiling(format: GameFormat | null): MatchType | null
export function cappedMatchType(selected: MatchType, ceiling: MatchType | null): MatchType
```

- `useBestOfThreeCeiling` holds `{ format, ceiling } | null`. The effect calls `bestOfThreeCeilingForFormat(format)`, with a cancelled flag; a rejection (module failed to load, export missing) resolves to `"Bo1"` (fail closed). The effect invokes the wrapper inside an async function with try/catch (`void (async () => { try { ceiling = await bestOfThreeCeilingForFormat(format) } catch { ceiling = 'Bo1' } ... })()`), so a rejection, a missing export on a mocked module, and a synchronous throw all resolve to Bo1. The hook returns the stored ceiling only when its `format` equals the current `format`, otherwise `null`; so a format switch never reuses the previous format's answer and "neither control offers Bo3 before the answer has resolved" holds for the first render too. A `null` `format` resolves to `null` without a call.
- `cappedMatchType(selected, ceiling)` returns `selected` only when `ceiling === "Bo3"`, otherwise `"Bo1"`. The frontend compares the engine's answer to the one value that admits Bo3; it derives no rule (which format caps is engine knowledge).

### 5.4 `HostSetup.tsx`

- `const ceiling = useBestOfThreeCeiling(formatConfig.format);` next to `effectiveMatchType`.
- `effectiveMatchType = playerCount === 2 ? cappedMatchType(matchType, ceiling) : "Bo1"` (the existing seat-count term stays; the ceiling term composes with it). `effectiveMatchType` flows to `useAiDeckCatalog({ selectedMatchType })` and `onHost({ matchType })`; `rememberHostConfig` stores the user's raw `matchType` (the seat-count resets already keep it Bo1 for a non-two-seat table), so a forced Bo1 is derived and never remembered. This changes the `matchType: effectiveMatchType` entry in `rememberHostConfig` (HostSetup.tsx:726 at HEAD) to `matchType`; the `onHost` entry (line 744) stays `effectiveMatchType`. A remembered Bo3 on a Dandan config (mount `useState(remembered?.matchType)`) submits Bo1 and stays remembered as Bo3.
- The local `matchType` state is left alone when the format changes: the choice the user made for other formats is preserved, and the effective value is derived, not stored (the existing `setMatchType("Bo1")` resets on seat-count change are unchanged).
- The two buttons highlight from `effectiveMatchType` (today they read `matchType`, which would keep "Bo3" lit while Bo1 is submitted). The Bo3 button: `disabled={playerCount !== 2 || ceiling !== "Bo3"}` with the same `cursor-not-allowed opacity-40` class condition. Bo1 stays enabled.
- The `bo3Note` line stays keyed to `playerCount !== 2`.
- No new frontend-authored string (decision, section 9): an explanatory sentence would need a key in `en/multiplayer.json` and in all seven other locale catalogs (`src/i18n/__tests__/localeParity.test.ts` requires identical key sets), for a control that follows the existing disabled-and-dimmed pattern.

### 5.5 `GameSetupPage.tsx`

- `import { useBestOfThreeCeiling, cappedMatchType } from "../components/lobby/HostSetup";` then `const ceiling = useBestOfThreeCeiling(formatConfig?.format ?? null);` and `const effectiveMatchType = cappedMatchType(matchType, ceiling);` (the page's seat-count handling stays in its existing handlers).
- `handleStartAI` navigates with `&match=${effectiveMatchType.toLowerCase()}`; `MyDecks` and `AiOpponentConfig` receive `selectedMatchType={effectiveMatchType}` (their deck filtering must see the match structure that will be played). The Bo1/Bo3 buttons highlight from `effectiveMatchType`; Bo3 gets `disabled={playerCount !== 2 || ceiling !== "Bo3"}` with the existing dim class.
- Remembered-choice rule (r4): the forced Bo1 is derived, never stored. No `setMatchType` or `setLastMatchType` call is added; the restore effect's `setMatchType(restoredLastFormat ? lastMatchType : "Bo1")` keeps the remembered value, so choosing Dandan and later another format restores the user's Bo3. The test asserts `usePreferencesStore.getState().lastMatchType` is still `"Bo3"` after starting a Dandan match.
- Start-before-resolve: the answer resolves in one wasm round trip; a Start click that beats it navigates Bo1 (the fail-closed reading of the charter's "neither control offers Bo3 before the answer has resolved"); Start is not gated on it.

## 6. Building blocks

`GameFormat::best_of_three_ceiling` (Phase 2's axis), `topology::archenemy` (existing structure kept), `MatchType` (existing type reused; one private exhaustive helper in `engine.rs`), wasm `to_js` and the `isCardCommanderEligibleForFormat` export shape, `engineRuntime.ts::ensureWasmInit`/`loadEngineModule`, existing `seg(...)` button styling, `formatSuppliesDeck`-style registry reads (unchanged). No parser file changes: Nom Compliance is N/A.

## 7. Rust idioms and logic placement

The clamp is an exhaustive two-axis `match` on a closed enum, named for what it returns. The ceiling is read from the format (the engine) and compared in the engine; the wasm export is a thin serialization boundary whose only logic is the fail-closed `Option` default, kept in a pure testable helper. The frontend holds one typed async answer and compares it to the single admitting value; no format name appears in any TypeScript conditional (`git grep -n '"Dandan"' -- client/src/components/lobby/HostSetup.tsx client/src/pages/GameSetupPage.tsx` must be empty after the change).

**Pattern coverage (charter attribution).** The ceiling is a class axis: every present or future format whose `best_of_three_ceiling()` is Bo1 is capped at every entry and is offered no Bo3 control, with zero per-format code in this phase; today's population is Dandan, and any `Custom(_)` keeps the stock Bo3 answer.

**Extension vs creation.** Extends the existing coercion condition and the existing `isCardCommanderEligibleForFormat` export family; adds one exported hook in `HostSetup.tsx` because two components share the behavior.

**Variant discoverability.** No enum variant is added (`MatchType` keeps `Bo1`/`Bo3`; the helper is a private function). `git grep -n 'fn match_type_within\|fn exceeds' -- crates/engine/src` at PHASE_BASE must show no sibling before adding.

## 8. Verification matrix

Conventions: every negative has a paired positive reach-guard; "red" means the assertion fails when the named revert is applied; all red rows are executor measurements (**unprobed** here).

### 8.1 Engine: new `crates/engine/tests/integration/best_of_three_ceiling.rs` (+ `mod best_of_three_ceiling;` in `main.rs`)

Driver (shared by R1 to R3 and their Bo1 legs): `GameState::new(<format config>, 2, seed)`, `state.match_config.match_type = MatchType::Bo3`, load through `load_and_hydrate_decks` (Dandan: Phase 6's real-DB fixture exactly as Phase 6's V1 builds it, read at PHASE_BASE; Momir: the synthetic snow-basic DB of `momir_basic_emblem.rs`), `engine::game::engine::start_game`, `GameRunner::from_state`, `Concede { player_id: PlayerId(1) }`.

| Row | Claim | Setup and assertion | Revert and expected failure | Paired positive reach-guard |
|---|---|---|---|---|
| R1 (charter red row) | A Dandan Bo3 config plays as Bo1 and never enters the between-games path | After `start_game`: `match_config.match_type == Bo1`. After P1 concedes: `match_phase == MatchPhase::Completed`, `waiting_for` is `GameOver`, no `BetweenGamesSideboard` was ever the `waiting_for` | Drop the `match_type_within` statement: `match_type` stays `Bo3`, the concede lands in `BetweenGamesSideboard` (the executor records the subsequent submission `Err` for the seat with no pool as the C1 measurement) | R2: the same driver on Momir reaches `BetweenGamesSideboard` |
| R2 (Momir sibling) | A format whose ceiling admits Bo3 keeps it and restarts | Momir config, Bo3: `match_type == Bo3` after start; after the concede `waiting_for` is `BetweenGamesSideboard`; submit the fixed deck for both seats as the existing Momir test does, `ChoosePlayDraw`, `game_number == 2` (this is the restart, `restart_between_games_with_starting_player`, reached) | Clamp unconditionally to Bo1: R2's `Bo3` assertion fails | (it is the guard for R1) |
| R3 (ordinary two-seat sibling) | A two-seat non-shared format keeps Bo3 | `GameState::new_two_player`-style Standard (or Commander) two-player state, Bo3: `match_type == Bo3` after `start_game_with_starting_player(P0)` | Same clamp revert | the same test asserts the state started (`waiting_for` is the mulligan or priority prompt), so the start ran |
| R4 (existing structure preserved) | The seat-count term and the Archenemy exemption still behave | A three-player Commander state configured Bo3 starts Bo1 (seat-count term; reach guard: `players.len() == 3`). An Archenemy state configured Bo3 keeps Bo3 (the `archenemy(state).is_none()` exemption; build the same state as `engine_mdfc_land_tests.rs::archenemy_starting_life_and_first_turn_use_configured_archenemy` (an inline `#[path]` module of `engine.rs`, not reachable from `tests/integration`; its fixture is public-API only: `FormatConfig::archenemy()`, `config.archenemy_player = Some(PlayerId(2))`, `GameState::new(config, 4, 7)`); reach guard: `topology::archenemy(&state).is_some()`) | Remove the `archenemy(state).is_none()` conjunct: the Archenemy assertion fails | R2/R3 |
| R5 (Bo1 legs of the clamp) | A configured Bo1 stays Bo1 under both ceilings (pairs Bo1/Bo1 and Bo1/Bo3); together with R1 (Bo3/Bo1) and R3 (Bo3/Bo3) all four `match_type_within` pairs are covered by behavior (the helper is private and is not called from `tests/integration`) | Dandan configured Bo1: `match_type == Bo1` after start; Standard two-player configured Bo1: `match_type == Bo1` after start | Make the `(Bo1, _)` arm return `Bo3`: both Bo1 legs fail | each leg asserts the state started (`waitingFor` is the mulligan or priority prompt) |
| R6 (host cannot raise the ceiling) | The ceiling is read from `state.format_config.format`, not from the match config | R1 already sets Bo3 through the match config (the host-controlled input) on Dandan and asserts Bo1; this row additionally builds the Dandan state with `MatchConfig { match_type: Bo3, loop_detection: Interactive }` and asserts `loop_detection` is unchanged after start (only `match_type` is written) | write the whole config instead: `loop_detection` resets | R1 |

The wasm helper's native unit test (in `lib.rs`'s existing inline test module): `best_of_three_ceiling_or_bo1(Some(Dandan)) == Bo1`, `Some(Standard) == Bo3`, `Some(Custom(..)) == Bo3`, `None == Bo1`. Revert: make `None` answer Bo3 and the last assertion fails; the `Some(Standard) == Bo3` assertion is the reach guard that the helper reads the axis.

`cargo test` census: `format_axis_census.rs` is unedited and stays green (the export lives in `engine-wasm` and `engineRuntime.ts`, not in the two files `no_format_axis_key_reaches_the_client_mirror` scans).

### 8.2 Client component tests (export mocked at the `engineRuntime.ts` wrapper)

Each test file adds `vi.mock("<path>/services/engineRuntime", async () => ({ ...(await vi.importActual(...)), bestOfThreeCeilingForFormat: vi.fn(async (format) => format === "Dandan" ? "Bo1" : "Bo3") }))`. The mock is the stand-in for the engine answer; the engine's answer itself is R1/R5 plus the native helper test.

| Row | Claim | Assertion | Revert and expected failure | Paired positive reach-guard |
|---|---|---|---|---|
| H1 | HostSetup, Dandan with a remembered Bo3, disables Bo3 and submits Bo1 | Seed `useMultiplayerStore.lastHostConfig = { format: "Dandan", formatConfig: FORMAT_DEFAULTS.Dandan, playerCount: 2, matchType: "Bo3", ... }`; after the ceiling resolves (`await waitFor`), the BO3 button is disabled and Bo1 is the lit segment; click Host; `onHost` is called with `matchType: "Bo1"`; `useMultiplayerStore.getState().lastHostConfig?.matchType` is still `"Bo3"` | Restore `effectiveMatchType = playerCount === 2 ? matchType : "Bo1"` and the disabled expression to `playerCount !== 2`: `onHost` receives `"Bo3"`; store `effectiveMatchType` in `rememberHostConfig` and the remembered value reads `"Bo1"` | H2 |
| H2 | HostSetup, a non-shared format, leaves Bo3 enabled and submits it | Same seed with `format: "Standard"` (or Commander), `matchType: "Bo3"`: BO3 enabled, `onHost` gets `"Bo3"`, remembered `matchType` is `"Bo3"` | the gate reading `ceiling !== "Bo3"` unconditionally false, or the hook never resolving: Bo3 not submitted | (is the guard for H1; also proves the hook resolved to Bo3) |
| H3 | Unresolved and fail-closed answers offer no Bo3 | With the mock returning a never-resolving promise: BO3 disabled and `onHost` gets `"Bo1"` for a Standard remembered Bo3 (reach guard: the BO3 button exists in the DOM), and hosting before the answer resolves leaves the remembered `lastHostConfig.matchType` `"Bo3"`. With the mock rejecting: same | make `cappedMatchType` return `selected` for `null` | H2 after resolution |
| H4 | Switching format does not reuse the previous answer | Start on Standard (Bo3 enabled after resolve), pick Dandan in the Format menu: Bo3 disabled before and after the Dandan answer resolves; pick Standard again: the user's Bo3 choice is still selected (state preserved) once it resolves | drop the `format` equality check in the hook: Bo3 stays enabled for the tick after the switch | the Standard leg |
| H5 | Fail-closed on a synchronous throw (the shape of a factory mock that omits the export) | `vi.mocked(bestOfThreeCeilingForFormat).mockImplementation(() => { throw new Error("missing export"); })`, Standard remembered Bo3: render does not throw, BO3 disabled, `onHost` gets `"Bo1"` | Call the wrapper synchronously in the effect body (no async try/catch): the render throws | H2 (same seed, normal mock, Bo3 enabled) |
| G1 | GameSetupPage, Dandan with a remembered Bo3, disables Bo3 and navigates `match=bo1` | `usePreferencesStore.setState({ lastFormat: "Dandan", lastPlayerCount: 2, lastMatchType: "Bo3" })`, render, wait for the BO3 button disabled, click Start Match (Dandan supplies its deck, no active deck needed); the routed location search contains `match=bo1`; `lastMatchType` in the store is still `"Bo3"` | Restore the `match=${matchType...}` navigation: `match=bo3` | G2 |
| G2 | GameSetupPage, a non-shared format, leaves Bo3 enabled and navigates `match=bo3` | `lastFormat: "Standard"` (or Commander), `lastMatchType: "Bo3"`, an active deck seeded with the file's `seedDeck`/`setActiveDeck`: BO3 enabled, Start navigates with `match=bo3` | as G1 | (guard for G1) |
| G3 | The existing `?format=TwoHeadedGiant` and the Bo3-remembered 2HG restore tests still pass (seat-count gate preserved) | existing tests in `GameSetupPage.test.tsx` | n/a | n/a |

To observe the navigation, G1/G2 replace the `/game/:id` route element with a component printing `useLocation().search` (the file's `renderGameSetupPage` builds the routes; the helper gains an optional probe element rather than a second render function). Test files that render these components without mocking the wrapper (`MultiplayerPage.*`, `GameSetupPage.startingLife/.visualPacks/.deckPlayed`) and `src/__tests__/offlineLocalPlay.integration.test.tsx`, whose factory mock of `engineRuntime` omits the export, exercise the fail-closed path; the executor runs all of them (added to the step 8 vitest list) and confirms no unhandled error and no changed assertion. The census is `git grep -ln 'services/engineRuntime' -- 'client/src/**/__tests__/*'` intersected with the test files that render `GameSetupPage` or `HostSetup`, re-run at PHASE_BASE. H5 is the in-file reach-guard for the synchronous-throw shape. `git grep -n 'Bo3' -- client/src/pages/__tests__/MultiplayerPage.*.tsx client/src/pages/__tests__/GameSetupPage.*.tsx` returns only `GameSetupPage.test.tsx:188`, a restore of 2HG.

Client commands (allowed): `pnpm exec tsc -b --noEmit`; `pnpm exec vitest run --coverage.enabled=false src/components/lobby/__tests__/HostSetup.test.tsx src/pages/__tests__/GameSetupPage.test.tsx src/__tests__/offlineLocalPlay.integration.test.tsx src/i18n/__tests__` plus the `MultiplayerPage.*` and `GameSetupPage.*` test files from the census.

### 8.3 Reference Readings

- R2 and R3 are preservation rows ("equals base"). Derived reading: CR 100.6a says a two-player match usually runs to two wins and CR 100.4 permits sideboarding between games, so a two-seat format with a per-seat pool entering a sideboard prompt after game one is the derived behavior. Measured reference: `momir_basic_emblem.rs` already asserts the Momir restart reaches `game_number == 2`, and R2 reuses its driver. The Dandan reading (Bo1) is a product decision, not a CR consequence (CR 100.6a says "usually"), stated as such.
- H2/G2 copy the pre-change behavior for non-shared formats; derived from the charter (Bo3 for every other format), measured by the pre-change seat-count-only gate read in section 3 C5.
- No row copies a sibling route's result without derivation.

### 8.4 Identity and provenance

The ceiling's authority is `state.format_config.format` (engine side) and the `GameFormat` value passed to the export (client side). Bound at game creation (the format is part of `GameState::new`'s config and immutable), read live at each start (including each restart, where it is a no-op because Bo3 cannot reach a Bo1-ceiling format). The hostile multi-authority fixture is R1/R6: the host-controlled match config says Bo3 and the format says Bo1; the format wins.

## 9. Decisions and what is not done

- **No explanatory string** beside the disabled Bo3 control (locale parity cost; see 5.4). Stated as a decision, not deferred.
- **Start is not gated on the ceiling resolve** (5.5).
- **`MenuPage.tsx`, `multiplayerStore.ts`, `CreateTournamentForm.tsx` unedited** (C5/C6): a persisted Dandan host session can only carry Bo1 once the controls land, and the engine coercion covers gameplay regardless.
- **Lobby listing**: a room hosted by a modified client can advertise Bo3 for a Dandan room until the game starts; the game itself is Bo1 (engine coercion). The lobby/broker frames are not edited (no serialized shape change in this phase).
- Charter deferral list: none. The shared-zone-aware restart is the charter's rejected alternative and is not built.

## 10. Scope matrix (literal paths)

| Path | Role | In charter scope rule? |
|---|---|---|
| `crates/engine/src/game/engine.rs` | private `match_type_within` helper above, and the clamp statement in, `start_game_with_starting_player` (the match-structure normalization only) | yes |
| `crates/engine/tests/integration/best_of_three_ceiling.rs` (new) | R1 to R6 | yes (new integration file) |
| `crates/engine/tests/integration/main.rs` | `mod best_of_three_ceiling;` | yes |
| `crates/engine-wasm/src/lib.rs` | export, helper, native helper test | yes |
| `client/src/wasm/engine_wasm.d.ts` | regenerated binding (groups with `lib.rs`) | yes |
| `client/src/services/engineRuntime.ts` | wrapper | yes |
| `client/src/components/lobby/HostSetup.tsx` | control, `effectiveMatchType`, raw `matchType` in `rememberHostConfig`, exported `useBestOfThreeCeiling` and `cappedMatchType` | yes |
| `client/src/components/lobby/__tests__/HostSetup.test.tsx` | H1 to H5 | yes |
| `client/src/pages/GameSetupPage.tsx` | control, `match` navigation | yes |
| `client/src/pages/__tests__/GameSetupPage.test.tsx` | G1, G2 | yes |

Explicitly unedited: `crates/engine/src/types/match_config.rs`, `crates/engine/src/game/match_flow.rs`, `crates/phase-server/src/main.rs`, `crates/lobby-broker/src/protocol.rs`, `crates/server-core/src/protocol.rs`, `scripts/check-protocol-version.mjs` (no pin edit), `client/src/i18n/locales/**`.

Standing inclusion classes (compiler-forced sites, comment-only edits) apply; none is expected.

## 11. Sizing

- **Units: 1.** One mechanic: the format's best-of-three ceiling enforced at the engine's match-structure authority and displayed on the two controls. Registration surfaces: the private clamp helper and its call, the wasm export with its generated binding, the wrapper, the exported hook, the two controls. Discriminating tests: R1 (engine red at base) and H1/G1 (client, revert-failing). Inter-unit dependency edges: none inside the phase; it consumes Phase 2's axis (landed) and Phase 6's Dandan provisioning (PHASE_BASE) for the R1 fixture.
- **Independently tested behaviors in the body:** the engine coercion (R1 to R6), the wasm helper's fail-closed default, the two controls (H, G). These are lockstep layers of the one mechanic (engine answer, transport, display), not separate skill passes.
- **Scope-path count (phase-fit rule):** 10 rows above; grouped (d.ts with `lib.rs`) 9 paths, matching the charter's 9 exactly; excluding the tests and the `mod` line, 5 authored source paths (`engine.rs`, `lib.rs` with its binding, `engineRuntime.ts`, `HostSetup.tsx`, `GameSetupPage.tsx`). All counts are below T2's 13. T1 = 1. The conjunction does not fire. The plan adds no path outside the charter's scope rule.

## 12. Seams

- `crates/engine-wasm/src/lib.rs`: Phase 2 (mirrors), Phase 6 (`initialize_game_impl` boot guard and its source-census test that slices that function) and this phase (a new export plus helper plus a test in the inline module); disjoint functions, but the executor re-reads the file at PHASE_BASE before editing and adds the export as one contiguous block.
- `client/src/wasm/engine_wasm.d.ts`: generated; Phases 5 and 6 may have regenerated it. Regenerate after editing `lib.rs`, then confirm the diff is the one added declaration.
- `crates/engine/src/game/engine.rs`: Phases 4b/5/6 do not edit `start_game_with_starting_player` (charter seam note); the executor confirms at PHASE_BASE with `git diff HEAD PHASE_BASE -- crates/engine/src/game/engine.rs` restricted to that function and the lines directly above it where the helper goes.
- Protocol: no bump in this phase; Phase 5 bumped 93 to 94 / wire 75 to 76 and Phase 2's lobby 15 stay as they are.

## 13. Steps (executor order)

1. Re-run C2 and C5 greps at PHASE_BASE; `git diff` the `engine.rs` start function and the `lib.rs` neighborhood of the new export.
2. `engine.rs`: add `match_type_within` above `start_game_with_starting_player`.
3. `engine.rs`: add the clamp statement after the existing seat-count block (5.1) with its one-sentence comment (CR 100.6a, CR 100.4 grepped above).
4. New integration file and `mod` line: R1 to R6; run R1 and R2 against the unmodified engine first to record the base red (R1) and green (R2), then apply steps 2 and 3.
5. `lib.rs`: export, pure helper, native helper test. Regenerate `engine_wasm.d.ts` with `./scripts/build-wasm.sh wasm-dev`.
6. `engineRuntime.ts`: wrapper.
7. `HostSetup.tsx` (hook and `cappedMatchType` exports, controls, raw `matchType` in `rememberHostConfig`) then `GameSetupPage.tsx` edits (5.4, 5.5); tests H1 to H5, G1, G2 (red with the revert, then green).
8. `cargo fmt --all`; targeted `cargo test`/nextest for `best_of_three_ceiling`, `momir_basic_emblem`, `format_axis_census`, the `engine-wasm` helper test; `node scripts/check-protocol-version.mjs`; `pnpm exec tsc -b --noEmit`; the vitest files above (including `offlineLocalPlay.integration.test.tsx`).
