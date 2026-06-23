# Plan-log: TYPE_WORDS "plan" unguarded-prefix regression fix

Worktree: /private/tmp/wt-msh-doctor-doom  branch card/msh-doctor-doom  HEAD ea05595e2 (Doctor Doom commit = tip)

## Verified facts (re-confirmed against rebased code; line numbers may drift, grep before edit)
- `parse_type_filter_word` @ crates/engine/src/parser/oracle_nom/target.rs:193.
- TYPE_WORDS table 195-224; new ("plans",Plan)/("plan",Plan) @ 212-213.
- Unguarded scan @ 235-239: `for &(word, ref tf) in TYPE_WORDS { if let Some(rest) = input.strip_prefix(word) { return Ok((rest, tf.clone())); } }` — NO boundary check.
- parse_outlaw_type @ 255-273 = precedent boundary guard (end-of-input or non-alphanumeric follower). Its regression test = test_parse_type_filter_word_outlawry_does_not_match_outlaw @ 1004.
- starts_with_word_ci @ oracle_util.rs:1435-1449 (canonical boundary helper; returns bool). parse_subtype @ 1454; parse_subtype_entry @ 1478 (plural boundary checks @ 1491/1510 = `after.is_empty() || after.starts_with(|c: char| !c.is_alphanumeric())` — the exact idiom to reuse inline).
- "Plant" IS in SUBTYPES (oracle_util.rs:1143). "Planet" IS in LAND_SUBTYPES (card_type.rs:237) reached via fixed_noncreature_subtypes() (card_type.rs:372) → parse_subtype loop @ oracle_util.rs:1462. "Power-Plant" IS in LAND_SUBTYPES (card_type.rs:238).
- "Plane" is a CoreType (card_type.rs:105/129), NOT a fixed-noncreature subtype → parse_subtype("plane") returns None → parse_type_filter_word("plane") returns Err post-fix (acceptable; "plane" was never legitimately handled and pre-fix it wrongly returned Plan).
- TypeFilter enum @ ability.rs:2366: `Plan` (unit, 2380), `Subtype(String)` (2389). 

## PRODUCTION ENTRY: parse_type_phrase (oracle_target.rs:1459) → parse_type_phrase_with_ctx (1465) lowercases input (`text.to_lowercase()`, line ~1468) BEFORE calling parse_type_filter_word. ⇒ parse_type_filter_word receives LOWERCASE in production. Tests MUST pass lowercase ("plant","planet","power-plant","power plant","plan") to mirror production and make pre-fix negatives fire.

## Doctor Doom POSITIVE test (must stay green): NOT in target.rs. It is in oracle_target.rs:
  - parse_type_phrase_recognizes_plan — parse_type_phrase("a Plan") → Typed{[Plan]}, rest empty.
  - parse_type_phrase_leaves_article_led_or_rhs_as_remainder, single_artifact_creature_still_typed_not_or, bare_connector_rhs_still_or.
  These reach parse_type_filter_word("plan") (after article strip + lowercase) with rest="" → boundary guard EOI-satisfied → still Plan. SAFE.

## Callers of parse_type_filter_word: parse_type_list @ target.rs:170-187 (lines 171 first word, 177 after " or "). After-" or " remainder always begins with the next type word; boundary follower is space/EOI/punct ⇒ guard never regresses. No caller relies on sub-word prefix matching (design check 1 = clean).

## Gate (check-parser-combinators.sh): FORBIDDEN_METHODS pattern requires `.starts_with("` (literal-quote follower). My new line uses `.starts_with(|c: char| ...)` (closure) ⇒ NOT matched. Existing `input.strip_prefix(word)` uses a variable, already grandfathered/not literal. No allow-noncombinator needed. Mirror oracle_util.rs:1491/1510 + parse_outlaw_type idiom = approved precedent.

## add-engine-variant: N/A — no new enum variant. Pure guard on existing scan. State explicitly in plan.

## FIX (general — all TYPE_WORDS entries):
```
for &(word, ref tf) in TYPE_WORDS {
    if let Some(rest) = input.strip_prefix(word) {
        // Word boundary (mirrors parse_outlaw_type + parse_subtype_entry): a type
        // word must be followed by end-of-input or a non-alphanumeric char, else a
        // longer subtype sharing the prefix ("plant"/"planet" vs "plan") is shadowed.
        if rest.is_empty() || rest.starts_with(|c: char| !c.is_alphanumeric()) {
            return Ok((rest, tf.clone()));
        }
    }
}
```
Byte-identical for passing inputs: "creatures you"→rest " you" (space ⇒ ok); "spell"→rest "" (EOI ⇒ ok); longest-first preserved.
