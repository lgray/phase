//! CR 603.3b: PR-6.75 trigger-ordering conflict-gate tests.
//!
//! Two families:
//!   * The §5.2 **allowlist-parity sweep** — the corpus-wide proof that the new
//!     `ability_rw` conflict gate reproduces the DELETED 12-string serde
//!     allowlist's decision on every printed no-ordering-input trigger, modulo
//!     the seven proven-order-dependent category-(1) rows (the CR 603.3b fix).
//!     A FROZEN in-test copy of the deleted walk is the reference oracle.
//!   * The **discriminators** N-A/N-B/N-B2/N-C/N-D/N-F — hand-built groups run
//!     through the production ordering-soundness authority
//!     (`group_is_order_independent`), each paired so exactly one classification
//!     decision is bracketed (revert-fails recorded in the impl report).
//!
//! N-E (the per-arm profile pairings) already lives in `ability_rw`'s unit tests
//! and is NOT duplicated here.

use super::*;
use crate::game::ability_rw::{
    ability_rw_profile, filter_excludes_source, profiles_conflict, source_census_overlaps_filter,
    trigger_condition_rw_profile, GroupStructure, SourceCensus,
};
use crate::game::ability_utils::{build_resolved_from_def, build_target_slots};
use crate::game::game_object::GameObject;
use crate::test_support::shared_card_db;
use crate::types::ability::{
    AbilityCondition, AbilityDefinition, AggregateFunction, Comparator, ControllerRef, Effect,
    ObjectScope, PlayerFilter, PlayerScope, QuantityExpr, QuantityRef, ResolvedAbility,
    SearchSelectionConstraint, TargetFilter, TriggerCondition, TriggerConstraint,
    TriggerDefinition, TypedFilter,
};
use crate::types::card::CardFace;
use crate::types::card_type::Supertype;
use crate::types::counter::CounterType;
use crate::types::events::GameEvent;
use crate::types::game_state::{GameState, ZoneChangeRecord};
use crate::types::identifiers::{CardId, ObjectId};
use crate::types::player::PlayerId;
use crate::types::triggers::TriggerMode;
use crate::types::zones::Zone;
use std::collections::BTreeSet;

// ===================================================================
// Frozen reference oracle: verbatim copy of the DELETED serde walk
// (was value_contains_trigger_event_context_ref / :3395-3420). Kept here so the
// parity sweep remains provable after the production fn is gone.
// ===================================================================

fn legacy_value_contains_event_context_ref(value: &serde_json::Value) -> bool {
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
            values.iter().any(legacy_value_contains_event_context_ref)
        }
        serde_json::Value::Object(map) => map.values().any(legacy_value_contains_event_context_ref),
        _ => false,
    }
}

/// The old `ability_uses_trigger_event_context`: serialize the ability, walk for
/// one of the 12 tags.
fn legacy_allowlist(ability: &ResolvedAbility) -> bool {
    serde_json::to_value(ability)
        .map(|v| legacy_value_contains_event_context_ref(&v))
        .unwrap_or(true)
}

/// The AST bears an `Effect::Unimplemented` (serde tag `Unimplemented`).
fn ast_bears_unimplemented(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(s) => s == "Unimplemented",
        serde_json::Value::Array(vs) => vs.iter().any(ast_bears_unimplemented),
        serde_json::Value::Object(map) => {
            // `Unimplemented` is a struct-variant key OR a `"type":"Unimplemented"` tag.
            map.contains_key("Unimplemented") || map.values().any(ast_bears_unimplemented)
        }
        _ => false,
    }
}

// ===================================================================
// §5.2 allowlist-parity sweep
// ===================================================================

/// The canonical category-(1) rows (§5.2): provably order-dependent printed
/// groups whose same-event auto→prompt flip IS the CR 603.3b fix. Each carries
/// its in-plan order-dependence proof (see PR-6.75-C0FULL-C1-PLAN.md §5.2). A
/// same-event diff on any OTHER card is an implementation finding, never a new
/// row (STRICT PROOF-GATE).
const CATEGORY_1_ROWS: &[&str] = &[
    // Ouroboroid: each copy's Power{Source} read is fed by the other's
    // PutCounterAll — token/counter totals differ by order.
    "ouroboroid",
    // Docent of Perfection: each copy's Wizard-token write feeds the shared
    // census threshold — WHICH copy transforms depends on order.
    "docent of perfection",
    // Sidequest: Hunt the Mark: token write feeds the shared census threshold.
    "sidequest: hunt the mark",
    // Promise of Tomorrow: first copy's mass return flips the second's CR 603.4
    // re-check — whose stash returns depends on order.
    "promise of tomorrow",
    // Spawn of Mayhem: damage order across the 10-life threshold decides which
    // copy gets the counter.
    "spawn of mayhem",
    // Your Inescapable Doom: PutCounter{Any} write × sibling CountersOn{Source}
    // read — same-kind counter feed (CR 904.3 two-copy scheme deck).
    "your inescapable doom",
    // Complex Automaton: ObjectCount{Permanent} intervening-if read × self-Bounce
    // whose moved census (permanent) overlaps the read filter.
    "complex automaton",
];

/// The census of a printed card's source (core types + subtypes + non-token).
fn face_census(face: &CardFace) -> SourceCensus {
    let mut tags: Vec<String> = Vec::new();
    for ct in &face.card_type.core_types {
        tags.push(ct.to_string());
    }
    for st in &face.card_type.subtypes {
        tags.push(st.clone());
    }
    tags.push("nontoken".to_string());
    SourceCensus::from_tags(tags)
}

/// Whether the firing event carries an object (production uses
/// `extract_source_from_event`; phase/turn/global modes carry none).
fn mode_carries_event_object(mode: &TriggerMode) -> bool {
    !matches!(
        mode,
        TriggerMode::Phase
            | TriggerMode::TurnBegin
            | TriggerMode::NewGame
            | TriggerMode::BecomeMonarch
            | TriggerMode::TakesInitiative
            | TriggerMode::LosesGame
    )
}

/// Whether the mode is a battlefield-departure (dies / LTB / sacrifice) — the
/// departure-batch structures are reachable only for these.
fn mode_is_battlefield_departure(
    mode: &TriggerMode,
    def: &crate::types::ability::TriggerDefinition,
) -> bool {
    match mode {
        TriggerMode::LeavesBattlefield | TriggerMode::Destroyed | TriggerMode::Sacrificed => true,
        // Zone-change modes are departures only when origin is the battlefield.
        TriggerMode::ChangesZone | TriggerMode::ChangesZoneAll => {
            def.origin == Some(Zone::Battlefield) || def.origin_zones.contains(&Zone::Battlefield)
        }
        _ => false,
    }
}

/// §5.2 per-trigger structure-reachability (CLASS-A guard): whether a same-event
/// 2-copy group is corpus-REACHABLE for this trigger. Applying a same-event
/// structure where two distinct non-legendary sources can NEVER share the firing
/// event, or where a 2-copy group can never form, is meaningless — a decision
/// diff there must NOT be counted (§5.2). Returns false (exempt from the
/// same-event structures) when:
///   * LEGENDARY — two same-name legendaries can't coexist under one controller
///     (CR 704.5j legend rule), so a 2-copy same-event group never persists; the
///     plan's `same_event` class is measured over NON-legendary cards (§1.3).
///     (gisela, the broken blade; doors of durin.)
///   * PER-SOURCE event mode — the event IS the source's own action, so two
///     distinct sources fire on DISTINCT events: self-scoped `DamageDone`/
///     `DamageDoneOnce` (combat damage is a per-source `DamageDealt` event keyed
///     by `source_id`, events.rs:408 — valeron wardens, acolyte of the inferno),
///     or a Saga-chapter `CounterAdded` on a SelfRef `valid_card` (per-source lore
///     counter). An OBSERVER DamageDone ("whenever a creature deals damage") is
///     NOT exempt (it CAN be shared).
///   * CONDITION self-exclusion (§1.3.1-F, CR 603.4) — the shared intervening-if
///     counts `Another`-self-exclusion objects the SOURCE itself matches and
///     requires that count to be 0; with two copies each sees the other (≥ 1) ⇒
///     each re-check is FALSE at trigger time ⇒ neither fires ⇒ no group forms
///     (thopter assembly; dust stalker).
fn same_event_group_reachable(
    face: &CardFace,
    trig: &TriggerDefinition,
    census: &SourceCensus,
) -> bool {
    if face.card_type.supertypes.contains(&Supertype::Legendary) {
        return false;
    }
    match trig.mode {
        // Damage triggers carry their SUBJECT in `valid_source` (`valid_card` is
        // ALWAYS None for them — make_base leaves it, oracle_trigger sets
        // valid_source), so a SELF-damage trigger is `valid_source == SelfRef`.
        // Combat damage is a per-source `GameEvent::DamageDealt{source_id}`
        // (events.rs:408), so two distinct self-sources never share it ⇒ exempt.
        // An OBSERVER damage trigger (`valid_source` non-SelfRef, e.g. "whenever a
        // creature you control deals damage") CAN fire two copies off ONE source's
        // damage event ⇒ NOT exempt (gating on `valid_card` here would wrongly
        // exempt every observer, since `valid_card` is None for all of them).
        TriggerMode::DamageDone | TriggerMode::DamageDoneOnce
            if matches!(trig.valid_source, Some(TargetFilter::SelfRef)) =>
        {
            return false
        }
        TriggerMode::CounterAdded if matches!(trig.valid_card, Some(TargetFilter::SelfRef)) => {
            return false
        }
        _ => {}
    }
    if let Some(cond) = &trig.condition {
        if condition_excludes_second_copy(cond, census) {
            return false;
        }
    }
    true
}

/// §1.3.1-F (CR 603.4): does the shared intervening-if become FALSE whenever a
/// second identical source exists? True when it counts objects matching an
/// `Another`-self-exclusion filter the source ITSELF matches and requires that
/// count to be 0 (or `< 1`) — each of two copies then sees the other (count ≥ 1),
/// so neither trigger's re-check passes at trigger time.
fn condition_excludes_second_copy(cond: &TriggerCondition, census: &SourceCensus) -> bool {
    match cond {
        TriggerCondition::QuantityComparison {
            lhs,
            rhs,
            comparator,
        } => {
            let Some(filter) = object_count_filter(lhs) else {
                return false;
            };
            let Some(n) = fixed_value(rhs) else {
                return false;
            };
            filter_excludes_source(filter)
                && source_census_overlaps_filter(census, filter)
                && count_ge_one_falsifies(*comparator, n)
        }
        // "controls no Another-X" ≡ count == 0.
        TriggerCondition::ControlsNone { filter } => {
            filter_excludes_source(filter) && source_census_overlaps_filter(census, filter)
        }
        // A single false conjunct falsifies the whole AND ⇒ the trigger can't fire.
        TriggerCondition::And { conditions } => conditions
            .iter()
            .any(|c| condition_excludes_second_copy(c, census)),
        _ => false,
    }
}

fn object_count_filter(q: &QuantityExpr) -> Option<&TargetFilter> {
    match q {
        QuantityExpr::Ref {
            qty: QuantityRef::ObjectCount { filter },
        } => Some(filter),
        _ => None,
    }
}
fn fixed_value(q: &QuantityExpr) -> Option<i32> {
    match q {
        QuantityExpr::Fixed { value } => Some(*value),
        _ => None,
    }
}
/// Is `count <cmp> n` FALSE for every `count ≥ 1`? (`EQ 0` / `LE ≤0` / `LT ≤1`.)
fn count_ge_one_falsifies(cmp: Comparator, n: i32) -> bool {
    match cmp {
        Comparator::EQ => n == 0,
        Comparator::LE => n <= 0,
        Comparator::LT => n <= 1,
        _ => false,
    }
}

/// A no-ordering-input trigger shape (mirrors `trigger_has_no_ordering_input`):
/// no modal / division / target announcement (CR 603.3c/3d). `build_target_slots`
/// is the production target-collection authority; an error is treated as
/// has-input (conservatively excluded).
fn sweep_no_ordering_input(
    state: &GameState,
    resolved: &ResolvedAbility,
    def: &AbilityDefinition,
) -> bool {
    def.modal.is_none()
        && def.mode_abilities.is_empty()
        && def.target_constraints.is_empty()
        && resolved.multi_target.is_none()
        && resolved.distribution.is_none()
        && build_target_slots(state, resolved)
            .map(|slots| slots.is_empty())
            .unwrap_or(false)
}

/// A trigger's ability reads a live source P/T / counter characteristic
/// (`Power`/`Toughness`/`CountersOn` at `ObjectScope::Source`).
fn reads_source_pt_or_counter(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            let is_src_read = ["Power", "Toughness", "CountersOn"].iter().any(|k| {
                map.get(*k).is_some_and(|inner| {
                    serde_json::to_string(inner)
                        .map(|s| s.contains("\"Source\""))
                        .unwrap_or(false)
                })
            });
            is_src_read || map.values().any(reads_source_pt_or_counter)
        }
        serde_json::Value::Array(vs) => vs.iter().any(reads_source_pt_or_counter),
        _ => false,
    }
}

fn ability_rw_profile_merged(
    resolved: &ResolvedAbility,
    trig_condition: Option<&TriggerCondition>,
) -> crate::game::ability_rw::RwProfile {
    let mut p = ability_rw_profile(resolved);
    if let Some(tc) = trig_condition {
        p.merge(trigger_condition_rw_profile(tc));
    }
    p
}

/// PR-6.75 (CR 603.3b + CR 500.1): the CLOSED sweep-side controller-privacy
/// predicate. A `Phase` trigger carrying the `OnlyDuringYourTurn` constraint fires
/// only when `state.active_player == controller` (fire-time gate,
/// `triggers.rs:702` → `check_trigger_constraint`), and one `PhaseChanged` event
/// has exactly one active player — so EVERY reachable same-event group of such a
/// trigger is same-controller. Fail-closed for every other shape: "each [player's]"
/// phases parse to `constraint: None` (`oracle_trigger.rs`), opponent possessives
/// to `OnlyDuringOpponentsTurn`, and enchanted/chosen-player forms to a
/// `valid_target` with `constraint: None` — all ⇒ `false` here. The owner-alignment
/// half of `same_controller` is computed LIVE and fail-closed in production; the
/// sweep models the reachable canonical group (an exotic donated-source variant
/// merely over-prompts in production, never under-prompts).
fn trigger_is_controller_private(trig: &TriggerDefinition) -> bool {
    matches!(trig.mode, TriggerMode::Phase)
        && matches!(trig.constraint, Some(TriggerConstraint::OnlyDuringYourTurn))
}

#[test]
fn ordering_parity_sweep() {
    let db = shared_card_db();
    let full_db = std::env::var_os("FORGE_TEST_FULL_DB").is_some();
    // A minimal state solely for `build_target_slots` structural collection.
    let state = GameState::new_two_player(1);
    let src = ObjectId(1);
    let ctrl = PlayerId(0);

    let mut swept = 0usize;
    let mut compared = 0usize;
    let mut unexplained: Vec<String> = Vec::new();
    let mut cat1_hit: BTreeSet<String> = BTreeSet::new();

    // Non-vacuity floors (full-DB; the committed fixture is a subset).
    let mut floor_batch_self_srcread = 0usize;
    let mut floor_batch_obs = 0usize;
    let mut floor_hadcounters_batch_self = 0usize;
    let mut floor_retained_prompt = 0usize;
    let mut floor_t1_source_indep = 0usize;

    for (_key, face) in db.face_iter() {
        let name = face.name.to_lowercase();
        let census = face_census(face);
        for trig in &face.triggers {
            let Some(def) = trig.execute.as_deref() else {
                continue;
            };
            let resolved = build_resolved_from_def(def, src, ctrl);
            let value = serde_json::to_value(&resolved).unwrap_or(serde_json::Value::Null);
            if ast_bears_unimplemented(&value) {
                continue;
            }
            if !sweep_no_ordering_input(&state, &resolved, def) {
                continue;
            }
            swept += 1;

            let profile = ability_rw_profile_merged(&resolved, trig.condition.as_ref());
            let legacy_serde = legacy_allowlist(&resolved);
            let legacy_prompt = profile.legacy_batch_prompt();
            let self_ref = matches!(trig.valid_card, Some(TargetFilter::SelfRef));
            let excludes = trig
                .valid_card
                .as_ref()
                .map(filter_excludes_source)
                .unwrap_or(false);
            let event_present = mode_carries_event_object(&trig.mode);
            let is_departure = mode_is_battlefield_departure(&trig.mode, trig);
            let has_src_read = reads_source_pt_or_counter(&value);
            let hadcounters = matches!(trig.condition, Some(TriggerCondition::HadCounters { .. }));

            let mk = |same_event: bool,
                      all_same_source: bool,
                      self_departed: bool,
                      same_controller: bool| GroupStructure {
                same_event,
                all_same_source,
                all_sources_self_departed: self_departed,
                event_object_excludes_sources: if self_ref { false } else { excludes },
                event_object_present: event_present,
                source_census: census.clone(),
                same_controller,
            };

            // --- Same-event, distinct-source observer (S2) ---
            // §5.2: only where a 2-copy same-event group is REACHABLE (CLASS-A
            // guard — skip legendary / per-source-event / condition-self-excluding
            // shapes where the structure can never form). PR-6.75: the modeled
            // canonical S2 group is controller-private exactly for the fire-time-
            // pinned Phase+OnlyDuringYourTurn class (`trigger_is_controller_private`);
            // production additionally computes owner-alignment live and fail-closed.
            if !self_ref && same_event_group_reachable(face, trig, &census) {
                let se = mk(true, false, false, trigger_is_controller_private(trig));
                let conflict = profiles_conflict(&profile, &se);
                let decision_new = !conflict; // decision_old (same-event) == auto == true
                compared += 1;
                if !decision_new {
                    // A same-event diff (auto -> prompt): must be a category-(1) row.
                    if CATEGORY_1_ROWS.contains(&name.as_str()) {
                        cat1_hit.insert(name.clone());
                    } else {
                        unexplained.push(format!(
                            "SAME-EVENT auto->prompt on '{name}' (not a category-(1) row)"
                        ));
                    }
                } else if profile.source_independent() {
                    floor_t1_source_indep += 1;
                }
            }

            // --- Departure-batch structures (S3 self-departed / S5 mixed obs) ---
            if is_departure {
                // §5: batch rows model `same_controller = false` (different-
                // controller co-deaths are reachable) — the static sweep's batch
                // model is fail-closed; production computes it live.
                let batch = if self_ref {
                    mk(false, false, true, false) // self-dies
                } else {
                    mk(false, false, false, false) // observer batch
                };
                let batch_conflict = legacy_prompt || profiles_conflict(&profile, &batch);
                let decision_new = !batch_conflict;
                let decision_old = !legacy_serde;
                compared += 1;
                if decision_new != decision_old {
                    // Batch parity is ZERO-diff by design (D3). Any batch diff is a
                    // finding — no category exists for it.
                    unexplained.push(format!(
                        "BATCH decision diff on '{name}' (old={decision_old}, new={decision_new}, \
                         legacy_serde={legacy_serde}, legacy_prompt={legacy_prompt})"
                    ));
                }
                // Retained-prompt parity (D5): every legacy-ref trigger must still
                // prompt on the batch path.
                if legacy_prompt {
                    assert!(
                        batch_conflict,
                        "retained-prompt parity: '{name}' carries a legacy event-context ref \
                         but its batch group auto-orders"
                    );
                    floor_retained_prompt += 1;
                }
                if decision_new {
                    if self_ref {
                        if has_src_read {
                            floor_batch_self_srcread += 1;
                        }
                        if hadcounters {
                            floor_hadcounters_batch_self += 1;
                        }
                    } else {
                        floor_batch_obs += 1;
                    }
                }
            }
        }
    }

    eprintln!(
        "ordering_parity_sweep: full_db={full_db} swept={swept} compared={compared} \
         unexplained={} cat1_hit={:?}",
        unexplained.len(),
        cat1_hit
    );
    eprintln!(
        "floors: batch_self_srcread={floor_batch_self_srcread} batch_obs={floor_batch_obs} \
         hadcounters_batch_self={floor_hadcounters_batch_self} \
         retained_prompt={floor_retained_prompt} t1_source_indep={floor_t1_source_indep}"
    );

    assert!(
        unexplained.is_empty(),
        "STRICT PROOF-GATE: {} unexplained decision diff(s):\n{}",
        unexplained.len(),
        unexplained.join("\n")
    );

    // Non-vacuity: the iteration must actually have classified triggers.
    assert!(swept > 0, "sweep visited zero triggers (fixture missing?)");
    assert!(compared > 0, "sweep compared zero decisions");

    if full_db {
        // §5.2 positive floors (full-DB measured minima).
        assert!(
            floor_batch_self_srcread >= 40,
            "batch_self src-P/T/counter readers auto floor: {floor_batch_self_srcread} < 40"
        );
        assert!(
            floor_batch_obs >= 8,
            "batch_obs auto floor: {floor_batch_obs} < 8"
        );
        assert!(
            floor_hadcounters_batch_self >= 57,
            "HadCounters batch_self auto floor: {floor_hadcounters_batch_self} < 57"
        );
        assert!(
            floor_retained_prompt >= 1,
            "retained-prompt floor: {floor_retained_prompt} < 1"
        );
        assert!(
            floor_t1_source_indep >= 38,
            "T1 source-independent auto floor: {floor_t1_source_indep} < 38"
        );
    }
}

// ===================================================================
// Discriminators N-A / N-B / N-B2 / N-C / N-D / N-F
// (group_is_order_independent = the production ordering-soundness authority;
//  true = auto/no prompt, false = prompt/OrderTriggers)
// ===================================================================

fn ctx(
    source: u64,
    ability: ResolvedAbility,
    condition: Option<TriggerCondition>,
    event: Option<GameEvent>,
    die_result: Option<i32>,
) -> PendingTriggerContext {
    PendingTriggerContext::single(PendingTrigger {
        source_id: ObjectId(source),
        controller: PlayerId(0),
        condition,
        ability,
        timestamp: 0,
        target_constraints: Vec::new(),
        distribute: None,
        trigger_event: event,
        modal: None,
        mode_abilities: vec![],
        description: None,
        may_trigger_origin: None,
        subject_match_count: None,
        die_result,
    })
}

fn ra(effect: Effect) -> ResolvedAbility {
    ResolvedAbility::new(effect, vec![], ObjectId(1), PlayerId(0))
}
fn qfix(v: i32) -> QuantityExpr {
    QuantityExpr::Fixed { value: v }
}
fn qref(r: QuantityRef) -> QuantityExpr {
    QuantityExpr::Ref { qty: r }
}
fn creature() -> TargetFilter {
    TargetFilter::Typed(TypedFilter::creature())
}
fn power_src() -> QuantityRef {
    QuantityRef::Power {
        scope: ObjectScope::Source,
    }
}
fn put_counter_all(count: QuantityExpr) -> Effect {
    Effect::PutCounterAll {
        count,
        target: creature(),
        counter_type: CounterType::Plus1Plus1,
    }
}
fn token_power_src() -> Effect {
    Effect::Token {
        name: "Elemental".into(),
        power: crate::types::ability::PtValue::Fixed(1),
        toughness: crate::types::ability::PtValue::Fixed(1),
        types: vec!["Creature".into()],
        colors: vec![],
        keywords: vec![],
        tapped: false,
        count: qref(power_src()),
        owner: TargetFilter::Controller,
        attach_to: None,
        enters_attacking: false,
        supertypes: vec![],
        static_abilities: vec![],
        enter_with_counters: vec![],
    }
}
fn return_all_creatures_gy_to_bf() -> Effect {
    Effect::ChangeZone {
        origin: Some(Zone::Graveyard),
        destination: Zone::Battlefield,
        target: creature(),
        owner_library: false,
        enter_transformed: false,
        enters_under: None,
        enter_tapped: crate::types::zones::EtbTapState::default(),
        enters_attacking: false,
        up_to: false,
        enter_with_counters: vec![],
        conditional_enter_with_counters: vec![],
        face_down_profile: None,
        enters_modified_if: None,
    }
}
fn gain_life_fixed() -> Effect {
    Effect::GainLife {
        amount: qfix(1),
        player: TargetFilter::Controller,
    }
}
fn power_ge(n: i32) -> AbilityCondition {
    AbilityCondition::QuantityCheck {
        lhs: qref(power_src()),
        rhs: qfix(n),
        comparator: Comparator::GE,
    }
}
fn cond(mut a: ResolvedAbility, c: AbilityCondition) -> ResolvedAbility {
    a.condition = Some(c);
    a
}

/// A self-departure ZoneChanged event (object leaves the battlefield to the
/// graveyard), co-departing with `co`.
fn self_departure(id: u64, co: &[u64]) -> Option<GameEvent> {
    Some(GameEvent::ZoneChanged {
        object_id: ObjectId(id),
        from: Some(Zone::Battlefield),
        to: Zone::Graveyard,
        record: Box::new(ZoneChangeRecord {
            co_departed: co.iter().map(|&x| ObjectId(x)).collect(),
            ..ZoneChangeRecord::test_minimal(ObjectId(id), Some(Zone::Battlefield), Zone::Graveyard)
        }),
    })
}

/// One shared ETB event of a third object (id 99) — both observers fire on it.
fn shared_etb_event() -> Option<GameEvent> {
    Some(GameEvent::ZoneChanged {
        object_id: ObjectId(99),
        from: Some(Zone::Hand),
        to: Zone::Battlefield,
        record: Box::new(ZoneChangeRecord::test_minimal(
            ObjectId(99),
            Some(Zone::Hand),
            Zone::Battlefield,
        )),
    })
}

fn empty_state() -> GameState {
    GameState::new_two_player(9)
}

/// N-A: two Nested Shamblers (Token{count: Power{Source}}) co-departing in one
/// SBA batch ⇒ their Source-power reads are LKI-frozen ⇒ NO OrderTriggers prompt.
/// (The frozen 3+1 token counts are a quantity-resolution concern pinned by
/// `quantity.rs`'s LKI tests; this asserts the ORDERING verdict this PR owns.)
#[test]
fn n_a_shambler_co_departure_auto_orders() {
    let state = empty_state();
    let group = vec![
        ctx(
            10,
            ra(token_power_src()),
            None,
            self_departure(10, &[11]),
            None,
        ),
        ctx(
            11,
            ra(token_power_src()),
            None,
            self_departure(11, &[10]),
            None,
        ),
    ];
    assert!(
        group_is_order_independent(&state, &group, false),
        "co-departing Shamblers read frozen LKI power ⇒ must auto-order (no prompt)"
    );
}

/// N-B: the frozen-vs-live discriminator. Same AST (PutCounterAll{Power{Source}}
/// — CR 122.1 counters feed P/T) resolves differently by group structure:
///   * co-departure batch ⇒ Source read FROZEN ⇒ no feed ⇒ AUTO.
///   * same-event alive pair ⇒ Source read LIVE ⇒ counter write feeds it ⇒ PROMPT.
///
/// Proves the freeze is applied exactly on the departure-batch path.
#[test]
fn n_b_frozen_vs_live_discriminator() {
    let state = empty_state();
    let ability = || ra(put_counter_all(qref(power_src())));

    let batch = vec![
        ctx(10, ability(), None, self_departure(10, &[11]), None),
        ctx(11, ability(), None, self_departure(11, &[10]), None),
    ];
    assert!(
        group_is_order_independent(&state, &batch, false),
        "co-departure: frozen Source read ⇒ auto"
    );

    let ev = shared_etb_event();
    let same_event = vec![
        ctx(10, ability(), None, ev.clone(), None),
        ctx(11, ability(), None, ev, None),
    ];
    assert!(
        !group_is_order_independent(&state, &same_event, false),
        "same-event alive: live Source read fed by sibling counter write ⇒ prompt"
    );
}

/// N-B2: the freeze-invalidation hostile. A co-departing pair that ALSO returns
/// creature cards from the graveyard to the battlefield (external existing-object
/// move, battlefield destination ⇒ re-entry hazard) re-binds a member's ObjectId
/// to live state, so the frozen Source read is invalidated ⇒ PROMPT. The plain
/// pair (no return clause) stays auto ⇒ the row prompts exactly the hazard groups.
#[test]
fn n_b2_freeze_invalidation_hostile() {
    let state = empty_state();
    let hazard = || ra(token_power_src()).sub_ability(ra(return_all_creatures_gy_to_bf()));
    let hazard_group = vec![
        ctx(10, hazard(), None, self_departure(10, &[11]), None),
        ctx(11, hazard(), None, self_departure(11, &[10]), None),
    ];
    assert!(
        !group_is_order_independent(&state, &hazard_group, false),
        "battlefield-return hazard invalidates the LKI freeze ⇒ prompt"
    );

    let plain = vec![
        ctx(
            10,
            ra(token_power_src()),
            None,
            self_departure(10, &[11]),
            None,
        ),
        ctx(
            11,
            ra(token_power_src()),
            None,
            self_departure(11, &[10]),
            None,
        ),
    ];
    assert!(
        group_is_order_independent(&state, &plain, false),
        "plain Shambler shape (no hazard write) ⇒ auto — the row is hazard-scoped"
    );
}

/// N-C: Case A (CR 603.3b). Two byte-identical "put +1/+1 on each creature you
/// control; draw if this creature's power ≥ 6" off ONE event: the counter write
/// feeds the sibling's live Source-power read ⇒ order-observable ⇒ PROMPT.
#[test]
fn n_c_case_a_same_event_prompts() {
    let state = empty_state();
    let ability = || cond(ra(put_counter_all(qfix(1))), power_ge(6));
    let ev = shared_etb_event();
    let group = vec![
        ctx(10, ability(), None, ev.clone(), None),
        ctx(11, ability(), None, ev, None),
    ];
    assert!(
        !group_is_order_independent(&state, &group, false),
        "Case A: counter write feeds live power threshold ⇒ prompt (CR 603.3b fix)"
    );
}

/// N-D: intervening-if Case A. The threshold rides the TRIGGER-level condition
/// (CR 603.4 — re-checked at resolution) instead of the ability. The merged
/// `trigger_condition_rw_profile` carries the Source read ⇒ PROMPT. Revert-fail:
/// dropping the trigger-condition merge auto-orders it.
#[test]
fn n_d_intervening_if_case_a_prompts() {
    let state = empty_state();
    let trig_cond = || TriggerCondition::QuantityComparison {
        lhs: qref(power_src()),
        rhs: qfix(6),
        comparator: Comparator::GE,
    };
    let ev = shared_etb_event();
    let group = vec![
        ctx(
            10,
            ra(put_counter_all(qfix(1))),
            Some(trig_cond()),
            ev.clone(),
            None,
        ),
        ctx(
            11,
            ra(put_counter_all(qfix(1))),
            Some(trig_cond()),
            ev,
            None,
        ),
    ];
    assert!(
        !group_is_order_independent(&state, &group, false),
        "intervening-if Source read fed by sibling counter write ⇒ prompt"
    );
}

/// N-F: die_result conjunct unit pin (CR 706.2 + CR 603.12). Two otherwise-
/// identical no-input source-independent triggers off one event with DIFFERENT
/// stamped die results are NOT the same state transformation ⇒ not order-
/// independent; EQUAL die results are admitted. Revert-fail: removing the
/// conjunct admits the differing pair.
#[test]
fn n_f_die_result_conjunct() {
    let state = empty_state();
    let ev = shared_etb_event();
    let differing = vec![
        ctx(10, ra(gain_life_fixed()), None, ev.clone(), Some(3)),
        ctx(11, ra(gain_life_fixed()), None, ev.clone(), Some(5)),
    ];
    assert!(
        !group_is_order_independent(&state, &differing, false),
        "differing die_result ⇒ not order-independent"
    );

    let equal = vec![
        ctx(10, ra(gain_life_fixed()), None, ev.clone(), Some(3)),
        ctx(11, ra(gain_life_fixed()), None, ev, Some(3)),
    ];
    assert!(
        group_is_order_independent(&state, &equal, false),
        "equal die_result + source-independent same-event pair ⇒ admitted (auto)"
    );
}

/// CLASS-A guard, FIX A: the `DamageDone` same-event exemption keys on
/// `valid_source` (damage triggers leave `valid_card` None — they carry their
/// subject in `valid_source`). A SELF-damage trigger (`valid_source == SelfRef`)
/// is per-source ⇒ exempt from the S2 comparison; an OBSERVER damage trigger
/// (`valid_source` non-SelfRef) fires two copies off ONE source's damage event
/// ⇒ compared. Revert-fail: gating on `valid_card` (None for BOTH) would exempt
/// the observer too, voiding the D3 same-event parity proof for observer damage.
#[test]
fn class_a_damage_done_exempts_self_source_not_observer() {
    let face = CardFace::default(); // non-legendary
    let census = SourceCensus::unknown();

    let mut self_dmg = TriggerDefinition::new(TriggerMode::DamageDone);
    self_dmg.valid_source = Some(TargetFilter::SelfRef);
    assert!(
        !same_event_group_reachable(&face, &self_dmg, &census),
        "self-damage (valid_source=SelfRef) ⇒ per-source ⇒ exempt from S2"
    );

    let mut observer = TriggerDefinition::new(TriggerMode::DamageDone);
    observer.valid_source = Some(TargetFilter::Typed(TypedFilter::creature()));
    // valid_card stays None (as for every real damage trigger).
    assert!(
        same_event_group_reachable(&face, &observer, &census),
        "observer damage (valid_source non-SelfRef) ⇒ shared source event ⇒ compared"
    );
}

/// N-G (D5, CR 603.10a): the fail-open batch-prompt hole this commit closes,
/// proven through the production ordering authority. A co-departing pair of
/// identical "when this leaves the battlefield, target player discards" triggers
/// carries `TargetFilter::TriggeringPlayer` in the `Discard` TARGET position —
/// one of the 12 frozen event-context tags. `rw_effect`'s `Discard` arm ignores
/// its `target` field, so before this commit the read/write walk never routed
/// the tag through a legacy leaf detector and `legacy_batch_prompt` stayed false:
/// the departure batch auto-ordered where the shipped engine prompted.
///
/// Revert-fail witness: removing the `p.legacy_batch_prompt =
/// contains_legacy_event_ref(a)` override in `ability_rw_profile` makes the
/// legacy group auto-order and flips the first assertion. The `Controller`
/// control (identical discard, no frozen tag) proves the prompt is driven by the
/// tag itself, not by `Discard`'s effect shape.
#[test]
fn n_g_dropped_target_legacy_ref_retains_batch_prompt() {
    let state = empty_state();
    let discard = |t: TargetFilter| Effect::Discard {
        count: qfix(1),
        target: t,
        unless_filter: None,
        filter: None,
        selection: crate::types::ability::CardSelectionMode::Chosen,
    };

    let legacy_group = vec![
        ctx(
            10,
            ra(discard(TargetFilter::TriggeringPlayer)),
            None,
            self_departure(10, &[11]),
            None,
        ),
        ctx(
            11,
            ra(discard(TargetFilter::TriggeringPlayer)),
            None,
            self_departure(11, &[10]),
            None,
        ),
    ];
    assert!(
        !group_is_order_independent(&state, &legacy_group, false),
        "Discard{{TriggeringPlayer}} on a departure batch carries a frozen tag ⇒ retain prompt"
    );

    let control_group = vec![
        ctx(
            10,
            ra(discard(TargetFilter::Controller)),
            None,
            self_departure(10, &[11]),
            None,
        ),
        ctx(
            11,
            ra(discard(TargetFilter::Controller)),
            None,
            self_departure(11, &[10]),
            None,
        ),
    ];
    assert!(
        group_is_order_independent(&state, &control_group, false),
        "identical Discard with no frozen tag (Controller) ⇒ auto-order"
    );
}

// ===================================================================
// PR-6.75 `same_controller` span discriminators (S1–S6), driven through the
// production authority `group_is_order_independent`. POSITIVE (auto) groups
// install source objects owned AND controlled by the pending controller so the
// LIVE `same_controller` check holds; each NEG breaks exactly ONE axis (span
// disjointness, controller-uniformity, or owner-alignment) and must re-prompt —
// the paired positive reach-guard proves the auto is delivered by the refined
// row, not an upstream fast-path short-circuit.
// ===================================================================

/// Install a battlefield source with explicit owner + controller (the live-state
/// precondition the chokepoint reads: `o.controller == o.owner == c0`).
fn install_source(state: &mut GameState, id: u64, owner: u8, controller: u8) {
    let mut o = GameObject::new(
        ObjectId(id),
        CardId(1),
        PlayerId(owner),
        "Src".to_string(),
        Zone::Battlefield,
    );
    o.owner = PlayerId(owner);
    o.controller = PlayerId(controller);
    state.objects.insert(ObjectId(id), o);
}

/// A pending-trigger context with an explicit controller (S4/S5/S6 vary it).
fn ctx_c(
    source: u64,
    controller: u8,
    ability: ResolvedAbility,
    condition: Option<TriggerCondition>,
    event: Option<GameEvent>,
) -> PendingTriggerContext {
    PendingTriggerContext::single(PendingTrigger {
        source_id: ObjectId(source),
        controller: PlayerId(controller),
        condition,
        ability,
        timestamp: 0,
        target_constraints: Vec::new(),
        distribute: None,
        trigger_event: event,
        modal: None,
        mode_abilities: vec![],
        description: None,
        may_trigger_origin: None,
        subject_match_count: None,
        die_result: None,
    })
}

fn creatures_of(cr: ControllerRef) -> TargetFilter {
    let mut tf = TypedFilter::creature();
    tf.controller = Some(cr);
    TargetFilter::Typed(tf)
}

fn sacrifice_self() -> Effect {
    Effect::Sacrifice {
        target: TargetFilter::SelfRef,
        count: qfix(1),
        min_count: 0,
    }
}
/// Defense-of-the-Heart shape: search the CONTROLLER's own library (no
/// `target_player`) → the phantom write earns a `You` chain move-owner fact.
fn search_own_creatures() -> Effect {
    Effect::SearchLibrary {
        source_zones: vec![Zone::Library],
        filter: creature(),
        count: QuantityExpr::UpTo {
            max: Box::new(qfix(2)),
        },
        reveal: false,
        target_player: None,
        selection_constraint: SearchSelectionConstraint::None,
        split: None,
    }
}
/// The chained opaque battlefield entry (`Library → Battlefield`, `Any` target,
/// no `enters_under`) that consumes the search's `You` move-owner fact.
fn change_zone_lib_to_bf() -> Effect {
    Effect::ChangeZone {
        origin: Some(Zone::Library),
        destination: Zone::Battlefield,
        target: TargetFilter::Any,
        owner_library: false,
        enter_transformed: false,
        enters_under: None,
        enter_tapped: crate::types::zones::EtbTapState::default(),
        enters_attacking: false,
        up_to: false,
        enter_with_counters: vec![],
        conditional_enter_with_counters: vec![],
        face_down_profile: None,
        enters_modified_if: None,
    }
}
fn shuffle_ctrl() -> Effect {
    Effect::Shuffle {
        target: TargetFilter::Controller,
    }
}
fn bounce_self() -> Effect {
    Effect::Bounce {
        target: TargetFilter::SelfRef,
        destination: None,
        selection: crate::types::ability::BounceSelection::default(),
    }
}
fn discard_hand(count: QuantityExpr, target: TargetFilter) -> Effect {
    Effect::Discard {
        count,
        target,
        unless_filter: None,
        filter: None,
        selection: crate::types::ability::CardSelectionMode::Chosen,
    }
}
fn obj_count_cmp(filter: TargetFilter, cmp: Comparator, rhs: i32) -> TriggerCondition {
    TriggerCondition::QuantityComparison {
        lhs: qref(QuantityRef::ObjectCount { filter }),
        rhs: qfix(rhs),
        comparator: cmp,
    }
}
fn handsize_cmp(player: PlayerScope, cmp: Comparator, rhs: i32) -> TriggerCondition {
    TriggerCondition::QuantityComparison {
        lhs: qref(QuantityRef::HandSize { player }),
        rhs: qfix(rhs),
        comparator: cmp,
    }
}

/// Defense-of-the-Heart chain: `Sacrifice{SelfRef}` → `SearchLibrary` →
/// `ChangeZone{Library→Bf, Any}` → `Shuffle`.
fn defense_ability() -> ResolvedAbility {
    let cz = ra(change_zone_lib_to_bf()).sub_ability(ra(shuffle_ctrl()));
    let search = ra(search_own_creatures()).sub_ability(cz);
    ra(sacrifice_self()).sub_ability(search)
}

/// S-1 — Defense of the Heart shape. POS: same-event 2-copy, both P0, sources
/// installed owner==controller==P0; the opponents'-board census read is
/// ctrl-disjoint (Opponents) from the your-library battlefield entry (You) ⇒
/// AUTO. NEG reach-guard: flip the read's controller to `You` ⇒ ctrl spans
/// overlap ⇒ PROMPT. The NEG also proves the auto is NOT a `source_independent`
/// fast-path fluke — `Sacrifice{SelfRef}` keeps `source_independent` false in
/// both, so only the membership-ctrl span differs.
#[test]
fn s1_defense_membership_ctrl_span() {
    let mut state = empty_state();
    install_source(&mut state, 10, 0, 0);
    install_source(&mut state, 11, 0, 0);
    let ev = shared_etb_event();

    let pos_cond = || obj_count_cmp(creatures_of(ControllerRef::Opponent), Comparator::GE, 3);
    let pos = vec![
        ctx_c(10, 0, defense_ability(), Some(pos_cond()), ev.clone()),
        ctx_c(11, 0, defense_ability(), Some(pos_cond()), ev.clone()),
    ];
    assert!(
        group_is_order_independent(&state, &pos, false),
        "S1 POS: opponents'-board read × your-library entry are ctrl-disjoint ⇒ auto"
    );

    let neg_cond = || obj_count_cmp(creatures_of(ControllerRef::You), Comparator::GE, 3);
    let neg = vec![
        ctx_c(10, 0, defense_ability(), Some(neg_cond()), ev.clone()),
        ctx_c(11, 0, defense_ability(), Some(neg_cond()), ev),
    ];
    assert!(
        !group_is_order_independent(&state, &neg, false),
        "S1 NEG: your-board read × your-library entry ctrl spans overlap ⇒ prompt"
    );
}

/// S-2 — Rekindled Flame shape (`Bounce{SelfRef}` + `HandSize{Opponent} EQ 0`
/// intervening-if). POS: opp-hand read (Opponents) × your-hand self-bounce write
/// (You) ⇒ AUTO. Sibling NEG: read `HandSize{Controller}` (You) ⇒ hand spans
/// overlap ⇒ PROMPT.
#[test]
fn s2_rekindled_player_hand_span() {
    let mut state = empty_state();
    install_source(&mut state, 10, 0, 0);
    install_source(&mut state, 11, 0, 0);
    let ev = shared_etb_event();

    let pos_cond = || {
        handsize_cmp(
            PlayerScope::Opponent {
                aggregate: AggregateFunction::Min,
            },
            Comparator::EQ,
            0,
        )
    };
    let pos = vec![
        ctx_c(10, 0, ra(bounce_self()), Some(pos_cond()), ev.clone()),
        ctx_c(11, 0, ra(bounce_self()), Some(pos_cond()), ev.clone()),
    ];
    assert!(
        group_is_order_independent(&state, &pos, false),
        "S2 POS: opp-hand read × your-hand self-bounce are player-disjoint ⇒ auto"
    );

    let neg_cond = || handsize_cmp(PlayerScope::Controller, Comparator::EQ, 0);
    let neg = vec![
        ctx_c(10, 0, ra(bounce_self()), Some(neg_cond()), ev.clone()),
        ctx_c(11, 0, ra(bounce_self()), Some(neg_cond()), ev),
    ];
    assert!(
        !group_is_order_independent(&state, &neg, false),
        "S2 NEG: your-hand read × your-hand write overlap ⇒ prompt"
    );
}

/// Brink-of-Madness chain: `Sacrifice{SelfRef}` → scoped-Opponent
/// `Discard{count, target: Controller}`.
fn brink_ability(count: QuantityExpr) -> ResolvedAbility {
    let mut discard = ra(discard_hand(count, TargetFilter::Controller));
    discard.player_scope = Some(PlayerFilter::Opponent);
    ra(sacrifice_self()).sub_ability(discard)
}

/// S-3 — Brink of Madness fused RMW discriminator. POS: `count:
/// HandSize{ScopedPlayer}` is the fused "discards their hand" read ⇒ dropped
/// under the gate, leaving the your-hand intervening-if (You) vs the opp-hand
/// Discard write (Opponents) ⇒ AUTO. Fusion NEG: the SAME opp-scoped Discard but
/// `count: HandSize{Opponent}` — a genuine (non-fused) opp-hand observation that
/// is NOT dropped ⇒ read span degrades to Any ⇒ overlaps the write ⇒ PROMPT.
/// Isolates the fusion arm exactly (same write both sides).
#[test]
fn s3_brink_fused_discard_span() {
    let mut state = empty_state();
    install_source(&mut state, 10, 0, 0);
    install_source(&mut state, 11, 0, 0);
    let ev = shared_etb_event();
    let cond = || handsize_cmp(PlayerScope::Controller, Comparator::EQ, 0);

    let fused = qref(QuantityRef::HandSize {
        player: PlayerScope::ScopedPlayer,
    });
    let pos = vec![
        ctx_c(
            10,
            0,
            brink_ability(fused.clone()),
            Some(cond()),
            ev.clone(),
        ),
        ctx_c(11, 0, brink_ability(fused), Some(cond()), ev.clone()),
    ];
    assert!(
        group_is_order_independent(&state, &pos, false),
        "S3 POS: fused HandSize{{ScopedPlayer}} count dropped ⇒ You-cond × Opp-write disjoint ⇒ auto"
    );

    let unfused = qref(QuantityRef::HandSize {
        player: PlayerScope::Opponent {
            aggregate: AggregateFunction::Min,
        },
    });
    let neg = vec![
        ctx_c(
            10,
            0,
            brink_ability(unfused.clone()),
            Some(cond()),
            ev.clone(),
        ),
        ctx_c(11, 0, brink_ability(unfused), Some(cond()), ev),
    ];
    assert!(
        !group_is_order_independent(&state, &neg, false),
        "S3 NEG: non-fused opp-hand count read is a live player read ⇒ prompt"
    );
}

/// S-4 — load-bearing shared-event observer (the gate is load-bearing). Two
/// Rekindled-shape triggers off ONE event with controllers P0 and P1: mixed
/// controllers ⇒ `same_controller = false` ⇒ the spans are UNCONSULTED ⇒ the
/// hand row fires ⇒ PROMPT. Revert-fail foil (same shapes, both P0): the ONLY
/// change is the controller/owner of source 11 ⇒ `same_controller = true` ⇒
/// AUTO. Deleting the gate in `profiles_conflict` would flip the P0/P1 case to
/// auto (RED).
#[test]
fn s4_gate_is_load_bearing() {
    let mut state = empty_state();
    install_source(&mut state, 10, 0, 0);
    install_source(&mut state, 11, 1, 1); // owned + controlled by P1
    let ev = shared_etb_event();
    let cond = || {
        handsize_cmp(
            PlayerScope::Opponent {
                aggregate: AggregateFunction::Min,
            },
            Comparator::EQ,
            0,
        )
    };

    let mixed = vec![
        ctx_c(10, 0, ra(bounce_self()), Some(cond()), ev.clone()),
        ctx_c(11, 1, ra(bounce_self()), Some(cond()), ev.clone()),
    ];
    assert!(
        !group_is_order_independent(&state, &mixed, false),
        "S4 NEG: mixed controllers ⇒ same_controller false ⇒ spans unconsulted ⇒ prompt"
    );

    // Revert-fail foil: reinstall source 11 under P0 and flip the pending
    // controller — the sole change that makes the group controller-private.
    install_source(&mut state, 11, 0, 0);
    let uniform = vec![
        ctx_c(10, 0, ra(bounce_self()), Some(cond()), ev.clone()),
        ctx_c(11, 0, ra(bounce_self()), Some(cond()), ev),
    ];
    assert!(
        group_is_order_independent(&state, &uniform, false),
        "S4 foil: both P0 ⇒ same_controller true ⇒ span disjointness clears ⇒ auto"
    );
}

/// S-5 — MULTIPLAYER (3 players). (a) members P0 and P1: mixed controllers ⇒
/// same_controller false ⇒ PROMPT (at 3p, P1's opponents include P0, so treating
/// them as controller-private would be unsound — the gate correctly refuses).
/// (b) both members P0: `You == {P0}` vs `Opponents == {P1,P2}` disjointness is
/// NOT a two-player artifact ⇒ AUTO at N players.
#[test]
fn s5_multiplayer_three_players() {
    let mut state = GameState::new(crate::types::format::FormatConfig::standard(), 3, 7);
    install_source(&mut state, 10, 0, 0);
    install_source(&mut state, 11, 1, 1);
    install_source(&mut state, 12, 0, 0);
    let ev = shared_etb_event();
    let cond = || {
        handsize_cmp(
            PlayerScope::Opponent {
                aggregate: AggregateFunction::Min,
            },
            Comparator::EQ,
            0,
        )
    };

    let mixed = vec![
        ctx_c(10, 0, ra(bounce_self()), Some(cond()), ev.clone()),
        ctx_c(11, 1, ra(bounce_self()), Some(cond()), ev.clone()),
    ];
    assert!(
        !group_is_order_independent(&state, &mixed, false),
        "S5a: 3p mixed controllers ⇒ same_controller false ⇒ prompt"
    );

    let both_p0 = vec![
        ctx_c(10, 0, ra(bounce_self()), Some(cond()), ev.clone()),
        ctx_c(12, 0, ra(bounce_self()), Some(cond()), ev),
    ];
    assert!(
        group_is_order_independent(&state, &both_p0, false),
        "S5b: 3p both P0 ⇒ You={{P0}} vs Opponents={{P1,P2}} disjoint ⇒ auto (not a 2p artifact)"
    );
}

/// S-6 — owner-alignment discriminator (the `o.owner == c0` conjunct is
/// load-bearing). Rekindled shape, both members controlled by P0, but source 11
/// is OWNED by P1 (donated / control-changed). NEG: owner mismatch ⇒
/// same_controller false ⇒ PROMPT — and this is a REAL under-prompt guard: the
/// self-bounce would put source 11 in P1's hand, which the `HandSize{Opponent}`
/// read observes. Revert-fail foil (source 11 owned by P0): same_controller true
/// ⇒ AUTO. Dropping the owner conjunct would auto the NEG (RED).
#[test]
fn s6_owner_alignment() {
    let mut state = empty_state();
    install_source(&mut state, 10, 0, 0);
    install_source(&mut state, 11, 1, 0); // owner P1, controller P0 (donated)
    let ev = shared_etb_event();
    let cond = || {
        handsize_cmp(
            PlayerScope::Opponent {
                aggregate: AggregateFunction::Min,
            },
            Comparator::EQ,
            0,
        )
    };

    let donated = vec![
        ctx_c(10, 0, ra(bounce_self()), Some(cond()), ev.clone()),
        ctx_c(11, 0, ra(bounce_self()), Some(cond()), ev.clone()),
    ];
    assert!(
        !group_is_order_independent(&state, &donated, false),
        "S6 NEG: source owned by an opponent ⇒ owner-misaligned ⇒ prompt (real under-prompt guard)"
    );

    install_source(&mut state, 11, 0, 0); // now owner == controller == P0
    let aligned = vec![
        ctx_c(10, 0, ra(bounce_self()), Some(cond()), ev.clone()),
        ctx_c(11, 0, ra(bounce_self()), Some(cond()), ev),
    ];
    assert!(
        group_is_order_independent(&state, &aligned, false),
        "S6 foil: owner == controller == P0 ⇒ same_controller true ⇒ auto"
    );
}
