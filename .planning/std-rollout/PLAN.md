# Standard-legal rollout plan

> **Hard rule (user, 2026-06-22): every card MUST be implemented. No "defer", no "out-of-scope."**
> All unsupported standard-legal cards are assigned to exactly one implementation cluster
> below. Hard cards are not dropped — they live in heavy-infra clusters gated by
> `/review-engine-plan` before code, but they ship. Backing artifact (per-card, exhaustive):
> `.planning/coverage-analysis/out/standard/cluster-assignment.tsv` (verified **0 unclustered**).
> _Re-measured 2026-06-23 @ `ae663ee8c`: 277 unsupported (was 268); pool grew +277 cards. See "Where standard stands"._

**Snapshot:** 2026-06-23 · main @ `ae663ee8c` (re-measured; was 2026-06-22 @ `c55670fd0`) · card-data regenerated against `ae663ee8c`.
**Method / reuse:** `coverage-breakdown.sh --format standard` then `cluster-assign.sh standard` (both in `.planning/coverage-analysis/`).

## Where standard stands

| metric | value (2026-06-23 @ `ae663ee8c`) | prior (2026-06-22 @ `c55670fd0`) |
|---|---|---|
| std-legal cards | 4924 | 4647 |
| supported | 4647 (**94.37 %**) | 4379 (94.23 %) |
| unsupported (all clustered below) | **277** = 245 parser-gap + 32 resolver-flagged | 268 = 239 parser-gap + 29 resolver-flagged |

> **Net read:** the pool grew **+277** standard-legal cards since the snapshot (new
> set releases / card-data refresh) while supported rose **+268** — so unsupported
> ticked **+9** net (268 → 277) even though 60 card-PRs merged. The fixes are real;
> the standard pool is simply growing slightly faster than it's being cleared.
> Top parser-gap handlers now: Swallow:Condition_If (77), Swallow:DynamicQty (38),
> Swallow:Duration_ThisTurn (15), Effect:for (13). Cluster prose below predates this
> re-measure — re-run `cluster-assign.sh standard` to refresh per-card cluster files.

## Tier 1 — shared-building-block clusters (fix once → unlock many; dispatch first)

| cluster | n | building block / approach | representative cards |
|---|---|---|---|
| **S01** reflexive "if it/that <past-state>" rider | 18 | `AbilityCondition`/`ReplacementCondition` over the just-affected object/event (`if it was tapped`, `if it had MV N`, `if it was dealt damage`, `if excess`). #3898 laid groundwork. CR 603/120. | Throw from the Saddle, Dose of Dawnglow, Faller's Faithful, Brackish Blunder, Driftgloom Coyote |
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
| **S21** static ability | 8 | continuous static the parser drops. | Fblthp Lost on the Range, Inquisitive Glimmer, Sandswirl Wanderglyph, Agatha of the Vile Cauldron, Nowhere to Run |
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

1. **S01** reflexive-if (18) — biggest shared parser cluster; also clears 76 in modern∩commander.
2. **R1 speed** (9) + **R2 as-long-as static** (11) — the two shared resolver subsystems.
3. **S08** for-each-qty (12) → **S04** activate-if (8) → **S05** alt-cost-if (9) → **S11** duration (9) → **S12** optional (7) → **S03** intervening-if (7) → **S02** cast-context (4).
4. Smaller shared clusters: S14, S20, S13, S17, S23, S06, S15, R4.
5. **Tier 2 family tracks**, largest first (S25, S07, S19, S10…), each dispatch owning one family; split into sub-batches of ~8-10 cards per `/engine-implementer` run.
6. **Tier 3**: run `/review-engine-plan` for S16 (and the flagged Tier-2 heavies), then implement.
7. Re-scan (`coverage-breakdown.sh --format standard && cluster-assign.sh standard`) after each tier to confirm the cluster shrinks and re-rank the residual.

Sequential dispatch; shared collision files (`types/ability.rs`, `parser/oracle.rs`, `effects/mod.rs`, layer system for R1/R2) — one cluster in flight unless file sets are disjoint.
