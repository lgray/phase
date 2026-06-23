# PR #4182 review-fix log (card/msh-doctor-doom) — [HIGH] Plan word-boundary regression

Rebased onto upstream/main (7746ef973..3a844e56d). Worktree clean except authoring agent's untracked docs (untouched).

## Finding (matthewevans HIGH, REGRESSION) — CoreType::Plan matches Plant/Planet/Plane as unguarded prefix

VERIFIED in code:
- parse_type_filter_word (oracle_nom/target.rs:193) has a TYPE_WORDS table including ("plans",Plan),("plan",Plan)
  at lines 212-213 (added by the Doctor Doom commit ea05595e2).
- The scan at target.rs:235-239 does `input.strip_prefix(word)` and returns IMMEDIATELY on first match,
  with NO word-boundary guard. So:
  - "plant" → strip_prefix("plan") OK (rest "t") → returns TypeFilter::Plan (WRONG; should be Subtype("Plant")).
  - "planet" → strip_prefix("plan") OK (rest "et") → TypeFilter::Plan (WRONG; Subtype("Planet")).
  - "plane" → strip_prefix("plan") OK (rest "e") → TypeFilter::Plan (WRONG).
  - "planeswalker"/"planeswalkers" are SAFE only because they are listed BEFORE "plan" (longest-match-first,
    comment 209-211) — fragile ordering, not a real boundary guard.
- parse_subtype (oracle_util.rs:1454) IS boundary-guarded: uses starts_with_word_ci (1435-1449, checks the
  following char is non-alphanumeric or EOI) and parse_subtype_entry (1478, same boundary check for plurals).
  So once the TYPE_WORDS scan stops shadowing, Plant/Planet/Plane correctly fall through to the subtype table.
- "Planet" is a real subtype (card_type.rs:237, filter.rs:4151, choose.rs:277); "Plant" is a real subtype
  (synthesis.rs:14655, replacement.rs:11330-11371). So the misparse is live production fallout.
- Precedent for the fix already exists: parse_outlaw_type (target.rs:255-273) and starts_with_word_ci both
  enforce "next char is non-alphanumeric or end of input". Test test_parse_type_filter_word_outlawry_does_not_
  match_outlaw (target.rs:1004) is the existing boundary-guard regression for the outlaw head noun.

CONCLUSION: maintainer is CORRECT. This is a real [HIGH] regression — the bare strip_prefix scan is an
architectural flaw that the new "plan"/"plans" entries exposed (they collide with the common Plant/Planet/
Plane subtypes). Confirmed fallout cards: A-Phylath/Phylath/Kirri/Volatile Orbit/Drill Too Deep/Insidious
Roots/Avenger of Zendikar (all reference Plant/Plants).

## FIX (general, build-for-the-class)
In parse_type_filter_word's TYPE_WORDS scan (target.rs:235-239), add a word-boundary guard after strip_prefix:
the matched word must be followed by a non-alphanumeric char or end of input — mirroring starts_with_word_ci /
parse_outlaw_type. Apply to ALL TYPE_WORDS (general fix, not just "plan"): no head-noun type word should match
as a sub-word of a longer word. This is the maintainer's lesson: "any new CoreType/prefix-matched type word
MUST add a word-boundary guard + negative tests for prefix-colliding subtypes."

Cleanest idiom: replace the `if let Some(rest) = input.strip_prefix(word)` body with a check that also verifies
`rest.is_empty() || rest.starts_with(|c: char| !c.is_alphanumeric())` before returning. (Could also reuse
starts_with_word_ci, but it returns bool not the rest; the inline boundary check on `rest` is the minimal idiom
already used in parse_subtype_entry at oracle_util.rs:1491/1510.)

NOTE on plurals: "creatures"/"lands"/etc. stay correct — they are listed as explicit entries and the boundary
after the plural form is satisfied (e.g. "creatures you" → rest " you" starts with space). The longest-match-
first ordering remains (so "creatures" wins over "creature" + leftover "s"). Verify test_parse_type_filter_
word_plurals (target.rs:962) stays green.

## DISCRIMINATING TESTS (negative regressions + positive)
Add to target.rs test module near 1004:
- "Plant" → NOT TypeFilter::Plan; must be TypeFilter::Subtype("Plant"). Pre-fix returns Plan → fails.
- "Plants" → Subtype("Plant") (plural). Pre-fix "plans" entry matches → Plan → fails.
- "Planet" → Subtype("Planet"). Pre-fix → Plan.
- "Power-Plant" / "Power Plant" → these start with "Power" so won't hit "plan" at position 0; but confirm
  the head-noun path doesn't misfire — actually parse_type_filter_word matches at input START, so "Power Plant"
  starts with "power" (not a type word) → would go to subtype table. Confirm Powerstone/etc. don't collide.
  The maintainer specifically asked for "Power-Plant"/"Power Plant" tests — include them asserting NOT Plan
  (they should resolve via subtype or fail, never Plan).
- POSITIVE: Doctor Doom's "a Plan" (i.e. parse_type_filter_word("plan") with EOI or space follower) → still
  TypeFilter::Plan. "plan" alone (rest "") and "plan " (rest " ") must still match — the boundary guard allows
  EOI and non-alphanumeric follower. Pre-fix AND post-fix both true (regression guard for the intended feature).
All revert-probe: each negative fails on pre-fix code (returns Plan), passes after.
