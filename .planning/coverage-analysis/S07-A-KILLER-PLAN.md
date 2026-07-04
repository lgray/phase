# S07 Implementation Plan — A Killer Among Us

**Card** (enchantment, {4}{G}):
> When this enchantment enters, create a 1/1 white Human creature token, a 1/1 blue Merfolk creature token, and a 1/1 red Goblin creature token. Then secretly choose Human, Merfolk, or Goblin.
> Sacrifice this enchantment, Reveal the creature type you chose: If target attacking creature token is the chosen type, put three +1/+1 counters on it and it gains deathtouch until end of turn.

**Goal:** flip `supported:true`, `gap_count:0`, no deferrals. Currently RED — the ETB "Then secretly choose Human, Merfolk, or Goblin" is a **swallowed clause** (evidence: `crates/engine/src/parser/oracle_ir/snapshots/…diagnostic_swallowed_clause.snap` is A Killer's ETB text), and the sacrifice ability misparses (PutCounter targets `SelfRef` instead of the attacking token; "Reveal the creature type you chose" misparses as a phantom `Sacrifice`; the `is the chosen type` gate is dropped).

**Verification note:** Tilt does NOT watch worktree `s07-impl-wt`. All build/test verification is **direct cargo** (`cargo test -p engine …`) run by the executor; the full engine test suite and workspace clippy **exceed the ~5-min background-job wall-clock cap — run them FOREGROUND with a 10-min timeout** (measured; foreground escapes the cap).

**⚠️ Line-number caveat:** every `file:line` anchor in this plan was traced at pre-rebase base `e7bd1a670`. The tranche is now rebased onto `c2c0162b1` (HEAD `d22ac9083`), so line numbers have drifted — **re-locate every anchor by CONTENT/symbol, not by line number** (standard executor practice). The *mechanism* (hoist at the conditions.rs strip site, `subject_slot: Some(0)`, the serde split, the LKI read) is base-independent and correct as written.

**This plan was revised after review-plan r1 (finding B1).** N4 uses the HOIST mechanism (§2/N4) — NOT the discarded `ctx.subject` approach. A1–A5 advisories are folded (call-site count 28; CR 700.2 dropped from N1; CR 702.106 is analogy-only; N3 cost-splitter gate; coverage necessary-not-sufficient).

---

## 1. Trace-First (file:line for every reused mechanism — MEASURED)

### Choose subsystem (the "secretly choose" mechanism)
- **`Effect::Choose { choice_type, persist, selection }`** — `types/ability.rs` (variant). Interactive resolver: `game/effects/choose.rs:21` (`resolve`) sets `WaitingFor::NamedChoice { player, choice_type, options, source_id }`; `source_id` is `Some` iff `persist` (choose.rs:74-83).
- **`compute_options`** — `game/effects/choose.rs:337` (fn); `ChoiceType::CreatureType` arm at **`choose.rs:350`** returns `state.all_creature_types` (or `FALLBACK_CREATURE_TYPES` at `:270`) — **no candidate restriction today** (design Q1 pivot).
- **`bind_named_choice`** — `game/effects/choose.rs:194` — single authority that, when `source_id` is `Some`, pushes the chosen value onto `obj.chosen_attributes` and recomputes layers (CR 607.2d + 613.1). Shared by the interactive `ChooseOption` answer handler (`engine_resolution_choices.rs`) and the random resolver.
- **`ChosenAttribute::CreatureType(String)`** — `types/ability.rs:867`. Written by `bind_named_choice`; read by `GameObject::chosen_creature_type()` at **`game/game_object.rs:1745`**.
- Precedent writers onto a persistent object: `game/replacement.rs:6637` (ETB choose), `game/casting_costs.rs:1891` (Celestial Reunion behold — writes onto the SPELL). A Killer writes onto the **battlefield enchantment permanent** — same `persist:true` + `source_id` path.

### "Is the chosen type" filter + gate
- **`FilterProp::IsChosenCreatureType`** — `types/ability.rs:3118`. Evaluation: **`game/filter.rs:3775`** (object path) and **`game/filter.rs:4219`** (record path) — both read `source.chosen_creature_type` and call `subtype_matches_with_changeling(...)` (CR 205.3e + 205.3m + 702.73a).
- **Source-context derivation:** `game/filter.rs:1652-1653` / `:2065-2066` build `source_chosen_creature_type = state.objects.get(&source_id).and_then(|s| s.chosen_creature_type())` into `SourceContext` (`filter.rs:3041`). **Reads the live object at `source_id`.**
- **`AbilityCondition::TargetMatchesFilter { filter, use_lki, subject_slot }`** — `types/ability.rs:14841`. Evaluates the ability's resolved target against `filter` (present-tense "is"; LKI when `use_lki`). Classified **Handled** in coverage at `game/coverage.rs:6380`. Parser already emits it with an `IsChosenCreatureType` leg for the Celestial "revealed card is the chosen type" gate at **`parser/oracle_effect/conditions.rs:505-525`** (`parse_revealed_card_is_chosen_type` at `:440`).

### LKI (sacrifice-as-cost source read at resolution — design Q3 crux)
- **ObjectId is stable across a zone move** — `game/zones.rs:125,138` ("ObjectId here is storage identity and persists across the zone change").
- **`chosen_attributes` are cloned into the moved object** — `game/zones.rs:169` (`chosen_attributes: obj.chosen_attributes.clone()` in the LKI snapshot) and are **only cleared on battlefield *entry*** (CR 400.7) — `game/game_object.rs:1532` (`self.chosen_attributes.clear()` inside `reset_for_zone_change`, doc'd "when a permanent ENTERS the battlefield", `:1484`). A sacrifice moves the enchantment *to* the graveyard; its chosen_attributes survive on the graveyard object at the same `source_id`.
- **`lki_cache` backstop** — `game/zones.rs:181` inserts `LKISnapshot` (incl. `chosen_attributes`, `game_object.rs:1453`) on every battlefield/exile exit. `state.lki_cache: HashMap<ObjectId, LKISnapshot>` (`types/game_state.rs:7313`). Filters already fall back to it (`filter.rs:717,1427,4315,4701`).
- **CR 608.2h** (verified `docs/MagicCompRules.txt:2806`): an effect reading a specific object uses current info if it's in the public zone it was expected to be in, else last-known information. The graveyard is public and retains the attribute → the gate reads correctly with **no new plumbing**.

### Token creation
- **`Effect::Token { name, power, toughness, types, colors, keywords, tapped, count, owner, attach_to, enters_attacking, supertypes, static_abilities, enter_with_counters }`** — `types/ability.rs:8390`. One token spec per effect.
- Parser entry: **`parser/oracle_effect/token.rs:39`** (`try_parse_token`). Three distinct tokens = a chain of **3** `Effect::Token` (effect-chain comma/"and" split). Coverage-Handled at `coverage.rs:2214`.

### PutCounter + deathtouch-until-EOT
- **`Effect::PutCounter { counter_type, count, target }`** — coverage-Handled `coverage.rs:2267`.
- "gains deathtouch until end of turn" → a `GenericEffect` carrying `ContinuousModification::AddKeyword { keyword: Deathtouch }` with an until-end-of-turn duration (the *current misparse already produces this shape*, only with the wrong `SelfRef` target — proves the primitive exists).

### Cost
- **`AbilityCost::Sacrifice(SacrificeCost)`** — `types/ability.rs` (variant ~6844); **`AbilityCost::Reveal { count, filter }`** — `types/ability.rs:6971` (reveals *cards*, not a chosen attribute). Coverage gates `supported` only on `AbilityCost::Unimplemented` (`coverage.rs`, `ability_cost_has_unimplemented`).

### Frontend / AI / Coverage seams (agent-verified)
- **Frontend:** `client/src/components/modal/NamedChoiceModal.tsx:46` renders `WaitingFor::NamedChoice` **fully data-driven** from `data.options`; `CreatureType` title key already exists (`client/src/i18n/locales/en/game.json` → `namedChoice.title.creatureType`). **Zero new i18n keys.**
- **AI:** `crates/engine/src/ai_support/candidates.rs:3945` (`named_choice_actions`) enumerates one `ChooseOption` per engine-provided option — **restricted candidate set handled automatically** (dispatch at `candidates.rs:1236-1241`). Sacrifice-cost activated ability enumerated by the generic `ActivateAbility` path. **Zero new AI seams.**
- **Coverage:** `supported` = no `Effect::Unimplemented` / `AbilityCost::Unimplemented` in the tree (`coverage.rs:5995` `is_ability_supported`); `gap_count` from the compile-forced Handled/Unhandled feature classifier. Every A Killer node (Token, Choose, PutCounter, Sacrifice, Reveal, TargetMatchesFilter, GenericEffect/AddKeyword) is already Handled. **Zero new coverage classification.**

---

## 2. NEW vs REUSED

### REUSED (no code change)
Effect::Token + `try_parse_token`; Effect::Choose + `resolve` + `bind_named_choice`; ChosenAttribute::CreatureType + `chosen_creature_type()`; FilterProp::IsChosenCreatureType + `SourceContext`; AbilityCondition::TargetMatchesFilter; Effect::PutCounter; GenericEffect + AddKeyword(Deathtouch)/until-EOT; AbilityCost::Sacrifice; the LKI stack (zones.rs:169/181 + graveyard-retains-attributes); NamedChoiceModal (data-driven); AI `named_choice_actions` + generic activation; all coverage classification.

### NEW (the actual work — 4 code items + 1 snapshot refresh)

**N1 — [ENUM FIELD] `ChoiceType::CreatureType { options: Vec<String> }`** (design Q1).
Parameterize the bare unit variant with a candidate-restriction list. `options` empty ⇒ all creature types (today's behavior, byte-stable); non-empty ⇒ restricted set. Sites:
- `types/ability.rs:300` — variant def → `CreatureType { #[serde(default)] options: Vec<String> }`; add `ChoiceType::creature_type()` constructor (empty) mirroring `color()`/`card_type()`.
- `types/ability.rs:420-421` (Serialize) — emit **unit** `"CreatureType"` when `options.is_empty()`, else struct variant (exact mirror of the `Color`/`CardType`/`Opponent` empty-vs-populated arms at `:423-448`,`:468-476`). Keeps every existing Morophon/Changeling card's JSON byte-identical.
- `types/ability.rs:502+` (Deserialize) — add `ChoiceTypeData::CreatureType { options }` arm; keep the Unit `"CreatureType"` ⇒ `{ options: vec![] }` arm.
- `types/ability.rs:915` (`ChosenAttribute::choice_type`) and `:1009` (`ChoiceValue::from_choice`) — pattern updates to `CreatureType { .. }` / construct empty.
- `game/effects/choose.rs:350` (`compute_options`) — when `options` non-empty, return `options.clone()` (the restricted set); else the existing all-types path. (CR 205.3m — **A2: do NOT add CR 700.2**, which is modal-spell definition, unrelated to a creature-type option list; keep only the correct 205.3m already on this arm.)
- **28 mechanical call sites** (A1 — MEASURED `grep -rn "ChoiceType::CreatureType"`, not ~51; breakdown: `choose.rs` 5, `oracle_effect/tests.rs` 3, `types/ability.rs` 2, `deck_loading.rs` 2, `card_db.rs` 2, `manabrew-compat/lib.rs` 2, `mtgish-import/convert/{replacement,action}.rs` 1+1, `game/replacement.rs` 1, `coverage.rs` 1, parser files 7, `tests/integration/morophon_chosen_type_1653.rs` 1): construction sites → `ChoiceType::creature_type()` or `{ options: vec![] }`; match arms → `CreatureType { .. }`. Files: `deck_loading.rs`, `replacement.rs`, `coverage.rs:1787`, `card_db.rs`, `oracle_replacement.rs`, `oracle_cost.rs:271`, `oracle_effect/{imperative.rs:4277,subject.rs:3887,mod.rs:17732}`, `oracle.rs:1167`, tests, **plus dormant `crates/mtgish-import/src/convert/{action.rs,replacement.rs}` and `crates/manabrew-compat/src/lib.rs`** — mechanical `{ .. }`/`{ options: vec![] }` only, **no new mtgish logic** (skill: mtgish is out of scope for *features*; a shared-enum compile-fix is not a feature).

`add-engine-variant` gate: this is a **leaf parameterization of an existing structural axis** (candidate restriction), identical in shape to `Color { excluded }` / `CardType { excluded }` / `Opponent { restriction }` / `Keyword { options }`. Not a new sibling. Categorical boundary: creature types = CR 205.3m, one rule section, field stays inside `CreatureType`. Existence check: `compute_options` today returns all types — no restricted mechanism exists. **PASS — parameterize, do not proliferate.**

**N2 — [PARSER] creature-type enumeration arm** in `try_parse_named_choice` (`parser/oracle_effect/mod.rs:17722`).
Add an arm (after the `"a creature type"` arm at `:17732`) that parses `"<Type>, <Type>, or <Type>"` (and 2/N-length lists) of **creature-type words** → `ChoiceType::CreatureType { options }`. Direct precedent: the `is_card_type_enumeration` arm (Cloud Key, `mod.rs` near `:17700`) that turns `"artifact, creature, enchantment, instant, or sorcery"` into `ChoiceType::card_type()`. Nom-compliant: `separated_list1(alt((tag(", or "), tag(", "), tag(" or "))), creature_type_word)` where `creature_type_word` validates against the CR 205.3m subtype set (reuse the existing subtype canonicalizer in `oracle_util.rs`; do NOT hardcode Human/Merfolk/Goblin — build for the class of "secretly choose <T1>, <T2>, or <T3>" cards). Preserve source order in `options`. "secretly " prefix is already stripped by the existing `alt((tag("choose "), preceded(tag("secretly "), tag("choose "))))` at `mod.rs:17724`.

**N3 — [PARSER] "reveal the creature type you chose" cost arm** in `parser/oracle_cost.rs`.
Recognize `"reveal the creature type you chose"` (and the general `"reveal the <chosen-attribute> you chose"`) as a **no-op cost** and consume it so it does not fall through to the phantom-`Sacrifice` misparse. Rationale (ponytail-marked): revealing an already-stored, engine-visible chosen attribute is informationally a no-op in this full-information engine — the same reason "secretly" is a no-op (design Q2). Nom: `tag("reveal the ")` + creature-type-word/`"creature type"` + `tag(" you chose")` → return "recognized, emit no cost component". Add `// ponytail: reveal-of-chosen-attribute is a no-op in a full-information engine; the chosen type is already stored on the source's chosen_attributes (CR 607.2d). Consume so the cost splitter doesn't misparse it as Sacrifice.` **Do NOT** map to `AbilityCost::Reveal` — that variant reveals *cards* and would try to reveal a nonexistent card. **A4 — executor gate:** assert the cost splitter yields EXACTLY `AbilityCost::Sacrifice(SelfRef)` with NO residual/second cost component and NO swallow diagnostic after N3 consumes the reveal phrase (test #7(b) asserts the single-Sacrifice shape — a leftover cost component must FAIL the gate).

**N4 — [PARSER] target-declaring leading-if conditional (HOIST)** — new `strip_target_declaring_chosen_type_conditional` in `parser/oracle_effect/conditions.rs` (sibling of `strip_additional_cost_conditional` ~`:452` and `strip_card_type_conditional` ~`:1249`), wired into the leading-conditional dispatch chain in `oracle_effect/mod.rs` **between `strip_additional_cost_conditional` and `strip_leading_general_conditional`** (~`mod.rs:21509-21514`) so Celestial's compound arm still wins first and the general handler can't pre-empt.

> ⚠️ **CRUX (review-plan B1) — the target must be HOISTED, NOT bound via `ctx.subject`.** The `ctx.subject` approach is runtime-DEAD: `resolve_it_pronoun` (`oracle_effect/mod.rs:175-182`) maps `ctx.subject = Some(Typed, non-SelfRef/Any)` → `TargetFilter::TriggeringSource`, which is UNBOUND for an activated ability (no triggering event). `TargetMatchesFilter{subject_slot:None}` then tests the current node's first object target / TriggeringSource — but the ability declares NO object target (the "target attacking creature token" was consumed by the strip). Counters/deathtouch would land on an unbound `TriggeringSource` and the gate has nothing to test. A Killer is the ONLY card in `card-data.json` with `": If target"` leading an ability effect — novel grammar; the Malamet precedent (targets declared in a SEPARATE leading clause) does NOT apply.

Handle `"if target <filter> is the chosen type, <body>"` by LIFTING the condition's target into the body's first anaphor:
1. `tag("if ")` then `parse_target` the `"target attacking creature token"` phrase → require `TargetFilter::Typed` (Creature + `Attacking{defender:None}` + `Token`); capture the target-phrase slice. `tag(" is the chosen type, ")` → `<body>`.
2. **Substitute the captured target phrase for the body's FIRST bare object pronoun** via `replace_first_object_pronoun(body, target_phrase)` — a ~3-line helper reusing the existing `is_bare_object_pronoun` classifier (`oracle_effect/mod.rs:160-165`). The body then reads "put three +1/+1 counters on **target attacking creature token** and it gains deathtouch until end of turn" — normal target parsing DECLARES the target (slot 0) and chains the 2nd "it"→`ParentTarget`. `// ponytail: rewrites the FIRST pronoun only (CR 608.2c anaphora); upgrade to per-pronoun tracking iff a multi-target "chosen type" card ever appears.`
3. Push `FilterProp::IsChosenCreatureType` onto the captured `Typed` filter and emit `AbilityCondition::TargetMatchesFilter { filter: TargetFilter::Typed(typed_with_chosen), use_lki: false, subject_slot: Some(0) }` — gate bound to the DECLARED slot 0. `subject_slot: Some(0)` is **NECESSARY, not stylistic**: eval at `effects/mod.rs:7908-7949` routes `Some(0)` → `resolve_parent_slot_from_root(state, ability, 0)` (`targeting.rs:741`) → `flatten_targets_in_chain(root).nth(0)` = the hoisted attacking token; `None` reads the current node's targets → unbound `TriggeringSource`.

Result AST: `PutCounter{P1P1,3,target: Typed(Creature,[Attacking,Token])}` (declares slot 0) → sub `GenericEffect{AddKeyword(Deathtouch)/EOT, target: ParentTarget}`; gate `TargetMatchesFilter{filter: Typed+IsChosenCreatureType, subject_slot: Some(0)}`. **Chaining proof:** the rewritten body is byte-shape-identical to **Sigil of Myrkul** ("put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn"); "and it gains" splits at `sequence.rs:2099` (`tag("it gains ")`) and the 2nd "it"→`ParentTarget`, proven by the **Ms. Bumbleflower** test `bumbleflower_it_gains_flying_binds_to_counter_target` (`tests.rs:12694-12733`). This is a **conditions.rs parse-time lift**, NOT a `lower.rs` sibling of `rewrite_two_target_counter_chain` (that keys on ≥2 `TargetOnly` slots, `lower.rs:2301`; A Killer has 1 target). Nom-compliant: `tag("if ")` + `parse_target` + `tag(" is the chosen type, ")`; reuse `parse_target` and the `" of/the chosen type" → IsChosenCreatureType` helper in `conditions.rs`.

**N5 — [SNAPSHOT] refresh** `parser/oracle_ir/snapshot_tests.rs:870` (full A Killer text) and the `diagnostic_swallowed_clause.snap` — regenerate after N1-N4 so the swallow diagnostic clears and the corrected AST is captured. Run `cargo insta review`/`INSTA_UPDATE=always` per repo convention; confirm the diagnostic snapshot no longer flags A Killer.

---

## 3. Field-Expressiveness Verdicts (design questions)

| Q | Verdict |
|---|---------|
| **Q1 Candidate restriction** | Reused `ChoiceType::CreatureType` (bare unit) does **not** express it — `compute_options:350` returns all types. **Add `options: Vec<String>`** (N1). Sanctioned parameterization, not a STOP. The candidate set is the **explicit Oracle enumeration** ("Human, Merfolk, or Goblin"), parsed by N2 — not dynamically coupled to the 3 tokens. |
| **Q2 "Secretly"** | **No-op in this engine.** Justified by full-information engine semantics: phase.rs has no per-player hidden-info masking for `chosen_attributes`, the chosen type is stored openly in serialized state, and the parser already strips "secretly" for ~40 existing cards ("secretly choose an opponent", etc., `mod.rs:17726`). **A3: CR 702.106a/b (Hidden Agenda / conspiracy, verified `MagicCompRules.txt:4780,4782`) is cited only as a descriptive ANALOGY for the paper-note model — it is NOT A Killer's governing rule.** A Killer's "secretly choose" is an ordinary linked choice (CR 607.2d) with no hidden-info subsystem. **Do not build one.** |
| **Q3 Reveal cost + enchantment-LKI** | **Reveal = no-op** (N3): revealing a stored, engine-visible attribute changes no state. **LKI = no new plumbing** (design Q3 crux, MEASURED): `source_id` is stable across sacrifice (`zones.rs:125,138`), the graveyard object retains `chosen_attributes` (cleared only on battlefield *entry*, `game_object.rs:1484,1532`), and `lki_cache` snapshots them (`zones.rs:169,181`). The `TargetMatchesFilter`→`IsChosenCreatureType`→`SourceContext.chosen_creature_type` read at `filter.rs:1652` resolves against that graveyard object (CR 608.2h). **CONTINGENCY** (executor must prove via the matching-type test): if the runtime read returns `None`, add a `lki_cache` fallback to the source-chosen-type derivation at `filter.rs:1652`/`:2065` (~4 lines, precedented by `filter.rs:717,4701`) — this is the only latent risk and is bounded. |
| **Q4 Target + gate** | Fully composes from **reused** blocks: `TargetMatchesFilter{IsChosenCreatureType}` (N4 emits) + `PutCounter` + `GenericEffect(AddKeyword Deathtouch, until-EOT)`. No new effect/condition types. |
| **Q5 ETB create-3-then-choose** | **Reused** effect types: chain of 3 `Effect::Token` + `Effect::Choose{persist:true}`. The only new piece is the **enumeration parse** (N2) — the choose clause is currently swallowed. |

**STOP-AND-RETURN flags: NONE.** The single enum-field addition (N1) is the skill-endorsed parameterization move, not a blocker. Scope caveat: N1's 28 call sites include dormant `mtgish-import`/`manabrew-compat` — mechanical compile-fixes only.

---

## 4. Mandatory Architectural Sections

- **Pattern Coverage.** N1+N2 build the reusable primitive "**secretly choose a creature type from a fixed Oracle-listed candidate set**" (CR 607.2d linked choice, restricted per CR 205.3m) — covers any current/future "secretly choose <T1>, <T2>, or <T3>" card, not just A Killer. N4 builds "**leading-if that declares the ability's target and gates it on a chosen attribute**" — a reusable target-declaring-conditional block. N3 covers the "reveal the <chosen X> you chose" no-op-cost class. Estimated direct class size today: small (A Killer is the sole current 3-type case) but each primitive is category-shaped, not card-shaped.
- **Building Blocks.** `try_parse_named_choice` + `is_card_type_enumeration` precedent (N2); `oracle_util.rs` subtype canonicalizer (N2, no hardcoded names); `parse_target` + `conditions.rs` `" of the chosen type"→IsChosenCreatureType` helper (N4); `ParseContext.subject` anaphora (N4); `compute_options` (N1); `bind_named_choice` (runtime, unchanged); the LKI stack (runtime, unchanged).
- **Logic Placement.** Candidate restriction is a **choice-domain** concern → `ChoiceType` field + `compute_options` (engine). Enumeration recognition + target-declaring conditional + no-op reveal are **parser** concerns → `oracle_effect/{mod.rs,conditions.rs}`, `oracle_cost.rs`. Zero frontend/AI logic (both data-driven). The gate and buff live in **engine** condition/effect resolvers (reused).
- **Rust Idioms.** `Vec<String>` restriction field mirrors sibling `excluded`/`options`/`restriction` fields; byte-stable unit/struct serde split; nom `separated_list1`/`alt`/`tag`/`preceded` (no `contains`/`split_once` for dispatch); exhaustive match arms forced by the coverage classifier and serde.
- **Nom Compliance.** N2: `separated_list1(alt((tag(", or "),tag(", "),tag(" or "))), creature_type_word)`. N3: `tag("reveal the ") … tag(" you chose")`. N4: `tag("if ") + parse_target + tag(" is the chosen type, ")`. All detection IS the parser — no string scanning for dispatch.
- **Extension vs Creation.** All four items EXTEND existing patterns (enum-field parameterization, a new `try_parse_named_choice` arm, a new cost arm, a new `strip_*_conditional` sibling). No new architecture.
- **Analogous Trace.** Traced Celestial Reunion end-to-end: `parser/oracle_cost.rs:237-271` (behold-chosen-type cost) → `casting_costs.rs:1891` (writes `ChosenAttribute::CreatureType` onto the object) → `conditions.rs:505-525` (`TargetMatchesFilter{IsChosenCreatureType}` gate) → `filter.rs:1652,3775` (source chosen-type read). A Killer reuses the same write→store→gate spine, differing only in (a) the write happens via a persistent ETB `Choose` instead of a cost, and (b) the source is read post-sacrifice via LKI. Also traced Morophon (`ChoiceType::CreatureType` all-types) as the compute_options baseline and Cloud Key (`is_card_type_enumeration`) as the N2 precedent.
- **Variant Discoverability.** `cargo engine-inventory` + `grep -rn "ChoiceType::CreatureType"` consulted; no restricted-creature-type mechanism exists. `add-engine-variant` checklist run in §2/N1 — parameterization, single CR section, PASS.
- **Identity / Provenance Contract.** "the chosen type" (CR 607.2d linked ability): **authority** = the enchantment permanent's `ChosenAttribute::CreatureType`; **binding time** = ETB resolution (`bind_named_choice`, `persist:true`, `source_id`=enchantment); **storage** = `obj.chosen_attributes` (+ `lki_cache` on exit); **consuming fn** = `GameObject::chosen_creature_type()` via `SourceContext` at `filter.rs:1652`; **live→snapshot transition** = at sacrifice the value latches on the graveyard object / lki_cache and is read as LKI (CR 608.2h); **invalidation** = cleared on any future battlefield re-entry (moot — sacrificed). **Multi-authority hostile fixture:** two A-Killer enchantments with *different* chosen types on the battlefield, each sacrificed for its own ability, must gate against ITS OWN chosen type (proves the read is source-scoped, not global `last_named_choice`).

---

## 5. Discriminating Tests (cast-level, measured, non-vacuous)

Fixture: full A Killer Oracle text (reuse `snapshot_tests.rs:870`). Use `GameScenario`+`GameRunner::cast(...).resolve()` per the `card-test` skill; assert on `CastOutcome`/state deltas, never on AST-internal flags.

1. **ETB creates 3 typed tokens + records a chosen type.** Resolve the ETB; assert exactly one 1/1 white Human, one 1/1 blue Merfolk, one 1/1 red Goblin token on the battlefield; answer the `NamedChoice` with "Goblin"; assert the enchantment's `chosen_creature_type() == Some("Goblin")`. *Discriminates:* revert N2 ⇒ choose clause swallowed ⇒ no chosen attribute recorded.
2. **Candidate restriction (design Q1).** Assert the `WaitingFor::NamedChoice.options` == `["Human","Merfolk","Goblin"]` (order-preserved), NOT the full creature-type list. *Discriminates:* revert N1/`compute_options` ⇒ options == all creature types (assert length > 3 and contains "Dragon" fails).
3. **Matching-type attacking token → 3 counters + deathtouch (the gate fires + LKI proven).** Chosen = Goblin; declare the Goblin token as an attacker; activate the sacrifice ability targeting it; resolve. Assert the token has 3 +1/+1 counters (P/T 4/4) and Deathtouch until EOT. *Discriminates + proves LKI:* the enchantment is already in the graveyard when this resolves; if the source chosen-type read failed post-sacrifice the gate would be false and the assert fails — this is the Q3 contingency probe.
4. **Non-matching type → NO counters (gate discriminates).** Chosen = Goblin; attack with the **Human** token; activate targeting it; resolve. Assert the Human token has 0 counters and no Deathtouch. *Discriminates:* proves `TargetMatchesFilter{IsChosenCreatureType}` actually gates (a vacuous "always apply" parse fails here).
5. **Target must be an ATTACKING token.** With no attackers declared, assert the sacrifice ability has no legal target (or targeting a non-attacking token is rejected). *Discriminates:* proves the `Attacking` leg of the target filter.
6. **Multi-authority (provenance hostile fixture).** Two A-Killer enchantments, chosen Goblin vs Merfolk; sacrifice each against a Goblin attacker. Assert only the Goblin-chooser grants counters, the Merfolk-chooser does not. *Discriminates:* proves the chosen-type read is source-scoped LKI, not global `last_named_choice`.
7. **Parser unit (non-vacuous).** `parse_oracle_text` on the full card: assert (a) an ETB trigger whose effect chain contains 3 `Effect::Token` + one `Effect::Choose{ choice_type: CreatureType{ options:["Human","Merfolk","Goblin"] }, persist:true }`; (b) an activated ability with a single `Sacrifice(SelfRef)` cost (no phantom second Sacrifice, no `Reveal`); (c) `AbilityCondition::TargetMatchesFilter{ IsChosenCreatureType }`; (d) `PutCounter` and the Deathtouch grant target the attacking token, not `SelfRef`. *Discriminates:* each leg reverts to the documented current misparse.

Coverage assertion: after N1-N5, `cargo coverage` (or the card-data pipeline) reports A Killer `supported:true`, `gap_count:0`, and the swallowed-clause diagnostic clears. **A5 — necessary but NOT sufficient:** `is_ability_supported` (`coverage.rs:5995`) keys only on ABSENCE of `Effect::Unimplemented`/`AbilityCost::Unimplemented`, so `supported:true` can flip while counters/deathtouch still target the wrong object (the exact B1 failure mode). The REAL correctness authority is cast tests **#3** (matching type → 3 counters + deathtouch) and **#4** (non-matching → nothing); the coverage flip alone does NOT close the tranche.

---

## 6. CR Annotations (grep-verified against `docs/MagicCompRules.txt`)

- **CR 111.1** (`:645`) — tokens (Effect::Token, N2 ETB).
- **CR 205.3m** (`:1439`) — creature types (Human/Merfolk/Goblin all listed) → candidate set + IsChosenCreatureType.
- **CR 607.2d** (`:2744`) — linked "choose a [value]" / "the chosen [value]" abilities → the ETB choose ↔ sacrifice-ability "the chosen type" linkage (annotate N1/N4).
- **CR 602.2 / 602.2a** (`:2527,:2529`) — activate ability, pay costs (Sacrifice + Reveal cost).
- **CR 608.2h** (`:2806`) — LKI: read the sacrificed source's chosen type at resolution (annotate the filter read / N3 rationale).
- **CR 700.2** — modal-spell definition; **A2: NOT applicable to `compute_options`/creature-type restriction — do not annotate N1 with it.**
- **CR 702.106a/b** (`:4780,:4782`) — Hidden Agenda/conspiracy; cited only as a descriptive ANALOGY for the "secretly = paper note" no-op model (A3), **not** as A Killer's rules basis. A Killer's secret choice is CR 607.2d.
- (Deathtouch grant / +1/+1 counters use existing annotated resolvers — no new CR text.)

---

## 7. Files the executor will touch

**Engine (types + choice):**
- `crates/engine/src/types/ability.rs` — N1 (enum def, Serialize/Deserialize, `choice_type()`/`from_choice` arms, `creature_type()` ctor).
- `crates/engine/src/game/effects/choose.rs` — N1 (`compute_options:350`).

**Parser:**
- `crates/engine/src/parser/oracle_effect/mod.rs` — N2 (`try_parse_named_choice` enumeration arm ~`:17732`).
- `crates/engine/src/parser/oracle_cost.rs` — N3 (no-op reveal-chosen-type arm).
- `crates/engine/src/parser/oracle_effect/conditions.rs` — N4 (target-declaring `strip_*` conditional + dispatch wiring).

**Mechanical call-site updates (N1, 28 sites):** `deck_loading.rs`, `replacement.rs`, `coverage.rs`, `database/card_db.rs`, `oracle_replacement.rs`, `oracle_effect/{imperative.rs,subject.rs}`, `oracle.rs`, plus test modules; dormant `crates/mtgish-import/src/convert/{action.rs,replacement.rs}` and `crates/manabrew-compat/src/lib.rs` (compile-fix only).

**Snapshots/tests:** `crates/engine/src/parser/oracle_ir/snapshot_tests.rs` + `…/snapshots/…diagnostic_swallowed_clause.snap` (N5 refresh); new cast-level tests in the engine test module per §5.

**Frontend / AI:** NONE (data-driven — confirmed).

**Contingency only (Q3):** `crates/engine/src/game/filter.rs:1652`/`:2065` — add `lki_cache` fallback to the source chosen-type derivation *iff* test 3 fails.
