# Phase 19 plan review, round 3 (phase-plan mode, phase-fit context declared; Sizing consistency blocking)

MODEL: claude-sonnet-5-5. Reviewed: delta `scratch/plan19-r3.diff` (plan.r2.md -> plan.md) plus what it references; plan base W 61f66c92e9 (layers.rs / text_substitution.rs byte-identical to the round-2 prototype's base). Delta has no design finding, so no whole-plan pass was run beyond the unchanged USER-requirement and Sizing checks below.

**Verdict: ACCEPT with 1 `text` finding (T1, replacement given; closed by applying it, no re-review). No `behavior` or `machinery` findings. r2 F1 (V1x/V1L rows) and r2 F2 (CR 205.3i) are closed. Sizing present and consistent (1 unit, 3 paths, small-change lane eligible).**

## Instruments
Scratch tree under dandan-run/scratch (`git archive` of W HEAD, round-2 prototype of plan items 1-5 copied in; tree and target deleted). Rows written by me from the plan's V1x/V1L spec (production `GameScenario` + `GameRunner::cast` via the file's `cast_text_change`, real Magical Hack x2 / Crystal Spray from the fixture, result read same turn). Base = unmodified source. Raw output: `scratch/p19r3rev-{proto,base,M2,M3}.log` (grep `^RREV`). Each leg exited `done rc=0` and printed 16 rows, so every leg reached the code; the base-vs-design differences are the positive control.

| row (A=Island->Swamp, B=Forest->Island, C=Swamp->Forest) | base | design prototype | plan expects | M2 | M3 |
|---|---|---|---|---|---|
| Island ABC / CAB / guard AB / CB | Forest{G} / Swamp{B} / Swamp / Island | Island{U} / Island / Swamp / Island | V1: Island both orders, guard Swamp | **Forest** / Island / Swamp / Island | **Forest / Swamp** / Swamp / Island |
| Forest ABC / CAB / guard AB | Island / Island / Swamp | Forest{G} / Swamp{B} / Swamp | Forest / Swamp, guard Swamp | Forest / Swamp / Swamp (green) | **Island / Island / Island** |
| Swamp ABC / CAB / guard CB | Forest / Island / Island | Island{U} / Swamp{B} / Island | Island / Swamp, guard Island | Island / **Island** / Island | **Forest / Island** / Island |
| V1L Island, A=Island->Swamp, D=Island->Forest: AD / DA / ADC / DAC | Swamp / Forest / Plains / Forest | identical | AD Swamp, DA Forest, ADC Plains | identical | identical |

Closure of the planner's measured claims, by my own runs:
- V1x: all four cycle rows (Forest ABC/CAB, Swamp ABC/CAB) are red at base and equal the plan's values under the design. Both guards (Forest A,B -> Swamp/{B}; Swamp C,B -> Island/{U}) are green at base and under the design. The planner's own base log has no Swamp CB row; I measured it (Island/{U}, green at base), so "both guards green at base" holds.
- V1L: green at base and under the design, with the plan's values; the loop rows are unchanged under M2 and M3 as the plan says ("not an M1-M4 discriminator").
- M3 (apply in timestamp order) turns all four V1x cycle rows red (and the Forest guard). M2 (edges computed once) turns only Swamp CAB red among the new rows (Island instead of Swamp), plus Island ABC from V1; the Forest rows and Swamp ABC stay green under M2. The plan says "M2 and M3 are also run against these rows and their red rows recorded" and claims nothing more, so it stands; the executor should record that V1x discriminates M2 only through Swamp CAB.

Hand derivation (CR 613.8a-c), done before reading the measured values, equals the measured design values for all six V1x rows and the V1L rows (Forest: only A->B at start; ABC applies B, then A, then C; CAB applies C (no-op), B, A. Swamp: only B->C at start; ABC applies A (no-op), C, B; CAB applies C, then A waits for B (B now makes an Island), B, A. V1L: A and D form a loop, C waits on A; loop picks the older member, then D (no-op), then C).

## USER requirement check
- Reuse the existing state-aware selection authority: satisfied in kind, unchanged from r2. `select_next_effect` extracted from the referenced-grant selector and called by both layers, the word-pair `depends_on` arm deleted, no fixed graph sees Layer 3 substitutions. The select/apply/repeat loop is restated per layer only because the edge producer differs (cloned recipient text vs cloned `GameState`).
- One effect at a time against current text: holds (M2 and M3 both red on V1/V1x).
- Failing-first production regression for the maintainer example: Island ABC red at base (Forest/{G}); rules-text variants V2/V2p/V3 per r1/r2.
- Oracle text and CRs: delta adds no card text. CRs re-grepped: 613.8a/b/c, 514.2, 305.6, 612.1, 205.3i; 205.3i now supports only "land types are subtypes on the type line" (its text lists the land types); the collapse is labeled a representation fact, so r2 F2 is closed. The sentence "the engine's subtype list holds no repeats" is a design statement for the collapse placement; the deferred collapse is what the V1d/M4 rows measure, and nothing else depends on it.

## Findings

### T1 [text] Forest-start derivation states a dependency that does not hold at the start and contradicts the same paragraph's CAB result
Plan, Reference Readings, V1x: "A is a no-op on Forest, B rewrites it, C is a no-op, and A depends on B (B creates the Island A rewrites, CR 613.8a) while C depends on A (A creates the Swamp C rewrites). Order ABC: ... Order CAB: C is independent and a no-op, then B (A waits for it), then A: `Swamp`."
Defect: on a Forest A is a no-op, so applying A creates no Swamp and C does not depend on A at the start (C depends on A only after B has made an Island); measured edges agree (only A->B). If C did depend on A, order CAB would apply B, A, C and end `Forest`, not the `Swamp` the same paragraph and the measurement give. Reference readings are acted on (check 9a), so the sentence is corrected, not left.
Replacement for the clause: "and A depends on B (B creates the Island A rewrites, CR 613.8a); C depends on nothing at the start (A is a no-op on Forest), and depends on A only once B has made an Island (then A creates the Swamp C rewrites)." The ABC and CAB derivations that follow stand unchanged.

## Pre-existing (untagged, non-blocking, not repeated)
- None new.
