# Phase 7 executor report r1

Mode: implementation/fix (phase mode). BASE_SHA = START_SHA = 1b112fdec97f7e244c7aabdab9e8591b9a77c196. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Worktree record: start clean, HEAD == START_SHA, nothing staged; end HEAD unchanged, nothing staged, unstaged delta = exactly the 10 scope paths. All results below are PREPARATORY, not completion evidence.

## Diff summary
- crates/engine/src/game/engine.rs: private exhaustive `match_type_within` above `start_game_with_starting_player`; one clamp statement after the seat-count block (reads `state.format_config.format.best_of_three_ceiling()`, writes only `match_type`).
- crates/engine/tests/integration/best_of_three_ceiling.rs (new) + main.rs mod line: R1-R6.
- crates/engine-wasm/src/lib.rs: `bestOfThreeCeilingForFormat` export, pure fail-closed helper `best_of_three_ceiling_or_bo1`, native test in `external_format_config_tests`.
- client/src/wasm/engine_wasm.d.ts: regenerated with `./scripts/build-wasm.sh wasm-dev` (see Risks: one extra doc hunk).
- client/src/services/engineRuntime.ts: wrapper (`ensureWasmInit`, not card DB).
- client/src/components/lobby/HostSetup.tsx: exported `useBestOfThreeCeiling`, `cappedMatchType`; `effectiveMatchType` composes seat count with ceiling; `rememberHostConfig` stores raw `matchType`; Bo3 `disabled` and both highlights use the effective value.
- client/src/pages/GameSetupPage.tsx: same hook; `match=` navigation, `MyDecks`/`AiOpponentConfig` `selectedMatchType`, button highlight/disable use the effective value; no `setMatchType`/`setLastMatchType` added.
- Tests: HostSetup.test.tsx (H1-H5), GameSetupPage.test.tsx (G1, G2).

## Step-0 re-measurement
- C2: entries unchanged (wasm `initialize_game_impl`, `start_game` branches, `match_flow` restart, `replay.rs`); `set_match_config` callers: wasm init, server-core session x2, match_flow restart, replay: all before start. No new post-start setter.
- C5: `setMatchType(` hits: HostSetup (mount, 2 seat-count resets, Bo1/Bo3 buttons), GameSetupPage (restore, applyFormat, slider, buttons), CreateTournamentForm (display only). No third control.
- No `match_type_within`/`exceeds` sibling existed.
- Tests that mock/render these components and mention engineRuntime: only HostSetup.test and GameSetupPage.test (census `git grep -ln 'services/engineRuntime' -- 'src/**/__tests__/*'`); the other MultiplayerPage/GameSetupPage/offlineLocalPlay files ran unmocked (fail-closed path) green.
- C1 measured at base (Dandan Bo3, concede game 1): `match_phase == BetweenGames`; first sideboard prompt is P0, submitting the full Dandan list is accepted (`pools=[P0]`, libs=[66,0]); the prompt for P1 then returns `Invalid action: Deck pool not found for player` on every retry (stall). Probe test removed after measuring.

## Discriminating-test gate (production-path coverage map)
| claim | seam | entry | test | revert-failing assertion | siblings |
|---|---|---|---|---|---|
| Dandan Bo3 plays as Bo1, never BetweenGames | clamp in `start_game_with_starting_player` | `start_game` + `load_and_hydrate_decks` + `Concede` via `GameRunner::act` | R1 | `match_type == Bo1` and `match_phase == Completed` (base run: R1 FAILED, probe above) | R2 (Momir reaches `game_number == 2`), R3, R5 |
| ceiling read from format, only `match_type` written | clamp statement | `set_match_config(Bo3, Interactive)` then start | R6 | base run: R6 FAILED (match_type stayed Bo3); `loop_detection` assertions guard a whole-config write | R1 |
| Bo3-admitting formats keep Bo3 | `(Bo3,Bo3)` arm | Momir, Standard 2-seat | R2, R3 | clamp-to-Bo1 would flip both (passed at base and after) | R1 |
| seat-count term and Archenemy exemption preserved | existing block | 3p Commander, 4p Archenemy | R4 | passed at base and after (preservation row) | |
| configured Bo1 stays Bo1 | `(Bo1,_)` arms | Dandan, Standard | R5 | passed at base and after | |
| wasm answer fails closed | `best_of_three_ceiling_or_bo1` | native unit test | `best_of_three_ceiling_reads_the_format_axis_and_fails_closed` | `None -> Bo3` made the test fail (run) | Standard/Custom -> Bo3 reach |
| HostSetup Dandan + remembered Bo3 | `effectiveMatchType`, `bo3Disabled`, `rememberHostConfig` | click Host Game | H1 | restoring seat-count-only gate: H1 fails (run) | H2 (Standard enabled, submits Bo3, remembered Bo3) |
| unresolved / rejected / sync-throw => no Bo3 | hook try/catch + null | never-resolving, rejecting, throwing mock | H3, H5 | gate revert fails H3, H5; removing try/catch: vitest Unhandled Rejection, run rc=1 (tests themselves still green) | H2 |
| format switch never reuses previous answer | hook `format` equality | Format menu Standard->Dandan->Standard | H4 | dropping equality: H4 fails (run) | |
| GameSetupPage Dandan + remembered Bo3 | `effectiveMatchType`, `bo3Disabled`, navigation | Start Match, `useLocation().search` | G1 | gate revert: G1 fails (run) | G2 |

Deviation: G2 uses Momir (ceiling Bo3, supplies a deck) instead of Standard because the page's start gate needs legal AI decks and the file's mocked catalog returns none; Momir is the exact fixed-deck twin of Dandan. R1 uses `expect` on the card fixture rather than the file-wide early-return-on-None convention, so it cannot pass vacuously. R4's Archenemy reach guard is `format_config.archenemy_player == Some(P2)` because `topology::archenemy` is `pub(crate)`.

## Maintainer-simulation matrix
| claim | entry / first branch reached | authority | bound value, when | mode | storage | consumers | invalidation | hostile fixture | serde impact |
|---|---|---|---|---|---|---|---|---|---|
| ceiling clamp | `start_game_with_starting_player` -> after the seat-count `if` | `state.format_config.format` | `MatchType`, at each start (restart included, no-op there) | live read of an immutable format | `state.match_config.match_type` | `handle_game_over_transition` (`match_type != Bo3` => Completed) | none: format immutable | R1/R6: host config Bo3, format Bo1, format wins | none (no types/ file edited) |
| wasm export | `best_of_three_ceiling_for_format` -> `from_value::<GameFormat>` `Some` / `None` | engine axis | `"Bo1"`/`"Bo3"` string | live | none | `engineRuntime.bestOfThreeCeilingForFormat` | decode failure => Bo1 | native test `None` leg | d.ts one declaration |
| client controls | hook effect on `format` change; `cappedMatchType` | engine answer | `{format, ceiling}` | per-format latch, returned only when `format` matches | hook state | HostSetup, GameSetupPage | stale/unresolved/rejected => `null`/Bo1 | H3, H4, H5 | none |
No incomplete rows; no DEFERRED rows (charter deferral list is none).

## PREPARATORY verification (direct cargo; no Tilt in this checkout)
- `cargo test -p phase-engine --test integration -- best_of_three_ceiling momir_basic_emblem format_axis_census no_top_level_test_binaries`: 31 passed.
- `cargo test -p engine-wasm`: 54 passed.
- `cargo clippy -p phase-engine -p engine-wasm --all-targets -- -D warnings`: clean.
- `cargo fmt` on the three Rust paths: clean.
- `node scripts/check-protocol-version.mjs`: rc 0, silent; no pin file edited. `./scripts/check-interaction-bindings.sh --check`: rc 0.
- vitest (HostSetup, GameSetupPage x4, MultiplayerPage x7, offlineLocalPlay.integration, src/components/lobby, i18n): 20 files, 378 tests green.
- `pnpm exec tsc -b --noEmit --force`: ONE error, pre-existing at START_SHA: `src/data/formatRegistry.ts(671,7) TS2353 'allow_experimental_dungeons' does not exist in type 'FormatConfig'` (the Dandan registry entry carries a key removed by upstream #9476; file is outside the 10 scope paths). Nothing in my diff touches it.
- Parser preparatory gate: no file under crates/engine/src/parser/ touched; N/A.
- CR gate: only CR 100.4 and CR 100.6a added; both grep-verified in docs/MagicCompRules.txt and describe the annotated line (100.6a: two-player match usually to two wins, multiplayer usually one game; 100.4: sideboard between games). No UNVERIFIED.
- `git grep -n '"Dandan"'` on HostSetup.tsx and GameSetupPage.tsx: empty (no format name in TS conditionals).

## New-field threading sweep
No field added to any enum or struct.

## Stop-and-return items
1. `client/src/data/formatRegistry.ts` line with `allow_experimental_dungeons: false` in the Dandan entry breaks `tsc -b` (merge artifact of upstream #9476 against the Phase 2 entry). Out-of-list path; fix is deleting that one line. It must land before any candidate gate that includes `check-frontend`.

## Judgement calls / risks
- d.ts regeneration produced three hunks: the new declaration, its `InitOutput` entry, and a doc comment change on `get_viewer_transition_snapshot_js` that already differs between the committed d.ts and the `lib.rs` doc at START_SHA (pre-existing staleness; CI byte-compares, so the regenerated text is the correct committed content).
- Remembered `matchType` in HostSetup is now the raw state, not seat-count-derived, per plan 5.4; a Discord seed with 3+ seats now persists a raw Bo3 that is still submitted as Bo1.
- Test noise: HostSetup.test logs pre-existing `ECONNREFUSED ::1:3000` AggregateErrors (no unhandled-error failure).
- Logs under .planning/dandan-5169/p7-*.log are scratch; do not commit.
