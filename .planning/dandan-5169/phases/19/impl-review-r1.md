MODEL: claude-sonnet-5-5

# Phase 19 implementation review r1 (phase mode)
Range 61f66c92e9..1cf3bd4c85 (single commit; 3 scope paths, no `types/` change, so no serialized shape, bindings or protocol delta). W start and end: `git status --porcelain` empty, HEAD == candidate, `git diff 1cf3bd4c85` empty; probe-touched files restored from `dandan-run/scratch/rev19/` copies, sha256 identical (`orig.sha`).

## Verdict: CLEAN. behavior 0, text 0, machinery 0.

## Card-first derivation (before reading code)
Cards read by jq over card-data.json (Magical Hack, Crystal Spray, Sleight of Mind, Terror, Sunken Hollow, Bog Wraith, Bad Moon: verbatim texts as in the plan). CR 613.8a/b/c, 613.1c-layer, 612.1/612.2, 305.6, 205.3i grepped; 613.8a's "what it does to any of the things it applies to" is the subject of the relation. Island, A=Island->Swamp, B=Forest->Island, C=Swamp->Forest: only C depends on A at the start (A creates the Swamp C rewrites); after A, B depends on C; so A, C, B in both cast orders gives Island/{U}. Forest start ABC: B (A waits), then A, then C: Forest; CAB: C (no-op), B, A: Swamp. Swamp start ABC: A (no-op), C, B: Island; CAB: Swamp. Terror: A, C, B returns "black" to black (nonblack, white victim legal). Sunken Hollow A then B: A makes a second Swamp that B rewrites, so [Forest] in both orders. All equal the test expectations.

## Lens results
- Authority reuse: `select_next_effect` is the one kernel (callers: ability-layer selector in `layers.rs`, `apply_in_dependency_order`); the `SubstituteTextWord` arm of `depends_on` is gone (`git grep` of the symbol in `layers.rs` leaves only the bucket skip, the incremental-path guard, the loop arm and tests). The text routine restates the select/apply/repeat loop because its edge producer clones the recipient text instead of `GameState`; it is a second producer feeding the one selector, not a second selector.
- Clone cost, measured (dev test profile, `evaluate_layers` on a 31-permanent board, one Island recipient, `k` live Fixed substitutions, 200 evals, probe row removed afterwards): k=0 564us, 1 829, 2 1265, 3 1670, 5 2640, 10 7354. Excess over the linear k*265us trend: +0.17ms (k=2), +0.31 (3), +0.75 (5), +4.1 (10). Real boards carry 2-3 text changes on one recipient; not a defect.
- Class sweep (set = every reader of text-substitution order): `git grep -n 'active_text_substitutions\|apply_battlefield_text_substitutions\|restamp_resolving_spell_text\|order_active_continuous_effects'` over crates: non-test callers of `active_text_substitutions` are exactly `apply_battlefield_text_substitutions` (called from `evaluate_layers`) and `restamp_resolving_spell_text` (called from `stack.rs::resolve_top`); both go through the routine. `order_active_continuous_effects(Layer::Text, ..)` has no remaining caller; the other hits are Copy/SetPT/Ability layers. Positive control: the same grep prints the definitions and the integration-test uses. Remaining `SubstituteTextWord` hits are exhaustive matches, the bucket skip and the incremental-path bail. I could not construct a further member: with a single `(from, to)`, `from != to`, per domain, a provider changes X's rewrite only by removing or creating `X.from` instances at classified carriers, which the count measures through the rewrite authority, and the type-line repeat case is held by the deferred collapse.
- CR annotations: every CR number in the added lines (205.3i, 608.2b, 612.1, 612.2, 613.7, 613.8a, 613.8a-c, 613.8b) resolves in docs/MagicCompRules.txt and its text matches the annotated code. Comments are single-sentence; `select_next_effect`'s doc carries the loop-rule invariant moved from the old inline comment.

## Mutation probes (production arms, each alone on the final source, `-E 'test(/text_substitution_cr612/) | test(/game::text_substitution::/)'`; unmutated control: same filter, 33 passed incl. my timing row, 0 failed)
| mutation | red rows (first panic is the discriminating assertion, never a guard) |
|---|---|
| M1 word-pair predicate in `dependency_edges` | `land_type_cycle_*` (CAB left Swamp), `cycle_verdict_*` (Forest ABC left Island), `land_cycle_on_a_keyword_*` (CAB left Forest) |
| M2 edges computed once before the loop | `land_type_cycle_*`, `cycle_verdict_*`, `land_cycle_on_a_keyword_*`, `color_cycle_on_a_static_*` ([2,3,2]), `color_cycle_on_a_stack_spell_*` (Battlefield), `restamp_*` |
| M3 no kernel, timestamp order | the six above plus `chain_of_text_changes_*`, `chain_plus_loop_*`, `type_line_repeats_*` (9 red) |
| M4 subtype collapse inside `apply_to_permanent_text` | only `type_line_repeats_*` (left ["Swamp","Forest"], right ["Forest"]) |
| M7 edge direction inverted | 9 red incl. `chain_of_text_changes_*`, `type_line_loop_*` |
| M8 type-line occurrences not counted | `land_type_cycle_*`, `cycle_verdict_*`, `type_line_repeats_*` (["Forest","Forest"]) |
Reach-guards (A,B only; C,B only; no sleight; A,B on Terror/Sirocco) stayed green under every mutation and equal the base values, so each leg reached the code. Maintainer's controls: false three-cycle read while Crystal Spray is live (V1, `land_after` reads the same turn; C-less guard differs, proving C live), genuine type-line loop with timestamp fallback (`type_line_loop_*`, preservation row: green at base and in every mutation but M7), different starting words (V1x, M1/M2/M3 red), Terror stack row (V2p, M2/M3 red; M1 not discriminating, as planned), type-line dedupe (V1d, M3/M4/M7/M8 red; preservation row at base, discriminating only through M4). Row-honesty: the executor table labels `type_line_loop_*` and `type_line_repeats_*` preservation, and the test docs claim only the rule they exercise.

## Findings
None.

## Pre-existing / out of range (untagged, non-blocking)
- `ManaColorSpent` symbol-vs-word provenance (review-9669-3 MED) is chartered to Phase 20; nothing in this diff forecloses it (`WORD_CARRIERS` untouched).
- The kernel's older-loop-member clause is pinned only by the unit row `select_next_effect_follows_the_cr_613_8b_loop_rule`; no production row reaches it (kernel body unchanged from the ability layer).
