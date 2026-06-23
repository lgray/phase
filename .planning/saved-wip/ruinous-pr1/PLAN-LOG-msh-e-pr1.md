# PR1 PLAN LOG — dynamic_max_choices building block + The Ruinous Wrecking Crew

## Verified seams (file:line evidence)
- ModalChoice struct: types/ability.rs:12559-12599. Fields: min/max/mode_count + serde-default
  extras (mode_descriptions, allow_repeat_modes, constraints, mode_costs, mode_pawprints,
  entwine_cost, chooser, selection). `#[derive(Default)]` → Option defaults None. ADD field here.
- QuantityExpr: types/ability.rs:4817 (Ref{qty}, Fixed, ...). QuantityRef::CostXPaid: ability.rs:4424.
- resolve entry: game/quantity.rs:68 `resolve_quantity(state, expr, controller, source_id) -> i32`.
  CostXPaid resolves quantity.rs:2792 = state.objects[source_id].cost_x_paid (stamped finalize_cast
  casting_costs.rs:5511; persists to bf game_object.rs:594).
- Parser chain: parse_modal_choose_count (oracle_modal.rs:1441) + scan_modal_count_override
  (oracle_modal.rs:1626; "choose up to <number>" arm at 1643) → ModalHeaderAst {min,max,...}
  (ast.rs:1520) via parse_modal_header_ast (oracle_modal.rs:558) → build_modal_choice
  (oracle_modal.rs:1213) → ModalChoice.
- RUNTIME single authority: modal_choice_for_player (ability_utils.rs:509) — produces `effective`
  ModalChoice with runtime-resolved max_choices (currently only ConditionalMaxChoices). Called by ALL
  8 modal-construction sites incl. triggered ETB (engine.rs:5158) + activated (casting.rs:12501) +
  spell (casting.rs:9126/9836, casting_costs.rs:834/2598) + triggers.rs:3948. INJECT HERE.
- Validation gate: validate_modal_indices (ability_utils.rs:5222) reads modal.max_choices (5237).
- AI legal modes: generate_modal_index_sequences (ability_utils.rs:5284) min..=max; random AI
  random_select_modal_indices (ability_utils.rs:1065) reads max_choices.
- Frontend: ModeChoiceModal.tsx reads modal.max_choices/min_choices (39,57,71,83,86,87,102-112).
  WaitingFor::AbilityModeChoice carries `modal: ModalChoice` (game_state.rs:3483; TS types.ts:1185).
  Because engine puts RESOLVED modal into WaitingFor, frontend reads resolved cap — NO FE change.

## Architecture decision
Resolve dynamic_max_choices → concrete max_choices inside modal_choice_for_player (the single
runtime authority). Every downstream consumer reads concrete usize. No change to validate/AI/FE.
This is the minimal additive, serde-safe, engine-owns-logic design.

## CR verified (docs/MagicCompRules.txt)
- 700.2 (line 3199) modal def; 700.2a (3201) controller chooses; 700.2d (3207) no repeat.
- 601.2b (2457) modal announcement. 107.3m (488) ETB ability X = cast X (EXACTLY Ruinous).

## Hawkeye sizing → SPLIT
No "pay {1} up to three times" repeated-optional-cost primitive exists. AdditionalCostPaymentCount
(ability.rs:4431) is for additional costs of one payment, not a 0..N pay-loop on a trigger. Needs new
infra (repeated-optional payment WaitingFor + count tracking + a payment-count ref binding). SPLIT.
