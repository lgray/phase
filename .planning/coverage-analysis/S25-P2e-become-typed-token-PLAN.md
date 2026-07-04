# P2e PLAN — "Become a typed token" (Vraska, the Silencer + Brilliance Unleashed)

> Driver: s25-impl-revived. Planner: p2e-planner (Opus/xhigh, read-only). Status: **APPROVE-WITH-CONDITIONS** (review aa616e09) — conditions + riders folded below.
> Base: HEAD bd3c72197 on c2c0162b1. HARD BAR (never edit): game/effects/delayed_trigger.rs, game/filter.rs, game/effects/mod.rs.

## ⚑ REVIEW OUTCOME — APPROVE-WITH-CONDITIONS (executor MUST satisfy each)

Verdict: diagnosis sound, zero-new-variants approach correct, parser claims confirmed at cited lines. Conditions gate the runtime binding + duration + CR.

**C1 — Vraska runtime bind must be PROVEN before any "supported" flip.** Step-0 probe (or the runtime test) must show the copula's continuous effect lands on the *returned creature's id* — NOT Vraska (source, via `use_self` misbind targeting.rs:667-668) and NOT nowhere (inert). If the probe shows inert/misbound, DO NOT flip Vraska supported; report to me. This is the block-D hollow-win trap in a new location.

**C2 — If Vraska is inert, the fix is a NON-FROZEN `effect.rs` arm, not `effects/mod.rs`.** Reviewer traced: the moved-object snapshot IS already threaded to the copula sub-ability via `apply_parent_chain_context(&mut resolved, ability, effect_context_object.as_ref(), state)` (mod.rs:6763/6792). The missing piece is *consumption*: add a `ParentTarget`/empty-`ability.targets` arm to `register_transient_effect` that binds `SpecificObject{ effect_context_object.object_id }`, mirroring the existing `cost_paid_object` arm (effect.rs:426-439). **Budget `crates/engine/src/game/effects/effect.rs` as an in-scope (non-frozen) touch — the plan's §2 file list is incomplete.** Escalate to the FROZEN owner (s07) ONLY if the probe shows `effect_context_object` is NOT populated for this chain shape (reviewer's read: it IS populated → likely no frozen edit). Same coordination pattern as fix#2 if it does break frozen.

**C3 — Duration = `UntilHostLeavesPlay`, NOT `Permanent`, for the reanimate copula (BOTH cards).** Overrides plan Q5. Precedent: `install_aura_continuous_effect` hard-codes `UntilHostLeavesPlay` (return_as_aura.rs:247) with CR 611.2a + CR 400.7 ("a new object on re-entry is not the same object"). Because ObjectId persists (zones.rs:125), `Permanent` + `SpecificObject{id}` risks wrong re-application on leave-and-return.

**C4 — Confirm Brilliance clause decomposition BEFORE implementing the Block-2 walk.** The "walk past the paired conditional to reach `Choose target artifact card`" mechanism is correct ONLY if that Choose clause is a *separate prior* ClauseIr. If it folds INTO the conditional return clause, the typed referent sits ON the conditional → skipping it skips the referent → the fix must instead READ the paired conditional's own target. Step-0 probe must pin this (three outcomes in Q6).

**C5 — Guard relaxation must NOT regress the second consumer.** `chain_has_prior_typed_referent` has two live callers: mod.rs:22545 (`parent_target_available`) and mod.rs:23148. Gate any line-14998 relaxation behind a `skip_first_conditional`/`is_otherwise` parameter (default `false`) so 23148's behavior is byte-for-byte unchanged for non-otherwise contexts.

**C6 — CR fix: the indefinite copula is CR 611.2a (no stated duration → lasts until game ends), NOT 611.2b** (611.2b = "for as long as" conditional duration). Combined with C3's `UntilHostLeavesPlay`, annotate the copula TCE: CR 611.2a + CR 400.7.

**C7 — Vraska runtime test MUST assert on the RETURNED creature's id specifically:** core_types == [Artifact], carries `Treasure`, exposes the mana ability, AND activating it sacrifices *that object* (not Vraska). This catches the `use_self`-misbind-to-Vraska *wrong-object* hollow win, distinct from the *no-object* inert case.

**DRIVER RIDERS (from reviewer's missed-risk notes):**
- **R1 — SECOND frozen dependency to check: `game/filter.rs:4707` (FROZEN)** reads `effect_context_object` during object matching. Low probability (`SpecificObject{id}` matching is direct, not filter-routed), but the executor's probe MUST confirm the copula's `ParentTarget`/`SpecificObject` match does NOT route through a filter path. If it does → second frozen gap (the exact two-gap pattern that bit block-D) → STOP + escalate to me.
- **R2 — the plan's `change_zone.rs:1178` anchor is MISLEADING** (that's the `ChangeZoneAll` mass-move player-scope resolver, resolves a PlayerId — not the single-object return path). The real runtime path is `register_transient_effect` (effect.rs) + `parent_referent_context_from_events`/`moved_object_context_from_events` (mod.rs:1178/1301). Do not rely on the change_zone.rs anchor.
- **R3 — `parse_its_a_type_loses_others` only fires with the "…loses all other card types" tail** (become_copy_except.rs:743-749). Vraska has the tail (matches). Brilliance has NO tail ("3/3 Robot…flying") → correctly routes to the animation arm, not this helper. Executor must not conflate the two paths.
- **R4 — FULL `cargo test -p engine`** (lib + integration, NOT --lib) per the P2f/block-D stale-pin lesson. Flip the std_longtail_e.rs:23-24 Vraska deferred note when Vraska lands.
- **R5 — SEQUENCING: P2e impl runs AFTER block-D commits.** mod.rs edit sites (14998/22545) are collision-SAFE vs block-D's hunks (~76/~847-951/~6322-6435), but `tests.rs` overlaps (block-D added ~161 lines) → coordinate the rebase there. No two impl streams on the tree at once (team-lead constraint).

## Executive summary
Both cards need the same lowering primitive: a copula clause **"It's a `<typed thing>` …"** applied to a **returned/reanimated non-copy object**, as an indefinite (`Duration::Permanent`) continuous effect bound to that object (`ParentTarget`). The runtime for this shape already exists and is proven — this is a **parser composition, zero new engine variants, zero new resolvers**. The novelty is making the copula clause *reach* the returned object across (a) the copy-path-only `parse_its_a_type_loses_others` boundary (Vraska), and (b) a conditional/`Otherwise` boundary (Brilliance ∩ Bre).

Runtime de-risk (kills the hollow-win concern up front):
- `zones.rs:125` — "ObjectId here is **storage identity and persists across the zone change**." So `ParentTarget` (bound to the pre-move object id) still reaches the object *after* it returns to the battlefield. CR 400.7 "new object" is modeled by attribute reset, not id churn.
- `game/effects/effect.rs:29-35` — a "becomes"/copula `GenericEffect` injects `Duration::Permanent` at parse time and `register_transient_effect` installs it as a TCE. This is the exact indefinite-duration path both cards need.
- `game/effects/return_as_aura.rs:223-252` — an existing **non-copy reanimate** (`return X, it's an Enchantment/Aura`) installs `SetCardTypes` + subtype + granted-ability mods as one TCE on the returned object. Proves Vraska's shape works at runtime with the exact modification kinds.

---

## 1. Per-card: Oracle text, measured gap, target lowering

### Vraska, the Silencer (Group C — reviewed-before-code)
Oracle (grep-confirmed, `data/card-data.json` key `"vraska, the silencer"`):
> Deathtouch
> Whenever a nontoken creature an opponent controls dies, you may pay {1}. If you do, return that card to the battlefield tapped under your control. **It's a Treasure artifact with "{T}, Sacrifice this artifact: Add one mana of any color," and it loses all other card types.**

Measured status: **honestly RED today** — explicitly deferred at `crates/engine/tests/std_longtail_e.rs:23-24` ("Vraska the Silencer (return-as-Treasure-with-ability — heavy continuous-self-modification infra)"). The trigger head + optional `pay {1}` + `return that card … tapped under your control` (dies-trigger anaphor → `ChangeZone`, cf. Yedora path `change_zone.rs:1051`) are established shapes. The **dropped clause is the bolded copula sentence**: it produces `Effect::Unimplemented` because `parse_its_a_type_loses_others` (the `SetCardTypes`+quoted-ability+lose-types builder) is reachable **only** from the copy path (`become_copy_except.rs:209`, its sole caller), not the subject-copula path.

Target lowering (matches the working analog `return_as_aura`):
- Head: existing `Effect::ChangeZone` (graveyard→battlefield, `enter_tapped`, `enters_under_player=You`), target = the dying-card anaphor.
- Follow-up copula: `Effect::GenericEffect { target: Some(ParentTarget), duration: Some(Permanent), static_abilities: [ SetCardTypes{core_types:[Artifact]}, AddSubtype{"Treasure"}, GrantAbility{ "{T}, Sacrifice this artifact: Add one mana of any color" } ] }`. All three modifications are exactly what `parse_its_a_type_loses_others` (`become_copy_except.rs:731-786`) already emits for Shelob; the granted ability routes through `classify_quoted_inner` → `GrantAbility` (`oracle_static/keyword_grant.rs:1608,1644`), identical representation to Shelob's Food-sac grant.

CR: **205.1a** (set card type replaces existing) + **205.1b** (subtype-set semantics for "artifact") + **613.1d** (Layer 4 type-change) + **611.2a** (no stated duration → lasts until game ends) + **400.7** (returned object is a new object; use `UntilHostLeavesPlay` per C3) + **603.6** (zone-change trigger looks for the object in the zone it moved to; the returned object is what the copula modifies). **Do NOT cite 707.9d for Vraska** — grep-confirmed (`docs/MagicCompRules.txt`) that 707.9d is scoped to *copy effects* ("When applying a copy effect …"); Vraska is a non-copy reanimate, so 707.9d does not apply here even though the master plan lists it. (707.9d remains correct for the copy-path caller.)

### Brilliance Unleashed (mode 2)
Oracle (grep-confirmed, key `"brilliance unleashed"`):
> Choose one or both —
> • Brilliance Unleashed deals 5 damage to target creature.
> • Choose target artifact card in your graveyard. Return it to the battlefield if it's an artifact creature card. **Otherwise, return it to the battlefield and it's a 3/3 Robot artifact creature with flying.**

Measured status: audit-GREEN was **shallow** (master plan §0 A2). Mode 1 and the mode-2 *if* branch are fine. The **dropped clause is the bolded `Otherwise` animation**. Evidence: the committed snapshot test `anaphoric_return_then_animation_honest_defers_when_no_parent_referent` (`snapshot_tests.rs:758-787`) proves the else fragment lowers to `Effect::Unimplemented` when parsed without a referent in scope, and the honest-bind gate at `subject.rs:342-347` declines the animation unless the subject resolves to `ParentTarget`. In the full modal card the `Choose target artifact card` establishes a typed referent, **but** `chain_has_prior_typed_referent` (`mod.rs:14996-15021`) **returns false at the first conditional clause it hits** (line 14998: `if prev.condition.is_some() { return false; }`). The `Otherwise` chunk pairs with the conditional `Return it … if it's an artifact creature card`, so the referent walk bails at that conditional → `parent_target_available=false` → the else animation declines → `Unimplemented`. Net: the card parses "supported" while the reanimated object is never actually animated = the hollow-win the mandate forbids.

Target lowering: the `Otherwise` else_ability = `Effect::ChangeZone` (return) + `Effect::GenericEffect{ target:ParentTarget, duration:Permanent, static_abilities:[ SetPower{3}, SetToughness{3}, AddType{Creature}, AddType{Artifact}, AddSubtype{"Robot"}, AddKeyword{Flying} ] }` — the **existing animation arm** (`subject.rs:323-363`, proven by `snapshot_tests.rs:709-750` for the non-modal case). Brilliance needs **no** new copula arm; it needs only the else-branch referent inheritance. CR: **205.1b** (artifact-creature type addition, retains prior types) + **613.1d** (Layer 4) + **613.4/Layer 7b** (base P/T set) — all grep-confirmed.

---

## 2. Files to touch (dependency order) — NONE in the s07-frozen set

| # | File | Function / site | Block |
|---|------|-----------------|-------|
| 1 | `crates/engine/src/parser/oracle_effect/become_copy_except.rs` | `parse_its_a_type_loses_others` (`:731`) → change `fn` to `pub(super) fn` (or hoist its post-article core into a shared helper) | 1 (Vraska) |
| 2 | `crates/engine/src/parser/oracle_effect/subject.rs` | `try_parse_contracted_subject_additive_type_clause` (`:287`) — add a THIRD arm after the additive check (`:306`), reusing #1, gated on `ParentTarget` exactly like the animation arm (`:342`), building a `PredicateAst::Become` with `Duration::Permanent` | 1 (Vraska) |
| 3 | `crates/engine/src/parser/oracle_effect/mod.rs` | `parent_target_available` derivation (`:22544`) and/or `chain_has_prior_typed_referent` (`:14996`): for an `is_otherwise` chunk, continue the referent walk **past** the paired conditional clause | 2 (Brilliance ∩ Bre) |
| 3b | `crates/engine/src/parser/oracle_effect/mod.rs` | referent helpers (`has_typed_target_widened` `:14946` / the return-clause publish) — **only if the executor probe (step 0) shows Vraska's copula binds to `TriggeringSource`/`SelfRef` instead of `ParentTarget`** | 3 (Vraska reanimate publish) |
| 4 | `crates/engine/src/parser/oracle_ir/effect_chain.rs` | `ChunkKind::Otherwise`/`is_otherwise` (`:68-71,151`) — alternative/complementary home for Block 2 if cleaner in the chunk loop | 2 |

**Frozen-set confirmation:** none of files 1-4 are in `{game/effects/delayed_trigger.rs, game/filter.rs, game/effects/mod.rs}`.

**Cross-driver dependency (flag):** the runtime chain-propagation that carries the returned object into the sub-ability's `ParentTarget` lives in `game/effects/mod.rs::resolve_ability_chain` — **FROZEN**. Analysis says no edit is needed there (ObjectId persistence + existing `GenericEffect(ParentTarget)` path already bind the target case at runtime). **If** the executor's runtime probe shows the anaphoric-return case does not thread the returned id to `ParentTarget`, that is a **cross-driver dependency on the s07 owner of `effects/mod.rs`** — do NOT edit it; escalate to the driver.

**Collision (flag):** block-D has **uncommitted** edits in `oracle_effect/mod.rs`, `oracle_effect/tests.rs`, `oracle_target.rs`. Block 2/3 edits touch `oracle_effect/mod.rs` and tests touch `oracle_effect/tests.rs`. The executor must sequence **after** block-D commits (or coordinate a rebase); do not stack uncommitted-on-uncommitted.

---

## 3. New enum variants: ZERO
**Explicit "zero new variants" statement.** add-engine-variant 3-stage gate not triggered — every needed variant exists:
- Existence (grep-confirmed): `ContinuousModification::{SetCardTypes (ability.rs:17434), AddSubtype, AddType (17317), GrantAbility (17364), SetPower/SetToughness, AddKeyword}`; `Effect::{ChangeZone, GenericEffect, Animate (9291)}`; `AbilityDefinition::else_ability (ability.rs:13948)`; `ChunkKind::Otherwise (effect_chain.rs:69)`.
- Parameterize: N/A (no sibling variant proposed).
- Categorical boundary: N/A.
No `data/engine-inventory.json` sibling-cluster check needed since nothing is being added. (Planner did not read/regenerate that file — regeneration would be a tree write.)

---

## 4. Shared-block design (else-branch, Brilliance ∩ Bre)

The `Otherwise` → `else_ability` machinery **already exists** (`effect_chain.rs:68-71`, `subject.rs:2338` reads `ctx.parent_target_available`). What is missing is **referent inheritance across the conditional**: an `Otherwise` chunk is the else of a conditional clause, and its anaphor ("it") must bind to the referent that was in scope **before** that conditional.

Design (single surgical change, `mod.rs`): when deriving `parent_target_available` for a chunk that `is_otherwise`, run `chain_has_prior_typed_referent`/`chain_prior_referent_is_chosen_target` starting **from the clause before the paired conditional**, i.e. do not let the guard at `mod.rs:14998` (`prev.condition.is_some() → false`) bail on the *matched* conditional. Keep the guard for *unrelated* conditionals (do not walk past a second, non-paired conditional).

How each consumer benefits with the one change:
- **Brilliance:** else walk skips the `Return it … if it's an artifact creature card` conditional → reaches the `Choose target artifact card` typed referent → `parent_target_available=true` → the existing animation arm binds `ParentTarget` → 3/3 Robot.
- **Bre of Clan Stoutarm (C12, other driver):** `exile … until you exile a nonland card` (`ExileFromTopUntil{NextMatches}`, already whitelisted `mod.rs:14882/14973`) → `You may cast that card if MV ≤ life gained` (conditional) → `Otherwise, put it into your hand`. Same guard-bail today; same fix makes the else `put it into your hand` (`ChangeZone{Hand, ParentTarget}`) bind to the exiled card. Bre consumes the identical block with **no card-specific code** — the definition of build-for-the-class here.

Vraska is deliberately **not** in this shared block: it is not an else-branch, it is an anaphoric-return case (Block 1 + conditional Block 3b). Keep them separate to avoid coupling.

---

## 5. CR annotations (grep-verified against `docs/MagicCompRules.txt`)
- **205.1a** (set card type replaces prior types; instant/sorcery retained) — Vraska `SetCardTypes`. Verified (line 1398).
- **205.1b** ("in addition to its other types"/"artifact creature" retains prior types) — Brilliance animation + Vraska subtype set. Verified.
- **613.1d** (Layer 4 type-changing) — both. Verified.
- **611.2a** (no stated duration → lasts until game ends) + **400.7** (returned object is a new object on re-entry) — the copula TCE, installed as `Duration::UntilHostLeavesPlay` per C3 (NOT 611.2b, NOT `Permanent`). Verified.
- **603.6** (zone-change trigger looks for the object in the zone it moved to) — Vraska modifies the returned object. Verified.
- **707.9d** — grep-confirmed **copy-effect scoped**; DO NOT annotate Vraska's non-copy reanimate with it. Valid only on the existing copy-path caller.

---

## 6. Test plan (non-vacuous, revert-to-red)

**Step 0 (executor, before coding): parse probe.** In a clean tree (post-block-D-commit), run the parser on both full Oracle strings and record the exact `Effect::Unimplemented`/dropped clause(s) and whether Vraska's copula subject resolves to `ParentTarget` vs `TriggeringSource`/`SelfRef`. This pins whether Block 3b is needed. (Planner could not run it — tree is dirty; this replaces static inference with ground truth.)

### Vraska
- **Parser round-trip** (`crates/engine/tests/std_longtail_e.rs`, flip the deferred note): parse full Oracle → `assert_zero_unimplemented`; assert the trigger effect chain contains a `GenericEffect`/Become sub-ability with `target==Some(ParentTarget)`, `duration==Some(Permanent)`, and modifications including `SetCardTypes{core_types==[Artifact]}`, `AddSubtype{"Treasure"}`, and a `GrantAbility`. **Revert proof:** revert the Block-1 arm in `subject.rs` → the copula sentence drops to `Effect::Unimplemented` → `assert_zero_unimplemented` fails.
- **Runtime** (card-test recipe, `GameScenario` + cast/resolve): opponent nontoken creature dies; controller pays {1}; assert the returned object is on `Battlefield`, controlled by P0, `tapped==true`, `card_types.core_types == [Artifact]` (NOT Creature), carries subtype `Treasure`, and exposes an activated mana ability. **Revert proof:** revert Block 1 → returned object retains its creature types / no Treasure mana ability → the `core_types==[Artifact]` and ability-presence assertions fail (discriminating: distinguishes real type-set from an inert return).

### Brilliance
- **Parser round-trip** (new test near `snapshot_tests.rs:702` region, full modal card): parse full Oracle → `assert_zero_unimplemented`; walk mode-2 `else_ability` and assert its animation sub-ability has `target==Some(ParentTarget)` with `SetPower{3}`, `AddSubtype{"Robot"}`, `AddKeyword{Flying}`. Keep the existing `anaphoric_return_then_animation_honest_defers…` test green (isolated fragment still has no referent → still defers; the Block-2 fix only fires when a pre-conditional referent exists). **Revert proof:** revert Block 2 (`mod.rs` else-referent walk) → the else animation returns to `Effect::Unimplemented` → `assert_zero_unimplemented` fails.
- **Runtime** (modal cast test): a **non-creature** artifact card (e.g. an Equipment) in graveyard; cast Brilliance, choose mode 2 targeting it; assert it returns to the battlefield as a creature with `power==Some(3)`, subtype `Robot`, and `Flying`. Second case: an artifact-creature card returns as-is (if-branch, no animation). **Revert proof:** revert Block 2 → returned object stays a non-creature artifact (`power==None`, no Robot/Flying) → the `power==Some(3)` assertion fails (discriminating: the inert-return hollow win is exactly `power==None`).

Parser round-trip alone is insufficient per the mandate — both runtime tests assert the reanimated object's *characteristics*, which are inert if the continuous effect never binds.

---

## 7. Risks / unknowns / open questions for the driver

1. **[Probe-gated] Block 3b necessity (Vraska).** Static read says the anaphoric return's target lowers to `TriggeringSource` (not `Typed`), so `has_typed_target_widened` (`mod.rs:14946`) returns false and the copula gate (`subject.rs:342`, requires `ParentTarget`) declines — meaning Vraska needs Block 3b (publish the returned object as `ParentTarget`) in addition to Block 1. Step-0 probe confirms/refutes. If refuted (referent already reaches), Vraska is Block 1 only.
2. **[Frozen-file risk] Runtime `ParentTarget` rebind for anaphoric returns.** If the returned-object id is not threaded into `ability.targets`/`ParentTarget` for the sub-ability at runtime (`change_zone.rs:1178` resolves `ParentTarget` from `ability.targets`), the fix would require logic in **`game/effects/mod.rs` (FROZEN)**. Escalate rather than edit. The target case (Brilliance) is lower-risk because the chosen target *is* in `ability.targets`.
3. **[Collision] block-D uncommitted edits** to `oracle_effect/mod.rs` + `oracle_effect/tests.rs` overlap Block 2/3 + tests. Sequence after block-D commits.
4. **"this artifact" self-reference (Vraska's granted sac ability).** Confirm `classify_quoted_inner` lowers `"{T}, Sacrifice this artifact: …"` so "this artifact" self-sacrifices the granted-to object (Shelob uses "Sacrifice ~"). Low risk (same GrantAbility path) but verify the anaphor.
5. **Duration on the copula TCE vs re-entry.** `Duration::Permanent` on a `SpecificObject{id}` — since ObjectId persists, a leave-and-return could theoretically re-apply; `return_as_aura` chose `UntilHostLeavesPlay` for this. Open question: reanimate copula use `Permanent` (matches the existing animation arm) or `UntilHostLeavesPlay` (matches `return_as_aura`)? Planner leans `Permanent` for parity with the proven `snapshot_tests.rs:709` path; flag for reviewer.
6. **Scope check:** two cards, ~2 small parser arms + 1 shared referent fix, zero engine variants, zero resolvers. If Step-0 probe shows Brilliance already binds (i.e. the conditional-guard-bail read is wrong), Brilliance collapses to a test-only flip — do not build Block 2 speculatively; the probe decides.

### Critical files
- crates/engine/src/parser/oracle_effect/subject.rs
- crates/engine/src/parser/oracle_effect/become_copy_except.rs
- crates/engine/src/parser/oracle_effect/mod.rs
- crates/engine/src/game/effects/return_as_aura.rs
- crates/engine/tests/std_longtail_e.rs
