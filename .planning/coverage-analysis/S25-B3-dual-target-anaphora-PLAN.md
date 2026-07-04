# S25-B3 — Dual-target "Choose target A and target B" + anaphoric slot binding (front half)

**Exemplar:** Stolen Uniform {U} Instant —
"Choose target creature you control and target Equipment. Gain control of that Equipment until end of turn. Attach it to the chosen creature. When you lose control of that Equipment this turn, if it's attached to a creature you control, unattach it."

> **Review verdict:** APPROVE WITH CONDITIONS (4 conditions folded in — §7 snapshot ownership rewritten, s07 prerequisite added, `:896`-arm front-run invariant + negative test added, anaphor figures pinned).
>
> **⛔ Depends-on: s07 target-model increment.** B3 implementation is sequenced AFTER s07 lands its 3-site `ParentTargetSlot` snapshot fix (`delayed_trigger.rs`, `filter.rs`, `effects/mod.rs` — see §7.3). Those 3 files are OUT of B3's edit scope. Without s07, the delayed trigger is inert (empty snapshot) even after B3 emits the correct `ParentTargetSlot{1}`.

**Scope boundary with B1 (commit #4):** B1 ships the *delayed-trigger container* for the last sentence — the "When you lose control of that Equipment this turn" `DelayedTrigger`, the lose-control-of-permanent event matcher, and its `valid_card = ParentTarget` binding. This plan owns the **front half**: the dual-target declaration, the anaphor→slot binding contract that all later clauses (GainControl, Attach, and B1's trigger body) consume, the `Attach` slot wiring, and the two reusable leaf combinators the B1 trigger body needs (`unattach it` → `UnattachAll`, and the `if it's attached to a creature you control` intervening-if). The B1↔front-half handoff contract is specified in §7.

---

## 0. Corpus fan-out probe (measured)

Corpus: `data/card-data.json` (35,397 cards; object keyed by lowercased name; Oracle text in `.oracle_text`).

| Pattern (jq `test(...;"i")` over `.oracle_text`) | Count |
|---|---|
| `target [^.]*? and target ` (any dual-target declaration) | **48** |
| `choose target [^.]*? and target ` (Choose-prefixed dual declaration — this class) | **12** |
| `the chosen (creature\|permanent\|player\|equipment)` (chosen-anaphor, pinned regex) | **77** |
| `the chosen [a-z]` (all chosen-anaphor noun phrases) | **459** |
| dual-decl **and** `that (equipment\|creature\|artifact\|permanent\|land\|aura)` anaphor | **3** |

**The 12 "Choose target X and target Y" cards** (the direct class this unlocks/repairs):
beastie beatdown, blizzard brawl, duel for dominance, goblin welder, grim contest, hog-monkey rampage, joust, longstalk brawl, malamet battle glyph, mouth to mouth, stolen uniform, tail swipe.

**Representative anaphor forms across the class** (this is what the slot binder must cover):
- *Filter-restating:* "the creature you control" / "the creature an opponent controls" (fight cards).
- *"the chosen X":* "put a +1/+1 counter on the chosen creature" (Duel for Dominance).
- *"that X":* "Gain control of **that Equipment**"; "you gain control of **that creature**" (Stolen Uniform, Mouth to Mouth).
- *"the X card" vs "the X" (same type, different slot):* "sacrifices **the artifact** and returns **the artifact card**" (Goblin Welder — two artifact slots disambiguated by the `card`/zone qualifier, not by type).
- *plural set:* "those creatures fight each other" / "the other" (fight cards, via `TrackedSet`).
- *bare pronoun:* "Attach **it** to the chosen creature" ("it" = the just-controlled Equipment).

**Conclusion:** this is a genuine class of 12 declaration cards. The anaphor-binding half generalizes to the 77 core / 459 broad "the chosen X" cards and every future multi-target spell. **Not a one-off.**

---

## 1. Existing-building-block trace (what already exists)

Traced feature end-to-end: **Goblin Welder** ("Choose target artifact a player controls and target artifact card in that player's graveyard … sacrifices the artifact and returns the artifact card"). This is the closest analogue — it *already parses correctly* with two precise slot bindings.

Trace path:
1. **Declaration split — EXISTS.** `parser/oracle_effect/imperative.rs:3803 try_parse_two_targets` — nom `scan_split_at_phrase` on `alt((tag("and target "), tag("and another target ")))` (CR 601.2c), runs `parse_target` on each side, yields `ChooseImperativeAst::TwoTargets { target_a, target_b }`. Caller at `imperative.rs:3118` (inside `parse_choose_ast`, which **already has `ctx: &mut ParseContext`** in scope).
2. **Lowering — EXISTS.** `imperative.rs:9717` lowers `TwoTargets` to a primary `Effect::TargetOnly { target: target_a }` with a chained `sub_ability` `Effect::TargetOnly { target: target_b }` (bare `Effect` can't express a chain; only `ParsedEffectClause.sub_ability` can).
3. **Slot-precise binding mechanism — EXISTS.** `TargetFilter::ParentTargetSlot { index }` (`types/ability.rs`). Runtime resolution: `game/effects/mod.rs:217 effect_object_targets` (index into the *accumulated* announced-target vector), `game/targeting.rs:802` + `:692` (whole-chain accumulation, CR 608.2c), `game/filter.rs:3076`. Runtime-tested at `game/targeting.rs:4438 resolved_targets_parent_target_slot_uses_resolving_stack_entry_root_chain` (proves index 1 walks the *root chain*, not just the nearest node).
4. **Anaphor→slot resolver — EXISTS BUT CARD-SPECIFIC.** `parser/oracle_target.rs:1484 parse_definite_parent_reference` hardcodes `"the artifact card" → slot 1`, `"the artifact" → slot 0`. Called from `parse_target_with_syntax` (`:891`), which **carries `ctx`**. This is the *only* precise-slot anaphor path and it is a Goblin-Welder special case — a "build-for-the-card" violation.
5. **Broad-anaphor fallback — the bug source.** `oracle_target.rs:1033-1036` maps bare "the creature"/"the player"/"the spell"/"the land" → plain `TargetFilter::ParentTarget`. Plain `ParentTarget` resolves to **ALL** inherited object targets (`effect_object_targets` `_ =>` arm, mod.rs:231). So on a two-slot chain, "that Equipment" and "the chosen creature" both collapse to *both* slots.
6. **`Effect::Attach { attachment, target }` — EXISTS** (`types/ability.rs:8763`); `Effect::GainControl { target }` — EXISTS; `Effect::UnattachAll { attachment, target }` — EXISTS (`:8775`, CR 701.3d); `TrackedSet { id }` for plural anaphors — EXISTS.
7. **`ctx.subject` threading — EXISTS.** ParseContext carries `subject: Option<TargetFilter>` for "it"/"that creature" (`context.rs:17`).

### Measured current mis-parse of Stolen Uniform (live parse-probe dump — NOT a card-data field)
The card is unsupported, so `card-data.json` `parse_details` is `null` for it. The tree below is a **parse-probe dump** obtained by running the parser on the Oracle text directly (equivalently, the `abilities` AST emitted during the card-data build), not a persisted card-data field.
```
TargetOnly(creature you control)                        # slot A  ✓
 └ TargetOnly(Equipment)                                # slot B  ✓
   └ GainControl { target: ParentTarget }               # ✗ resolves to BOTH slots
     └ Attach { attachment: ParentTarget, target: ParentTarget }  # ✗ both args = BOTH slots (collision)
       └ Unimplemented("when you lose control …")        # → B1
```
**The declaration and lowering are already correct.** The defect is purely the anaphor binder emitting ambiguous `ParentTarget` where it must emit `ParentTargetSlot`.

---

## 2. What is genuinely missing (new work — minimal)

1. **A declared-target-slot registry on `ParseContext`** — currently absent. This is why the resolver at (4) had to hardcode artifact: it had no way to know the chain declared `[creature-you-control, Equipment]`.
2. **Generalize `parse_definite_parent_reference`** from the hardcoded artifact arms into a **type-driven slot resolver** that consults the registry. This *deletes* the Goblin-Welder special case (Goblin Welder must keep passing via the general path — see §4 ambiguity handling).
3. **Populate the registry** from `try_parse_two_targets`.
4. Two reusable leaf combinators for B1's trigger body: `unattach it` → `Effect::UnattachAll`, and the `if it's attached to a creature you control` intervening-if.

No new `Effect` variant, no new `TargetFilter` variant. The binding primitive (`ParentTargetSlot`) and the unattach effect (`UnattachAll`) already exist.

---

## 3. Design

### 3.1 ParseContext: declared-target-slot registry (types layer of parser state)

Add to `parser/oracle_ir/context.rs::ParseContext`:
```rust
/// CR 601.2c + CR 608.2c: Ordered target slots declared by the current
/// effect chain's "Choose target X and target Y" head. Index i is the filter
/// announced for the i-th `target` word (slot 0 = A, slot 1 = B, …). Later
/// clauses in the chain resolve anaphors ("that Equipment", "the chosen
/// creature", "the artifact card") to `ParentTargetSlot { index }` by matching
/// the anaphor's noun phrase against these filters. Reset per effect chain in
/// `parse_effect_chain_ir` (alongside the existing per-chain resets).
pub declared_target_slots: Vec<TargetFilter>,
```
- **Why `Vec<TargetFilter>` not a bool / index count:** the resolver must disambiguate by the *full filter shape* (type + card/zone qualifier), per the Goblin-Welder two-artifact case. Storing the filters (not just a count) is the composable representation. Typed enum reuse — no new type.
- **Binding time:** populated at parse time when the declaration clause is parsed (the head clause is parsed before any anaphor clause in the same chain, and `ctx` persists across chunks — same lifecycle as the existing `chosen_player_count` and `target_selection_mode` fields).
- **Reset:** in `parse_effect_chain_ir` where the other per-chain ctx fields reset, so slots never leak across cards/abilities.

### 3.2 Populate the registry (`imperative.rs`)

`try_parse_two_targets` gains `ctx: &mut ParseContext` (caller at `:3118` already holds it) and, on success, pushes `target_a` then `target_b` into `ctx.declared_target_slots`. Nom-compliant: the function body already uses `scan_split_at_phrase` + `tag` + `parse_target`; only the two `push` calls are added. (Generalizes to N slots if a future card chains "and target … and target …", though no current card does — do **not** build N-way now; YAGNI. The `Vec` is already N-ready.)

### 3.3 Type-driven slot resolver (`oracle_target.rs`) — replaces the hardcoded artifact arms

Rewrite `parse_definite_parent_reference` to take `(input, &[TargetFilter])` (the registry) and:
1. Parse the anaphor noun phrase with **existing combinators**: `opt(alt((tag("the chosen "), tag("the "), tag("that "))))` then `nom_target::parse_type_filter_word` (type/subtype token) then `opt(parse_card_or_cards_word)` (the "card" qualifier, already used at `oracle_target.rs:1068`). No `contains`/`find`.
2. For each registered slot filter, score a match against the parsed `(type_token, is_card)`:
   - type token ∈ the slot filter's `type_filters` (creature/artifact/…) or matches its `Subtype` (Equipment) — reuse the filter's own type predicates from `game/filter.rs`/`oracle_util` subtype canonicalization; do not re-implement type comparison.
   - `is_card` requires the slot filter carry a non-battlefield-zone `FilterProp::InZone`/card qualifier; absence requires the slot be battlefield-scoped.
3. Bind to the slot index of the **unique** best match. If zero or ≥2 slots tie, return `None` (fall through to the existing broad-`ParentTarget` behavior — honest degradation, never a wrong guess).

**Goblin Welder under the general path:** slots = `[artifact (bf), artifact card (graveyard)]`. "the artifact" → `is_card=false`, battlefield → unique slot 0. "the artifact card" → `is_card=true`, graveyard → unique slot 1. The hardcoded arms are **deleted**; the general resolver reproduces them from the registry (verified by keeping Goblin Welder's existing tests green).

**Stolen Uniform:** slots = `[creature-you-control, Equipment]`. "that Equipment" → subtype Equipment → unique slot 1. "the chosen creature" → type creature → unique slot 0.

Call site: `parse_target_with_syntax` (`:891`) already has `ctx`; pass `&ctx.declared_target_slots`. Placement stays **before** the broad `the creature`/`the player` `ParentTarget` arms (`:1033`) so a slot match wins under longest-match-first; the broad arm remains the fallback when the registry is empty (single-target spells) or ambiguous — zero regression for the ~thousands of single-target cards.

**Front-runs the `SELECTED_FROM_SET_PHRASES` arm (`:896`, invoked `:921`).** The generalized resolver at `:891` runs *before* the set-selection arm that maps "the chosen creature"/"the chosen card"/… → `ParentTarget` (`:905-913`). Cards in that arm (Agrus Kos, Duel for Dominance's "the chosen creature" put-counter, etc.) have **no** `TwoTargets` declaration, so `ctx.declared_target_slots` is empty → the resolver's empty-registry `None` path fires → control falls through unchanged to the `:896` arm → still `ParentTarget`. **Required invariant:** the empty-or-ambiguous-registry `None` path MUST leave every `:896`-arm card byte-identical. Enforced by verification-matrix claim 7.

### 3.4 "it" → equipment slot (Attach)

"Attach **it** to the chosen creature": "it" is a bare pronoun resolved via the existing `ctx.subject` mechanism. The preceding "Gain control of that Equipment" clause resolves its target to `ParentTargetSlot { 1 }`; that clause must set `ctx.subject = Some(ParentTargetSlot { 1 })` (subject-threading already happens for other verbs — extend GainControl's clause to snapshot its resolved target as subject, same pattern as existing subject setters). Then Attach's `attachment` = `ctx.subject` (slot 1, the Equipment) and `target` = "the chosen creature" via §3.3 (slot 0). Result:
```
Attach { attachment: ParentTargetSlot { 1 }, target: ParentTargetSlot { 0 } }
```
GainControl becomes `GainControl { target: ParentTargetSlot { 1 } }` (slot-precise, no longer both slots).

### 3.5 Leaf combinators for B1's trigger body (§7 handoff)

- **`unattach it`** → `Effect::UnattachAll { attachment: ParentTargetSlot { 1 } (the Equipment), target: TargetFilter::Any }`. Reuse `UnattachAll` (CR 701.3d, verified `docs/MagicCompRules.txt:3291`) — it already scopes *which* attached object moves via `attachment`; "unattach it" is the single-object case (`attachment` binds one Equipment). **No new `Effect::Unattach` variant** (ponytail: `UnattachAll` covers the class; adding a singular sibling is a parameterization-that-didn't-happen smell).
- **`if it's attached to a creature you control`** intervening-if → an `AbilityCondition` built from the equipment (`ParentTargetSlot { 1 }`) being attached to a `creature you control` host. Reuse `FilterProp::HasAttachment` / the `AttachedTo` host relation (`types/ability.rs:2806, :3818`) — do not invent a new condition variant without first running `/add-engine-variant` against `AbilityCondition`. (This condition is *consumed by* B1's `DelayedTrigger`; the front half only provides the parse.)

---

## 4. CR verification (grepped against `docs/MagicCompRules.txt`)

| CR | Text (verified) | Used for |
|---|---|---|
| CR 115.1c | activated/spell ability targeted by "target [something]" | dual declaration |
| CR 601.2c | "same object … chosen once for each instance of 'target'" | two independent slots |
| CR 608.2c | later instructions reference earlier objects; whole-chain accumulation | `ParentTargetSlot` binding |
| CR 701.3d (`:3291`) | "to 'unattach' an Equipment … move it away … on the battlefield but not equipping anything" | `UnattachAll` |
| CR 603.4 | intervening-if checked on trigger + on resolution | "if it's attached to a creature you control" |

All five verified present. (CR 701.3d confirmed verbatim at line 3291; do not cite from memory — re-grep at implementation time per the CR-annotation protocol.)

---

## 5. Verification matrix (discriminating, non-vacuous, real parse+cast pipeline)

| # | Claim | Changed seam | Production entry | Test (runtime, `GameScenario`+`GameRunner::cast().resolve()`) | Revert-failing assertion | Sibling / hostile |
|---|---|---|---|---|---|---|
| 1 | Stolen Uniform GainControl binds ONLY the Equipment | §3.3 resolver + §3.4 | cast + resolve | Cast with creature C (yours) + Equipment E (opponent's). Assert after resolution: E is controlled by you; **C's controller is unchanged**. | If revert → plain `ParentTarget` → you'd "gain control" of C too. The C-controller-unchanged assertion fails on the buggy build (this is the slot-discriminating check). | Hostile: give the opponent a *second* creature D so `ParentTarget`-all would grab an extra object; assert only E moves. |
| 2 | Attach binds equipment→creature, not slot collision | §3.4 | cast + resolve | Same cast. Assert E is attached to **C** (slot 0), not to itself/E. | Buggy build: `attachment==target==both slots` → attach is a no-op/illegal; E not equipping C. | Wrong-slot fixture: assert E is NOT attached to the Equipment slot object (proves attachment≠target). |
| 3 | Goblin Welder still parses to slots 0/1 via the general path | §3.3 (deletes hardcoded arms) | existing parser + existing runtime test | Keep `oracle_target.rs` tests at `:9043/:9050` (assert slot 0 / slot 1) green; keep the runtime `resolved_targets_…root_chain` test green. | If the general resolver regresses artifact disambiguation, these existing assertions fail. | Two-same-type slots (both artifact) — the discriminating case for "card"/zone disambiguation. |
| 4 | Registry ambiguity/empty → honest fallback (no wrong guess) | §3.3 return-`None` path | parser | Parser test: single-target "destroy the creature" (empty registry) still → `ParentTarget`; a synthetic 2-creature-slot phrase with a bare "the creature" anaphor → `None`/`ParentTarget` (ambiguous), NOT a silent slot-0 guess. | Asserts we never bind an ambiguous anaphor to a specific slot. | Negative: ambiguous tie must not resolve to slot 0. |
| 5 | `unattach it` → `UnattachAll { attachment: slot1 }` (leaf combinator) | §3.5 | parser shape test | Parser test on the isolated clause. | Assert `attachment == ParentTargetSlot{1}`, not `Any`. | Parser-shape only is acceptable here **because** full-card coverage stays red via the B1 `Unimplemented` until B1 lands — see §6. |
| 6 | Full Stolen Uniform coverage stays honest until B1 | — | coverage report | The last sentence remains `Effect::unimplemented("when", …)` until B1; card stays UNSUPPORTED. | Front-half tests assert the front clauses only; do not claim card-supported. | — |
| 7 | Empty/ambiguous registry does NOT shadow the `:896` set-selection arm | §3.3 `None` path | parser | Parser test: a `SELECTED_FROM_SET_PHRASES` card with **no** `TwoTargets` declaration ("the chosen creature" set-selection, e.g. Agrus Kos / a synthetic set-selection clause) still parses "the chosen creature" → `TargetFilter::ParentTarget`. | If the generalized resolver mis-fired on an empty registry it would emit `ParentTargetSlot{0}` and this assertion (`== ParentTarget`) fails. | Ambiguous 2-same-type registry with a bare anaphor → also `None` → `ParentTarget` (not slot 0). |
| 8 | **B1↔B3 integration** — delayed trigger fires and unattaches ONLY the Equipment | §3.5 emit `ParentTargetSlot{1}` in valid_card + effect body; s07 snapshot fix (prereq) | cast + control-loss + SBA/trigger resolution | Cast Stolen Uniform (creature C yours + Equipment E opponent's) → resolve (E controlled, attached to C) → cause you to lose control of E (duration expiry at end of turn or a control-change) → assert the delayed trigger fires and E becomes unattached, and **only E** (no other Equipment/attachment disturbed). | Revert-fails on BOTH bugs: (a) slot-collision → wrong object bound; (b) empty-snapshot → `delayed_ability.targets=[]` → trigger inert / never unattaches. | Hostile: a second Equipment F you control (attached elsewhere) must be untouched — proves `ParentTargetSlot{1}` snapshot binds E specifically, not "all attachments". |

**Non-vacuity evidence:** claim 1's "C-controller-unchanged" and claim 2's "attachment≠target" both *fail on the current (pre-change) build* because plain `ParentTarget` returns both slots — that is the discriminating property. Provide the pre-change parse dump (§1) in the impl PR as the baseline.

---

## 6. Coverage-regression / risk note (CI is authoritative)

Dual-target parsing (`try_parse_two_targets`) and the anaphor resolver (`parse_definite_parent_reference` → generalized) are **shared parse paths** hit by every card that says "the creature"/"the artifact"/"that <type>". A registry-consulting resolver that mis-scores could silently *re-bind* anaphors on unrelated cards (e.g. any single-target spell whose registry is empty must keep returning `ParentTarget`). Mitigations baked into the design: (a) resolver returns `None` on empty/ambiguous registry → unchanged behavior; (b) placement keeps the broad `ParentTarget` arm as fallback. **The card-data coverage-regression CI check is the authority** (per the "parser coverage regression is CI-only" memory — `cargo test -p engine` will NOT catch a swallowed-clause regression on other cards). Impl must diff `data/coverage-data.json` before/after and confirm zero net regressions; run the parser combinator gate + targeted semantic checks, then let Tilt's `card-data` resource confirm.

---

## 7. Identity/provenance contract + snapshot ownership (review checklist item 10)

### 7.1 Live-chain clauses (GainControl, Attach) — resolved LIVE, front-half owns
- **Source phrase → authority:** "that Equipment" / "it" → `ParentTargetSlot { 1 }`; "the chosen creature" → `ParentTargetSlot { 0 }`. Authority = positional index into the resolving ability chain's accumulated announced-target vector.
- **Binding time / semantics:** parse time stamps the *index*; runtime binds the *object* at resolution via `effect_object_targets` walking the root chain (CR 608.2c). **Live, not snapshotted — this applies ONLY to the on-stack GainControl/Attach clauses**, which resolve while the chain still exists on the stack. If the Equipment changes zone mid-resolution the slot re-resolves against the live chain per existing `ParentTargetSlot` semantics.

### 7.2 Delayed-trigger clause ("When you lose control … unattach it") — snapshotted at RUNTIME, B1/s07 own
- **The snapshot is a RUNTIME concern, owned by `game/effects/delayed_trigger.rs::resolve` — NOT the parser.** The parser is compile-time and has no runtime `ObjectId`. A delayed trigger fires *after* the spell has left the stack, so the chain no longer exists; the concrete `ObjectId` of the Equipment must be captured at delayed-trigger **creation time** (when the on-stack spell resolves) by the runtime snapshot machinery, then stored on the created delayed ability's `targets`.
- **Front half's ONLY obligation for this clause:** emit `TargetFilter::ParentTargetSlot { 1 }` in BOTH the delayed trigger's `valid_card` AND its effect body (`UnattachAll { attachment: ParentTargetSlot { 1 }, … }`). That is the whole compile-time deliverable. **No "expose the resolved ObjectId to B1"** — that claim is deleted; the parser cannot and must not produce a runtime id.

### 7.3 KNOWN GAP — `ParentTargetSlot` snapshot is a no-op today (owned by s07, NOT B3)
The existing delayed-trigger snapshot machinery is **`ParentTarget`-only and silently skips `ParentTargetSlot { index }`** at three sites (verified):
- `game/effects/delayed_trigger.rs::concrete_parent_target_filter` (`:285`) — has a `TargetFilter::ParentTarget` arm (`:291`) but **no `ParentTargetSlot` arm** → passes through unconcretized.
- `game/filter.rs::normalize_contextual_filter` (`ParentTargetSlot` listed `:88` but not concretized like `ParentTarget`).
- `game/effects/mod.rs::filter_refs_parent_target` (`:4084`) / `effect_refs_parent_target` (`:4000`) → return `false` for a `ParentTargetSlot`-only effect → **no snapshot taken** → `delayed_ability.targets = []` → **trigger inert** (never unattaches).

**Ownership: the s07 driver OWNS the fix for these 3 sites** — it is a shared target-model increment (they carry a committed Longstalk Brawl bug from the same root, plus first-target/chain propagation). **These 3 files are OUT of B3's edit scope.** Do not touch them.

### 7.4 Sequencing + prerequisite
> **Depends-on: s07 target-model increment (the 3-site `ParentTargetSlot` snapshot fix) must land before B3 implementation.**

B3 IMPLEMENTATION is **sequenced AFTER** s07's target-model increment. B3's own deliverable is:
1. Emit `ParentTargetSlot { 1 }` in the delayed trigger's `valid_card` + effect body (§7.2) — plus the front-half §1–§6 work.
2. Add the **B1↔B3 integration test** (verification-matrix claim 8): cast Stolen Uniform → cause control loss → assert the delayed trigger fires and unattaches ONLY the Equipment. Revert-fails on BOTH the slot-collision bug (front half) and the empty-snapshot bug (s07 prereq), so the test is the joint acceptance gate for the two increments meeting.

### 7.5 Who parses the last sentence
B1 owns the `DelayedTrigger` container + condition + `valid_card`. The front half provides the two leaf combinators (§3.5) and guarantees they emit `ParentTargetSlot { 1 }`. Confirm B1's trigger-body parser calls these leaves rather than re-implementing them.

---

## 8. Applicable skills / checklist coverage

- `/oracle-parser` (authoritative — all changes are parser-side except the ctx field).
- `/add-engine-variant` — **required gate** for the §3.5 intervening-if if it needs any new `AbilityCondition` variant. No new `TargetFilter`/`Effect` variant is introduced (reuse `ParentTargetSlot`, `UnattachAll`). Run `cargo engine-inventory` and the checklist before adding the condition variant.
- Nom mandate: every detection uses `tag`/`alt`/`opt`/`value`/`scan_split_at_phrase`/`parse_type_filter_word`/`parse_card_or_cards_word`/`parse_target`. Zero `contains`/`starts_with`/`find`/`split_once` in new dispatch.

---

## 9. Files touched

| File | Change |
|---|---|
| `parser/oracle_ir/context.rs` | add `declared_target_slots: Vec<TargetFilter>` + reset doc |
| `parser/oracle_effect/imperative.rs` | thread `ctx` into `try_parse_two_targets`; push both slot filters; §3.4 GainControl subject-set + Attach slot wiring |
| `parser/oracle_target.rs` | generalize `parse_definite_parent_reference` to registry-driven; **delete** hardcoded artifact arms; pass `&ctx.declared_target_slots` at `:891` |
| `parser/oracle_effect/*` (reset site) | reset `declared_target_slots` in `parse_effect_chain_ir` |
| `parser/…` (§3.5) | `unattach it` → `UnattachAll` leaf; `if it's attached to a creature you control` intervening-if leaf (coordinate with B1); emit `ParentTargetSlot{1}` in valid_card + effect body |
| tests | claims 1–8 (runtime cast tests + parser shape tests + `:896`-arm negative test + B1↔B3 integration test + keep Goblin Welder green) |
| **OUT of B3 scope — s07 owns** | `game/effects/delayed_trigger.rs`, `game/filter.rs`, `game/effects/mod.rs` (the 3-site `ParentTargetSlot` snapshot fix, §7.3). **Do NOT edit.** |
