# S25 — Granted-ability self-reference dual binding (The Dominion Bracelet + class)

Planner: s25-planner-r2. Worktree `/home/lgray/vibe-coding/s25-impl-wt` (branch `feat/std-s25-completion`, HEAD `330a1f18b`).
Mandated skills followed and named per step below: **`add-engine-variant`** (the crux gate), **`add-engine-effect`** (lifecycle), **`oracle-parser`** (nom mandate for the parser change), **`card-test`** (discriminating revert-to-red runtime tests).

---

## 0. Design crux (dual binding)

One runtime `source_id` currently serves two referents inside a **granted** activated/triggered ability, and BOTH have correct consumers:

- **Granter-referential** — the granted ability's reference to the **granting object's own printed name** ("Exile **The Dominion Bracelet**", "Sacrifice **Deconstruction Hammer**", "Unattach **Leonin Bola**", "Return **Razor Boomerang** to its owner's hand"). Governed by **CR 201.5a** (verified `docs/MagicCompRules.txt:1324`): *"If an ability's effect grants another ability to an object, and that second ability refers to that first ability's source by name, the name refers only to the specific object which is that first ability's source."* → must resolve to the **granting object** (equipment/aura). Today it wrongly resolves to the host creature.
- **Host-referential** — the same granted ability's reference to its **bearer**: "this creature", "this permanent", and the {X}-less reduction "where X is **this creature's** power". Must keep resolving to `source_id` = the equipped/enchanted creature (host). Today this is correct.

**Root cause of the collapse (measured):** `normalize_card_name_refs` (`crates/engine/src/parser/oracle_util.rs:1605`) rewrites the card **name** → `~` (lines 1640-1650) AND the generic self-phrases in `SELF_REF_TYPE_PHRASES` ("this creature", …) → `~` (lines 1738-1739). Both collapse to the same `~` token, which every downstream self-ref parser maps to `TargetFilter::SelfRef`. Confirmed on The Dominion Bracelet: its granted definition's cost is `Exile{ filter: SelfRef }` (from the card name) and its reduction description is `"…where X is ~'s power"` (from "this creature") — **byte-identical `~`, two different referents.** At runtime `TargetFilter::SelfRef` resolves to `ability.source_id` = host (`targeting.rs:640-650`, `filter.rs:1506`), so `Exile{SelfRef}` exiles the creature, not the Bracelet — a CR 201.5a / CR 701.13a violation.

**Why a blanket "SelfRef-in-granted → granter" rewrite is wrong (proven, not asserted):** 73 host-referential granted abilities (Slivers "Sacrifice **this permanent**", Auras "regenerate **~**"=enchanted creature, etc.) carry `SelfRef` in cost/effect that MUST stay on the host. E.g. Acidic Sliver grants `{2}, Sacrifice this permanent: this permanent deals 2 damage` to every Sliver; activated on another Sliver, the SelfRef must be that Sliver (host), never the granting Sliver. A resolution-time-only or position-based heuristic (cost-SelfRef→granter) is therefore **unsound** — the cost-SelfRef of Acidic Sliver (host) and of Deconstruction Hammer (granter) are the identical AST. **The only sound disambiguator is the source phrase (card-name vs. "this X"), a parse-time fact that is destroyed at normalization.** So the distinction MUST be re-introduced at/before normalization and carried as a distinct AST node.

### Chosen mechanism (parse-time distinct ref + grant-time concretization)

Two coordinated pieces:

1. **Parse-time (bounded emission):** the card-name self-reference *inside a granted quoted body* becomes a new typed leaf `TargetFilter::GrantingObject`; the generic "this creature/permanent" self-phrase stays `TargetFilter::SelfRef`. Emission is **bounded to granted bodies** (≈19 cards) via a masking pre-pass, so no corpus-wide reclassification.
2. **Grant-time concretization (`layers.rs`):** when a `GrantAbility`/`GrantTrigger` static clones the granted definition onto the host, rewrite every `TargetFilter::GrantingObject` → `TargetFilter::SpecificObject { id: effect.source_id }` (the live granting-object id, already in scope at `layers.rs:4326/4547`). Runtime then sees only the fully-supported `SpecificObject` — **no new runtime resolution logic, no touch of the frozen `filter.rs::SelfRef` arm.** `Power{Source}`/`SelfPower` and the host `SelfRef`s are untouched and keep reading `source_id` (host).

This is the crux's "granted-by context with per-consumer/per-referent resolution", realized as: **the referent is decided at parse time (GrantingObject vs SelfRef); the granting-object id is threaded at grant time; resolution is per-referent for free** because GrantingObject→SpecificObject{granter} and SelfRef→source are two independent AST channels, and the power reduction is a third (QuantityRef) channel that never touches TargetFilter.

**`/add-engine-variant` verdict on `TargetFilter::GrantingObject`:**
- **Stage 1 (existence, 5-grep):** `DOES_NOT_EXIST`. Enumerated the full `TargetFilter` enum (`types/ability.rs`): `SelfRef`=the source object; `SourceOrPaired`=source+partner; `AttachedTo`=the host the equipment is attached to (the *opposite* referent); `SpecificObject{id}`=a concrete id but no parse-time symbolic "granter"; `CostPaidObject`=the object paid as a cost. None denote "the object that granted this ability." No inverse/parameter form exists.
- **Stage 2 (parameterization):** `EXTEND_OK`. No sibling-cluster smell — the source-relative object leaves (`SelfRef`/`SourceOrPaired`/`AttachedTo`/`ParentTarget`) are distinct referents, not an X/OpponentX/TargetX or comparator/scope axis. `GrantingObject` is *not* a parameterization of `SelfRef`: it names a different object (the grantOR, not the source). Adding it does not create a cluster.
- **Stage 3 (categorical boundary):** `WITHIN_SECTION`. `TargetFilter` is the canonical cross-section object-reference layer; the referent is a single coherent rule, CR 201.5a. **APPROVED: `TargetFilter::GrantingObject`** (CR-annotate 201.5a). Type-only-until-rewritten is acceptable: it is always concretized to `SpecificObject` at grant time, and its defensive runtime arms mirror `SelfRef`→source (fail-safe, never worse than today).

---

## 1. Grant path & resolution trace (file:line evidence)

- **Static grant clone.** `ContinuousModification::GrantAbility { definition }` is applied per-recipient at `game/layers.rs:4967-4970`: `Arc::make_mut(&mut obj.abilities).push(*definition.clone())` with **no self-object remap**. The enclosing loop `for id in affected_ids` (`layers.rs:4537`) binds `id`/`obj` = recipient (host creature) and has `effect.source_id` = **the granting equipment/aura** in scope (`FilterContext::from_source(state, effect.source_id)` at `4326`; used throughout the apply body, e.g. `4547`). `GrantTrigger` is the sibling at `layers.rs:4984-4991`. **This is the single rewrite site** for both.
- **Activation source_id.** A granted ability is read back from `obj.abilities` at `game/casting.rs:206` (`activation_ability_definition`); the activator is the host, so the resulting `ResolvedAbility.source_id` (`types/ability.rs:17881`) = host. Confirmed the granted definition is what's activated (so post-clone rewrite is visible at activation).
- **SelfRef resolution (host today).** `resolved_targets` short-circuits `SelfRef` → `vec![TargetRef::Object(ability.source_id)]` at `game/targeting.rs:640-650` (before any `ability.targets` fallback); `filter.rs:1506` `SelfRef => object_id == source_id`. `SpecificObject{id}` resolves against `id` through the normal path — fully supported, no special casing needed.
- **Power reduction (host today, unchanged).** "where X is this creature's power" → `~'s power` → `QuantityRef::Power { scope: ObjectScope::Source }` via `parse_self_power_ref`/`parse_self_possessive` (`oracle_nom/quantity.rs`; `~'s` is an explicit tag). This is a `QuantityExpr::Ref` channel that resolves against `source_id` (host) and **never becomes a `TargetFilter`** — it is structurally immune to the SelfRef→GrantingObject change. This is the load-bearing proof that the dual binding is free: the two referents live in different AST types.

## 2. Measured class size (build-for-the-class)

Corpus scan of `data/card-data.json` (35,397 cards) for `static_abilities → GrantAbility/GrantTrigger` whose granted body's **quoted text names the granting card itself** in a verb-object cost/effect position (scanner: `scratchpad/broad.py` + `classify.py`; SelfRef-AST cross-check: `scan_class.py`).

**Granter-referential — TargetFilter channel, FIXED by this change (~19 cards):**
Blazing Torch, Blinding Powder, Citizen's Crowbar, **Deconstruction Hammer**, Fishing Pole, Hankyu, Heartseeker, Heliod's Punishment (Aura), Leonin Bola, Ninja's Kunai, Razor Boomerang, Shuriken, Spare Dagger*, Sunfire Torch*, Surestrike Trident, **The Dominion Bracelet**, Toralf's Hammer, Trickster's Talisman*, Trusty Boomerang.
(* = granted **trigger** naming the granter in a "you may sacrifice X" action — requires the same rewrite in the `GrantTrigger` path, `layers.rs:4984`.)
The self-naming appears as: `Sacrifice/Exile/Unattach/Tap/RemoveCounter <name>` (cost) → `SelfRef`/currently-`SelfRef`-or-unparsed; `Destroy/Return/GainControl/PutCounter-on <name>` (effect target) → `SelfRef`. All uniformly become `GrantingObject` and are concretized to the granter.

**Host-referential — MUST NOT CHANGE (73 cards):** all Slivers ("Sacrifice this permanent"), pump/regenerate Auras ("~ gets +N", "regenerate ~"=enchanted creature), Food Fight (`Sacrifice this artifact` cost = host; "permanents named Food Fight" is a `NameCount`, not a self-ref — my first scan misflagged it, corrected here), Meandered Towershell (self-trigger where name = source = host). These carry `SelfRef` from "this X" / possessive-`~` and are **not** produced inside a card-name-in-quote position, so the masking never marks them → they stay `SelfRef` → host. Untouched.

**Adjacent classes, SAME root cause, DIFFERENT AST channel — explicitly out of scope, note as follow-ups:**
- *QuantityRef channel:* Archery Training — "where X is the number of arrow counters on Archery Training" → `QuantityRef::CountersOn{scope}` reading the aura's counters; needs the same granter-id threaded into `ObjectScope` resolution, not `TargetFilter`.
- *Damage-source channel:* Blazing Torch / Ninja's Kunai / Razor Boomerang / Toralf's Hammer / Shuriken / Surestrike Trident — "[equipment]/it deals damage" attributes the damage **source** to `source_id` (host), not the equipment. The implicit effect source is not a `TargetFilter`. The confirmed self-exile/sacrifice **cost** bug IS fixed for all of these; only the deeper damage-source attribution nuance remains. Both follow-ups reuse the granter-id concept.

The chosen mechanism (parse-time granter marking + grant-time granter-id) is the general building block; extending it to `ObjectScope` and effect-source is additive.

## 3. Dual-binding mechanism — implementation map

Follow **`add-engine-effect`** lifecycle + **`add-engine-variant`** gate (verdict in §0).

**3a. Type (`types/ability.rs`, NOT frozen).** Add `TargetFilter::GrantingObject` with `// CR 201.5a` doc-comment noting it is always concretized to `SpecificObject` at grant-clone and otherwise resolves like `SelfRef`→source (fail-safe). No serde default concerns beyond the standard enum add (card-data.json is gitignored/regenerated; snapshots are curated — only the ≈19 granter cards churn).

**3b. Parser (`oracle_parser` skill — nom mandate).**
- *Masking pre-pass* (in `oracle_util.rs`, mirroring the existing `mask_card_name_keyword_action`/`mask_ring_tempts_you_phrase` precedents; invoked from `normalize_card_name_refs` at `:1605` before the name→`~` replacement, or from `parse_oracle_ir` before the `:387` normalize call): within each double-quoted region `"…"`, replace occurrences of the card name (+ comma/of/short-name variants already computed by the normalizer) with a reserved placeholder token (reuse the `KEYWORD_ACTION_PLACEHOLDER` char convention). Outside quotes, the name normalizes to `~` as today. `SELF_REF_TYPE_PHRASES` ("this creature") still → `~` everywhere. Net: placeholder appears ONLY in granted bodies that name the card.
- *Recognition:* extend the canonical `parse_self_reference` (`oracle_nom/target.rs:298`) with `value(TargetFilter::GrantingObject, tag(PLACEHOLDER))` as the first alt — this covers all effect-target self-refs (`parse_target`) in one edit. Add the same placeholder→GrantingObject recognition at the cost self-ref sites in `oracle_cost.rs` (the inline `~`→SelfRef points ≈ `:596/:809/:1037/:1500/:1596`). **Safety:** any un-migrated self-ref site that meets the placeholder must degrade to `SelfRef` (never leak the literal token) — make `parse_self_reference` the single authority where feasible and add a final `placeholder→~` cleanup in `parse_quoted_ability` before returning, so a body path we didn't migrate parses as today (host) rather than breaking. (This preserves coverage honesty; worst case an un-migrated granter-ref stays the *old* buggy host binding, not a regression.)

**3c. Grant-time concretization (`game/layers.rs`, NOT frozen).** In the `GrantAbility` arm (`:4967`) and `GrantTrigger` arm (`:4984`), before the dedup/push, run a recursive rewrite over the cloned definition replacing `TargetFilter::GrantingObject` → `TargetFilter::SpecificObject { id: effect.source_id }`. Walk: `AbilityCost` tree (Sacrifice/Exile/Tap/Unattach/RemoveCounter targets + `Composite`), `Effect` `target` fields, and recurse `sub_ability`/`else_ability`/`mode_abilities`. `// CR 201.5a + CR 613.1f`. Dedup on the rewritten value. (A focused mutator is fine; if a general TargetFilter-map helper is wanted it can live next to the read-only `ability_scan.rs` walker — not required.)

**3d. Non-frozen resolution defense (`game/targeting.rs`).** In `resolved_targets` (`:630`), add a `GrantingObject` short-circuit mirroring `SelfRef` → `vec![TargetRef::Object(ability.source_id)]` (source_id fail-safe), so any GrantingObject that ever reaches runtime unrewritten degrades to today's behavior, never worse. Classify `GrantingObject` in the fail-closed `ability_scan.rs` walker (same axes as `SelfRef` — a source-relative object ref). `find_legal_targets` (`targeting.rs`) similarly mirrors the `SelfRef` handling.

## 4. The Dominion Bracelet — full remaining work (one increment)

The Bracelet needs BOTH the dual binding (§3) AND the trailing cost-reduction coverage fix, sequenced together so the card is only marked supported once the self-exile is correct (per memory `unimplemented-to-supported-exposes-unwired-path`: flipping to supported while the exile still hits the creature would be *worse* than Unimplemented).

**Coverage fix (simpler than the brief's suggestion — reuse the established pattern):** the granted body already parses the reduction sentence into a terminal `Effect::Unimplemented { description: "This ability costs {X} less to activate, where X is ~'s power" }` sub_ability (confirmed in the exported AST). `extract_cost_reduction_from_chain` (`oracle.rs:4922`) already strips exactly this node via `try_parse_cost_reduction` → `try_parse_dynamic_x_cost_reduction` → `parse_dynamic_x_clause` → `parse_quantity_ref("~'s power")` = `QuantityRef::Power{Source}` (all verified to parse). It is called at 7 standalone-ability sites in `oracle.rs` (`:2453,:2498,:2912,:2950,:2979,:3057,:4499`) but **NOT** from `parse_quoted_ability`. Fix = call `extract_cost_reduction_from_chain(&mut def)` in the cost-separator branch of `parse_quoted_ability` (`oracle_static/grammar.rs:1281-1284`, after `def` is built) and make `extract_cost_reduction_from_chain` `pub(crate)`. This is the ~2-line minimal fix and reuses the exact standalone-ability pattern.
- The brief's alternative (call `split_trailing_self_cost_reduction`, make it `pub(crate)`, split before parsing) also works but requires the borrow-dance around `strip_activated_constraints`'s `String` return and duplicates logic; **prefer the chain-extractor** (established, AST-level, no borrow gymnastics). Note either way the reduction's `Power{Source}` is host-referential and is the untouched third channel — no interaction with the GrantingObject rewrite.

**Sequence (single commit / increment):** (1) `TargetFilter::GrantingObject` type + parser masking/recognition; (2) `layers.rs` grant-clone rewrite (GrantAbility + GrantTrigger); (3) targeting/ability_scan defensive arms + the 3 frozen filter.rs fall-through arms (§5); (4) `parse_quoted_ability` cost-reduction wiring; (5) tests (§6). Only after (1)-(4) does the Bracelet's `Exile ~` exile the Bracelet and the reduction read the creature's power.

## 5. Frozen-file check & CR annotations

Frozen set: `game/effects/mod.rs`, `game/filter.rs`, `game/effects/delayed_trigger.rs` (design edits prohibited; `filter.rs::SelfRef` at `:1506` READ-ONLY).
- **`effects/mod.rs`** — TargetFilter matches at `:221`/`:283` both end in `_ =>` wildcards → **no edit** (new variant absorbed by wildcard). ✅
- **`delayed_trigger.rs`** — TargetFilter matches near `:329`/`:370` have `_ =>` arms → **no edit**. ✅
- **`filter.rs`** — THREE exhaustive `match filter` blocks with NO wildcard, each ending in a `| … => false` fall-through: `:48` (ends `| AllPlayers => false` ~`:101`), `:251` (~`:307`), and `filter_inner_for_object` `:1494` (ends `| … | Owner => false` ~`:1670`). A new `TargetFilter` variant is **compile-forced** to add an arm in all three. Required change = append `| TargetFilter::GrantingObject` to each existing `=> false` fall-through — **3 mechanical, compile-only lines; NOT the `SelfRef` arm at `:1506`; no design/behavior change** (GrantingObject is rewritten to `SpecificObject` at grant-clone, so it never reaches `filter.rs` at runtime; `=> false` is the correct never-reached value). **Coordinate these 3 append-only lines with the frozen-file owner, or land after the freeze window clears.** This is the one unavoidable frozen touch; there is no wildcard to hide behind and Rust requires the arm. The alternative — a magic reserved-`ObjectId` sentinel on `SpecificObject` (no new variant) — is rejected: it violates the typed-over-magic hard rule and is fail-open (an un-rewritten sentinel silently no-ops), whereas the typed variant is fail-safe.

**CR annotations (all grep-verified against `docs/MagicCompRules.txt`):**
- `CR 201.5a` (`:1324`) — granted ability's by-name reference resolves to the specific grant-source object. **Primary rule; annotate the new variant + the layers rewrite.**
- `CR 201.5` (`:1322`) — text refers to the object it's on by name means that object.
- `CR 613.1f` (`:2970`) — Layer 6 ability-adding (the grant clone).
- `CR 701.13a` (`:3380`) — "To exile an object, move it to the exile zone." (Dominion Bracelet exile-cost).
- `CR 701.21a` (`:3451`) — "To sacrifice a permanent…" (Deconstruction Hammer sacrifice-cost).
- `CR 702.6a` (`:3935`) — Equip. **NB: the brief's "701.3x for equip" is a hallucinated CR — equip is 702.6a, verified; do not annotate 701.3x.**
- `CR 601.2f` — self-referential cost reduction (already used by `CostReduction`).

## 6. Test plan — bidirectional, discriminating, revert-to-red (`card-test` skill)

Model on the existing equipment harness `engine_tests.rs:7138 setup_equip_game`/`:7150 create_equipment`. Every test drives the real cast/activate pipeline and asserts which object left the battlefield (not AST shape).

**Direction A — granter-referential COST resolves to the GRANTING object (the bug):**
- A1 Deconstruction Hammer (sacrifice): attach to a creature, activate `{3},{T},Sacrifice ~`, pay costs, target an artifact. Assert: **the Hammer is in its owner's graveyard AND the equipped creature is still on the battlefield** (and the targeted artifact destroyed). *Revert-to-red:* remove the `layers.rs` GrantingObject→SpecificObject rewrite → the SelfRef path sacrifices the creature (creature leaves, Hammer stays) → assertion fails.
- A2 The Dominion Bracelet (exile): attach, activate `{15}, Exile ~`, pay. Assert: **the Bracelet is in exile AND the creature survives.** *Revert-to-red:* same as A1 → exiles the creature.
- A3 (effect-target channel) Trusty Boomerang / Razor Boomerang: activate, assert the **equipment** returns to hand, not the host. *Revert-to-red:* rewrite removed → bounces the creature.

**Direction B — host-referential reads STILL read the HOST (must not change):**
- B1 The Dominion Bracelet reduction: equip a creature with power N (e.g. 3), assert the ability's generic mana cost is reduced by exactly N (reads the equipped creature's power via `Power{Source}`). *Revert-to-red on host path:* if a (wrong) implementation rebound the power read to the granter (equipment, no power), reduction = 0 → cost stays {15} → assertion fails. Since the reduction is a `QuantityRef` (untouched channel), this test also guards against accidental over-reach of the GrantingObject change.
- B2 Sliver host-ref: grant Acidic-Sliver-style `Sacrifice this permanent: ~ deals 2 damage` to a *second* Sliver, activate on the second Sliver. Assert: **the second (host) Sliver is sacrificed**, NOT the granting Sliver, and the damage source is the host. *Revert-to-red:* if the design blanket-rebound SelfRef→granter, the granting Sliver would be sacrificed → assertion fails. This is the discriminating proof that "this permanent" (SelfRef) is NOT rebound.

**Parser-level (shape, supplementary — not sufficient alone):** `parse_oracle_text("The Dominion Bracelet …")` → the granted def has cost `Exile{ GrantingObject }` (not `SelfRef`), and `def.cost_reduction = Some(CostReduction{ count: Power{Source}, .. })` with no residual `Unimplemented` sub_ability; the Sliver granted def keeps `SelfRef`. Non-vacuous: assert the two normalize to *different* filters from the same `~`-collapsed source. Add an `insta` snapshot for the Bracelet granted def.

**Coverage:** `cargo coverage` must show The Dominion Bracelet + Deconstruction Hammer lose their `Unimplemented`/gap and no regression in the 73 host-ref cards (run the card-data coverage-regression check — parser masking can affect other cards; memory `parser-coverage-regression-ci-only`).

---

## Risks / open questions for /review-engine-plan

1. **Frozen `filter.rs` (3 compile-forced fall-through arms).** Unavoidable for any new `TargetFilter` variant (three exhaustive no-wildcard matches: `:48`, `:251`, `:1494`). Proposed: 3 append-only `| TargetFilter::GrantingObject` additions to existing `=> false` lists, coordinated with the frozen owner, or land after the freeze clears. Is coordination acceptable, or should the increment wait? (Rejected the magic-`ObjectId`-on-`SpecificObject` alternative on typed-over-magic + fail-open grounds — please confirm.)
2. **Masking bounds.** The card-name-in-quotes masking must (a) reuse the normalizer's already-computed name variants (full/comma/of/short) so it stays in sync, and (b) never touch a token that isn't the card's own name (token-creation quotes name a *different* token, so they're safe; verify no `named "<self>"` false hit). Is a quote-region masker the right layering vs. threading a `ParseContext.in_granted_body` flag into every self-ref site? (Chose masking to avoid ctx-threading ≈8 parser sites; the un-migrated-site cleanup `placeholder→~` is the safety net.)
3. **`GrantTrigger` cost/effect coverage.** Spare Dagger / Sunfire Torch / Trickster's Talisman self-name inside a granted *trigger's* "you may sacrifice X" action. Confirm the rewrite walker at `layers.rs:4984` reaches the trigger's execute chain (the `GrantTrigger` stores the trigger def; ensure the mutator recurses into its `execute`/effect chain, not just top-level).
4. **Adjacent channels deliberately deferred.** QuantityRef granter-refs (Archery Training "counters on [aura]") and damage-source attribution ("[equipment] deals damage") are the same root cause via non-TargetFilter channels and are NOT fixed here. Confirm they may ship as follow-up increments (the mechanism generalizes: thread the same granter id into `ObjectScope` resolution and the effect source). The confirmed self-exile/sacrifice bug is fully fixed for the ~19-card TargetFilter class.
5. **CR 201.5a zone-change clause.** "…if the second ability also moved the first ability's source to a different public zone, the name refers to the object the source became in its new zone." For Exile/Sacrifice-as-cost the object leaves before the effect and isn't re-referenced (Hammer/Bracelet) — fine. For "Return X to hand" (Razor/Toralf/Trusty Boomerang) the equipment is still on the battlefield when referenced (Unattach ≠ zone change) — `SpecificObject{id}` at the live id is correct. Any card that exiles-then-references-in-new-zone would need incarnation handling; none in the current class. Confirm out of scope.
6. **`extract_cost_reduction_from_chain` vs brief's `split_trailing_self_cost_reduction`.** Chose the former (established 7-site AST pattern, no borrow-dance). Confirm this substitution is acceptable vs. the brief's named function.

---

## REVIEW OUTCOME (/review-engine-plan, opus/xhigh) — AUTHORITATIVE; hard gates

**VERDICT: APPROVE-WITH-REQUIRED-REVISIONS.** Core design (parse-time GrantingObject + grant-time SpecificObject,
per-referent) CONFIRMED sound; crux passes (dual/triple binding verified on real Bracelet text; host-ref Acidic Sliver
"this permanent" untouched; all CR verified 201.5a/613.1f/701.13a/701.21a/702.6a/601.2f; zone-change edge safe —
SpecificObject re-minted each layer pass). Revisions are enumeration/boundary gaps:

### R1 [BLOCKER] — masking corrupts `named <self>` name-filters (8 measured regressions)
Naive "mask every card-name in quotes" also rewrites the name in name-FILTER positions (not just self-REFERENCE).
8 cards have own name after `named` in a quoted body: Food Fight, Deathpact Angel, Ominous Traveler, Ozox, Pass the
Torch, Rekindling Phoenix, Sengir Nosferatu, Sound the Call. Masking to PLACEHOLDER before the :1640 name→~ step
bypasses the `named ~ → named <name>` restoration (:1827) → parse fails. §3b safety net yields `named ~` (still wrong).
FIX: masker SKIPS card-name in `named …`/`for each … named …` position (safe — granter self-ref is never in `named`
position), OR add `named <PLACEHOLDER> → named <name>` restoration mirroring :1827. Add Food Fight as revert-to-red
host-ref NEGATIVE test. Run CI card-data coverage-regression + semantic-audit (masker runs corpus-wide at normalize).

### R2 — frozen filter.rs is SIX compile-forced arms, not three
Named :48/:251/:1494; sweep found :1980, :2363, :2600 (all closer `| TargetFilter::Owner => false`). All 6 identical
append-only `| TargetFilter::GrantingObject` to a `=> false` fall-through — append-only, compile-forced, zero logic.
(:1494 closer is :1927 not :1670 — line drift.) Coordination surface = 6 lines in the one frozen file.

### R3 — enumerate ALL forced arms via cargo check (don't hardcode)
Beyond ability_scan.rs:2252 (§3d), 4 NON-frozen forced sites omitted: cost_payability.rs:50 + :103, coverage.rs:464
(needs a REAL description arm e.g. "granting object", not => false), trigger_matchers.rs:736. Total = 11 arms.
effects/mod.rs + delayed_trigger.rs have `_ =>` wildcards → NO edit → entire frozen touch CONTAINED to filter.rs.
MANDATE: run `cargo check -p engine` to enumerate all 11 (per add-engine-variant step 2), don't ship a hardcoded list.

### R4 — cost self-ref channel scattered; "~19 FIXED" imprecise
Effect-target channel = clean single edit (parse_target oracle_target.rs:696 → parse_self_reference target.rs:296;
one `value(GrantingObject, tag(PLACEHOLDER))` first-alt covers Return/GainControl/Destroy ~). Cost channel = ~15 inline
tag("~")/rest=="~" sites in oracle_cost.rs (:596,:809,:1037,:1500,:1596,:1993,:2003-2042,:2603-2639) — NOT centralized;
"≈5" undercounts. **Unattach <name> is nullary AbilityCost::Unattach (oracle_cost.rs:1016, no TargetFilter) → Shuriken +
Leonin Bola NOT fixed by GrantingObject** (Unattach correctness = runtime cost resolver, OUT OF SCOPE — don't claim them).
Genuinely cost-SelfRef-fixed: Exile/Sacrifice cards (Bracelet, Deconstruction Hammer, Toralf's, † Sacrifice-in-trigger).
Enumerate per-card the actual channel; route scattered cost `~` sites through ONE shared self-ref combinator (single-
authority, compose-don't-enumerate) recognizing both ~→SelfRef and PLACEHOLDER→GrantingObject.

### FROZEN filter.rs — SAFE to land (append-only, 6 arms); no non-frozen alternative (variant is compile-forced).
Grant-time rewrite replaces GrantingObject→SpecificObject before any runtime filter path → `=> false` never reached.
Note the intentional asymmetry: filter.rs default `false` (predicate) vs targeting.rs default `source_id` (target-producer)
— per-context, don't "fix" it.

### CONFIRMED-SOUND (blessed): triple-channel per-referent binding; host-ref must-not-change; variant gate
(GrantingObject EXTEND_OK, ≠AttachedTo/SelfRef/SpecificObject); cost-reduction wiring (extract_cost_reduction_from_chain
oracle.rs:4922 → pub(crate), grammar.rs:~1281 mutable def; Power{Source} host untouched); zone-change re-mint.
RESIDUAL: GrantStaticAbility (layers.rs:5002) third grant vector uncovered — verify no class card grants a granter-naming
STATIC (none in sample, low); damage-source + CountersOn channels correctly deferred.

## FOLLOW-UP LEDGER (added 2026-07-04, post /review-impl round 1)
**CR 201.5a QuantityRef/condition/damage-source/exclusion granter-ref channel — KNOWN LATENT, OUT OF SCOPE.**
A card's own name inside a quoted granted ability, in a NON-verb-object position, host-binds (`~`/SelfRef → host) at baseline but per CR 201.5a should bind to the GRANTER. This increment fixed only the cost + effect-target channels; it deliberately narrows the masker to verb-object positions to keep these channels BYTE-IDENTICAL to baseline (no regression). Pre-existing rules-wrong (e.g. Gutter Grime's token counts counters on the token, not the granting enchantment). Evidence/guard cards: Archery Training, Gutter Grime (Modern-legal), Saproling Burst (QuantityRef); Animal Friend (exclusion); Torrent of Lava (damage-source). Widening = per-channel combinator work (parse_quantity_ref `tag(" counters on ~")` oracle_quantity.rs:174-177, etc.) = its own building-block increment. Memory: quantref-granter-ref-201-5a-followup. Inline comment at the masker position-guard.
