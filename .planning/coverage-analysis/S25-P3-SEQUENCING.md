# S25 P3 — authoritative remaining work-list + sequencing (re-resolved 2026-07-03)

Source: re-resolution agent a636c7e84cdef28ea vs `cluster-assignment.tsv` + `S25-PLAN-FINAL.md` Group tables.
**40 = 18 DONE + 2 IN-PROGRESS (C1) + 20 REMAINING.** All names verified in card-data.json (0 resolution errors).

**DATA CAVEAT:** `data/card-data.json` mtime (07-03 01:14) PREDATES the S25 commit batch → UNI counts are a pre-batch
snapshot; git-log is the DONE authority. For a post-HEAD coverage signal regenerate `cargo card-data` (a build).
Reality check vs my earlier triage: only **Another Round** is truly PARSER-ONLY; the other 19 need engine work.

## IN-PROGRESS (do NOT touch — active C1 control work, own plans)
- Secret of Bloodbending (C1, phase-boundary control) — S25-Secret-of-Bloodbending-PLAN.md + §9 gates
- The Dominion Bracelet (C1/C9, granted-ability dual binding + {X}-less cost) — S25-granted-ability-self-binding-PLAN.md + gates

## 20 REMAINING (P3) — group id · gap class · reuse target
| Card | gid | class | reuse target / gap |
|---|---|---|---|
| Another Round | A10 | PARSER-ONLY | RepeatContinuation (effects/mod.rs:693,5226); verify interactive "exile any number + return" |
| Sandman, Shifting Scoundrel | C11 | NEEDS-ENGINE (small) | parse_target graveyard zone-ext (S17-adj) + ChangeZone reanimate |
| Alania, Divergent Storm | B5 | NEEDS-ENGINE (small trigger) | watcher EXISTS (SpellsCastThisTurn{scope,filter} quantity.rs:383,2483; ==0 gate game_object.rs:942) → add spell-type filter + intervening-if |
| Crowd-Control Warden | A8 | NEEDS-ENGINE (replacement) | PutCounter + ObjectCount(4288); "as enters OR turned-face-up" |
| Sarkhan, Dragon Ascendant | B7 | NEEDS-ENGINE (new effect) | wrap BeholdCostAction/AbilityCost::Behold(6974,6483) as ETB effect; CR 701.4. **RIDER (team-lead): behold-class → intersects #5051 (already_chosen stale creature-type on gy/exile recast, CR 400.7). Implement to CURRENT convention; do NOT fix #5051 in-tranche; pin the intersection with a code comment/test note referencing #5051 so the fix sweep finds it.** |
| Kav Landseeker | B2 | NEEDS-ENGINE (compose) | delayed-trigger builder (delayed_trigger.rs, DelayedTriggerCondition ability.rs:2461) + Sacrifice; CR 603.7 |
| Rhys, the Evermore | A8 | NEEDS-ENGINE (interactive) | RemoveCounter + NEW variable-count WaitingFor (counters.rs:1943-1979 fixed/all only) |
| Foraging Wickermaw | A3 | NEEDS-ENGINE (variant) | SetColor(17411) bound to event-context mana color (provenance) |
| The Tomb of Aclazotz | A2 | NEEDS-ENGINE (compose) | AddType/AddSubtype(17317) + enters-with-counter + cast-creature-from-gy permission |
| The Skullspore Nexus | B4 | NEEDS-ENGINE (/add-engine-variant) | Aggregate{Sum,Power}(4452) + token-spec P/T Option<i32>→QuantityExpr (dynamic CDA); CR 604.3+208.4b |
| Parker Luck | B9 | NEEDS-ENGINE (new compose) | LoseLife(8426) bound to OTHER player's revealed card ManaValue (cross-binding NEW) + return-to-hand |
| Bumi, Unleashed | C1 | NEEDS-ENGINE (combat restriction) | attack-restriction static scoped to the extra combat phase (extra-combat already parses). NOT the control subsystem |
| Glen Elendra's Answer | C7 | NEEDS-ENGINE | mass counter of opponent-controlled stack ABILITIES (sibling CounterAll for abilities) |
| No Witnesses | C5 | NEEDS-ENGINE | most-of player aggregate (PlayerCount/ObjectCountBySharedQuality 4315/4322) + Investigate(8807) |
| Niko, Light of Hope | C3 | NEEDS-ENGINE | BecomeCopy(9164) → mass dynamic-set + duration-bound; CR 707+611.2c |
| Moonlit Meditation | C2 | NEEDS-ENGINE (/add-replacement-effect) | token-creation replacement + once-per-turn latch; CR 614+616 |
| Esper Terra | A8 | NEEDS-ENGINE (Saga subsystem, ~C) | transforming Saga: copy-token + lore-counter + multi-symbol Mana list + chapter-IV transform |
| Graceful Takedown | C6 | NEEDS-ENGINE (heavy targeting Tier-3) | multi-slot targeting state-machine (0..n × 0..1 × sink) + per-source DealDamage=power |
| Vincent's Limit Break | C8 | NEEDS-ENGINE (Tiered) | Tiered choose-one-additional-cost + chosen base P/T + quoted dies-return. SINGLE instant (not 3 faces) |
| Vanille, Cheerful l'Cie | C10 | NEEDS-ENGINE (MELD subsystem) | own+control Vanille+Fang → meld into Ragnarok Divine Deliverance |

## Execution mode: STRICTLY SERIAL (2026-07-03 user directive — ≤3-concurrent grant REVOKED)
Max ONE sub-agent (implementer OR reviewer OR planner) active at any moment. Full pipeline per card, one card at a time.
The collision lanes below are now a **serialization ORDER within each lane** (no cross-lane parallelism); waves are just cheapest-first batching, executed card-by-card.

## Collision lanes (serialization order within a lane; NO cross-lane parallelism — serial only)
- **α — oracle_effect/imperative.rs (counter dispatch):** Crowd-Control Warden → Rhys → Esper Terra (serialize)
- **β — oracle_static/animation.rs:** Foraging Wickermaw → The Tomb of Aclazotz (serialize)
- **γ — oracle_effect/sequence.rs:** Another Round (keep separate from Esper Terra)
- Shared cores that ALWAYS serialize: types/ability.rs, oracle.rs, effects/mod.rs, coverage.rs (every new variant/effect registers here)

## Wave sequence (cheapest-first; each card full pipeline plan→review→impl→/review-impl→commit)
1. **Wave 1 (cheapest-first, run one at a time):** Another Round · Sandman · Alania · Sarkhan · Kav Landseeker
2. **Wave 2 (α serialize):** Crowd-Control Warden → Rhys → Esper Terra
3. **Wave 3 (β serialize):** Foraging Wickermaw → The Tomb of Aclazotz
4. **Wave 4 (variant-gated, disjoint):** Skullspore Nexus · Parker Luck · Bumi Unleashed
5. **Wave 5 (Group-C subsystems, /review-engine-plan BEFORE code):** Glen Elendra's Answer · No Witnesses · Niko · Moonlit Meditation · Graceful Takedown · Vincent · Vanille (heaviest last)

## Launch gate
P3 begins ONLY after both C1 commits land (they touch types/ability.rs+coverage.rs — parallelizing P3 with them is unsafe).
Re-anchor P3 plan line-refs to the post-C1 HEAD at each planner dispatch.
