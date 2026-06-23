# MSH-D: Doctor Doom — resolver-flagged conditional-static (R2) + "Plan" card type

Worktree: /private/tmp/wt-msh-doctor-doom, branch card/msh-doctor-doom, off UPSTREAM/MAIN c8c1f6855
(per stale-fork hazard — NOT origin/main). Tilt DOWN → direct nightly cargo
(`export PATH="$HOME/.cargo/bin:$PATH"`). card-data/coverage gitignored; query MAIN checkout
/Users/lgray/vibe-coding/phase-rs-workdir/phase/data/.

## RESOLVER-FLAG ROOT CAUSE (gap_count=0 but supported=false)
Doctor Doom: "When ~ enters, create two 3/3 ... Doombot tokens. / As long as you control an artifact
creature or a Plan, ~ has indestructible. / At the beginning of your end step, you draw a card and
lose 1 life."
All 3 abilities show parse_details supported=true, gap_count=0, gap_details=null. BUT supported=false
(resolver/silent-drop audit). Root cause found in the static AST (card-data):
  static_abilities[0] = { affected: SelfRef, modifications: [AddKeyword Indestructible],
                          condition: StaticCondition::Unrecognized { text: "you control an artifact
                          creature or a Plan" } }
The keyword grant parses, but the CONDITION is `Unrecognized` → at runtime the conditional-static
keyword grant is silently dropped/mis-evaluated (the coverage audit flags any Unrecognized condition as
unsupported). This is the std R2 / "as long as you control <filter>, ~ has <keyword>" conditional-
static-grant class.

## WHY the condition is Unrecognized — two sub-gaps
parse_inner_condition (oracle_nom/condition.rs:54) → parse_condition_disjunction (line 65) splits on
" or " and parses each side with parse_single_inner_condition. For Doctor Doom:
  LHS "you control an artifact creature" parses; RHS "a Plan" alone is NOT a standalone condition
  (no "you control" verb) → disjunction fails. Then parse_single_inner_condition → parse_control_
  conditions → parse_you_control_a (condition.rs:2668) does parse_type_phrase("an artifact creature or
  a Plan"): parse_type_phrase consumes "artifact creature" but leaves " or a Plan" as remainder AND
  "Plan" is not a recognized type → the whole condition can't fully consume → Unrecognized.
Sub-gaps:
  1. CARD TYPE "Plan": CoreType (types/card_type.rs:49) has NO `Plan` variant. "Plan" is a real MTG
     card type (Marvel's Spider-Man / MSH set — the first "Plan" cards). CoreType::from_str("Plan")
     returns Err. So the type-phrase parser cannot recognize "a Plan". NOTE: 0 cards in the current
     card-data corpus have core_type Plan (the Plan cards may be absent from this MTGJSON snapshot),
     so the "or a Plan" branch is runtime-unreachable today — but the condition MUST still parse it
     (rules-correct: recognize "a Plan" so the artifact-creature branch works and the condition is
     typed, not Unrecognized).
  2. ELIDED-VERB DISJUNCTIVE CONTROL: "you control [an artifact creature] or [a Plan]" — one "you
     control" verb governing TWO type-filters joined by "or" (elided second verb). The existing
     parse_condition_disjunction handles "condition-A or condition-B" (each a full clause), NOT
     "you control A or B" (shared verb, filters disjoined). parse_you_control_a parses a SINGLE
     type-phrase, not a disjunction of type-phrases.

## SCOPE (run add-engine-variant gate for #1)
A. Add CoreType::Plan (new engine variant). CR 30x (card types). add-engine-variant gate: this is a
   genuine new card type, not a parameterization — CoreType is the canonical card-type enum; "Plan" is
   a distinct CR-defined type (verify the CR number for Plan in docs/MagicCompRules.txt — it may be a
   very new rule; if absent from the local CR text, flag as needs-manual-verification). Must update:
   CoreType enum + FromStr + Display/to-string + any exhaustive matches (cargo check finds them) +
   serde (card-data export/import — CoreType appears in card_type.core_types, a serialized surface) +
   any is_permanent/type-classification logic (a Plan is a nontraditional command-zone card like
   Plane/Scheme — model it after CoreType::Plane: NOT a permanent, command-zone). Serialized-surface
   audit per the gate.
B. Parse the elided-verb disjunctive control condition: "you control <type-phrase> or <type-phrase>"
   → a typed StaticCondition (Or([IsPresent(filter_A), IsPresent(filter_B)]) OR IsPresent(AnyOf(...)) —
   pick the idiom matching how existing "you control A or B" filters are represented; check whether
   parse_type_phrase / TargetFilter already has an AnyOf/disjunctive-type form so the condition is
   IsPresent{ filter: <artifact-creature OR Plan> }). Build for the CLASS (the R2 conditional-static-
   grant family + any "you control A or B" control condition), not just Doctor Doom. Delegate to
   parse_inner_condition / extend parse_you_control_a or parse_control_conditions; nom combinators only.
   The fix must make Doctor Doom's static condition a typed StaticCondition (not Unrecognized) so the
   resolver evaluates the indestructible grant.

## VERIFY the conditional-static GRANT is runtime-evaluated (the actual resolver flag)
The lead says the R2 class "isn't runtime-evaluated". Must confirm: once the condition is typed
(IsPresent/Or), does the layer/static system actually (a) gate the AddKeyword Indestructible grant on
the condition each layer pass, granting only while you control an artifact creature/Plan, and (b)
remove it otherwise? Trace AddKeyword + StaticCondition::IsPresent through game/layers.rs evaluate_
condition. If the typed condition already drives the grant (likely — IsPresent is a common condition),
then the ONLY gap is the parse (Unrecognized → typed) and "Plan". If the grant itself isn't gated at
runtime, that's an additional R2 evaluator gap to build. CONFIRM before claiming scope.

## Tests (non-vacuous, discriminating, revert-probe)
- parse: "as long as you control an artifact creature or a Plan, ~ has indestructible" → static with
  condition = typed Or/IsPresent over {artifact creature, Plan}, NOT Unrecognized. Fail-before:
  Unrecognized.
- CoreType: CoreType::from_str("Plan") == Ok(Plan); serde round-trips; Plan is non-permanent/command-
  zone (mirror Plane).
- discriminating: "you control an artifact creature" (single) still → IsPresent(artifact creature)
  (no regression); a non-control "or" condition still parses as before.
- RUNTIME (the resolver flag — strongest): a board where you control an artifact creature → Doctor
  Doom HAS indestructible (survives lethal/destroy); a board where you do NOT → Doctor Doom does NOT
  have indestructible (dies to destroy). Revert-probe: with Unrecognized condition the grant is
  dropped/always-on — the toggle proves the fix.
CR: verify card-type CR for Plan + 604 (static abilities) + 611 (continuous effects) + 702.12
(indestructible — VERIFY the exact number) in docs/MagicCompRules.txt.

## add-engine-variant GATE VERDICT: CoreType::Plan — APPROVED (structurally), CR = NEEDS-MANUAL-VERIFICATION
- Stage 1 DOES_NOT_EXIST: no CoreType::Plan; no Unknown/Other fallback; FromStr errors on "Plan" and
  synthesis.rs imports via `CoreType::from_str(s).ok()` + filter_map → an unknown "Plan" type is
  SILENTLY DROPPED on import (so a real Plan card loses its type today).
- Stage 2 EXTEND_OK: CoreType is a flat enum of the CR card types (artifact..conspiracy + legacy
  Tribal). Not a sibling-cluster/parameterization smell — each variant is a distinct card type. Plan is
  a genuine new sibling, exactly analogous to the existing nontraditional command-zone types
  Plane/Scheme/Conspiracy/Phenomenon.
- Stage 3 WITHIN_SECTION: card types, CR section 3 / CR 205.2a. Single section.
- CR ANNOTATION: ⚠ "Plan" is NOT in the CR snapshot (CR 205.2a, dated 2026-04-17, lists artifact,
  battle, conspiracy, creature, dungeon, enchantment, instant, kindred, land, phenomenon, plane,
  planeswalker, scheme, sorcery, vanguard — NO Plan). Marvel's Spider-Man postdates the CR text. So
  there is NO grep-verifiable CR number. Per CLAUDE.md: do NOT fabricate a CR number — annotate the
  CoreType::Plan variant with a `// CR: needs-manual-verification — "Plan" card type is from Marvel's
  Spider-Man (post-2026-04-17 CR snapshot); not yet in docs/MagicCompRules.txt 205.2a` comment (NOT a
  fake CR XXX). The structural model is verifiable against the existing nontraditional types; the CR
  number is the only unverifiable part and is honestly flagged.

## REQUIRED match-arm/serde updates for CoreType::Plan (gate step 2 + 6)
- types/card_type.rs: enum variant + FromStr arm ("Plan" => Ok(Plan)) + Display/to_string arm + any
  Ord/category helpers. cargo check -p engine finds every exhaustive match (NO wildcard to silence).
- Classification: model after CoreType::Plane — Plan is a nontraditional command-zone card, NOT a
  permanent. Update is_permanent / permanent-type classification + zone/command-zone logic to treat
  Plan like Plane/Scheme (verify exactly how Plane is classified and mirror it).
- Serialized surface: CoreType serializes in card_type.core_types (card-data export/import). Serde
  derive will handle the new variant; confirm no fixed-set deserialization shim rejects it. Existing
  repo-owned serialized data does not contain "Plan" (0 cards), so no migration needed, but confirm a
  card-data round-trip with a synthetic Plan still loads.

## PRIMARY FIX remains the parse (resolver flag): condition Unrecognized → typed.
With CoreType::Plan recognized, the type-phrase parser can match "a Plan", and the elided-verb
disjunctive control condition "you control <type A> or <type B>" must parse → typed StaticCondition.
The runtime conditional-static evaluator ALREADY works (StaticCondition::IsPresent gates AddKeyword
grants; Anger test layers.rs:11417 proves the toggle), so NO new evaluator — once the condition is
typed, Doctor Doom's indestructible grant is correctly gated. This is the whole resolver flag.
