# S07 — Celestial Reunion: behold-choose-a-creature-type cost subsystem

Plan produced via `/engine-planner`, structured on the `/add-interactive-effect` checklist
(this is a NEW interactive **cost-phase** subsystem) with `/casting-stack-conditions` for the
cost flow. **Read-only research done in `/home/lgray/vibe-coding/s07-impl-wt`.**

## Card (verified against `data/card-data.json` → `["celestial reunion"]`)

Mana cost `{X}{G}`. Oracle:

> As an additional cost to cast this spell, you may choose a creature type and behold two
> creatures of that type.
> Search your library for a creature card with mana value X or less, reveal it, put it into
> your hand, then shuffle. If this spell's additional cost was paid and the revealed card is
> the chosen type, put that card onto the battlefield instead of putting it into your hand.

Current status: `supported: null`, `parse_details: null` — fully red. **Root cause (corrected):**
there is **no parser at all** for the cost shape "choose a creature type and behold N creatures of
that type" — no combinator in `parser/oracle_cost.rs` emits an `AbilityCost` for it, so the cost
line never classifies. (`is_choose_behold_prefix` at `parser/oracle_casting.rs:141` matches a
*different* prefix — `choose a <type> you control or …` — and is neither the cause nor an obstacle;
its `" you control or "` `take_until` simply fails on this line.) Secondary gap: even once the cost
parses, no provenance path stores the chosen type for the resolution destination gate.

**Two corrections vs the batch brief:** the search filter is *creature card with mana value X or
less* (X = the cast's X), and the battlefield-instead gate is a **conjunction** of TWO conditions:
`additional cost was paid` **AND** `revealed card is the chosen type`.

---

## Analogous trace (engine-planner hard gate)

**Traced `AbilityCost::Behold` end-to-end** (the closest existing cost) plus the
`ChosenAttribute::CreatureType` / `FilterProp::IsChosenCreatureType` provenance pair and the
`SearchLibrary` continuation:

- Cost type: `types/ability.rs:6974` `AbilityCost::Behold { count, filter, action }`;
  `BeholdCostAction` at `6483`.
- Parser: `parser/oracle_cost.rs:216` `parse_behold_cost`, `:282` `parse_choose_or_reveal_behold_cost`;
  cost-line guard `parser/oracle_casting.rs:141` `is_choose_behold_prefix`.
- Cost payability + dispatch: `game/cost_payability.rs:557`; `game/casting_costs.rs:4085`
  (`AbilityCost::Behold` → `WaitingFor::PayCost { kind: PayCostKind::Behold }`, choices from
  `eligible_behold_choices`).
- Cost payment handler: `game/casting_costs.rs:1885` `handle_behold_for_cost`
  (sets `additional_cost_paid = true`, then `finish_pending_cost_or_cast`); engine entry
  `game/engine.rs:2085` (`PayCostKind::Behold` arm).
- Provenance write side (existing writers of `ChosenAttribute::CreatureType`):
  `game/engine_resolution_choices.rs:3860` (the `NamedChoice`/`ChooseOption` resolution handler,
  `persist` path pushes onto the source's `chosen_attributes`);
  option generation `game/effects/choose.rs:341` `compute_options` → `ChoiceType::CreatureType`
  arm at `:350` (uses `state.all_creature_types`).
- Provenance read side: `game/filter.rs:3716` (`FilterProp::IsChosenCreatureType` reads
  `source.chosen_creature_type` → `subtype_matches_with_changeling`); the `FilterContext` binds
  the *source* object via `FilterContext::from_ability` (`game/filter.rs:1635-1648`).
- Destination gate primitive: `types/ability.rs` `AbilityCondition::TargetMatchesFilter`
  (evaluated `game/effects/mod.rs:7857`, filter source = `ability.source_id` via
  `FilterContext::from_ability`) and `AbilityCondition::And`; the "cost paid" leg is the existing
  `AbilityCondition::AdditionalCostPaid` (`types/ability.rs:6075`), parser at
  `parser/oracle_effect/conditions.rs:491` ("if this spell's additional cost was paid, ").
- Search: `types/ability.rs:9435` `Effect::SearchLibrary`; resolver `game/effects/search_library.rs`
  (sets `WaitingFor::SearchChoice`); completion + continuation
  `game/engine_resolution_choices.rs:1845` (`SearchChoice` → moves found card via
  `pending_continuation` with the found card propagated as the continuation target).

### Destination-swap trace — From Father to Son + the `ConditionInstead` machinery (B-R1)

**Traced the cross-sentence "instead" destination-swap end-to-end.** Structural twin
**From Father to Son** (`data/card-data.json`): "Search your library for a Vehicle card, reveal it,
and put it into your hand. **If this spell was cast from a graveyard, put that card onto the
battlefield instead.** Then shuffle." **Measured status: `parse_details = null` — currently
UNSUPPORTED**, so it is a design reference, not a passing example.

- **Cross-sentence "instead" composer:** `parser/oracle.rs:3805-3813` (the `is_instead ||
  is_instead_replacement_line` block: guard `:3805`, `def.condition = ConditionInstead` `:3810`,
  `def.else_ability = base.sub_ability.take()` `:3813`) — pops the prior ability as `base`, sets
  `def.condition = ConditionInstead { inner }`, `def.else_ability = base.sub_ability.take()`,
  `base.sub_ability = def`. **This — not `lower.rs:808` — is the real search-destination/instead
  helper.** (`lower.rs:808` `attach_graveyard_redirect_rider_to_prior_cast_from_zone` is a
  `CastFromZone` graveyard-redirect rider, CR 614.1a / CR 608.2n — unrelated; the plan's earlier
  citation was wrong and is corrected below.)
- **Swap decision (the disqualifying fact):** the `ConditionInstead` swap is decided at the **top**
  of the parent's resolution (`game/effects/mod.rs:5476`, `should_swap = evaluate_condition(inner,
  state, ability)`) **before the parent effect runs**, and when it fires it REPLACES the parent's
  effect wholesale (`game/ability_utils.rs:149` `apply_instead_swap` → `overridden.effect =
  sub.effect.clone()`, line 154). Proven by the engine's own tests:
  `condition_instead_swaps_when_met` (`effects/mod.rs:13191`) asserts life **15** — the sub's 5
  damage *replaces* the parent's 2 (not 2+5), and `condition_instead_runs_base_chain_when_not_met`
  (`:13237`) asserts parent-runs-then-`else_ability`.
- **Why `ConditionInstead` is UNSUITABLE for Celestial Reunion (and for FFTS):**
  1. Celestial's destination condition includes **"the revealed card is the chosen type"** — a
     `TargetMatchesFilter` on the **found card**, which resolves against `ability.targets`' first
     object (`effects/mod.rs:7857-7867`, no trigger-source fallback during spell resolution). **The
     found card is not a target of the base `SearchLibrary` until AFTER the search resolves**
     (`engine_resolution_choices.rs:1954` injects it post-choice). So at the top-level swap check the
     condition is always **false** → the swap never fires.
  2. Even for a cast-context condition like FFTS's "cast from a graveyard" (decidable pre-search), if
     the swap fired it would **replace `SearchLibrary` with `ChangeZone→Battlefield` and destroy the
     search** — there is no found card yet to move. Fatal for any "always search, then conditionally
     change the destination" card.
  3. The not-fired path (`effects/mod.rs:6768` arm) stashes `sub.else_ability` (= `ChangeZone→Hand`)
     as the continuation, so the found card **always goes to hand** — the battlefield branch is
     unreachable.
  - **Proven adjacent analog (corrected framing).** The look/reveal-top → conditionally-onto-
    battlefield variant IS supported and shipping — **bison whistle, chaos warp, primal surge,
    scrying sheets** (`supported=true`) — via the `last_revealed_ids` injection at
    `effects/mod.rs:6905-6934` (the `effect_writes_last_revealed_ids` block) + **eager** condition
    eval. That path works for them precisely because their parent (Dig pure-peek / RevealTop /
    ExileTop) does **NOT** suspend: e.g. Dig's pure-peek branch (`dig.rs:139-155`,
    `raw_keep_count == 0 && !is_reveal`) writes `last_revealed_ids` and returns synchronously, so at
    the `:6934` eager check the revealed object is already injectable as the parent's target. **Only
    the SUSPENDING search-tutor variant is genuinely unproven** — a `SearchLibrary` that pauses at
    `WaitingFor::SearchChoice` has NOT performed a synchronous reveal, so the `:6905` `last_revealed`
    seam is empty at the eager check and cannot be reused (`SearchChoice` populates
    `last_revealed_ids` only on the player's response, `engine_resolution_choices.rs:1892`, after the
    eager check has already run). That is exactly why `SearchLibrary` needs the new
    `SearchChoice`-scoped deferral rather than the existing `last_revealed` eager seam — the minimal
    correct addition. (Consistent with: **zero** `supported==true` cards use the *suspending*
    search→battlefield-instead shape; Celestial Reunion is that seed.)
- **The mechanism that DOES work (measured):** on continuation resume, `resolve_ability_chain` on
  the stashed `cont.chain` evaluates that node's **top-level** `condition` (`effects/mod.rs:5827`)
  and runs its `else_ability` when false (`:5830-5846`), with `cont.chain.targets` = the found card
  (set at `engine_resolution_choices.rs:1954`). So the destination gate must be a **plain
  conditional node that is the search continuation head** — see Resolution below.

---

## Provenance contract for the chosen creature type (the load-bearing decision)

| Aspect | Decision |
|---|---|
| **Where stored** | `ChosenAttribute::CreatureType(String)` pushed onto the **spell object's** `chosen_attributes` (`GameObject` at `pending_cast.object_id` — the spell on the stack). Do NOT invent a parallel `chosen_creature_type` cost field; reuse the exact object slot every existing "choose a creature type" card uses. |
| **When bound** | During additional-cost payment, in the new `WaitingFor::CostTypeChoice` handler, **before** the behold selection is computed (so behold eligibility already sees the type). |
| **Lifetime** | Lives on the spell `GameObject` for as long as the spell is on the stack — through cost payment and through resolution (CR 608.2; CR 400.7d confirms costs/choices remain referenceable while the object exists). Read twice: once during cost (behold), once during resolution (destination gate). No expiration handling needed — object leaves the stack after resolution and the slot dies with it. |
| **Live vs snapshot** | Live read of the source object each time (both `eligible_behold_choices` and `TargetMatchesFilter` read `source.chosen_creature_type` fresh). No latching. |
| **Who reads (1)** | Cost step: `eligible_behold_choices(state, player, pending.object_id, filter)` where `filter = creature + IsChosenCreatureType`. `IsChosenCreatureType` reads the spell's chosen type → "behold two creatures **of that type**". |
| **Who reads (2)** | Resolution: destination gate `AbilityCondition::TargetMatchesFilter { filter: creature + IsChosenCreatureType }` — `FilterContext::from_ability` binds the filter source to the conditional ability's `source_id` = the spell object → reads the same chosen type against the found card's subtypes. |
| **Multi-authority hostile fixture** | Two candidate creature cards findable (one Elf, one Goblin); chosen type = Elf. Proves the gate reads THIS spell's stored type, not "any creature". Plus a decline path (cost not paid) proving the `AdditionalCostPaid` leg. |

Because both reads route through the existing `IsChosenCreatureType` + `FilterContext` machinery
with source = the spell object, **the only new provenance code is the single write** in the
`CostTypeChoice` handler. This is the minimal wiring; no new storage type.

---

## Shared feasibility primitive — `feasible_behold_creature_types` (closes B1 + B2)

**The single load-bearing new function.** Both the payability probe (B1) and the option list (B2)
MUST be computed by one authority so they can never disagree — an unpayable type must never be
offered (CR 601.2h: an unpayable cost can't be paid), and a payable cost must always surface a
choice (else `finish_pending_cost_or_cast`'s `Optional{Once}` arm silently removes it —
`casting_costs.rs:641-644`).

**Why this is required (verified):**
- `AbilityCost::Behold::is_payable` = `eligible_behold_choices(..filter..).len() >= count`
  (`cost_payability.rs:557-560`). With the behold `filter` carrying `FilterProp::IsChosenCreatureType`
  but **no type chosen yet** (`source.chosen_creature_type == None`), `IsChosenCreatureType` matches
  nothing (`filter.rs:3716` returns `false` on `None`; `:4160` likewise) → `eligible = 0` →
  `is_payable = false` → the `Optional{Once}` arm at `casting_costs.rs:641-644` **removes the cost and
  recurses**, so the player is never prompted, `additional_cost_paid` stays false, and the subsystem
  is dead. B1 fix = payability must ask "**does there exist** a creature type T with ≥count beholdable
  creatures of T", not "are there ≥count beholdable creatures of the (unchosen) type".
- `compute_options(state, &CreatureType, ..)` returns **all** `state.all_creature_types` unfiltered
  (`choose.rs:349-358`). If offered as-is, a player can pick a type they hold <count of; resolution
  then recomputes `eligible_behold_choices` with that type and hits `if choices.len() < count { Err
  (ActionNotAllowed) }` (`casting_costs.rs:4091-4095`) → a reachable engine error via normal play,
  violating CR 601.2h. B2 fix = the option list must equal the feasible-type set.

```rust
// game/filter.rs (sibling to the private `subtype_matches_with_changeling` authority it needs;
// `eligible_behold_choices` is already `pub(crate)` in casting_costs.rs and callable from here).
/// CR 701.4a + CR 205.3m + CR 601.2h: the creature types for which the player can
/// actually pay "choose a creature type and behold N of that type" — types T such
/// that ≥ `count` beholdable creatures (hand + controlled battlefield permanents)
/// are of type T (Changeling counts as every type, CR 702.73a). Single authority
/// feeding BOTH the Optional-cost payability probe (B1: set non-empty) AND the
/// `CostTypeChoice` option list (B2: the set itself), so the offered options and the
/// payability gate can never disagree.
pub(crate) fn feasible_behold_creature_types(
    state: &GameState,
    player: PlayerId,
    source: ObjectId,
    behold_filter: &TargetFilter, // the stored Behold.filter (creature + IsChosenCreatureType)
    count: u32,
) -> Vec<String> {
    // 1. Gather beholdable candidates against the BASE creature filter — the same
    //    filter with the `IsChosenCreatureType` leg removed, since that leg is the
    //    per-type discriminator this fn enumerates (with it present + no type chosen,
    //    `eligible_behold_choices` would return empty — the exact B1 trap).
    let base = behold_filter.without_prop(&FilterProp::IsChosenCreatureType); // small clone, prop stripped
    let candidates =
        super::casting_costs::eligible_behold_choices(state, player, source, &base);
    // 2. Keep each creature type with ≥count candidates of that type (changeling ⇒ all).
    state
        .all_creature_types
        .iter()
        .filter(|t| {
            candidates
                .iter()
                .filter(|&&id| {
                    state.objects.get(&id).is_some_and(|o| {
                        subtype_matches_with_changeling(
                            t,
                            &o.card_types.subtypes,
                            &o.keywords,
                            &state.all_creature_types,
                        )
                    })
                })
                .count()
                >= count as usize
        })
        .cloned()
        .collect()
}
```

- **`behold_filter.without_prop(..)`** — **verified absent** (grep for `without_prop` / `fn without_`
  over `crates/engine/src/` finds only unrelated `without_paying` helpers). Add a trivial
  `TypedFilter::without_prop(&self, prop) -> TargetFilter` that clones and retains `properties != prop`.
  (If another agent lands an equivalent filter-clone-minus-prop helper first, reuse it — do not
  hand-roll a second one.) This keeps callers passing the stored cost filter unchanged — one source of
  truth for "beholdable creature".
- **Callers (both, no other logic duplicated):**
  - **B1 — `cost_payability.rs:557` Behold arm** becomes:
    ```rust
    AbilityCost::Behold { count, filter, type_choice, .. } => match type_choice {
        // existing fixed-quality behold: candidates of the fixed filter must exist.
        None => eligible_behold_choices(state, player, source, filter).len() >= *count as usize,
        // pre-choice behold: payable iff SOME creature type is feasible (∃).
        Some(_) => !filter::feasible_behold_creature_types(state, player, source, filter, *count).is_empty(),
    },
    ```
  - **B2 — `casting_costs.rs` cost dispatch** builds `CostTypeChoice.options` from the *same* call
    (see Phase 2 below). Because the option list ⊆ feasible types and payability = "feasible set
    non-empty", the resolution error at `casting_costs.rs:4091-4095` is now **unreachable via normal
    play** (kept only as defense-in-depth) — every offered type has ≥count candidates, so
    `eligible_behold_choices` with that chosen type returns ≥count.

---

## Cost model

Extend the existing `AbilityCost::Behold` with one optional, **typed** pre-choice field — do NOT
add a sibling cost variant (satisfies "Parameterize, don't proliferate"; the behold's "of that
type" IS the chosen type, so the choice belongs on the behold cost):

```rust
// types/ability.rs — AbilityCost::Behold
Behold {
    #[serde(default = "default_one")]
    count: u32,
    filter: TargetFilter,
    action: BeholdCostAction,
    /// CR 601.2b + CR 701.4a: when Some, the player first chooses a value of this
    /// kind (creature type for Celestial Reunion) as part of paying the behold
    /// cost. The choice is recorded as a `ChosenAttribute` on the spell; the behold
    /// `filter`'s `IsChosenCreatureType` leg then scopes "of that type". None = the
    /// existing fixed-quality behold (Monstrous Emergence, Close Encounter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    type_choice: Option<ChoiceType>,
}
```

- `Option<ChoiceType>` (not a bool) reuses the existing choice-kind enum; a future
  "choose a color and behold N of that color" card reuses it with `ChoiceType::Color` + an
  `IsChosenColor` filter leg. Class-covering by construction.
- All existing `Behold` construction sites (`oracle_cost.rs:253`, `:319`; `costs.rs:1412`, `:1523`)
  add `type_choice: None`. `#[serde(default)]` keeps `card-data.json` deserialization churn-free.

The additional cost as a whole is `AdditionalCost::Optional { cost: Behold{ count:2,
filter: creature+IsChosenCreatureType, action: ChooseOrReveal, type_choice:
Some(ChoiceType::CreatureType) }, repeatability: Once }` — "you may".

### Parser (nom-only, no verbatim match)

`parser/oracle_cost.rs`: add `parse_choose_type_and_behold_cost(lower)` tried alongside the
existing behold parsers (called from the same dispatch that reaches `parse_behold_cost` /
`parse_choose_or_reveal_behold_cost`, ~`:432`). Compose combinators — one `alt()` per axis, no
permutation expansion:

```
tag("choose ")
alt((tag("a "), tag("an ")))
tag("creature type")                         // (axis: ChoiceType::CreatureType)
tag(" and behold ")
parse_number_or_x  (or alt((tag("a "),tag("an ")))→1)   // count = 2
tag(" creatures of that type")               // → filter creature + IsChosenCreatureType
```

On success emit `AbilityCost::Behold { count, filter: TypedFilter::creature()
.properties(vec![FilterProp::IsChosenCreatureType]).into(), action: ChooseOrReveal,
type_choice: Some(ChoiceType::CreatureType) }`.

`parser/oracle_casting.rs`: `is_choose_behold_prefix` is unrelated to this line (it matches a
different prefix, `choose a <type> you control or `; its `" you control or "` `take_until` fails
here). It is neither the current cause of redness nor an obstacle — leave it untouched. The line goes
GREEN purely because the new combinator now emits a `Behold`; no regression to the existing guard.
Confirm ordering: the new `parse_choose_type_and_behold_cost` is tried within the same dispatch that
reaches the other behold parsers and, on match, returns before any honest-deferral path.

The additional-cost line ("As an additional cost to cast this spell, you may ...") already routes
`choose ... behold ...` through the behold cost parser; the `you may` → `AdditionalCost::Optional`
wrapping is existing behavior — verify by re-parsing after the parser change.

---

## Interactive round-trip (`/add-interactive-effect` Phases 1-7)

The new choice happens during **cost payment**, so it needs a cost-phase `WaitingFor` carrying the
`pending_cast` (mirrors `BlightChoice` / `OptionalCostChoice`), resumed into
`finish_pending_cost_or_cast` — NOT into a resolution `pending_continuation`.

**Phase 1 — WaitingFor + GameAction**
- `types/game_state.rs` `WaitingFor`: add
  ```rust
  CostTypeChoice {
      player: PlayerId,
      choice_type: ChoiceType,   // CreatureType here
      options: Vec<String>,
      pending_cast: Box<PendingCast>,
  },
  ```
  Also add it to: the variant name map (`game_state.rs:4629` area), the `player` extraction match
  (`:4765`), and the `pending_cast` extraction matches (`:4885`, `:4917`).
- `GameAction`: **reuse `GameAction::ChooseOption { choice: String }`** — the response shape is a
  string creature type, identical to `NamedChoice`. No new `GameAction` variant.

**Phase 2 — cost dispatch (not an effect resolver)**
- `game/casting_costs.rs` `AbilityCost::Behold` arm (`:4085`, extend the destructure to bind
  `type_choice`): when `type_choice` is `Some(_)`, return
  ```rust
  WaitingFor::CostTypeChoice {
      player,
      choice_type: ct,
      options: filter::feasible_behold_creature_types(state, player, pending.object_id, filter, count),
      pending_cast,
  }
  ```
  **before** computing behold choices. **The option list is the B2 fix — the feasible-type set, NOT
  `compute_options`'s unfiltered `all_creature_types`.** (`compute_options` stays the authority for
  every *resolution-time* creature-type choice; it is simply the wrong list here because a behold
  pre-choice must exclude types the player can't satisfy.) When `type_choice` is `None`, keep the
  existing `PayCost { Behold }` path unchanged.
- **B1 payability (`game/cost_payability.rs:557`)** is now correct-by-construction via the shared
  helper: `Some(_) => !feasible_behold_creature_types(..).is_empty()`. This is the fix, not a
  deferral — do **not** rely on the "you may" Optional wrapper to save an empty behold, because the
  `Optional{Once}` arm at `casting_costs.rs:641-644` *removes* a non-payable optional cost silently
  and the choice is never offered (the exact dead-subsystem bug B1). The `None` branch keeps the
  existing specific-filter payability.

**Phase 3 — engine handler**
- `game/engine.rs` (or `engine_resolution_choices.rs` cost section): add
  `(WaitingFor::CostTypeChoice { player, choice_type, options, pending_cast },
  GameAction::ChooseOption { choice })`:
  1. Validate `choice` ∈ `options` (mirror the `NamedChoice` validation at
     `engine_resolution_choices.rs:3452`).
  2. **Write provenance**: push `ChosenAttribute::CreatureType(choice)` onto the spell object
     `state.objects.get_mut(&pending_cast.object_id).chosen_attributes` (replace any prior
     `CreatureType` to be re-choose-safe, mirroring existing chosen-attribute writers).
  3. Resume cost payment: continue the behold step for the same `pending_cast` — i.e. run the
     `Behold` branch's `eligible_behold_choices` + return `WaitingFor::PayCost { Behold }` (factor
     the behold-choices half of the `casting_costs.rs:4085` arm into a helper both the dispatch and
     this handler call, so eligibility is computed once, now with the chosen type set). The behold
     completion (`handle_behold_for_cost`, `:1885`) already sets `additional_cost_paid = true` and
     calls `finish_pending_cost_or_cast`.

**Phase 4 — AI legal actions (two seams — measured)**
- **Primary enumerator (load-bearing): `crates/engine/src/ai_support/candidates.rs:1236`.** Add a
  `WaitingFor::CostTypeChoice { player, choice_type, options, pending_cast }` arm returning one
  `GameAction::ChooseOption { choice }` per **feasible** option — mirror the `NamedChoice` arm at
  `:1236` (call `named_choice_actions(state, player, options, choice_type, pending_cast.object_id)`;
  `source_id` = the spell). `candidate_actions` (`:2773`) additionally appends `CancelCast` for any
  `has_pending_cast` state — legal (CR 601.2e) and harmless; the AI weighs `ChooseOption` vs cancel.
- **Fallback safety net: `crates/phase-ai/src/search.rs` `fallback_action` (the exhaustive `match
  &state.waiting_for`, no `_ =>`, ending in the `CancelCast` OR-group at `~:1444-1461`).** Adding
  `CostTypeChoice` forces a compile error here — good. **Measured correction to the review note:** do
  NOT give it a live `ChooseOption` arm like `NamedChoice` at `:923`. `fallback_action`'s guard at
  `:666` (`if state.waiting_for.has_pending_cast() { return CancelCast }`) fires **before** the
  match, and `has_pending_cast()` = `pending_cast_ref().is_some() || ManaPayment | Phyrexian`
  (`game_state.rs:4956`). Because `CostTypeChoice` carries `pending_cast` (added to
  `pending_cast_ref` in Phase 1), the guard already returns `CancelCast` for it, so a `ChooseOption`
  arm would be **dead code**. Correct placement: add `CostTypeChoice` to the pending-cast
  `CancelCast` OR-group at `~:1444-1461` (alongside `BlightChoice` / `OptionalCostChoice` / `PayCost` — all
  pending-cast states). `NamedChoice` gets a live arm at `:923` only because it is *not* a
  pending-cast state. The live `ChooseOption` action for `CostTypeChoice` comes from the primary
  `candidates.rs` enumerator above; `fallback_action` is a debug-panicking safety net that must not
  be reached during a well-gated cast.

**Phase 5 — multiplayer routing**
- Player routing is centralized in `game_state.rs` (`acting_player` in `server-core/src/session.rs`
  delegates to the `WaitingFor` `player` match). Adding `CostTypeChoice { player, .. }` to that
  match (Phase 1) is sufficient — no separate `session.rs` arm needed. Verify no explicit
  per-variant arm exists in `session.rs` that would need updating.

**Phase 6 — frontend**
- `client/src/adapter/types.ts`: add the `CostTypeChoice` `WaitingFor` variant (tsify-generated;
  add manual override only if generation misses it). `ChooseOption` `GameAction` already exists.
- Render: **reuse `NamedChoiceModal`** — it already renders a creature-type picker for
  `choice_type: "CreatureType"` and dispatches `ChooseOption`. Route
  `waitingFor.type === "CostTypeChoice"` to the same modal (same `choice_type` + `options` shape).
- **i18n (CI parity gate):** the modal title is keyed by `CHOICE_TYPE_TITLE_KEYS` /
  `namedChoice.title.*` which already has a creature-type leaf. If a distinct
  "additional cost" prompt string is added, it MUST be added to **all 7 locales**
  (`client/src/i18n/locales/*/game.json`) or `resources.test.ts` parity fails CI. Prefer reusing
  the existing creature-type title key to avoid new chrome strings entirely.

**Phase 7 — hidden-info filtering:** behold-from-hand reveals are already handled by the existing
`Behold` visibility path (`game/visibility.rs:546`); the creature-type choice is public. No new
`filter_state_for_player` work.

---

## Resolution

The main clause is a standard tutor plus a conditional destination:

```rust
Effect::SearchLibrary {
    filter: creature card + ManaValue ≤ X,   // "with mana value X or less"
    count: QuantityExpr::fixed(1),
    reveal: true,                            // "reveal it"
    split: None,                             // single destination, handled downstream
    // source_zones = default (library only), target_player = None, selection_constraint = default
}
```

**N2 — no double-move (required).** `Effect::SearchLibrary` searches + reveals only; it does **not**
place the found card (its doc: *"The destination is handled by the sub_ability chain (ChangeZone +
Shuffle)"* — `types/ability.rs:9433-9434`; it has no destination field). This is exactly what we
need: the "put it into your hand … put onto battlefield instead" swap is the **continuation's**
if/then/else `ChangeZone`, not a SearchLibrary side-effect. If SearchLibrary auto-placed to hand,
the conditional `ChangeZone` would then move an already-moved card and the "then shuffle" ordering
would break. The found card is chained as the continuation's object target — verified: after
`SearchChoice`, `chosen` ids become `cont.chain.targets` as `TargetRef::Object` and are propagated
through the search→shuffle chain (`engine_resolution_choices.rs:1954-1966`); `source_id` stays the
spell. **Chain order (single tutor line):** SearchLibrary → conditional `ChangeZone` (destination
decided by the And gate below) → `Shuffle` — so shuffle runs after placement, matching "put …, then
shuffle." Parse "with mana value X or less" via the existing MV-comparator filter combinator (verify
the tutor + reveal + shuffle line already parses; only the cost line and the conditional were red).

**Conditional destination — the ONE final AST (B-R1 resolved).** The destination gate is a **plain
conditional `ChangeZone` node `N` that becomes the search continuation head** (`cont.chain`), NOT a
`ConditionInstead` swap (proven unsuitable — see the Analogous trace: the swap is decided pre-search
and would destroy the search / stash the hand branch). `N` is `SearchLibrary`'s `sub_ability`:

```rust
// N — the conditional destination node (search continuation head; cont.chain after selection).
// CR 608.2c: later text ("...put onto the battlefield instead of putting it into your hand")
// modifies the earlier destination — read the whole text.
AbilityDefinition {
    effect: Effect::ChangeZone { destination: Zone::Battlefield, target: <found card>, .. }, // then-branch
    condition: Some(AbilityCondition::And { conditions: vec![
        AbilityCondition::additional_cost_paid_any(),                      // cost-paid leg (ctor → AdditionalCostPaid{subject:Source, min_count:1, ..})
        AbilityCondition::TargetMatchesFilter {                            // chosen-type leg, on the found card
            filter: TypedFilter::creature().properties(vec![FilterProp::IsChosenCreatureType]).into(),
            use_lki: false,
        },
    ]}),
    else_ability: Some(Box::new(C)),   // "instead of putting it into your hand" → default dest = Hand
    sub_ability:  Some(Box::new(Shuffle)),   // "then shuffle" runs after the battlefield placement
    ..
}
// C — the else (default) chain: ChangeZone{ destination: Hand, target: <found card> } .sub_ability(Shuffle)
```

- This is a **plain `And`**, deliberately NOT wrapped in `ConditionInstead`: the general condition/
  else path at `effects/mod.rs:5827` (evaluate) + `:5830-5846` (else) handles it, and — unlike the
  `ConditionInstead` swap — it does **not** replace or pre-empt `SearchLibrary`. The "instead of
  putting it into your hand" trailing clause is consumed as `N.else_ability = ChangeZone→Hand` (the
  otherwise/default destination), so there is exactly one AST — no separate instead-swap node.
- Both legs read the spell object: `AdditionalCostPaid { Source }` reads
  `ability.context.additional_cost_paid_matches` (`effects/mod.rs:7492`), which
  `apply_parent_chain_context` copies onto `N` from the base at stash time; `TargetMatchesFilter`
  binds filter source = spell via `FilterContext::from_ability` and reads the stored
  `ChosenAttribute::CreatureType` against the found card's subtypes.

**B-R1 engine wiring — defer the result-dependent conditional so `N` becomes the continuation head.**
Measured trap: a conditional sub of an interactive `SearchLibrary` is **eagerly** evaluated at
`effects/mod.rs:6934` (`evaluate_condition(condition, state, base)`) — at that point the base has no
found-card target, so `TargetMatchesFilter` is false → `else`(hand) always, battlefield unreachable
(the same end-state defect the reviewer flagged for plain-`And`, via the eager-eval path rather than
the `6737` instead-list). Fix: extend the existing interactive-parent **deferral** at
`effects/mod.rs:6890-6893` (currently gated to `condition_depends_on_effect_performed ||
condition_depends_on_zone_change_this_way || WhenYouDo`, all under a `waits_for_resolution_choice`
guard) with one more disjunct so a condition that depends on the parent's **suspended-selection
result object** (the found/revealed card) is stashed **with its condition** via
`prepend_to_pending_continuation` instead of eagerly evaluated. **The new disjunct MUST be scoped to
`WaitingFor::SearchChoice` specifically — NOT the generic `waits_for_resolution_choice` guard**
(BLOCKING #1 fix; see the safety proof below):

```rust
// effects/mod.rs — new building-block predicate (mirrors condition_depends_on_effect_performed:2016
// and condition_depends_on_zone_change_this_way:2037; recurse And/Or/Not).
// CR 608.2c: a "that card / the revealed card"-referential gate can't be evaluated until the
// parent's selection (search) injects the result as the continuation target.
fn condition_depends_on_result_object(c: &AbilityCondition) -> bool {
    match c {
        AbilityCondition::TargetMatchesFilter { .. } => true,
        AbilityCondition::Not { condition } => condition_depends_on_result_object(condition),
        AbilityCondition::And { conditions } | AbilityCondition::Or { conditions } =>
            conditions.iter().any(condition_depends_on_result_object),
        _ => false,
    }
}
// ...at the 6890 gate — the NEW disjunct is gated on SearchChoice, the ONLY choice whose
// completion injects the result object as `cont.chain.targets` (engine_resolution_choices.rs:1965).
if waits_for_resolution_choice(&state.waiting_for)
    && (condition_depends_on_effect_performed(condition)
        || condition_depends_on_zone_change_this_way(condition)
        || matches!(condition, AbilityCondition::WhenYouDo)
        || (matches!(state.waiting_for, WaitingFor::SearchChoice { .. })   // NEW — SearchChoice-scoped
            && condition_depends_on_result_object(condition)))
{ /* stash sub WITH condition via prepend_to_pending_continuation */ }
```

**BLOCKING #1 — why `SearchChoice`-only, not the generic `waits_for_resolution_choice` guard
(code-verified).** The broad form (OR'ing `condition_depends_on_result_object` directly onto the
`waits_for_resolution_choice` gate) introduces a **latent, forward-looking** mis-deferral: it routes
*any* suspended-choice parent whose sub carries a `TargetMatchesFilter` condition through
`prepend_to_pending_continuation`, but only `SearchChoice` completion actually injects the result
object as `cont.chain.targets`. A `jq` sweep of `data/card-data.json` found **no shipping
`supported=true` card** exhibiting the guarded shape (a suspended `Effect::Sacrifice` →
`EffectZoneChoice` with a `TargetMatchesFilter`-gated `SequentialSibling` sub), so the broad form
breaks **no live card today** — it is a booby-trap for the next card of that shape. Narrowing to
`SearchChoice` is the minimal-correct scope that disarms it before it can fire. The mechanics:
- **`waits_for_resolution_choice` includes `EffectZoneChoice`** (`effects/mod.rs:1873`), which
  `Effect::Sacrifice` of N-of-M permanents raises (`sacrifice.rs:306`). The conditional-sub region has
  **no `sub_link` guard**, so a `SequentialSibling` `TargetMatchesFilter`-gated sub of a suspended
  `Sacrifice` would flow through the `:6890-6893` gate.
- **Illustrative shape — `grave choice`** (measured `supported: null`, `parse_details: null` — it does
  **NOT** parse: there is no parser path for its "conjure a duplicate of it" clause; `ConjureSource::
  Duplicate` exists as a type at `types/ability.rs:7917` but nothing routes Oracle text to it). It is
  used here **only as an illustration** of the vulnerable AST shape, not as a card any test can cast.
  Its Oracle shape *would* AST as `Sacrifice{count:1, target: Creature controller:TargetPlayer}` → sub
  `Conjure{duplicate_of: ParentTarget}`, `condition: TargetMatchesFilter{Cmc LE 2}`, `sub_link:
  SequentialSibling`; when the opponent controls ≥2 nontoken creatures, `Sacrifice` raises
  `EffectZoneChoice`. Under the broad disjunct that sub would re-route to
  `prepend_to_pending_continuation` — but **`EffectZoneChoice`/Sacrifice completion does NOT inject the
  sacrificed creature as `cont.chain.targets`**; it stores the sacrificed ids in
  `tracked_object_sets`/`chain_tracked_set_id` (`engine_resolution_choices.rs:3343-3361`, the
  `ParentTarget` mechanism) and drains. On resume `resolve_ability_chain(N)` would evaluate
  `TargetMatchesFilter` against an **empty** `cont.chain.targets` → `false` → wrong Conjure/destination
  behavior.
- **The deferral only WORKS because of the `SearchChoice` result-injection at
  `engine_resolution_choices.rs:1965`** (`cont.chain.targets = continuation_targets` = the found cards,
  propagated `:1966`); for `EffectZoneChoice`/`DigChoice`/etc. it is both unnecessary and unsafe.
  Scoping the new disjunct to `matches!(state.waiting_for, WaitingFor::SearchChoice { .. })` leaves the
  broad `Sacrifice`→TMF shape on the existing eager `:6934` path (its `EffectZoneChoice` fails the
  `SearchChoice` match; a `TargetMatchesFilter` condition is not one of the three existing disjuncts,
  so no existing leg is affected either). If a future choice variant also injects a result object into
  the continuation, add it to an explicit allowlist only after verifying it injects — default to
  `SearchChoice`-only.

Scope is otherwise safe: the gate already requires `waits_for_resolution_choice(waiting_for)`, so a
non-suspending parent that carries a resolved `TargetMatchesFilter` target (e.g. the `Bounce`+`Draw`
test at `effects/mod.rs:11607`) never enters this branch. After the stash, `cont.chain = N`;
`SearchChoice` completion sets `cont.chain.targets = [found card]`
(`engine_resolution_choices.rs:1965`); on drain, `resolve_ability_chain(N)` evaluates `N.condition`
at `5827` with the found card as target → then (`ChangeZone→Battlefield`) / else
(`ChangeZone→Hand`). This is the general "**search**, then conditionally act on the found card"
building block — it unlocks From Father to Son and the whole tribal-tutor destination-swap class, not
just Celestial Reunion. The three existing disjuncts are byte-behavior-unchanged: their
regression guards `if_you_do_gate_...` (`effects/mod.rs:10416`), `when_you_do_...` (`:11773`/`:11833`/
`:11912`), and `optional_discard_if_you_do_...` (`:18774`/`:18873`) still pass, because the new
`SearchChoice && condition_depends_on_result_object` leg cannot fire for any of their states
(EffectZoneChoice / embedded-cost / DiscardChoice) or condition shapes.

**N1 — conjunction parser (required).** The parser must emit the plain-conditional `N` above and
must **not** fall through to the `oracle.rs:3805` auto-`ConditionInstead` composer (which would
produce the unsuitable swap). The existing `strip_additional_cost_conditional` (`conditions.rs:437`)
only matches the *standalone* form: its arm at `conditions.rs:491` is
`alt((tag("if this spell's additional cost was paid, "), …))`, which demands `", "` **immediately
after** "paid" — it will NOT match "…paid **and** the revealed card is the chosen type, ". Add a new
arm **before** that `alt` (its failure falls through to the generic `tag("if ")` branch at `:502`, so
ordering only needs the new arm reached first). Exact nom sequence (combinators only — no verbatim
`==`, no `contains`/`starts_with`):

```rust
// conditions.rs — new leading arm in strip_additional_cost_conditional, via nom_on_lower.
// CR 608.2c + CR 205.3m: "if this spell's additional cost was paid AND the revealed
// card is the chosen type, <body>" → plain And{ AdditionalCostPaid, TargetMatchesFilter{chosen type} }.
if let Some(((), rest)) = nom_on_lower(text, &lower, |i| {
    let (i, _) = tag("if this spell's additional cost was paid and ").parse(i)?;
    let (i, _) = parse_revealed_card_is_chosen_type(i)?;   // predicate leg, below
    let (i, _) = tag(", ").parse(i)?;
    Ok((i, ()))
}) {
    return (
        Some(AbilityCondition::And { conditions: vec![
            AbilityCondition::additional_cost_paid_any(),   // reuse existing ctor (:461, :478)
            AbilityCondition::TargetMatchesFilter {
                filter: TypedFilter::creature()
                    .properties(vec![FilterProp::IsChosenCreatureType]).into(),
                use_lki: false,
            },
        ]}),
        rest.to_string(),
    );
}
```

The returned **plain `And`** is attached as `N.condition` when the tutor line and the destination
clause are composed into the `SearchLibrary → N(then=Battlefield, else=Hand) → Shuffle` chain (the
composition mirrors `oracle.rs:3805-3813` but emits a plain `And` on the `ChangeZone→Battlefield`
node instead of `ConditionInstead`, and sets `else_ability = ChangeZone→Hand`). **This is the single
reconciliation:** the earlier draft both "returned a plain `And`" and "folded the instead via
`lower.rs:808`" — both are dropped. There is now ONE AST: the plain-`And` conditional `N` with
`else_ability = Hand`; the trailing "instead of putting it into your hand" IS the `else_ability`
(the otherwise-destination), consumed here, not a second swap node and not a `lower.rs:808` rider
(that region is the unrelated `CastFromZone` graveyard rider, CR 614.1a / CR 608.2n).

The **predicate leg** `parse_revealed_card_is_chosen_type` recognizes the copular/PREDICATE form
`"the revealed card is the chosen type"` — distinct from the adjectival `"of the chosen type"`
combinator (`oracle_target.rs:2476-2529`, which parses a suffix on a noun phrase). The classifier
already lists `"is the chosen type"` as a known context fragment (`oracle_classifier.rs:318`), so this
is a recognized clause shape, not a one-off. Compose it, don't string-match:

```rust
// small combinator (predicate form): subject "the revealed card" + copula "is" + "the chosen type".
fn parse_revealed_card_is_chosen_type(i: &str) -> IResult<&str, (), OracleError<'_>> {
    value((),
        (tag("the revealed card"), tag(" is "), tag("the chosen type")),
    ).parse(i)
}
```

**Base-type trap (kept):** emit the chosen-type filter with a **creature base** (subtype-bearing),
NOT a card-type base — an `IsChosenCreatureType` prop on a card-type base never matches at runtime
(`oracle_target.rs:2497-2520`, which flips to `IsChosenCardType` only
for card-typed bases); a creature base is correct because the found object is a creature card whose
subtypes carry the type.

---

## `behold` sub-effect

Behold is already modeled end-to-end as a **cost** (`AbilityCost::Behold`,
`handle_behold_for_cost`). The only delta is the dynamic type scoping, delivered entirely by
(a) the new `type_choice` pre-choice and (b) the `IsChosenCreatureType` filter leg. No behold
resolution/effect changes; "behold two creatures of that type" is `count: 2` + the chosen-type
filter, and `handle_behold_for_cost` already validates count and reveals/chooses.

---

## CR annotations (grep-verified against `docs/MagicCompRules.txt`)

- **CR 701.4 / 701.4a** — Behold: "Reveal a [quality] card from your hand or choose a [quality]
  permanent you control on the battlefield." (line 3293/3295) — the behold cost + `type_choice`.
- **CR 601.2b** — during announcement, "If the value of that variable is defined ... by a choice
  that player would make later ... that player makes that choice at this time instead" (line 2459)
  — the creature-type choice is made during casting. Primary rule for the cost-phase choice.
- **CR 601.2f** — total cost includes additional costs (line 2468); **CR 601.2h** — pay the total
  cost (line 2472) — the additional-cost payment flow.
- **CR 701.20 / 701.20a** — Reveal (line 3436/3438) — "reveal it".
- **CR 701.23** — Search (line 3463) — the tutor.
- **CR 205.3m** — creature types are the shared subtype list (line 1439) — the chosen-type domain
  (`all_creature_types`) and `IsChosenCreatureType` matching.
- **CR 400.7 / 400.7d** — zone change → new object; a permanent may reference the costs/choices of
  the spell that became it (line 1950/1958) — provenance lifetime + the found card becoming a
  battlefield permanent.

(All line numbers from the local `docs/MagicCompRules.txt`. "Behold" is CR 701.4, not a 701.2x
number — verified; no hallucinated number written.)

---

## Discriminating gate test (acceptance criterion)

`card-test` style: `GameScenario` + `GameRunner::cast(...).resolve()`, asserting `CastOutcome`
deltas and the real cost round-trip. Set `state.all_creature_types = ["Elf","Goblin"]`.

**Setup:** caster library contains one **Elf** creature card and one **Goblin** creature card,
both MV ≤ X; hand contains two Elf creature cards (to behold). Cast Celestial Reunion with X large
enough to find either.

1. **Round-trip (positive):** cast → assert `WaitingFor::CostTypeChoice { choice_type:
   CreatureType, .. }` is reached through the real cast path (not hand-built). Submit
   `GameAction::ChooseOption { choice: "Elf" }` → assert `WaitingFor::PayCost { Behold }` with the
   two Elves offered. Behold both → assert `additional_cost_paid`. Search → choose the **Elf** card
   → **assert it enters the battlefield** (not hand).
2. **Negative sibling A (type mismatch):** same, but search chooses the **Goblin** card → assert it
   goes to **hand** (chosen type Elf, found not-Elf → `TargetMatchesFilter` leg false).
3. **Negative sibling B (cost declined):** decline the Optional additional cost at
   `OptionalCostChoice` → search finds even the **Elf** card → assert **hand** (`AdditionalCostPaid`
   leg false). This exercises the empty/decline path.
4. **Non-vacuity proof (case 1 routes through the `ConditionInstead`-free continuation-head path).**
   Case 1's positive arm (found chosen-type Elf → **battlefield**) is only reachable because `N` is
   deferred to the search continuation head and its `And` re-evaluates post-selection with the found
   card as target. Each of the following reverts independently flips case 1 from battlefield to hand
   (record each as a revert-failing assertion, proving the assertion is discriminating, not vacuous):
   - Removing the new `SearchChoice`-scoped `condition_depends_on_result_object` disjunct on the
     `6890` deferral gate → eager pre-search eval at `6934` → `else`(hand). (Proves the wiring is
     load-bearing.)
   - Reverting the provenance write (handler step 2) → chosen type unset → `TargetMatchesFilter`
     false → hand.
   - Dropping the `TargetMatchesFilter` (chosen-type) leg of the `And` → gate never distinguishes
     type → case A (Goblin) would also go to battlefield (case A's assertion fails).
   - Dropping the `AdditionalCostPaid` leg → case 3 (declined) would go to battlefield (case 3's
     assertion fails).
   Case A isolates the type leg; case B (decline) isolates the cost leg — each leg independently
   discriminating. (Four independent reverts flip case 1 — non-vacuous.)
5. **Deferral-scope non-regression (BLOCKING #1 guard — proves the disjunct is `SearchChoice`-scoped,
   not broad).** This is an **engine-level AST/deferral-scope test**, deliberately **NOT** a
   "byte-unchanged supported card" test: a `jq` sweep of `card-data.json` shows **no** `supported=true`
   card ships the vulnerable shape today, so the guard targets a **latent/forward-looking**
   mis-deferral, not a live shipping-card break. (grave choice is `supported: null`/unparsed and cannot
   be cast — it is only the shape illustration in BLOCKING #1.) Construct the shape directly with
   `GameScenario`: build an ability whose parent `Effect::Sacrifice` of N-of-M permanents (so it raises
   `EffectZoneChoice`, `sacrifice.rs:306`) has a `SequentialSibling` sub gated by
   `AbilityCondition::TargetMatchesFilter{..}` (e.g. a `ChangeZone`/marker whose destination depends on
   the gate reading `cont.chain.targets`), put it on the stack so `Effect::Sacrifice` raises
   `EffectZoneChoice`, drive that choice to completion, and assert the sub's behavior/destination is
   **identical** with vs without the new disjunct. **Revert-failing:** under the *broad* form (OR'ing
   `condition_depends_on_result_object` onto the generic `waits_for_resolution_choice` gate) the sub
   re-routes through `prepend_to_pending_continuation`; because `EffectZoneChoice`/Sacrifice completion
   never sets `cont.chain.targets` (`engine_resolution_choices.rs:3343-3361` uses the
   tracked-set/`ParentTarget` path, not target injection), the re-evaluated `TargetMatchesFilter` reads
   **empty** targets → the sub's destination changes and the assertion fails. Under the
   `SearchChoice`-scoped form the sub is **unchanged** (its `EffectZoneChoice` fails the
   `matches!(SearchChoice)` guard → stays on the eager `:6934` path). This is the single BLOCKING-#1
   discriminator. (If a genuinely `supported=true` card with this exact
   Sacrifice→`EffectZoneChoice`→TMF-gated-`SequentialSibling` shape later exists — verify against
   `card-data.json` first — it may be substituted as a cast-level fixture; grave choice is NOT one.)
6. **Whiff / fail-to-find (empty `SearchChoice` selection).** Cast Celestial Reunion, pay the cost
   (chosen type Elf), but the library contains **no legal creature card with MV ≤ X** (or submit an
   empty `SelectCards` under the search's `allows_partial_find`, `engine_resolution_choices.rs:1861`).
   Assert: `cont.chain.targets = []` (no found card injected) → `N`'s `TargetMatchesFilter` leg is
   `false` → the else (`ChangeZone→Hand`) runs with no object → no card moves, **no fire against a
   stale/absent result object, no panic**, then `Shuffle`. With the `SearchChoice`-narrowed deferral
   this path is clean (the eager `:6934` path would behave equivalently, but the deferred path must
   also not dereference an absent found card — this test proves it doesn't).
7. **AI test:** `legal_actions` on `WaitingFor::CostTypeChoice` returns one `ChooseOption` per
   **offered (feasible)** creature type — i.e. it mirrors `options`, not the full type catalog.
8. **B2 feasibility restriction (no reachable error state):** with the same setup — hand holds two
   **Elf** cards and **zero** beholdable Goblins, `all_creature_types = ["Elf","Goblin"]`, behold
   `count = 2` — assert the reached `WaitingFor::CostTypeChoice.options == ["Elf"]` and that
   **`"Goblin"` is NOT offered** (only 0 < 2 beholdable Goblins → excluded by
   `feasible_behold_creature_types`). This proves B2: a player can never pick an unpayable type, so
   the `casting_costs.rs:4091` `Err(ActionNotAllowed)` is unreachable via normal play. Discriminating:
   reverting the option list to `compute_options` (unfiltered `all_creature_types`) reintroduces
   `"Goblin"` and this assertion fails. Complementary payability check: with hand holding **only**
   Goblins (0 beholdable Elves too — e.g. empty hand of the chosen classes), assert the Optional
   additional cost is offered-and-declinable but never silently dropped when *some* feasible type
   exists, and that with **no** feasible type the cost is correctly non-payable (declined path, not a
   crash).

Verification cadence (per CLAUDE.md): `cargo fmt --all`; then Tilt `clippy` + `test-engine` +
`card-data` (coverage flips Celestial Reunion GREEN — confirm no coverage regression on other
behold/search cards via the card-data diff).

---

## New engine surface — `/add-engine-variant` justification

Consulted the traced surface (`cargo engine-inventory` to be re-run at implementation time for a
live existence check). Proposed additions:

1. **`AbilityCost::Behold.type_choice: Option<ChoiceType>` (new FIELD, not a variant).**
   Parameterization axis = "value chosen before beholding", within the single CR 701.4 behold
   rule. Typed `Option<ChoiceType>` reuses an existing enum; not a bool. Extends `Behold` rather
   than adding a sibling `BeholdChosenType` cost — passes the parameterization filter and the
   categorical-boundary check (stays in CR 701.4).

2. **`WaitingFor::CostTypeChoice` (new variant).** Justified: no existing `WaitingFor` models a
   creature-type choice made *during cost payment* carrying `pending_cast` for resumption into
   `finish_pending_cost_or_cast`. `NamedChoice` is categorically resolution-time (resumes a
   `pending_continuation`), so reusing it would route cost payment through the resolution
   continuation — wrong seam. This mirrors the existing cost-phase interactive variants
   (`BlightChoice`, `OptionalCostChoice`, `PayCost`) that each carry `pending_cast`. `WaitingFor` is
   UI/turn state, not a rules enum.

3. **`filter::feasible_behold_creature_types(state, player, source, behold_filter, count) ->
   Vec<String>` (new pub(crate) FUNCTION, not an enum variant).** The single authority for "which
   creature types can actually pay a choose-and-behold cost." Lives in `game/filter.rs` (sibling to
   the private `subtype_matches_with_changeling` it needs; calls the already-`pub(crate)`
   `casting_costs::eligible_behold_choices`). Feeds **both** B1 payability (`is_payable` = set
   non-empty) and B2 the `CostTypeChoice` option list (the set) — one computation, so options and
   payability cannot diverge (CR 601.2h). Not a one-off: any future "choose a subtype and behold N of
   that type" reuses it; the color/land variants get sibling `feasible_behold_*` fns or a
   `ChoiceType`-parameterized generalization at that time. Supporting helper:
   `TypedFilter::without_prop` (add only if no equivalent filter-clone-minus-prop helper exists).

4. **`effects::condition_depends_on_result_object` (new private FUNCTION + one **`SearchChoice`-scoped**
   disjunct on the `effects/mod.rs:6890-6893` deferral gate).** Required because no existing machinery
   resolves a found-card-dependent destination swap (measured: `ConditionInstead` replaces the parent
   effect and is decided pre-search — tests `13191`/`13237`; and a plain-`And` sub is eagerly evaluated
   pre-search at `6934`; the suspending search→battlefield-instead shape has zero supported cards). This
   predicate (mirroring `condition_depends_on_effect_performed:2016`, annotated `CR 608.2c` — the
   read-the-whole-text / anaphora rule this region uses pervasively) defers a conditional sub of a
   *suspended interactive parent* so it becomes the continuation head and re-evaluates post-selection.
   **The disjunct is gated on `matches!(state.waiting_for, WaitingFor::SearchChoice { .. })`, NOT the
   generic `waits_for_resolution_choice` guard** — because only `SearchChoice` completion injects the
   result object as `cont.chain.targets` (`engine_resolution_choices.rs:1965`); `EffectZoneChoice`
   (raised by `Effect::Sacrifice`) instead uses the `tracked_object_sets`/`ParentTarget` mechanism
   (`:3343-3361`) and would leave `cont.chain.targets` empty, silently mis-deferring the vulnerable
   Sacrifice→`TargetMatchesFilter`-gated-`SequentialSibling` shape (illustrated by **grave choice**,
   itself `supported: null`/unparsed — a latent/forward-looking hazard, not a live break; jq sweep
   confirms no `supported=true` card ships it today). General building block for
   the "search, then conditionally act on the found card" class. **No new `AbilityCondition` variant**
   — reuse `And` + `AdditionalCostPaid` + `TargetMatchesFilter`.

5. **No new `GameAction`** — reuse `ChooseOption`. **No new `FilterProp`** — reuse
   `IsChosenCreatureType`. **No new `ChosenAttribute`** — reuse `CreatureType`. **No new `Effect`**
   — reuse `SearchLibrary` + conditional `ChangeZone` + `Shuffle`. The destination gate is a plain
   `AbilityCondition::And` node (NOT `ConditionInstead`) evaluated by the general condition/else path
   (`effects/mod.rs:5827`/`:5830`), so no `ConditionInstead` dispatch change is needed.

---

## Architectural sections (engine-planner)

- **Pattern coverage:** the class is "choose a [subtype] as an additional cost and behold N of that
  type, then gate a downstream effect on the chosen type." Celestial Reunion is the seed; the
  `Option<ChoiceType>` + `IsChosen*` filter design generalizes to color/land-type behold variants
  and to any "revealed/found card is the chosen type" destination gate (a recurring
  tribal-tutor shape). Not a one-off: the cost extension, the `CostTypeChoice` round-trip, and the
  `And{AdditionalCostPaid, TargetMatchesFilter}` gate are each independently reusable primitives.
- **Building blocks:** `compute_options` (option gen), `eligible_behold_choices` +
  `handle_behold_for_cost` (behold), `finish_pending_cost_or_cast` (cost sequencing),
  `FilterContext::from_ability` + `IsChosenCreatureType` (provenance read), `AbilityCondition::And`
  / `TargetMatchesFilter` / `AdditionalCostPaid` (gate), `SearchLibrary` + `ChangeZone` + `Shuffle`
  (resolution), the general condition/else continuation path (`effects/mod.rs:5827`/`:5830`) + the
  interactive-parent deferral (`:6890`) (destination gate on resume). New code is ~one parser
  combinator, one cost-dispatch branch, one WaitingFor + handler, one AI arm, one frontend route, and
  one deferral predicate (`condition_depends_on_result_object`) — everything else composes.
- **Logic placement:** parser (cost-line + conditional recognition) in `parser/`; cost dispatch +
  provenance write + resolution gate in `game/`; types in `types/ability.rs` /
  `types/game_state.rs`. Zero frontend logic beyond routing to an existing modal.
- **Nom compliance:** the new cost parser is pure combinators (`tag`/`alt`/`parse_number_or_x`), one
  `alt()` per axis, no `contains`/`starts_with`/`find`. Detection = the parser returning `Some`.
- **Extension vs creation:** extends `AbilityCost::Behold`, reuses the entire chosen-type provenance
  pair and the general condition/else + continuation machinery; creates only the unavoidable
  cost-phase `WaitingFor` and one general deferral predicate. The destination gate is a plain `And`
  (NOT `ConditionInstead`) — measured: the `ConditionInstead` swap replaces the parent effect
  (destroys the search) and is decided pre-search, so it cannot model "always search, conditionally
  place"; no existing "battlefield instead" card ships, so this class is built here as a reusable
  primitive rather than reused from an unsuitable path.
- **Verification matrix:** see the discriminating gate test — every behavioral claim (type-leg,
  cost-leg, round-trip, AI) has a production-path test, a revert-failing assertion, and a
  negative/decline sibling. Parser: the cost line is only accepted when the full combinator matches
  (creature base emitted); a partial match falls through to honest deferral, keeping coverage honest.

## Plan path
`/home/lgray/vibe-coding/s07-impl-wt/.planning/coverage-analysis/S07-CELESTIAL-REUNION-PLAN.md`
