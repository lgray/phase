# MSH (Marvel) rollout plan

> **Hard rule (user, 2026-06-22): every card MUST be implemented. No "defer", no "out-of-scope."**
> All unsupported MSH cards are assigned to an implementation cluster below; the two
> heaviest (Cosmic Cube, Hawkeye Young Avenger) are in scope as a heavy cluster gated by
> `/review-engine-plan`, not dropped. Backing: `.planning/coverage-analysis/out/MSH/cluster-assignment.tsv`.
> _Re-measured 2026-06-26 @ `5eca83b8c` (**v0.7.0**; local main fast-forwarded to upstream tip; card-data regenerated): **7 unsupported** — **identical set, no change** vs `a2c3033f8` (was 9 @ `ae663ee8c`, 13 @ `c55670fd0`). The 75-commit v0.7.0 window cleared no MSH card. Doctor Doom (#4182) and Hulkling (#4169) remain clear. See "Where MSH stands."_
> _⚠️ **Verify finding (still standing):** #4186 (Ruinous Wrecking Crew) is merged (`23c50148a`, confirmed ancestor of HEAD) but the card is **still resolver-flagged unsupported** in the v0.7.0 re-measure — the merged "dynamic modal max" work did NOT clear its coverage flag. Treat the earlier "Ruinous ✓" prediction as refuted; see "GitHub delta" below._
> _Re-measured 2026-06-27 @ `c1b61ded5` (card-data regenerated, verified via card-data-meta.json): **5 unsupported** (down from 7) — **281 supported (98.25 %)**. The **−2** is **Cosmic Cube + Hawkeye, Young Avenger CLEARED via MSH-F #4471** (the heavy plan-gated cluster). The remaining MSH-E resolver-flagged pair (**The Ruinous Wrecking Crew + Hawkeye, Master Marksman**) is shipping via **PR #4482** (CI green, awaiting merge) — both clear on merge. No regressions: Doctor Doom (#4182), Hulkling (#4169), Black Widow (#4184) all remain supported. See "Where MSH stands."_
> _Re-measured **2026-06-29 @ `dd6c22ea7`** (33-commit ff from `7c1c1cf67`; card-data regenerated, inputs stamp `2026-06-29T13:58Z`): **0 unsupported — 286/286 = 100.00 % supported. MSH POOL COMPLETE.** Net **5 → 0 (−5)**, 0 parser-gap + 0 resolver-flagged, **0 unclustered** (`cluster-assign.sh MSH` = 0 cards). The final 5 cleared: **The Ruinous Wrecking Crew + Hawkeye, Master Marksman** via **#4482** (`9da9f3a5a`) — confirms the v0.7.0 "#4186 merged but still flagged" finding is now resolved by the dedicated #4482; **Loki, God of Mischief** via **#4491** (`671ad89f0`, "you or a permanent becomes the target" trigger); **Baron Helmut Zemo** via **#4533** (`b2c4f6980`, Boast aggregate-exile + copy/cast up to three); **The Incredible Hulk** via **#4526** (`c85e4ee8d`, gate Enrage riders + untap when attacking, CR 608.2c). All four PRs verified ancestors of HEAD `dd6c22ea7`. No regressions: every previously-DONE card stays supported. The cluster tables below are now fully historical — the pool needs no further dispatch._

**Set:** `MSH` — a Marvel product with **no format legalities** (standard/modern/commander all null). These are **engine-completeness** targets: a standalone pool that std/modern coverage will *not* clear (0 of the 13 are in the standard pool). The building-block *classes* overlap with the std/modern clusters, so most fixes reuse those primitives.
**Snapshot:** 2026-06-26 · local main fast-forwarded to upstream tip `5eca83b8c` (**release v0.7.0**)
· card-data + coverage-data regenerated against `5eca83b8c` (inputs stamp `2026-06-26T12:47Z`).
Prior snapshots: 2026-06-24 @ `a2c3033f8` (#4280), 2026-06-23 @ `ae663ee8c`, 2026-06-22 @ `c55670fd0`.
**Method:** `coverage-breakdown.sh --set MSH && cluster-assign.sh MSH`.

## Where MSH stands

| metric | value (2026-06-27 @ `c1b61ded5`, **re-measured**) | prior (2026-06-26 @ `5eca83b8c` v0.7.0) | prior (2026-06-24 @ `a2c3033f8`) | prior (2026-06-23 @ `ae663ee8c`) |
|---|---|---|---|---|
| members | 286 | 286 | 286 | 286 |
| supported | 281 (**98.25 %**) | 279 (97.55 %) | 279 (97.55 %) | 277 (96.85 %) |
| unsupported | **5** = 3 parser-gap + 2 resolver-flagged | 7 = 5 parser-gap + 2 resolver-flagged | 7 = 5 parser-gap + 2 resolver-flagged | 9 = 6 parser-gap + 3 resolver-flagged |

### Re-measured 2026-06-27 @ `c1b61ded5` — the current **5** unsupported

Pool size unchanged (286). Net **7 → 5** (−2): **Cosmic Cube** and **Hawkeye, Young Avenger**
(the MSH-F heavy cluster) **CLEARED via #4471** — both absent from `out/MSH/unsupported.tsv`.
No regressions: every previously-DONE card (Doctor Doom #4182, Hulkling #4169, Black Widow #4184,
Cosmic Cube + Hawkeye Young Avenger #4471) stays supported. Exact remaining set from
`out/MSH/unsupported.tsv` / `cluster-assignment.tsv` (name · cluster · status):

| card | cluster | status |
|---|---|---|
| The Ruinous Wrecking Crew | R5-runtime-bespoke (resolver-flagged, gap=0) | **shipping via PR #4482** (CI green, awaiting merge) — clears on merge |
| Hawkeye, Master Marksman | R4-cant-restriction-static (resolver-flagged, gap=0) | **shipping via PR #4482** (CI green, awaiting merge) — clears on merge |
| Loki, God of Mischief | S19-new-trigger-matcher | no PR; "becomes the target of an ability you control" + once-per-turn (`/add-trigger`) |
| Baron Helmut Zemo | S17-anaphoric-target | no PR; anaphoric "those exiled cards" (parse_target target-fallback) |
| The Incredible Hulk | S01-reflexive-if-rider | no PR; Enrage "if he's attacking" reflexive-if tail — **known building-block class (tractability signal)** |

### Re-measured 2026-06-26 @ `5eca83b8c` (v0.7.0) — the prior 7 unsupported (unchanged set)

Pool size unchanged (286); the 75-commit v0.7.0 fast-forward cleared **no** MSH card —
the set is identical to the `a2c3033f8` measure. Two were cleared earlier (vs `ae663ee8c`):
**Doctor Doom** (#4182 merged `2af0f855e`) and **Hulkling, Burgeoning Bruiser** (#4169 merged `798857711`).
Exact remaining set from `out/MSH/unsupported.tsv` (name · gap-handler · status):

| card | handler | status |
|---|---|---|
| The Ruinous Wrecking Crew | resolver-flagged (gap=0) | ⚠️ **#4186 MERGED** (`23c50148a`, ancestor of HEAD) but **STILL resolver-flagged unsupported** — merged modal-max work did not clear the coverage flag. Needs a runtime/coverage-audit follow-up, not the closed PR. |
| Hawkeye, Master Marksman | resolver-flagged (gap=0) | no PR; modal "pay {1} up to three times → choose up to that many" (MSH-E) |
| Cosmic Cube | Swallow:DynamicQty | MSH-F heavy cluster (plan-gated) |
| Hawkeye, Young Avenger | Swallow:DynamicQty | MSH-F heavy cluster (plan-gated) |
| Baron Helmut Zemo | ParseWarning:target-fallback | no PR; anaphoric "those exiled cards" (→ std S17) |
| Loki, God of Mischief | Trigger:became-target-of-ability | no PR; new trigger matcher (`/add-trigger`) |
| The Incredible Hulk | Swallow:Condition_If | no PR; Enrage "if he's attacking" tail |

### GitHub delta — resolved by the 2026-06-24 re-measure

Local main was fast-forwarded `ae663ee8c` → `a2c3033f8` (34 commits), then card-data
regenerated, so the three previously-open PRs are now measured, not predicted:

- **#4182 (Doctor Doom) → MERGED** (`2af0f855e`) — now **supported**; cleared from both
  the MSH and standard unsupported sets. ✓ (matches prediction)
- **#4169 (Hulkling) → MERGED** (`798857711`) — now **supported**; cleared from MSH. ✓
- **#4186 (Ruinous Wrecking Crew) → MERGED** (`23c50148a`) — **prediction REFUTED.**
  The earlier plan predicted this would clear the card (9→8). Measured against fresh
  data with #4186's code confirmed in HEAD, Ruinous is **still resolver-flagged
  unsupported**. The merged "dynamic modal max (choose up to X)" work apparently does
  not satisfy whatever the coverage tool's resolver audit checks for this card. **Action:**
  re-open Ruinous as an MSH-E follow-up (audit why `gap_count==0` yet `supported==false`)
  — do NOT mark it done on the strength of the merged PR.

Net MSH unsupported **9 → 7** (−2: Doctor Doom + Hulkling). `#4202` (restricted
`ChoiceType::CardType`) merged adjacently and was the card-type-enumeration groundwork
#4182 (Plan card type) rode on. `#4203` unlocked MSH/MSC/TMSH on release (removed
`GATED_SETS`). The remaining MSH resolver-flagged pair (Ruinous, Hawkeye Master Marksman)
**also appears in the standard unsupported set** — see the standard plan.

Cluster prose below predates this re-measure (it enumerated the original 13) — re-run
`cluster-assign.sh MSH` to refresh per-card cluster files before dispatching new work.

## Clusters (all 13 — every card has a home)

### MSH-A — "for each Equipment attached to it" dynamic count · **2** · HIGH ROI (real 12-card class)
Reusable `QuantityRef` = Equipment attached to the source. Covers a genuine 12-card class beyond MSH (Bruenor Battlehammer, Armament Master, Captain America Liberator, Catti-brie, Goblin Gaveleer, Golem-Skin Gauntlets, Kemba's Legion…) — build it first-class. CR 301.
- **Winter Soldier, Icy Assassin** — static "+2/+0 for each Equipment attached."
- **Whiplash, Vengeful Engineer** — trigger "lose/gain X = Equipment attached" (also the MSH-B intervening-if "if he's equipped").

### MSH-B — intervening-if on a trigger (= std **S03**) · **2** · `parse_inner_condition`
- ✅ **Hulkling, Burgeoning Bruiser** — DONE (#4169 merged `798857711`; supported as of the 2026-06-24 re-measure). "Whenever another creature you control enters, **if it has greater power or toughness than Hulkling**…" (entering-creature-vs-source P/T comparison).
- **Whiplash** — "if he's equipped" (overlaps MSH-A).

### MSH-C — count-with-offset dynamic quantity (= std **S08** ext) · **1**
- **Klaw, Sonic Subjugator** — "reveals **one plus** the number of creature cards in your graveyard," then choose/discard. `1 + ObjectCount` quantity.

### MSH-D — "as long as <board>" conditional static (resolver, = std **R2**) · **1**
- ✅ **Doctor Doom** — DONE (#4182 merged `2af0f855e`; supported as of the 2026-06-24 re-measure; `#4202` `ChoiceType::CardType` groundwork landed alongside). "As long as you control an artifact creature **or a Plan**, …has indestructible." Rode the R2 conditional-static evaluator + `Plan` card-type recognition. CR 604/611.

### MSH-E — modal "choose up to X" with dynamic / repeated count · **2** (resolver-flagged)
Building block: modal `ChooseUpTo { count: QuantityExpr }`. CR 601.2b/700.2.
- ⚠️ **The Ruinous Wrecking Crew** — enters with X counters; "choose up to X —" of 4 modes. **#4186 merged (`23c50148a`) yet STILL resolver-flagged unsupported in the 2026-06-24 re-measure** — needs a coverage-audit follow-up (why `gap_count==0` but `supported==false`), not a re-implement of the merged modal-max primitive.
- **Hawkeye, Master Marksman** — "pay {1} up to three times. When you do, choose up to that many" of 3 modes. Still resolver-flagged unsupported (no PR).

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

> Status as of 2026-06-27 @ `c1b61ded5`: MSH-B Hulkling ✅, MSH-D Doctor Doom ✅,
> MSH-singleton Black Widow ✅, and MSH-F Cosmic Cube + Hawkeye Young Avenger ✅ (#4471)
> are merged & supported. **5 cards remain unsupported** (down from 7); the MSH-E pair
> (Ruinous Wrecking Crew + Hawkeye Master Marksman) is shipping via PR #4482 (CI green).

1. **MSH-A** Equipment-attached count — reusable 12-card-class quantity (clears Winter Soldier + half of Whiplash). _(Winter Soldier/Whiplash are not in the current 7 unsupported — confirm against a fresh `cluster-assign.sh MSH` before dispatch.)_
2. ✅ **MSH-B** intervening-if (rides std S03) — Hulkling DONE (#4169).
3. ✅ **MSH-D** Doctor Doom DONE (#4182); **Black Widow** DONE (#4184). (Both rode the std R2 / S11 dispatches.)
4. **MSH-E** modal choose-up-to-X: **Hawkeye Master Marksman** (no PR) + **Ruinous Wrecking Crew follow-up** — Ruinous's #4186 is merged but still resolver-flagged; first audit *why* it stays unsupported (coverage tool resolver check), don't re-build the merged primitive.
5. Singletons: **Loki** trigger, **Baron Zemo** anaphor, **Incredible Hulk** additional-combat. _(MSH-C Klaw is no longer in the 7 unsupported — verify before dispatch.)_
6. **MSH-F** (Cosmic Cube, Hawkeye Young Avenger): `/review-engine-plan` first, then implement — **not deferred**, just plan-gated. (This was the in-flight cluster; pipeline blocked earlier on a GLM provider outage — re-dispatch on Opus.)
