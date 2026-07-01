# MSH-E Audit Log — Hawkeye + Ruinous Wrecking Crew

## Measured root cause (both cards)
Both cards: `gap_count=0`, every parse node `supported:true`, yet card `supported:false`.
The flip is NOT a parser gap and (traced) NOT `check_resolver_features`/swallow detectors —
it is **`check_silent_drops`** (coverage.rs:4458 / 5117).

`count_effective_oracle_lines` folds modal bullets ONLY when `is_modal_header_line`
(coverage.rs:5432) recognizes the header. Its phrase list has `"choose up to one".."ten"`
+ `"choose x."` but **NOT `"choose up to X"` nor `"choose up to that many"`** → bullets
counted as separate Oracle lines → `effective_oracle > effective_parsed` → `SilentDrop`.

- Hawkeye: oracle=5 vs parsed=4 → `SilentDrop:4_of_5`.
- Ruinous: oracle=6 vs parsed=4 → `SilentDrop:4_of_6`.
(`count_effective_parsed_items` counts top-level item + DIRECT children only, 1 level deep.)

## Per-card runtime verdicts (measured)
RUINOUS — runtime FULLY WORKS; pure coverage-marker. #4186 (23c50148a) fixed parser+runtime
dynamic modal max but never updated the coverage line-counter.
- `FilterProp::Token => obj.is_token` (filter.rs:3179) ✓ destroy target token.
- `Sacrifice{player_scope:All}` iterates APNAP, each player chooses own creature
  (effects/mod.rs:4037-4090; test `player_scope_all_sacrifice_iterates_each_player`) ✓.
- `PutCounter{count:Ref(CostXPaid)}` via Moved replacement: `cost_x_paid` stamped at
  finalize_cast (casting_costs.rs:5506), read by quantity.rs:2837; Walking Ballista test
  (engine.rs:7901) proves X→counters ✓.
- modal `dynamic_max_choices:Ref(CostXPaid)` present + resolved by modal_choice_for_player
  (ability_utils.rs:543-576) ✓.
→ Ruinous needs ONLY the coverage line-counter fix.

HAWKEYE — coverage marker is active, BUT genuine semantic gaps. Fixing coverage alone =
FALSE GREEN. Real AST: PayCost{optional:true, repeat_for:Fixed(3), sub_ability=GenericEffect
{condition:WhenYouDo, modal{min:1,max:1,mode_count:3, dynamic_max_choices:None}}}.
- modal min/max fixed (1,1); "choose up to that many" (= times paid) NOT captured.
- repeat_for is on the PayCost-with-nested-modal; `resolve_chain_body` resolves effect+sub
  per iteration (effects/mod.rs:3991) → modal would WRONGLY repeat each payment. Structural
  misparse.
- NO QuantityRef expresses resolution-local "times a cost was paid" (all *Paid/*Count refs
  are cast-time: CostXPaid/KickerCount/AdditionalCostPaymentCount).
- BUG: `cost_payment_failed_flag` not cleared between repeat_for iterations (effects/mod.rs
  ~3988-4008) → WhenYouDo wrongly suppressed if an earlier iteration declined.

## CR (grep-verified in docs/MagicCompRules.txt)
- 603.12 / **603.12a** (reflexive "when you do"; pay-cost-multiple-times → reflexive fires
  ONCE; count = times paid) — THE Hawkeye rule.
- 700.2 / 700.2b / 700.2d (modal triggered ability; mode-count ceiling).
- 107.3m (ETB ability/replacement X = spell's cast X) — Ruinous counters + modal max.
- 701.26 Tap (Taps trigger); 701.21 Sacrifice; 119.3 lose life; 122.1 counter.

## Decision: SPLIT — two independently-committable sub-plans
- A (Ruinous): coverage-only. is_modal_header_line recognizes dynamic headers +
  discriminating swallow detector `Modal_DynamicMaxDropped` keeps Hawkeye honestly red.
- B (Hawkeye): new resolution-local payment-count QuantityRef + bounded optional repeated
  payment w/ count + repeat_for flag-carryover fix + parser restructure (reflexive-once,
  min=0, dynamic_max=Ref(count)) + reuse #4186 modal cap. Then Hawkeye goes green correctly.
