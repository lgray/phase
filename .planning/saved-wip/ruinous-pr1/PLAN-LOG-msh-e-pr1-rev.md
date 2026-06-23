# PR1 REVISED PLAN LOG — dynamic_max_choices + The Ruinous Wrecking Crew

Revision of PLAN-LOG-msh-e-pr1.md folding in 2 gate gaps (no compile blocker found in prior plan).

## Re-verified seams (worktree /private/tmp/wt-msh-modal-choose @ 3a844e56d)
- ModalChoice struct: types/ability.rs:12559-12599 (`#[derive(... Default, PartialEq, Eq, Serialize, Deserialize)]`).
  Last field `selection` (12597). ADD `dynamic_max_choices` after it.
- QuantityExpr enum: ability.rs:4817 (Ref{qty}/Fixed{value}/...). QuantityRef: ability.rs:3883. CostXPaid: 4424.
- resolve_quantity(state, expr, controller, source_id) -> i32 : quantity.rs:68. CostXPaid arm: quantity.rs:2792
  = state.objects[source_id].cost_x_paid (or chosen_x fallback).
- modal_choice_for_player(state, player, source_id, modal, context) -> ModalChoice : ability_utils.rs:509.
  Sets effective.max_choices = constraint cap at 531 WITHOUT runtime .min(mode_count). INJECT after the
  constraint loop (after line 533, before `effective` returned at 534).
- Parser chain (all in oracle_modal.rs):
  scan_modal_count_override:1626 (returns Option<(usize,usize)>; "choose up to N" arm at 1643)
  → parse_modal_choose_count:1441 + parse_modal_count_remainder:529 (both consume scan)
  → parse_modal_header_ast:558 (consumes both; gate is_modal_header_text:539)
  → ModalHeaderAst (oracle_ir/ast.rs:1520; min/max/...)
  → build_modal_choice:1213 (literal at 1224; max clamp at 1219-1223).
  ALL callers of the 3 count fns are inside oracle_modal.rs (grep-confirmed). Blast radius contained.

## GAP A (CORRECTNESS): runtime clamp .min(mode_count)
cap_modal_constraints (oracle_modal.rs:1241) clamps ConditionalMaxChoices caps to mode_count at PARSE
time, so modal_choice_for_player needs NO runtime clamp for that path. But dynamic cap resolves from
cost_x_paid at RUNTIME and can exceed mode_count (Ruinous: 4 modes, X=10). Injection MUST clamp:
    if let Some(expr) = &modal.dynamic_max_choices {
        let resolved = super::quantity::resolve_quantity(state, expr, player, source_id);
        effective.max_choices = (resolved.max(0) as usize).min(modal.mode_count);
    }
Test: X=10/4 modes → cap 4. Keep X=3→3, X=0→0.

## GAP B (BUILD): missing-field ModalChoice literals (cargo check -p engine does NOT see siblings)
- crates/server-core/src/filter.rs:435 (test) — complete 11-field literal; add dynamic_max_choices: None.
- crates/mtgish-import/src/convert/mod.rs:1316 (prod, DORMANT) — already has all newer fields as
  "compile-keep-alive"; add dynamic_max_choices: None ONLY (compiler-forced, no logic). Allowed exception.
- crates/engine/src/database/forge/translate.rs:171 (prod, behind default-off `forge` feature) — STALE:
  only 7 fields (through mode_costs), ALREADY missing mode_pawprints/entwine_cost/chooser/selection →
  already won't build --features forge independent of our change. Add dynamic_max_choices: None for the
  one field we own; pre-existing-stale fields are OUT OF PATH (don't expand scope). Note as not in
  verification path unless --features forge is built.
- In-engine (cargo check -p engine flags): oracle_modal.rs:1224 (build_modal_choice, the main prod edit),
  triggers.rs:25140 + 25253 (test literals).
VERIFICATION: `cargo check --workspace --all-targets` (NOT -p engine) to catch siblings. --features forge
out-of-path.

## CR re-verified (docs/MagicCompRules.txt grep, this worktree)
- 700.2 line 3199 (modal def); 700.2a 3201 (controller chooses); 700.2b 3203 (modal TRIGGERED ability —
  Ruinous's ETB is triggered, this is the precise rule); 700.2d 3207 (no repeat); 601.2b 2457 (modal
  announcement); 107.3m 488 (ETB X = cast X — EXACTLY Ruinous).

## TS mirror: client/src/adapter/types.ts ModalChoice (1075-1090) — add optional dynamic_max_choices
field for type fidelity (unread by UI; engine puts RESOLVED cap into WaitingFor).
