# S25-B3 block-D — "When you lose control of that Equipment this turn, if it's attached to a creature you control, unattach it" DelayedTrigger recognizer

**Card:** Stolen Uniform {U} Instant. **Owns:** the LAST sentence only (front half shipped in e74285982). **Consumes:** the front-half slot registry (§3.3 of `S25-B3-dual-target-anaphora-PLAN.md`), the ChangesController lose-control runtime (cdb373503), the reflexive-delayed builder family (0e55ed3c5), and the s07 `ParentTargetSlot` snapshot infra (ebca1fab0).

> **⛔ SHIP-GATE VERDICT (read first): STOP-and-coordinate.** Block-D's parser work is fully in-scope and takes Stolen Uniform to *coverage* `supported:true, gap:0`. But that alone is a **hollow win** — the delayed trigger would fire and unattach **nothing** at runtime (measured below), because the s07-frozen `game/effects/mod.rs::effect_parent_ref_slots` has no `UnattachAll` hidden-slot arm, so `UnattachAll{attachment: ParentTargetSlot{1}}` is never snapshotted (`delayed_ability.targets = []`). This is exactly the "Unimplemented→supported exposes an unwired path" trap. **TRUE correctness (verification claim 8) requires ONE ~1-line arm in the s07-owned `mod.rs` (a file already on §7.3's list).** Block-D must FLAG this to the s07 driver and NOT be marked shippable-as-supported until that arm lands. Everything else (recognizer, valid_card, unattach leaf, intervening-if) is buildable parser-only with **zero new engine variants**.

---

## 0. STEP 0 — measured baseline (LIVE re-parse, not stale card-data)

`data/card-data.json` is **stale** (mtime `2026-07-01 22:15`, predates e74285982's `2026-07-02 18:04`; the Tilt `card-data` resource last ran `2026-07-01 20:04`). It still shows the pre-e74 `GainControl{ParentTarget}` / `Attach{ParentTarget,ParentTarget}`. Ignore it. Live parse via `cargo run -p engine --features cli --bin oracle-gen -- data --filter "stolen uniform" --output <tmp>`:

```
TargetOnly(Creature, controller=You)                       # slot 0  ✓
 └ TargetOnly(Subtype=Equipment)                           # slot 1  ✓
   └ GainControl { target: ParentTargetSlot{index:1} }     # ✓ e74 fixed
     └ Attach { attachment: ParentTargetSlot{1}, target: ParentTargetSlot{0} }  # ✓ e74 fixed
       └ Unimplemented { name:"when",
           description:"When you lose control of that Equipment this turn,
                        if it's attached to a creature you control, unattach it" }   # ← BLOCK-D TARGET
```

**Confirmed:** front half now emits `ParentTargetSlot` (driver's claim holds; card-data.json was just stale). The last sentence is STILL `Effect::unimplemented("when", …)` — exactly as expected, and the sole remaining gap. The `Unimplemented("when")` clause already reaches `parse_effect_clause_inner` and falls through every existing arm to the fallback, so a new arm added there will intercept it (no upstream re-route needed).

---

## 1. Corpus fan-out probe (measured; `data/card-data.json`, object keyed by lowercased name, `.oracle_text`)

| Pattern (jq `test(…;"i")`) | Count | Notes |
|---|---|---|
| `when you lose control of that [a-z]+ this turn` (the **precise ThisTurn head**) | **1** | Stolen Uniform only — honest single-consumer for the exact `this turn` head. |
| `when you lose control of` (broad lose-control-trigger class) | **9** | Duplicity, Gustha's Scepter, Khârn the Betrayer, Krovikan Vampire, Magus of the Unseen, Ogre Geargrabber, Ray of Command, Seraph, Stolen Uniform. |
| `when you lose control of that Equipment … unattach it` (**block-D's recognizer class**) | **2** | **Stolen Uniform** + **Ogre Geargrabber** ("Whenever this creature attacks, gain control of target Equipment … until end of turn. Attach it to this creature. **When you lose control of that Equipment, unattach it.**"). |
| `unattach` (any effect/cost) | **46** | but NO effect-level `"unattach it" → UnattachAll` parser exists yet (see §3 trace). |
| `if it's attached to a creature you control` (intervening-if phrase) | **11** | Bride's Gown, Groom's Finery, Halvar, Miracle Worker, Nomad Mythmaker, One Last Job, Siona, Springheart Nantuko, The Companion of the Wilds, Would You Have Done the Same?, Stolen Uniform. |

**Class breakdown of the 9 "lose control" siblings** (measured): 5 self-referential (`this enchantment/artifact/creature`, self-name — Duplicity, Gustha's, Khârn, Krovikan, Seraph → handled today by the `~` self-ref path, oracle_trigger.rs:11714), 2 broad-anaphor (`the artifact/creature` — Magus, Ray of Command), **2 "that Equipment …unattach it" (Ogre + Stolen)** — the delayed-unattach class block-D unlocks.

**Build-for-the-class conclusion:** the recognizer head `"when you lose control of that <permanent-type>[ this turn]"` + `"unattach it"` leaf generalizes across **Stolen + Ogre** (parameterize the anaphor type, make `this turn` and the intervening-if optional via `opt`). The precise `this turn` head is honestly Stolen-only (COUNT 1); the delayed-unattach body is a real 2-card class. The one frozen-file arm (§5) is likewise needed by **both** Stolen and Ogre — a class fix, not a one-off.

---

## 2. Existing-building-block trace (file:line — reuse, don't rebuild)

### 2.1 DelayedTrigger recognizer family — `parser/oracle_effect/mod.rs`
- **Dispatch host:** `parse_effect_clause_inner(text, ctx: &mut ParseContext)` — **mod.rs:5868**. Carries the OUTER `ctx` (with `ctx.declared_target_slots` populated by the front-half `try_parse_two_targets`, and `ctx.subject = ParentTargetSlot{1}` set after GainControl per B3 §3.4). Delayed-trigger arms chain at **mod.rs:6308** (`try_parse_whenever_this_turn`), **:6316** (`try_parse_reflexive_this_way_trigger`), **:6321** (`try_parse_when_next_event`).
- **Builder to reuse — `build_when_next_delayed_trigger(mode, valid_card, inner, or_branch)` — mod.rs:815.** Produces exactly `Effect::CreateDelayedTrigger { condition: WhenNextEvent { trigger{mode, valid_card, valid_target: Controller}, or_trigger, lifetime: ThisTurn }, effect: inner, uses_tracked_set: false }`. This is the **exact shape** the runtime expects (see 2.2).
- Analogues traced end-to-end: `try_parse_when_next_event` (mod.rs:907), `try_parse_when_next_generic_event` (mod.rs:995, delegates condition recognition to `parse_trigger_condition`), `try_parse_reflexive_this_way_trigger` (mod.rs:1064, splits on `" this way, "`). None match `"when you lose control of …"` (no `next`, no `this way`, `whenever`≠`when`), so no collision.

### 2.2 Lose-control RUNTIME (cdb373503) — already Stolen-shaped, do NOT rebuild
- **Target trigger shape** (`crates/engine/tests/lose_control_this_turn_delayed_trigger.rs::install_lose_control_draw`): `TriggerDefinition::new(TriggerMode::ChangesController)`, `valid_card = Some(<the equipment>)`, `execute = None`, wrapped in `DelayedTriggerCondition::WhenNextEvent { or_trigger: None, lifetime: DelayedTriggerLifetime::ThisTurn }`, `one_shot: true`.
- **Matcher — `game/trigger_matchers.rs::match_changes_controller`.** Checks `valid_card_matches` + a direction gate; its `source_id != object_id` branch comment **names Stolen Uniform verbatim** ("CR 603.2: delayed/`SpecificObject` case (Stolen Uniform). The source is the graveyard spell whose controller is the player who temporarily held the object; firing only when `old_controller == source.controller` …"). **`valid_target` is NOT consulted** → `build_when_next_delayed_trigger`'s `valid_target: Controller` is benign; reuse it directly.
- Control-loss `ControllerChanged` emitted at cleanup when the until-EOT control effect ends (CR 514.2/514.3a/613.1b, cdb373503).

### 2.3 `UnattachAll` effect + resolver — reuse for the "unattach it" leaf AND the intervening-if
- **Def — `types/ability.rs:8819`:** `UnattachAll { attachment: TargetFilter, target: TargetFilter }` — doc: "`attachment` scopes which attached objects move; **`target` scopes the host object.**"
- **Resolver — `game/effects/attach.rs::resolve_unattach_all`:** `target_ids = resolved_object_ids_for_filter(state, ability, target_filter)` (the **hosts**); for each host, iterate its `attachments`, unattach those matching `attachment_filter`. → `UnattachAll{ attachment: <the equipment>, target: <creature you control> }` unattaches the equipment **iff its host is a creature you control** — the intervening-if, folded into the effect (see §3.3).
- **No parser recognizer for `"unattach it" → UnattachAll` exists** — only a *cost* form (`oracle_cost.rs:917` `"unattach this equipment"/"unattach ~"`) and a *"becomes unattached" trigger* (`oracle_trigger_snapshot_tests.rs`). The effect leaf is genuinely new (§3.2).

### 2.4 Anaphor "that Equipment" → slot — reuse front-half §3.3
- `parser/oracle_target.rs::parse_definite_parent_reference(input, &ctx.declared_target_slots)` (front-half, B3 §3.3) already resolves `"that Equipment"` → `ParentTargetSlot{1}` (proven by the live GainControl parse). Block-D's head reuses it (or `ctx.subject`, which the front half set to `ParentTargetSlot{1}`).

### 2.5 s07 `ParentTargetSlot` snapshot infra (ebca1fab0, IN TREE) — asymmetric coverage
- **FIRING condition path — WORKS.** `delayed_trigger.rs:56 bind_contextual_filter_to_condition(&mut condition, &ability.targets)` → `concrete_parent_target_filter` has a `ParentTargetSlot{index}` arm (**delayed_trigger.rs:323**) binding to `parent_targets.get(index)`. `parent_target_snapshot` returns `ability.targets` (the outer Stolen chain's accumulated `[C, E]`, **delayed_trigger.rs:181**), so `valid_card = ParentTargetSlot{1}` → concrete E. ✅
- **EFFECT-BODY path — BROKEN (the gap).** `delayed_trigger.rs:124` gates the snapshot on `effect_refs_parent_target(&delayed_ability.effect)`. That calls `effect_parent_ref_slots` (**mod.rs:4034**), which surfaces only `effect.target_filter()` + a fixed set of hidden-slot arms (`CopyTokenOf`, `Token`, **`Attach{attachment}` at :4045**) — **no `UnattachAll` arm.** For `UnattachAll`, `target_filter()` surfaces the **`target`** (host) slot only (`ability.rs` `target_filter()`, `UnattachAll { target, .. }` arm), so `attachment: ParentTargetSlot{1}` is invisible → detector returns `false` → **no snapshot** → `delayed_ability.targets = []` (mod.rs:149) → at fire time `attachment: ParentTargetSlot{1}` resolves against `[]` → **nothing unattached.** ✗

---

## 3. Design (parser-only; nom-first; zero new engine variants)

### 3.1 Recognizer `try_parse_lose_control_delayed_trigger(tp: TextPair, ctx: &mut ParseContext) -> Option<ParsedEffectClause>`
Add to the `parse_effect_clause_inner` dispatch chain (mod.rs, alongside :6316–:6321). It needs `ctx` (for the slot registry), unlike the sibling arms — pass `ctx` through. Nom-first, all combinators / existing building blocks:

1. **Split head/body on the FIRST comma** (`tp.split_around(", ")` — the comma after `this turn`; the body keeps its own inner comma). Existing `TextPair` splitter, as used by the reflexive/when-next arms.
2. **Head** = `tag("when you lose control of ")` → resolve the `"that <type>"` anaphor via `parse_definite_parent_reference(rest, &ctx.declared_target_slots)` → `ParentTargetSlot{index}` (Equipment ⇒ slot 1) → then `opt(tag(" this turn"))`. Reject (return `None`) if the anaphor does not resolve to a slot — no silent guess.
3. **Body** (nom): `opt(preceded(tag("if it's attached to "), parse_host_filter))` where `parse_host_filter` reuses `parse_target`/`parse_type_phrase` → `Typed{ Creature, controller: You }` (the `"you control"` qualifier captured compositionally, NOT dropped); then `tag("unattach it")`, `all_consuming` on the remainder (trailing `.` stripped).
4. **Assemble** (see §3.2/§3.3), then `build_when_next_delayed_trigger(TriggerMode::ChangesController, ParentTargetSlot{index}, inner_ability, None)` → the exact 2.2 runtime shape.

**Guard is narrow** (`"when you lose control of "`), so shared-path collision risk is low; still a coverage-diff seam (§6).

### 3.2 `"unattach it"` leaf
`Effect::UnattachAll { attachment: ParentTargetSlot{index}, target: <host filter | Any> }`. **No new `Effect` variant** — `UnattachAll` (CR 701.3d) already exists. Ponytail: a singular `Effect::Unattach` sibling would be a parameterization-that-didn't-happen; `UnattachAll` with an `attachment` bound to one object IS the single-object case.

### 3.3 Intervening-if — FOLDED into `UnattachAll.target` (host scope), NOT a separate condition
The intervening-if `"if it's attached to a creature you control"` is represented by setting `UnattachAll.target = Typed{Creature, controller: You}` (the host filter from §3.1.3); absence ⇒ `target: Any` (Ogre's no-if form). Because `resolve_unattach_all` scopes `target` to the **host**, this yields precisely "unattach the equipment iff its host is a creature you control."

**Why fold instead of B3 §3.5's `target: Any` + a separate `AbilityCondition`:**
- **Strictly more correct.** The only existing "is attached to a creature" condition is `StaticCondition::SourceAttachedToCreature` (`oracle_nom/condition.rs:1302`), which is (a) **source-relative** — but Stolen's `"it"` is the equipment (`ParentTargetSlot{1}`), while the delayed trigger's `source_id` is the graveyard spell → wrong subject; and (b) **drops `"you control"`** ("consumed but not represented in the AST"). Folding into the host filter preserves `"you control"` and binds the correct subject.
- **Zero new variants.** A target-relative "attached to a creature you control" condition would need a NEW `AbilityCondition` (or a new host-controller attachment `FilterProp` for `TargetMatchesFilter`, whose only fields are `{filter, use_lki}`) → the `add-engine-variant` gate. Folding avoids it entirely. B3 §3.5/§8 explicitly hedged ("do not invent a new condition variant without first running /add-engine-variant") — this is the reuse that resolves that hedge.
- **Rules-equivalent for this body.** CR 603.4's intervening-if is checked on trigger-placement and on resolution; folding checks only at resolution. For a body whose *sole* effect is the unattach, the observable game state is identical (a false host filter unattaches nothing; a false intervening-if also does nothing). `// ponytail: intervening-if folded into UnattachAll host scope; CR 603.4 stack-presence distinction is immaterial for a pure-unattach body.`

This is a **deliberate, justified deviation from B3 §3.5** (which specified `target: Any` + a separate intervening-if). It still satisfies B3 **§7.2** (emit `ParentTargetSlot{1}` in BOTH the `valid_card` AND the effect body's `attachment`). Flag the deviation for the plan reviewer with the above measured justification.

### 3.4 Resulting AST (block-D emits)
```
CreateDelayedTrigger {
  condition: WhenNextEvent {
    trigger: TriggerDefinition { mode: ChangesController,
                                 valid_card: ParentTargetSlot{1},   // §7.2, s07-bound to E ✅ (2.5)
                                 valid_target: Controller },        // ignored by matcher (2.2)
    or_trigger: None, lifetime: ThisTurn },
  effect: UnattachAll { attachment: ParentTargetSlot{1},            // §7.2 — s07 gap (§5) ✗
                        target: Typed{Creature, controller:You} },  // folded intervening-if
  uses_tracked_set: false }
```

---

## 4. CR verification (grepped against `docs/MagicCompRules.txt` — present)

| CR | Verified text (grep) | Use |
|---|---|---|
| CR 603.7 | "An effect may create a delayed triggered ability that can do something at a later time." | the `CreateDelayedTrigger` container |
| CR 603.4 | "A triggered ability may read 'When/Whenever/At [trigger event], if [condition], [effect]'…" | the intervening-if |
| CR 603.2 | "Whenever a game event or game state matches a triggered ability's trigger event…" | ChangesController match |
| CR 701.3d | "To 'unattach' an Equipment from a creature means to move it away from that creature…" | `UnattachAll` leaf |
| CR 514.2 / CR 514.3a | cleanup-step simultaneous actions / SBA check | control-loss emitted at end of turn |
| CR 613.1b | "Layer 2: Control-changing effects are applied." | the control reversion producing the loss |
| CR 608.2c | "The controller … follows its instructions in the order written…" | `ParentTargetSlot` accumulation ([C,E]) |

(Re-grep at implementation time per the CR-annotation protocol; annotate the new recognizer + leaf.)

---

## 5. add-engine-variant gate — NOT triggered by parser work; ONE frozen-file arm FLAGGED

- **No new engine variant is introduced by block-D.** Reuses `Effect::CreateDelayedTrigger`, `DelayedTriggerCondition::WhenNextEvent`, `DelayedTriggerLifetime::ThisTurn`, `TriggerMode::ChangesController`, `Effect::UnattachAll`, `TargetFilter::ParentTargetSlot`, `TargetFilter::Typed`. The `add-engine-variant` checklist is therefore N/A for the parser change (verify with `cargo engine-inventory` grep at impl time to confirm no accidental sibling).

- **⛔ REQUIRED s07-frozen edit (STOP-and-coordinate) — `game/effects/mod.rs::effect_parent_ref_slots` (mod.rs:4034).** Add, mirroring the existing `Attach` arm at mod.rs:4045:
  ```rust
  Effect::UnattachAll { attachment, .. } if attachment.is_context_ref() => slots.push(attachment),
  ```
  `is_context_ref()` includes `ParentTargetSlot` (`ability.rs`), so this arm fires for `attachment: ParentTargetSlot{1}` → `effect_refs_parent_target` returns `true` → `parent_target_snapshot` seeds `delayed_ability.targets = [C, E]` → `ParentTargetSlot{1}` → E → the equipment actually unattaches.
  - **This file is on the s07-frozen §7.3 list (`game/effects/mod.rs`).** Block-D must NOT edit it. Hand off to the s07 driver as a **4th snapshot site** extending their existing 3-site `ParentTargetSlot` increment (natural: same file, same mechanism, same root as the `Attach` arm). Ogre Geargrabber needs the identical arm → class fix.
  - **Without this arm, block-D is a hollow coverage win:** the trigger fires (valid_card binds via the working firing path, 2.5) but the body unattaches nothing. Do NOT mark Stolen `supported` until this lands (verification claim 8 is the joint gate).

---

## 6. Coverage-regression note (CI is authoritative)

The recognizer is added to the shared `parse_effect_clause_inner` path. Its guard (`"when you lose control of that "`) is narrow, but this is a shared trigger/effect seam. **Impl MUST run the empirical before/after coverage diff** (`cargo coverage` / the Tilt `card-data` resource + `coverage-parse-diff`) and confirm zero net regressions — per the "parser coverage regression is CI-only" memory, `cargo test -p engine` will NOT catch a swallowed-clause regression on unrelated cards. Sanity: the 9 "lose control" siblings — confirm the 5 self-ref ones still route to the `~` path (oracle_trigger.rs:11714, `all_consuming`, unaffected) and the 2 broad-anaphor ones are untouched.

---

## 7. Discriminating verification matrix (non-vacuous; real parse + cast pipeline)

| # | Claim | Seam | Test (runtime `GameScenario` + `GameRunner::cast().resolve()` unless noted) | Revert-failing assertion | Hostile / sibling |
|---|---|---|---|---|---|
| 1 | Last sentence parses to `CreateDelayedTrigger{ChangesController, ThisTurn}`, not `Unimplemented` | §3.1 | Parser shape test on the full card. | Assert the 5th clause `type == CreateDelayedTrigger` (baseline: `Unimplemented("when")`, §0). | Head without a resolvable slot → `None`/`Unimplemented` (no wrong guess). |
| 2 | `valid_card == ParentTargetSlot{1}` and effect `attachment == ParentTargetSlot{1}` (§7.2) | §3.1/§3.2 | Parser shape test. | Assert both are `ParentTargetSlot{index:1}`, not `ParentTarget`/`Any`. | — |
| 3 | Intervening-if folded: `UnattachAll.target == Typed{Creature,You}` (with-if) vs `Any` (Ogre no-if) | §3.3 | Parser test on Stolen (host filter) and on the isolated Ogre clause (`Any`). | Assert `target == Typed{Creature, controller:You}` for Stolen (drop of `you control` → `Any` fails it). | Ogre variant → `Any`. |
| 4 | Firing condition binds to the concrete equipment (s07 path, 2.5) | s07 (pre-existing) | Cast Stolen (your creature C + opponent's Equipment E) → resolve → cause you to lose control of E at EOT cleanup → assert the delayed trigger's `valid_card` matched E and the trigger fired. | Buggy s07 (no ParentTargetSlot arm in `concrete_parent_target_filter`) → never fires. | Unrelated control change on another object must NOT fire (matcher `valid_card` gate). |
| **8** | **B1↔B3 integration — the trigger unattaches ONLY E** | §3.1–3.3 **+ the §5 s07 `UnattachAll` arm** | Cast Stolen (C yours + E opponent's) → resolve (E controlled by you, attached to C) → lose control of E at EOT → assert **E is unattached from C**, C intact. | **Revert-fails on BOTH:** (a) drop the §5 arm → `delayed_ability.targets=[]` → E never unattaches (the hollow-win check); (b) front-half slot-collision → wrong object. | **Hostile:** a 2nd Equipment F you control attached to C (or elsewhere) stays attached — proves `attachment: ParentTargetSlot{1}` binds E specifically, not "all attachments". **Hostile-if:** E attached to an OPPONENT's creature → NOT unattached (proves the folded `you control` host scope). |

**Non-vacuity:** claim 1 flips vs the §0 baseline (`Unimplemented`); claim 8(a) flips on removing the §5 arm; claim 8 hostile-F flips if `attachment` were `Any`; claim 8 hostile-if flips if the host filter were `Any`. Attach the §0 live-parse dump as the impl-PR baseline.

---

## 8. Files touched

| File | Change | Scope |
|---|---|---|
| `parser/oracle_effect/mod.rs` | new `try_parse_lose_control_delayed_trigger(tp, ctx)`; wire into `parse_effect_clause_inner` dispatch (~:6316–6321); reuse `build_when_next_delayed_trigger` | **block-D** |
| `parser/oracle_effect/mod.rs` (or a leaf helper module) | `"unattach it" → UnattachAll{attachment, target}` leaf combinator; folded intervening-if host-filter extraction (nom: `tag` + `parse_target`) | **block-D** |
| `parser/oracle_effect/tests.rs` (+ a runtime test file) | claims 1–4 parser shape tests + the claim-8 cast/control-loss integration test | **block-D** |
| `game/effects/mod.rs` — `effect_parent_ref_slots` | **⛔ s07-OWNED (§7.3 frozen).** Add `UnattachAll{attachment,..} if attachment.is_context_ref()` arm. **FLAG to s07 driver; do NOT edit in block-D.** | **s07 (coordinate)** |
| `game/effects/delayed_trigger.rs`, `game/filter.rs` | untouched — s07-frozen | **OUT** |

---

## 9. Ship-gate statement (final)

- **Parser-only (block-D scope):** achievable now — Stolen's last sentence parses to a real `CreateDelayedTrigger` (no `Unimplemented`), flipping coverage to `supported:true, gap:0`. Zero new engine variants. Firing-condition binding already works (s07, 2.5).
- **But that is a hollow win** until the s07-frozen `game/effects/mod.rs::effect_parent_ref_slots` gains the one-line `UnattachAll` arm (§5). Without it the trigger fires and unattaches nothing (measured: `delayed_ability.targets=[]`). **STOP-and-report: coordinate the §5 arm with the s07 driver before marking Stolen supported.** Verification claim 8 is the joint acceptance gate for block-D + that s07 arm meeting.

---
## REVIEW CONDITIONS (ae2a3166 — APPROVE-WITH-CONDITIONS; MUST honor at impl)

Core design VERIFIED sound: recognizer (build_when_next_delayed_trigger@mod.rs:815 → matcher match_changes_controller@trigger_matchers.rs:2886 names Stolen verbatim; concrete_parent_target_filter ParentTargetSlot arm@delayed_trigger.rs:329 binds concrete E); the §3.5 FOLD (intervening-if → UnattachAll.target=Typed{Creature,You}) SOUND — resolve_unattach_all(attach.rs:88-118) scopes HOSTS by target filter, so E on an opponent's creature is never a host → correctly NOT unattached; zero new variants; ship-gate WITH cherry-picked 1b04e15c2 arm reaches Stolen supported:true/gap:0 runtime-correct. NOM + CRs + frozen-safety all verified.

- **C1 (MUST-FIX — false class claim, measured-facts rule).** class=2 is FALSE. `declared_target_slots` is populated ONLY by the DUAL-target head parser `try_parse_two_targets` (imperative.rs:3958); no single-target registration exists. Ogre Geargrabber is SINGLE-target ("gain control of target Equipment") → its slots stay empty → `parse_definite_parent_reference` returns None (oracle_target.rs:1515 `if slots.is_empty() { return None }`) → recognizer never fires → **Ogre stays Unimplemented**. Block-D recognizer is class-general in SHAPE but runtime-unlocks **Stolen ONLY**. DOWNGRADE the narrative: "general recognizer; runtime unlock Stolen-only until the front-half registers single-target gain-control slots (deliberately NOT done — would reopen broad single-target anaphor coverage-regression risk; Ogre is out-of-tranche)." Honest single-consumer for now.
- **C2 (MUST-FIX — Ogre test discrimination).** Do NOT hand-feed a slot for the Ogre test (would falsely pass). Add a FULL-CARD Ogre parse assertion documenting the CURRENT reality (Ogre's last sentence STILL Unimplemented — empty registry, single-target). This empirically pins C1's limitation honestly rather than claiming a false unlock.
- **C3 (SHOULD-FIX — CR annotation precision).** Sharpen the §3.3 comment: the fold DROPS CR 603.4's placement-time (first) check — the trigger fires unconditionally and no-ops when the host filter is empty. Name the practically-unreachable reorder divergence (E on opponent's creature at loss-of-control, then moved onto your creature before resolution — impossible: equip is sorcery-speed, can't respond to a cleanup trigger). Do NOT read the fold as full CR 603.4 equivalence.
- **C4 (NOTE — resolved).** STOP-and-coordinate language is RESOLVED by the confirmed cherry-pick of 1b04e15c2 (adds the exact UnattachAll snapshot arm). Reframe §5/§9 to "arm present via cherry-pick; claim-8 is the joint acceptance gate."

Bottom line: ship block-D for Stolen (in-tranche must-ship) after C1/C2 honest-downgrade; design needs no new variants.
