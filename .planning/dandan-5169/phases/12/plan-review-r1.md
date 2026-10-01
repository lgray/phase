# Phase 12 plan review, round 1 (phase-plan mode, phase-fit context declared)

Reviewer model: claude-sonnet-5-5. No cargo run; reads, grep, git grep only.

**Verdict: no behavior and no machinery findings. Three `text` corrections, one `text` scope note, one non-blocking sizing note. A round whose findings are all corrections is a clean round (review-engine-plan, charter-mode paragraph); the orchestrator applies them.**

## Adjudications

### (a) Held declarations in `WaitingFor::MulliganDecision.declared`: ACCEPT
- Within the frozen goal and acceptance rows. Charter goal: "each `Declare` entry records a ... declaration without redrawing; once every entry has declared, all redrawers are dealt interleaved". It names no carrier. Claim 3 ("entry-shape change lands only as far as the declare round needs it") is met trivially: `MulliganDecisionEntry` and `MulliganDecisionPhase` are unchanged. `game_state.rs` is in the scope rule, and `WaitingFor::MulliganDecision` lives in that file.
- Phase 13's "availability rides the entry or `MulliganDecisionPhase`" is unaffected. The plan's Phase 13 seam (a kind field on `MulliganDeclaration`, one match in `close_declare_round`) is coherent.
- Right on the rule. CR 103.5 (grepped, line 296): declarations are made in order, then "all players who decided to take mulligans do so at the same time", and the process repeats. CR 103.5b (line 300): other players may already have declared when an action is taken, and the player then declares. Recording a declaration, holding it, then closing the round and dealing active-player-first matches the text. Serum Powder stays an immediate action; a player in `declared` cannot use it, matching "they then declare".
- `pending`-based readers are correct by construction. Read at HEAD:
  - `acting_authority` (`game_state.rs` ~16742) is a bare, unguarded arm whose doc comment forbids guards, and the census A8 enforces that. A phase filter there would be refused.
  - `candidates.rs:868` iterates `pending`.
  - `aiController.ts::aiPendingForMulligan` (lines 255-272) returns the first AI seat in `pending` with no phase read. A declared seat must therefore leave `pending`, as the plan does.
  - `p2p-adapter.ts:760` (`aiActorFromWaitingFor`) is the same shape and is also correct.
  - The `pending` readers in `interaction.rs`, `search.rs` and `manabrew-compat` need no edit.
- Hang invariant (pending empty and declared non-empty never rests) is closed by `advance_after_decision` case 3. A single lone redrawer, round after round, closes immediately (traced).

### (b) Out-of-charter paths
- `elimination.rs::prune_mulligan_pending`: the pattern `WaitingFor::MulliganDecision { pending, free_first_mulligan }` at line 598 has no `..`, so the file is compiler-forced by the new field (standing class). The behavior edit is a design choice, not a forced one: filtering `declared`, `settle_mulligan_pending`, and removing `finish_mulligans_public`. It is semantically required (CR 800.4a: otherwise a held declaration is dropped when a concede empties `pending`). It needs the orchestrator's charter revision. See F1.
- `client/src/adapter/types.ts`: hand-written mirror (not a generated binding); the field is optional, so it is not compiler-forced. A registration/mirror-class addition, comment-and-type only. Two options: admit it, as Phase 2's "client mirrors" precedent does, or defer it to Phase 16 (declaration display), whose scope rule already names `types.ts`. Recommendation: admit; it is two lines, and the `WaitingFor` mirror is wrong without it. See F4.
- Phase 6 and Phase 11 test files (`dandan_shared_pile_storage.rs`, `dandan_hand_entry_ownership.rs`): a charter decision (a revision), not a standing class. Would they actually fail? Yes, by derivation from the accepted plans. They do not exist at HEAD, so the measurement is a read of the plan rows:
  - `phases/6/plan.md` V3 (line 128): a lone `apply(P1, Mulligan)` then "pile is `before` minus the seven cards drawn". Under the round the lone Mulligan is held, so the pile is unchanged.
  - `phases/6/plan.md` V3 paired guard (canonical seat mulliganing): the same.
  - `phases/11/plan.md` V3 (line 171): "P1 then takes a mulligan: P1's redraw is `HAND(P1)`" and P0's hand is untouched; the redraw never happens.
  - `grep` for `MulliganChoice::Mulligan` over `phases/*/plan.md` finds only Phase 6 (plus this plan); Phase 11 uses prose. At HEAD, `git grep MulliganChoice::Mulligan -- crates/engine/tests` hits only non-Dandan files (`combo_infinite_pile.rs`, `gemstone_caverns_begin_game.rs`), which the shared-axis gate leaves unaffected.
  - So the edits are a fix forced by this phase's own behavior change, in the later phase's scope. See F3.

### (c) Dealer
- The brief's S4 section (line 203) settles it: "One pregame interleave primitive, used by the opening deal (`start_mulligan`) and by the round's redraws. Use `players.rs::apnap_order_from` for order." The charter goal repeats this. So the opening deal is interleaved, active player first, not sequential. The phrase "dealt one at a time, starting with the active player" is the article text (brief line 32), and Settled decision 2 applies it to Day's Undoing (Phase 15), not to the opening deal. The plan's reading is right.
- CR 103.5 gives no deal order; CR 121.2c (grepped, line 1152) is the stock order that `DealOrder::Interleaved` departs from. The plan states this.
- `apnap_order_from(state, None, active_player)` starts at the active player and skips eliminated seats (read, `players.rs` 281-345). `start_game_with_starting_player` sets `active_player` and rotates `seat_order` before `start_mulligan` (read, `engine.rs` 18440-18482). The companion path also reaches `start_mulligan` after both are set. Non-shared identity holds.
- Splitting the dealer into its own unit: not warranted. The charter froze 1 unit ("the interleave primitive that is its only consumer pair"). The dealer is the round's own deal step, and `start_mulligan` shares it. T1 fails, so the conjunction cannot fire regardless of T2. The plan's step order lands the dealer first as a seam if the orchestrator disagrees. See F5.

### (d) Rows, reach-guards, protocol
- Rows use real cards (the 80-card Dandan pile, Serum Powder verbatim from the card data). They are red at base: V1/V2 (no consumer of `deal_order`), V3 (sequential deal), V4a (eager redraw), V4b-f (no round), V4g (manual finish), V5 (P1 stays in `pending` at base), V6.
- Reach-guards are present for each: Standard-format siblings, P0 candidates present, the same flow with Keep instead of Mulligan, and count 5 versus 6 for the cap. The only weak row is V4c's shuffle evidence (F2).
- Protocol arithmetic is consistent: HEAD `PROTOCOL_VERSION` is 93 (`protocol.rs:771`), with `EXPECTED = UPSTREAM(71) + 22`. 96 is +25 and wire 78 is `54 + 24` (`PHASE_TWO_BASE = 54`). The Phase 5 and 11 plans say 93 to 94 to 95 and 75 to 76 to 77, and that Phases 6-10 add no serialized shape. The bump is warranted: a v95 peer would drop `declared`. The `skip_serializing_if` plus `default` keeps non-shared JSON byte-identical.
- CRs grepped and correct: 103.5, 103.5b, 103.5c, 121.1, 121.2c, 400.1, 701.24a, 800.4a. Note the alive-skip in `apnap_order_from` is cited in-code to CR 800.4f; this plan cites 800.4a for the elimination and does not need 800.4f.

## Findings

**F1 (text, scope matrix 8 item 1):** The plan calls `elimination.rs` "outside the charter's literal scope rule ... needs a charter revision" but does not say the file is also compiler-forced.
- Old: "1. `crates/engine/src/game/elimination.rs`: one function, `prune_mulligan_pending` (3.5). Required: without it a held declaration is lost when an elimination empties `pending`."
- New: "1. `crates/engine/src/game/elimination.rs`: one function, `prune_mulligan_pending` (3.5). The file is compiler-forced (its `WaitingFor::MulliganDecision { pending, free_first_mulligan }` pattern has no `..`), so it is a standing-class site; the behavior edit beyond the forced destructure (filtering `declared`, `settle_mulligan_pending`, removing `finish_mulligans_public`) is a design decision that needs the orchestrator's charter revision: without it a held declaration is lost when an elimination empties `pending`."

**F2 (text, V4c):** "exactly one pile shuffle of the container (the pile's order differs from the unshuffled expectation ...)" does not distinguish one shuffle from two or many, and the M5 revert covers only skipped.
- Replace the evidence clause with: "the pile's order equals the order obtained by replaying the close on a cloned state (same seeded rng, return both hands, one `shuffle_library_of`), and differs from the unshuffled expectation `(before ++ old hands).skip(14)`; the rng `get_word_pos()` delta of the close equals that of the single replayed shuffle of a pile of that length. Add M5b: shuffle once per redrawer (the dedup of distinct holders removed): the equality fails." The replay is the reference; the `UNESTABLISHED` label stays until probed.

**F3 (text, V8 and scope item 3):** the row is vague and cites the grep as the discovery method, but both files are absent at HEAD, so the grep is vacuous at plan time.
- Old: "each Dandan row that submits a lone `MulliganChoice::Mulligan` and then asserts the redraw now submits the other seat's `Keep` first ... Found by `git grep -n 'MulliganChoice::Mulligan' -- crates/engine/tests` at scope-freeze".
- New: "the named rows are Phase 6 V3 (both the P1 leg and its canonical-seat paired guard; each asserts a pile change after a lone Mulligan) and Phase 11 V3 (P1's redraw is `HAND(P1)`); each now submits the other seat's `Keep` before its assertions, and each asserted value is otherwise unchanged. At scope-freeze, `git grep -n 'Mulligan' -- crates/engine/tests/integration/dandan_shared_pile_storage.rs crates/engine/tests/integration/dandan_hand_entry_ownership.rs` is the discovery command; a Dandan Mulligan use not named here is a stop-and-return." Also state in scope item 3 that, by reading `phases/6/plan.md` V3 and `phases/11/plan.md` V3, those tests would fail unedited (a lone Mulligan is only held).

**F4 (text, scope item 2):** `types.ts` is a hand-written type mirror, not compiler-forced and not read by any component. Add one sentence: "admission requested as a mirror-only addition (the alternative, deferring to Phase 16, leaves the `WaitingFor` mirror wrong for a phase)."

**F5 (non-blocking sizing note, for the orchestrator):** about 1,050 added LOC against the charter's ~900. T1 fails (1 unit), so the conjunction cannot fire. The plan's text that `deal_sequence` serves Phases 14 and 15 does not make the dealer a second unit. Section 6 gives two T2 counts (17 or 9); both are consistent with the body. The planner should state that the charter's "forced" greps (`MulliganDecisionEntry {` and `MulliganDecisionPhase::`) are empty-by-design here because the entry and phase types are unchanged, and that the forced population is `WaitingFor::MulliganDecision` literals without `..` (compiler-enumerated). Measured: `git grep -c 'free_first_mulligan:' -- crates` lists 12 files, which matches the plan's section 8 list (10 files) plus `mulligan.rs` and `game_state.rs`.

## Not blocking, noted
- V5's accessor name (`legal_actions` or `candidate_actions`) stays an executor probe.
- The plan's dependency on Phases 6 and 11 forms (`library_of`, `.performed_by`) is honestly marked UNESTABLISHED.
