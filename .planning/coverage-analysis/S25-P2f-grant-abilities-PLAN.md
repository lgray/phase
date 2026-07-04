# S25 P2f — Grant abilities/keywords to an object — IMPLEMENTATION PLAN

**Planner:** P2f (xhigh). **Scope:** Symbiote Spider-Man + Choreographed Sparks (BOTH ship — no-defer tranche rule).
**Working tree:** `/home/lgray/vibe-coding/s25-impl-wt` (branch `feat/std-s25-completion`).
**Status:** review-ready. Read-only investigation; this doc is the only write.

All file:line citations below were grep/read-verified in this tree on 2026-07-02.

---

## STEP 0 — Measured residual (premise verification)

### Tranche premise (S25-PLAN-FINAL.md B3, lines 65/166/214/225)
> `Effect::GainActivatedAbilitiesOfTarget` exists; resolver `gain_activated_abilities.rs:79` filters `.filter(|a| a.kind == AbilityKind::Activated)` → activated-only. Symbiote's "other abilities" includes a triggered ability → dropped. Fix: parameterize with `GrantedAbilityScope { ActivatedOnly | AllOther }`.

### Verdict: **PREMISE TRUE — but materially INCOMPLETE.** Three measured additions the tranche premise omits:

1. **The triggered ability is not even in the store the resolver reads.** The variant doc (`types/ability.rs:17338-17356`, `GrantAllTriggeredAbilitiesOf`) states: activated abilities live in `obj.abilities` (CR 602.1) → `GrantAbility`; **triggered abilities live in a *separate* store `obj.trigger_definitions` (CR 603.1) → `GrantTrigger`.** Confirmed: `GameObject.trigger_definitions: Definitions<TriggerDefinition>` (`game/game_object.rs:189,446`). Symbiote's combat-damage trigger is parsed into the card's `triggers` array (measured: `jq .["symbiote spider-man"].triggers` = one `DamageDone` trigger), i.e. it lands in `trigger_definitions`, **not `abilities`**. So removing the `.filter(activated)` is *not sufficient* — `donor.abilities.iter()` will never see the trigger. `AllOther` must **also iterate `donor.trigger_definitions` and emit `GrantTrigger`**. This is the substantive resolver change; the tranche's "just widen the filter" framing is wrong.

2. **Symbiote inverts the donor/recipient axes vs. the existing variant.** Measured Oracle: `"Put a +1/+1 counter on target creature you control. It gains this card's other abilities."` So:
   - **donor = "this card" = `SelfRef` (the source)** — NOT a declared target.
   - **recipient = "It" = the +1/+1-counter target creature** (`ParentTarget`).
   The existing resolver (`gain_activated_abilities.rs:54-66`) reads the *donor* from `ability.targets` and defaults recipient to `SelfRef` (Quicksilver: donor = target creature, recipient = source). Symbiote is the mirror image: donor = source, recipient = the target. The resolver's donor/recipient resolution must branch on the donor filter.

3. **Symbiote's grant is PERMANENT, not `UntilEndOfTurn`.** The parser combinator (`imperative.rs:5224`) hard-defaults `duration.or(Some(Duration::UntilEndOfTurn))`. Symbiote states no duration → **CR 611.2a: "If no duration is stated, it lasts until the end of the game."** = `Duration::Permanent` (variant exists, `types/ability.rs`). The new parse arm must set `Some(Duration::Permanent)`, not fall through to the UEOT default.

### Current parse status (measured `jq` over `data/card-data.json`)
- **Symbiote** — Find New Host lowers `PutCounter` correctly; its `sub_ability` = `Unimplemented { name: "gain", description: "gain this card's other abilities" }`. The `"It"` subject was already stripped by the subject layer (the anaphor becomes `ParentTarget`). The combat trigger lowers cleanly (`DamageDone` → `Dig`). **Residual = one Unimplemented sub_ability + resolver widening.**
- **Choreographed Sparks** — mode-1 (copy instant/sorcery) lowers to `CopySpell` fine; `"This spell can't be copied"` → static `CantBeCopied` fine. Mode-2 lowers `CopySpell { target: creature spell, retarget: KeepOriginalTargets }`, and its `sub_ability` = `Unimplemented { name: "the", description: "The copy gains haste and \"At the beginning of the end step, sacrifice ~.\"" }`. **Residual = one Unimplemented sub_ability + a small copy_spell resolver extension.**

---

## Corpus fan-out probe (measured — proves each is a CLASS)

### B3 grant-abilities-of-donor class
`jq` for `gains? all activated abilities of` → **Quicksilver Elemental, Grell Philosopher, Havengul Lich** (3 cards, all `ActivatedOnly`, all already supported). `jq` for `gains? ... other abilities` (grant sense) → **Symbiote Spider-Man** (the `AllOther` case). The `GrantedAbilityScope` parameter is the axis that separates these two measured sub-classes of the same variant.
- **Honest weakness (flagged, mirrors the tranche's C8 disclosure):** the *runtime* `AllOther` side is 1 card today (Symbiote). The parameterization is still the correct architecture because (a) **parameterize-don't-proliferate** forbids a sibling variant, (b) it generalizes the Quicksilver class to the "gains all abilities of" reading (tranche line 166), and (c) the related static class `"has all abilities of"` (Clandestine Chameleon, Questing Cosplayer, Pin Collection, Polis — measured 4 cards) is the same conceptual axis at the `ContinuousModification` layer. The scope field is a building block, not a Symbiote special-case.

### Grant-to-copy (haste + delayed end-step sac) class
`jq` for `the copy gains` → **Choreographed Sparks + Nalfeshnee** (2 cards, near-identical text; measured Nalfeshnee `sub_ability` = the *same* `Unimplemented { name: "the" }`). The broader `"... gains haste. Sacrifice/Exile it at the beginning of the next end step"` template is **~25 cards** (Cadric, Chrome Dome, Flameshadow Conjuring, Helm of the Host, Molten Echoes, Saheeli Rai, …) — already handled for the `CopyTokenOf` (token-creation) path. Choreographed/Nalfeshnee are the **`CopySpell` (spell-copy)** members of that class. Fixing the copy_spell path clears both and every future "the copy gains `<kw>` and `\"<ability>\"`" card. Solid ≥2 class.

---

## Existing-building-block trace

### `GainActivatedAbilitiesOfTarget` end-to-end (measured)
- **Type** — `types/ability.rs:9240` `GainActivatedAbilitiesOfTarget { target: TargetFilter (donor), recipient: TargetFilter (=SelfRef default), duration: Option<Duration> }`.
- **Parser** — `imperative.rs:5194` `try_parse_gain_all_activated_abilities_of_target`: strips `"gain[s] all activated abilities of "`, delegates donor to `parse_target`, defaults recipient `SelfRef`, duration UEOT. Group-recipient (Grell) rebind at `oracle_effect/mod.rs:15163` (`*recipient = subject.affected`).
- **Resolver** — `game/effects/gain_activated_abilities.rs:31`: donor from `ability.targets` first `Object`; snapshot `donor.abilities.filter(kind==Activated).map(GrantAbility)` (line 72-85); register via `add_transient_continuous_effect` onto `SelfRef`→`SpecificObject{source}` or a battlefield scan for group filters (line 90-141); `flush_layers`. CR 611.2c snapshot-once semantics already documented and correct.
- **EffectKind / gates** — `EffectKind::GainActivatedAbilitiesOfTarget` (`ability.rs:13115`); dispatch `game/effects/mod.rs:3027`; coverage `coverage.rs:2107` (`{ target, .. }`); printed_cards `printed_cards.rs:1055` (`{ .. }`); trigger_index `trigger_index.rs:777` (EffectKind arm); ability_graph `analysis/ability_graph.rs:879` (`{ .. }`); sequence `sequence.rs:4905` (`{ .. }`).
- **`TargetFilter::ParentTarget`** exists (`ability.rs:2878`); "inherits targets from the parent ability at resolution time" (`game/targeting.rs:116-118`); handled in `filter.rs:87,290`. `"It"` is a bare object pronoun → `ParentTarget` (`oracle_effect/mod.rs:157 is_bare_object_pronoun`, doc `:153`).

### Choreographed Sparks — every piece already exists (measured); residual is composition + one apply-gap
- **`CopySpell`** (`ability.rs:8995`) with `additional_modifications: Vec<ContinuousModification>` ("Non-keyword copy exceptions stamped onto spell copies at creation" — Ob Nixilis). Resolver `game/effects/copy_spell.rs:31`; mods applied by `apply_spell_copy_modifications` (`copy_spell.rs:227`).
- **`ContinuousModification::AddKeyword`** and **`GrantTrigger { trigger: Box<TriggerDefinition> }`** (`ability.rs:17306,17350`) — the two mods "the copy gains haste and `\"<ability>\"`" decomposes into.
- **Quoted-ability → `GrantTrigger` lowering already exists** — `oracle_static/keyword_grant.rs:1636-1644` routes a trigger-prefixed quoted ability to `GrantTrigger`; `become_copy_except.rs:169,610,818,1329` proves `it has "<triggered ability>"` → `GrantTrigger` for the copy case.
- **Delayed end-step sac lowering already exists** — `"at the beginning of the next end step, sacrifice it"` → `Effect::CreateDelayedTrigger { AtNextPhase{End}, Sacrifice }` (snapshot test `oracle_effect/snapshot_tests.rs:248`; `oracle.rs:426,443`). **Cadric, Soul Kindler** proves the whole token-copy composition works today (measured parse: `CopyTokenOf` → `GenericEffect{AddKeyword Haste, ParentTarget}` → `CreateDelayedTrigger{AtNextPhase End, Sacrifice{LastCreated}}`).
- **The one measured gap:** `apply_spell_copy_modifications` (`copy_spell.rs:227-254`) applies **only** `RemoveSupertype` + `starting_loyalty_from_casualty_sacrifice`. It silently ignores `AddKeyword` / `GrantTrigger`. And `copy_spell.rs` registers **no** transient continuous effect or delayed trigger anywhere (grep for `add_transient_continuous_effect|GrantTrigger|delayed` in the file = 0 hits). So stamping haste + the sac-trigger into `additional_modifications` would today be dropped. This is the genuine residual for Choreographed — an ~15-line extension to that one function, mirroring the existing `RemoveSupertype` base+live stamp pattern.

---

## What's genuinely missing (true residual)

| Card | Parser | Engine |
|------|--------|--------|
| **Symbiote** | 1 new nom arm: `"gain[s] this card's other abilities"` → `GainActivatedAbilitiesOfTarget { target: SelfRef, recipient: SelfRef, scope: AllOther, duration: Some(Permanent) }`. | New enum `GrantedAbilityScope`; new `scope` field on the variant; resolver: SelfRef-donor branch, ParentTarget-recipient branch, `AllOther` dual-store snapshot (`abilities` minus granting-ability + `trigger_definitions`). |
| **Choreographed** | 1 fold: detect the mode-2 `sub_ability` `"the copy gains haste and \"<end-step sac>\""` → append `[AddKeyword(Haste), GrantTrigger(<sac>)]` to the parent `CopySpell.additional_modifications` (reuse `keyword_grant` quoted→GrantTrigger). | Extend `apply_spell_copy_modifications` to apply `AddKeyword` + `GrantTrigger` (base+live) so they survive copy→token resolution (CR 608.3f / 707.10f). **No new Effect variant.** |

---

## Design

### B3 — Symbiote (parameterize the existing variant; NO rename)

**Enum surface (the only engine-inventory delta):**
```rust
// types/ability.rs — new enum near GainActivatedAbilitiesOfTarget
/// CR 611.2c: which of the donor's abilities the resolution-time grant snapshots.
#[derive(...serde, Default...)]
pub enum GrantedAbilityScope {
    /// Quicksilver Elemental / Grell / Havengul: activated abilities only.
    #[default]
    ActivatedOnly,
    /// Symbiote Spider-Man "this card's OTHER abilities": all abilities of every
    /// kind (activated + triggered + static), EXCLUDING the granting ability
    /// itself. (For a non-self donor there is no self to exclude; that pairing
    /// does not arise in the corpus — documented invariant.)
    AllOther,
}
```
Add to the variant (`ability.rs:9240`):
```rust
#[serde(default)]
scope: GrantedAbilityScope,
```
**Decision — add a field, do NOT rename `GainActivatedAbilitiesOfTarget` → `GainAbilitiesOfTarget`.** Rationale, load-bearing:
- Every existing match arm uses `{ .. }` or `{ target, .. }` (measured at all 11 sites incl. both consumer-crate sites). A `#[serde(default)]` field means **zero** of them need editing.
- **A rename would touch `game/effects/mod.rs:3027` and `game/filter.rs`-adjacent arms — both are in the s07 FROZEN set.** Field-add keeps this change entirely out of the frozen trio. This is decisive.
- The name is retained with an updated doc comment explaining the scope axis. A future cosmetic rename is a mechanical, semantically-risk-free follow-up.

**Parser** (`imperative.rs`, new sibling to `try_parse_gain_all_activated_abilities_of_target`, nom-only):
```
// nom: value((), tag("this card's other abilities")) after alt((tag("gain "), tag("gains ")))
// (bridge nom_on_lower / TextPair per module convention — NO contains/find/split_once)
Some(Effect::GainActivatedAbilitiesOfTarget {
    target: TargetFilter::SelfRef,          // donor = "this card"
    recipient: TargetFilter::SelfRef,       // rebound to ParentTarget by the subject layer
    scope: GrantedAbilityScope::AllOther,
    duration: Some(Duration::Permanent),    // CR 611.2a
})
```
The subject layer already strips `"It"` (measured) and, at `oracle_effect/mod.rs:15163`, sets `*recipient = subject.affected`; for the bare object pronoun `"It"`, `subject.affected = ParentTarget`. **VERIFY at impl** that this rebind fires for this sub_ability once the effect is no longer `Unimplemented` (the Unimplemented result currently proves subject-application ran and stripped `"It"`, but the `:15163` arm only matches `GainActivatedAbilitiesOfTarget`, so it was skipped). If the bare-pronoun path routes around `:15163`, set `recipient: ParentTarget` directly in the arm instead of relying on the rebind. Either way the shipped effect must carry `recipient = ParentTarget`.

**Resolver** (`game/effects/gain_activated_abilities.rs`):
1. Read `scope` and donor filter from the effect.
2. **Donor id:** `match donor_filter { SelfRef => ability.source_id, _ => ability.targets first Object }`. (Quicksilver/Grell unchanged; Symbiote reads source.)
3. **Recipient id:** extend the recipient `match`: add a `ParentTarget => resolve recipient_id from ability.targets first Object` branch that registers onto `SpecificObject{recipient_id}` directly — **do not** route ParentTarget through the battlefield `matches_target_filter` scan (avoids depending on ParentTarget resolution inside `filter.rs`, which is FROZEN). SelfRef and group-filter branches unchanged.
4. **Snapshot (`AllOther`):**
   ```
   for a in donor.abilities.iter() {
       if scope==ActivatedOnly && a.kind != Activated { continue; }        // existing behavior
       if scope==AllOther && effect_contains_all_other_grant(&a.effect) { continue; } // exclude "this" ability
       mods.push(GrantAbility { definition: Box::new(a.clone()) });
   }
   if scope==AllOther {
       for t in donor.trigger_definitions.iter_all() {
           mods.push(GrantTrigger { trigger: Box::new(t.clone()) });      // the combat-damage trigger
       }
   }
   ```
   `effect_contains_all_other_grant(&Effect) -> bool` = a small recursive walk (self, `sub_ability`, `else_ability`, nested chains) returning true iff it finds `GainActivatedAbilitiesOfTarget { scope: AllOther, .. }`. This self-identifies Find New Host (the sole ability carrying this effect) and excludes exactly it — no ability-id needed, robust to reordering. (`ResolvedAbility` carries no ability-index; measured struct `ability.rs:17680`.)

**CORRECTNESS GATE #1 (donor-in-exile) — must be discharged by a runtime test, not assumed.** Find New Host's cost is `"Exile this card from your graveyard"` (`activation_zone: Graveyard`, measured). At resolution the source card is in **exile**; per CR 400.7 a zone change makes a new object, so `ability.source_id` may be stale. The plan reads `state.objects.get(&source_id)`. The runtime test below MUST prove the exiled source object still resolves with its `abilities` + `trigger_definitions`. If it does not, the impl falls back to reading the **printed** abilities/triggers from the card definition (`card_db`) for the `SelfRef` donor. Flag hard at impl review.

### Choreographed — compose existing blocks (Approach: `additional_modifications`)

**Parser fold** (post-lowering patch, analogous to the `:15163` recipient rebind; location: the CopySpell effect-chain lowering, `oracle_effect/mod.rs` or `sequence.rs` — a shared parser file, NOT the frozen `game/effects/mod.rs`):
- Detect the mode-2 `sub_ability` whose subject is `"the copy"` and effect is `"gains haste and \"<quoted end-step sac>\""`.
- Decompose (nom, per-axis `alt`/`tag`, reusing `keyword_grant`):
  - `"haste"` → `ContinuousModification::AddKeyword { keyword: Haste }`.
  - `"\"At the beginning of the end step, sacrifice ~.\""` → `keyword_grant` quoted-ability path → `ContinuousModification::GrantTrigger { trigger: <at End step, Sacrifice SelfRef> }` (the `~`/"this token" self-reference resolves to the token, per copy `source_id` rebind `copy_spell.rs:109 set_resolved_source_recursive`).
- **Append** both mods to the preceding `CopySpell.additional_modifications` and drop the `Unimplemented` sub_ability.
- This is chosen over a Cadric-style chained `CreateDelayedTrigger`+`GenericEffect` because a `CopySpell` puts the copy **on the stack** (it becomes a token only at resolution, CR 707.10f/608.3f) — a chained effect running when the CopySpell resolves would target a stack object, not a permanent. `additional_modifications` is the purpose-built carrier that rides the copy through the stack→token transition (exactly as Ob Nixilis's `RemoveSupertype` does).

**Engine** (`game/effects/copy_spell.rs`, `apply_spell_copy_modifications`, NOT frozen) — add two arms mirroring the existing `RemoveSupertype` base+live stamp:
```
ContinuousModification::AddKeyword { keyword } => {
    copy_obj.keywords.push(*keyword);           // live
    // + base store so it survives copy→token (CR 608.3f), mirroring RemoveSupertype
}
ContinuousModification::GrantTrigger { trigger } => {
    // push to copy_obj.trigger_definitions + base_trigger_definitions
}
```
**CORRECTNESS GATE #2 — must be discharged by a runtime test.** Verify (a) the resolved token has haste (attacks the turn it's made), and (b) the granted "at end step, sacrifice this token" trigger fires and sacrifices the token at the next end step. If base-store stamping does not survive the spell→permanent transition for triggers, escalate to registering the mods as a transient continuous effect keyed on `copy_id` in `copy_spell.rs` after the copy is inserted. The RemoveSupertype precedent (Ob Nixilis, a shipped card) is the evidence that base+live stamping persists; the trigger case must be independently confirmed.

---

## CR verification (every number grep-verified against `docs/MagicCompRules.txt` BEFORE citing)

| CR | Line | Text (verified) | Used for |
|----|------|------|----------|
| **611.2a** | 2904 | "If no duration is stated, it lasts until the end of the game." | Symbiote grant = `Duration::Permanent` |
| **611.2c** | 2909 | Continuous effect from resolution: affected set + snapshot fixed when it begins; won't change. | AllOther snapshot-once; Symbiote hostile test |
| **603.7** | 2610 | An effect may create a delayed triggered ability... contains "when/whenever/at". | Choreographed granted end-step sac |
| **702.10 / 702.10a** | 3969/3971 | "Haste" / "Haste is a static ability." | Choreographed haste grant |
| **707.10** | 5666 | To copy a spell = put a copy onto the stack; copies characteristics + decisions. | CopySpell mode-2 |
| **707.10f** | 5682 | "As that copy resolves, it ceases being a copy of a spell and becomes a token permanent." | copy→token persistence of granted haste/trigger |
| **608.3f** | 2836 | Copy of a permanent spell becomes a token as it enters the battlefield; not "created". | base-store stamp survives resolution |
| **111.13** | 717 | A copy of a permanent spell becomes a token as it resolves; has the spell's characteristics. | token identity |
| **701.21** | 3449 | **Sacrifice** (keyword action). | delayed sac |
| **602.1 / 603.1** | (variant doc `ability.rs:17347-17356`) | activated → `obj.abilities`; triggered → `obj.trigger_definitions` (different stores). | AllOther dual-store snapshot |

**TRANCHE CR ERROR FOUND:** S25-PLAN-FINAL.md B2/B3 cite **"sacrifice 701.16"** — grep shows **701.16 = Investigate**; **Sacrifice is CR 701.21** (line 3449). Correct number is used above. Flag for the driver to fix in the tranche doc.

---

## Discriminating verification matrix (runtime cast+resolve; measured deltas; revert-failing)

All runtime tests use `GameScenario` + `GameRunner::cast(...).resolve()` per the `card-test` skill (assert on `CastOutcome`/board deltas, not AST shape). Parser round-trip tests are a *supplement*, never the proof of support.

| # | Test | Non-vacuous / discriminating property | Revert that fails it |
|---|------|--------------------------------------|----------------------|
| **S1** | **Symbiote runtime (donor-in-exile GATE #1):** board Symbiote + a vanilla creature; graveyard-cast Find New Host paying `{2}{U/B}` + exile-from-gy; target the vanilla creature. Assert: (a) +1/+1 counter present; (b) the creature now has the DamageDone combat-trigger (deal it combat damage to a player → it digs & fills hand/gy). | Proves the **triggered** ability transferred AND that the exiled source's abilities were readable. A shape-only parse test cannot prove either. | Removing the `trigger_definitions` snapshot → creature has no trigger → (b) fails. |
| **S2** | **Load-bearing negative (tranche line 225), building-block level:** construct a donor with ONLY a triggered ability (nothing in `abilities`). Grant twice: `scope=ActivatedOnly` → recipient gains **nothing**; `scope=AllOther` → recipient gains the trigger. | Two-authority discriminator: the scope parameter must actually gate the trigger. This is the core proof the parameterization is non-vacuous. | Making both scopes snapshot `trigger_definitions` → ActivatedOnly leg wrongly transfers → fails. |
| **S3** | **Symbiote hostile (line 214, CR 611.2c):** grant (AllOther), THEN add a new trigger to the donor, assert recipient did NOT gain the post-grant ability. | Set-fixed-at-start. | A live-rescan implementation → recipient gains the late ability → fails. |
| **S4** | **"Other" exclusion:** after S1, assert the creature did NOT gain **Find New Host** (the granting activated ability). | Proves `effect_contains_all_other_grant` excludes the source ability. | Dropping the exclusion → creature gains Find New Host → fails. |
| **S5** | **Regression:** Quicksilver Elemental still gains the target creature's activated ability under the default `ActivatedOnly` scope (existing test must stay green). | Proves field-add didn't change existing behavior. | A wrong default (`AllOther`) → Quicksilver over-grants → fails. |
| **C1** | **Choreographed runtime (GATE #2):** cast a vanilla creature spell; cast Choreographed choosing mode 2 targeting it; resolve the copy. Assert: (a) the token can attack this turn (haste); (b) at the next end step the token is sacrificed (leaves battlefield). | Proves haste + granted end-step-sac both survive copy→token. Also clears **Nalfeshnee** (same text) — verify Nalfeshnee parse flips too. | Not extending `apply_spell_copy_modifications` → mods dropped → (a)/(b) fail. |
| **C2** | **Choreographed mode-1 regression + can't-be-copied:** mode 1 copies an instant/sorcery with new-target choice; `CantBeCopied` static still blocks copying Choreographed itself. | Proves the fold didn't disturb mode-1 or the static. | — |
| **P1..P2** | Parser round-trips (supplement): Symbiote sub_ability → `GainActivatedAbilitiesOfTarget{ scope: AllOther, recipient: ParentTarget, target: SelfRef, duration: Permanent }`; Choreographed → `CopySpell` with `additional_modifications` containing `AddKeyword(Haste)` + `GrantTrigger`. No residual `Unimplemented` in either tree. | Shape check only — NOT counted as support. | Removing the new arm/fold → `Unimplemented` returns → fails. |

---

## Coverage-regression note

Both changes touch shared parser dispatch (`imperative.rs`, the CopySpell chain lowering). **The card-data coverage-regression check is CI-authoritative** (per memory: parser changes can silently swallow clauses on *other* cards, invisible to `cargo test -p engine`). At impl time: capture `coverage-breakdown.sh --format standard` (or the per-card `card-data.json` coverage) BEFORE and AFTER, and diff. Required: Symbiote + Choreographed + Nalfeshnee flip `supported=false→true`; **no** unrelated card regresses. Empirical before/after diff is a hard merge gate.

---

## Files-touched table

| File | Frozen? | Change |
|------|---------|--------|
| `crates/engine/src/types/ability.rs` | shared (coordinate) | NEW enum `GrantedAbilityScope { ActivatedOnly(#[default]), AllOther }`; add `#[serde(default)] scope: GrantedAbilityScope` to `GainActivatedAbilitiesOfTarget` (`:9240`). No rename → no other arm in this file changes. |
| `crates/engine/src/game/effects/gain_activated_abilities.rs` | no | Resolver: donor `SelfRef→source_id`; recipient `ParentTarget→ability.targets[0]` direct; `AllOther` dual-store snapshot; `effect_contains_all_other_grant` helper. + `#[cfg(test)]` S2/S3/S4. |
| `crates/engine/src/parser/oracle_effect/imperative.rs` | shared (coordinate) | NEW nom arm `"gain[s] this card's other abilities"`. |
| `crates/engine/src/parser/oracle_effect/mod.rs` **or** `sequence.rs` | shared (coordinate) | (a) VERIFY/ensure `recipient=ParentTarget` for the `"It"` sub_ability; (b) CopySpell sub_ability fold into `additional_modifications`. |
| `crates/engine/src/game/effects/copy_spell.rs` | no | Extend `apply_spell_copy_modifications` to apply `AddKeyword` + `GrantTrigger` (base+live). + runtime test C1. |
| `crates/engine/src/parser/oracle_static/keyword_grant.rs` | reuse only | Reused for quoted-ability → `GrantTrigger` (no change expected; confirm the entry point is callable from the fold site). |
| **No change** | — | `game/effects/mod.rs` (FROZEN), `game/filter.rs` (FROZEN), `game/effects/delayed_trigger.rs` (FROZEN), `coverage.rs`, `printed_cards.rs`, `trigger_index.rs`, `analysis/ability_graph.rs`, consumer crates `phase-ai`, `mtgish-import` — all use `{ .. }`/`{ target, .. }` or are unaffected by a `#[serde(default)]` field. |

---

## /add-engine-variant gate checklist (for `GrantedAbilityScope` + the new field)

1. **`cargo engine-inventory`** — regenerate `data/engine-inventory.json` locally (gitignored; present in tree). Grep the `Grant*`/`Gain*` cluster (measured today): `GainActivatedAbilitiesOfTarget, GrantAbility, GrantAllActivatedAbilitiesOf, GrantAllTriggeredAbilitiesOf, GrantKeyword(s), GrantStaticAbility, GrantTrigger`.
2. **Sibling-cluster smell check → PASS (parameterize, not proliferate).** There is NO existing `GainAbilities*` scope enum. The `GrantAll{Activated,Triggered}AbilitiesOf` pair is the **static-side** (layer-rescanned CR 604/611.3) mechanism — a deliberately separate axis from this **resolution-time snapshot** variant (documented invariant `ability.rs:9229-9234`). Adding a scope *field* to the resolution variant does NOT duplicate them. Adding a sibling `GainAllAbilitiesOfTarget` variant WOULD be the smell — rejected.
3. **Categorical-boundary check → PASS.** The parameterization axis (which *kinds* of one donor's abilities to snapshot) lies entirely within CR 611.2/611.2c (resolution-time continuous effects). It does not cross into the static-ability layer system.
4. **Registration points threaded (enumerated):** variant def (`ability.rs:9240`) ✔ resolver ✔ parser ✔. **All other arms absorb the new field via `{ .. }`/`{ target, .. }`** — verified at: `ability.rs:11945,12460,12690,12886,13353`; `game/effects/mod.rs:3027`; `coverage.rs:2107`; `printed_cards.rs:1055`; `trigger_index.rs:777`; `ability_graph.rs:879`; `sequence.rs:4905`; and **consumer crate `phase-ai`** `cast_facts.rs:378` (`{ target, .. }`) + `redundancy_avoidance.rs:441` (`{ .. }`). `mtgish-import`: **zero** references (grep-verified). `#[serde(default)]` keeps old persisted snapshots valid (they deserialize as `ActivatedOnly`).
5. **Exhaustive-match compiler gate:** `GrantedAbilityScope` is matched only in the resolver; a non-wildcard `match scope { ActivatedOnly => .., AllOther => .. }` there makes the compiler flag any future third scope.
6. **Choreographed needs NO gate** — it adds no variant (reuses `CopySpell.additional_modifications`, `AddKeyword`, `GrantTrigger`). Only a resolver-apply extension + parser fold.

---

## Scope-bar coordination flags (for the driver)

- **Clean result:** with the field-add (not rename) decision and reading `ability.targets` directly for the ParentTarget recipient, **neither card touches the s07-FROZEN trio** (`game/effects/delayed_trigger.rs`, `game/filter.rs`, `game/effects/mod.rs`). Choreographed's delayed sac is a *granted trigger* via `additional_modifications`, not a `CreateDelayedTrigger` in `delayed_trigger.rs`.
- **Shared-file collision points (coordinate serialization, not frozen):** `types/ability.rs` (enum add), `parser/oracle_effect/imperative.rs` (new arm), `parser/oracle_effect/mod.rs`/`sequence.rs` (recipient-rebind verify + CopySpell fold). These are the usual multi-driver contention files — land P2f's edits when the parser-file lock is clear, re-read before edit.

---

## Verdict (to driver s25-impl-revived)

- **(a) Premise:** TRUE but INCOMPLETE. The filter is activated-only as claimed, but the real work is 3× larger than "widen the filter": triggered abilities live in a *separate store* (`trigger_definitions`) the resolver never reads; Symbiote *inverts* donor/recipient (donor=`SelfRef`, recipient=`ParentTarget`); grant is `Permanent` (CR 611.2a), not UEOT. Plus a tranche CR error: Sacrifice is **701.21**, not 701.16 (=Investigate).
- **(b) True residual:** Symbiote = 1 parser arm + resolver (SelfRef-donor / ParentTarget-recipient / AllOther dual-store snapshot with granting-ability exclusion). Choreographed = 1 parser fold + extend `apply_spell_copy_modifications` to apply `AddKeyword`/`GrantTrigger`. Two live correctness GATES to discharge by runtime test: donor-in-exile readability (Symbiote), granted-trigger persistence across copy→token (Choreographed).
- **(c) Enum-surface delta:** ONE new enum `GrantedAbilityScope` + ONE `#[serde(default)]` field. Threads through only variant-def + resolver + parser; all 11 existing/consumer match sites absorb it via `{ .. }`. Field-add chosen over rename specifically to avoid editing the FROZEN `game/effects/mod.rs`.
- **(d) Choreographed composes cleanly** from existing blocks (`CopySpell.additional_modifications`, `AddKeyword`, `GrantTrigger`, `keyword_grant` quoted→trigger) with ONE small engine gap: `apply_spell_copy_modifications` currently drops those mods. No new Effect variant. Simultaneously fixes Nalfeshnee (measured identical text) → proves the class.
- **(e) Scope-bar:** no frozen-file edits required.
- **(f) Plan doc:** `/home/lgray/vibe-coding/s25-impl-wt/.planning/coverage-analysis/S25-P2f-grant-abilities-PLAN.md`.

---
## DRIVER RIDERS (post-review, from team-lead) — MUST honor at impl

1. **New-enum walker classification (REBASE-time registration point — plan at IMPL, not rebase).** `GrantedAbilityScope` is a new enum; this plan was written against a base that PREDATES the PR-#4904 walker on current main. When the tranche rebases onto current main, `GrantedAbilityScope` will hit the #4904 walker's EXHAUSTIVE matches. At impl time (before rebase), pull the walker's expected classification shape from current main and add the `GrantedAbilityScope` classification arm proactively — do NOT defer to rebase-time breakage. The plan reviewer (a9cae226) reviews against the pre-walker base and will NOT surface this; it is a known impl-time addition.
2. **Sacrifice CR = 701.21a, NEVER 701.16.** Choreographed's delayed end-step sac annotation MUST cite CR 701.21a (701.16 = Investigate — a codebase-wide drift bug team-lead owns as a separate sweep; do not re-introduce it here).
3. **add-engine-variant gate is mandatory** for `GrantedAbilityScope` (cargo engine-inventory + checklist) — team-lead reconfirmed.

---
## REVIEW CONDITIONS (a9cae226 — APPROVE-WITH-CONDITIONS; MUST honor at impl)

- **C1 (compile break — plan's Section-B "all 11 sites absorb via {..}" is WRONG).** `#[serde(default)]` helps deserialization ONLY, never Rust construction/exhaustive-bind. CONSTRUCTION sites must add `scope: GrantedAbilityScope::ActivatedOnly`. DRIVER-VERIFIED load-bearing sites: `imperative.rs:5270` (real NON-TEST parser construction — confirmed) + `game/effects/gain_activated_abilities.rs:237` (test construction — confirmed). Reviewer also cited test destructures `imperative.rs:15854/16027` — line numbers IMPRECISE (15854 is a `GenericEffect` bind w/ `..`); **treat the COMPILER as the authority** for the exact construction + no-`..` exhaustive-bind set (field-add is compiler-forced — build once, fix every flagged site). Non-test match arms (ability.rs 11945/12460/12690/12886/13353, mod.rs:3027, coverage.rs:2107, printed_cards.rs:1055, trigger_index.rs:777, phase-ai ×2) DO absorb via `{..}`; mtgish-import/engine-wasm/server-core zero refs.
- **C2 (fold location — MUST pin to `lower_effect_chain_ir`, lower.rs:1031).** Nalfeshnee's byte-identical grant lives in a TRIGGER-execute chain, not an activated-ability chain. The fold MUST go at `lower_effect_chain_ir` (the chokepoint called by BOTH abilities `oracle.rs:1930` AND triggers `oracle_trigger.rs:1169`), NOT the `parse_effect_chain` wrappers (triggers bypass them — matches the [[lower_effect_chain_ir-is-the-chain-chokepoint]] fact). Wrapper-path fold → Nalfeshnee stays Unimplemented → ≥2-class claim fails. lower.rs is NOT frozen. Gate: the runtime test MUST assert Nalfeshnee flips supported.
- **C3 (rules-correctness — Nalfeshnee conditional grant).** Nalfeshnee's grant is "If it's a permanent spell"; unconditional append of haste+sac drops that condition (non-permanent copy → sac trigger mis-applied; practical effect negligible, Choreographed mode-2 = creature spell so unaffected). Either gate the grant to permanent-spell copies or document the deviation in the arm.
- **C4 (keep hard runtime gates).** GATE#1 donor-in-exile readability (Symbiote source exiled; `ability.source_id` may be stale post-CR-400.7 → card_db fallback) + GATE#2 granted-trigger persistence across copy→token — both discharged by S1/C1 runtime cast+resolve tests with revert-failing assertions BEFORE flipping supported.

Frozen-file safety CLEAN (contingent on C2 fold→lower.rs). Parameterize-vs-proliferate PASS (Grant* static-side siblings = deliberately-separate axis per ability.rs:17347-17356; scope = leaf param within CR 611.2c). No CR errors; plan's 701.16→701.21 catch is correct.
