# Review: Doctor Doom (card/msh-doctor-doom) — msh-equip-count

VERDICT: ZERO BLOCKING FINDINGS — ready to commit.

## Measured evidence
- 4 Doctor Doom tests PASS (3 layers.rs toggle + 1 condition parse).
- Plan suite PASS (97 plan-named incl. round-trips, serde, filter, protection).
- Touched modules full pass: condition.rs 331, layers.rs 183, card_type.rs 3.
- parser-combinator gate: exit 0.

## Deviation adjudication (#1) — VERIFIED CORRECT
- (a) "a plan" is NOT a standalone inner condition: parse_inner_condition("a plan") => Err.
  => parse_condition_disjunction genuinely cannot split it. Control-layer loop is correct seam.
- (b) Plan's parse_type_phrase guard change REPRODUCED: temporarily broadened non-comma
  guard to starts_with_or_article_type_segment => 4 pre-existing prod tests FAIL:
  conditional_modal_max_supports_compound_presence_conditions,
  bridge_you_control_artifact_and_enchantment, test_recipient_is_subtype_disjunction,
  test_you_control_compound_presence. Reverted cleanly (0 removed lines).
- (c) oracle_target.rs production: 0 removed lines (purely additive tests). Confirmed.
- (d) New loop scoped to article-led RHS via peek(article). Bare-plural "or creatures"
  is consumed by PRE-EXISTING parse_type_phrase separator branch (verified: ptp returns
  Or with empty remainder), so loop's tag(" or ") fails on "" and breaks. No over-capture.

## Other checks
- inject_controller recursively distributes You + InZone{Battlefield} into Or disjuncts.
- is_permanent_type / PERMANENT_TYPES: Plan NOT added (correct; non-permanent).
- type_implies_battlefield: Plan falls to _ => false (correct).
- CR numbers 109.5/205.2a/604.1/608.2c/611.3a/702.12b all grep-resolve and describe code.
  205.2a confirmed lists NO Plan => needs-manual-verification is honest, not fabricated.
- mtgish/ untouched.
- Coverage honesty: Unrecognized -> typed IsPresent{Or}, no Effect::unimplemented, no dropped semantics.
