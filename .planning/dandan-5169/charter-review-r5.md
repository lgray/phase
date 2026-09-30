# Charter review r5 (charter mode, revision r4 = decision revision re-scoping Phase 5)

MODEL: claude-sonnet-5-5

Method: `diff phase-charter.r3 phase-charter`, read the Phase 5 plan's Sizing / "Charter scope additions" / matrix rows P4, S1, L1, read `layers.rs` `order_with_dependencies` and `depends_on` and `stack.rs` `resolve_top` at HEAD, grepped CR 608.2b, 612.x, 613.8a-c and the charter for layers/stack/resolve_top. Read-only, no cargo.

## Verified clean
- No accepted phase moved. The diff touches Phase 5 (title label, scope rule, claims, verification, seams, T1/T2), one clause in Phase 11's T1/T2 line (remaining phase, sequencing note only), and the run-level note's LOC sentence. Phases 1-4 are byte-identical.
- The Phase 5 re-scope is justified by measurement. `resolve_top` has a CR 608.2c re-stamp block that the spell-text re-stamp sits beside, and its `SetTextName` hits are all in the `#[cfg(test)]` module, so `stack.rs` is not compiler-forced. `order_with_dependencies` at HEAD returns the whole timestamp-sorted bucket on any stalled Kahn pass, and its own comment says so and cites CR 613.8b.
- CR 613.8b text (grepped): dependent effects form a loop -> "the effects in the dependency loop are applied in timestamp order". The charter claim is phrased as a claim bought by a discriminating test, not as a reading.
- Seams: the only later-phase `layers.rs` mention is Phase 11's forced literals, stated as other functions. No later phase edits `stack.rs` or `resolve_top`. Other "layers.rs" grep hits are `players.rs` substrings.
- New verification rows are discriminating as written. The loop-only test is red at base (base returns whole-bucket timestamp order) with a paired whole-loop positive guard. A non-text-layer loop is constructible because `depends_on` makes two type-changing effects with type-referencing filters mutually dependent. The spell-on-stack row has a no-change baseline in the same test.

## Findings

### R5-1 (decision revision, class: design) Phase 5 is two units, and the T1 "fails" argument contradicts the charter's own test design
The r4 T1/T2 text calls the CR 613.8b loop-only rule "a lockstep part" of the primitive. Yet the charter requires its discriminating test to be red on a NON-text layer "so the fix is shown independent of the new variant". That is a variant-independent mechanic. It has its own CR subsection (613.8b, not 612), it changes shared ordering for every layer, and it is independently shippable and testable. The skill defines a unit as one coherent mechanic by one checklist pass, so this is a second unit. T2 already fires at 30 paths, so T1 (>=2 units) AND T2 holds and the recursive gate fires.
The plan's "no half-built primitive across a phase boundary" rationale does not apply. The loop fix is infrastructure that lands green first, and infrastructure -> consumer is the preferred T3 seam. Only the chain-plus-loop-on-one-bucket case (plan S3-1) needs it. A whole-loop bucket is already handled by the current fallback.
Required: split Phase 5 into 5a (`game/layers.rs` `order_with_dependencies` loop-only rule, unit test, non-text-layer discriminating test, positive reach-guard, no dependence on the new variant) and 5b (the primitive, with `stack.rs` and the remaining `layers.rs` edits). 5a goes first. Each phase gets its own T1/T2 line. The split carries Phase 5's loop count. Alternatively the charter must show why the loop fix is not a second unit, which the non-text-layer test requirement undercuts.

### R5-2 (decision revision, same revision as R5-1: acceptance row) No regression guard for real boards on a shared-machinery change
The HEAD comment says loops are "unreachable today". `depends_on` makes any two type-changing effects with type-referencing filters mutually dependent, so real-card layer-4 loops look reachable (read, not probed). The charter calls the change "confined to the loop-only rule" but has no acceptance row that a real-card loop keeps its CR-correct result, or that the existing layers suite stays green. The plan itself lists the latter as UNESTABLISHED.
Required: add to the loop-fix phase's verification a real-card (verbatim Oracle text) layer-4 loop case with its expected CR 613.8b result, plus "the existing layers suite passes unchanged" as an acceptance row.

### R5-3 (correction) Seam note undercounts later bumps
Old -> new: "The protocol bump here precedes Phase 14's and Phase 16's; each amends the doc entry" -> "The protocol bump here precedes every later phase's bump (Phases 11, 12, 13, 14 and 16 each carry a serialized shape); each amends the doc entry in `crates/lobby-broker/src/protocol.rs` per the Standing rules." Their scope rules at lines 257, 278, 301, 322 already name the bump.

### R5-4 (correction) "Single entry" claim's measurement does not buy it
`grep -n 'pub fn resolve_top'` proves only that the function exists. Replace it with `git grep -n 'stack::resolve_top\|resolve_top(' -- crates/engine/src ':!*tests*'`, to be classified into production callers versus `#[cfg(test)]` at phase start. The S1 row already covers the cast path.

Note (no finding): the Phase 5 ~1,300 LOC stands unmeasured, and the plan has a large test module plus a fixture re-slice. The ~10,140 sum is far below the 30,000 ceiling, so no decision rests on it.

## Verdict
NOT clean: one decision revision (R5-1 and R5-2 as one revision) plus two corrections. Counts: decision revision 2 (one revision), review-only 0, correction 2.
