# Phase 2 plan review, round 1 (phase-plan mode, phase-fit context declared)

MODEL: claude-sonnet-5-5
Reviewed: `phases/2/plan.md` against the frozen charter r3 Phase 2 entry, the brief's Settled decisions, and the code at HEAD 2cf3d3aa.

## Verdict

No `behavior` or `machinery` finding. Six `text` findings (F1-F6), each with replacement text; a round whose findings are all `text` is closed by applying them. The design (one unit, the 6 axes, routing through `evaluate_constructed`, the refusal, the pins) holds.

## Orchestrator adjudications

**(a) `brokerClient.test.ts` (plan scope item 19).** Out of the charter scope rule and in none of the standing classes, so it cannot be admitted. The floor test can be homed in chartered `client/src/network/__tests__/protocol.test.ts`.
- Probe: a throwaway vitest file in `network/__tests__/` (deleted; `git status` clean) imported `lobbyProtocolRequiredForFormat` and `MIN_LOBBY_PROTOCOL_FOR_FREEFORM_FORMATS` from `../../adapter/ws-adapter`. The import resolves and runs.
- The two assertions that were meaningful passed: Freeform returns the floor and Standard returns null.
- The instrument also showed the discriminating value. An unwired `"Dandan"` reaches the `never` default, which returns the format string itself, not `null`. So `expect(lobbyProtocolRequiredForFormat("Dandan")).toBe(15)` is revert-failing.
- The `registerHost` send-refusal is generic: `brokerClient.ts:212` and `tournamentClient.ts:659` both read `lobbyProtocolRequiredForFormat`. The charter's own claim is "`MIN_LOBBY_PROTOCOL_FOR_DANDAN` wired into `lobbyProtocolRequiredForFormat`", which the pure-function test establishes. See F1.

**(b1) No `quick_dandan_check` / `evaluate_dandan`.** Accepted.
- The pair appears only in the charter's scope-rule parenthetical. The Goal, Claims and Verification plan carry no acceptance row for it; the scope rule is an upper bound, and dropping it reduces scope.
- The brief's "must read axis methods, not literals" is met: `evaluate_constructed` and `quick_constructed_check` read `deck_size.accepts`, `sideboard_policy`, `commander_pairing().admits_count` and `default_deck_copy_limit` from `FormatConfig`.
- Measured: `CardPoolAuthority::for_format` maps `NoEngineAuthority` to `AdmitsEveryCard`. `requirement_phrase` renders "exactly 80". The reason strings row 6 asserts ("does not allow a sideboard", "do not use a commander slot") exist verbatim in `evaluate_constructed`.
- Momir's pair exists to check a fixed composition (12 of each snow basic); Dandan's pile is engine-supplied, so a pair would restate `evaluate_constructed`.

**(b2) Refusal plus registry entry instead of a documentary entry.** Accepted, and not a second unit.
- The charter says "the mechanism — entry only, or a refusal at start — is chosen by the phase plan", and its T1∧T2 sentence lists "adapter declaration" among the one unit's lockstep layers. It is an existing-error-variant guard beside `UnsupportedPlayerCount` (`prepare_snapshot_with_prompt_id`), with no new enum or type.
- Under the unit anchor ("regardless of how many lockstep layers"), T1 fails. The plan's own hedge in Sizing must go (F2), because a conditional Sizing reads as an undecided count.
- Measured: `manabrew-protocol` 5.2.0 has "One entry per (zone, owner) pair" (`game/mod.rs:99`) and `MulliganOutput` with the single upstream variant `MulliganDecision { keep: bool }`.
- Measured: the `no_emitted_capability_code_is_undeclared` scanner reads `"upstream.…"` literals from the production half, so the guard's literal and the registry entry are both seen.
- Measured: array length and the two asserts are at 93 today.

## Findings (all `text`, all blocking until applied)

**F1 (text) Scope item 19 and row 8: home the floor test in chartered `protocol.test.ts`.**
Old (Sizing, item 19): "`client/src/services/__tests__/brokerClient.test.ts` — **not in the charter's scope rule**: … or the floor test moves into `protocol.test.ts` (in scope, wrong topic)."
Replacement: delete item 19. The count becomes **18**, matching the charter's 18 named paths. Add to item 11's scope note: "`protocol.test.ts` also carries the floor test."

Old (Sizing "Not edited" list): the sentence introducing `brokerClient.test.ts` as an edit does not exist there, but the adjudication note says "T2 is 19 (≥13)". Replacement: "T2 is 18 (≥13)". Old (Sizing last bullet): "T1 fails (1 unit); T2 fires (19)". Replacement: "T1 fails (1 unit); T2 fires (18)".

Old (row 8): Test column "new tests in `brokerClient.test.ts`: `Dandan` against lobby 14 rejects with `LobbyCapabilityError` … against lobby 15 it sends once".
Replacement (Test column): "new describe in `network/__tests__/protocol.test.ts` importing `lobbyProtocolRequiredForFormat`, `MIN_LOBBY_PROTOCOL_FOR_DANDAN`, `LOBBY_PROTOCOL_VERSION` from `../../adapter/ws-adapter`: `lobbyProtocolRequiredForFormat('Dandan')` is `MIN_LOBBY_PROTOCOL_FOR_DANDAN`; `MIN_LOBBY_PROTOCOL_FOR_DANDAN` is `15` (frozen literal) and `<= LOBBY_PROTOCOL_VERSION`."
Replacement (Production entry): `lobbyProtocolRequiredForFormat` (the sole reader for the send path: `brokerClient.registerHost`, `tournamentClient`).
Replacement (Revert-failing): "delete the `case \"Dandan\"` arm: the `never` default returns the string `\"Dandan\"`, so the `toBe(15)` assertion is red (measured with a throwaway probe; tsc also refuses the missing case)."
Replacement (Reach-guard): "same describe: `Freeform` returns `MIN_LOBBY_PROTOCOL_FOR_FREEFORM_FORMATS` and `Standard` returns `null`, so the function is read per format and not constant."
Also fix the E7 table row "P2P wire" file list to add the floor test to `protocol.test.ts`, and remove `brokerClient.test.ts` from "Analogous Trace" item (1)'s test list only as an edit target (it stays a precedent).

**F2 (text) Sizing: remove the conditional.**
Old: "If a reviewer counts it as a second unit, T2 is 19 (≥13), so T1∧T2 would fire; the documentary-only alternative (entry with no code path) avoids that at the cost of an unenforced claim, and would be chosen instead. The plan's choice is the refusal because the crate's own rules make every registry entry a claim that must not be false."
Replacement: "The refusal is a lockstep layer of the one unit, as the charter's T1∧T2 sentence counts the adapter declaration: the charter leaves the mechanism to this plan, the guard adds no enum or type, and it uses the existing `AdapterError::UnsupportedProtocolFeature`. The plan chooses the refusal because the crate's own rules make every registry entry a claim that must not be false."

**F3 (text) Row 12 and the safety sentence overclaim: "a Dandan game cannot start before Phase 6" holds only for the wasm path.**
Measured: only `engine-wasm` has the "Empty library after deck load" guard (`initialize_game`). The server session path (`server-core/src/session.rs` → `load_and_hydrate_decks` → `start_game`) has no such guard. `phase-server/src/main.rs` (~9548) validates the host's submitted deck with `validate_name_deck_for_format_full`, which after this phase admits any 80-card list for Dandan (exactly 80, unlimited copies, every card). A hand-built client can therefore start a server-hosted "Dandan" game with per-seat decks. No first-party client path does: the client submits empty or placeholder decks (`formatSuppliesDeck`), which the server rejects as not exactly 80. The Dandan branch that Phase 6 adds to `load_and_hydrate_decks` is the shared function server-core calls, so it closes every transport.
Old (Logic/Step 0 gate paragraph): "a Dandan game cannot start before Phase 6: empty submitted decks trip the wasm empty-library boot guard". Old (row 12 claim): "Interim safety: a Dandan game cannot start before Phase 6".
Replacement (both places): "No first-party client path starts a Dandan game before Phase 6: the wasm path refuses empty submitted decks at its empty-library boot guard, and the server-hosted path rejects the client's empty or placeholder deck as not exactly 80. A hand-built client can submit a legal-looking 80-card list to a server session, which then loads per-seat decks; Phase 6's Dandan branch in `load_and_hydrate_decks` (shared by wasm, server-core and replay) closes it."
Replacement (row 12 seam and evidence): "`GameFormat::Dandan` config via `initialize_game` (wasm) and via the server `CreateGameWithSettings` deck check; read (not run)."
Also fix the same overclaim in "Claims not established": add "5. Server-hosted interim behavior above is read, not run."

**F4 (text) E1.4 `grants_free_first_mulligan` comment names a nonexistent item.**
Measured: `git grep -n free_first_mulligan -- crates/engine/src` finds `mulligan.rs::free_first_mulligan(state)` (a function that reads `seat_order.len() > 2 || format.grants_free_first_mulligan()`) and no `FormatConfig::free_first_mulligan`.
Old: "Dandân's own free mulligan is `free_reveal_mulligan()`, and `FormatConfig::free_first_mulligan` must stay false so `mulligan_count == 0` means \"no regular mulligan yet\"".
Replacement (one sentence, in the code comment): "// CR 103.5c's free first mulligan is multiplayer and Brawl only; Dandân's free mulligan is the `free_reveal_mulligan()` axis."
Same replacement for the Reference Readings row 5 sentence "`grants_free_first_mulligan() == false` … measured: the existing table returns `false`": keep the measurement, drop the `FormatConfig::free_first_mulligan` clause if it appears.

**F5 (text) CR 100.6a is cited as a rule Dandan "departs from"; its text says "usually".**
Grepped: "100.6a … A two-player match usually involves playing until one player has won two games." A best-of-one is not a departure from a "usually".
Old (E1.2 variant doc): "and CR 100.6a (best-of-one); each departure is a typed axis method below."
Replacement: "and CR 103.5 (a free mulligan by revealing a hand); its match is capped at one game (CR 100.6a describes the usual two-win match, CR 100.4 sideboarding between games); each departure is a typed axis method below."
Old (Step 0 CR list): "100.6a (a two-player match is played until one player has won two games)".
Replacement: "100.6a (a two-player match usually involves playing until one player has won two games)".
Old (Stage 3 boundary line and Pattern-Coverage wording): "match structure (CR 100.6a)". Replacement: "match structure (CR 100.6a, the usual structure the ceiling caps)". Also drop the duplicate "CR 103.5" from the departure list if it now appears twice.

**F6 (text) p2p test comment: stale numerals beyond the one the plan names.**
Measured (`p2p-adapter-multiplayer.test.ts` ~5388-5397): the comment reads "(74 → 73) … the v73 frame now equals the reverted constant … and the v74 frame no longer equals it". The script guards titles and `setupFrameAt`, not this prose, so stale numerals would pass every gate.
Old (E7 P2P row): "the comment's `(74 → 73)` becomes `(75 → 74)`".
Replacement: "the comment's `(74 → 73)`, `the v73 frame`, `the v74 frame` and `the v74` all move up one (`(75 → 74)`, `the v74 frame`, `the v75 frame`, `the v75`)".
Also: state that the new `protocol_version_is_93_for_dandan_format` keeps its doc paragraph and gains a v93 sentence, since the old paragraph's last sentence describes v92 only.

## Checked and found sound (no finding)

- Census: `matches.py HEAD` reports 18 exhaustive prod matches (wasm 1, `deck_validation.rs` 2, `custom_format.rs` 1, `format.rs` 14) and 0 test-scope entries. Wildcard matches (`card_subset.rs`, `deck_validation.rs` inner, `topology`, `max_deck_copies`) need no Dandan arm: `card_subset` returns `None`, `topology` and the rest are not Dandan-specific.
- Pins: `check-protocol-version.mjs` is green at base (92 / 14 / 74 / 30). Its constants are `UPSTREAM 71 + 21`, `PHASE_TWO_BASE 54 + 20`, lobby 14. The plan's targets (`+22`, `+21`, 15, `MIN_LOBBY_PROTOCOL_FOR_DANDAN` in `AUTHORED_LITERALS`) match. The script derives its title, function-name and `setupFrameAt` patterns from those constants, and the plan covers each.
- The server-core rename is required by `refusePattern(protocol_version_is_92)`; the plan names both function sites. The lobby-broker chain test currently ends at `HOSTED_MATCH_LOBBY_VERSION` (14), so the `DANDAN_FORMAT_LOBBY_VERSION` addition is correct.
- CRs grepped in `docs/MagicCompRules.txt`: 100.2a, 100.4, 100.6a, 103.4, 103.5, 103.5c, 108.3, 110.2, 121.2c, 400.1, 400.3 all exist and support the uses (except the F5 wording).
- `built_in_axes_no_looser_than_rules` needs no change: `deck_size_authority` `RulesFixed` has empty options; a `dandan()` config equals `for_format(Dandan)` by construction.
- `legality_key` for Dandan is `None` (no recorded table), so the search.rs test (`legal_format: "dandan"` rejected), `CardGrid.legality` (`scryfallLegalityKey` is `undefined`) and `registry_constructed_formats_declare_a_deck_construction_pool` (`Multiplayer` group) stay green by reading.
- Card-bot: `bun test scripts/card-bot` re-run, 158 pass; the "every registry format is reachable by typing its label" and cap-at-25 tests exist, so a 26th entry stays reachable.
- The client has no `Momir` literal in a setup path (`git grep`); `HostSetup.tsx` surfaces the engine's `customFormatFromLobbyConfig` rejection verbatim, so the extended error string needs no client change.
- `evaluate_selected_format` and `evaluate_selected_format_summary` both have a `Freeform`-headed constructed group that Dandan can join with no other change.
- The full-plan compile and test claims (census completeness by compiler, the row 5 flipped-arm red leg, the `--lib` and `integration` rebuilds) are executor measurements as the plan says; I ran no cargo (a cold ~10 min rebuild at `CARGO_BUILD_JOBS=2`, and no design uncertainty that a build would settle).
