# Phase 14 plan review, round 2 (phase-plan mode, Sizing consistency blocking)

MODEL: claude-sonnet-5-5

Reviewed the whole of `phases/14/plan.md` (rev 1, 194 lines) against the charter Phase 14 entry and the code at HEAD. No cargo was run; every claim below rests on a read, grep, python or jq command run this session.

## Verdict: APPROVE (0 blocking findings; 2 non-blocking notes)

## F1-F6 closure (verified in the revised text and against code)

- F1 closed. 3.4 adds `record_player_actions_performed` (the fold is real: `effects/mod.rs:16669-16677`, `player_actions_this_way.insert` + `record_player_action_this_turn`), called by the effect step and both `Completed` branches; V1/V2/V3 carry the parity rows; the parked-resume limit is stated and matches `engine_replacement.rs:707-722` (the choice handler folds only Scry).
- F2 closed. `insert_ability_continuation_parent_at_child_boundary` exists (`game_state.rs:24165`); `child_stack_start` is captured at `resolve_chain_body` entry (`mod.rs:14749`, function starts `:14732`) and is in scope at the layer-1 driver (`:15054`) and layer-2 (`:14879`); precedent use at `mod.rs:17939` and `attach.rs:448`. V11 added with its paired positive and no-prompt control. Its fixture is consistent with the code: Familiar is `IndividualDraw` so `draw_instruction_may_be_replaced` (`replacement.rs:7207`) stays false for it, Alms is `InstructionCount`, so the park is a nested individual draw inside P1's Settling consult.
- F3 closed. Row `("DrawDealerSeat", None, "applied", "HashSet", HASH_SET)` and the path are in V9 and section 9.
- F4 closed per the orchestrator ruling. `deal_sequence(state, &[(PlayerId, usize)]) -> Vec<PlayerId>` matches `phases/12/plan.md:81`; `apnap_order_from(None, ..)` anchors on the active player (`players.rs:256-258, 295-305`); the `starting_with` refusal is in `plan_simultaneous_draw`; `mulligan.rs` is out of scope and the path count is 12 (5 production + 2 comment-only + 5 pins), T1 still fails.
- F5 closed (V4 first cell and R3). F6 closed in substance (see note N1).
- Nit closed: `is_plain_parent_target_delivery` has 26 conjuncts (`scoped_library_search.rs:117-153`, counted), 22 after removing `targets`, `multi_target`, `player_scope`, `starting_with`.

## S1-S5 verification

- S1 correct: the dealer intercept precedes `capture_clause_minimum_snapshot` (`mod.rs:15184`), so clearing it on `Parked` is right.
- S2 correct: a Settling consult runs before any `attempted_empty_library` can be recorded; the hazard is Dealing-only.
- S3 correct: both layers default to the active player (see F4 evidence); `starting_with: Some` is refused.
- S4 correct and parity-true (choice handler folds only Scry).
- S5 fine.

## Fresh review: measured and sound

- Premise: Jace -7 parsed JSON (`optional_targeting: false`, `optional: false`, `forward_result: false`, `multi_target {0,null}`, `sub_ability: null`) passes the rider predicate once `targets`/`multi_target` are bound by the layer-2 narrowing; no layer-2 `Draw` head in the corpus carries a `player_scope` (python census: Donatello's Science Lesson, Gleaming Splendor, Huddle Up, Jace all `None`).
- Driver semantic check: `draw.rs:614-710` unit loop, `:556-580` `settle_draw_instruction` (requires `frame.player == player_id`), `engine_replacement.rs:655-668` Instruction arm and `:676-690` credit rule are all preserved by the working-copy rule; the only seat switch is at `begin_next_unit`, after `pending_delivery` is consumed (`:620-643`) and `remaining -= 1` (`:659`) with nothing pushing between them, so the `validate` invariant `remaining == schedule.len()` holds at every park.
- Table publication: `install_previous_effect_counts_by_player` (`mod.rs:12742`) and `publish_player_scope_clause_results` (`:12785`) are private to `effects`, reachable from the child `draw.rs`; the publisher's `None` arm clears the table (`:12755-12766`), so the plan's re-install after publication is required and correct; `previous_effect_amount_from_events` for Draw reads `last_effect_count`, which the dealer sets to the sum.
- `DrawSequenceFrame {` literals: `game_state.rs:2` only (git grep, re-run). No `DrawSequenceFrame`/`draw_sequences` mention in client or any other test beyond those the plan names.
- Wire: `RESOLUTION_STATE_WIRE_VERSION = 3` with legacy 1/2 (`resolution.rs:3689-3698`), decode match `:4034-4042`; the only tests naming a version literal for the writer compare against the constant; the "expected 1, 2, or" message is asserted nowhere; the `[1, 2]` loops in `game_state.rs:31664, 31905` are unaffected by a 3-and-4 decode arm. Protocol: HEAD tree is 94 / wire 76 with script `+23` / `+22`, so `+27` / `+26` for 98 / 80 follows the stated Phase 11-13 chain.
- CR numbers 109.5, 121.1, 121.2, 121.2a, 121.2c, 121.4, 121.6a, 121.6b, 614.6, 616.1g, 104.4a, 704.5b grepped; each says what the plan cites.
- S6 outcomes re-derived: pile 4, X=3 dealt order gives P0 {c1,c3}, P1 {c2,c4}, both then attempt (draw, CR 104.4a); sequential gives P1 alone flagged, so (c) is red at base and (a)/(b) are green at base by construction, as stated. V5 hand split `P0 {c1,c4}, P1 {c2,c3,c5}` follows from the skip leaving c3 on the pile. V4 order `[P0,P1,P0,P0]` follows from settle-first with Alms draws performed at settle (`OriginalController` = P0, `mod.rs` resolves to `original_controller.unwrap_or(controller)`).

## Notes (non-blocking, tag `text`)

- N1 (`text`): 3.2.1 reads "the single-seat start pushes the frame with `count` and calls the same function for its consult arm only". The existing consult arm (`draw.rs:525`) pushes the frame with `0` (so a Prevented/substituted instruction leaves it owing nothing); only the skip arm pushes `count` (`:522`). Pushing `count` on the consult arm would draw a replaced instruction anyway. Replace the quoted clause with: "the single-seat start keeps pushing the frame with `count` on its skip arm and with `0` on its consult arm, and calls `consult_draw_instruction` for the latter only".
- N2: the layer-2 `Completed` branch (and layer 1) does not reset `state.cost_payment_failed_flag` per seat (the sequential driver does, `mod.rs:15199`) and layer 2 does not restamp `last_zone_changed_ids`. No corpus card is affected (the seat riders exclude any condition; the only layer-2 Draw head with a tail, Communal Brewing, has a `sub_ability` and keeps the sequential path), so this is recorded, not a finding.

## Result

No `behavior`, `text` (blocking) or `machinery` findings. Sizing is consistent: 1 unit, 12 counted paths, tests excluded, T1 fails so the T1 and T2 conjunction cannot fire. Plan is ready for implementation; the executor probes P1-P8 stay as written.
