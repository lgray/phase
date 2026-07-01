# Implementation Plan — Loki, God of Mischief (MSH)

> **Worktree:** `/home/lgray/vibe-coding/wt-msh-loki` (base `c1b61ded5`). All file:line anchors below were re-verified against this worktree on 2026-06-27. Do NOT edit `/home/lgray/vibe-coding/phase-rs-workdir` source. This planning file lives in the main worktree's gitignored `.planning/` and is NEVER part of any PR.

## Card

```
Loki, God of Mischief
Whenever a player or permanent becomes the target of an ability you control,
draw a card. This ability triggers only once each turn.
```

(MDFC; this plan addresses the quoted triggered-ability face only — the other face / typeline is out of scope and parses independently.)

---

## 0. Headline architectural decision

**No new `TriggerMode` variant. No new event. No new constraint. No new `TargetFilter` variant.** The `BecomesTarget` machinery already exists and is the *exactly correct* parameterized mode; Loki is a new *value combination* of its existing axes, not a new structural axis. This satisfies "parameterize, don't proliferate": the three axes the prompt names (target kind, source kind, controller relationship) are already encoded as data, not as enum siblings:

| Prompt axis | Existing encoding (typed, not bool) | Loki's value |
|---|---|---|
| target kind (player \| permanent \| any) | `TriggerDefinition.valid_target: Option<TargetFilter>` (player axis) **+** `valid_card: Option<TargetFilter>` (object axis) | `valid_target = Player`, `valid_card = Typed(Permanent) ∧ InZone(Battlefield)` |
| source kind (ability \| spell \| any) | `TriggerDefinition.valid_source: Option<TargetFilter>` via `StackAbility` / `StackSpell` | `valid_source = StackAbility { .. }` (ability only — no spell) |
| controller relationship (you \| opponent \| any) | `TargetFilter::StackAbility { controller: Option<ControllerRef>, .. }` (`types/ability.rs:3471`) | `controller: Some(ControllerRef::You)` |

The work is **four surgical edits** plus tests:

1. **Matcher** (`trigger_matchers.rs:2343-2347`): make the player-target and object-target axes *independent* so one `TriggerDefinition` fires for **both** a player target (via `valid_target`) and a permanent target (via `valid_card`). Today the Player arm requires `valid_card.is_none()`, which is mutually exclusive with the Object arm and silently drops Loki's player-target draws.
2. **Parser — subject split** (`oracle_trigger.rs:7292` `set_trigger_subject`): when the subject is an `Or` mixing a player leaf and object leaf(s), route the player leaf → `valid_target` and the object leaf(s) → `valid_card`. General building block for the whole "a player or <permanent-type>" subject class.
3. **Parser — new source arm** (`oracle_trigger.rs:8109` `parse_simple_event` + dispatch at `8296`): add `BecomesTargetAbility` recognizing "becomes the target of an ability" (+ optional "you control" / "an opponent controls") → `valid_source = StackAbility { controller, tag: None, kind: None }` (ability-only; **excludes** the spell branch the existing spell-or-ability arm ORs in). **F1 guard:** the new tag is a *prefix* of source-restricted siblings (Skophos Maze-Warden, Agrus Kos), so the dispatch arm consumes the optional controller clause via a tail-exposing wrapper and then **rejects any non-empty remainder → Unknown** (scoped to this arm only; see §3b).
4. **Parser — battlefield gate** for the permanent leaf so a targeted *graveyard card* (also a `TargetRef::Object`) does NOT trigger (CR 110.1). Implemented by attaching `FilterProp::InZone { zone: Battlefield }` to the permanent leaf in the new arm.

The "only once each turn" limiter and the `Draw` effect body require **zero new code** — both already parse and wire automatically (see §4, §5).

---

## 1. TriggerMode design (axes & rationale)

**Chosen mode:** `TriggerMode::BecomesTarget` (`types/triggers.rs:302`). Verified doc at `types/triggers.rs:300-303`:

```rust
// Targeting — CR 115 (Targets)
/// CR 603.2e: "Becomes the target" trigger — fires when a spell/ability targets an object.
BecomesTarget,
BecomesTargetOnce,
```

- `BecomesTargetOnce` (`:303`) is the **batched** sibling (CR 603.2c — once per simultaneous batch), NOT the "once each turn" mechanism. Loki uses plain `BecomesTarget` + a separate `TriggerConstraint::OncePerTurn` (§4). Do not conflate.
- Index bucket `TriggerEventKey::BecomesTarget` (`types/triggers.rs:45`) — both modes map to it (`trigger_index.rs:241`). No change.

**Why no new variant / no parameterization refactor:** run the `add-engine-variant` gate mentally — the proposed "axes" are *leaf-level value parameterizations of existing structural axes already present on `TriggerDefinition`* (`valid_target`/`valid_card`/`valid_source`), each backed by a typed enum (`TargetFilter`, `ControllerRef`, `StackAbilityKind`). Adding a `BecomesTargetOfAbilityYouControl` sibling would be precisely the sibling-cluster smell the CLAUDE.md prohibits. The categorical-boundary rule is also satisfied: every axis lives inside CR 115 (targeting) — we are not crossing rule sections.

`make_trigger` test helper (`trigger_matchers.rs:4461`) already constructs `BecomesTarget` triggers, confirming the mode is fully wired for unit testing.

---

## 2. The MATCHER — the #1 risk, addressed head-on

### Seam where targeting is finalized and observable

`emit_targeting_events(state, targets, source_id, controller, events)` — **`crates/engine/src/game/casting.rs:241-280`**. This is the single shared chokepoint that pushes `GameEvent::BecomesTarget { target, source_id }` once per locked-in target: objects at `casting.rs:252`, players at `casting.rs:265`. Doc (`casting.rs:237-240`): *"Called whenever targets are locked in for a spell **or** ability."* Verified callers covering **abilities** (not just spells):

- Activated abilities / activation costs: `casting_costs.rs:2475, 2490, 2502, 2533, 5587`
- Planeswalker loyalty (activated) abilities: `planeswalker.rs:286`
- Triggered-ability target assignment: `triggers.rs:4283`
- Modal target selection: `engine_modes.rs:245`; spell targets: `casting_targets.rs:304, 411`; stack push: `engine_stack.rs:25`

**Conclusion: the seam already exists and is clean — no new emission seam is required.** This is the load-bearing de-risking finding: "a player/permanent becomes the target of an ability the Loki-controller controls" is fully observable today. CR grounding: CR 601.2c (the parenthetical — abilities that trigger when objects/players "become the target" trigger at the point targets are chosen) and CR 602.2b (activated-ability targeting reuses the 601.2 cast steps). Both verified in §6.

### Matcher function & registry

`match_becomes_target` — **`trigger_matchers.rs:2286-2349`**; registry `trigger_matchers.rs:72` (dispatch) and `277-278` (`r.insert(...)`). No registry change.

The `valid_source` block (`:2302-2332`) already: finds the targeting stack entry (or `resolving_stack_entry`, CR 608.2), resolves the trigger controller from `state.objects.get(&source_id).controller` (`:2318-2322`), and runs `targeting::stack_entry_matches_filter(...)` (`:2323`). For Loki this evaluates `StackAbility { controller: Some(You), .. }` against the targeting ability — already supported at runtime via `stack_ability_matches_filter` (`targeting.rs:1246`) + `stack_entry_controller_matches` (`targeting.rs:1331`). **No change needed in the source block.**

### The fix — make the subject axes independent (`trigger_matchers.rs:2334-2348`)

Current code:

```rust
match target {
    TargetRef::Object(object_id) => {
        if trigger.valid_card.is_some() {
            valid_card_matches(trigger, state, *object_id, source_id)
        } else {
            *object_id == source_id            // self-only default
        }
    }
    TargetRef::Player(player_id) => {
        trigger.valid_card.is_none()           // <-- BLOCKS combined subject
            && trigger.valid_target.is_some()
            && valid_player_matches(trigger, state, *player_id, source_id)
    }
}
```

**Change:** ⚠️ **SUPERSEDED by the round-1 correction below.** The original plan dropped the `trigger.valid_card.is_none()` precondition on the Player arm and had it read `valid_target`. That over-fires Venerated Rotpriest (overloaded `valid_target`). The corrected Player arm reads a NEW, distinct `valid_subject_player` field set only by the subject Or-split — see "Review round 1 correction". The Object arm is unchanged in both versions. The code block below shows the original (buggy) approach, retained for the record.

```rust
match target {
    TargetRef::Object(object_id) => {
        if trigger.valid_card.is_some() {
            valid_card_matches(trigger, state, *object_id, source_id)
        } else {
            *object_id == source_id
        }
    }
    // CR 115.1 + CR 603.2e: object and player subject axes are independent. A
    // trigger like Loki ("a player or permanent") sets BOTH valid_target (player
    // leaf) and valid_card (object leaf); a player target must still match via
    // valid_target even when valid_card is populated for the object half.
    TargetRef::Player(player_id) => {
        trigger.valid_target.is_some()
            && valid_player_matches(trigger, state, *player_id, source_id)
    }
}
```

**Why this is safe (verified, non-regressive):**

- `valid_player_matches` (`trigger_matchers.rs:604-614`) returns `true` when `valid_target` is `None`. The retained `trigger.valid_target.is_some()` guard is therefore **load-bearing**: an *object-only* subject trigger (`valid_target == None`, e.g. Bonecrusher "becomes the target of a spell") still never fires on a player target. Removing the guard would be a bug; keeping it preserves exactly the old behavior for every existing card.
- ⚠️ **FALSE — corrected in "Review round 1 correction" below.** ~~No existing `BecomesTarget` trigger sets **both** `valid_card` and `valid_target` — `set_trigger_subject` (`:7292`) sets exactly one today. So dropping `valid_card.is_none()` changes behavior **only** for the new mixed-subject class we are introducing. The Object arm is untouched, so object-subject matching is bit-for-bit identical.~~ **Venerated Rotpriest is a baseline counterexample:** its OBJECT subject sets `valid_card=Typed(Creature,You)` AND its EFFECT ("target opponent gets a poison counter") sets `valid_target=Some(Player)` via the effect-target slot (`oracle_trigger.rs:1255-1268`). `valid_target` is OVERLOADED (subject-player filter ∪ effect-target slot), so dropping `valid_card.is_none()` and reading `valid_target` as a subject filter makes Rotpriest over-fire on ANY player being targeted. The corrected fix introduces a DISTINCT `valid_subject_player` field — see the correction section.
- Truth table after the change:
  - object-only (`valid_card=Some`, `valid_target=None`): Object→filter; Player→`false` (guard). Unchanged.
  - player-only (`valid_card=None`, `valid_target=Some`): Object→`object_id==source_id` (pre-existing self-default); Player→filter. Unchanged.
  - **Loki** (`valid_card=Some`, `valid_target=Some`): Object→permanent filter; Player→player filter. **Both fire — new, correct.**

### Graveyard-card discriminator (CR 110.1) — must be closed in the parser, not the matcher

`valid_card_matches` → `target_filter_matches_object` (`trigger_matchers.rs:716`) → `filter::matches_target_filter`. **Verified gap:** `TypeFilter::Permanent` (`filter.rs:2155-2162`) checks only `obj.card_types.core_types` — it does **not** check zone. A creature *card in a graveyard* is `TargetRef::Object` with `CoreType::Creature`, so a bare `Typed(Permanent)` filter would incorrectly match it. CR 110.1 (verified §6): "A permanent is a card or token **on the battlefield**." Therefore the permanent leaf MUST carry a battlefield zone gate.

**Fix location = parser (§3), not matcher.** The battlefield restriction belongs to the *meaning of the subject* "permanent", so it is encoded into the filter the parser emits. `FilterProp::InZone { zone }` already exists and evaluates correctly: `filter.rs:3398` → `obj.zone == *zone`. We must NOT add this gate globally to every "permanent" subject (it would break dies/leaves triggers whose object is in the graveyard at match time) — it is added **only** in the new becomes-target-ability arm. See §3.

---

## 3. The PARSER — nom decomposition

All edits in `crates/engine/src/parser/oracle_trigger.rs`. No verbatim full-sentence `tag()`; the phrase decomposes along its grammatical axes, each a reused building block.

### 3a. Subject "a player or permanent" — already parses to `Or`, must be *split*

`parse_trigger_subject` (`oracle_trigger.rs:6447`) parses "a player" → `TargetFilter::Player` (alt arm `oracle_trigger.rs:6739`), detects the "or " separator (`:6454-6459`), recurses on "permanent" → `parse_single_subject` → `parse_type_phrase` → `TypeFilter::Permanent` (the `value(TypeFilter::Permanent, tag("permanent"))` arm at `oracle_trigger.rs:4741`), then `merge_or_filters(Player, Typed(Permanent))` (`oracle_util.rs:1170`) ⇒ `Or { [Player, Typed(Permanent)] }`. **This already works — no subject-parser change.**

The defect is **routing**: `set_trigger_subject` (`oracle_trigger.rs:7292-7298`) is binary — `subject_is_player(subject)` (`:7278`) is `false` for an `Or`, so the *entire* `Or` (player leaf included) is dumped into `valid_card`, where the player leaf is dead weight and the matcher's Player arm never consults it.

**Edit — extend `set_trigger_subject` to partition `Or` subjects (general building block):**

```rust
fn set_trigger_subject(def: &mut TriggerDefinition, subject: &TargetFilter) {
    if subject_is_player(subject) {
        def.valid_target = Some(subject.clone());
    } else if let TargetFilter::Or { filters } = subject {
        // CR 115.1: A mixed "a player or <permanent>" subject spans both target
        // axes. Route player leaves -> valid_target, object leaves -> valid_card,
        // so the matcher can fire on either kind independently.
        let (players, objects): (Vec<_>, Vec<_>) =
            filters.iter().cloned().partition(subject_is_player);
        if players.is_empty() {
            def.valid_card = Some(subject.clone());   // pure object Or — unchanged
        } else {
            def.valid_target = Some(collapse_or(players));   // -> Player
            if !objects.is_empty() {
                def.valid_card = Some(collapse_or(objects)); // -> Typed(Permanent)
            }
        }
    } else {
        def.valid_card = Some(subject.clone());
    }
}
```

- `collapse_or(vec)` = single element → that element, else `TargetFilter::Or { filters }`. (Add as a tiny local helper, or reuse `merge_or_filters` fold; a 4-line helper is clearest.)
- **Non-regression:** when the `Or` has no player leaf (`players.is_empty()` — every existing "artifact or creature you control" subject), behavior is byte-identical to today (`valid_card = Some(Or)`). The new branch only activates for mixed player+object subjects, the class we are adding. This is the building-block fix: it covers "a player or permanent", "a player or creature", "a player or planeswalker", etc., not just Loki.
- For Loki: `valid_target = Player`, `valid_card = Typed(Permanent)` (before the §3c zone gate).

### 3b. Source "an ability" + controller "you control" — new arm

Reused building blocks:
- `parse_target_source_controller(rest)` (`oracle_trigger.rs:150-161`) — already recognizes `"you control"` → `ControllerRef::You` and `"an opponent controls"` → `ControllerRef::Opponent`, **but it returns only `Option<ControllerRef>` and discards the leftover tail** (`.ok().map(|(_, controller)| controller)` at `:159-160`). The new arm needs the tail to enforce a remaining-empty guard (see F1 below), so it must be refactored / wrapped to expose the post-controller remainder. **Reused, with a tail-exposing wrapper (§3b-tail).**
- `TargetFilter::StackAbility { controller, tag, kind }` (`types/ability.rs:3471`) — the typed source filter; runtime-matched by `stack_ability_matches_filter` (`targeting.rs:1246`).

**Add `SimpleEvent::BecomesTargetAbility` to the enum** (`oracle_trigger.rs:8049` region) and a recognizer arm. **nom 8.0's `alt` tuple-arity ceiling is 21** (`alt_trait! A..U`). MEASURED at base `c1b61ded5`: the first block has 20 elements and the **second** `.or(alt((...)))` block (`oracle_trigger.rs:8192-8250`) is already at **21/21** (full — the Backup arm fills it); adding any arm there overflows to 22 and **will not compile** (no `Choice` impl for a 22-tuple — collapsing the two forms into one nested `value(_, alt((becomes, become)))` still overflows the outer tuple to 22). The **third** `.or(alt((...)))` block (`oracle_trigger.rs:8251-8269` — 7 `value()` + the bare `parse_becomes_unattached` = **8 elements**) has room for 13 more, so **place the new arm in the THIRD block** (or open a fresh `.or(alt((...)))` block). Do **NOT** add it to the second block:

```rust
// CR 115.1 + CR 602.2b: "becomes the target of an ability [you control]".
// Ability-only source (excludes spells) — distinct from the spell-or-ability
// arm. Loki, God of Mischief.
value(
    SimpleEvent::BecomesTargetAbility,
    tag("becomes the target of an ability"),
),
// CR 115.1: batched plural form.
value(
    SimpleEvent::BecomesTargetAbility,
    tag("become the target of an ability"),
),
```

#### 3b-tail. Controller-tail refactor (F1 blocker)

`parse_target_source_controller` currently throws away the parse remainder. Refactor it to surface the tail so the new arm can assert the post-controller remainder is empty. Minimal, non-breaking shape — keep the existing function for the two callers that only want the controller, add a tail-returning sibling and have the old one delegate:

```rust
/// CR 115.1: parse an optional source-controller clause off the front of `rest`,
/// returning BOTH the recognized controller (if any) and the unconsumed tail.
/// `None` controller + full `rest` back means no clause was present.
fn parse_target_source_controller_tail(rest: &str) -> (Option<ControllerRef>, &str) {
    let rest = rest.trim_start();
    match alt((
        value(ControllerRef::You, tag::<_, _, OracleError<'_>>("you control")),
        value(ControllerRef::Opponent, tag("an opponent controls")),
    ))
    .parse(rest)
    {
        Ok((tail, controller)) => (Some(controller), tail),
        Err(_) => (None, rest),
    }
}

// Existing callers (BecomesTargetSpellOrAbility :8303) keep their behavior:
fn parse_target_source_controller(rest: &str) -> Option<ControllerRef> {
    parse_target_source_controller_tail(rest).0
}
```

This is a pure refactor: the **single** existing call site of `parse_target_source_controller` (`:8303`, the spell-or-ability arm — measured: it is the only caller in `crates/engine/src/`) is byte-identical — it still ignores the tail, so **no existing card changes** (the shared spell arm keeps dropping its trailing text exactly as today; see the corpus note below for why that must not change).

**Dispatch arm** (in the `match event { .. }` at `oracle_trigger.rs:8296+`, alongside the existing `BecomesTargetSpellOrAbility` / `BecomesTargetSpell` / `BecomesTargetBackupAbility` arms; the recognizer lives in the **third** `.or(alt(..))` block — §3b above — so it is reached only after every earlier becomes-target arm has failed):

```rust
// CR 115.1 + CR 602.2b: ability-only targeting source (no spell branch).
// "you control" / "an opponent controls" restricts the source controller.
// F1 guard: after consuming the OPTIONAL controller clause, the remainder MUST
// be empty (modulo whitespace) or we fall through to Unknown. This rejects
// source-restricted siblings whose tail this arm cannot model — Skophos
// Maze-Warden ("...of an ability OF A LAND you control named...") and Agrus Kos
// ("...of an ability THAT TARGETS ONLY IT...") — instead of silently dropping
// the restriction and over-firing. Guard is scoped to THIS arm only (see corpus
// note) — the shared spell-or-ability arms are NOT touched.
SimpleEvent::BecomesTargetAbility => {
    let (controller, tail) = parse_target_source_controller_tail(remaining);
    if !tail.trim().is_empty() {
        return None; // unmodellable source restriction -> Unknown
    }
    def.mode = TriggerMode::BecomesTarget;
    set_trigger_subject(&mut def, &battlefield_scope_permanent(subject)); // see 3c
    def.valid_source = Some(TargetFilter::StackAbility {
        controller,            // Some(You) for Loki; None = any controller
        tag: None,
        kind: None,
    });
}
```

Trace of the guard on the three-card interception surface (the *only* cards whose text reaches this arm — see corpus note):
- **Loki** — `remaining = " you control"` → controller clause consumes `"you control"` → `tail = ""` → empty → `BecomesTargetAbility { controller: Some(You) }`. ✅ parses.
- **Skophos Maze-Warden** — `remaining = " of a land you control named Labyrinth of Skophos"` → controller clause sees `"of a land…"` (NOT `"you control"`/`"an opponent controls"` at the front) → no match → `tail = "of a land you control named…"` → **non-empty → return None → Unknown**. ✅ stays inert. (The embedded `"you control"` is mid-phrase, never at the front, so it is never consumed.)
- **Agrus Kos, Eternal Soldier** — `remaining = " that targets only it"` → no controller clause → `tail = "that targets only it"` → **non-empty → Unknown**. ✅ stays inert.

**Collision safety — PREFIX case (F1, the dimension round-1 missed).** The round-1 note below only checked that the *spell arms* don't swallow "of an ability". The real risk is the reverse: the NEW tag `tag("becomes the target of an ability")` is a **prefix** of longer trigger conditions, so it could intercept cards that should stay Unknown and flip them to active-but-wrong. **Measured interception surface** (`data/card-data.json`, regex `becomes? the target of an ability`): **exactly 3 cards** — Loki (intended), Skophos Maze-Warden, Agrus Kos — and the other two are exactly the over-firing victims the guard above neutralizes. No other card in the corpus contains "of an ability". The guard converts the prefix-collision from a silent over-fire into a clean fall-through to Unknown for both non-Loki cards.

**Collision safety — spell arms (round-1, still valid):** input "becomes the target of an ability you control" — the existing `tag("becomes the target of a spell or ability")` arm (`:8123`) consumes "becomes the target of a" then requires " spell…" but sees "n ability" → fails; `tag("becomes the target of a spell")` (`:8155`) and `"...of an aura spell"` (`:8136`) likewise don't match "an ability". No existing arm shares the "of an ability" suffix. The new tag is collision-free regardless of `alt` ordering, exactly as the Backup arm is.

#### 3b-corpus. Measured BecomesTarget corpus + guard scoping (F1(b))

**Question:** does adding/tightening a remaining-empty guard regress existing `BecomesTarget` cards?

**Decision: guard is scoped to the NEW `BecomesTargetAbility` arm ONLY** (not shared with the spell-or-ability / spell / backup arms). Measured evidence:

1. **~109 cards** currently parse to `TriggerMode::BecomesTarget` (`data/card-data.json`, counting any trigger with `mode == "BecomesTarget"`; round-1 review re-measured baseline=109 → post=110, delta +1 = Loki — the original "120" figure was a stale snapshot, corrected here per L3). **Zero** of them contain the substring "becomes/become the target of an ability" — they all use "…of a spell or ability", "…of a spell", "…of an aura spell", "…of an instant or sorcery spell", or "…of a backup ability". Those phrases are matched by the **first**-block arms (or the Backup second-block arms, which precede the new arm). Therefore **no existing BecomesTarget card ever reaches the new arm**, and the new-arm guard cannot regress any of them. (Interception surface of the new tag = the 3 cards above, full stop.)
2. **A SHARED guard WOULD regress real cards** — this is why the guard is new-arm-only. The existing spell arms call `parse_target_source_controller(remaining)` and deliberately ignore trailing text. Measured tails after "becomes the target of a spell or ability" on currently-`BecomesTarget` cards include: `" you control for the first time each turn"` (Heartfire Hero / A-Heartfire Hero), `" for the first time each turn"` (Angelic Cub). The controller clause consumes only `"you control"` (or nothing), leaving `"for the first time each turn"` as a non-empty tail. A shared remaining-empty guard would flip these to Unknown — a measured regression. Hence the guard must NOT be hoisted onto the shared arms.

**Net:** new-arm-only guard ⇒ Loki parses, Skophos + Agrus Kos stay Unknown, all ~109 existing BecomesTarget cards unchanged. Zero new Unknowns. (Parse-level only — the matcher over-fire BLOCKER is a RUNTIME regression not visible at this parse-level check; see correction below.)

**Why not reuse `becomes_target_source_filter`** (`oracle_trigger.rs:168`): that builder ORs `StackSpell` with `StackAbility` — Loki explicitly excludes spells. We want the `StackAbility`-only branch. Building a bare `StackAbility { .. }` is the correct, minimal expression and matches the "an ability" wording precisely.

### 3c. Battlefield gate on the permanent leaf (CR 110.1)

Add a small helper applied to the subject inside the new arm (so it scopes ONLY to becomes-target-ability, never to dies/ETB triggers):

```rust
/// CR 110.1: A permanent exists only on the battlefield. A targeted card in a
/// graveyard/exile is also a TargetRef::Object, so a "permanent" subject for a
/// becomes-target trigger must be battlefield-scoped to exclude non-permanents.
fn battlefield_scope_permanent(subject: &TargetFilter) -> TargetFilter {
    fn gate(f: &TargetFilter) -> TargetFilter {
        match f {
            TargetFilter::Typed(t)
                if t.type_filters.iter().any(|tf| matches!(tf, TypeFilter::Permanent)) =>
            {
                let mut props = t.properties.clone();
                if !props.iter().any(|p| matches!(p, FilterProp::InZone { .. })) {
                    props.push(FilterProp::InZone { zone: Zone::Battlefield });
                }
                TargetFilter::Typed(TypedFilter { properties: props, ..t.clone() })
            }
            TargetFilter::Or { filters } => TargetFilter::Or {
                filters: filters.iter().map(gate).collect(),
            },
            other => other.clone(),
        }
    }
    gate(subject)
}
```

- Evaluation verified: `FilterProp::InZone { zone: Battlefield }` → `filter.rs:3398` `obj.zone == Battlefield`. A targeted graveyard creature card (`obj.zone == Graveyard`) → `Typed(Permanent) ∧ InZone(Battlefield)` is `false` → Loki does not fire. **This is the negative discriminator (§8c).**
- The gate is applied to `subject` *before* `set_trigger_subject` partitions it, so the `InZone` prop rides into `valid_card` on the permanent leaf. The `Player` leaf is untouched.
- `TypedFilter` builders exist (`types/ability.rs:3120 controller`, `:3125 subtype`, `:3129 properties`) — the struct-update form above is idiomatic and matches existing call sites. Confirm `TypedFilter` derives/fields permit `..t.clone()` (it does — `type_filters`/`controller`/`properties` are the three fields, per the `filter_inner_for_object` destructure at `filter.rs:1476-1480`).

---

## 4. "Triggers only once each turn" — reuse, zero new code

- **Constraint variant:** `TriggerConstraint::OncePerTurn` (`types/ability.rs:15350-15352`, doc literally "This ability triggers only once each turn.").
- **Parse:** `parse_trigger_constraint` (`oracle_trigger.rs:1496-1505`) matches `scan_contains(lower, "this ability triggers only once each turn")` (CR 603.2h family) → `Some(OncePerTurn)`. Loki's sentence matches verbatim-class (scan, not equality).
- **F2 annotation migration (required, code edit):** the pre-existing annotation at `oracle_trigger.rs:1501` reads `// CR 603.12: "Do this only once each turn" is functionally equivalent.` — this CR number is **wrong**. CR 603.12 (docs:2656) is *reflexive triggered abilities*; the rule for "Do this only once each turn" is **CR 603.2h** (docs:2580). Per CLAUDE.md's annotation-migration rule (we are editing `oracle_trigger.rs`), change line 1501 to `// CR 603.2h: "Do this only once each turn" is functionally equivalent.`. **Do NOT touch** the other CR 603.12 references in this file (`:17332`, `:17351`, `:17454`, `:17467`, `:17618`, `:32813`) — those are legitimate reflexive / "When you do" usages and are correct.
- **Auto-wiring (verified):** `parse_trigger_constraint` is called at `oracle_trigger.rs:1041` during IR production and stored on `TriggerModifiers.constraint` (`:1053`); lowering applies it at `oracle_trigger.rs:1258`: `def.constraint = modifiers.constraint.clone().or(def.constraint.take())`. **The becomes-target dispatch arm needs to do nothing** — the constraint is attached by the shared lowering path.
- **Enforcement (fire-time):** `check_trigger_constraint` (`triggers.rs:5154`), arm `OncePerTurn => !state.triggers_fired_this_turn.contains(&key)` (`triggers.rs:5172`), `key = (obj_id, trig_idx)` (`:5169`).
- **Recording:** `record_trigger_fired` arm at `triggers.rs:6335-6336`: `state.triggers_fired_this_turn.insert(key)`.
- **Proven non-vacuous by an existing card:** Esper-Sentinel-style "Whenever an artifact you control enters, draw a card. This ability triggers only once each turn." (referenced `swallow_check.rs:5003` per Understand-phase map) exercises the identical `OncePerTurn` path.

No edit required for §4 beyond confirming the limiter fires (test §8b).

---

## 5. Effect body — draw a card

`Effect::Draw { count, .. }` (`types/ability.rs:7846`). "draw a card" parses to `Draw { count: 1 }` via the standard effect-body parser and is stored as `TriggerDefinition.execute: Option<Box<AbilityDefinition>>` (`types/ability.rs:15492`). No targeting on the effect ⇒ **Phase 4 of /add-trigger (target extraction via `extract_target_filter_from_effect`) is N/A** — confirmed: `Draw` has no `target` slot to surface. No edit.

---

## 6. CR grounding (every number grep-verified against `docs/MagicCompRules.txt`)

| CR | Verified line | Text (abbrev.) | Use in plan |
|---|---|---|---|
| **115.1** | 838 | "Some spells and abilities require their controller to choose one or more targets… These targets are object(s) and/or player(s)…" | Authority that targets span objects **and** players — justifies the two-axis (`valid_card` + `valid_target`) model. |
| **115.1a** | 840 | "…If an activated or triggered **ability**… uses the word target, that **ability is targeted**, but the spell is not." | Justifies "target of an **ability**" as distinct from spell — the ability-only `StackAbility` source (§3b). |
| **601.2c** | 2461 | "The chosen objects and/or players each **become a target**… (Any abilities that trigger when those objects and/or players **become the target**… trigger at this point…)" | The exact moment Loki observes; matches the `emit_targeting_events` seam (§2). |
| **602.2b** | 2531 | "The remainder of the process for activating an ability is identical to… casting a spell… 601.2b–i…" | Activated abilities reuse 601.2c targeting ⇒ the seam covers abilities, not just spells. |
| **603.1** | 2555 | "Triggered abilities have a trigger condition and an effect… '[When/Whenever/At] [event], [effect].'" | The "Whenever … draw a card" form. |
| **603.2** | 2561 | "Whenever a game event or game state matches a triggered ability's trigger event, that ability automatically triggers." | Base triggering rule. |
| **603.2c** | 2567 | "An ability triggers only once each time its trigger event occurs. However, it can trigger repeatedly if one event contains multiple occurrences." | Distinguishes `BecomesTargetOnce` (batch) from Loki's per-turn limit; also governs multi-target edge (§9). |
| **603.2h** | 2580 | "A triggered ability may have an instruction followed by 'Do this only once each turn.' This ability triggers only if its source's controller has not yet taken the indicated action that turn." | The "triggers only once each turn" limiter (§4) ⇒ `TriggerConstraint::OncePerTurn`. **Corrects the prior plan's CR 603.12 mis-citation** and the pre-existing `oracle_trigger.rs:1501` annotation (migrate to CR 603.2h). |
| **603.2e** | 2572 | "Some trigger events use the word 'becomes'… These trigger only at the time the named event happens…" | The "becomes the target" event class. |
| **603.3** | 2582 | "Once an ability has triggered, its controller puts it on the stack… the next time a player would receive priority." | Stack placement — no engine change (existing pipeline). |
| **110.1** | 614 | "A permanent is a card or token **on the battlefield**…" | The battlefield gate on the permanent leaf (§3c) — graveyard-card negative discriminator. |
| **110.4** | 624 | "There are six permanent types: artifact, battle, creature, enchantment, land, and planeswalker." | Confirms `TypeFilter::Permanent`'s six core types (`filter.rs:2155-2162`) is the right permanent test. |
| **608.2** | (matcher comment, `trigger_matchers.rs:2307`) | resolving entry follows resolution after pop | Why the matcher also checks `resolving_stack_entry` — no change. |

> Verification commands run: `grep -n "^115.1" / "^601.2c" / "^602.2b" / "^603.2e" / "^603.2c" / "^603.2h" / "^603.1" / "^603.2 " / "^603.3" / "^110.1" / "^110.4" / "^603.12" docs/MagicCompRules.txt`. All returned the lines above. **CR 603.2h** (docs:2580) is the rule for "Do this only once each turn" (the parser's third synonym arm, `oracle_trigger.rs:1501`); the pre-existing annotation there mis-cites CR 603.12 (docs:2656 = *reflexive* triggered abilities) and is migrated to CR 603.2h as part of this change (§4).

**Annotations to add in code:** the matcher fix gets `// CR 115.1 + CR 603.2e:` (independent axes); the parser arm gets `// CR 115.1 + CR 602.2b:` (ability-targeted source); the battlefield helper gets `// CR 110.1:`; the once-per-turn limiter / migrated line 1501 annotation gets `// CR 603.2h:`. (Loki's own new annotations — CR 115.1+603.2e, 115.1+602.2b, 110.1 — are already correct.)

---

## 7. /add-trigger registration checklist (file:line)

| Phase | Registration point | File:line | Action |
|---|---|---|---|
| 1 Types — `TriggerMode` | `types/triggers.rs:302` | **none** — `BecomesTarget` reused | skip (existing mode fits) |
| 1 Types — `TriggerConstraint` | `types/ability.rs:15350` | **none** — `OncePerTurn` reused | skip |
| 1 Types — `TargetFilter`/`FilterProp` | `types/ability.rs:3471` (`StackAbility`), `:2580` (`FilterProp`, has `InZone`) | **none** — reused | skip |
| 2 Event emission — `GameEvent::BecomesTarget` | `casting.rs:252, 265` (emit); `types/events.rs` (variant) | **none** — already emitted for objects+players from the shared ability/spell seam | skip |
| 3 Matcher fn | `trigger_matchers.rs:2343-2347` | **EDIT** — drop `valid_card.is_none()` on Player arm (§2) | required |
| 3 Matcher registry | `trigger_matchers.rs:72, 277-278` | **none** — `BecomesTarget`→`match_becomes_target` already inserted | skip |
| 4 Target extraction | `extract_target_filter_from_effect()` | **none** — effect is `Draw` (untargeted) (§5) | skip |
| 5 Parser — `parse_simple_event` | `oracle_trigger.rs:8109` (enum `:8049`), **third** `alt` block `:8251-8269` (8/21 — room; the second block `:8192-8250` is FULL at 21/21) | **EDIT** — add `BecomesTargetAbility` variant + recognizer arm(s) to the THIRD block (§3b) | required |
| 5 Parser — dispatch | `oracle_trigger.rs:8296+` | **EDIT** — add `BecomesTargetAbility` match arm with F1 remaining-empty guard (§3b) | required |
| 5 Parser — controller tail | `oracle_trigger.rs:150-161` `parse_target_source_controller` | **EDIT/ADD** — add `parse_target_source_controller_tail` (returns `(Option<ControllerRef>, &str)`); old fn delegates to `.0` (§3b-tail) | required |
| 5 Parser — subject routing | `oracle_trigger.rs:7292` `set_trigger_subject` (+ helper `collapse_or`) | **EDIT** — partition `Or` mixed subject (§3a) | required |
| 5 Parser — battlefield gate | new helper `battlefield_scope_permanent` near `set_trigger_subject` | **ADD** — §3c | required |
| 6 Constraint tracking | `oracle_trigger.rs:1041, 1258`; `triggers.rs:5172, 6336` | **none** — `OncePerTurn` auto-wired + enforced (§4) | skip |
| 6 Constraint annotation (F2) | `oracle_trigger.rs:1501` | **EDIT** — migrate pre-existing `// CR 603.12:` → `// CR 603.2h:` ("Do this only once each turn") | required |
| 7 Stack resolution `resolve_top` | `game/triggers.rs` | **none** — generic Draw resolution | skip |
| Frontend display | trigger stack rendering keys on `TriggerMode` / ability text | **none** — no new mode; the ability renders via its `execute` text. No `WaitingFor` (untargeted draw). | skip — confirm no new SimpleEvent surfaces to frontend (SimpleEvent is parser-internal, `oracle_trigger.rs` local enum) |
| AI | `phase-ai` legal actions / policy | **none** — untargeted mandatory draw produces no decision point; no `cargo ai-gate` trigger | skip |
| Forge importer | `database/forge/trigger.rs:136-137` maps `"BecomesTarget"`/`"BecomesTargetOnce"` | **none** — MTGJSON path; Forge already maps the mode | skip |
| 8 Tests | see §8 | **ADD** | required |

**Silent-failure guard (from the skill's common-mistakes table):** the matcher + registry are the spot where "parses but never fires" bugs hide. Here the registry is already correct; the *matcher logic* is the live risk, and §8a/§8c test it directly with `setup_with_ability_on_stack()`.

---

## 8. TESTS — non-vacuous & discriminating

Existing scaffolding to reuse (verified in `trigger_matchers.rs` `#[cfg(test)]`): `make_trigger(TriggerMode::BecomesTarget)` (`:4461`), `setup_with_ability_on_stack()` (`:9608`), `setup_with_spell_on_stack(bool)` (`:9553`), `setup_with_sorcery_on_stack()` (`:9561`). Parser tests follow the `parse_trigger_line` pattern; integration follows `crates/engine/tests/integration/backup_becomes_target_trigger.rs`.

### 8.0 Parser test (assert the AST, not the card)

`assert_eq!` on the lowered `TriggerDefinition` for input `"Whenever a player or permanent becomes the target of an ability you control, draw a card. This ability triggers only once each turn."`:
- `mode == BecomesTarget`
- `valid_target == Some(TargetFilter::Player)`
- `valid_card == Some(Typed(Permanent ∧ InZone(Battlefield)))`
- `valid_source == Some(StackAbility { controller: Some(ControllerRef::You), tag: None, kind: None })`
- `constraint == Some(OncePerTurn)`
- `execute` body is `Draw { count: 1 }`

**Discrimination:** flip each field's expectation and the test fails — e.g. asserting `valid_source` is the spell-or-ability `Or` (the old builder) fails, proving the ability-only source is distinct. Asserting `valid_card` lacks `InZone` fails, proving the zone gate is emitted.

### 8.0-neg PREFIX-COLLISION NEGATIVE parser tests (F1, blocker discriminators)

Two parser-level regression tests asserting that the new prefix arm does **not** intercept source-restricted siblings — the cards measured as the only other members of the new tag's interception surface. Both must parse the "becomes the target of an ability …" trigger line to **Unknown** (the triggered ability fails to parse / produces no `BecomesTarget` `TriggerDefinition`), NOT `BecomesTarget`:

1. **Skophos Maze-Warden** — `"Whenever another creature becomes the target of an ability of a land you control named Labyrinth of Skophos, you may have this creature fight that creature."` Post-"an ability" tail = `" of a land you control named …"` → controller clause does not match at the front (`"of a land…"`, and the embedded `"you control"` is mid-phrase) → non-empty tail → guard returns `None` → Unknown.
2. **Agrus Kos, Eternal Soldier** — `"Whenever Agrus Kos becomes the target of an ability that targets only it, you may pay {1}{R/W}. …"` Post-"an ability" tail = `" that targets only it"` → no controller clause → non-empty tail → guard returns `None` → Unknown.

**Discrimination argument (non-vacuous):** these tests fail the instant the F1 remaining-empty guard is removed — without it, the dispatch arm falls through to `return Some(..)` (the unconditional `return Some((def.mode.clone(), def))` at `oracle_trigger.rs:8502`) and BOTH cards flip from Unknown to a `BecomesTarget { valid_source: StackAbility { controller: None, .. } }` that over-fires on every ability targeting the creature, silently dropping each card's real source restriction. So the pair is a live, discriminating regression fence around the exact F1 bug. (Corpus check: these two + Loki are the entire interception surface — §3b-corpus.)

### 8.0-mixed MIXED-SUBJECT latent-bug fix regression (LOW-2)

⚠️ **OVER-CLAIMED — corrected in round 1 (MEDIUM-1).** The global `set_trigger_subject` `Or`-split + the matcher relaxation change runtime behavior for the mixed player+object subject class. The plan claimed **5** existing cards are fixed — **Leovold, Emissary of Trest; Parnesse, the Subtle Brush; Rayne, Academy Chancellor; Unsettled Mariner; Valkmira, Protector's Shield**. Round-1 review measured the FULL card pipeline (not just `parse_trigger_line`): only **Valkmira** ("you or *another* permanent") actually reaches the `Or`-split and gets its player half fixed. The other **4** ("you or a permanent you control") are split UPSTREAM into a separate `Unknown("Whenever you")` + a permanent-only `BecomesTarget` (`valid_target=null`) before the `Or`-split is reached, so their player halves remain unfired — a **separate, pre-existing parser gap** (the "Whenever you" upstream split), NOT fixed by this PR and NOT in scope. The change is non-regressive for those 4 (they are byte-identical before/after). For all 5 the player half never fired pre-change; Valkmira's is now a CORRECT latent-bug fix, the other 4 stay gated by the upstream gap. **TODO(parser-gap):** make the "Whenever you" upstream split cover "you or a permanent you control" so the remaining 4 reach the `Or`-split.

Regression/fix test (Leovold, "Whenever you or a permanent you control becomes the target of a spell or ability **an opponent controls**, that player draws a card."): build a scenario with Leovold's controller as the trigger owner; have an **opponent-controlled** spell/ability target the Leovold *player* → assert the trigger fires (player half now lives via `valid_target = Controller` → `player_matches_filter`, `trigger_matchers.rs:628`). Then have a **you-controlled** ability target the player → assert it does NOT fire (source-controller axis = "an opponent controls"). **Discrimination:** before the `set_trigger_subject` `Or`-partition + Player-arm relaxation, the player-target case returns `false` (the player leaf is dead weight inside `valid_card`), so this test fails on the unpatched engine — proving the fix is live, not vacuous. Single-subject cards stay bit-identical (`valid_player_matches` returns `true` when `valid_target` is `None`).

### 8.a POSITIVE — ability you control targets a permanent ⇒ draw once (matcher unit)

```
trigger = make_trigger(BecomesTarget)
trigger.valid_target = Some(Player)
trigger.valid_card   = Some(Typed(Permanent ∧ InZone(Battlefield)))
trigger.valid_source = Some(StackAbility { controller: Some(You), .. })
state, ability_id = setup_with_ability_on_stack()   // ability controlled by trigger owner
event = BecomesTarget { target: Object(<a battlefield permanent>), source_id: ability_id }
assert!(match_becomes_target(&event, &trigger, trigger_owner, &state));
```
Plus a sibling with `target: Player(<some player>)` ⇒ also `true` (proves the relaxed Player arm).
**How it fails if matcher is wrong:** with the *unpatched* matcher, the Player-target variant returns `false` (because `valid_card.is_some()`), so the player half of Loki silently never draws — this test catches exactly that regression.

### 8.b ONCE-PER-TURN — two qualifying targetings ⇒ exactly one draw (integration)

Build a `GameScenario` with Loki on the battlefield + a controlled activated/triggered ability that targets, activate it twice in one turn (two `BecomesTarget` emissions). Assert hand size increases by **+1**, not +2. Then advance to the next turn, target again, assert a second draw fires (limiter resets via `triggers_fired_this_turn` cleared at turn boundary).
**How it fails if wrong:** if `constraint` weren't wired (e.g. dispatch arm overwrote it), the second targeting draws again ⇒ +2 ⇒ test fails. Proves §4 auto-wiring is live, not vacuous.

### 8.c NEGATIVE discriminators (each isolates one axis)

1. **Source is a SPELL, not an ability** — reuse `setup_with_spell_on_stack(false)`; `event.source_id = spell_id`. With `valid_source = StackAbility{..}`, `match_becomes_target` must return `false`. *Discrimination:* if the parser had reused `becomes_target_source_filter` (spell-or-ability OR), this returns `true` and the test fails — proving Loki excludes spells. (Mirrors the existing `..._rejects_ability_source` test at `:10151`, inverted.)
2. **Ability you do NOT control** — controller mismatch: ability on stack controlled by an opponent; `valid_source = StackAbility { controller: Some(You) }`. `stack_entry_matches_filter` controller check fails ⇒ `false`. *Discrimination:* drop the controller from the `StackAbility` filter and this would wrongly pass — proves the "you control" axis.
3. **Targeted card is in a GRAVEYARD (not a permanent)** — object target whose `obj.zone == Graveyard` but `CoreType::Creature` present; `valid_card = Typed(Permanent ∧ InZone(Battlefield))`. Must return `false`. *Discrimination:* remove the `InZone(Battlefield)` prop and this passes (CR 110.1 violation) — proves the zone gate (§3c). This is the test that justifies the entire §3c helper.
4. **No legal target / ability targets nothing** — no `BecomesTarget` event emitted ⇒ no draw. (Covered implicitly; assert hand unchanged when an untargeted ability resolves.)

Each negative test toggles exactly one of the three axes (source-kind, controller, target-zone) to red, so a single broken axis surfaces as a single failing test — the discrimination matrix is complete.

---

## 9. Multiplayer / edge correctness

- **Controller is the *ability's* controller, not the permanent's owner** (CR 109.4 / 602.2a). If the Loki controller has gained control of another player's permanent and activates *its* ability, the stack entry's controller is the Loki controller, so `stack_entry_controller_matches` (`targeting.rs:1331`) yields `You` and Loki draws — correct. Conversely, if an opponent activates an ability of a permanent they control (even one Loki's owner owns), controller ≠ You ⇒ no draw. The matcher resolves controller from `state.objects.get(&source_id).controller` (`trigger_matchers.rs:2318-2322`) = the *targeting source's* (ability's) controller, which is the activator. Verified correct for the gained-control case.
- **Self-targeting** — an ability you control that targets Loki itself (a battlefield permanent) ⇒ `Object(loki_id)` matches `Typed(Permanent ∧ InZone(Battlefield))` ⇒ draws (once). No `SelfRef` special-casing needed; the object arm uses `valid_card_matches`, not the `object_id==source_id` self-default (because `valid_card.is_some()`).
- **One ability targeting multiple objects/players** (CR 603.2c) — `emit_targeting_events` pushes one `BecomesTarget` per target, so the trigger *matches* multiple times in the batch, but `TriggerConstraint::OncePerTurn` (`triggers.rs:5172`) caps actual firing to one draw for the turn. Multiple targets in one turn ⇒ still exactly one draw. (Loki is per-turn, not per-batch — `BecomesTargetOnce` would be the per-batch cap, which Loki does NOT use.) Test 8.b's two-activation case covers the multi-event/one-draw behavior.
- **APNAP** — single triggered ability, single controller; no ordering complexity. The generic `process_triggers` path handles stack placement (CR 603.3). No edge code.

---

## RISKS / ASSUMPTIONS FOR THE REVIEWER TO SCRUTINIZE

1. **(Primary) Matcher Player-arm relaxation breadth.** Dropping `valid_card.is_none()` is safe *only because* (a) no existing `BecomesTarget` trigger sets both `valid_card` and `valid_target`, and (b) the retained `valid_target.is_some()` guard preserves object-only behavior. Reviewer should confirm via grep that no other code path constructs a `BecomesTarget` `TriggerDefinition` with both fields set in a way that would now over-fire. (Verified at planning time: only `set_trigger_subject` populates these, and only the new `Or`-split branch sets both.)
2. **`set_trigger_subject` is shared across many trigger modes.** The `Or`-partition branch is gated on `players.is_empty()` so non-player `Or` subjects are untouched — but the reviewer should confirm no non-becomes-target mode relies on a mixed player+object `Or` landing wholesale in `valid_card` today (none found; such a subject is currently mis-handled regardless, so the change is strictly an improvement). **Measured: exactly 5 existing `BecomesTarget` cards carry a mixed player+object subject and are affected — Leovold, Emissary of Trest; Parnesse, the Subtle Brush; Rayne, Academy Chancellor; Unsettled Mariner; Valkmira, Protector's Shield.** For all five the change is a CORRECT latent-bug fix: their player half currently never fires (the matcher's Player arm is blocked by `valid_card.is_some()`), and after the change `valid_target = Controller` fires scoped to "you" via `player_matches_filter` (`trigger_matchers.rs:628`). Zero new Unknowns, no regression. **A regression/fix test is added (§8.0-mixed): assert one of these (Leovold) fires its effect when a `you`-controlled object becomes the target of the relevant ability after the change, and does NOT fire for an opponent-controlled target.**
3. **Battlefield gate placement.** The `InZone(Battlefield)` prop is added *only* in the becomes-target-ability arm via `battlefield_scope_permanent`, NOT globally — because dies/leaves triggers legitimately match graveyard objects. Reviewer should confirm the helper is not hoisted into `parse_single_subject`/`parse_type_phrase` (which would regress those triggers).
4. **nom arity placement (compile-blocking if wrong).** The new recognizer arm MUST go in the **third** `.or(alt((…)))` block (`oracle_trigger.rs:8251-8269`, 8/21 — room for 13), NOT the second. MEASURED: the second block (`:8192-8250`) is at **21/21** = nom 8.0's `alt` tuple ceiling (`alt_trait! A..U`); adding any arm there (or even collapsing both forms into one nested `value(_, alt((becomes, become)))`) overflows to 22 and **fails to compile** (no `Choice` impl for a 22-tuple). The first block holds 20. Reviewer/implementer: confirm the chosen block's `value()` count stays ≤ 21 after the edit, or open a fresh `.or(alt((…)))` block.
5. **`TypedFilter` struct-update assumption.** §3c uses `TypedFilter { properties, ..t.clone() }`; reviewer should confirm `TypedFilter` has exactly the three fields (`type_filters`, `controller`, `properties`) — verified via the `filter_inner_for_object` destructure (`filter.rs:1476-1480`) and builders (`types/ability.rs:3120-3130`).
6. **Verification cadence:** parser-only + matcher edits — run `cargo fmt --all`, then `./scripts/tilt-wait.sh --timeout 240 clippy test-engine card-data` (do NOT run cargo directly; Tilt owns the target lock). Add the new parser test, the two F1 negative parser tests (§8.0-neg), matcher unit tests, and one integration test; confirm `test-engine` green before marking fixed-unreleased.

---

## Revision log (review round 1)

- **F1 (HIGH — prefix-collision regression).** The new recognizer `tag("becomes the target of an ability")` is a **prefix** of source-restricted siblings, and the becomes-target dispatch arms do not check `remaining.is_empty()` (`parse_target_source_controller` discarded its tail and the dispatch returns `Some` unconditionally at `:8502`) — so Skophos Maze-Warden and Agrus Kos would silently flip from inert Unknown to active-but-wrong over-firing.
  - **Fix (lead's decision (a)):** added §3b-tail — refactor `parse_target_source_controller` to expose its remainder via `parse_target_source_controller_tail(rest) -> (Option<ControllerRef>, &str)` (old fn delegates to `.0`, so the single existing spell-arm caller at `:8303` is byte-identical). The new `BecomesTargetAbility` dispatch arm consumes the optional controller clause, then **rejects any non-empty tail → `return None` (Unknown)**, scoped to this arm only. Traced Loki→parses, Skophos→Unknown, Agrus Kos→Unknown.
  - **Guard-regression measurement (decision (b), §3b-corpus):** measured 120 cards parse to `TriggerMode::BecomesTarget`; **zero** contain "becomes/become the target of an ability" (they use spell / spell-or-ability / aura / instant-or-sorcery / backup forms matched by earlier arms), so none reach the new arm. The new tag's full interception surface is **exactly 3 cards** (Loki + the two victims). A **shared** guard would regress real cards — measured tails like `" you control for the first time each turn"` (Heartfire Hero) / `" for the first time each turn"` (Angelic Cub) would flip to Unknown — therefore the guard is **scoped to the new arm only**. Net: zero new Unknowns.
  - **Negative tests (decision (c), §8.0-neg):** added two discriminating parser regression tests asserting Skophos Maze-Warden AND Agrus Kos parse to Unknown (not `BecomesTarget`); both flip to a wrong over-firing `BecomesTarget` the instant the guard is removed (discrimination via the unconditional `Some` at `:8502`). Kept the existing positive Loki AST test (§8.0).
  - Updated §0 item 3, §7 (added controller-tail refactor row), and §3b collision-safety to cover the PREFIX dimension (round 1 only checked the spell arms).
- **F2 (MEDIUM — wrong CR).** "Do this only once each turn" is **CR 603.2h** (docs:2580), not CR 603.12 (docs:2656 = reflexive triggered abilities). Corrected §4 and the §6 verification paragraph, added a CR 603.2h row to the §6 table, and recorded the required migration of the pre-existing `oracle_trigger.rs:1501` annotation (`// CR 603.12:` → `// CR 603.2h:`) with a §7 row. Confirmed the other CR 603.12 references in the file (`:17332/17351/17454/17467/17618/32813`) are legitimate reflexive usages and left untouched. Loki's own new annotations were already correct — left as-is.
- **F3 (LOW — stale test anchors).** Corrected §8 helper anchors: `setup_with_ability_on_stack()` `:10152`→`:9608`, `setup_with_spell_on_stack(bool)` `:10095`→`:9553`, added `setup_with_sorcery_on_stack()` `:9561` and `make_trigger` `:4461` (also fixed the stale `make_trigger` `:9636` reference in §1). Load-bearing engine anchors (matcher 2334-2348, `emit_targeting_events` casting.rs:241-280, `BecomesTarget` triggers.rs:302, `StackAbility` ability.rs:3471, `InZone` filter.rs:3398) re-verified accurate — left unchanged.

## Revision log (review round 2)

- **NEW-H1 (HIGH — compile-blocking nom-arity misdirection).** Round 1 directed the new recognizer arm into the **second** `.or(alt((…)))` block (`oracle_trigger.rs:8192-8250`), which is MEASURED at **21/21** = nom 8.0's `alt` tuple ceiling (`alt_trait! A..U`) — adding any arm there overflows to 22 and will not compile. Corrected §3b, the §7 table row, risk #4, and the §3b dispatch note to target the **third** `.or(alt((…)))` block (`oracle_trigger.rs:8251-8269`, 8 elements = 7 `value()` + bare `parse_becomes_unattached`, room for 13) — or a fresh `.or(alt((…)))` block. Independently re-counted both blocks at base `c1b61ded5` (first=20, second=21, third=8). Risk #4's prior (wrong) reassurance ("first block is at the ceiling") replaced with the measured fact.
- **LOW-1 (caller count overstated).** `parse_target_source_controller` has exactly **one** caller (`:8303`, the spell-or-ability arm), not three — corrected §3b-tail and the round-1 log entry. The delegating refactor is therefore even safer than stated (cosmetic only).
- **LOW-2 (mixed-subject cards not enumerated).** Named the 5 affected existing `BecomesTarget` cards (Leovold, Emissary of Trest; Parnesse, the Subtle Brush; Rayne, Academy Chancellor; Unsettled Mariner; Valkmira, Protector's Shield) in risk #2 and added the §8.0-mixed regression/fix test (Leovold player-half fires after the change for an opponent-controlled targeting; does not fire for a you-controlled source), with its discrimination argument. The change is a correct latent-bug fix (player half currently never fires) — zero new Unknowns, no regression.

**Round-2 verdict context:** the round-2 re-review confirmed F1/F2/F3 genuinely resolved and the prefix-collision fix PROVEN (Skophos + Agrus Kos → Unknown, Loki correct, guard correctly scoped to the new arm only — a shared guard would have regressed Heartfire Hero/Angelic Cub). NEW-H1/LOW-1/LOW-2 above are the only remaining items and are now applied; the plan is implementation-ready. Final adversarial gate is the implementation review (engine-implementer pipeline), where the executor's build immediately validates the third-block placement compiles.

---

## Implementation review round 1 correction (BLOCKER + MEDIUM-1)

The implementation built green (`cargo test -p engine` = 15827/0, clippy/fmt clean) and the recognizer/CR/nom/zone-gate/Valkmira work was confirmed correct, BUT the independent impl-review caught one reproduced BLOCKER and one over-claimed narrative. The plan's pre-implementation analysis (§2, RISK#1) had a **factually false safety premise**; corrected here.

- **BLOCKER — matcher Player-arm relaxation over-fires Venerated Rotpriest (RUNTIME, not parse-level).** The §2 fix "drop `valid_card.is_none()` and read `valid_target` as the subject-player filter" is **unsafe by construction** because `valid_target` is OVERLOADED: it is set both by the subject `Or`-split (subject-player filter) AND by the effect parser (`oracle_trigger.rs:1255-1268`) for any effect containing "target opponent"/"target player" (the effect-target slot). Venerated Rotpriest ("Whenever **a creature you control** becomes the target of a spell, **target opponent** gets a poison counter") sets BOTH `valid_card=Typed(Creature,You)` (object subject) AND `valid_target=Some(Player)` (its effect target) in baseline — refuting RISK#1's "no existing BecomesTarget trigger sets both" premise. Dropping the guard makes Rotpriest fire on ANY player targeted by ANY spell (reproduced empirically: patched matcher=true, old guard=false). Class risk: every future "Whenever <object-subject> becomes the target …, target opponent/player …" card. **CORRECTED FIX (per CLAUDE.md "separate abstraction layers"):** a DISTINCT `valid_subject_player` field on `TriggerDefinition`, set ONLY by `set_trigger_subject`/the `Or`-split, read by `match_becomes_target`'s Player arm; `valid_target` reverts to effect-target-slot-only semantics. Permanent regression fence added (object-subject + player-effect-target trigger must NOT fire on a player target; discriminates against the overloaded read). This separation also makes the Loki two-axis model genuinely independent (subject-player vs object vs effect-target are now three distinct fields).
- **MEDIUM-1 — §8.0-mixed "5-card latent-bug fix" was true for only 1/5.** Corrected in the §8.0-mixed section above and the §2/RISK#2 entries: only **Valkmira** ("you or *another* permanent") reaches the `Or`-split in the FULL card pipeline; the other 4 ("you or a permanent you control") are split upstream into `Unknown("Whenever you")` + permanent-only `BecomesTarget` (`valid_target=null`) before the split — a separate, pre-existing parser gap, left as `TODO(parser-gap)`, not fixed here (non-regressive). The §8.0-mixed test was corrected to reflect the real pipeline (or its docstring made honest).
- **L1** — `collapse_or` vs `merge_or_filters` reuse evaluated. **L2** — the plural `tag("become the target of an ability")` matched 0 corpus cards with no test (speculative dead arm); resolved by adding a test or removing it. **L3** — the "120 BecomesTarget cards" figure was a stale snapshot; corrected to ~109 baseline / 110 post (delta +1 = Loki) in §3b-corpus.

**Net:** RISK#1's premise was the single load-bearing error; the `valid_subject_player` separation is the correct, building-block fix (covers the whole "object-subject trigger with a player-targeting effect" class, not just Rotpriest). Round-2 impl-review re-verifies the BLOCKER fix + regression fence + no new regressions before ship.
