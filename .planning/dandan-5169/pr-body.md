<!-- DRAFT (run parked 2026-10-01). Complete after run-level acceptance: Files changed, Verification, Gate A, Final review-impl. Do not open until all 17(+4b) phases are accepted. -->
## Summary

Adds the Dandan format (closes #5169): two players share one 80-card library and one graveyard, with a CR 103.5 declare round, a free-reveal mulligan, hand-entry ownership, an in-game simultaneous-draw dealer and the Day's Undoing wheel split, plus the CR 612 text-changing primitive (Magical Hack, Crystal Spray) and the Memory Lapse swallow-check fix.

## Files changed

<!-- fill from `git diff --stat <merge-base>..HEAD` at handoff -->

## Track

Developer

## LLM

Model: claude-sonnet-5-5 (executors, planners, reviewers); claude-opus-5-5 (orchestration)
Tier: Frontier
Thinking: high

## Implementation method (required)

Method: /engine-implementer

## CR references

CR 100.4, 100.6a, 103.4, 103.5, 108.3, 110.2, 121.2c, 400.1, 400.3, 401.2, 401.3, 401.4, 612, 613.7, 613.8a–c, 614.1a, 701.6a, 701.30a, 707.9b, 724 (final list from `git diff` at handoff).

## Verification

- [ ] Required checks ran clean, or the exact CI-owned alternative is stated below.
- [ ] Gate A output below is for the current committed head.
- [ ] Final review-impl below is clean for the current committed head.
- [ ] Both anchors cite existing analogous code at the same seam.

Notes for reviewers:
- `cargo ai-gate` (no `--refresh-baseline`) runs once at run level after Phase 17; Phase 3's AI change is reachable only from fixed-deck formats.
- Dandan is best-of-one (engine-enforced ceiling axis); the tournament form keeps a Bo3 default as display metadata.
- Free reveal is repeatable while the player's regular-mulligan count is 0.
- Known boundaries (no Dandan card affected): record-door "your graveyard" triggers stay owner-based in a shared graveyard; shape-based `depends_on` false loops (Blood Moon + Urborg) are pre-existing.
- Includes a fix to CR 613.8b dependency-loop ordering (whole layer previously fell back to timestamp order) and to Clash revealing the bottom card (CR 701.30a).

## Gate A

Gate A PASS head=<40-hex-sha> base=<40-hex-sha>

## Anchored on

- <fill at handoff>

## Final review-impl

<fill at handoff>
