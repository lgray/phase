# MSH-E r2 planning worklog

Base for measurement: worktree `wt-msh-f` @ `aa4e88ec4` (v0.8.0 + MSH-F). Data symlinked from main worktree.

## Sub-plan A — coverage seams RE-LOCATED (new base)
- `analyze_coverage` → `coverage.rs:4396`; calls `check_silent_drops` @ **4464**, `check_parse_warnings` @ **4469**, `supported_before_parse_warnings = missing.is_empty()` @ 4466.
- `check_silent_drops` → `coverage.rs:5123-5141` (fires when effective_oracle > effective_parsed; label `SilentDrop:{p}_of_{o}`).
- `count_effective_oracle_lines` → `coverage.rs:5377-5434`; folds bullets only when `is_modal_header_line` true (call @ 5413).
- `is_modal_header_line` → `coverage.rs:5438-5464`. CHOOSE_PHRASES list 5439-5462: has "choose up to one..ten", "choose any number", "choose x." — LACKS "choose up to x" / "choose up to that many". Idiom = `CHOOSE_PHRASES.iter().any(|p| lower.contains(p))`.
- `count_effective_parsed_items` → `coverage.rs:5589-5601` (1 + children.len() per item with children).
- `parse_warning_gap_label` → `coverage.rs:5195-5210`; `SwallowedClause{detector}` → `Swallow:{detector}` @ 5204.
- `check_parse_warnings` → `coverage.rs:5184-5193` (folds labels into `missing`).
- swallow_check `check_swallowed_clauses` → `swallow_check.rs:78-118`; early-return on `any_ability_has_unimplemented` @ **93**; `ast_json = serde_json::to_string(parsed)` @ **103**; detector calls 105-117.
- Model detector `detect_dynamic_qty` → `swallow_check.rs:1355-1518+`; uses `json_has_any(ast_json, markers)` (helper @ **1342**) and `// allow-noncombinator: swallow detector marker scan on classified text` comments.
- A2: `dynamic_max_choices: Option<QuantityExpr>` serde attr → `ability.rs:12924` = `#[serde(default, skip_serializing_if = "Option::is_none")]` ⇒ OMITTED when None. Key on ABSENCE of `"dynamic_max_choices":{`. CONFIRMED.
- `modal: Option<ModalChoice>` field → `ability.rs:13275` — NO skip_serializing_if ⇒ serializes `"modal":null` when None, `"modal":{` when Some. So `"modal":{` substring ⇔ a parsed modal node exists. CONFIRMED.

## A1 RE-MEASUREMENT (current card-data, 10-card marker class) — fully discriminating
Class = cards whose oracle (lc) matches `choose up to (x|that many)`:
| card | supported | gap | modal_node `"modal":{` | dyn_max `"dynamic_max_choices":{` | detector fires (modal && !dyn) | outcome |
|---|---|---|---|---|---|---|
| Hawkeye, Master Marksman | false | 0 | TRUE | false | **FIRES** | stays RED (honesty guard) ✓ |
| The Ruinous Wrecking Crew | false | 0 | TRUE | TRUE | no | greened by line-counter fix ✓ |
| Tranquil Frillback | false | 0 | TRUE | false | FIRES | already red (Hawkeye-shape) ✓ |
| Bumi, King of Three Trials | false | 1 | TRUE | false | FIRES | already red (gap1); reason explicit ✓ |
| Riku of Many Paths | false | 1 | TRUE | false | FIRES | already red (gap1) ✓ |
| Discordant Dirge | false | 1 | false | false | no | already red (gap1) ✓ |
| Reap Intellect | false | 1 | false | false | no | already red (gap1) ✓ |
| Suppression Ray | false | 1 | false | false | no | already red (gap1) ✓ |
| **Heroic Feast** | **true** | 0 | **false** | false | **no** | **stays GREEN — NO REGRESSION** ✓ |
| **Temporal Firestorm** | **true** | 0 | **false** | false | **no** | **stays GREEN — NO REGRESSION** ✓ |

→ BLOCKER A1 RESOLVED by adding `"modal":{` requirement. Both green cards have NO modal node (their "choose up to X/that many <nouns>" is a non-modal selection clause) → detector silent. ZERO green regressions.

### A3 line-counter recognizer is independently safe on greens
- Heroic Feast: 0 bullets; Temporal Firestorm: 0 bullets. Recognizing their "choose up to" line as a header flips in_modal=true but the FOLLOWING line is non-bullet → in_modal cleared, counted normally → effective_oracle UNCHANGED. No flip.
- Ruinous: 4 bullets. Recognizer folds them: effective_oracle 6→2; effective_parsed=4 (2 items ×(1+1)) ⇒ 2≤4 no SilentDrop ⇒ GREEN.
- Hawkeye: 3 bullets. Recognizer would fold: effective_oracle 5→2; effective_parsed=4 (FS=1, Reach=1, trigger w/1 child=2) ⇒ 2≤4 ⇒ would FALSE-GREEN. ⇒ detector mandatory. CONFIRMED.

parse_details measured (coverage-data.json): Ruinous = 2 items each children=1 all supported, gap0; Hawkeye = [FS children0, Reach children0, trigger children1] all supported, gap0.

## Sub-plan B — engine seams RE-LOCATED (new base) — ALL in effects/mod.rs unless noted
- `drive_repeat_for_outermost` → **3972**; gate `repeat_for_outermost_with_scope_or_unless` → **3962-3967** = requires `player_scope.is_some() || unless_pay.is_some()`. Hawkeye has NEITHER ⇒ NEVER enters this driver. **B1 review finding CONFIRMED: old plan cited wrong fn.**
- **Real path (B2):** all inside `resolve_chain_body` (**4815-5923**).
  - Up-front optional gate: **5403-5448** — `if ability.optional && !has_kind_driven_repeat && !has_member_driven_repeat_after_hydration` → fires ONE `WaitingFor::OptionalEffectChoice` @ **5441**, stashes `pending_optional_effect` @ 5428, `return Ok(())` @ 5447. Hawkeye (optional, no kind/member) enters here ⇒ ONE up-front yes/no.
  - `resolve_optional_effect_decision` → **1733-1807**: Accept sets `optional=false` @ 1740 + `optional_effect_performed=true` @ 1743, then `resolve_ability_chain` @ 1747 (re-enters with optional cleared).
  - Engine handler `handle_optional_effect_choice` → `engine_payment_choices.rs:28`; calls `resolve_optional_effect_decision(...,1)` @ **57**.
  - Repeat loop: member_driven @ 5698-5710; kind_driven @ 5733-5742; `base_iterations` @ 5754-5766 (Fixed(3)→3 via `resolve_quantity_with_targets` @ 5762); `repeated_full_chain = repeat_for.is_some() && effective.sub_ability.is_some()` @ **5800-5801**; loop `while iteration < iterations` @ 5802.
  - **Per-iteration optional TEMPLATE (kind/member only):** `iter_ability.repeat_for=None` @ 5836-5838 (`if kind_driven || (member_driven && iter_ability.optional)`); resolve branch @ 5854-5868: `if repeated_full_chain` → clones, clears repeat_for, `resolve_ability_chain(full_chain_iteration)` @ 5859 (re-resolves PayCost + modal EVERY iteration — the bug); `else if (kind_driven || member_driven) && iter_effective.optional` → `resolve_ability_chain` @ 5865 (the per-iteration optional path); `else resolve_effect` @ 5867. `if repeated_full_chain { return Ok(()) }` @ 5920-5922.
  - **B2 CONFIRMED: plain Fixed-count (`repeated_full_chain` w/o kind/member) has NO per-iteration optional** — optional applied ONCE up front (5403), then full chain run N× mandatorily (5854/5859). The new Fixed-count per-iteration-optional + early-stop driver is FIRST-CLASS new work, modeled on the kind/member branch @ 5836-5865.
- **B3 flag:** `cost_payment_failed_flag` set by PayCost handler `pay.rs:40` (no payer) / **233 / 257** (payment failed); set @ effects/mod.rs:5005 (missing chosen player). Cleared @ effects/mod.rs:**5078** = inside the `player_scope` driver loop (NOT Hawkeye's path). Hawkeye's repeated_full_chain loop (5800-5922) NEVER clears it per iteration ⇒ a declined/failed payment leaks to the reflexive. Fix belongs in the new driver. WhenYouDo eval → **6885-6887**: `WhenYouDo => !(matches!(ability.effect, Effect::PayCost{..}) && state.cost_payment_failed_flag)`.
- **Reuse spine (re-confirmed, new lines):**
  - `modal_choice_for_player` → `ability_utils.rs:543-576`; dynamic clamp @ **571-573** = `resolve_quantity(state, expr, player, source_id)` then `.min(modal.mode_count)`. **NOTE: resolve_quantity gets only (state, player, source_id) — NO ability** ⇒ the new `TimesCostPaidThisResolution` MUST resolve from a GameState field.
  - `build_chained_resolved` → `ability_utils.rs:201`; `build_target_slots_labelled` → `ability_utils.rs:386`.
  - `WaitingFor::AbilityModeChoice` → `game_state.rs:3607`; `OptionalEffectChoice` → 3630. `acting_player()` @ **4542** (covers AbilityModeChoice @ 4609); `acting_players()` @ **4689** (covers OptionalEffectChoice @ 4630) ⇒ MP routing reuses them, NO session.rs arm (grep session.rs = 0). [NB: earlier 4464/4611 grep was the MAIN worktree old base — corrected to wt-msh-f.]
  - `GameAction::SelectModes` → `actions.rs:333`; `GameAction::DecideOptionalEffect{accept}` → `actions.rs:444` (response to OptionalEffectChoice).
  - `handle_ability_mode_choice` → `engine_modes.rs:19`.
  - AI candidates: `DecideOptionalEffect` yes/no @ `candidates.rs:441/446`; `SelectModes` generation @ **1310/1396**, `AbilityModeChoice` @ **1372** (reads resolved cap off WaitingFor).
- **resolve_quantity** → `quantity.rs:68` → `resolve_quantity_with_ctx`. CostXPaid reads `obj.cost_x_paid` (quantity.rs:1699/2334). New ref needs a value arm reading the state counter.
- **B1 add-engine-variant gate:** inventory `*Paid/*Count` = CostXPaid(4500), KickerCount(4503), AdditionalCostPaymentCount(4507), AdditionalCostPaymentCountFor(4511), ConvokedCreatureCount(4516) — ALL cast-time (CR 601.2/702.33/702.51, read from source spell/object). NO resolution-local payment-count ref. `TimesCostPaidThisResolution` = resolution-local (CR 603.12a/608.2c) ⇒ Stage1 DOES_NOT_EXIST, Stage2 EXTEND_OK (different CR section ⇒ not a leaf-param of cast-time refs), Stage3 WITHIN_SECTION (CR 603.12a). APPROVED.
- **Ruinous provenance (pre-existing, re-confirmed):** `cost_x_paid = ability.chosen_x` `casting_costs.rs:5601`, stamped `obj.cost_x_paid = Some(x)` @ **5641** (CR 107.3m). `FilterProp::Token => obj.is_token` `filter.rs:3179` (Destroy-token mode). Tests: `player_scope_all_sacrifice_iterates_each_player` `sacrifice.rs:1064`; Walking Ballista X-counters `engine.rs:7979`.

## CR VERIFICATION (grep MagicCompRules.txt — all hits confirmed)
603.12 (2656), 603.12a (2659 — verbatim "pay a cost multiple times... reflexive triggers ONLY ONCE" = Hawkeye), 700.2 (3203), 700.2a (3205 — illegal mode can't be chosen, modal SPELL/activated, "see 601.2b"), 700.2b (3207 — illegal mode can't be chosen, **modal TRIGGERED ability**, "if no mode chosen, removed from stack"), 700.2d (3211 — can't choose same mode twice unless permitted / dynamic cap), 700.2e (3213 — **a DIFFERENT player chooses the mode** — NOT "illegal mode"), 107.3m (488), 601.2b (2459 — modal announcement DURING CASTING), 608.2c (2793), 701.26 Tap (3518), 701.21 Sacrifice (3449), 119.3 lose life (1065), 122.1 counter (1178).

### B4 CR CORRECTION (measured, supersedes review)
Review B4 said "CR 700.2e is more precise than 601.2b for 'illegal mode can't be chosen'." MEASURED WRONG: 700.2e (line 3213) = a player OTHER than controller chooses the mode (N/A — Hawkeye's controller chooses). The precise rule for Hawkeye's reflexive (TRIGGERED) modal "illegal mode can't be chosen" is **CR 700.2b** (line 3207). 601.2b is casting-time announcement (N/A to a reflexive triggered modal). r2 uses 700.2b + 700.2d; drops 601.2b/700.2e for the reflexive modal.
