# MSH (Marvel) rollout plan

> **Hard rule (user, 2026-06-22): every card MUST be implemented. No "defer", no "out-of-scope."**
> All unsupported MSH cards are assigned to an implementation cluster below; the two
> heaviest (Cosmic Cube, Hawkeye Young Avenger) are in scope as a heavy cluster gated by
> `/review-engine-plan`, not dropped. Backing: `.planning/coverage-analysis/out/MSH/cluster-assignment.tsv`.
> _Re-measured 2026-06-23 @ `ae663ee8c`: 9 unsupported remain (was 13). See "Where MSH stands"._

**Set:** `MSH` — a Marvel product with **no format legalities** (standard/modern/commander all null). These are **engine-completeness** targets: a standalone pool that std/modern coverage will *not* clear (0 of the 13 are in the standard pool). The building-block *classes* overlap with the std/modern clusters, so most fixes reuse those primitives.
**Snapshot:** 2026-06-23 · main @ `ae663ee8c` (re-measured; was 2026-06-22 @ `c55670fd0`) · fresh card-data regenerated against `ae663ee8c`.
**Method:** `coverage-breakdown.sh --set MSH && cluster-assign.sh MSH`.

## Where MSH stands

| metric | value (2026-06-23 @ `ae663ee8c`) | prior (2026-06-22 @ `c55670fd0`) |
|---|---|---|
| members | 286 | 286 |
| supported | 277 (**96.85 %**) | 273 (95.45 %) |
| unsupported | **9** = 6 parser-gap + 3 resolver-flagged | 13 = 10 parser-gap + 3 resolver-flagged |

### Re-measured 2026-06-23 — the current 9 unsupported (4 cleared since the snapshot)

Pool size unchanged (286); 4 of the original 13 now parse/resolve. Exact remaining
set from `out/MSH/unsupported.tsv` (name · gap-handler · status):

| card | handler | status |
|---|---|---|
| The Ruinous Wrecking Crew | resolver-flagged (gap=0) | **OPEN PR #4186** (card/msh-modal-choose) |
| Doctor Doom | resolver-flagged (gap=0) | **OPEN PR #4182** (card/msh-doctor-doom) |
| Hulkling, Burgeoning Bruiser | Swallow:Condition_If | **OPEN PR #4169** (card/msh-intervening-if) |
| Cosmic Cube | Swallow:DynamicQty | heavy cluster |
| Hawkeye, Young Avenger | Swallow:DynamicQty | heavy cluster |
| Hawkeye, Master Marksman | resolver-flagged (gap=0) | not yet assigned a PR |
| Baron Helmut Zemo | ParseWarning:target-fallback | not yet assigned a PR |
| Loki, God of Mischief | Trigger:became-target-of-ability | not yet assigned a PR |
| The Incredible Hulk | Swallow:Condition_If | not yet assigned a PR |

The three OPEN PRs (CI-green, review CHANGES_REQUESTED) cover 3 of the 9; landing
them drops MSH unsupported to **6**. Cluster prose below predates this re-measure
(it enumerated the original 13) — re-run `cluster-assign.sh MSH` to refresh
per-card cluster files before dispatching new work.

## Clusters (all 13 — every card has a home)

### MSH-A — "for each Equipment attached to it" dynamic count · **2** · HIGH ROI (real 12-card class)
Reusable `QuantityRef` = Equipment attached to the source. Covers a genuine 12-card class beyond MSH (Bruenor Battlehammer, Armament Master, Captain America Liberator, Catti-brie, Goblin Gaveleer, Golem-Skin Gauntlets, Kemba's Legion…) — build it first-class. CR 301.
- **Winter Soldier, Icy Assassin** — static "+2/+0 for each Equipment attached."
- **Whiplash, Vengeful Engineer** — trigger "lose/gain X = Equipment attached" (also the MSH-B intervening-if "if he's equipped").

### MSH-B — intervening-if on a trigger (= std **S03**) · **2** · `parse_inner_condition`
- **Hulkling, Burgeoning Bruiser** — "Whenever another creature you control enters, **if it has greater power or toughness than Hulkling**…" (entering-creature-vs-source P/T comparison).
- **Whiplash** — "if he's equipped" (overlaps MSH-A).

### MSH-C — count-with-offset dynamic quantity (= std **S08** ext) · **1**
- **Klaw, Sonic Subjugator** — "reveals **one plus** the number of creature cards in your graveyard," then choose/discard. `1 + ObjectCount` quantity.

### MSH-D — "as long as <board>" conditional static (resolver, = std **R2**) · **1**
- **Doctor Doom** — "As long as you control an artifact creature **or a Plan**, …has indestructible." Rides the R2 conditional-static evaluator; also needs `Plan` card-type recognition. CR 604/611.

### MSH-E — modal "choose up to X" with dynamic / repeated count · **2** (resolver-flagged)
Building block: modal `ChooseUpTo { count: QuantityExpr }`. CR 601.2b/700.2.
- **The Ruinous Wrecking Crew** — enters with X counters; "choose up to X —" of 4 modes.
- **Hawkeye, Master Marksman** — "pay {1} up to three times. When you do, choose up to that many" of 3 modes.

### MSH-F — impulse / replacement singletons (HEAVY — in scope, `/review-engine-plan` first) · **2**
- **Cosmic Cube** — impulse-cast-from-top-six gated by a **dynamic mana-value ceiling** ("MV ≤ greatest power among your attacking creatures"), cast without paying. Cast-from-library with a dynamic cost constraint.
- **Hawkeye, Young Avenger** — noncombat-damage **replacement** that adds X = source's power ("deals that much **plus X**"). Damage-amplification replacement effect. CR 614.

### Singletons (one-off primitives) · **3**
- **Black Widow, Super Spy** — impulse-exile-until-nonland, then "may cast it **until end of turn**, any mana type" → std **S11/impulse-cast** class.
- **Loki, God of Mischief** — new trigger matcher "**becomes the target of an ability you control**" + once-per-turn (`/add-trigger`). CR 603.2.
- **Baron Helmut Zemo** — anaphoric target "**those exiled cards**" (Boast copy-exiled) → std **S17**. CR 707.
- **The Incredible Hulk** — Enrage tail "**if he's attacking, untap + additional combat phase**" → `Effect::AdditionalCombatPhase` + attacking-self condition.

(13 = 2 A + 2 B + 1 C + 1 D + 2 E + 2 F + 3 singletons + Incredible Hulk → 2+2+1+1+2+2+4 = **14 slots, 13 cards** since Whiplash appears in both A and B; net distinct cards = 13. ✓)

## Dispatch order (medium effort, via `/engine-implementer`)

1. **MSH-A** Equipment-attached count — reusable 12-card-class quantity (clears Winter Soldier + half of Whiplash).
2. **MSH-B** intervening-if (rides std S03) — clears Hulkling + rest of Whiplash.
3. **MSH-D** Doctor Doom + **Black Widow** — piggyback on the std R2 / S11 dispatches (don't build twice).
4. **MSH-E** modal choose-up-to-X (Ruinous Wrecking Crew + Hawkeye Master Marksman).
5. Singletons: **MSH-C** Klaw, **Loki** trigger, **Baron Zemo** anaphor, **Incredible Hulk** additional-combat.
6. **MSH-F** (Cosmic Cube, Hawkeye Young Avenger): `/review-engine-plan` first, then implement — **not deferred**, just plan-gated.
