# PLAN — S25 P2c-Expose (Expose the Culprit, mode 2)

> Driver: s25-impl-revived. Planner: expose-planner (Opus/xhigh, read-only). Base HEAD c37108ae9.
> HARD BAR (never edit): game/effects/delayed_trigger.rs, game/filter.rs, game/effects/mod.rs.

Card (jq-verified): "Choose one or both — • Turn target face-down creature face up. • Exile any number of face-up creatures you control with disguise in a face-down pile, shuffle that pile, then cloak them."

## ⚑ FROZEN VERDICT (load-bearing): ZERO frozen edits — reuse path (disposition = option 1, NO #4990 dependency)
- effects/mod.rs Shuffle dispatch (:3044) `Effect::Shuffle { .. }` and Cloak dispatch (:3177) `Effect::Cloak { .. }` are non-destructuring → absorb. effect_object_targets NOT extended (cloak.rs does its own tracked-set read). No edit.
- filter.rs HasKeywordKind evaluator (:2842) is generic over KeywordKind → new KeywordKind::Disguise needs no arm. No edit.
- delayed_trigger.rs not involved.

## ⚑ DRIVER DECISIONS (on planner §7 open questions)
- **Q1 KeywordKind::Disguise → APPROVED (the one required new variant).** add-engine-variant PASS: parameterization impossible (Keyword::Disguise(_).kind()→Unknown catch-all shared by ~60 keywords; HasKeywordKind{Unknown} would over-match); Disguise is the exact Morph/Mutate parallel (own KeywordKind), CR 702.168. Non-frozen. Compile-forces the `.kind()` match (edited anyway) + any exhaustive `match KeywordKind` — executor enumerates + handles each (WATCH: none must mishandle Disguise by falling into a Morph/Manifest-like path).
- **Q3 Shuffle surface → REUSE `target: TrackedSet` (APPROVED, no new field).** Value change not shape change; avoids the ability_scan.rs:787 walker force. Resolver branches `if let TargetFilter::TrackedSet{..} = target`.
- **Q2 exile modeling → RULES-CORRECT END STATE IS NON-NEGOTIABLE (the #1 hard rule); mechanism is the executor's choice, GATED by a test.** The planner's lazy "elide the exile zone-change, cloak in place" path is ACCEPTABLE **only if** it produces the rules-correct end state. Rules (CR 701.58a + exile-and-return): the chosen creatures become FRESH face-down 2/2 ward-2 objects — NO carried-over +1/+1 counters, attached Auras/Equipment fall off, and any "when ~ leaves the battlefield" trigger FIRES. Executor Step-0 probe: does manifest_card on an ALREADY-battlefield permanent create a new object (reset/leave-triggers) or flip in place (preserves counters/auras = WRONG)? If lazy achieves the correct end state → lazy OK (ponytail-commented, upgrade path noted). If manifest_card preserves object state → use the FAITHFUL exile-and-return path. If the faithful path needs tracked-set-remap-across-exile infra that does NOT exist → STOP + escalate to me. **BINDING runtime test:** a chosen creature carrying a +1/+1 counter (and ideally an Aura) → after cloak, the resulting face-down permanent has NO counter (Aura fell off). This discriminates rules-correct-object-reset from in-place-flip.
- Q4 cloak.rs TrackedSet object_source read (order-preserving, non-frozen, precedent choose_from_zone.rs:600) — OK. Q5 parser combinator (unique idiom, nom mandate, bulk of work) — OK.

## ⚑ REVIEW OUTCOME — APPROVE-WITH-CONDITIONS (review ae1e9a2b; executor MUST satisfy each)

Architecture CONFIRMED (6 files non-frozen; KeywordKind::Disguise fan-out SAFE — actually FIXES merge_extracted_keywords over-stripping; TrackedSet threading sound; scan_target_filter already handles TrackedSet→Axes::NONE so NO walker edit; CRs correct; face-up implied by HasKeywordKind{Disguise}). **Q2 RESOLVED DECISIVELY: the lazy "manifest in place" path is not merely rules-wrong — it is a TOTAL NO-OP** (manifest_card→ZoneMoveRequest::effect(id,Battlefield)→move_object returns Done at the Battlefield→Battlefield guard zone_pipeline.rs:553-560 before touching the object). → FAITHFUL exile-and-return is MANDATORY, implemented NON-frozen inside cloak.rs. No new variant, no chain reshape, no escalation (object_id stable across zone changes → no tracked-set remap).

**C1 (Q2 HARD GATE — MANDATORY).** cloak.rs's TrackedSet branch must EXILE each chosen member BEFORE manifesting: clone the (shuffled) set, `move_object`/`move_objects_simultaneously` each Battlefield→Exile (real zone change — clears counters zones.rs:396, detaches Auras via battlefield-departure SBA, fires "leaves the battlefield" triggers), THEN manifest_card each Exile→Battlefield (cloaked_2_2). Entirely within non-frozen cloak.rs. Checkable: test C(a) creatures become face-down 2/2 ward-2 (FAILS under the no-op); C(d) +1/+1 counter gone AND Aura fell off.

**C2 (set-consumption trap).** Do NOT exile via a `ChangeZone{Exile, TrackedSet{0}}` chain step — change_zone.rs:1336-1337 REMOVES the tracked set after consuming it, starving Shuffle+Cloak (empty set). Exile via a direct move_object inside cloak.rs (which clones the set first), which does not touch tracked_object_sets.

**C3 (shuffle observability).** cloak.rs TrackedSet branch reads `state.tracked_object_sets[bound_id]` DIRECTLY (order-preserving), NOT effect_object_targets (that's Vannifar's ability.targets path, cloak.rs:48 — Shuffle can't reorder ability.targets). Both Shuffle and Cloak resolve TrackedSetId(0) via resolve_tracked_set_sentinel (targeting.rs:2053/2058) → same set. Checkable: test C(b) creation order = shuffled order.

**C4 (no ShuffledLibrary).** shuffle.rs TrackedSet branch reorders via state.rng and RETURNS before the unconditional PlayerPerformedAction{ShuffledLibrary} push (shuffle.rs:58-61). Checkable: test C(c) witness "whenever you shuffle your library" trigger does NOT fire.

**C5 (min:0 empty selection).** Verify + TEST the zero-creature path is inert (publish_fresh_tracked_set sets chain_tracked_set_id even empty; latest_tracked_set_id skips empty targeting.rs:2009; Shuffle/Cloak read empty → nothing). Add an empty-selection test so it can't regress into reading a stale prior set.

**C6 (modal integration).** Test B asserts the mode-2 chain lands as ability[1] WITHIN the "choose one or both" modal, mode 1 (TurnFaceUp) at ability[0] untouched — not just that the sentence parses standalone.

**Test-authoring caveats (from review missed-risks):**
- C(d) AURA-FALLS-OFF (team-lead: add alongside the counter test — two observables): Auras detach as an SBA that runs AFTER the effect finishes resolving → assert the Aura is in the graveyard AFTER resolve + SBA pass, NOT mid-cloak-loop (else spurious fail).
- Token/non-card member edge: a face-up token/copy with granted disguise, if exiled, ceases to exist (SBA) and never returns — arguably rules-correct (a token can't return). Add a `ponytail:` note, not silent. Non-blocking.
- Parser: mirror the `value(KeywordKind::Mutate, tag("mutate"))` block (oracle_target.rs:5098) with a Disguise arm — cleaner than editing the matches! allowlist at 5110-5133 (two places).

## 1. Recommended lowered chain (mode 2) — mode 1 (TurnFaceUp) parses today, do not touch
```
ChooseObjectsIntoTrackedSet{ chooser:Controller,
                             filter: Typed[Creature, You, HasKeywordKind{Disguise}], min:0, max:None }
  └ sub: Shuffle{ target: TrackedSet{0} }        // reorder tracked set via rng; NO ShuffledLibrary event
      └ sub: Cloak{ target:Controller, count:Fixed(1), object_source: Some(TrackedSet{0}) }
```
- 1a "any number of face-up creatures you control with disguise": filter DROPPED to Any today. Two layers: parse_keyword_match (oracle_target.rs:5071) allowlist @5110-5133 lacks "disguise"; and Keyword::Disguise(_).kind()→Unknown (keywords.rs:1229/1275) so HasKeywordKind{Disguise} not expressible. Target: Typed[Creature, You, HasKeywordKind{Disguise}] (evaluator generic filter.rs:2842). "face-up" needs no extra prop — a face-down disguise permanent is a 2/2 with no abilities → record.keywords lacks Disguise → HasKeywordKind{Disguise} inherently selects only face-up (ponytail-comment).
- 1b "in a face-down pile": model as the chain's tracked object set (state.tracked_object_sets, ordered Vec, game_state.rs:6158), NOT a modeled zone (CR 701.24a: pile is a first-class shuffle target, not a zone). ChooseObjectsIntoTrackedSet (ability.rs:10407; constructor mod.rs:5506; publishes preserving order mod.rs:3970). SEE Q2 for exile modeling.
- 2 "shuffle that pile": shuffle.rs:6 today only shuffles library + always emits ShuffledLibrary (:58-61). Add TrackedSet branch: reorder tracked_object_sets[chain_id] via state.rng (util::im_ext::shuffle_vector), emit EffectResolved{Shuffle}, RETURN before ShuffledLibrary push (CR 701.24a — pile shuffle ≠ library shuffle → no "whenever you shuffle your library" trigger).
- 3 "then cloak them": reuse Effect::Cloak{object_source:Some(TrackedSet{0})}. cloak.rs TrackedSet branch reads state.tracked_object_sets[id] DIRECTLY, order-preserving (precedent choose_from_zone.rs:600), NOT effect_object_targets (Vannifar's ability.targets path — Shuffle can't reorder ability.targets). ⚠️ PER C1: manifest_card on a battlefield object is a NO-OP → the branch must EXILE each member (Battlefield→Exile, real zone change) THEN manifest_card (Exile→Battlefield cloaked_2_2). This is what makes the creatures actually become fresh face-down 2/2s AND makes the shuffle observable (reordered creation order).

## 2. Files to touch (dependency order) — ALL non-frozen
1. types/keywords.rs — add KeywordKind::Disguise variant (@140) + map Keyword::Disguise(_)=>KeywordKind::Disguise in .kind() (out of Unknown catch-all @1229/1275; mirror Morph @1160). Compile-forces exhaustive .kind() + any exhaustive `match KeywordKind`.
2. parser/oracle_target.rs — add "disguise" to Kind-routing allowlist in parse_keyword_match (@5110-5133, mirror "mutate" @5098).
3. game/effects/shuffle.rs — TargetFilter::TrackedSet pile branch (rng reorder, no library shuffle, no ShuffledLibrary, early return).
4. game/effects/cloak.rs — in Some(object_source) arm (@44), when filter is TrackedSet, read state.tracked_object_sets[bound_id] directly (order-preserving cloned) instead of effect_object_targets; manifest_card each.
5. parser/oracle_effect/ (sequence.rs / imperative.rs) — new nom combinator for "Exile any number of <filter> in a face-down pile, shuffle that pile, then cloak them" → the 3-effect chain. Bulk of the work. Nom mandate (no string dispatch).
6. game/ability_scan.rs — verify scan_target_filter tolerates TrackedSet inside Effect::Shuffle{target} (@787/789); likely no edit.
7. Tests (§4).

## 3. add-engine-variant verdict
- KeywordKind::Disguise: REQUIRED. Stage1 parameterize impossible (Unknown catch-all); Stage2 categorical parallel to Morph/Mutate (CR 702.168); Stage3 confirmed absent. Walker (#4904) classifies Effect not KeywordKind → NOT forced by this. data/engine-inventory.json exists (1.4MB) — grep during impl, do NOT regenerate.
- Effect::Shuffle.pile: NOT added — reuse target:TrackedSet (value not shape). Zero new Effect variants, zero new FilterProp otherwise.

## 4. CR annotations (grep-verified)
- **CR 701.24a** (3490): "shuffle a library OR a face-down pile of cards, randomize…" — anchors the pile-shuffle branch. USE THIS, not 701.24e.
- CR 701.24e (3501): library-scoped 0/1-card-still-triggers rule — does NOT apply to a pile; the basis for "no library-shuffle trigger" is 701.24a categorical distinction (resolver emits no ShuffledLibrary).
- CR 701.58a-e (3781-3789): Cloak = face-down 2/2 ward{2}; 701.58e one-at-a-time.
- **CR 702.168** (5227): **Disguise** (NOT 702.16=Protection — prior guess wrong). Cite on KeywordKind::Disguise + parser allowlist.
- CR 708.5 (5711): look only at own face-down → pile shuffle is opponent-facing; model as ordered tracked set.

## 5. Test plan (discriminating, revert-to-red)
- **A parser (disguise filter):** parse_type_phrase("creatures you control with disguise") → Typed with FilterProp::HasKeywordKind{Disguise} (mirror "with flying" @8803). Revert: remove allowlist arm → drops/WithKeyword{Disguise(empty)} → fail.
- **B parser (mode-2 chain):** parse "expose the culprit" → ability[1] = ChooseObjectsIntoTrackedSet{…HasKeywordKind{Disguise}} → Shuffle{TrackedSet} → Cloak{object_source:Some(TrackedSet)}, NOT Unimplemented. Revert: current tree → ChangeZone{Exile}+Unimplemented{shuffle}.
- **C runtime (hollow-win + rules-correctness):** P0 controls ≥3 face-up Disguise creatures; cast Expose mode 2, select all; seed → non-identity permutation:
  - (a) chosen-not-library: the three chosen become face-down 2/2 ward-2 (back_face = chosen cards); library untouched. Revert: object_source→library-top → wrong cards.
  - (b) pile shuffled: cloaked-permanent creation order = shuffled Vec order ≠ selection order. Revert: stub pile branch no-op → order matches input → fail.
  - (c) no library-shuffle trigger: assert no ShuffledLibrary event; witness "whenever you shuffle your library" trigger does NOT fire. Revert: route pile through library path → witness fires.
  - **(d) RULES-CORRECT OBJECT RESET (Q2 gate):** a chosen creature carrying a +1/+1 counter (and/or an Aura) → resulting face-down permanent has NO counter (Aura fell off). Revert: in-place flip preserving state → counter present → fail. (Discriminates rules-correct new-object from in-place-flip.)

## 6. Risks / open questions — ADJUDICATED above (⚑ DRIVER DECISIONS). Residual for executor/review:
- KeywordKind::Disguise exhaustiveness fan-out: enumerate every compile-forced `match KeywordKind` site; ensure none mishandles Disguise.
- TrackedSet order-preservation end-to-end (ChooseObjectsIntoTrackedSet publish-order → Shuffle reorder → Cloak read-order).
- manifest_card semantics on a battlefield permanent (Q2 rules-correctness probe — THE load-bearing runtime question).
- Parser combinator held to nom mandate (main bandaid risk).

### Critical files
- crates/engine/src/types/keywords.rs
- crates/engine/src/parser/oracle_target.rs
- crates/engine/src/game/effects/shuffle.rs
- crates/engine/src/game/effects/cloak.rs
- crates/engine/src/parser/oracle_effect/imperative.rs (+ sequence.rs)
