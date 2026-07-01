I now have a complete, measured picture. Producing the plan.

---

# PLAN — LINE 4: S21-static-ability STANDARD cluster (8 cards)

## (1) Summary + skills followed

I read full Oracle text for all 8 cards from `data/card-data.json` (the live parser export) — NOT the snippets — and located the exact swallowed clause for each by inspecting `abilities[]` (Unimplemented), `triggers[].execute.effect`, and `static_abilities[]`. Skills consulted: `add-static-ability`, `add-engine-effect` (read in full), plus targeted reads of the cost/static/targeting infrastructure they point to. The headline result: this "cluster" is a catch-all, and the 8 cards decompose into **5 distinct building blocks**, three of which heavily REUSE existing infrastructure rather than needing new variants. Three cards (Sandswirl, Koh, Fblthp) have a **second, non-static gap** that must also be fixed for the card to flip to supported — I flag those for re-homing.

Measured per-card gap (file = `data/card-data.json`):

| Card | Already parsed | The swallowed clause(s) | Gap kind |
|---|---|---|---|
| Doc Aurlock | L1 cast-cost `ModifyCost{Reduce,2,Card@[GY,Exile]}` ✓ | L2 "Plotting cards from your hand cost {2} less" | static (special-action cost) |
| Inquisitive Glimmer | L1 `ModifyCost{Reduce,1,Enchantment}` ✓ | L2 "Unlock costs you pay cost {1} less" | static (special-action cost) |
| Agatha | {4}{R}{G} pump activated ✓ | L1 whole "Activated abilities of creatures you control cost {X} less… X is ~'s power… floor one mana" | static (dynamic ReduceAbilityCost) |
| Nowhere to Run | Flash ✓, ETB Pump −3/−3 trigger ✓ | L3 "Creatures your opponents control … as though they didn't have hexproof. Ward abilities of those creatures don't trigger." | static (IgnoreHexproof + ward-suppress) |
| Locus of Enlightenment | L2 CopySpell trigger ✓ | L1 "~ has each activated ability of the exiled cards used to craft it. … only once each turn." | static (grant-from-exiled) |
| Sandswirl | Flying ✓ | **L2 trigger effect is ALSO Unimplemented** `can't attack you or planeswalkers` (`Effect:can't`); L3 "Each opponent who attacked you or a planeswalker you control this turn can't cast spells" (`static_structure`) | 1 effect (re-home) + 1 static |
| Fblthp | Ward {2} ✓, MayLookAtTopOfLibrary ✓ | L3 "The top card of your library has plot…" (`static_structure`); **L4 "You may plot nonland cards from the top of your library" (`effect_structure`)** | 2 statics (play-from-top) |
| Koh | ETB exile ✓, dies→exile trigger ✓ | **L3 "Pay 1 life: Choose a creature card exiled with ~" (`Effect:choose`, activated)**; L4 "~ has all activated and triggered abilities of the last chosen card" (`static_structure`) | 1 interactive effect (re-home) + 1 static |

## (2) Building-block DECOMPOSITION

| # | Building block | Cards (standard) | New type surface | Reuse | mc∩commander tail reach |
|---|---|---|---|---|---|
| **B1** | Dynamic + filter-scoped **activated-ability** cost reduction | Agatha (1) | none (fields exist) | `StaticMode::ReduceAbilityCost` (statics.rs:893) already has `minimum_mana` + `dynamic_count`; `QuantityRef::Power{scope:Source}` (ability.rs:4092); `ObjectScope::Source` (ability.rs:3858) | Zirda the Dawnwaker is an adjacent sibling ("Abilities you activate that aren't mana abilities cost {2} less… floor one mana") — same family/runtime; the runtime dynamic-count fix benefits the whole `ReduceAbilityCost` dynamic family. Conservative reach: **1–2** |
| **B2** | **Keyword/special-action** cost reduction (plot, unlock) | Doc Aurlock L2, Inquisitive L2 (2) | new `StaticMode::ReduceActionCost{action,mode,amount}` + `SpecialAction::Plot` variant | `SpecialAction` enum already has `UnlockDoor` (mana.rs:343, CR 116.2m+709.5e); `CostModifyMode`; generic-mana reducers | Currently unique in the 35k corpus (jq scan returned only these 2). Parameterized over `SpecialAction` so any future "X costs you pay cost {N} less" lands free. Reach: **2** |
| **B3** | Filter-scoped **IgnoreHexproof** + **ward-suppression** | Nowhere to Run (1) | extend `IgnoreHexproof` to a battlefield/filter-scoped static; new `SuppressedTriggerEvent::BecomesTargeted` (ward) | `StaticMode::IgnoreHexproof` (statics.rs:990) + transient grant plumbing in targeting.rs:2082-2168; `StaticMode::SuppressTriggers` (statics.rs:1314) | **Glaring Spotlight** (tail) shares the exact hexproof clause (no ward) → real class ≥2. Reach: **2** |
| **B4** | **Grant-abilities-from-exiled** static | Locus (1); Koh static (1, blocked) | likely none for Locus; for Koh: triggered-ability grant variant + "last chosen card" referent | `ContinuousModification::GrantAllActivatedAbilitiesOf{source}` (ability.rs:16738) + `TargetFilter::ExiledBySource` (filter.rs:1784) — Myr Welder path | Narrow class. Locus reach **1**; Koh needs more (see §3/§7) |
| **B5** | **Play/plot-from-top-of-library** | Fblthp (2 lines) | plot play-mode + "top card has [keyword]" grant static | `StaticMode::TopOfLibraryCastPermission{alt_cost}` (statics.rs:1046); `CardPlayMode` (ability.rs:734) | Future Sight / Bolas's Citadel / Realmwalker family already modeled; plot-from-top is the new wrinkle. Reach: **1** (highest effort) |

**Re-home (NOT static work):** Sandswirl L2 trigger `Effect:can't` (temporary CantAttack restriction — belongs to the restriction-effect path The Second Doctor added, `effects/add_restriction.rs`, commit 53289c31d); Koh L3 `Effect:choose` (interactive activated ability — belongs to choose-effect/`add-interactive-effect` work). Both are required for their cards to flip but are outside the static-ability mandate.

## (3) Per-group: closest analog traced + file-by-file changes

### B1 — Agatha (dynamic activated-ability cost reduction)

**Closest analog, traced end-to-end:** Training Grounds, "Activated abilities of creatures you control cost {2} less to activate."
- Parser: `dispatch.rs:2502-2535` — `tag("activated abilities of ")` → `take_until(" cost ")` (subject) → `delimited("{", parse_number, "}")` → `alt(less/more)`; emits `ReduceAbilityCost{mode, keyword:"activated", amount, minimum_mana: parse_activated_cost_reduction_minimum_mana(...), dynamic_count: None}` with `affected = parse_type_phrase(subject)`.
- Runtime: on activation, casting.rs:13734 calls `apply_static_activated_ability_cost_reduction` (13737); it scans `battlefield_active_statics`, matches `keyword=="activated"`, checks `def.affected` against the ability's source object (13768), applies `reduce_generic_in_cost_with_minimum_mana` (13784).

**Why Agatha falls through today:** the amount is `{X}` and `parse_number` rejects "x", so the whole branch returns `None` → swallowed (matches observed: zero `static_abilities`).

**Changes:**
- **Parser — `parser/oracle_static/dispatch.rs:2502` branch (extend in place, do not add a sibling):** replace the inner `delimited("{", parse_number, "}")` amount-parse with a small combinator returning `(amount:u32, dynamic_count:Option<QuantityRef>)`:
  - `{N}` → `(N, None)` (unchanged behavior — discriminating revert target).
  - `{X}` followed (after `" less to activate"`) by `", where x is ~'s power"` → `(1, Some(QuantityRef::Power{scope:ObjectScope::Source}))`. Parse the "where X is …" tail with `alt((value(Power{Source}, tag("'s power")), value(Toughness{Source}, tag("'s toughness")) …))` keyed on the `~` self-reference (`SELF_REF_TYPE_PHRASES`) so it covers "where X is [source]'s {power|toughness|…}" as a class, not just Agatha. Keep `minimum_mana` via the existing `parse_activated_cost_reduction_minimum_mana(tp.lower)` (cost_mod.rs:34) — already parses "…less than one mana" → `Some(1)`.
- **Runtime — `game/casting.rs:13755`:** the destructure drops `dynamic_count` via `..`. Add `dynamic_count` to the pattern and, when `Some`, multiply `amount` by `resolve_quantity(QuantityExpr::Ref{qty}, static_source.controller, static_source.id)` before reducing — **reuse the exact pattern already in `keywords.rs:896-908`** (resolve against the static's source id `static_source.id` so "X = Agatha's power" reads Agatha's post-layer power, CR 208.1 + CR 113.7). `.max(0) as u32`, then `reduce_generic_in_cost_with_minimum_mana(cost, amount*mult, minimum_mana.unwrap_or(0))`. CR 601.2f / CR 118.7 (verified). Generic-only reduction already matches the Doc Aurlock/Agatha ruling ("can't reduce colored mana").
- **Targeting / multiplayer filter / frontend / AI:** none. `ReduceAbilityCost` is `is_data_carrying_static` (coverage.rs:60) already — no coverage change. (Note ai_support/mod.rs:898 already accounts for engine-effective activation cost.)
- **Tests:** parser test (Agatha {X}+power+floor → `ReduceAbilityCost{amount:1, dynamic_count:Some(Power{Source}), minimum_mana:Some(1)}`, affected = creatures you control); a fixed-`{2}` Training-Grounds regression test in the same arm (proves the new combinator didn't break the numeric path); runtime test casting an activated ability of a controlled creature with Agatha at power 3 and a `{4}`-generic ability → reduced to `{1}` (floor), and power 0 → no reduction.

### B2 — Doc Aurlock L2 + Inquisitive Glimmer L2 (keyword/special-action cost reduction)

**Closest analog:** the cast-cost `ModifyCost` path that already handles each card's L1 (`static_helpers.rs:620`, `try_parse_cost_modification` at dispatch.rs:2553). It reduces generic mana of a *spell* cost; plot/unlock are *special actions* (CR 702.170 / CR 709.5e — both verified), so they need a parallel hook. The cost_mod.rs:18 comment explicitly excluded plot from `ReduceAbilityCost` because plot's payment doesn't carry an `AbilityTag` and would never fire.

**Rules note (why not just tag plot as an activated ability):** Doc Aurlock's own ruling distinguishes plot from activated abilities; routing plot through generic `ReduceAbilityCost{keyword:"activated"}` would make Zirda/Training-Grounds-style "activated abilities cost less" wrongly reduce plot costs. So a dedicated action axis is the rules-correct choice.

**Changes:**
- **Types — `types/mana.rs:342` `SpecialAction`:** add `Plot` variant (CR 702.170a). Keep `UnlockDoor` (exists).
- **Types — `types/statics.rs` `StaticMode`:** add `ReduceActionCost { action: SpecialAction, mode: CostModifyMode, amount: ManaCost }` (anchor CR 118.7 cost reduction + the action's CR). `#[serde(default)]` not needed (new variant). Add Display/FromStr arms alongside the existing cost-mode siblings (statics.rs:2003).
- **Parser — `parser/oracle_static/cost_mod.rs` (new combinator `parse_action_cost_reduction`, dispatched from dispatch.rs near the other cost branches):**
  - "plotting cards from your hand cost {N} less" → `ReduceActionCost{action:Plot, mode:Reduce, amount:N generic}`.
  - "unlock costs you pay cost {N} less" → `ReduceActionCost{action:UnlockDoor, mode:Reduce, amount:N}`.
  - Compose as one combinator: `alt((value(Plot, tag("plotting cards from your hand")), value(UnlockDoor, tag("unlock costs you pay"))))` → `take_until` `" cost "` → `delimited("{",parse_number,"}")` → `alt(less/more)`. No verbatim full-string match; each axis is its own `tag`/`alt`.
- **Runtime hooks:**
  - **Plot:** the plot cost is the `Keyword::Plot(ManaCost)` paid through the synthesized hand activation (casting.rs ~19990-20032, `CastingVariant::Plot` at casting.rs:3481/4076). Reduce the generic component of that plot mana cost when computing/charging it, scanning battlefield `ReduceActionCost{action:Plot}` statics controlled by the plotting player.
  - **Unlock:** room.rs computes door options (room.rs:39-66); the unlock mana cost is the door's mana cost (CR 709.5e). Reduce generic there before payment.
  - Single authority: add one helper `apply_special_action_cost_reduction(state, player, action, cost) -> ManaCost` (mirrors keywords.rs:865 `apply_ability_cost_reduction`) and call it from both sites — do not inline reduction logic at each call site (CLAUDE.md "single authority for ability costs").
- **Coverage — `game/coverage.rs:43` `is_data_carrying_static`:** add `| StaticMode::ReduceActionCost { .. }` so the card counts as supported (regression-critical: omitting this leaves the card "unsupported" even though it parses).
- **swallow_check.rs:** add a recognizer so the parsed `ReduceActionCost` line is not counted as a swallowed clause.
- **Targeting/MP filter/frontend/AI:** none (cost reduction is server-side; engine-effective cost already surfaces).
- **Tests:** parser tests for both phrasings → correct `action`/`amount`; runtime test: plot a card with Doc Aurlock out, assert generic plot cost drops by 2 (floor 0); unlock a Room with Inquisitive out, assert door generic cost drops by 1; revert-probe: drop the `Plot` SpecialAction arm → Doc Aurlock L2 reverts to Unimplemented.

### B3 — Nowhere to Run (IgnoreHexproof scoped + ward-suppression)

**Closest analog:** `IgnoreHexproof` transient player grant in `targeting.rs:2082-2168` (CR 702.11 hexproof verified; 702.18 = shroud, never bypassed). Today it's created as an ephemeral per-player static during a bypass effect; there is no parser path that emits a *battlefield* `IgnoreHexproof` static scoped by an `affected` filter, and the targeting check consults transient grants, not battlefield statics' `affected`.

**Changes (two sub-features):**
- **Sub-feature 3a — scoped IgnoreHexproof:**
  - **Parser — `oracle_static` (new branch):** "creatures your opponents control [with hexproof] can be the targets of spells and abilities [you control] as though they didn't have hexproof" → `StaticDefinition{ mode:IgnoreHexproof, affected: <opponents' creatures> }`. Combinator: `parse_type_phrase` for the subject + `tag(" as though they didn't have hexproof")`. This same branch parses **Glaring Spotlight** (tail) — build the subject via `parse_type_phrase` so "creatures your opponents control with hexproof" and "creatures your opponents control" both land.
  - **Runtime — `game/targeting.rs`:** at the hexproof legality check (targeting.rs:1682, CR 702.11e), additionally consult `battlefield_active_statics` for `IgnoreHexproof` whose `affected` matches the would-be target AND whose controller is the targeting player (CR 702.11e — only *your* spells/abilities ignore it). Extend, don't duplicate, the existing `transient_grants_static_mode_to_player` check (static_abilities.rs:1031-1036).
- **Sub-feature 3b — ward-suppression (genuinely new):**
  - **Types — `types/statics.rs:149` `SuppressedTriggerEvent`:** add `BecomesTargeted` (CR 702.21a — ward triggers when the permanent becomes targeted). Add Display arm. Reuse `StaticMode::SuppressTriggers{source_filter, events}` (statics.rs:1314) — no new StaticMode.
  - **Parser:** the second sentence "Ward abilities of those creatures don't trigger" → `SuppressTriggers{ source_filter: <same opponents' creatures>, events:[BecomesTargeted] }`. ("those creatures" anaphors the first sentence's filter — reuse the parsed subject.) Emit both statics from the one Oracle line (the parser already returns `Vec<StaticDefinition>` in cost_mod.rs:70-81, same pattern).
  - **Runtime — `game/triggers.rs:1669`:** at the ward-trigger creation point (`Keyword::Ward(cost)` → counter on becomes-targeted), skip creating the ward trigger when an active `SuppressTriggers{events∋BecomesTargeted}` matches the targeted creature. This mirrors how ETB/Dies suppression already gates trigger registration.
  - **Coverage:** `SuppressTriggers` is already supported; confirm `is_data_carrying_static` covers it (it does — statics.rs:1904 path). `IgnoreHexproof` is nullary marker — verify it's registered as supported in coverage.rs (it's in the registry at static_abilities.rs:113); add to `is_data_carrying_static` if the filter-scoped form needs it.
- **MP filter/frontend/AI:** none for hexproof/ward (targeting legality is engine-side).
- **Tests:** parser test → two statics (IgnoreHexproof + SuppressTriggers[BecomesTargeted]) with the opponent-creature filter; targeting test: opponent's hexproof creature is a legal target for the static's controller but NOT for the opponent themselves; ward test: a Ward {2} creature under the static does not generate a ward counter-trigger when targeted by the controller; revert-probe: remove the SuppressTriggers emission → ward fires again.

### B4 — Locus of Enlightenment (grant abilities from exiled/craft pool); Koh static (blocked)

**Closest analog (traced):** Myr Welder — `layers.rs:8973-9040` test `grants_all_activated_abilities_of_cards_exiled_with_it`: a `StaticDefinition::continuous().affected(SelfRef).modifications([GrantAllActivatedAbilitiesOf{ source: ExiledBySource }])`; `evaluate_layers` expands it in layer 6 (CR 613.1f) by pulling activated abilities off objects matching `ExiledBySource` (filter.rs:1784, resolves via `cards_exiled_with_source_this_turn` / `exile_links`).

**Locus changes:**
- **Parser — `oracle_static/keyword_grant.rs` (the "~ has all activated abilities of …" home, keyword_grant.rs:736):** add an arm for "~ has each activated ability of the exiled cards used to craft it" → `GrantAllActivatedAbilitiesOf{ source: ExiledBySource }`, `affected: SelfRef`. "exiled cards used to craft it" should map to `ExiledBySource` (CR 702.167 craft exiles cards with the crafted permanent).
- **Verify (open question, §7):** craft must create persistent `exile_links{kind:TrackedBySource}` so `ExiledBySource` (which the layer path uses) resolves to the craft pile — `ExiledBySource` resolves from `cards_exiled_with_source_this_turn` for the *this-turn* reading (filter.rs:1795); the craft pile is persistent, so confirm the layer-side expansion reads `exile_links` (the Myr Welder test pushes `ExiledBySource` via `exile_links`, so this is likely correct).
- **Extra constraint — "only once each turn":** the granted abilities get a per-turn activation cap. Reuse the activation-limit machinery (`ModifyActivationLimit`/`OnlyOnceEachTurn`, statics.rs:914). This is the one piece beyond Myr Welder; if it cannot be cleanly attached to dynamically-granted abilities, ship the grant and leave the once-per-turn cap as an annotated runtime TODO (the card still flips — no Unimplemented).
- **Coverage/tests:** parser test → the `GrantAllActivatedAbilitiesOf{ExiledBySource}` modification; layer test analogous to the Myr Welder test but driven through `parse_oracle_text` for Locus; revert-probe: change the source filter and assert the donated ability disappears.

**Koh static (L4):** "~ has all activated AND triggered abilities of the last chosen card." This needs (a) a triggered-ability grant (current `GrantAllActivatedAbilitiesOf` is activated-only — would need a sibling or a parameter `{ activated:bool, triggered:bool }`), and (b) a **"last chosen card" referent** populated by L3's `Pay 1 life: Choose a creature card exiled with ~` (the `Effect:choose` gap, non-static). No "last chosen card" object referent exists (grep found only `IsChosenCardType` for the unrelated name/type-pick class). **Koh's static is therefore gated on the choose-effect work and a new triggered-grant capability — see §7.**

### B5 — Fblthp (play/plot from top of library)

**Closest analog:** `StaticMode::TopOfLibraryCastPermission{ play_mode, frequency, alt_cost }` (statics.rs:1046) — Future Sight/Bolas's Citadel/Realmwalker. It grants casting/playing the top card. Fblthp's twist is **plot**, not cast/play.

**Changes (two static lines):**
- **L3 "The top card of your library has plot. The plot cost is equal to its mana cost":** grant the plot keyword to the top library card with plot cost = its mana cost. This is a "top card of library has [keyword]" continuous grant. There is `RevealTopOfLibrary`/`MayLookAtTopOfLibrary` (statics.rs:998/1439) but no "top card has keyword" grant. New static (or a `GrantKeywordToTopOfLibrary{ keyword }`); plot-cost = mana cost is the canonical plot-from-anywhere reading.
- **L4 "You may plot nonland cards from the top of your library":** permission to take the plot special action sourced from the top of library. Either extend `CardPlayMode` (ability.rs:734) with a `Plot` variant on `TopOfLibraryCastPermission`, or a dedicated permission. (Note L4 is tagged `effect_structure` but is semantically a static permission — keep it here, not re-homed.)
- **Effort:** highest of the five — touches the casting pipeline's plot path + a new top-of-library keyword grant + the top-of-library permission's interaction with the plot special action. Recommend **parser-complete typed statics now (flips Fblthp), runtime wiring as a tightly-scoped follow-up** — this matches the codebase's documented "parser-complete structured gap; runtime hook deferred" convention (e.g. `AlternativeKeywordCost`, statics.rs:847). Both gapped lines must emit typed statics (not Unimplemented) for Fblthp to flip.

## (4) Discriminating tests + revert-probe per building block

Each block's parser test asserts the *typed structure* (not the card), plus a within-arm control proving the change is discriminating:

- **B1:** positive: Agatha `{X}…power…floor` → `ReduceAbilityCost{amount:1,dynamic_count:Some(Power{Source}),minimum_mana:Some(1)}`. Control: fixed `{2}` (Training Grounds) still → `dynamic_count:None,amount:2`. Runtime discriminator: same ability, Agatha power 3 vs power 0 → reduced vs not. Revert-probe: revert casting.rs:13755 to `..` (drop dynamic_count) → Agatha test asserts reduction == 0, proving the runtime line is load-bearing.
- **B2:** positive: both phrasings → correct `SpecialAction`. Control: a normal "creatures you control cost {1} less to cast" must still route to `ModifyCost`, NOT `ReduceActionCost`. Revert-probe: remove the `Plot` arm → Doc Aurlock L2 reverts to Unimplemented; remove the coverage.rs arm → card flips back to unsupported (proves the coverage wiring matters).
- **B3:** positive: two statics emitted; control: Glaring Spotlight (no ward) emits ONLY IgnoreHexproof. Targeting discriminator: legal for controller, illegal for the hexproof creature's controller (proves scope, not blanket). Ward discriminator: Ward {2} creature targeted by controller → no counter-trigger; same creature with the static removed → trigger fires (revert-probe).
- **B4:** positive: Locus parse → `GrantAllActivatedAbilitiesOf{ExiledBySource}`. Layer discriminator: a card in Locus's craft pile donates its activated ability; a card exiled by a *different* source does NOT (proves filter scoping). Revert-probe: swap the source filter → donation disappears.
- **B5:** parser tests assert the two typed statics; revert-probe: remove each arm → the respective line reverts to Unimplemented.

All tests live in `parser/oracle_static/tests.rs`, `game/layers.rs`, `game/targeting.rs`, `game/triggers.rs` inline modules (existing homes). Use the `card-test` harness recipe for runtime cast/activation tests.

## (5) Sequencing + PR-split

Independent building blocks ship as separate squash-merge PRs. Shared collision files: `types/statics.rs`, `types/ability.rs`, `parser/oracle_static/dispatch.rs`, `game/coverage.rs` — so PRs touching them must land **sequentially** (orchestration standard #3), rebasing between, not concurrently.

1. **PR-A (B1, Agatha)** — smallest, highest-confidence, no new types. dispatch.rs + casting.rs + tests. Land first.
2. **PR-B (B2, Doc Aurlock + Inquisitive)** — new `ReduceActionCost` + `SpecialAction::Plot`. types + cost_mod.rs + casting.rs/room.rs + coverage.rs + swallow_check.rs.
3. **PR-C (B3, Nowhere to Run + Glaring Spotlight)** — IgnoreHexproof scoping + SuppressTriggers[BecomesTargeted]. types + oracle_static + targeting.rs + triggers.rs.
4. **PR-D (B4, Locus)** — keyword_grant.rs + layer verification (+ once-per-turn or annotated TODO).
5. **PR-E (B5, Fblthp)** — largest; parser-complete + deferred runtime. Land last (or split parser-flip from runtime).

Koh and Sandswirl are **NOT single-PR static flips** — see §7. Recommend a 6th coordinated PR (or hand-off) bundling each card's static line WITH its non-static gap so the card actually flips.

PR-A/B/C are the clean wins (4 of 8 cards: Agatha, Doc Aurlock, Inquisitive, Nowhere to Run flip outright). PR-D flips Locus. PR-E flips Fblthp (parser-complete). That is **6 of 8** via static work; Sandswirl + Koh need the flagged effect work to flip.

## (6) Verification + regression plan

Static/continuous-effect changes are the documented parser-coverage-regression hazard (swallowed clauses on *other* cards, caught only by CI's card-data coverage job). Per PR:
- `cargo fmt --all` (always direct).
- Tilt-first: `./scripts/tilt-wait.sh --timeout 240 clippy test-engine card-data` (do NOT run cargo directly — target-lock contention). Read `tilt logs <resource>` only after `updateStatus==error && currentBuild.spanID==none`.
- **Full-corpus coverage delta:** run `cargo coverage` (one-shot binary, direct) before+after; assert the target cards move to supported and **total Unimplemented count does not rise** (catches swallowed-clause regressions). Diff the per-card `static_abilities`/`abilities` for a sample of unrelated cost-reduction and hexproof/ward cards to prove no collateral re-parse.
- **Guard test (non-target not mis-parsed):** e.g. for B2, assert a plain "spells you cast cost {1} less" card still parses to `ModifyCost` (not `ReduceActionCost`); for B3, assert a card that *grants* hexproof (Shalai) is unaffected.
- `cargo semantic-audit` on the touched cards. Snapshot updates in `crates/engine/tests/oracle_parser.rs` where parsed abilities changed.
- `validate-cr-annotations` skill on every new `// CR` line (all numbers used here grep-verified above: 702.170 plot, 702.21 ward, 702.11 hexproof, 709.5e unlock, 601.2f/118.7 cost, 702.167 craft, 613.1f layer-6).

## (7) Risks & open questions

- **Sandswirl — re-home + heavy static.** Two gaps: L2 trigger effect (`Effect:can't` — temporary CantAttack restriction) is **effect work**, belongs to the restriction-effect path added by commit 53289c31d (`effects/add_restriction.rs`); it likely just needs a "can't attack you or planeswalkers you control" parse arm. L3 static "each opponent who attacked you… can't cast spells" is a static CantBeCast, but `ProhibitionScope` (statics.rs:27) has only 4 flat variants — there is **no conditional player scope**. Adding `OpponentsWhoAttackedYouThisTurn` would be a card-specific sibling (violates "parameterize, don't proliferate"); the rules-correct design is a typed per-player predicate scope reading `state.attacked_defenders_this_turn` (restrictions.rs:265, which tracks attacker→defenders). **Recommend:** treat Sandswirl as a separate, larger design item (new conditional ProhibitionScope + verify "attacked a planeswalker you control" records the controller as a defender). It will NOT flip from a quick static arm.
- **Koh — re-home + missing infra.** Its static (L4) needs (a) a triggered-ability grant (today only `GrantAllActivatedAbilitiesOf` = activated-only) and (b) a "last chosen card" referent that does not exist, populated by the non-static L3 `Effect:choose` activated ability. Koh cannot flip via static work alone; recommend bundling with interactive-choose work (`add-interactive-effect`) or deferring.
- **Fblthp — scope.** Full runtime for "top card has plot" + "plot from top" is the heaviest item; recommend parser-complete-now / runtime-deferred per codebase convention, but confirm the orchestrator accepts a parser-flip (no Unimplemented) without full runtime for this card.
- **B2 plot vs activated-ability conflation.** Must NOT tag plot as a generic activated ability (would make Zirda/Training-Grounds wrongly reduce plot costs, contra Doc Aurlock's ruling). The dedicated `SpecialAction::Plot` axis avoids this — this is a correctness constraint, not a style choice.
- **B4 craft-pool resolution.** `ExiledBySource` resolves from `cards_exiled_with_source_this_turn` for the this-turn reading (filter.rs:1795); Locus's craft pile is persistent. Must confirm the layer-side `GrantAllActivatedAbilitiesOf` expansion reads `exile_links` (persistent) — the Myr Welder test pushes via `exile_links`, suggesting yes, but verify craft creates `ExileLink{kind:TrackedBySource}`. Also the "only once each turn" cap on dynamically-granted abilities is unproven infra.
- **Glaring Spotlight has an activated ability** ("{3}, Sacrifice…") — its `static_structure` gap is only the hexproof line, so B3 flips that line, but confirm the activated ability already parses (it's not in our 8, just a class-reach beneficiary).
- **None of the 8 is mis-clustered as static when it's purely non-static** — but three (Sandswirl, Koh, Fblthp-L4) have a non-static *co-gap* that the cluster tag obscures; flagged above.

Key file anchors: `parser/oracle_static/dispatch.rs:2502` (B1 parser), `game/casting.rs:13737-13789` (B1 runtime), `game/keywords.rs:865-918` (dynamic-count pattern to reuse), `types/statics.rs:860/893/990/1046/1314` + `types/mana.rs:342` + `types/ability.rs:734/3858/4092/16738` (type surface), `game/targeting.rs:1682/2082-2168` (B3 targeting), `game/triggers.rs:1669` (B3 ward), `game/layers.rs:8973-9040` + `game/filter.rs:1784-1795` (B4), `game/restrictions.rs:258-271` (Sandswirl tracking), `game/coverage.rs:43` (coverage gate).

---
# BINDING AMENDMENTS (round-1 adversarial review — these OVERRIDE the plan above on any conflict)

**SCOPE FOR THE FIRST EXECUTOR PASS: implement B2 + B1 ONLY (cost-reduction building blocks).** B3/B4/B5 follow in later passes within this same line. Re-homes (Sandswirl `Effect:can't`, Koh `Effect:choose`) are OUT of static scope — do NOT implement; report them as blockers preventing those cards from fully flipping.

1. **[LOW→DO FIRST] B2 parser verb form.** Doc Aurlock's text is "Plotting cards from your hand **COSTS** {2} less" (singular verb); the plan's `take_until(" cost ")` fails on "costs ". Terminate on `alt((tag(" costs "), tag(" cost ")))` (or split subject/verb) so BOTH Doc Aurlock ("costs") and Inquisitive Glimmer ("cost") parse.

2. **[LOW] B1 self-name `~` normalization.** Agatha's text is "where X is Agatha's power". Either (a) assert with a code reference that self-name→`~` normalization precedes `oracle_static` dispatch, or (b) make the where-clause combinator also accept `SELF_REF_TYPE_PHRASES`. Add a parser test proving "where x is ~'s power" → `QuantityRef::Power{scope:Source}`. The revert-probe: drop the dynamic `{X}` arm → Agatha reverts to no static_abilities.

3. **[LOW] AI anchor correction.** `crates/engine/src/game/ai_support/` does NOT exist. Drop the "ai_support/mod.rs:898" anchor. Confirm whether any **phase-ai** crate activation-cost scorer needs the dynamic/`ReduceActionCost` reduction reflected (per add-engine-effect AI step); if not, state "AI: none" with the correct reasoning.

4. **Keep for B2/B1 (reviewer-confirmed correct):** EXTEND existing cost machinery, no parallel path; B1 fixes the dropped `dynamic_count` in casting.rs:13755 + the `{X}` parse_number rejection in dispatch.rs:2502; B2 adds `SpecialAction::Plot` + `StaticMode::ReduceActionCost{action,mode,amount}` with a SINGLE authority `apply_special_action_cost_reduction` called from both plot and unlock sites; update `is_data_carrying_static` (coverage.rs:43) AND `swallow_check.rs` so the cards count supported and the line isn't a swallowed clause; the plot-via-ReduceAbilityCost trap is real (cost_mod.rs:18) — keep plot as a dedicated action axis.

5. **DEFERRED to later passes (B3/B4/B5) — record these fixes now so they aren't lost:**
   - **[HIGH] B3 hexproof scoping is multiplayer-WRONG.** Nowhere to Run = "Creatures your opponents control can be the targets of spells and abilities as though they didn't have hexproof" — NO "you control". Scope the `IgnoreHexproof` bypass by the static's `affected` filter ONLY (opponents-of-static-controller creatures), apply for ANY targeting player (drop "targeting player == static controller"). In game/targeting.rs near the hexproof check, bypass when the would-be target matches an active `IgnoreHexproof` static's `affected`, independent of source_controller.
   - **[MED] B3 CR:** cite **CR 702.11b** (hexproof blocks opponents' spells/abilities); do NOT use the "CR 702.11e — only your" gloss (702.11e is hexproof-from-quality). Grep-verify.
   - **[MED] B3 class claim:** DROP the Glaring Spotlight reach claim (it's "lose hexproof" = layer-6 removal, a different block; its card-data text is empty). Re-justify B3 on the ward-suppression building block (`SuppressTriggers[BecomesTargeted]`) + reusable affected-filter scoping; B3 is honestly ~1 card.

6. **Per-card flip honesty:** after B1+B2, only Agatha, Doc Aurlock, Inquisitive Glimmer fully flip. Sandswirl/Koh need re-homed non-static work; Fblthp needs B5 (play/plot-from-top). Report the exact supported-status delta measured via full-corpus coverage.

7. **PR-split:** B2+B1 ship as ONE squash-merge PR (both cost-reduction). B3/B4/B5 as separate PRs. Each: rebase onto current upstream/main + run FULL CI-equiv (fmt/clippy --workspace/test -p engine) before push (drift-gate hazard).
