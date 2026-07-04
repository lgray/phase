# PR-6.75 plan review — ROUND 4 (fresh fable xhigh, confirmation, 2026-07-02)

VERDICT: CLEAN-WITH-NITS — 0 blockers / 0 majors / 0 minors / 2 nits. "Ship it to implementation."

All F1-F5 closures GENUINE (independently re-derived from primary evidence: resolver code at HEAD,
card ASTs via jq, corpus jq, CR text). Cactuar Phase-arm re-disposition SOUND (verified: mode Phase;
None-pin ⇒ no-op write is conflict-free conditional on the impl pin, fail-closed if unpinnable;
source-fallback ⇒ writes_self cannot feed a sibling's member-private SelfRef predicate; correctly
NOT category-(1)). F2 floors reproduced by fresh jq (57 triggers/56 cards, dup = keeper of the
accord x2; 38/37). Doom + Automaton rows verified on measured ASTs. pr65-wt HEAD 2e7ad800c, zero
commits since, zero anchor drift (ability_scan comment-only hunks, equal add/del in place).
13 CR spot-checks pass (incl. full 904.3 two-copy text). Zero regressions to the confirmed ledger.
Mission items 1-6 ✓; all adjudication items ✓ (7 proof-gated rows canonical, strict proof-gate at
merge, ledger row 5 priced work-list, axis-3 byte-identity strengthened post-inc2b, zone_changes
retained, series bookkeeping w/ content-signature re-verification mandate).

## The 2 nits (BOTH APPLIED to the plan by the driver, same day, per this report's exact spec)
1. §2 rule 1 root-walk: "TOPMOST OBJECT REFERENT" contradicted the resolver-mirror clause in the
   same sentence (resolver = LAST object the chain wrote = NEAREST object-referent ancestor). All
   printed chains coincide (Wedding/Managorger/Egg/Ring/Thing verified by jq) — zero decision
   impact — but "topmost" was fail-OPEN on a hypothetical SelfRef-rooted chain with an external-
   object intermediate, and the parity sweep can't catch unprinted shapes. FIX: nearest-ancestor
   wording, resolver-pinned like the parentless clause. APPLIED.
2. Parentless clause (a)/(b) should be marked instantiations, not an exhaustive partition — the
   resolver's ParentTarget arm has further event-object shapes (Stationed/VehicleCrewed/Saddled,
   blocked-attacker; targeting.rs:924-941) classifying like (a). Both printed parentless roots are
   Phase (b). APPLIED.

Post-application state: plan FINAL (R4 record line added to the status header).
