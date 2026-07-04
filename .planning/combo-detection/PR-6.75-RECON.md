# PR-6.75 recon — measured inputs for the C0-full + C1 plan

Two read-only recon reports against `pr65-wt` @ `47adf7fc1` (branch feat/combo-pr6.5-growing-cascade),
gathered 2026-07-02 by the c0c1-plan driver. All line anchors verified live at that commit.
pr65-wt is STRICTLY READ-ONLY (another agent implements inc2b there); anchors WILL move — treat
them as "@47adf7fc1" references.

---

# REPORT 1: walker-recon — C0 ability-scan walker

**File:** `/home/lgray/vibe-coding/pr65-wt/crates/engine/src/game/ability_scan.rs` (3102 lines)

## A. The `Axes` type

Defined at **lines 68–104**. 3-bool struct, one bool per axis, OR-accumulated over a single walk:

```rust
// line 68
#[derive(Clone, Copy)]
struct Axes {
    event: bool,      // axis 1 — reads concrete triggering-event / cost-paid-object char (CR 603.4/608.2k)
    sibling: bool,    // axis 2 — reads source/recipient or board-scoped mutable aggregate a sibling copy could mutate (CR 603.3b)
    projected: bool,  // axis 3 — reads player-level monotone resource / per-turn journal project_out_resources neutralizes (CR 106.1/119/122.1)
}
```

Constants + combinator (**lines 82–104**):

```rust
const NONE: Axes = Axes { event: false, sibling: false, projected: false };   // line 84
const CONSERVATIVE: Axes = Axes { event: true,  sibling: true,  projected: true };  // line 91
fn or(self, other: Axes) -> Axes { /* field-wise || */ }   // line 97
```

`CONSERVATIVE` is the wholesale "reads everything / assume worst" constant: **all three fields `true`**.

**Three axis-predicate entry points** (public API, lines 2795–2855). Each is a thin projection of one
field off `resolved_ability_axes(ability)`:

| Axis | Fn | Line | Reads field | Status |
|---|---|---|---|---|
| 1 event | `ability_uses_event_context(&ResolvedAbility) -> bool` | 2809 | `.event` | **LIVE** (no `dead_code`) |
| 2 sibling | `ability_reads_sibling_mutable(&ResolvedAbility) -> bool` | 2817 | `.sibling` | **LIVE** (no `dead_code`) |
| 3 projected | `ability_reads_projected_resource(&ResolvedAbility) -> bool` | 2803 | `.projected` | `#[allow(dead_code)]` (dormant, inc2b) |

Axis-3 additionally exposes five off-stack scan-surface readouts, all `.projected`, all
`#[allow(dead_code)]`: `trigger_condition_reads_projected_resource` (2824),
`static_condition_reads_projected_resource` (2831), `replacement_condition_reads_projected_resource`
(2838), `ability_condition_reads_projected_resource` (2846), `duration_reads_projected_resource` (2853).

All entry points route through `resolved_ability_axes` (line 115), which destructures
`ResolvedAbility` **with no `..`** (lines 116–162) — a future field fails to compile until classified.

## B. Wholesale-CONSERVATIVE arms (the precision-pass input set)

**Every `Axes::CONSERVATIVE` arm sets ALL THREE axes conservative** (event=true, sibling=true,
projected=true). There is **no arm that is partially conservative**.

Exact counts (grep-verified): **35 `Effect::` arms + 7 non-`Effect` arms = 42 wholesale-CONSERVATIVE
match arms.** (The mission's "~46" was an over-estimate; the true Effect count is 35.) Plus two
non-arm conservative mechanisms inside `resolved_ability_axes` (see end of section).

### 35 `Effect::` CONSERVATIVE arms

| # | Arm | Line |
|---|---|---|
| 1 | `Effect::Pump { .. }` | 324 |
| 2 | `Effect::Counter { .. }` | 345 |
| 3 | `Effect::Token { .. }` | 351 |
| 4 | `Effect::PumpAll { .. }` | 400 |
| 5 | `Effect::ChangeZone { .. }` | 430 |
| 6 | `Effect::ChangeZoneAll { .. }` | 431 |
| 7 | `Effect::Vote { .. }` | 523 |
| 8 | `Effect::SeparateIntoPiles { .. }` | 524 |
| 9 | `Effect::CopySpell { .. }` | 530 |
| 10 | `Effect::EpicCopy { .. }` | 531 |
| 11 | `Effect::CopyTokenOf { .. }` | 540 |
| 12 | `Effect::BecomeCopy { .. }` | 589 |
| 13 | `Effect::Animate { .. }` | 650 |
| 14 | `Effect::ReturnAsAura { .. }` | 651 |
| 15 | `Effect::GenericEffect { .. }` | 653 |
| 16 | `Effect::Mana { .. }` | 655 |
| 17 | `Effect::SearchLibrary { .. }` | 684 |
| 18 | `Effect::RevealFromHand { .. }` | 705 |
| 19 | `Effect::Choose { .. }` | 727 |
| 20 | `Effect::CreateDelayedTrigger { .. }` | 793 |
| 21 | `Effect::AddTargetReplacement { .. }` | 794 |
| 22 | `Effect::AddRestriction { .. }` | 795 |
| 23 | `Effect::CreateEmblem { .. }` | 820 |
| 24 | `Effect::PayCost { .. }` | 821 |
| 25 | `Effect::CastFromZone { .. }` | 822 |
| 26 | `Effect::CreateDamageReplacement { .. }` | 849 |
| 27 | `Effect::RollDie { .. }` | 871 |
| 28 | `Effect::FlipCoin { .. }` | 872 |
| 29 | `Effect::FlipCoins { .. }` | 873 |
| 30 | `Effect::FlipCoinUntilLose { .. }` | 874 |
| 31 | `Effect::GrantCastingPermission { .. }` | 928 |
| 32 | `Effect::ExileFromTopUntil { .. }` | 985 |
| 33 | `Effect::PutAtLibraryPosition { .. }` | 1022 |
| 34 | `Effect::Conjure { .. }` | 1232 |
| 35 | `Effect::ChooseOneOf { .. }` | 1244 |

### 7 non-`Effect` CONSERVATIVE arms

| # | Arm | Line |
|---|---|---|
| 36 | `QuantityRef::DistinctCardTypes { .. }` | 1442 |
| 37 | `QuantityRef::ManaSpentToCast { .. }` | 1683 |
| 38 | `AbilityCondition::RevealedHasCardType { .. }` | 1791 |
| 39 | `TargetFilter::Typed(..)` | 1957 |
| 40 | `TriggerCondition::AttackersDeclaredCount { .. }` | 2336 |
| 41 | `StaticCondition::UnlessPay { .. }` | 2484 |
| 42 | `ReplacementCondition::UnlessControlsOtherLeq { .. }` | 2638 |

### Two non-arm conservative mechanisms in `resolved_ability_axes`

- **`mode_abilities` non-empty → `Axes::CONSERVATIVE`** (line 221–223): reflexive-modal per-mode
  `AbilityDefinition`s are def-level structs the walk does not descend into. Sets all three axes.
- **`unless_pay.is_some()` → `acc.projected = true`** (lines 202–204): sets **only axis-3**
  (projected). Not full-CONSERVATIVE; noted as a hard-coded axis-3 setter.

**Mechanism note:** CONSERVATIVE is applied per-arm as an explicit RHS, not via any
default/fallthrough (there is no fallthrough — see F). A precision pass edits the arm's RHS directly.

## C. Axis-3 isolation (the plan's hard constraint)

**Axis-3 is NOT computed by a separate walk or predicate.** All three axes live in the same `Axes`
struct, accumulated by the **same single traversal**; `projected` rides the same match arms and is
OR-combined by the same `Axes::or` (line 97).

**Consequently, the CONSERVATIVE effect kinds in (B) have `projected == true` too.** Their axis-3
value is *also* conservative, not computed precisely.

**Can axes 1&2 be made precise without touching axis-3? Yes — but ONLY if the arm's replacement
explicitly preserves `projected: true`.** Naively replacing `Axes::CONSERVATIVE` with `Axes::NONE`
(or descending precisely) would flip `projected` from `true`→`false`, changing axis-3.

The precise-arm pattern already used throughout the file is the template — axes set independently in
one struct literal:

```rust
// line 1256 — LifeTotal: projected-only
QuantityRef::LifeTotal { player, .. } => {
    let mut acc = Axes { event: false, sibling: false, projected: true };
    acc = acc.or(scan_player_scope(player)); acc
}
// line 1276 — ObjectCount: sibling-only
QuantityRef::ObjectCount { filter, .. } => {
    let mut acc = Axes { event: false, sibling: true, projected: false };
    acc = acc.or(scan_target_filter(filter)); acc
}
// line 1491 — EventContextAmount: event-only
QuantityRef::EventContextAmount => Axes { event: true, sibling: false, projected: false },
```

**Planner rule:** to make a CONSERVATIVE effect arm precise on axes 1&2 without touching axis-3,
replace `Axes::CONSERVATIVE` with `Axes { event: <precise>, sibling: <precise>, projected: true }`
(or start `acc` from that literal and OR in the descended sub-scans). Keeping `projected: true` pins
axis-3 unchanged. The axis-3 consumers are currently dormant (all six axis-3 readout fns are
`#[allow(dead_code)]`, wired in inc2b) — the constraint is about not silently changing the recorded
classification, not about a live consumer today.

CAVEAT (driver): `projected: true` preserved per-arm is the SUFFICIENT mechanical rule for "axis-3
unchanged". A descent that ORs in sub-scans can only ADD trues, so starting from `projected: true`
is exactly identity-preserving on axis 3.

## D. Axis-1 (event-context) precise arms — the six mission escapees

Four are handled by their **own precise `event:true, sibling:false, projected:false` arm**; two are
handled **conservatively via a CONSERVATIVE carrier** (their scope enum is deliberately never
traversed — see module doc lines 38–41):

| Escapee | Where handled | Line | How |
|---|---|---|---|
| `ObjectScope::EventSource` | `scan_object_scope` | **2091** | precise `event:true` arm |
| `TargetFilter::TriggeringSourceController` | `scan_target_filter` | **2031** | precise `event:true` arm |
| `TargetFilter::ParentTargetSlot { .. }` | `scan_target_filter` | **2041** | precise `event:true` arm |
| `QuantityRef::TimesCostPaidThisResolution` | `scan_quantity_ref` | **1678** | precise `event:true` arm |
| `CastManaObjectScope::TriggeringSpell` | **not traversed** | — | covered via `QuantityRef::ManaSpentToCast → Axes::CONSERVATIVE` (1683); scope enum never imported/scanned |
| `RestrictionPlayerScope::ParentTargetedPlayer` | **not traversed** | — | covered via `Effect::AddRestriction`/`AddTargetReplacement → Axes::CONSERVATIVE` (794–795); scope enum never imported/scanned |

The last two get `event:true` only as a side effect of their carrier's all-true CONSERVATIVE (they
also carry sibling=true, projected=true — NOT precise event-only). Grep confirms
`CastManaObjectScope`/`RestrictionPlayerScope` appear only in the module doc (38) and the test module
(2861, 3030, 3034) — never in the walk body. Test `event_context_axis_discriminates` (2998–3046)
asserts exactly these 5 reachable escapees classify `event == true`.

Other precise `event:true` arms (the walker classifies the whole triggering-event family precisely):
`TargetFilter::{TriggeringSpellController 2006, TriggeringSpellOwner 2011, TriggeringPlayer 2016,
TriggeringSource 2021, EventTarget 2026, ParentTarget 2036, ParentTargetController 2046,
ParentTargetOwner 2051, PostReplacementSourceController 2058, PostReplacementDamageTarget 2063,
PostReplacementDamageTargetOwner 2068, ChosenDamageSource 2075, CostPaidObject 1992},
ObjectScope::{CostPaidObject 2096, EventTarget 2103}, QuantityRef::{EventContextAmount 1491,
EventContextSourceCostX 1503}, ControllerRef::{ParentTargetController 2762, ParentTargetOwner 2767,
TriggeringPlayer 2775}, PlayerScope::ParentObjectTargetController 2747,
PlayerFilter::{TriggeringPlayer 2573, OpponentOtherThanTriggering 2578, OpponentOfTriggeringPlayer
2583, OpponentOfTriggeringPlayerNotAttacked 2588, ParentObjectTargetController 2594,
ParentObjectTargetOwner 2620}`, plus event-reading `TriggerCondition`/`AbilityCondition` arms
(e.g. `TriggeringSpellTargetsFilter`, `ZoneChangeObjectMatchesFilter`, `ZoneChangedThisWay`).

## E. Axis-2 (sibling-mutable) structure & the Rubblebelt/Orcish class

**Axis-2 is a SINGLE boolean flag** (`sibling: bool`), not a split read-set/write-set.
`ability_reads_sibling_mutable` (2817) returns `.sibling`. No read-vs-write distinction and no caller
intersection inside this file — any conflict intersection is the consumer's responsibility.

**Rubblebelt Rioters / Orcish Siegemaster** ("Whenever this creature attacks, it gets +X/+0, X =
greatest power among creatures you control"):

- The top effect is **`Effect::Pump { .. }` → line 324 → `Axes::CONSERVATIVE`**. This
  **short-circuits**: `scan_effect` returns all-true immediately and **never descends into the pump
  amount**. Today this class classifies event=true, sibling=true, projected=true — driven entirely by
  the Pump arm, *not* by the board-aggregate read. **This is the precision opportunity**: making
  `Effect::Pump` descend into `amount`/`target` routes this class through the already-correct arms
  below, classifying it **sibling-only** (event=false).

The arms that *would* fire if Pump descended — each **sibling=true, event=false, projected=false**:

```rust
// line 1418 — "greatest power among creatures you control" (Max aggregate)
QuantityRef::Aggregate { filter, .. } => {
    let mut acc = Axes { event: false, sibling: true, projected: false };
    acc = acc.or(scan_target_filter(filter)); acc
}
// line 1336 — Power of an object scope
QuantityRef::Power { scope, .. } => {
    let mut acc = Axes { event: false, sibling: true, projected: false };
    acc = acc.or(scan_object_scope(scope)); acc
}
// line 1354 — Toughness (same shape); line 1308 — CountersOn { scope } (same shape)
```

The `AggregateFunction` (Max/Min) is dropped in the `{ filter, .. }` rest — Max/Min board aggregates
are all classified uniformly as sibling reads via `QuantityRef::Aggregate`. (`QuantityExpr::Max` at
1764 / `Sum` etc. carry no axis of their own; they OR their children.)

**`ObjectScope::Source` and `Recipient` themselves return `Axes::NONE`** (2088, 2090) — the sibling
flag is carried at the *QuantityRef layer* (the `Power`/`Toughness`/`CountersOn` arms set
`sibling:true` before OR-ing in `scan_object_scope(scope)`). So "Power of Source" ⇒ sibling=true
(from the `Power` arm), event=false. Full `scan_object_scope`:

```rust
// line 2086
ObjectScope::Source    => Axes::NONE,        // 2088
ObjectScope::Target    => Axes::NONE,        // 2089
ObjectScope::Recipient => Axes::NONE,        // 2090
ObjectScope::EventSource => Axes { event: true, .. },   // 2091
ObjectScope::CostPaidObject => Axes { event: true, .. },// 2096
ObjectScope::Anaphoric => Axes::NONE,        // 2101
ObjectScope::Demonstrative => Axes::NONE,    // 2102
ObjectScope::EventTarget => Axes { event: true, .. },   // 2103
```

Other sibling=true carriers (board/object mutable reads): `QuantityRef::{ObjectCount 1276,
ObjectCountDistinct 1285, ObjectCountBySharedQuality 1294, CountersOn 1308, CountersOnObjects 1317,
Intensity 1345, ObjectManaValue 1363, TargetObjectManaValue 1372, ObjectColorCount 1381,
ObjectNameWordCount 1390, ObjectTypelineComponentCount 1399, ManaSymbolsInManaCost 1408, Aggregate
1418, ControlledByEachPlayer 1427, Devotion 1437, DistinctColorsAmongPermanents 1691,
DistinctCounterKindsAmong 1700, EnteredThisTurn 1520 (also projected)};
TriggerCondition::{ControlsType 2124, ControlCount 2204, HadCounters 2290, HasCounters 2297};
StaticCondition::{DevotionGE 2401, HasCounters 2443, RecipientHasCounters 2449};
PlayerFilter::ControlsCount 2599`.

## F. Wildcard audit

- **Zero `_ =>` / `_ if` wildcard arms** in the entire file (grep confirmed). Every enum match is
  exhaustive and explicit; every root struct (`ResolvedAbility` 116, `ModalChoice` 250,
  `MultiTargetSpec` 189, `TargetSelectionConstraint` 275) is destructured **without `..`**. (The 144
  `{ .. }` occurrences are all payload-field elisions inside named variant arms — never a catch-all.)
- **`#[allow(dead_code)]`: exactly 6**, all on axis-3 readout fns (2802, 2823, 2830, 2837, 2845,
  2852), each with `TODO(PR-6.5 inc2b): remove — consumed by analysis::resource …`. The two live
  axes-1/2 fns (2809, 2817) carry no attribute.

## G. Traversal set

18 walk functions. Every root the closure requires is traversed:

| Enum / type | Fn | Line |
|---|---|---|
| `ResolvedAbility` (root, no-`..`) | `resolved_ability_axes` | 115 |
| `Effect` | `scan_effect` | 286 |
| `QuantityRef` | `scan_quantity_ref` | 1249 |
| `QuantityExpr` | `scan_quantity_expr` | 1713 |
| `AbilityCondition` | `scan_ability_condition` | 1774 |
| `TargetFilter` | `scan_target_filter` | 1949 |
| `ObjectScope` | `scan_object_scope` | 2086 |
| `TriggerCondition` | `scan_trigger_condition` | 2111 |
| `Duration` | `scan_duration` | 2370 |
| `StaticCondition` | `scan_static_condition` | 2399 |
| `PlayerFilter` | `scan_player_filter` | 2535 |
| `ReplacementCondition` | `scan_replacement_condition` | 2628 |
| `PlayerScope` | `scan_player_scope` | 2732 |
| `ControllerRef` | `scan_controller_ref` | 2756 |
| `CountScope` | `scan_count_scope` | 2784 |
| `RepeatContinuation` | `scan_repeat_continuation` | 230 |
| `ModalChoice` | `scan_modal_choice` | 249 |
| `TargetSelectionConstraint` | `scan_target_selection_constraint` | 275 |

Plus `MultiTargetSpec` destructured inline (189). Types deliberately **outside** the traversal set
(classified conservatively at their carriers per module doc 33–41): `ContinuousModification`,
`ManaProduction`, `ReplacementDefinition`, nested `ResolvedAbility` inside def-level structs,
`FilterProp`, reflexive-modal `mode_abilities`, `CastManaObjectScope`, `RestrictionPlayerScope`.

## Walker bottom line

- Precision-pass surface = **35 `Effect::` wholesale-CONSERVATIVE arms** (+7 non-Effect, +2 non-arm
  mechanisms), each currently the all-true constant with no descent.
- Hard invariant: **preserve `projected: true`** in every rewritten arm (axis-3 byte-identical).
- Archetypal win: `Effect::Pump` (324) descent → Rubblebelt/Orcish routes through
  `Aggregate`/`Power` sibling-only arms, dropping the false `event=true`.
- Keep: zero wildcards, no-`..` root destructures, `event_context_axis_discriminates` + axis-3 tests.

---

# REPORT 2: wiring-recon — trigger-ordering C0/C1/C2, #4269, C1-broken tests

**Primary file:** `/home/lgray/vibe-coding/pr65-wt/crates/engine/src/game/triggers.rs` @ 47adf7fc1.
All line numbers CURRENT (inc2a shifted them from the PR-6.25-era anchors).

## A. The allowlist (C0's replacement target)

`value_contains_trigger_event_context_ref` — **triggers.rs:3358–3383** (doc header 3350–3357):

```rust
/// Legacy fail-open event-context allowlist. RETAINED for the pre-feature
/// same-event and ZoneChanged same-departure-batch auto-resolve paths, whose
/// shipped behavior depends on this classifier's exact (fail-open) semantics —
/// notably co-departing death triggers that read `EventSource` power (issue
/// #4269) auto-order today because this allowlist does NOT list `EventSource`.
/// The fail-closed `ability_scan` walker is used ONLY for the new gated-C2
/// distinct-event term; replacing this allowlist wholesale on the legacy paths
/// regresses those cards (see inc2a report — C0 full replacement is DEFERRED).
fn value_contains_trigger_event_context_ref(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(tag) => matches!(
            tag.as_str(),
            "TriggeringSpellController"
                | "TriggeringSpellOwner"
                | "TriggeringPlayer"
                | "TriggeringSource"
                | "ParentTarget"
                | "ParentTargetController"
                | "ParentTargetOwner"
                | "StackSpell"
                | "CostPaidObject"
                | "EventContextAmount"
                | "EventContextSourceCostX"
                | "ManaSpentToCast"
        ),
        serde_json::Value::Array(values) => {
            values.iter().any(value_contains_trigger_event_context_ref)
        }
        serde_json::Value::Object(map) => {
            map.values().any(value_contains_trigger_event_context_ref)
        }
        _ => false,
    }
}
```

**Exactly 12 allowlisted strings.**

`ability_uses_trigger_event_context` — **triggers.rs:3385–3389**:

```rust
fn ability_uses_trigger_event_context(ability: &ResolvedAbility) -> bool {
    serde_json::to_value(ability)
        .map(|value| value_contains_trigger_event_context_ref(&value))
        .unwrap_or(true)
}
```

**Two distinct failure modes — do not conflate:**
- **Unlisted string, successful serialize ⇒ fail-OPEN** (the shipped #4269 dependency): unlisted
  event-context refs return `false` ⇒ batch auto-resolve permitted.
- **Serialize error ⇒ fail-CLOSED**: `unwrap_or(true)` ⇒ "uses event context" ⇒ prompt. This is the
  conservative direction — NOT the fail-open knob. Fail-open lives at the `matches!` allowlist.

## B. `trigger_events_match_for_ordering` (the C1 target) — triggers.rs:3416–3458

```rust
fn trigger_events_match_for_ordering(
    first: &PendingTrigger,
    candidate: &PendingTrigger,
    legacy_uses_trigger_event: bool,
    c2_order_independent: bool,
    loop_detection_on: bool,
) -> bool {
    // Same firing event (CR 603.2c): pre-feature auto-order, UNCHANGED. Gating
    // this on soundness is the C1 CR 603.3b fix — DEFERRED (see inc2a report):
    // the committed `ability_scan` walker classifies ~46 common effect kinds
    // (CopySpell/Token/Pump/Mana/ChangeZone/…) as conservative, so `sibling`
    // reads true for them; gating same-event on that axis would over-prompt
    // printed copy/token keywords (Demonstrate, Replicate) in all modes,
    // contradicting the "affects no printed card" guarantee. Needs a precise
    // read/write predicate first.
    if first.trigger_event == candidate.trigger_event {
        return true;
    }

    // Distinct firing events. Pre-feature (and OFF): only an explicitly
    // simultaneous ZoneChanged same-departure batch (CR 603.2c) with no
    // event-context read auto-resolves. Gated by the LEGACY allowlist (not the
    // walker) so shipped co-departing death triggers reading `EventSource` power
    // (issue #4269) keep auto-ordering — the walker correctly flags EventSource
    // event-context, which would defeat this batch path.
    if !legacy_uses_trigger_event {
        if let (Some(first_event), Some(candidate_event)) =
            (&first.trigger_event, &candidate.trigger_event)
        {
            if zone_changes_are_same_departure_batch(first_event, candidate_event) {
                return true;
            }
        }
    }

    // C2 (GATED on `loop_detection.is_on()`): when the growing-cascade detector
    // is ON, a distinct-event group the fail-closed C0 walker deems order
    // independent (reads neither event context nor sibling-mutable state) also
    // auto-resolves so the loop-detect ring can accumulate the super-critical
    // fan-out. When OFF this term is false, so distinct-event non-ZoneChanged
    // groups PROMPT exactly as pre-feature (default gameplay byte-preserved).
    loop_detection_on && c2_order_independent
}
```

**Branch order:** (1) same-event short-circuit :3431–3433 — returns `true` BEFORE any gate (the C1
target); (2) distinct-event ZoneChanged batch :3441–3449 — `!legacy_uses_trigger_event` (allowlist,
NOT walker), mode-independent; (3) C2 tail :3457 — `loop_detection_on && c2_order_independent`.

## C. `zone_changes_are_same_departure_batch` — triggers.rs:3391–3414

```rust
fn zone_changes_are_same_departure_batch(a: &GameEvent, b: &GameEvent) -> bool {
    let (
        GameEvent::ZoneChanged { object_id: a_id, from: a_from, to: a_to, record: a_record },
        GameEvent::ZoneChanged { object_id: b_id, from: b_from, to: b_to, record: b_record },
    ) = (a, b)
    else { return false; };

    a_from == b_from
        && a_to == b_to
        && a_record.co_departed.contains(b_id)
        && b_record.co_departed.contains(a_id)
}
```

- **No `///` doc on the fn** (rationale lives in the caller block comments).
- Computes: both `ZoneChanged`, identical `from`/`to`, mutual `co_departed` (CR 603.10a look-back;
  producers `zones.rs:779–815`, `sba.rs:198`).
- **Exactly one call site** — :3445, inside the DISTINCT-event path (NOT the same-event
  short-circuit).
- **Reachable in ALL modes** (the batch block is not guarded by `loop_detection_on`) ⇒ the helper is
  live on the OFF path ⇒ **NOT deletable; it stays**.

## D. `group_is_order_independent` + inc2a C2 gating — triggers.rs:3510–3547 (block-doc 3477–3509)

```rust
fn group_is_order_independent(group: &[PendingTriggerContext], is_on: bool) -> bool {
    let Some((first, rest)) = group.split_first() else { return false; };
    if rest.is_empty() { return false; }
    if !trigger_has_no_ordering_input(&first.pending) { return false; }
    let mut reference = first.pending.ability.clone();
    normalize_ability_identity(&mut reference);
    // Legacy allowlist: drives the pre-feature same-event / ZoneChanged paths.
    let legacy_uses_trigger_event = ability_uses_trigger_event_context(&reference);
    // C0 (fail-closed AST walker): two distinct soundness axes (event context,
    // sibling-mutable) — consumed ONLY by the gated-C2 distinct-event term.
    let c2_order_independent = !crate::game::ability_scan::ability_uses_event_context(&reference)
        && !crate::game::ability_scan::ability_reads_sibling_mutable(&reference);
    rest.iter().all(|ctx| {
        let t = &ctx.pending;
        trigger_has_no_ordering_input(t)
            && t.condition == first.pending.condition
            && trigger_events_match_for_ordering(
                &first.pending, t,
                legacy_uses_trigger_event, c2_order_independent, is_on,
            )
            && t.subject_match_count == first.pending.subject_match_count
            && t.may_trigger_origin == first.pending.may_trigger_origin
            && {
                let mut candidate = t.ability.clone();
                normalize_ability_identity(&mut candidate);
                candidate == reference
            }
    })
}
```

- Gate wiring (from `git show 47adf7fc1`): `begin_trigger_ordering` reads
  `state.loop_detection.is_on()` once at **:3590**, passes into
  `group_is_order_independent(&g.triggers, loop_detection_on)` at **:3592**; the predicate threads
  `is_on` to the leaf; the one-line conjunct is the tail `loop_detection_on && c2_order_independent`
  (:3457).
- `trigger_has_no_ordering_input` at :3467–3475 (targets/constraints/distribute/modal/mode_abilities/
  multi_target/distribution all empty — CR 603.3c/603.3d gate).

## E. Nested Shambler / issue #4269

**The card (measured, `data/card-data.json`):** *Nested Shambler* — "When this creature dies, create
X tapped 1/1 green Squirrel creature tokens, where X is this creature's power." Parsed trigger:
`mode: ChangesZone`, effect `Token { …, count: Ref { qty: Power { scope: Source } } }` — i.e.
`QuantityRef::Power { scope: ObjectScope::Source }`.

**Test pinning #4269:** `resolve_source_power_prefers_lki_when_source_left_battlefield` —
**quantity.rs:9715–9767** (doc names "#4269 Nested Shambler", CR 608.2h + CR 603.10a). It is a
QUANTITY-resolution test (buffed LKI power wins after the source hits the graveyard ⇒ token count 3
not 1). **No dedicated trigger-ORDERING test pins #4269**; ordering protection is asserted
generically by the C2 OFF-arm / same-departure-batch tests (triggers.rs tests at
20479/20528/20558/20603).

**Rules-terms dependency:** a board wipe killing N Shamblers fires one dies-trigger per
`ZoneChanged` event with mutual `co_departed` (CR 603.10a). Distinct events ⇒ distinct-event path.
Auto-resolve requires `legacy_uses_trigger_event == false` + the batch predicate. The ability
serializes with tag `"Source"` — NOT among the 12 allowlisted strings ⇒ fail-open ⇒ auto-resolve
today.

**MEASURED DISCREPANCY (crux input for the plan):** the code comments (:3353–3354, :3438–3439) claim
#4269 reads `EventSource`. **The card as parsed reads `ObjectScope::Source`, not `EventSource`.**
- Legacy allowlist: `Source` not listed ⇒ fail-open ⇒ auto-resolve.
- Walker event axis: `scan_object_scope(Source) => Axes::NONE` (2088) — NOT flagged.
- Walker sibling axis: `QuantityRef::Power { scope: Source }` arm (1336) ⇒ **sibling=true** — flagged.

So for Nested Shambler as actually parsed, replacing the allowlist with the walker's EVENT axis on
the batch path would NOT regress it — but applying the walker's TWO-AXIS conjunction
(`!event && !sibling`) there WOULD (via sibling). The "EventSource ⇒ walker defeats the batch path"
narrative applies to a hypothetical co-departing card reading `EventSource`, not to the pinned
Shambler AST. The plan must re-measure the co-departing-death class against the corpus (which refs
actually occur) and correct the comments. Driver note: `.pr65-driver-log.md:174`'s "C0-full flags
EventSource → Nested Shambler prompts" measurement claim should be re-checked against the `Source`
parse — the regression is real either way (sibling axis), but the MECHANISM matters for the fix
design (LKI-frozen Source reads on departed sources are exactly why the batch path is sound; see
PR-6.25 findings §4).

## F. The two C1-broken tests

Both are the **copy-keyword class → `Effect::CopySpell` → `Axes::CONSERVATIVE`** (ability_scan.rs:530).
Break recorded in `.pr65-driver-log.md:171–174`.

### F1. `multiple_dynamic_demonstrate_grants_enqueue_multiple_triggers` — triggers.rs:19036–19072

Two cast-time granted Demonstrate instances (CR 702.144a) ⇒ the granted-Demonstrate seam
(triggers.rs:2608–2643) enqueues two `PendingTrigger`s, each `trigger_event = Some(SpellCast{…})`
(SAME event), description "Demonstrate", ability = `demonstrate_copy_ability_definition()` whose
effect is `Effect::CopySpell`. `count_demonstrate_triggers` (18944–18956) counts entries ON
`state.stack` with `description == Some("Demonstrate")`; asserts == 2.
- Today: same-event short-circuit ⇒ auto-order ⇒ both dispatched to stack ⇒ 2. Passes.
- Under coarse-walker C1: CopySpell ⇒ `c2_order_independent == false` ⇒ gated same-event false ⇒
  `PromptForChoice` (`WaitingFor::OrderTriggers`), triggers in `pending_trigger_order`, NOT on stack
  ⇒ count 0 ⇒ FAILS.

### F2. `granted_replicate_paid_twice_creates_two_copies` — casting_costs.rs:15092–15143

Replicate paid twice ⇒ two `AdditionalCostInstancePayment` records
(`record_additional_cost_instance_payment`, casting_costs.rs:577; ability.rs:15243) ⇒ granted-
replicate seam (triggers.rs:2547–2593) enqueues one copy trigger per record ⇒ two same-event
(`SpellCast`) CopySpell triggers, description "Replicate".
`drain_counting_spell_copies` (casting_costs.rs:15001–15022) pumps `PassPriority` counting
`GameEvent::SpellCopied`, `break`s on any `Err`; asserts copies == 2.
- Today: same-event short-circuit ⇒ auto-order ⇒ 2 copies. Passes.
- Under coarse-walker C1: prompt ⇒ drain stalls ⇒ copies < 2 ⇒ FAILS.

**Key structural note:** `"StackSpell"` IS in the 12-string allowlist (CopySpell's scope serializes
to it), but the same-event short-circuit never consults the allowlist — so Demonstrate/Replicate
auto-order today REGARDLESS of being "event-context" by the allowlist's own standard. A C1 gate of
"reads no event context" would break them even with a PRECISE walker. For same-event groups the
event-context read is IDENTICAL across siblings (same event) — the C1-relevant order-sensitivity
axis is the SIBLING conflict axis (and Case-A shows byte-identical normalized ASTs still differ as
resolution functions via live Source-binding).

## G. CR verification (grep-verified, `docs/MagicCompRules.txt`)

- **CR 603.2c** (line 2567): "An ability triggers only once each time its trigger event occurs.
  However, it can trigger repeatedly if one event contains multiple occurrences."
- **CR 603.3b** (line 2586): "If multiple abilities have triggered since the last time a player
  received priority, the abilities are placed on the stack in a two-part process. First, each player,
  in APNAP order, puts each triggered ability they control with a trigger condition that isn't
  another ability triggering on the stack in any order they choose. …"
- **CR 603.3c** (line 2588): modal triggered ability — mode announced when putting on the stack.
- **CR 603.3d** (line 2590): remainder of process identical to casting (601.2c–d).
- **CR 603.4** (line 2592): intervening-if clause rule.
- **CR 603.10a** (line 2638): look-back-in-time zone-change triggers (leaves-the-battlefield etc.).

## H. Chokepoint

- `group_is_order_independent` is the SOLE production soundness authority: only non-test call site is
  triggers.rs:3592 inside `begin_trigger_ordering` (:3562–3612). (Hits at 20479/20528/20558/20603 are
  `#[cfg(test)]` C2-arm tests.)
- `begin_trigger_ordering` called from exactly two production sites: `process_triggers` :3234 and
  `drain_deferred_trigger_queue_unchecked` :4613.
- `drain_order_triggers_with_identity` (:3653–3669, doc :3646–3652) is now a "Test/legacy helper" —
  invoked only from test bodies (13170, 20161, 22445, 23371, 23430, 23568). NOT a widening route.

## Wiring bottom line

- **C1** = gate the ungated same-event short-circuit (:3431) on soundness; blocked because the coarse
  walker marks CopySpell/Token/Pump CONSERVATIVE, over-prompting printed Demonstrate/Replicate in all
  modes. Needs the precision pass first — and the C1 gate axis must be chosen carefully (sibling
  conflict, not event-context, is the same-event order-sensitivity axis).
- **C0-full** = replace the 12-string fail-open allowlist with the walker on the legacy batch path;
  deferred over the co-departing-death class — with the measured `Source`-vs-`EventSource`
  discrepancy above, which changes the mechanism (sibling axis, LKI-frozen reads) and must be
  re-measured/designed around.
- **C2** (shipped, gated) stays as-is; `zone_changes_are_same_departure_batch` stays (OFF-path live).
