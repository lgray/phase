# Phase 2 executor report, round 1 (implementation/fix, phase mode)

MODEL: claude-sonnet-5-5
Mode: implementation/fix (phase mode). BASE_SHA(run)=b9ba9360. START_SHA=2cf3d3aae0aeb555eae93c7b230cf35612b41100. IMPLEMENTATION_WORKTREE=/home/user/phase.
Start check: HEAD == START_SHA, `git status` empty, nothing staged. End check: HEAD unchanged (2cf3d3aa), 0 staged, unstaged delta = exactly the 18 scope paths (18 files, +776 / -51). Everything below is PREPARATORY evidence, not completion evidence.

## 1. Diff summary (+/-)
- engine `types/format.rs` +483/-9: variant `Dandan` (after `FreeformCommander`), FromStr/Display, 14 forced arms, `label`, `for_format`, `registry()` entry, `FormatConfig::dandan()`, five axis types (`ZoneScope`, `SharedZones`, `DealOrder`, `FreeRevealMulligan`, `HandEntryOwnership`, `OpeningHandEquivalence`) and six axis methods (`shared_zones`, `deal_order`, `free_reveal_mulligan`, `hand_entry_ownership`, `opening_hand_equivalence`, `best_of_three_ceiling`), 2 new tests + Dandan added to `no_commander_per_count`.
- engine `game/deck_validation.rs` +87/-2: Dandan joins the constructed arm group in both dispatchers; new test `dandan_deck_compatibility_reads_the_format_axes_on_both_paths`.
- engine `types/custom_format.rs` +7/-6: `from_source_format` arm; comment and error text name Dandan's shared zones.
- `engine-wasm/src/lib.rs` +2/-1: arm in `is_card_commander_eligible_for_format`.
- integration tests: `custom_format_schema.rs` +21 (two ordered tables + `from_lobby_config_rejects_dandan_source`), `format_axis_census.rs` +17/-1 (six methods in wildcard census, six keys in mirror census).
- protocol: `lobby-broker/src/protocol.rs` +16/-10 (PROTOCOL 92->93, LOBBY 14->15, doc entries, tests, chain test renamed `..._four_through_fifteen`), `server-core/src/protocol.rs` +7/-3 (test renamed `protocol_version_is_93_for_dandan_format`), `scripts/check-protocol-version.mjs` +8/-3, client `ws-adapter.ts` +22/-2 (93, 15, `MIN_LOBBY_PROTOCOL_FOR_DANDAN = 15`, switch arm, doc entries), `network/protocol.ts` +5/-1 (wire 75 + entry), `network/__tests__/protocol.test.ts` +21/-2 (v75 pin + floor describe), `p2p-adapter-multiplayer.test.ts` +7/-7 (v74 refused / v75 admitted, comment numerals).
- client mirrors: `types.ts` +2/-1, `formatRegistry.ts` +26, `deckUrlImport.test.ts` +1; card-bot `formats.ts` +1.
- `manabrew-compat/src/lib.rs` +43/-3: refusal guard, registry entry `upstream.shared-zone-ownership-missing` (94 entries), test `prepare_snapshot_refuses_a_format_with_shared_zones`, two `93->94` asserts.

## 2. PREPARATORY verification (commands, all cargo `CARGO_BUILD_JOBS=2`; logs under phases/2/)
- `cargo fmt --all`: run, clean.
- `cargo check -p phase-engine -p engine-wasm`: green (check1.txt); confirms the census of 18 forced arms was complete (no further arm needed).
- `cargo clippy -p phase-engine -p engine-wasm -p manabrew-compat -p lobby-broker -p server-core --all-targets -- -D warnings`: green (clippy.txt).
- `cargo nextest run -p phase-engine -E 'types::format:: | deck_validation | custom_format | format_axis_census | dandan'`: 343/343 pass (nextest-engine.txt), including `client_builtin_game_format_union_matches_the_engine` and `client_format_registry_matches_the_engine_registry`.
- `cargo nextest run -p manabrew-compat -p lobby-broker -p server-core -E 'capabilit|prepare_snapshot|protocol'`: 140/140 pass (nextest-rest.txt).
- `node scripts/check-protocol-version.mjs`: exit 0 (92/14/74 -> 93/15/75; draft 30 unchanged).
- `scripts/check-interaction-bindings.sh --check`: exit 0 (bindings.txt); no generated binding changed.
- `bun test scripts/card-bot`: 158 pass. Client: `pnpm exec tsc -b --noEmit` exit 0; vitest (protocol, p2p-adapter-multiplayer, deckUrlImport, brokerClient) 275 pass; eslint on all changed client files: clean.
- `cargo engine-inventory`: `GameFormat` variants end `Freeform, FreeformCommander, Dandan, Custom`; the five new enums exist with no sibling-cluster smell.
- Environment note: the box hit "No space left on device" twice (target/ ~25 GB on a ~38 GB usable disk). I deleted only stale incremental caches and stale duplicate `deps/` artifacts of my own mutation builds (no `cargo clean`, no whole-target delete) and built the later runs with `CARGO_INCREMENTAL=0`. Disk is at ~92%; a later full build may need the same.

## 3. Parser gate
N/A: no file under `crates/engine/src/parser/` changed.

## 4. Discriminating-test gate: production-path coverage map (red shown by reverting, then green)
| claim | seam | entry | test | assertion flipped when reverted | negative/reach-guard |
|---|---|---|---|---|---|
| axis answers only Dandan (Momir for equivalence) | six axis methods | `GameFormat::<axis>()` | `types::format::tests::format_axes_depart_from_the_stock_answers_only_where_declared` | RED with `shared_zones(Dandan)` set to `NONE`: deviating set empty != `[Dandan]` | each axis observes stock and deviating sets non-empty, Custom(0) in walk, `len >= 26` |
| config/registry | `dandan()`, `registry()` | `for_format`, `registry()` | `dandan_config_and_registry_entry` | RED with `deck_size` `Exactly(60)` | Momir struct-update control differs on exactly format/deck_size/command_zone |
| Custom-save refusal | `has_unrepresentable_auxiliary_deck_component` | `CustomFormatDef::from_lobby_config` | `custom_format_schema::from_lobby_config_rejects_dandan_source` | RED with Dandan arm `false` (config accepted) | `FormatConfig::standard()` saves |
| deck verdicts read axes | two deck_validation arms | `evaluate_deck_compatibility` (summary_only false and true) | `dandan_deck_compatibility_reads_the_format_axes_on_both_paths` | RED with Dandan copy limit `UpTo(4)` (80 copies refused) | 80-copy list passes; 79, sideboard, commander, unknown card each refused on both paths |
| serde/FromStr, order | FromStr/Display, enum order | serde | existing `game_format_serialization_is_byte_identical...`, `commander_eligibility_rule_from_source_format_covers_every_builtin` (Dandan rows added; ordered iter equality) | red if the arm is missing or the variant misplaced (compile/ordered-table); not separately mutation-run | 26 built-ins |
| no wildcard / no client key | six methods | source scan | `format_axis_census::*` (six headers, six keys added) | existing non-vacuity guards; not separately mutation-run | |
| floor wired | `lobbyProtocolRequiredForFormat` | vitest `protocol.test.ts` describe "lobby capability floor for the Dandan format" | RED with `case "Dandan"` deleted (red-floor.txt: 1 failed) | Freeform and Standard rows in same describe |
| P2P wire pinned | `WIRE_PROTOCOL_VERSION` | `protocol.test.ts` v75 pin, p2p gate test | RED with const at 74 (red-wire.txt: 2 failed; script also red) | refuses v74 / admits v75 literals |
| protocol pins agree | E7 table | `check-protocol-version.mjs` | RED with lobby `PROTOCOL_VERSION` back at 92: "Protocol version mismatch: Rust=92, client=93" (red-script.txt) | |
| card-bot/client mirror | `formats.ts` | `formats.test.ts` mirror equality | RED with Dandan row removed (157 pass / 1 fail: deep-equals the registry's projection) | |
| manabrew refusal | guard in `prepare_snapshot_with_prompt_id` | `prepare_snapshot` | `prepare_snapshot_refuses_a_format_with_shared_zones` | RED with guard `&& false`: `unwrap_err()` on `Ok` (nextest-red-mb.txt) | same two-player state prepares under stock format; code present in the registry |
| wasm arm | match arm | compile-forced | `cargo check`/clippy green | | (no runtime test possible, as planned) |
All Rust red legs were one mutation build (engine, 4 tests: 4 failed / 0 passed), format.rs restored byte-identical (`cmp` against backup), then the 343-test green run preceded it and the 140-test run followed. Seams whose consumers are charter-deferred (booted Dandan game, each axis's consumption) are `DEFERRED(phase 6/3/7/11/12/13/14)` per the plan; nothing else unmapped. No shape-only test stands in for a runtime claim.

## 5. Maintainer-simulation matrix (no runtime behavior changes; rows condensed)
- Axis answers: entry `state.format_config.format.<axis>()` (no consumer yet: `DEFERRED(phase 3/6/7/11/12/13/14)` per consumer); authority is the latched `GameFormat` value (not a `FormatConfig` field, so no payload can carry a different answer); binding at game start, live match on a `Copy` enum; storage = the enum discriminant in `GameState.format_config.format`; invalidation N/A; hostile fixtures: `Custom(0)` and Momir in the axes test; serde impact: none (axis types deliberately not `Serialize`).
- `Dandan` variant: serialized as its Display string (wire change) -> full-game 93, lobby 15 with frozen client floor 15, P2P 75; `ability_scan`/interaction bindings/card-data do not name it (bindings `--check` green).
- Custom-save guard: entry `from_lobby_config`, first branch = `has_unrepresentable_auxiliary_deck_component`; hostile = Dandan config (command_zone false, would otherwise fall to Disabled); shown red.
- manabrew guard: reads `raw_state.format_config.format`; invalidation N/A.
- Incomplete rows: none; none foreclosed by this phase's code.

## 6. New-field threading sweep
No field added to an existing struct/variant. New `GameFormat::Dandan` variant: every construction/consumption site is the 18 compiler-forced matches plus `FormatConfig::for_format`/registry; workspace `cargo check` (phase-ai, phase-llm, seat-reducer, engine-wasm included) found no further arm. `FormatConfig` gains no field (`defaults intentionally because` axes are methods, not stored).

## 7. CR-annotation diff gate
Grep of every `CR n` in `git diff`: zero `UNVERIFIED` (all present: 100.2a, 100.4, 100.6a, 103.4, 103.5, 103.5c, 108.3, 109.4c, 110.2, 114.1, 121.2c, 400.1, 400.3). Descriptions checked against plan Step 0 wording.

## 8. Judgement calls
- Re-word in `custom_format.rs` comment: "each get an auxiliary deck/component from `deck_loading.rs`" became "each rely on an auxiliary deck/component keyed on this exact `GameFormat` literal" because Dandan has no `deck_loading.rs` grant yet.
- `p2p-adapter-multiplayer.test.ts` comment: also moved the `"refuses v73"` phrase to `"refuses v74"` (plan listed the other four numerals; this stale numeral is in the same comment).
- `area: "formats"` for the new capability entry (plan text; no closed set of areas exists).
- Doc comments use the real `Dandân` character; Rust string literals use `\u{e2}` per file convention.
- Axis types documented with one-sentence CR docs; `best_of_three_ceiling` doc says a host-supplied match config never raises it (as planned).

## 9. Stop-and-return items
None. No out-of-scope path was needed.

## 10. Deviations from the plan
None in design. Extra evidence beyond the plan: red legs for the axes/config/card-bot/wire/script/manabrew rows.

## 11. Risks
- Rows `format_axis_census` (six headers, six keys) and the FromStr/ordering rows were not mutation-run individually (existing non-vacuity guards cover them; ordered-table equality is red on a misplaced variant by construction).
- Disk pressure on the box (see section 2); orchestrator gates that rebuild the engine need free space.
- `FixedDeckKeepMulligan` still force-keeps any `supplies_fixed_deck` hand including Dandan's until Phase 3 (unreachable: no Dandan game can start; plan's interim note holds).
