# Modern ∩ Commander rollout plan

**Set:** cards legal in **both** Modern **and** Commander (intersection).
**Snapshot:** 2026-06-22 · main @ `c55670fd0` (== upstream/main) · card-data regenerated this session (verified fresh: built against `c55670fd0`).
**Method / reuse:** `.planning/coverage-analysis/coverage-breakdown.sh --format modern --format commander` (filters are now repeatable + intersected).
Raw outputs: `.planning/coverage-analysis/out/modern+commander/{report,unsupported,parser-gap,resolver-flagged}.{txt,tsv}`.
All counts measured from `data/coverage-data.json` × `data/card-data.json`.

## Where the intersection stands

| metric | value |
|---|---|
| members (Modern ∩ Commander) | **22,669** |
| supported | 21,118 (**93.16 %**) |
| unsupported | **1,551** |
| └ parser-gap (`gap_count > 0`) | 1,376 |
| └ resolver-flagged (`gap_count == 0`, parses fully) | 175 |

This is ~5× the standard pool (4,647). Because Commander legality is a near-superset of Modern, the intersection ≈ "all Modern cards that aren't Commander-banned" — i.e. the full eternal-ish Modern card base.

## The dominant insight: it's the *same* building blocks as Standard, scaled up

Modern single-gap counts per handler (the intersection's ROI anchor, since Modern ⊂ Commander here):

```
269 Swallow:Condition_If     32 Effect:deal              16 Effect:spend
107 Swallow:DynamicQty       26 Swallow:Duration_ThisTurn 14 Effect:flip
 66 Effect:static_structure  26 Effect:for               13 Effect:repeat
 58 Effect:unknown           25 Swallow:Optional_YouMay   12 Effect:create / all
 35 Effect:choose            24 Effect:the                + long tail
```

The top buckets are **identical to Standard's** (Condition_If, DynamicQty, for-each…). The same sub-clusters recur, just with more (older) cards:

| cluster (building block) | Std | Modern∩Cmdr | note |
|---|---|---|---|
| C1 reflexive "if it/that <past-state>" rider | 16 | **89** | same fix; #3898 laid groundwork |
| Condition_If (all sub-clusters) | 75 | 304 | C1/C2/C3/C5 + alt-cost |
| DynamicQty (for-each count → effect qty) | 36 | 119 | C4 |
| Effect:for | 13 | 30 | incl. per-player defer |

**Consequence:** dispatching the Standard clusters (C1–C7 in `../std-rollout/PLAN.md`) directly unlocks the corresponding Modern∩Commander members too. The intersection mainly *widens the regression corpus* (older cards exercising the same parser paths) and adds the intersection-specific clusters below. **Do not plan the intersection as separate parser work for the shared classes — drive it off the Standard cluster dispatches and re-scan.**

## Intersection-specific high-ROI clusters (beyond Standard)

### M1 — Flip cards (Kamigawa "flip") · **16** · one subsystem
`Effect:flip` = the original Kamigawa flip-permanent mechanic (`flip ~` / `flip it`): Akki Lavarunner, Budoka Gardener/Pupil, Bushi Tenderfoot, Callow Jushi, Cunning Bandit, Faithful Squire, Hired Muscle, Nezumi Graverobber, Student of Elements, Kuon… A single transform-on-condition subsystem (parser + runtime, akin to the existing transform/DFC path) unlocks all 16. Not present in Standard. CR 711 (double-faced) / 701.transform-adjacent.

### M2 — "Repeat this process" loops · **15** · infra partly exists
`Effect:repeat`: Sphinx's Tutelage, Skywriter Djinn, Eureka/Hypergenesis ("until no one puts a card"), Forgotten/Shrouded Lore. `RepeatContinuation` already shipped (#4030); these are extending it to the remaining repeat shapes. Med-high ROI. (Also the path for the post-Standard combo-corpus enablers Grindstone/Professor Onyx — [[phase-combo-detector]].)

### M3 — "as long as <global board>" conditional static (RESOLVER) · **46** · biggest resolver cluster
Parses (`Continuous` static) but the condition isn't runtime-evaluated → flagged unsupported. Knight of Grace/Malice class at scale. One runtime evaluator for static-ability conditions over board state unlocks ~46. CR 611 (continuous effects) / 604 (static abilities).

### M4 — Level up (RESOLVER) · **21** · mechanic runtime
`Level up` (Rise of Eldrazi): Hexdrinker, Kargan Dragonlord, Guul Draz Assassin, Transcendent Master, Knight of Cliffhaven… Parses but the leveler counter / level-band P/T+ability gating isn't resolved. CR 711 (leveler cards). Not in Standard.

### M5 — Speed / "Start your engines!" (RESOLVER) · **9** · overlaps Std
Same subsystem as the Standard speed cluster; doing it for Standard clears these too.

## Larger heterogeneous buckets (lower per-fix ROI — velocity-loop / parser-gap-finder)

- `Effect:static_structure` (66) — assorted continuous statics (foretell-granting, "P/T equal to life paid", "all creatures attack enchanted's controller"). Cohesion low; pick the 2–3 recurring sub-patterns.
- `Effect:choose` (35), `Effect:deal` (32), `Effect:the` (24) — heterogeneous; mine for sub-patterns at dispatch time from the TSVs.

## LOW-VALUE / out-of-scope (do not spend cluster budget here)

`Effect:unknown` (58 modern single-gap) is dominated by cards that are largely **non-functional in a real engine**: un-set token-augment (`{tk}{tk} — N/N`, 18), `starting intensity` (12), `poison tolerance +N` (6), ante (`remove this card from your deck … if you're not playing for ante`, 5), Horde/Archenemy (`the horde casts that card`, 5), `commander enchantment` subtype ("Taught by…", 5), `choose a letter`. Skip these; they inflate the gap count without engine value.

## Genuine heavy-infra DEFERS (carry over from Std; cross-ref [[phase-std188-pause-state]])

Per-player OBJECT-target enumeration (`Effect:for`, needs `ChooseFromZone{EachPlayer}`); prime/advanced-count quantity; alt-cost cast-from-exile/graveyard remainder; the per-player iterated battlefield-choice set. Same `/review-engine-plan` gate as Standard.

## Recommended dispatch order (xhigh, via `/engine-implementer`)

1. **Run the Standard clusters first** (C1 reflexive-if, C4 for-each, C3 intervening-if, speed) — these unlock the bulk of the shared intersection classes (C1 alone = 89 here) and Standard is the smaller, cleaner proving ground.
2. **M3 as-long-as conditional static (resolver, 46)** — biggest intersection-only cluster, single runtime evaluator.
3. **M1 flip cards (16)** — self-contained subsystem.
4. **M4 level up (21)** — self-contained mechanic.
5. **M2 repeat-this-process (15)** — extends existing `RepeatContinuation`; also unblocks combo-corpus enablers.
6. Re-scan (`coverage-breakdown.sh --format modern --format commander`) and re-rank the residual long tail (static_structure / choose / deal) before committing to the heterogeneous buckets.

Sequential dispatch; shared files (`types/ability.rs`, `oracle.rs`, `effects/mod.rs`, layer system for M3/M4) are collision points — one cluster in flight unless file sets are disjoint.
