# MSH-F Heavy Cluster — Implementation Plan (Cosmic Cube + Hawkeye, Young Avenger) — ROUND 2

> Revised by `mshf-planner` after `/review-engine-plan` round 1 (CHANGES REQUIRED on both sub-plans).
> Worktree: `/home/lgray/vibe-coding/wt-msh-f`, branch `feat/msh-f-cosmic-cube-hawkeye` (off main `5eca83b8c` = v0.7.0).
> Every file:line below was re-opened and confirmed THIS round (see `MSH-F-VERIFY-r2.md`). Planning only — no code written.

Both cards are `Swallow:DynamicQty` gaps. Deep tracing (round 1) + re-verification (round 2) confirm **neither needs a new enum variant**. Each reduces to an existing, supported type specimen plus one narrow extension. They touch disjoint subsystems (cast-from-library parser vs. damage-replacement type+resolver+parser) → two independent, separately committable sub-plans.

---
## Round-2 changes — how each review finding is resolved

### Sub-Plan A (Cosmic Cube)
| Finding | Severity | Resolution (file:line verified this round) |
|---|---|---|
| Aggregate delegation fails on real input (snapshot_ok gate) | **HIGH** | Round-1 plan delegated to `parse_quantity_ref_with_context`, whose aggregate branch gates on `snapshot_ok = remainder empty OR cast-snapshot suffix` (oracle_quantity.rs:300-310) — the trailing `" without paying its mana cost"` makes snapshot_ok=false → `None`. **Fix:** delegate instead to the nom combinator `nom_quantity::parse_quantity_ref` (oracle_nom/quantity.rs:560), whose `parse_object_property_aggregate_ref` arm (oracle_nom/quantity.rs:959) **returns the remainder** (`final_remainder`, lines 1013-1031) rather than requiring it empty. It consumes only `"the greatest power among attacking creatures you control"` and leaves `" without paying its mana cost"` for the constraint parser to ignore. This is the **class-general** fix — no hardcoded `take_until(" without paying")` card-specific delimiter. Verification-matrix row A0 added. |
| `parse_mv_comparator` factoring conflates two positions | MEDIUM | The existing Beseech comparator is a **POST-value suffix** (`parse_quantity_expr_number` @mod.rs:15621, then `tag("or less")`/`tag("or greater")` @15623-15629). Cosmic Cube's is a **PRE-value prefix** (`"less than or equal to" N`). The plan no longer claims a shared `parse_mv_comparator`. The new branch defines a **dedicated prefix-comparator `alt`** (LE/GE/LT/GT/EQ, all in `Comparator` ability.rs:5220). Two distinct grammar shapes → two distinct sub-combinators. |
| row-2 enforcement seam mis-stated | MEDIUM | Real enforcement is at cast **FINALIZATION**: `selected_exile_alt_cost_permission_accepts_resulting_mv` (casting.rs:1695) + `exile_alt_cost_permissions_accept_resulting_mv` (casting.rs:1763), called casting_costs.rs:5350-5374 with `Some(resulting_mv)`; over-ceiling ⇒ `handle_cancel_cast` + `Err(ActionNotAllowed("Spell mana value does not satisfy the cast permission"))` (casting_costs.rs:5359-5362). The dynamic branch of `cast_permission_constraint_allows_cast` (casting.rs:1592-1598) returns permissive `true` when `resulting_mv` is `None` (no printed-MV fallback, unlike the Fixed branch @1586-1590), so the spell **is offered** but **rejected at finalize**. Row-2 reframed to assert finalize-time rejection. |
| provenance "Consumer: cast_from_zone.rs:357" wrong path | LOW | The `from among them` arm hard-codes `driver: LingeringPermission` (mod.rs:15503; constraint @15501). Constraint is consumed by `grant_lingering_permissions` (cast_from_zone.rs:491) → `CastingPermission::ExileWithAltCost{constraint}` (cast_from_zone.rs:556/568) — **not** the during-resolution path. (casting_costs.rs:5946 is the Cascade/Discover resolution-cleanup caller, unrelated.) Runtime test targets the lingering path. |
| line drift / CR | LOW | `FilterProp::Attacking` = ability.rs:**2591**; `ObjectProperty::Power` variant = ability.rs:**4606** (enum decl 4605); `QuantityRef::Aggregate` = ability.rs:4132; `CastPermissionConstraint::ManaValue` = ability.rs:2185 (variant 2188-2191, `value` already `QuantityExpr`). CR 601.2e = cast-legality (line 2466); CR 601.2f = total-cost lock-in (line 2468) — paired at the finalize seam; CR 202.3 = mana value (line 1359). |

### Sub-Plan B (Hawkeye)
| Finding | Severity | Resolution (file:line verified this round) |
|---|---|---|
| never acknowledges sibling `QuantityModification::Plus{value:u32}` | MEDIUM | Added as **explicit NON-GOAL** with categorical-boundary rationale (counters/tokens are a different resolvable section than damage CR 120). Blast-radius **disambiguated**: a naïve `Plus {` grep = **19 hits** in oracle_replacement.rs and conflates the two enums. The `DamageModification::Plus` field-lift is **compiler-enforced** across exactly **4 files** (`grep -rln DamageModification::Plus`): replacement.rs, oracle_replacement.rs, add_target_replacement.rs, oracle_trigger.rs. `QuantityModification::Plus` (oracle_replacement.rs:5737/6292/6308/12704/14144/14716/14739) is a **separate enum** — untouched. |
| deferred follow-up mis-named | LOW | Reframed: the genuine parameterize-don't-proliferate candidate is `SetTo{value}` (ability.rs:15800) + `SetToSourcePower` (ability.rs:15794) → lifting `SetTo.value` collapses `SetToSourcePower` into `SetTo{Ref(Power{Source})}`. `Minus` is genuinely independent. Both out of scope NOW. |
| parser-arm foot-guns | LOW | (1) New dynamic arm must **precede** `value(Plus{0}, tag("plus x"))` in `parse_that_much_damage_offset` (oracle_replacement.rs:4942), else the freeze arm shadows it. (2) Hawkeye text ends `"...~'s power."` WITH a period — `parse_cda_quantity` **strips trailing `.`** internally (oracle_quantity.rs:676 `text.trim().trim_end_matches('.')`), so the period is tolerated; it returns `Option<QuantityExpr>` so it composes via `map_opt`. `scan_damage_modification` uses `scan_at_word_boundaries` (position-independent) so the leading `"instead"` does not block matching at the `"that much damage"` boundary. |
| name the regression-guard migrations | LOW | Artist's Talent assertions oracle_replacement.rs:**11783** + **11810** (`Plus{value:2}`); placeholder test **14026** (`Plus{value:0}`); literal test **14036** (`Plus{value:2}`). All migrate to `Plus{value: QuantityExpr::Fixed{value:N}}`. |

---
## SHARED EVIDENCE (re-verified this round)

| Concept | Existing slot | Evidence (file:line, this round) |
|---|---|---|
| Look-top-N → cast one free → bottom rest random | `Dig{count,keep_count:0,filter}` → `CastFromZone{ExiledBySource,without_paying:true}` → `PutAtLibraryPosition{ExiledBySource,0,Bottom}` | **Aetherworks Marvel** parsed AST (`supported:true`). Cosmic Cube ALREADY parses to `YouAttack`+this chain with `"constraint":null` (card-data.json) |
| Dynamic MV ceiling | `CastPermissionConstraint::ManaValue{comparator:Comparator, value:QuantityExpr}` | ability.rs:2185 (variant 2188-2191) — `value` **already** `QuantityExpr` |
| "greatest power among \<filter\>" | `QuantityRef::Aggregate{Max, ObjectProperty::Power, filter}` | ability.rs:4132; nom combinator `parse_object_property_aggregate_ref` oracle_nom/quantity.rs:959 (remainder-returning); Power variant ability.rs:4606 |
| "attacking creatures you control" | `Typed(creature, [FilterProp::Attacking{defender:Some(You)}], controller:You)` | `FilterProp::Attacking` ability.rs:2591 |
| "whenever you attack" | `TriggerMode::YouAttack` | live in card-data.json Cosmic Cube parse |
| Damage-amp replacement | `ReplacementDefinition{event:DamageDone, damage_modification:Plus{..}, source Typed{You}, target PlayerOrPermanentsControlledBy{Opponent}, NoncombatOnly}` | **Artist's Talent** (`supported:true`). Hawkeye ALREADY parses to this with `Plus{value:0}` (frozen) — card-data.json |
| Replacement reads source power (live) | `SetToSourcePower` reads `state.objects.get(&rid.source).power` | `damage_done_applier` replacement.rs:956-968 |
| Resolve a QuantityExpr | `resolve_quantity(state,&expr,controller,source_id)` | quantity.rs:68 |
| QuantityExpr serde back-compat | custom `Deserialize`: bare int → `Fixed{value}` (recursive) | ability.rs:5013-5028 |

**CR numbers** (grep `docs/MagicCompRules.txt`, line in parens; re-grep before writing each annotation per CLAUDE.md):
CR 601.2e@2466 (cast legality) · CR 601.2f@2468 (total cost locked-in) · CR 614.1a@3056 (instead=replacement) · CR 508.1@2260 / CR 508.1b@2264 (declare attackers) · CR 202.3@1359 (mana value) · CR 208.1@1509 (power) · CR 120.1@1087 / CR 120.3@1097 (damage) · CR 107.1b@455 (negative→0 clamp) · CR 109.4@594 (off-battlefield controller→owner) · CR 107.3a@466 (X chosen at cast).

---
# SUB-PLAN A — Cosmic Cube  (Artifact {5}; Ward {2} already supported)

Text: *"Ward {2}\nWhenever you attack, look at the top six cards of your library. You may cast a spell from among them with mana value less than or equal to the greatest power among attacking creatures you control without paying its mana cost. Put the rest on the bottom of your library in a random order."*

**Today's parse (card-data.json):** `YouAttack` trigger wrapping the Aetherworks-Marvel `CastFromZone{ExiledBySource, without_paying:true}` chain with `"constraint":null`. **The ONLY gap is the dropped MV ceiling.** The constraint parser `parse_cast_permission_constraint` (mod.rs:15613-15631) recognizes only the Beseech form `"if that spell's mana value is N or less"`; it does not recognize Cosmic Cube's `"with mana value less than or equal to <dynamic quantity>"`.

### Pattern Coverage
The "you may cast a spell from among [looked-at top N] **with mana value ≤ \<dynamic\>** without paying" class. The new combinator covers the whole **comparator × quantity-expr** grid (LE/GE/LT/GT/EQ × {aggregate, fixed N, X}), not one card. The put-onto-battlefield twin of this class already works via `FilterProp::Cmc` (Ayesha Tanaka, Ao the Dawn Sky); this plan brings the **cast** branch to parity. ≥6-10 existing/future impulse-cast cards whose ceiling is a game-state quantity.

### Logic Placement — 100% parser
The effect chain, the constraint type (`value` already `QuantityExpr`), the aggregate quantity, the trigger, the filter, and the runtime enforcement all already exist. Only the surface-form recognizer for the new constraint phrasing is missing.

### Nom Compliance (mandatory) — design
Refactor `parse_cast_permission_constraint` (mod.rs:15613) into a dispatch over **two grammar-shape sub-combinators** (the existing comparator is post-value; the new one is pre-value — they cannot share a comparator combinator):

```text
parse_cast_permission_constraint(lower):
    parse_beseech_mv_constraint(lower)            // EXISTING, unchanged
        .or_else(|| parse_with_mana_value_constraint(lower))   // NEW
```

- **`parse_beseech_mv_constraint`** = today's body verbatim: `take_until("if that spell's mana value is ")` → value (`parse_quantity_expr_number`) → POST-value suffix comparator (`tag("or less")`→LE, `tag("or greater")`→GE, else EQ). No change.
- **`parse_with_mana_value_constraint`** (NEW, nom-only):
  1. Locate anchor: `take_until` for the first of `alt((tag("with mana value "), tag("of mana value ")))`, then consume the anchor `tag`.
  2. **Dedicated PREFIX comparator** `alt` (longest-match ordering — LE/GE before LT/GT; EQ last):
     ```text
     alt((
       value(Comparator::LE, tag("less than or equal to ")),
       value(Comparator::GE, tag("greater than or equal to ")),
       value(Comparator::LT, tag("less than ")),
       value(Comparator::GT, tag("greater than ")),
       value(Comparator::EQ, tag("equal to ")),
     ))
     ```
  3. **Quantity** — aggregate-or-fixed `alt`, dynamic first:
     ```text
     alt((
       map(nom_quantity::parse_quantity_ref, |qty| QuantityExpr::Ref { qty }),   // remainder-returning; leaves " without paying its mana cost"
       nom_quantity::parse_quantity_expr_number,                                  // Fixed{N} | Variable{X}
     ))
     ```
     `nom_quantity::parse_quantity_ref` (oracle_nom/quantity.rs:560) routes `"the greatest power among attacking creatures you control …"` through `parse_object_property_aggregate_ref` (959), which returns `Aggregate{Max, Power, attacking-you-control}` **and the leftover suffix** — the suffix is discarded (the constraint is fully determined). `nom_quantity` is already imported (mod.rs:73; used @4181/8811).
  4. Emit `CastPermissionConstraint::ManaValue{comparator, value}`. **Annotate CR 202.3 + CR 601.2e** (mirror the existing constraint-type doc).

Detector-is-parser honored: dispatch is `parse_cast_permission_constraint(rest).is_some()` (mod.rs:15367), never `contains`. No string-match dispatch added.

### Why the nom-combinator path beats `take_until(" without paying")`
The review's `take_until(" without paying")` works but hardcodes a Cosmic-Cube-specific delimiter — it would silently fail any sibling whose suffix is worded differently ("without paying that spell's mana cost", "rather than paying…", or no suffix). `nom_quantity::parse_quantity_ref` stops at the **filter boundary** structurally, so it covers the whole class. This is "build for the class, not the card" (CLAUDE.md).

### Enforcement (runtime, already wired — no engine change)
1. The constraint flows untouched: `from among them` arm (mod.rs:15490) stores it in `CastFromZone.constraint` (15501), `driver: LingeringPermission` (15503).
2. `grant_lingering_permissions` (cast_from_zone.rs:491) stamps `CastingPermission::ExileWithAltCost{constraint}` (556/568) on each looked-at card.
3. **Offer:** `exile_alt_cost_permission_supports_cast(..., resulting_mv=None)` (casting.rs:1670) → dynamic branch returns permissive `true` (casting.rs:1594) → spell offered.
4. **Finalize:** `selected_exile_alt_cost_permission_accepts_resulting_mv` (casting.rs:1695) + `exile_alt_cost_permissions_accept_resulting_mv` (casting.rs:1763) re-check with `Some(resulting_mv)`; the dynamic branch (casting.rs:1592-1598) computes `required = resolve_quantity(state, value, obj.controller=caster, obj.id)` and enforces `comparator.evaluate(resulting_mv, required)`. Over-ceiling ⇒ `Err(ActionNotAllowed("Spell mana value does not satisfy the cast permission"))` (casting_costs.rs:5359-5362). **Governed by CR 601.2e (legality) + CR 601.2f (resulting cost/MV locked-in).**

### Identity / Provenance Contract
- Cast pool: `from among them` → `TargetFilter::ExiledBySource` (same provenance as Aetherworks Marvel).
- MV ceiling: `QuantityExpr::Ref(Aggregate{Max, Power, attacking-creatures-you-control})` — **live**, resolved at cast finalization against the current battlefield. Attackers are still attacking during the attack-trigger resolution (CR 508.1/508.1b). Stored in `CastFromZone.constraint`; consumed via `grant_lingering_permissions` → `ExileWithAltCost{constraint}` (cast_from_zone.rs:556/568) → `resolve_quantity(...obj.controller, obj.id)`.
- Hostile multi-authority fixture: two attacking creatures of differing power → ceiling = max of the two; a third **non-attacking** high-power creature you control must NOT raise the ceiling (the `FilterProp::Attacking` filter excludes non-attackers).

### Variant Discoverability — no new variant
`cargo engine-inventory`: `CastPermissionConstraint::ManaValue` (value:QuantityExpr), `QuantityRef::Aggregate`, `ObjectProperty::Power`, `AggregateFunction::Max`, `FilterProp::Attacking`, `TriggerMode::YouAttack`, `Comparator::{LE,GE,LT,GT,EQ}` all present. **No enum surface change → `add-engine-variant` gate N/A.** Inventory impact: none.

### Verification Matrix (Sub-Plan A) — every test names the revert-fail assertion
| # | Claim | Seam | Test (discriminating) | Revert-fail assertion | Hostile / negative |
|---|---|---|---|---|---|
| A0 | trailing suffix does NOT defeat quantity parse | `nom_quantity::parse_quantity_ref` | unit: `parse_quantity_ref("the greatest power among attacking creatures you control without paying its mana cost")` ⇒ `Ok((" without paying its mana cost", Aggregate{Max,Power, attacking creatures you control}))` | delegating to `parse_quantity_ref_with_context` (snapshot_ok gate) ⇒ `None` → constraint `None` | empty filter / `Any` ⇒ `Err` (no aggregate) |
| A1 | "with mana value ≤ \<aggregate\>" → constraint | `parse_cast_permission_constraint` | parser: parse Cosmic Cube line ⇒ `CastFromZone.constraint == Some(ManaValue{LE, Ref(Aggregate{Max,Power, attacking creatures you control})})` | today the constraint is `null` (verified in card-data.json) → assert fails without the new branch | (a) `"with mana value less than or equal to 4"` ⇒ `ManaValue{LE, Fixed{4}}` (fixed fallback); (b) `"greater than"` ⇒ GE/GT not LE; (c) existing `"if that spell's mana value is 4 or less"` ⇒ `ManaValue{LE, Fixed{4}}` unchanged (Beseech regression guard) |
| A2 | dynamic ceiling enforced at **finalize** (lingering path) | `selected_/exile_alt_cost_permissions_accept_resulting_mv` (casting.rs:1695/1763) | runtime: attackers max power 3; from the six looked-at, casting an MV-3 spell resolves, casting an MV-4 spell ⇒ `Err(ActionNotAllowed("Spell mana value does not satisfy the cast permission"))` | dropping/freezing the ceiling (today's `constraint:null`) lets the MV-4 spell cast ⇒ assert it is rejected | non-attacking high-power creature does not raise the ceiling; decline path bottoms all six in random order |
| A3 | trigger fires on attack | `TriggerMode::YouAttack` | runtime: trigger fires once on declare-attackers | n/a (already supported) | does not fire on opponent's attack |

Use `card-test` (GameScenario + GameRunner; assert via CastOutcome deltas / `Err` on the over-ceiling cast). **Frontend/AI/MP: no new surface** — `Dig`/`CastFromZone` overlays + impulse cast already play (Aetherworks Marvel today). No MP-filter or frontend work.

---
# SUB-PLAN B — Hawkeye, Young Avenger  (Creature 2/4 {3}; Reach already supported)

Text: *"Reach\nIf a source you control would deal noncombat damage to an opponent or a permanent an opponent controls, instead it deals that much damage plus X, where X is Hawkeye's power."*

**Today's parse (card-data.json):** `ReplacementDefinition{ event:DamageDone, damage_modification:{"type":"Plus","value":0}, NoncombatOnly, target PlayerOrPermanentsControlledBy{Opponent}, source Typed{You} }` — i.e. Artist's Talent's replacement **minus** the ClassLevel condition, but with the `+X` **frozen to `Plus{value:0}`** by the bare `"plus x"` arm. The single gap: `DamageModification::Plus.value` is `u32` (fixed) — there is no expression for "amount + a dynamic quantity." `SetToSourcePower` is *set*, not *add*, so it cannot compose.

### add-engine-variant gate (run — this changes an engine enum's field type)
- **Stage 1 (existence):** dynamic additive damage modification does not exist. `Plus{u32}` is fixed (ability.rs:15783); the `"plus x"` arm emits frozen `Plus{value:0}` (oracle_replacement.rs:4942). **DOES_NOT_EXIST.**
- **Stage 2 (parameterization):** **not** a new sibling — the canonical *parameterize-the-value-axis* lift: `Plus { value: u32 }` → `Plus { value: QuantityExpr }`. `QuantityExpr::Fixed` subsumes every existing fixed `Plus` (Torbran +2, Artist's Talent +2). **EXTEND_OK via field-type lift.**
- **Stage 3 (categorical boundary):** axis = "magnitude of a CR 614.1a / CR 120 damage-event modification" — single rule section. **WITHIN_SECTION. APPROVED.**

### Explicit NON-GOAL — `QuantityModification::Plus{value:u32}` (ability.rs:15826)
The structurally-parallel sibling on `QuantityModification` (counters/tokens; "Modeled after DamageModification", ability.rs:15810) is **NOT** lifted. **Categorical-boundary rationale:** although both carry `// CR 614.1a`, `QuantityModification` modifies CR 121 counter-placement / CR 111 token-creation events — a **separately resolvable** event class from damage (CR 120). The value-magnitude lift stays within the damage-event section; unifying across event classes would conflate sections the engine resolves independently (the categorical-boundary rule). **Blast-radius disambiguation:** a naïve `grep 'Plus {'` returns **19 hits** in oracle_replacement.rs and conflates the two enums. The lift touches **only** `DamageModification::Plus`, which is **compiler-enforced** across exactly 4 files (`grep -rln DamageModification::Plus`): `game/replacement.rs`, `parser/oracle_replacement.rs`, `game/effects/add_target_replacement.rs`, `parser/oracle_trigger.rs`. `QuantityModification::Plus` sites (oracle_replacement.rs:5737/6292/6308/12704/14144/14716/14739) are a different enum and are not touched — the exhaustive-match compiler error will fire only on the 4 `DamageModification` files, so no site is silently missed.

### Pattern Coverage
Class: damage-amplification replacements whose added amount is a live game quantity — "deals that much damage plus \<quantity\>". The new parser arm composes `parse_cda_quantity` (any CDA quantity: own power, life total, "the number of …"), so it covers the whole **fixed-and-dynamic additive** class, not just Hawkeye (X = own power). Reuses the source/target/combat filters that already serve Artist's Talent (fixed) and Torbran (fixed).

### Logic Placement
- **types/ability.rs:15783** — `Plus { value: u32 }` → `Plus { value: QuantityExpr }`. (`DamageModification` derives `Eq`, ability.rs:15775; `QuantityExpr` derives `Eq` → compiles.)
- **game/replacement.rs:949** — resolve the expression in the (already exhaustive) Plus arm of `damage_done_applier`:
  ```text
  // CR 614.1a + CR 120 + CR 107.1b: additive damage modification; added magnitude is a
  // live game quantity resolved against the replacement source (rid.source), clamped ≥0.
  DamageModification::Plus { value } => {
      let added = state.objects.get(&rid.source)
          .map(|obj| resolve_quantity(state, value, replacement_source_player(obj), rid.source).max(0) as u32)
          .unwrap_or(0);
      amount.saturating_add(added)
  }
  ```
  `damage_modification_for_rid` returns an **owned** clone (replacement.rs:873-892), so `value` is owned and the immutable `resolve_quantity(state, …)` borrow does not conflict with the `&mut state` of the applier (mirrors the existing `SetToSourcePower` arm's `state.objects.get(&rid.source)` read @956-968). `replacement_source_player` (replacement.rs:51, CR 109.4) supplies the controller — harmless for `Power{Source}` (source-scoped, controller ignored) but correct for any future aggregate value.
- **parser/oracle_replacement.rs:4935** — add a dynamic arm to `parse_that_much_damage_offset`, **before** the `value(Plus{0}, tag("plus x"))` freeze arm (4942):
  ```text
  // CR 614.1a + CR 107.3a: dynamic additive offset — "plus X, where X is <quantity>".
  // Lives BEFORE the bare-"plus x" freeze arm so the where-binding is not shadowed.
  // parse_cda_quantity strips a trailing '.' (oracle_quantity.rs:676) so "~'s power." parses;
  // it returns Option<QuantityExpr> → composed via map_opt.
  map_opt(
      preceded(tag("plus x, where x is "), nom::combinator::rest),
      |q: &str| parse_cda_quantity(q).map(|value| DamageModification::Plus { value }),
  ),
  ```
  Keep the existing fixed `"plus N"`/`"minus N"` arms and the bare-`"plus x"` freeze arm (now `Plus{Fixed{0}}`). `scan_damage_modification` (oracle_replacement.rs:4918) reaches this via `scan_at_word_boundaries` (primitives.rs:856, returns on first success ignoring remainder; position-independent → leading `"instead"` is fine). Input is lowercased + self-ref normalized (evidenced by the existing `tag("plus x")` and `tag("...~'s power...")`), so lowercase tags suffice; if impl finds mixed-case in this path, add the `"... where X is ..."` variant (cf. oracle_replacement.rs:3106-3107).

### Rust Idioms
`QuantityExpr` (typed) over a `u32`+flag. Single `resolve_quantity` authority (no duplicated inline power read). `saturating_add` + `.max(0)` clamp (CR 107.1b). The field-type change forces the compiler to revisit every `Plus{value}` site (the 4 files) — good.

### Serialized-surface audit (serde back-compat — confirmed in-repo)
Hawkeye's **live** card-data.json record is `"damage_modification":{"type":"Plus","value":0}` (bare int). After the lift, `QuantityExpr`'s custom `Deserialize` (ability.rs:5013-5028) loads a bare integer → `Fixed{value}`, so `{"type":"Plus","value":2}` (Artist's Talent, Torbran) and `…"value":0` round-trip to `Plus{Fixed{2}}` / `Plus{Fixed{0}}` unchanged. New records serialize tagged `{"type":"Fixed","value":N}`. No wire bump; card-data.json is regenerated anyway. **Add a load-test** asserting old `Plus{value:2}` JSON deserializes to `Plus{Fixed{2}}`. (`Minus{value:u32::MAX}` continuous-prevent sentinel is on `Minus`, NOT `Plus`, so the lift cannot disturb it.)

### Identity / Provenance Contract
"X is Hawkeye's power" = the **replacement source's own** current power. Authority: `QuantityRef::Power{scope:ObjectScope::Source}` bound to `rid.source`. **Live**, re-read each application (CR 614 applies as the event would occur), matching `SetToSourcePower`. Storage: `Plus.value`. Consumer: `resolve_quantity(…, source_id=rid.source)` (quantity.rs:68). Hostile fixture: two noncombat damage events in one batch with Hawkeye's power changed between them (a +1/+1 counter added mid-resolution) must use the **updated** power on the second — proving live read, not snapshot; and a **second** Hawkeye-like permanent's replacement must read **its own** `rid.source`, not Hawkeye's.

### Variant Discoverability
`cargo engine-inventory`: `DamageModification` (Plus field-type now `QuantityExpr`), `QuantityExpr`, `QuantityRef::Power{scope}`, `ObjectScope::Source`, `DamageTargetFilter::PlayerOrPermanentsControlledBy`, `CombatDamageScope::NoncombatOnly`, `ReplacementEvent::DamageDone` all present. **No new variant** — a field-type lift only. Inventory impact: the `DamageModification::Plus` field type changes `u32 → QuantityExpr` (regenerate `cargo engine-inventory` after the lift).

### Verification Matrix (Sub-Plan B) — every test names the revert-fail assertion
| # | Claim | Seam | Test (discriminating) | Revert-fail assertion | Hostile / negative |
|---|---|---|---|---|---|
| B1 | parse "plus X where X is ~'s power" → dynamic | `parse_that_much_damage_offset` new arm | parse Hawkeye line ⇒ `damage_modification == Plus{Ref(Power{Source})}`, `combat_scope==NoncombatOnly`, target `PlayerOrPermanentsControlledBy{Opponent}`, source `Typed{controller:You}` | with the freeze arm only (today) ⇒ `Plus{Fixed{0}}` (over-frozen, verified in card-data.json) → assert fails | `"plus 2"` ⇒ `Plus{Fixed{2}}` (Artist's Talent regression guard, oracle_replacement.rs:11783/11810/14036); `"minus 1"` ⇒ `Minus{1}` unaffected; trailing `"."` tolerated |
| B2 | resolver adds **live** source power | `damage_done_applier` Plus arm (replacement.rs:949) | runtime: Hawkeye power 2, a 3-damage noncombat source you control → opponent takes **5**; add a +1/+1 counter (power 4) → next noncombat event in the same batch adds **4** not 2 | hardcoding `Fixed{0}`/freezing ⇒ opponent takes 3 → assert fails | combat damage ⇒ NOT amplified (NoncombatOnly); damage to **your own** permanent ⇒ NOT amplified (target filter); a **different controller's** source ⇒ NOT amplified (source filter); damage to a **permanent an opponent controls** ⇒ amplified (second positive case) |
| B3 | serde back-compat | `QuantityExpr` Deserialize (ability.rs:5013) | load `{"type":"Plus","value":2}` ⇒ `Plus{Fixed{2}}` | n/a (proves no data break) | malformed/non-integer `value` rejected |

Use `card-test`/GameRunner with a concrete noncombat damage source (a ping ability) controlled by Hawkeye's controller. **AI/MP/frontend: no new surface** — replacement effects are engine-internal; no `GameAction`/`WaitingFor`/MP-filter added (Artist's Talent needs none).

### Deferred follow-up (out of scope NOW)
`SetTo{value:u32}` (ability.rs:15800) + `SetToSourcePower` (ability.rs:15794) are the genuine parameterize-don't-proliferate candidate: lifting `SetTo.value` to `QuantityExpr` would collapse `SetToSourcePower` into `SetTo{Ref(Power{Source})}`, removing one sibling. `Minus` is genuinely independent. Both deferred until a card demands them — naming the correct follow-up, not implementing it.

---
## Commit plan (both independently committable)
1. **Cosmic Cube** — parser-only (`oracle_effect/mod.rs` constraint combinators) + card enablement. Fast parser gate: combinator unit tests (A0/A1) + targeted runtime (A2) via `card-test`, then let Tilt continue. No engine/shared-type changes.
2. **Hawkeye** — engine field-lift (`DamageModification::Plus` → `QuantityExpr`) + resolver (`game/replacement.rs`) + parser arm + serde load-test + 4 regression-assertion migrations (oracle_replacement.rs:11783/11810/14026/14036). Touches shared `types/ability.rs` + `game/replacement.rs` → **full Rust verification** via Tilt resources (`clippy`, `test-engine`) before marking fixed; watch for other agents on those files and defer per the multi-agent-safety rule.

`cargo fmt --all` run directly; all other checks via `tilt logs` / `tilt-wait.sh`. After the Hawkeye lift, re-run `cargo engine-inventory`. Both sub-plans CR-annotated at every rules-bearing line (re-grep `docs/MagicCompRules.txt` per annotation — non-negotiable). No mtgish changes (dormant). No frontend/AI/MP surface for either card.
