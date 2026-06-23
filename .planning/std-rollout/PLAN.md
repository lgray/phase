# Standard-legal rollout plan

> **Hard rule (user, 2026-06-22): every card MUST be implemented. No "defer", no "out-of-scope."**
> All 268 unsupported standard-legal cards are assigned to exactly one implementation cluster
> below. Hard cards are not dropped — they live in heavy-infra clusters gated by
> `/review-engine-plan` before code, but they ship. Backing artifact (per-card, exhaustive):
> `.planning/coverage-analysis/out/standard/cluster-assignment.tsv` (verified **0 unclustered**).

**Snapshot:** 2026-06-22 · main @ `c55670fd0` (== upstream/main) · card-data regenerated this session (built against `c55670fd0`).
**Method / reuse:** `coverage-breakdown.sh --format standard` then `cluster-assign.sh standard` (both in `.planning/coverage-analysis/`).

## Where standard stands

| metric | value |
|---|---|
| std-legal cards | 4647 |
| supported | 4379 (**94.23 %**) |
| unsupported (all clustered below) | **268** = 239 parser-gap + 29 resolver-flagged |

## Tier 1 — shared-building-block clusters (fix once → unlock many; dispatch first)

| cluster | n | building block / approach | representative cards |
|---|---|---|---|
| **S01** reflexive "if it/that <past-state>" rider | 17 | `AbilityCondition`/`ReplacementCondition` over the just-affected object/event (`if it was tapped`, `if it had MV N`, `if it was dealt damage`, `if excess`). #3898 laid groundwork. CR 603/120. | Consuming Ashes, Dose of Dawnglow, Wisecrack, Summoner's Grimoire, Kylox's Voltstrider |
| **S08** "for each <count>" → effect quantity | 12 | `parse_for_each_clause` + `QuantityExpr::ObjectCount` into existing effects. | Teysa Opulent Oligarch, Sheriff of Safe Passage, Dragonfire Blade, Bounding Felidar |
| **R2** "as long as <board>" conditional static (resolver) | 10 | runtime evaluator for static-ability conditions over board state. CR 604/611. | Living Conundrum, Howling Galefang, Elenda Saint of Dusk, The Lunar Whale |
| **S05** alt-cost / cast-permission "if" | 9 | cast-permission gated by condition (graveyard/flash/cost-reduction-if). | Noctis Prince of Lucis, Undead Sprinter, Lashwhip Predator, Antiquities on the Loose |
| **S11** "until end of turn / this turn" duration grant | 9 | duration wrapper on granted effect/keyword (incl. impulse-cast-until-EOT). | Inventive Wingsmith, Reno and Rude, Thaumaton Torpedo, Sidequest: Hunt the Mark |
| **R1** Speed / "Start your engines!" (resolver) | 9 | speed-counter + max-speed gating runtime subsystem. | Lightwheel Enhancements, Racers' Scoreboard, Streaking Oilgorger, Nesting Bot |
| **S04** "Activate only if <condition>" | 8 | activation-condition gate (mirror "activate only as a sorcery"). | Bonecache Overseer, Matzalantli, Cavernous Maw, Temple of Power/the Dead |
| **S12** optional "you may <sub-effect>" | 7 | optional sub-effect parse inside a larger ability. | Magitek Scythe, Hades Sorcerer of Eld, Omniscience, Dracogenesis |
| **S03** intervening-if on ETB/attack trigger | 6 | delegate to `oracle_nom/condition.rs::parse_inner_condition`. CR 603.4. | Sharp-Eyed Rookie, Massacre Girl Known Killer, Stalwart Successor, Anti-Venom |
| **S02** "if this spell was cast [from/for]" cast-context | 4 | cast-context condition variant. | Freestrider Commando, Ran and Shaw, Leonardo Sewer Samurai |
| **S14** "unless" clause | 4 | `UnlessQuantity`/unless-condition parse. | Steamcore Scholar, Repulsive Mutation, Combustion Man, Waterbending Lesson |
| **S20** orphaned copy-retarget | 4 | copy/retarget resolution for the affected object. CR 707. | Pit Automaton, Pyromancer's Goggles, Spinerock Tyrant, Spider-Verse |
| **S13** "as long as" gating (parser side) | 3 | `Condition_AsLongAs` parse → static condition. | Tishana's Tidebinder, Cloud Planet's Champion, Braided Net |
| **S17** anaphoric / unclassified target | 3 | extend `parse_target` for anaphors ("those exiled cards"). | Emeritus of Ideation, Grub Notorious Auntie, Ultimecia |
| **S23** alt-cost cost parsing | 3 | `parse_single_cost` extension for named/alt costs. | Close Encounter, Ignis Scientia, Page Loose Leaf |
| **S06** Saga chapter conditional | 2 | intervening-if on a Saga chapter (verify S01/S03 cover it). | The Tale of Tamiyo, Maximum Carnage |
| **S15** "activate only during <phase>" | 1 | `ActivateOnlyDuring` timing gate. | Katara, Water Tribe's Hope |

Tier-1 subtotal: **111**.

## Tier 2 — per-card family tracks (each card an individual fix, grouped by mechanism — all in scope)

These do not share one cheap building block; each is its own small implementation, but grouped so a single dispatch owns a coherent family. Full per-card lists in `cluster-assignment.tsv`.

| family | n | nature | sample cards (heavy ones flagged → Tier 3 gate) |
|---|---|---|---|
| **S25** effect-verb bespoke | 41 | each an effect the parser can't yet lower (verb-specific). | Rhino's Rampage, The End, Glen Elendra's Answer; **Quick Draw**, **Vraska the Silencer** (→ review-plan) |
| **S07** condition-if bespoke | 29 | `Condition_If` shapes outside S01-S06. | Arid Archway, Take the Fall, Break the Spell, Eliminate the Impossible, Avatar Aang |
| **S19** new trigger matcher | 23 | each needs a `TriggerDefinition` matcher (`/add-trigger`). | Case File Auditor, Grievous Wound, Firebender Ascension, Shinryu, Surrak Elusive Hunter |
| **S10** dynamic-qty bespoke | 22 | DynamicQty not reducible to plain for-each. | Judgment Bolt, Hama, Solstice Revelations; **Zimone Paradox Sculptor** (prime/advanced count → review-plan) |
| **R5** runtime bespoke (resolver) | 10 | parses, per-card runtime gap. | Season of Loss, Tifa's Limit Break, Fire Magic, Throne of the Grim Captain |
| **S18** for-each simple count | 9 | straightforward per-iteration effect. | Twisted Sewer-Witch, Hollow Marauder, Sovereign Okinec Ahau; **Doppelgang** (X-target fan-out → review-plan) |
| **S21** static ability | 8 | continuous static the parser drops. | Fblthp Lost on the Range, Doc Aurlock, Nowhere to Run, Agatha of the Vile Cauldron |
| **S24** unknown-effect bespoke | 8 | `Effect:unknown` — real cards, per-card lowering. | Tinybones Bauble Burglar, Elvish Refueler, Sorcerous Spyglass, Edgar King of Figaro |
| **S22** choose-effect | 3 | choice-effect parse/resolution. | Calamity Galloping Inferno, The Legend of Yangchen, Discerning Financier |

Tier-2 subtotal: **153**.

## Tier 3 — heavy-infra clusters (IN SCOPE; `/review-engine-plan` before any code)

| cluster | n | missing subsystem |
|---|---|---|
| **S16** for-each-PLAYER object-target enumeration | 4 | `ChooseFromZone{EachPlayer}` + battlefield target fan-out in the targeting state machine. | Kitesail Larcenist, Winnowing, Kaya Spirits' Justice, Unstable Glyphbridge |

Plus the individually-heavy cards flagged inside Tier 2 (Quick Draw opponent-constrained target slot, Vraska return-as-Treasure-with-ability, Zimone prime-count quantity, Doppelgang X-target fan-out) — each gets a `/review-engine-plan` pass when its family track reaches it, but **none are dropped.**

Tier-3 subtotal (dedicated cluster): **4**.   **Grand total: 111 + 153 + 4 = 268.** ✓

## Dispatch order (medium effort, sequential, via `/engine-implementer`)

1. **S01** reflexive-if (17) — biggest shared parser cluster; also clears 89 in modern∩commander.
2. **R1 speed** (9) + **R2 as-long-as static** (10) — the two shared resolver subsystems.
3. **S08** for-each-qty (12) → **S04** activate-if (8) → **S05** alt-cost-if (9) → **S11** duration (9) → **S12** optional (7) → **S03** intervening-if (6) → **S02** cast-context (4).
4. Smaller shared clusters: S14, S20, S13, S17, S23, S06, S15.
5. **Tier 2 family tracks**, largest first (S25, S07, S19, S10…), each dispatch owning one family; split into sub-batches of ~8-10 cards per `/engine-implementer` run.
6. **Tier 3**: run `/review-engine-plan` for S16 (and the flagged Tier-2 heavies), then implement.
7. Re-scan (`coverage-breakdown.sh --format standard && cluster-assign.sh standard`) after each tier to confirm the cluster shrinks and re-rank the residual.

Sequential dispatch; shared collision files (`types/ability.rs`, `parser/oracle.rs`, `effects/mod.rs`, layer system for R1/R2) — one cluster in flight unless file sets are disjoint.
