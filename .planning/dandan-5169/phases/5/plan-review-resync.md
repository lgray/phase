MODEL: claude-sonnet-5-5
# Phase 5 plan resync review (diff: plan.r5.pre.md vs plan.md)

Method: `diff` of the two texts (9 hunks: header line 3, §3.3 line 111, §6.1 line 166, §8 lines 203/233/238, P4 row 254, step 5 line 285, ledger 308-309); read 4b/plan.md H1-H6, H2 and hand-off; grepped residual loop-only mentions (lines 3,111,166,203,254,285,308,309 only; all reworded).

## (a) Removal is exact
All SCC/loop-only implementation text (Kahn-stall algorithm, source-component choice, edge removal, mutually-copying-test note, `order_with_dependencies` helper unit tests, scope/step/matrix mentions) is removed or re-attributed to Phase 4b. No ordering code or test for it remains in Phase 5. Step 5 now says "no edit to `order_with_dependencies`". PASS.

## (b) Everything else unchanged
No hunk touches types, latch, stack seam, parser, pre-pass, escalation, probes, or other matrix rows. The `depends_on` arm, recipient guard (1) and per-recipient grouping (2) text is byte-identical apart from the (3) attribution. PASS.

## (c) P4 chain+loop row
Asserts the derived CR 613.8b reading (A,B,C or A,C,B both give Forestwalk; I re-derived the edges: A/C loop via b.from==a.from, B depends on A via A.to==B.from, no other edges). Correctly red without 4b (whole bucket gives B,A,C = Plainswalk) and matches 4b's H2. The "reverting 4b leaves chain, loop, third-object rows green and flips only chain-plus-loop" statement is correct (per-recipient grouping isolates Y; a pure loop bucket equals timestamp order).

## (d) Sizing/scope
30 paths unchanged, T1/T2 outcome unchanged, 4b's test file correctly excluded. PASS.

## Findings (real defects introduced by the resync)

F1 [text, MED] P4 row, discriminator sentence overclaims. Quote (old): "every other P4 row discriminates the Phase 5 arm (the `depends_on` arm, its symmetric loop edge, its recipient guard)."
This contradicts the very next sentence of the same cell ("Dropping ONLY the same-recipient guard ... is NOT visible in the integration scenarios ... pinned by the unit assertion"), and the `b.from == a.from` edge is likewise stated earlier in the cell to be invisible to the integration loop pair (pinned by unit). The Y1/Y2 variant discriminates neither. Replacement: "every other P4 integration row discriminates only the `depends_on` arm itself (the chain pair and the third-object X scenario go red without it; the loop pair goes red if the predicate is one-directional); the symmetric `b.from == a.from` edge and the recipient guard are pinned by the `depends_on` unit assertions, not by any integration row."

F2 [text, LOW] §3.3, "Two defects follow (the first measured against the whole-bucket fallback that Phase 4b removes) if the edge were recipient-blind or the bucket mixed recipients: two same-`from` effects on an unrelated object form a 2-cycle and revert the chain on another object to timestamp order." Only one defect is listed, and with 4b landed the "measured" defect is no longer reachable at this base (4b's loop-only rule confines it). Replacement: "One defect follows if the edge were recipient-blind (measured against the whole-bucket fallback Phase 4b removes; with 4b's loop-only rule it is bounded, not eliminated, which is why measure (1) stays): two same-`from` effects on an unrelated object form a 2-cycle whose dependency edge would be read as applying to another object's chain."
(Pre-existing "Two defects" wording was in r4; flagged only because the resync rewrote the sentence.)

Verdict: one MED text finding (F1), one LOW text (F2); no behavior or machinery defects. Apply F1, then clean.
