# Modern ∩ Commander rollout plan

**Set:** cards legal in **both** Modern **and** Commander (intersection).
**Snapshot:** 2026-06-23 · main @ `ae663ee8c` (re-measured; was 2026-06-22 @ `c55670fd0`) · card-data regenerated against `ae663ee8c`.
**Method / reuse:** `.planning/coverage-analysis/coverage-breakdown.sh --format modern --format commander` (filters are now repeatable + intersected).
Raw outputs: `.planning/coverage-analysis/out/modern+commander/{report,unsupported,parser-gap,resolver-flagged}.{txt,tsv}`.
All counts measured from `data/coverage-data.json` × `data/card-data.json`.

## Where the intersection stands

| metric | value (**2026-06-29 @ `dd6c22ea7`**) | prior (2026-06-27 @ `c1b61ded5`) | prior (2026-06-23 @ `ae663ee8c`) | prior (2026-06-22 @ `c55670fd0`) |
|---|---|---|---|---|
| members (Modern ∩ Commander) | **22,943** | 22,943 | 22,943 | 22,669 |
| supported | **21,474 (93.60 %)** | 21,438 (93.44 %) | 21,387 (93.22 %) | 21,118 (93.16 %) |
| unsupported | **1,469** | 1,505 | 1,556 | 1,551 |
| └ parser-gap (`gap_count > 0`) | **1,297** | 1,330 | 1,380 | 1,376 |
| └ resolver-flagged (`gap_count == 0`, parses fully) | **172** | 175 | 176 | 175 |

> **Net read (re-measured 2026-06-29 @ `dd6c22ea7`, 33-commit ff; card-data regenerated, inputs
> stamp `2026-06-29T13:58Z`):** pool flat at **22,943**; supported **+36** (21,438 → 21,474),
> unsupported **1,505 → 1,469 (−36)** — parser-gap 1,330 → 1,297 (−33), resolver-flagged 175 → 172 (−3).
> **0 unclustered.** Decline stays broad-and-shallow, driven by the Standard cluster dispatches
> bleeding through (S01/S21 etc.). **Fresh full cluster table (1,469 cards):**
> S25-effect-verb 511 · S07-condition-if 127 · S19-new-trigger 93 · R2-aslongas 80 · **S21-static 65** ·
> **S01-reflexive-if 61** · R5-runtime 61 · S24-unknown 60 · S08-foreach-qty 53 · S03-intervening-if 48 ·
> S10-dynamic-qty 47 · S22-choose 43 · S11-duration 28 · S18-foreach-simple 23 · S05-alt-cost-if 23 ·
> S12-optional 22 · R3-level-up 21 · S14-unless 20 · S02-cast-context 13 · S23-alt-cost 10 ·
> S04-activate-if 10 · S17-anaphoric 9 · S13-aslongas-parse 9 · R1-speed 9 · S20-copy-retarget 8 ·
> S16-foreach-player-HEAVY 6 · S06-saga 3 · S26-alt-keyword-cost 2 · S29-modal 1 · S28-replacement-instead 1 ·
> S27-trigger-subject 1 · R4-cant-restriction 1. **Cluster movers vs the 2026-06-23 record:**
> **S01 reflexive-if 76 → 61 (−15)** (Standard S01 dispatches bleed through; the same uncovered
> reflexive shapes remain), **R2 as-long-as 81 → 80 (−1)**. **Handler-histogram deltas (per-occurrence):**
> Swallow:Condition_If 306 → 285 (−21), Swallow:DynamicQty 121 → 109 (−12), Effect:static_structure 77 → 67 (−10),
> Optional_YouMay 35 → 31 (−4), Swallow:Duration_ThisTurn 41 → 38 (−3). Resolver-flagged 172 = R2 (80) +
> R5 (61) + R3 level-up (21) + R1 speed (9) + R4 (1) — the runtime subsystems still dominate the
> resolver bucket. **S99-UNCLUSTERED resolved:** the prior Nephalia Academy gap is now absorbed by the
> S28-replacement-instead bucket (1 card); 0 unclustered.

> **Net read (re-measured 2026-06-27 @ `c1b61ded5`):** pool size held flat at **22,943**
> while supported rose **+51** (21,387 → 21,438) and unsupported fell **−51**
> (1,556 → 1,505; 93.22 % → 93.44 %). Split moved to **1,330 parser-gap + 175
> resolver-flagged** (parser-gap −50, resolver-flagged −1). **This snapshot closes the
> v0.7.0 measurement gap for this pool** — the intersection was *not* re-measured at
> v0.7.0/`5eca83b8c`, so this −51 covers the full v0.7.0 + post-v0.7.0 commit window
> (widest catch-up of any pool). Decline is broad and shallow across the shared Standard
> building blocks: Swallow:Condition_If 306→295 (−11), Swallow:DynamicQty 121→110 (−11),
> Effect:static_structure 77→73, Optional_YouMay 35→32, Effect:create 15→13, spend 17→15
> — no single cluster cleared wholesale; the gains are the Standard cluster dispatches
> bleeding through. **No regressions:** every card named in the M1–M5 / S26 / S27 clusters
> below remains unsupported as expected (none were marked DONE), and no card recorded as
> cleared reappeared in `unsupported.tsv`.
>
> **S99 ruleset gap (cluster-assign.sh):** 1 card is `S99-UNCLUSTERED` — **Nephalia
> Academy** (`Swallow:Replacement_Instead`, "If a spell or ability an opponent controls
> causes you to discard a card, you may reveal that card and put it on top of your library
> instead…"). `classify()` has **no bucket for replacement "…instead…" effects**. Proposed
> rule (lead to land): add an S-cluster matching `Replacement_Instead` handlers / oracle
> `/ instead /`, e.g. `*Replacement_Instead*|*' instead '*) cluster="S28-replacement-instead" ;;`
> placed before the catch-all so replacement-instead effects (discard-to-library, damage
> redirection, ETB substitution) cluster as one reusable building block.

This is ~5× the standard pool (4,924). Because Commander legality is a near-superset of Modern, the intersection ≈ "all Modern cards that aren't Commander-banned" — i.e. the full eternal-ish Modern card base.

> **Net read (re-measured 2026-06-23):** pool grew **+274** (22,669 → 22,943) while
> supported rose **+269**, so unsupported is essentially flat at **1,556** (+5). The
> handler mix is unchanged in shape — Swallow:Condition_If (306) and Swallow:DynamicQty
> (121) still dominate, exactly the Standard building blocks scaled up. Cluster prose
> below predates this re-measure — re-run `cluster-assign.sh` to refresh per-card files.

## The dominant insight: it's the *same* building blocks as Standard, scaled up

Modern∩Commander parser-gap handler occurrences (re-measured 2026-06-23 @ `ae663ee8c`; per-handler over the 1,380 parser-gap cards):

```
306 Swallow:Condition_If     32 Effect:deal              16 Effect:flip
121 Swallow:DynamicQty       30 Effect:for               15 Effect:repeat
 77 Effect:static_structure  27 Effect:the               15 Effect:create
 68 Effect:unknown           17 Effect:spend             14 Effect:add
 46 Effect:choose            41 Swallow:Duration_ThisTurn 13 Effect:all / Condition_AsLongAs
 35 Swallow:Optional_YouMay  12 Effect:can't             + long tail
```

The top buckets are **identical to Standard's** (Condition_If, DynamicQty, for-each…). The same sub-clusters recur, just with more (older) cards (Std vs intersection, both re-measured 2026-06-23):

| cluster (building block) | Std | Modern∩Cmdr | note |
|---|---|---|---|
| C1 reflexive "if it/that <past-state>" rider (S01) | 18 | **76** | same fix; #3898 laid groundwork |
| Condition_If (all sub-clusters S01–S07) | 77 | 306 | C1/C2/C3/C5 + alt-cost |
| DynamicQty (for-each count → effect qty, S08–S10) | 36 | 112 | C4 |
| Effect:for (S16+S18) | 13 | 30 | incl. per-player defer |

**Consequence:** dispatching the Standard clusters (C1–C7 in `../std-rollout/PLAN.md`) directly unlocks the corresponding Modern∩Commander members too. The intersection mainly *widens the regression corpus* (older cards exercising the same parser paths) and adds the intersection-specific clusters below. **Do not plan the intersection as separate parser work for the shared classes — drive it off the Standard cluster dispatches and re-scan.**

## Intersection-specific high-ROI clusters (beyond Standard)

### M1 — Flip cards (Kamigawa "flip") · **16** · one subsystem
`Effect:flip` = the original Kamigawa flip-permanent mechanic (`flip ~` / `flip it`): Akki Lavarunner, Budoka Gardener/Pupil, Bushi Tenderfoot, Callow Jushi, Cunning Bandit, Faithful Squire, Hired Muscle, Nezumi Graverobber, Student of Elements, Kuon… A single transform-on-condition subsystem (parser + runtime, akin to the existing transform/DFC path) unlocks all 16. Not present in Standard. CR 711 (double-faced) / 701.transform-adjacent.

### M2 — "Repeat this process" loops · **15** · infra partly exists
`Effect:repeat`: Sphinx's Tutelage, Skywriter Djinn, Eureka/Hypergenesis ("until no one puts a card"), Forgotten/Shrouded Lore. `RepeatContinuation` already shipped (#4030); these are extending it to the remaining repeat shapes. Med-high ROI. (Also the path for the post-Standard combo-corpus enablers Grindstone/Professor Onyx — [[phase-combo-detector]].)

### M3 — "as long as <global board>" conditional static (RESOLVER) · **81** · biggest resolver cluster
Parses (`Continuous` static) but the condition isn't runtime-evaluated → flagged unsupported. Knight of Grace/Malice class at scale. One runtime evaluator for static-ability conditions over board state unlocks ~81 (= `R2-aslongas-conditional-static` in the fresh `cluster-assignment.tsv`; the prior "46" was a narrower Modern-single-gap hand-measure — this is the full intersection resolver cluster). CR 611 (continuous effects) / 604 (static abilities).

### M4 — Level up (RESOLVER) · **21** · mechanic runtime
`Level up` (Rise of Eldrazi): Hexdrinker, Kargan Dragonlord, Guul Draz Assassin, Transcendent Master, Knight of Cliffhaven… Parses but the leveler counter / level-band P/T+ability gating isn't resolved. CR 711 (leveler cards). Not in Standard.

### M5 — Speed / "Start your engines!" (RESOLVER) · **9** · overlaps Std
Same subsystem as the Standard speed cluster; doing it for Standard clears these too.

## Larger heterogeneous buckets (lower per-fix ROI — velocity-loop / parser-gap-finder)

- `Effect:static_structure` (66) — assorted continuous statics (foretell-granting, "P/T equal to life paid", "all creatures attack enchanted's controller"). Cohesion low; pick the 2–3 recurring sub-patterns.
- `Effect:choose` (35), `Effect:deal` (32), `Effect:the` (24) — heterogeneous; mine for sub-patterns at dispatch time from the TSVs.

## LOW-VALUE / out-of-scope (do not spend cluster budget here)

`Effect:unknown` (68 intersection parser-gap occurrences) is dominated by cards that are largely **non-functional in a real engine**: un-set token-augment (`{tk}{tk} — N/N`, 18), `starting intensity` (12), `poison tolerance +N` (6), ante (`remove this card from your deck … if you're not playing for ante`, 5), Horde/Archenemy (`the horde casts that card`, 5), `commander enchantment` subtype ("Taught by…", 5), `choose a letter`. Skip these; they inflate the gap count without engine value.

## Genuine heavy-infra DEFERS (carry over from Std; cross-ref [[phase-std188-pause-state]])

Per-player OBJECT-target enumeration (`Effect:for`, needs `ChooseFromZone{EachPlayer}`); prime/advanced-count quantity; alt-cost cast-from-exile/graveyard remainder; the per-player iterated battlefield-choice set. Same `/review-engine-plan` gate as Standard.

### New small clusters surfaced by the 2026-06-23 re-measure
The refreshed `cluster-assignment.tsv` (0 unclustered) added two named clusters via an extended ruleset (`cluster-assign.sh` rules S26/S27):
- **S26 — alternative cost to pay a keyword cost** · **2** · `Static:AlternativeKeywordCost` (`Heart of Kiran` "remove a loyalty counter … rather than pay crew cost"; `New Perspectives` "pay {0} rather than pay cycling costs"). Reusable: alt-payment for crew/cycling/equip-style keyword costs.
- **S27 — trigger-subject anaphora** · **1** · `ParseWarning:trigger-subject` (`Psychic Possession` "Whenever enchanted opponent draws a card…"). Bind the trigger's subject to the enchanted/affected object (sibling of S17 anaphoric-target).

## Recommended dispatch order (xhigh, via `/engine-implementer`)

1. **Run the Standard clusters first** (C1 reflexive-if, C4 for-each, C3 intervening-if, speed) — these unlock the bulk of the shared intersection classes (C1 alone = 76 here) and Standard is the smaller, cleaner proving ground.
2. **M3 as-long-as conditional static (resolver, 81)** — biggest intersection-only cluster, single runtime evaluator.
3. **M1 flip cards (16)** — self-contained subsystem.
4. **M4 level up (21)** — self-contained mechanic.
5. **M2 repeat-this-process (15)** — extends existing `RepeatContinuation`; also unblocks combo-corpus enablers.
6. Re-scan (`coverage-breakdown.sh --format modern --format commander`) and re-rank the residual long tail (static_structure / choose / deal) before committing to the heterogeneous buckets.

Sequential dispatch; shared files (`types/ability.rs`, `oracle.rs`, `effects/mod.rs`, layer system for M3/M4) are collision points — one cluster in flight unless file sets are disjoint.
