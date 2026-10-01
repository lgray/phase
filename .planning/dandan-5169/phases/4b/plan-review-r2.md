# Phase 4b plan review r2 (phase-plan mode, FRESH)

MODEL: claude-sonnet-5-5
Scope: whole revised `plan.md` (effect-level nodes keyed by `ContinuousEffectGroupKey`), against charter entry, HEAD code, executor r1 (`executor-r1.md`, `full-r1.txt`, `candidate-r1.patch`). Read-only; no cargo run (not needed: every question settled by reading; `pgrep -x cargo` empty).
Phase-fit: 1 unit, 3 counted paths, no registration surface; neither trigger fires; Sizing is consistent with the Scope matrix (blocking check passes).

## VERDICT: ACCEPT after 2 small `text` fixes (0 `behavior`, 0 `machinery`)

The design is sound. The effect-as-node move is what CR 613.8a/613.8b/613.7 say (all three speak of "effects"), and it keeps a 707.9b copy exception with its own copy effect.

## Measured checks (all by command or read)

- CR text grepped in `docs/MagicCompRules.txt`: 613.6 (l.3007), 613.7/613.7a (3013/3015), 613.8/613.8a/613.8b/613.8c (3042-3048), 707.9b (5672). Plan quotes match. 613.2a not separately re-read (copy layer 1a, uncontested).
- `ContinuousEffectGroupKey` (layers.rs ~200) has exactly three variants (Static, Transient, GrantedStatic), derives Hash+Eq, and is the key of `started_effect_sets` (`started_effect_sets.get(key)` in `apply_continuous_effect_filtered`). So the ordering and the applier agree on effect identity.
- Entry sources, by reading each `ActiveContinuousEffect {` literal:
  - printed statics (def_index Some, transient None): key Static, one key per static definition, all its modifications;
  - transient (def None, transient Some(tce.id)): key Transient; a tce's `CopyValues`+`SetName` share it (the room test shape);
  - granted statics (def None, transient None, origin Some(host origin incl. modification_index)): key GrantedStatic, one per host grant per recipient;
  - granted triggers (origin Some): one GrantedStatic group per host grant per recipient (fine, one effect);
  - granted activated abilities, Ring emblem, stickers: key None, each its own node (equal to entry-level; the `depends_on` guard triple already gives them no edges among themselves).
  - CDA: `characteristic_defining` is per definition, so one effect never splits across the CDA-first partition.
- Guard consistency: equal keys imply equal `(source_id, def_index, transient_id)` triples in every arm, so no `depends_on` edge joins two entries of one effect. (The plan words this as load-bearing for termination; it is not, since `P != Q` is skipped and edges are deduplicated. See T2.)
- `depends_on(a, b)` reads `a` only through `a.affected_filter` plus the guard, and `b` through its modification, so all entries of one effect share incoming dependencies (they become eligible together). Confirmed by reading the whole function.
- E1 discrimination: `full-r1.txt` has the room-name test FAIL at engine_tests.rs:2642 with left "Wrong Turn", right "Bright Hall" under the entry-level rule. Re-derived G2 by hand: CV1 and CV2 mutual (unconditional CopyValues arm, different transient ids), SN1 depends on CV2 only (guard removes CV1), CV2 does not depend on SN1. Entry-level gives `[CV1,CV2,SN1]` (red); effect nodes E1={CV1,SN1}, E2={CV2} form one SCC, all edges deleted, rank order gives `[CV1,SN1,CV2]` (green); whole-bucket base also gives that, so G2 and E1 are green at base. At the final E1 stage the Copy bucket holds three copy effects (two on bear, one on bear2), all one SCC, so pure rank order and "Bright Hall". SetName is `Layer::Copy` (types/layers.rs:114), so riders share the bucket.
- Phase 5 P4: Hacks are single-transient effects, so the chain pair, loop pair, third-object scenario and the B,A,C chain-plus-loop row derive as H2 (re-ran the Kahn trace by hand: stall on SCC {A,C}, then A, B, C). `mutually_copying_*`: the copy-granted `SetName`s are printed statics on left/right (own effects), so effect and entry level agree there.

## Findings

### T1 (text, blocking-minor): the "acyclic order can change" statement is incomplete
Plan §5 "Behaviour change on acyclic buckets" says effect nodes can change an acyclic order "only where ... an older dependent entry D depends on a strict subset of the effect's entries". There is a second, edge-free case. The sort key is `(!cda, timestamp, source_id, def_index, mod_index)` and does not include `transient_id` or the grant origin. Two granted-static effects on one recipient get timestamp `host_timestamp.max(recipient_timestamp)` (layers.rs, `expand_granted_static_effects`), `source_id = recipient`, `def_index None`. When the recipient is the newest object both effects get equal timestamps and today's entry order interleaves them by `mod_index` (G1m0, G2m0, G1m1, ...). Node expansion makes each effect contiguous (G1m0, G1m1, G2...), with no dependency edge involved. (Transient effects get distinct `next_timestamp()` values, so they do not interleave unless replay-installed.) This is rules-correct (613.7a: each effect keeps written order), but the sentence is false as written.
Old string: "Effect-level nodes can change an ACYCLIC order only where one effect's entries are treated differently by today's entry-level pass."
Replacement: "Effect-level nodes can change an ACYCLIC order in exactly two ways, both rules-correct. (1) An older dependent entry D that depends on a strict subset of a multi-entry effect's entries can currently apply between two entries of that effect; under effect nodes it waits for the whole effect. (2) Two different effects whose entries tie on `(timestamp, source_id, def_index)` (granted statics on a recipient newer than both hosts, timestamp `host.max(recipient)`) interleave by `mod_index` today; under effect nodes each effect's entries stay contiguous (CR 613.7a). Both need a multi-entry effect in the bucket; the full suite in step 4 is the closure."

### T2 (text, non-blocking): over-claims
(a) §3 last bullet of "Consistency" and §5 step 4 say termination "rests on" no edge between entries of one effect. It does not: node pairs with `P == Q` are skipped and edges are deduplicated, so a singleton source node has in-degree 0 regardless. Replace "which the §5 termination argument needs" with "which keeps the node graph free of self-edges (they are skipped regardless)". Also the §7 "Unreachable hostile" sentence ("rests on the §3 consistency argument") should read "follows from the P != Q skip and edge deduplication".
(b) §10 and §11 say Phase 5 P4 "states 'one effect each'" for Hack effects. In Phase 5 `plan.md` that phrase is only about two Hacks on two different objects ("Y1, Y2 (one effect each)"), not about the entry count of one Hack. Replace with: "§3.3 builds each Hack as one `SubstituteTextWord` modification per transient (parser-produced via `ParentTarget`); if a Hack ever carried two Text-layer modifications they stay contiguous (§5 step 5), so P4 derives either way."

### N1 (non-blocking, advisory)
- G2 and H7 do not fail on a revert of the loop-only fix (G2 is green at base by design; H7 and G1 are new-helper tests). That is fine because G2's job is to fail the r1 design. The plan should prove that red instead of citing r1 history: add to step 4 a one-line mutation check "temporarily make `effect_nodes` return singletons; confirm G2 and E1 go red; restore; record in the report". Without this, E1/G2 red-under-entry-level rests on `full-r1.txt` only.
- H7's "with G2 it pins that expansion keeps A's entries adjacent" over-reaches: G2 has no dependent, and H7 is graph-level only. No row drives `order_with_dependencies` with a multi-entry loop effect and an outside dependent. Risk is low (expansion is by construction), but either soften the sentence to "H7 pins the node-level order; G2 pins the entry expansion without a dependent" or add one `order_with_dependencies`-level row.
- Cost of `effect_nodes` (one key lookup per entry, `ObjectIncarnationRef` clone) is negligible next to the existing O(n^2) `depends_on` pass; no concern.

## Disposition
Plan is acceptable once T1 and T2 are applied (text only, no design change, no change to Sizing, scope matrix or the verification rows' truth). No `behavior` or `machinery` findings; no further plan round is needed beyond the text edits.
