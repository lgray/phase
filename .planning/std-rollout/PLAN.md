# ⛔ OLD PLAN — SUPERSEDED 2026-07-03, DO NOT USE FOR NEW WORK ⛔

> This document is retained ONLY so in-flight tranches keep their meaning (the S25 40-card
> tranche and anything else already dispatched under these cluster definitions finishes under
> them). For ALL new tranche selection and ranking use **`PLAN-V2-TRACE.md`** in this
> directory — a trace-keyed re-clustering (cards grouped by the parser code path a fix must
> touch, per the S07 batching model) derived from measured data by
> `.planning/coverage-analysis/trace_cluster.py` @ main `b7194fc30`. The S/R cluster names
> below (S01…S29, R1…R5) remain valid HISTORICAL identifiers for shipped/in-flight work only.

# Standard-legal rollout plan

> **Hard rule (user, 2026-06-22): every card MUST be implemented. No "defer", no "out-of-scope."**
> All unsupported standard-legal cards are assigned to exactly one implementation cluster
> below. Hard cards are not dropped — they live in heavy-infra clusters gated by
> `/review-engine-plan` before code, but they ship. Backing artifact (per-card, exhaustive):
> `.planning/coverage-analysis/out/standard/cluster-assignment.tsv` (verified **0 unclustered**).
> _Re-measured 2026-06-27 @ `c1b61ded5` (55-commit jump from v0.7.0; card-data regenerated): **270 unsupported** (was 273 @ `5eca83b8c` v0.7.0, 274 @ `a2c3033f8`, 277 @ `ae663ee8c`). See "Where standard stands"._
> _Re-measured **2026-06-29 @ `dd6c22ea7`** (33-commit ff from `7c1c1cf67`; card-data regenerated, inputs stamp `2026-06-29T13:58Z`): **250 unsupported** = **220 parser-gap + 30 resolver-flagged**; **4674 supported (94.92 %)** of 4924. Net **270 → 250 (−20)**; parser-gap 238 → 220 (−18), resolver-flagged 32 → 30 (−2). Pool flat at 4924. **0 unclustered.** See the 2026-06-29 re-measure block in "Where standard stands"._
> _Re-measured **2026-07-01 @ `acd2f5e6b`** (89-commit ff from `dd6c22ea7`; card-data regenerated, inputs stamp `2026-07-01T20:05Z`): **231 unsupported** = **203 parser-gap + 28 resolver-flagged**; **4693 supported (95.31 %)** of 4924. Net **250 → 231 (−19)**; parser-gap 220 → 203 (−17), resolver-flagged 30 → 28 (−2). Pool flat at 4924. **0 unclustered.** **✅ S01 reflexive-if (11 → 0) and S21 static (2 → 0) are now FULLY CLEARED** — S01 via #4688 (`f70323d5a`, 11 cards / 7 mechanic groups), S21 via #4611 (`75a25277e`, Koh + Sandswirl Wanderglyph). See the 2026-07-01 re-measure block in "Where standard stands"._

**Snapshot:** 2026-06-27 · local main at `c1b61ded5` (55-commit jump from v0.7.0 `5eca83b8c`)
· card-data + coverage-data regenerated against `c1b61ded5` (inputs stamp `2026-06-27T18:02:59Z`).
Prior snapshots: 2026-06-26 @ `5eca83b8c` (**release v0.7.0**), 2026-06-24 @ `a2c3033f8` (#4280), 2026-06-23 @ `ae663ee8c`, 2026-06-22 @ `c55670fd0`.
**Method / reuse:** `coverage-breakdown.sh --format standard` then `cluster-assign.sh standard` (both in `.planning/coverage-analysis/`).

## Where standard stands

| metric | value (**2026-07-01 @ `acd2f5e6b`, re-measured**) | prior (2026-06-29 @ `dd6c22ea7`) | prior (2026-06-27 @ `c1b61ded5`) | prior (2026-06-26 @ `5eca83b8c` v0.7.0) | prior (2026-06-24 @ `a2c3033f8`) |
|---|---|---|---|---|---|
| std-legal cards | 4924 | 4924 | 4924 | 4924 | 4924 |
| supported | **4693 (95.31 %)** | 4674 (94.92 %) | 4654 (94.52 %) | 4651 (94.46 %) | 4650 (94.44 %) |
| unsupported (all clustered below) | **231** = 203 parser-gap + 28 resolver-flagged | 250 = 220 parser-gap + 30 resolver-flagged | 270 = 238 parser-gap + 32 resolver-flagged | 273 = 241 parser-gap + 32 resolver-flagged | 274 = 242 parser-gap + 32 resolver-flagged |

> **Net read (2026-07-01 @ `acd2f5e6b`, 89-commit ff):** pool flat at 4924; supported **+19**
> (4674 → 4693 = 95.31 %), unsupported **250 → 231 (−19)** — parser-gap 220 → 203 (−17),
> resolver-flagged 30 → 28 (−2). **✅ S01 reflexive-if 11 → 0 and S21 static 2 → 0 — both clusters
> FULLY CLEARED** (S01 = #4688 `f70323d5a`, 11 cards / 7 mechanic groups; S21 = #4611 `75a25277e`,
> Koh, the Face Stealer + Sandswirl Wanderglyph). **Fresh full cluster table (231 cards, 0 unclustered):**
> S25-effect-verb 40 · S07-condition-if 28 · S19-new-trigger 21 · S10-dynamic-qty 21 ·
> S08-foreach-qty 11 · R5-runtime 10 · S18-foreach-simple 9 · S11-duration 9 · S05-alt-cost-if 9 ·
> R2-aslongas 9 · R1-speed 9 · S24-unknown 8 · S04-activate-if 8 · S03-intervening-if 6 ·
> S20-copy-retarget 4 · S17-anaphoric 4 · S16-foreach-player-HEAVY 4 · S14-unless 4 · S02-cast-context 4 ·
> S22-choose 3 · S13-aslongas-parse 3 · S12-optional 3 · S23-alt-cost 2 · S06-saga 2.
> **(S01-reflexive-if and S21-static are now 0 — absent from the table.)** **Cluster movers vs the
> 2026-06-29 table:** **S01 11 → 0**, **S21 2 → 0**, **S12 optional 6 → 3 (−3)**, **R2 as-long-as
> 11 → 9 (−2)**, **S07 condition-if 29 → 28 (−1)**; all other clusters flat. **Handler histogram:**
> Swallow:Condition_If now **57** (down from 76 @ `c1b61ded5` — the S01 reflexive-if riders were
> Condition_If cards); **Effect:static_structure now 0** (the S21 gap handler is fully gone from the
> standard pool). Resolver-flagged 28 = R2 (9) + R5 (10) + R1 (9) — the three runtime subsystems,
> untouched this window. No regressions: no card recorded cleared reappeared in `unsupported.tsv`.

> **Net read (2026-06-29 @ `dd6c22ea7`, 33-commit ff):** pool flat at 4924; supported **+20**
> (4654 → 4674), unsupported **270 → 250 (−20)** — parser-gap 238 → 220 (−18), resolver-flagged
> 32 → 30 (−2). **Fresh full cluster table (250 cards, 0 unclustered):**
> S25-effect-verb 40 · S07-condition-if 29 · S19-new-trigger 21 · S10-dynamic-qty 21 ·
> S08-foreach-qty 11 · **S01-reflexive-if 11** · R2-aslongas 11 · R5-runtime 10 · S18-foreach-simple 9 ·
> S11-duration 9 · S05-alt-cost-if 9 · R1-speed 9 · S24-unknown 8 · S04-activate-if 8 ·
> S12-optional 6 · S03-intervening-if 6 · S20-copy-retarget 4 · S17-anaphoric 4 · S16-foreach-player-HEAVY 4 ·
> S14-unless 4 · S02-cast-context 4 · S22-choose 3 · S13-aslongas-parse 3 · S23-alt-cost 2 ·
> **S21-static 2** · S06-saga 2. (S15-activate-during and R4-cant-restriction now **0** — fully cleared.)
> **Cluster movers vs the 2026-06-23 baseline table:** **S01 reflexive-if 17 → 11** (6 cleared via
> #4553/#4559/#4567 — PARTIAL: #4553's recognizer handles only the FilterProp-on-affected-object shape;
> the 11 remaining are OTHER reflexive shapes — card-type, excess-damage, replacement-instead, phase,
> subtype, cast-context, compound — none yet delivered); **S21 static 8 → 2** (6 cleared via
> #4551/#4557/#4561/#4573; remaining Koh + Sandswirl Wanderglyph); **S19 24 → 21**, **S10 24 → 21**,
> **S08 12 → 11**, **S25 41 → 40**, **R5 11 → 10**, **S12/S03 7 → 6**, **S23 3 → 2**, **S15/R4 1 → 0**.
> Resolver-flagged 30 = R2 as-long-as static (11) + R5 runtime-bespoke (10) + R1 speed (9) — the three
> runtime subsystems, untouched this window. No regressions: no card recorded cleared reappeared in `unsupported.tsv`.

> **Net read (2026-06-27 @ `c1b61ded5`, 55-commit jump from v0.7.0):** pool size held at
> 4924; supported rose **+3** (4651 → 4654), so unsupported dropped **273 → 270 (−3)**.
> Parser-gap fell **241 → 238 (−3)**; resolver-flagged held at **32** (full set unchanged —
> the 3 cleared are all parser-gap cards). Top parser-gap handlers unchanged in rank:
> Swallow:Condition_If (76), Swallow:DynamicQty (34, was 37), Swallow:Duration_ThisTurn (15),
> Effect:for (13). **Cleared-card attribution:** the prior `out/standard/unsupported.tsv` (at
> `5eca83b8c`) was overwritten by this re-measure, so an exact set-diff is not recoverable
> (same limitation as the prior re-measure). Strong candidate among the 55-commit parser
> one-offs: **Dragonfire Blade** (#4426, DFT, std-member, now supported); the other two are
> not pinned to a named PR to avoid an unverifiable claim. **Regression check: clean** — no
> card the plan recorded as cleared/merged/supported (Doctor Doom #4182, Bloodchief's Thirst,
> Macabre Waltz, Insidious Roots, Show and Tell, Maarika/Rith) reappears in the new
> unsupported set. The 2 MSH-E resolver-flagged cards (The Ruinous Wrecking Crew, Hawkeye,
> Master Marksman) remain unsupported as expected (clear only when #4482 merges). Cluster
> tables below still reflect the 2026-06-23 `ae663ee8c` (277) clustering — vs that recorded
> histogram the new (270) set shrank S10 (24→21), S25 (41→40), S19 (24→23), S08 (12→11),
> S03 (7→6); no cluster grew, no new handler bucket, **0 unclustered** re-confirmed.

> **Net read (2026-06-26 @ `5eca83b8c`):** pool size held at 4924; supported rose **+1** (4650 → 4651), so
> unsupported dropped **274 → 273**. Parser-gap fell 242 → 241 (−1); resolver-flagged
> held at 32. Top parser-gap handlers unchanged: Swallow:Condition_If (76), Swallow:DynamicQty
> (37), Swallow:Duration_ThisTurn (15), Effect:for (13). Cluster prose below predates
> this re-measure — re-run `cluster-assign.sh standard` to refresh per-card cluster files.

## GitHub delta — `a2c3033f8` → `5eca83b8c` (v0.7.0), re-measured 2026-06-26

Local main was fast-forwarded `a2c3033f8` → `5eca83b8c` (**75 commits**; release v0.7.0),
then card-data regenerated. Measured standard unsupported **274 → 273 (net −1)**;
resolver-flagged held at 32 (so the −1 is a single parser-gap card clearing). Many
parser/engine PRs landed in this window — a sample of the parser-coverage-relevant ones:
#4391 (The Second Doctor "can't attack you" cluster — *not* a standard member),
#4340 (game-state `unless` gates → effect conditions), #4333 (equal-to / enter-with-counters
→ full CDA), #4286 (`ControllerRef::EnchantedPlayer` triggers), #4254 (bare "except it's
legendary" on token copies), #4310 ("beneath the top X cards" for Unexpectedly Absent),
#4318 (Memory's Journey up-to-three count). Most of these were runtime/bug fixes on
already-supported cards (no coverage delta) or affected non-standard pools, which is why
the net parse-coverage movement is only −1.

> **Exact cleared card not pinpointed:** the prior `out/standard/unsupported.tsv` (at
> `a2c3033f8`) was overwritten by this re-measure run, so an exact set-diff is not
> recoverable. The −1 is the measured headline movement; the specific card is not
> credited to a named PR here to avoid an unverifiable claim. Re-run with the list
> preserved if attribution is needed.

### GitHub delta — resolved by the 2026-06-24 re-measure (`ae663ee8c` → `a2c3033f8`)

Local main was fast-forwarded `ae663ee8c` → `a2c3033f8` (34 commits), then card-data
regenerated, so the previously-pending PRs are now **measured**, not predicted:

**Merged & confirmed cleared from the standard unsupported set (measured):**
- **#4182 Doctor Doom** (`2af0f855e`) — was standard resolver-flagged; now **supported**.
  Confirmed absent from the new `out/standard/unsupported.tsv`. ✓ (matches prediction)
  (`#4202` restricted `ChoiceType::CardType` landed as its card-type-enumeration groundwork.)

**Merged but PREDICTION REFUTED — still unsupported (measured):**
- **#4186 The Ruinous Wrecking Crew** (`23c50148a`, confirmed ancestor of HEAD) — the
  earlier plan predicted this clears the card (277 → 276). Measured against fresh data,
  Ruinous is **still resolver-flagged unsupported** in both the standard and MSH pools.
  The merged "dynamic modal max (choose up to X)" work did not satisfy the coverage
  tool's resolver audit for this card. **Action:** keep Ruinous in scope as a follow-up
  (audit why `gap_count==0` yet `supported==false`); do not credit the closed PR.

**Merged — bug fixes on already-supported cards (no delta; still verified absent from unsupported):**
- #4112 (Maarika/Rith excess-damage intervening-if), #4193 (Bloodchief's Thirst),
  #4190 (Adventure instant faces), #4188 (dual-target fight), #4191 (Show and Tell),
  #4196 (Macabre Waltz), #4194 (Insidious Roots), #4276 (no-legal-target trigger drop),
  #4167 (Contraptions assemble/crank — new subsystem, no Contraption was unsupported).

**Not in standard pool:** #4169 Hulkling (MSH-only) merged (`798857711`) — cleared MSH, no standard delta.

**Net standard unsupported `277 → 274` (−3):** Doctor Doom (#4182) + 2 other cards cleared
across the 34-commit fast-forward. **Ruinous (#4186) is NOT one of them** despite being
merged. These numbers are the real re-measure at `a2c3033f8`, not a snapshot+delta estimate.

## Tier 1 — shared-building-block clusters (fix once → unlock many; dispatch first)

| cluster | n | building block / approach | representative cards |
|---|---|---|---|
| **S01** ✅ **DONE** reflexive "if it/that <past-state>" rider | ~~18~~ **0** | `AbilityCondition`/`ReplacementCondition` over the just-affected object/event (`if it was tapped`, `if it had MV N`, `if it was dealt damage`, `if excess`). #3898 laid groundwork. CR 603/120. **Standard set fully cleared @ `acd2f5e6b` (last 11 via #4688 `f70323d5a`).** | ~~Throw from the Saddle, Dose of Dawnglow, Faller's Faithful, Brackish Blunder, Driftgloom Coyote~~ — all supported |
| **S08** "for each <count>" → effect quantity | 12 | `parse_for_each_clause` + `QuantityExpr::ObjectCount` into existing effects. | Luxurious Locomotive, Bounding Felidar, Machinist's Arsenal, Diligent Zookeeper, Teysa Opulent Oligarch |
| **R2** "as long as <board>" conditional static (resolver) | 11 | runtime evaluator for static-ability conditions over board state. CR 604/611. | Living Conundrum, Knight of Malice, Elenda Saint of Dusk, Howling Galefang, Hundred-Battle Veteran |
| **S05** alt-cost / cast-permission "if" | 9 | cast-permission gated by condition (graveyard/flash/cost-reduction-if). | Sandman's Quicksand, Valgavoth Terror Eater, Otterball Antics, Antiquities on the Loose, Lashwhip Predator |
| **S11** "until end of turn / this turn" duration grant | 9 | duration wrapper on granted effect/keyword (incl. impulse-cast-until-EOT). | Subterranean Schooner, Sidequest: Hunt the Mark, Thaumaton Torpedo, Torch the Tower, Treacherous Greed |
| **R1** Speed / "Start your engines!" (resolver) | 9 | speed-counter + max-speed gating runtime subsystem. | Burnout Bashtronaut, Gastal Thrillseeker, Streaking Oilgorger, Racers' Scoreboard, Lightwheel Enhancements |
| **S04** "Activate only if <condition>" | 8 | activation-condition gate (mirror "activate only as a sorcery"). | Cavernous Maw, Temple of the Dead, Master's Manufactory, Puca's Eye, Bonecache Overseer |
| **S12** optional "you may <sub-effect>" | 7 | optional sub-effect parse inside a larger ability. | Omniscience, Zaffai and the Tempests, Magitek Scythe, Hades Sorcerer of Eld, Mirrormind Crown |
| **S03** intervening-if on ETB/attack trigger | 7 | delegate to `oracle_nom/condition.rs::parse_inner_condition`. CR 603.4. | Fearless Swashbuckler, Massacre Girl Known Killer, Anti-Venom, Sharp-Eyed Rookie, Stalwart Successor |
| **S02** "if this spell was cast [from/for]" cast-context | 4 | cast-context condition variant. | Leonardo Sewer Samurai, Ran and Shaw, Intrepid Paleontologist, Freestrider Commando |
| **S14** "unless" clause | 4 | `UnlessQuantity`/unless-condition parse. | Waterbending Lesson, Steamcore Scholar, Combustion Man, Repulsive Mutation |
| **S20** orphaned copy-retarget | 4 | copy/retarget resolution for the affected object. CR 707. | Pit Automaton, Spider-Verse, Spinerock Tyrant, Pyromancer's Goggles |
| **S13** "as long as" gating (parser side) | 3 | `Condition_AsLongAs` parse → static condition. | Tishana's Tidebinder, Braided Net, Cloud Planet's Champion |
| **S17** anaphoric / unclassified target | 4 | extend `parse_target` for anaphors ("those exiled cards"). | Emeritus of Ideation, Ultimecia Time Sorceress, Baron Helmut Zemo, Grub Notorious Auntie |
| **S23** alt-cost cost parsing | 3 | `parse_single_cost` extension for named/alt costs. | Close Encounter, Page Loose Leaf, Ignis Scientia |
| **S06** Saga chapter conditional | 2 | intervening-if on a Saga chapter (verify S01/S03 cover it). | The Tale of Tamiyo, Maximum Carnage |
| **S15** "activate only during <phase>" | 1 | `ActivateOnlyDuring` timing gate. | Katara, Water Tribe's Hope |
| **R4** "can't <attack/block/cast/…>" restriction static (resolver) | 1 | runtime continuous static restriction. CR 604. _(new cluster this re-measure.)_ | Hawkeye, Master Marksman |

Tier-1 subtotal: **116**.

## Tier 2 — per-card family tracks (each card an individual fix, grouped by mechanism — all in scope)

These do not share one cheap building block; each is its own small implementation, but grouped so a single dispatch owns a coherent family. Full per-card lists in `cluster-assignment.tsv`.

| family | n | nature | sample cards (heavy ones flagged → Tier 3 gate) |
|---|---|---|---|
| **S25** effect-verb bespoke | 41 | each an effect the parser can't yet lower (verb-specific). | Graceful Takedown, Foraging Wickermaw, Parker Luck, Shifting Scoundrel; **Quick Draw**, **Vraska the Silencer** (→ review-plan) |
| **S07** condition-if bespoke | 29 | `Condition_If` shapes outside S01-S06. | Slumbering Trudge, Fear of Immobility, Agency Coroner, Cinder Strike, Oviya Automech Artisan |
| **S19** new trigger matcher | 24 | each needs a `TriggerDefinition` matcher (`/add-trigger`). | The Millennium Calendar, Corpseberry Cultivator, Eriette the Beguiler, Shinryu Transcendent Rival, Curator of Sun's Creation |
| **S10** dynamic-qty bespoke | 24 | DynamicQty not reducible to plain for-each. | Hama the Bloodbender, Bumi King of Three Trials, Fractalize, Judgment Bolt; **Zimone Paradox Sculptor** (prime/advanced count → review-plan) |
| **R5** runtime bespoke (resolver) | 11 | parses, per-card runtime gap. | Fire Magic, Cecil Dark Knight, Throne of the Grim Captain, Restless Prairie, Timeline Culler |
| **S18** for-each simple count | 9 | straightforward per-iteration effect. | Rottenmouth Viper, Heirloom Epic, Hollow Marauder, Twisted Sewer-Witch; **Doppelgang** (X-target fan-out → review-plan) |
| **S21** ✅ **DONE** static ability | ~~8~~ **0** | continuous static the parser drops. **Standard set fully cleared @ `acd2f5e6b` (last 2 — Koh, the Face Stealer + Sandswirl Wanderglyph — via #4611 `75a25277e`; earlier via #4551/#4557/#4561/#4573).** | ~~Fblthp Lost on the Range, Inquisitive Glimmer, Sandswirl Wanderglyph, Agatha of the Vile Cauldron, Nowhere to Run~~ — all supported |
| **S24** unknown-effect bespoke | 8 | `Effect:unknown` — real cards, per-card lowering. | Dawnhand Dissident, Edgar King of Figaro, Sorcerous Spyglass, Warped Space, Leyline of Transformation |
| **S22** choose-effect | 3 | choice-effect parse/resolution. | The Legend of Yangchen, Discerning Financier, Calamity Galloping Inferno |

Tier-2 subtotal: **157**.

## Tier 3 — heavy-infra clusters (IN SCOPE; `/review-engine-plan` before any code)

| cluster | n | missing subsystem | representative cards |
|---|---|---|---|
| **S16** for-each-PLAYER object-target enumeration | 4 | `ChooseFromZone{EachPlayer}` + battlefield target fan-out in the targeting state machine. | Kaya Spirits' Justice, Winnowing, Unstable Glyphbridge, Kitesail Larcenist |

Plus the individually-heavy cards flagged inside Tier 2 (Quick Draw opponent-constrained target slot, Vraska return-as-Treasure-with-ability, Zimone prime-count quantity, Doppelgang X-target fan-out) — all four re-verified present in this re-measure — each gets a `/review-engine-plan` pass when its family track reaches it, but **none are dropped.**

Tier-3 subtotal (dedicated cluster): **4**.   **Grand total: 116 + 157 + 4 = 277.** ✓ (re-measured 2026-06-23 @ `ae663ee8c`; cluster-assignment.tsv has 0 unclustered.)

## Dispatch order (medium effort, sequential, via `/engine-implementer`)

1. ~~**S01** reflexive-if (18)~~ ✅ **DONE @ `acd2f5e6b`** (0 standard remaining; bled 11 off modern∩commander, 61 → 50). ~~**S21** static (8)~~ ✅ **DONE** (0 standard remaining; bled 2 off modern∩commander, 65 → 63).
2. **R1 speed** (9) + **R2 as-long-as static** (9) — the two shared resolver subsystems (R2 11 → 9 this window).
3. **S08** for-each-qty (12) → **S04** activate-if (8) → **S05** alt-cost-if (9) → **S11** duration (9) → **S12** optional (7) → **S03** intervening-if (7) → **S02** cast-context (4).
4. Smaller shared clusters: S14, S20, S13, S17, S23, S06, S15, R4.
5. **Tier 2 family tracks**, largest first (S25, S07, S19, S10…), each dispatch owning one family; split into sub-batches of ~8-10 cards per `/engine-implementer` run.
6. **Tier 3**: run `/review-engine-plan` for S16 (and the flagged Tier-2 heavies), then implement.
7. Re-scan (`coverage-breakdown.sh --format standard && cluster-assign.sh standard`) after each tier to confirm the cluster shrinks and re-rank the residual.

Sequential dispatch; shared collision files (`types/ability.rs`, `parser/oracle.rs`, `effects/mod.rs`, layer system for R1/R2) — one cluster in flight unless file sets are disjoint.
