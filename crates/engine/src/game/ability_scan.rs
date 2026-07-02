//! CR 603.3b + CR 603.4 + CR 106.1/119/122.1: the PR-6.25/PR-6.5 fail-closed AST
//! scanner — a single compiler-exhaustive, wildcard-free walk of a resolved
//! ability's typed AST that answers three independent classification questions
//! ("axes") used by trigger ordering (CR 603.3b) and the growing-cascade
//! coverability detector (`analysis::resource`):
//!
//! 1. **event-context read** — does the ability read a characteristic of the
//!    concrete triggering event / cost-paid object (CR 603.4 / CR 608.2k)? Two
//!    order-independent-looking triggers off *distinct* events are only truly
//!    interchangeable if neither reads the event that distinguishes them.
//! 2. **sibling-mutable read** — does the ability read a source/recipient or
//!    board-scoped mutable P/T / counter aggregate that a sibling copy resolving
//!    first could change (the Rubblebelt Rioters / Orcish Siegemaster class)?
//! 3. **projected-resource read** — does the ability read a player-level monotone
//!    resource or per-turn/per-game journal that
//!    `analysis::resource::project_out_resources` zeroes/clears (life CR 119,
//!    floating mana CR 106.1, poison/energy/player counters CR 122.1, and the
//!    per-turn tally/journal block)? Object counters and marked damage are NOT on
//!    this axis — they are strict-compared by gate (1) of
//!    `loop_states_cover_modulo_growth` (R5-B1), so an object-counter reader
//!    (`CountersOn`/`Power`/`Toughness`) classifies as a NON-reader here.
//!
//! # Why hand-rolled and wildcard-free
//!
//! The soundness of both consumers rests on the scanner being **fail-closed on
//! future variants**: a new `Effect`/`QuantityRef`/`TriggerCondition`/… variant
//! must fail to compile until it is given an explicit reads/doesn't-read decision
//! on every axis. A `_ =>` wildcard (or a serde-tag string walk) silently defeats
//! that — a new event-context or resource reader would be classified inert and
//! ride a false auto-resolution / false coverability win. Therefore every arm is
//! explicit; provably-inert variants get a one-line `Axes::NONE` arm. Types the
//! walk does not descend into (`ContinuousModification`, `ManaProduction`,
//! `ReplacementDefinition`, a nested `ResolvedAbility`, `FilterProp`, the
//! per-mode `AbilityDefinition`s of a reflexive-modal trigger (`mode_abilities`),
//! …) that can transitively express a read are classified **conservatively**
//! (`Axes::CONSERVATIVE` — reads on every axis), the fail-safe direction for all
//! three consumers (over-prompt / over-reject, never a false auto-resolve or
//! false win). `RestrictionPlayerScope` and `CastManaObjectScope` are also in the
//! conservative set: their only carriers (`Effect::AddRestriction` /
//! `AddTargetReplacement`, `QuantityRef::ManaSpentToCast`) already return
//! `Axes::CONSERVATIVE`, so the scopes themselves are never traversed.
//!
//! # Traversal closure (R4-G2)
//!
//! The compiler-exhaustiveness floor holds only for TRAVERSED subtrees: an
//! untraversed payload is silently skipped with no compile error, so the traversal
//! set is part of the trusted base. It is closed under payload reachability across
//! `Effect`, `QuantityRef`, `QuantityExpr`, `AbilityCondition`, `TargetFilter`,
//! `ObjectScope`, `TriggerCondition`, `Duration` (its `ForAsLongAs` `StaticCondition`),
//! `StaticCondition`, `PlayerFilter`, `ReplacementCondition`, the target-count and
//! target-set specs (`MultiTargetSpec`, `TargetSelectionConstraint`), the loop and
//! modal headers (`RepeatContinuation`, `ModalChoice`), and the player scope
//! selectors (`PlayerScope`, `ControllerRef`, `CountScope`). The `ResolvedAbility`
//! and `ModalChoice` fields are destructured without `..`, so a new field must be
//! classified before it compiles. Any type outside this set that can reach a read
//! is in the conservative set above.

use crate::types::ability::{
    AbilityCondition, ControllerRef, CountScope, Duration, Effect, ModalChoice, MultiTargetSpec,
    ObjectScope, PlayerFilter, PlayerScope, QuantityExpr, QuantityRef, RepeatContinuation,
    ReplacementCondition, ResolvedAbility, StaticCondition, TargetFilter, TriggerCondition,
};
use crate::types::game_state::TargetSelectionConstraint;

/// The three independent classification axes, accumulated over one AST walk.
/// `true` on an axis means "reads (or may read) that dimension"; the fail-safe
/// direction for every consumer.
#[derive(Clone, Copy)]
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
struct Axes {
    /// Reads a concrete-triggering-event / cost-paid-object characteristic
    /// (CR 603.4 / CR 608.2k). Used by trigger ordering to keep distinct-event
    /// groups from auto-resolving.
    event: bool,
    /// Reads a source/recipient or board-scoped mutable aggregate a sibling copy
    /// could mutate (CR 603.3b ordering-relevance).
    sibling: bool,
    /// Reads a player-level monotone resource / per-turn journal that
    /// `project_out_resources` neutralizes (CR 106.1/119/122.1).
    projected: bool,
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
impl Axes {
    /// No read on any axis.
    const NONE: Axes = Axes {
        event: false,
        sibling: false,
        projected: false,
    };
    /// A subtree the walk does not descend into but which can transitively express
    /// a read — classified as reading everything (fail-closed / fail-safe).
    const CONSERVATIVE: Axes = Axes {
        event: true,
        sibling: true,
        projected: true,
    };

    fn or(self, other: Axes) -> Axes {
        Axes {
            event: self.event || other.event,
            sibling: self.sibling || other.sibling,
            projected: self.projected || other.projected,
        }
    }
}

/// Walk a resolved ability's read-bearing fields.
///
/// The `ResolvedAbility` destructure below is **exhaustive with no `..` rest
/// pattern** — the struct-level analogue of the walk's no-wildcard match
/// discipline. Every field is either scanned (read-bearing) or bound to `_`
/// with a one-line "read-free" justification; a FUTURE field added to
/// `ResolvedAbility` fails to compile here until it is classified, closing the
/// "unread aux field" hole class at compile time (not just `multi_target` /
/// `target_constraints`).
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn resolved_ability_axes(a: &ResolvedAbility) -> Axes {
    let ResolvedAbility {
        // ---- read-bearing: scanned into `acc` below ----
        effect,
        sub_ability,
        else_ability,
        condition,
        duration,
        player_scope,
        starting_with,
        repeat_for,
        multi_target,
        target_constraints,
        unless_pay,
        target_chooser,
        repeat_until,
        modal,
        mode_abilities,
        // ---- read-free: concrete ids / cast-time snapshots / flags / links,
        //      none of which express a resolution-time dynamic read ----
        targets: _,               // concrete announced target refs (already resolved)
        source_id: _,             // object id
        source_incarnation: _,    // epoch guard token
        controller: _,            // player id
        original_controller: _,   // player id
        scoped_player: _,         // player id (iteration binding)
        kind: _,                  // AbilityKind tag (no payload)
        context: _,               // SpellContext: cast-time fact snapshot, not a live read
        optional_targeting: _,    // bool
        optional: _,              // bool
        optional_for: _,          // OpponentMayScope: AnyOpponent/AnyPlayer, no read
        target_choice_timing: _,  // Stack/Resolution tag
        description: _,           // display string
        min_x_value: _,           // u32
        cant_be_copied: _,        // bool
        copy_count_status: _,     // status tag
        forward_result: _,        // bool
        distribution: _,          // concrete pre-assigned (TargetRef, u32) portions
        chosen_x: _,              // concrete cast-time X
        cost_paid_object: _,      // concrete captured-object snapshot
        effect_context_object: _, // concrete captured-object snapshot
        ability_index: _,         // usize provenance
        may_trigger_origin: _,    // provenance tag
        target_selection_mode: _, // Chosen/Random tag
        chosen_players: _,        // concrete chosen player ids
        sub_link: _,              // SubAbilityLink kind tag
        dig_found_nothing_for_parent_target: _, // bool seam flag
    } = a;

    let mut acc = scan_effect(effect);
    if let Some(sub) = sub_ability {
        acc = acc.or(resolved_ability_axes(sub));
    }
    if let Some(else_branch) = else_ability {
        acc = acc.or(resolved_ability_axes(else_branch));
    }
    if let Some(condition) = condition {
        acc = acc.or(scan_ability_condition(condition));
    }
    if let Some(duration) = duration {
        acc = acc.or(scan_duration(duration));
    }
    if let Some(player_scope) = player_scope {
        acc = acc.or(scan_player_filter(player_scope));
    }
    if let Some(starting_with) = starting_with {
        acc = acc.or(scan_controller_ref(starting_with));
    }
    if let Some(repeat_for) = repeat_for {
        acc = acc.or(scan_quantity_expr(repeat_for));
    }
    // CR 601.2c / CR 115.1d: variable-count targeting bounds (min/max) are
    // `QuantityExpr`s that can read a projected/event resource (e.g. a die-result X).
    // MultiTargetSpec is itself destructured without `..` (same no-wildcard floor).
    if let Some(MultiTargetSpec { min, max }) = multi_target {
        acc = acc.or(scan_quantity_expr(min));
        if let Some(max) = max {
            acc = acc.or(scan_quantity_expr(max));
        }
    }
    // CR 115.1 / CR 601.2c: cross-target legality constraints; `TotalManaValue`'s
    // where-X bound carries an `EventContextAmount` (axis-1) read.
    for c in target_constraints {
        acc = acc.or(scan_target_selection_constraint(c));
    }
    // CR 605.3a / CR 608.2g: a resolution-time "unless a player pays {cost}"
    // consults floating mana (CR 106.1), a projected axis.
    if unless_pay.is_some() {
        acc.projected = true;
    }
    // CR 601.2c / CR 603.3d: `target_chooser` selects who announces targets; a
    // TargetFilter like `TriggeringSourceController` reads the triggering event.
    if let Some(chooser) = target_chooser {
        acc = acc.or(scan_target_filter(chooser));
    }
    // CR 608.2c / CR 107.1c: a "repeat this process while <condition>" predicate is
    // re-evaluated against freshly-resolved state each iteration — a resolution read.
    if let Some(repeat_until) = repeat_until {
        acc = acc.or(scan_repeat_continuation(repeat_until));
    }
    // CR 700.2: a modal header's dynamic mode cap / chooser can read dynamic state.
    if let Some(modal) = modal {
        acc = acc.or(scan_modal_choice(modal));
    }
    // CR 700.2b: reflexive-modal per-mode `AbilityDefinition`s are def-level structs
    // the walk does not descend into — conservative (fail-closed) when present.
    if !mode_abilities.is_empty() {
        acc = acc.or(Axes::CONSERVATIVE);
    }
    acc
}

/// CR 608.2c / CR 107.1c: a loop-continuation predicate. Only `WhileCondition`
/// re-reads game state (per-iteration re-evaluation); the controller-prompted and
/// boolean-stop variants read no dynamic resource.
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_repeat_continuation(r: &RepeatContinuation) -> Axes {
    match r {
        RepeatContinuation::ControllerChoice => Axes::NONE,
        RepeatContinuation::UntilStopConditions {
            stop_on_put_to_hand: _,
            stop_on_duplicate_exiled_names: _,
        } => Axes::NONE,
        RepeatContinuation::WhileCondition {
            condition,
            max_iterations: _,
        } => scan_ability_condition(condition),
    }
}

/// CR 700.2: the read-bearing payloads of a modal header. `dynamic_max_choices`
/// (a `QuantityExpr`) and `chooser` (a `PlayerFilter`) can read dynamic state; the
/// remaining fields are cast/announce-time metadata (concrete counts, costs, and
/// static cast-time predicates) that do not express a resolution-time dynamic read.
/// Destructured without `..` — a future `ModalChoice` field must be classified here.
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_modal_choice(m: &ModalChoice) -> Axes {
    let ModalChoice {
        dynamic_max_choices,
        chooser,
        min_choices: _,
        max_choices: _,
        mode_count: _,
        mode_descriptions: _,
        allow_repeat_modes: _,
        constraints: _, // cast-time modal-cap predicates (announcement-time, not resolution)
        mode_costs: _,
        mode_pawprints: _,
        entwine_cost: _,
        selection: _,
    } = m;
    let mut acc = scan_player_filter(chooser);
    if let Some(qty) = dynamic_max_choices {
        acc = acc.or(scan_quantity_expr(qty));
    }
    acc
}

/// CR 115.1 / CR 601.2c: cross-target legality constraints. Only `TotalManaValue`
/// carries a read — its `value` is a `QuantityExpr` documented to hold the where-X
/// `EventContextAmount` (axis 1); the `Different*` variants are pure structural
/// predicates over the chosen set with no dynamic read.
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_target_selection_constraint(c: &TargetSelectionConstraint) -> Axes {
    match c {
        TargetSelectionConstraint::DifferentTargetPlayers => Axes::NONE,
        TargetSelectionConstraint::DifferentObjectControllers => Axes::NONE,
        TargetSelectionConstraint::TotalManaValue {
            value,
            comparator: _,
        } => scan_quantity_expr(value),
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_effect(x: &Effect) -> Axes {
    match x {
        Effect::StartYourEngines { player_scope, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_filter(player_scope));
            acc
        }
        Effect::ChangeSpeed {
            player_scope,
            amount,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_filter(player_scope));
            acc = acc.or(scan_quantity_expr(amount));
            acc
        }
        Effect::DealDamage { amount, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ApplyPostReplacementDamage { .. } => Axes::NONE,
        Effect::EachDealsDamageEqualToPower {
            sources, recipient, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(sources));
            acc = acc.or(scan_target_filter(recipient));
            acc
        }
        Effect::Draw { count, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Pump { .. } => Axes::CONSERVATIVE,
        Effect::PairWith { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Destroy { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Regenerate { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::RemoveAllDamage { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Counter { .. } => Axes::CONSERVATIVE,
        Effect::CounterAll { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Token { .. } => Axes::CONSERVATIVE,
        Effect::GainLife { amount, player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            acc = acc.or(scan_target_filter(player));
            acc
        }
        Effect::LoseLife { amount, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            if let Some(x) = target {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        Effect::SetTapState { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::RemoveCounter { count, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Sacrifice { target, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::DiscardCard { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Mill { count, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Scry { count, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::PumpAll { .. } => Axes::CONSERVATIVE,
        Effect::DamageAll {
            amount,
            target,
            player_filter,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            acc = acc.or(scan_target_filter(target));
            if let Some(x) = player_filter {
                acc = acc.or(scan_player_filter(x));
            }
            acc
        }
        Effect::DamageEachPlayer {
            amount,
            player_filter,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            acc = acc.or(scan_player_filter(player_filter));
            acc
        }
        Effect::DestroyAll { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ChangeZone { .. } => Axes::CONSERVATIVE,
        Effect::ChangeZoneAll { .. } => Axes::CONSERVATIVE,
        Effect::Dig {
            player,
            count,
            filter,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(player));
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        Effect::GainControl { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::GainControlAll { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ControlNextTurn { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Attach {
            attachment, target, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(attachment));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::UnattachAll {
            attachment, target, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(attachment));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Surveil { count, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Fight {
            target, subject, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_target_filter(subject));
            acc
        }
        Effect::Bounce { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::BounceAll { target, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            if let Some(x) = count {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc
        }
        Effect::Explore => Axes::NONE,
        Effect::ExploreAll { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        Effect::Investigate => Axes::NONE,
        Effect::Tribute { .. } => Axes::NONE,
        Effect::TimeTravel => Axes::NONE,
        Effect::BecomeMonarch => Axes::NONE,
        Effect::NoOp => Axes::NONE,
        Effect::Proliferate => Axes::NONE,
        Effect::ProliferateTarget { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Populate => Axes::NONE,
        Effect::Clash => Axes::NONE,
        Effect::EndTheTurn => Axes::NONE,
        Effect::EndCombatPhase => Axes::NONE,
        Effect::Vote { .. } => Axes::CONSERVATIVE,
        Effect::SeparateIntoPiles { .. } => Axes::CONSERVATIVE,
        Effect::SwitchPT { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::CopySpell { .. } => Axes::CONSERVATIVE,
        Effect::EpicCopy { .. } => Axes::CONSERVATIVE,
        Effect::CastCopyOfCard { target, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            if let Some(x) = count {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc
        }
        Effect::CopyTokenOf { .. } => Axes::CONSERVATIVE,
        Effect::CreateTokenCopyFromPool {
            owner,
            type_filter,
            mv_bound,
            count,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(owner));
            acc = acc.or(scan_target_filter(type_filter));
            acc = acc.or(scan_quantity_expr(mv_bound));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::Myriad => Axes::NONE,
        Effect::Encore => Axes::NONE,
        Effect::CombineHost { host, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(host));
            acc
        }
        Effect::ChooseAugmentAndCombineWithHost { filter, host, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc = acc.or(scan_target_filter(host));
            acc
        }
        Effect::Meld { .. } => Axes::NONE,
        Effect::ExileHaunting { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::HideawayConceal { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::CopyTokenBlockingAttacker {
            source_filter,
            owner,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(source_filter));
            acc = acc.or(scan_target_filter(owner));
            acc
        }
        Effect::BecomeCopy { .. } => Axes::CONSERVATIVE,
        Effect::GainActivatedAbilitiesOfTarget {
            target,
            recipient,
            duration,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_target_filter(recipient));
            if let Some(x) = duration {
                acc = acc.or(scan_duration(x));
            }
            acc
        }
        Effect::ChooseCard { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::PutCounter { count, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::PutCounterAll { count, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::MultiplyCounter { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::DoublePT { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::DoublePTAll { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::MoveCounters {
            source,
            count,
            target,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(source));
            if let Some(x) = count {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Animate { .. } => Axes::CONSERVATIVE,
        Effect::ReturnAsAura { .. } => Axes::CONSERVATIVE,
        Effect::RegisterBending { .. } => Axes::NONE,
        Effect::GenericEffect { .. } => Axes::CONSERVATIVE,
        Effect::Cleanup { .. } => Axes::NONE,
        Effect::Mana { .. } => Axes::CONSERVATIVE,
        Effect::Discard {
            count,
            target,
            unless_filter,
            filter,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            if let Some(x) = unless_filter {
                acc = acc.or(scan_target_filter(x));
            }
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        Effect::Shuffle { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Transform { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::SearchLibrary { .. } => Axes::CONSERVATIVE,
        Effect::SearchOutsideGame { filter, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::RevealHand {
            target,
            card_filter,
            count,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_target_filter(card_filter));
            if let Some(x) = count {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc
        }
        Effect::RevealFromHand { .. } => Axes::CONSERVATIVE,
        Effect::Reveal { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::RevealTop { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(player));
            acc
        }
        Effect::ExileTop { player, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(player));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::TargetOnly { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Choose { .. } => Axes::CONSERVATIVE,
        Effect::ChooseDamageSource { source_filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(source_filter));
            acc
        }
        Effect::Suspect { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Unsuspect { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Connive { target, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::PhaseOut { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::PhaseIn { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ForceBlock { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ForceAttack {
            target,
            required_player,
            duration,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_target_filter(required_player));
            acc = acc.or(scan_duration(duration));
            acc
        }
        Effect::SolveCase => Axes::NONE,
        Effect::BecomePrepared { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::BecomeUnprepared { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::BecomeSaddled { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::SetClassLevel { .. } => Axes::NONE,
        Effect::CreateDelayedTrigger { .. } => Axes::CONSERVATIVE,
        Effect::AddTargetReplacement { .. } => Axes::CONSERVATIVE,
        Effect::AddRestriction { .. } => Axes::CONSERVATIVE,
        Effect::ReduceNextSpellCost { spell_filter, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = spell_filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        Effect::GrantNextSpellAbility {
            player,
            spell_filter,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            if let Some(x) = spell_filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        Effect::AddPendingETBCounters { count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::CreateEmblem { .. } => Axes::CONSERVATIVE,
        Effect::PayCost { .. } => Axes::CONSERVATIVE,
        Effect::CastFromZone { .. } => Axes::CONSERVATIVE,
        Effect::FreeCastFromZones { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        Effect::ExileResolvingSpellInsteadOfGraveyard => Axes::NONE,
        Effect::PreventDamage {
            amount_dynamic,
            target,
            damage_source_filter,
            prevention_duration,
            ..
        } => {
            let mut acc = Axes::NONE;
            if let Some(x) = amount_dynamic {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc = acc.or(scan_target_filter(target));
            if let Some(x) = damage_source_filter {
                acc = acc.or(scan_target_filter(x));
            }
            if let Some(x) = prevention_duration {
                acc = acc.or(scan_duration(x));
            }
            acc
        }
        Effect::CreateDamageReplacement { .. } => Axes::CONSERVATIVE,
        Effect::CreateDrawReplacement {
            replacement_effect, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_effect(replacement_effect));
            acc
        }
        Effect::LoseTheGame { target, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = target {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        Effect::WinTheGame { target, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = target {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        Effect::RollDie { .. } => Axes::CONSERVATIVE,
        Effect::FlipCoin { .. } => Axes::CONSERVATIVE,
        Effect::FlipCoins { .. } => Axes::CONSERVATIVE,
        Effect::FlipCoinUntilLose { .. } => Axes::CONSERVATIVE,
        Effect::RingTemptsYou => Axes::NONE,
        Effect::VentureIntoDungeon => Axes::NONE,
        Effect::VentureInto { .. } => Axes::NONE,
        Effect::TakeTheInitiative => Axes::NONE,
        Effect::Planeswalk => Axes::NONE,
        Effect::OpenAttractions { .. } => Axes::NONE,
        Effect::RollToVisitAttractions => Axes::NONE,
        Effect::AssembleContraptions { count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::AssembleContraptionsFromRollDifference => Axes::NONE,
        Effect::CrankContraptions { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ReassembleContraption { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::AssembleContraptionOnSprocket { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ReassembleContraptionOnSprocket { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::PutSticker {
            target,
            count,
            max_ticket_cost,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(count));
            if let Some(x) = max_ticket_cost {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc
        }
        Effect::ApplySticker { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ProcessRadCounters => Axes::NONE,
        Effect::GrantCastingPermission { .. } => Axes::CONSERVATIVE,
        Effect::ChooseFromZone { filter, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        Effect::RememberCard { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ForEachCategoryExile { .. } => Axes::NONE,
        Effect::ChooseObjectsIntoTrackedSet {
            chooser, filter, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(chooser));
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        Effect::ChooseAndSacrificeRest {
            choose_filter,
            sacrifice_filter,
            total_power_cap,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(choose_filter));
            acc = acc.or(scan_target_filter(sacrifice_filter));
            if let Some(x) = total_power_cap {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc
        }
        Effect::Exploit { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::GainEnergy { amount, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            acc
        }
        Effect::GivePlayerCounter { count, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::LoseAllPlayerCounters { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ExileFromTopUntil { .. } => Axes::CONSERVATIVE,
        Effect::RevealUntil {
            player,
            filter,
            count,
            enters_under,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(player));
            acc = acc.or(scan_target_filter(filter));
            acc = acc.or(scan_quantity_expr(count));
            if let Some(x) = enters_under {
                acc = acc.or(scan_controller_ref(x));
            }
            acc
        }
        Effect::Discover {
            mana_value_limit,
            player,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(mana_value_limit));
            acc = acc.or(scan_target_filter(player));
            acc
        }
        Effect::Heist { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::HeistExile => Axes::NONE,
        Effect::Cascade => Axes::NONE,
        Effect::Ripple { .. } => Axes::NONE,
        Effect::MiracleCast { .. } => Axes::NONE,
        Effect::MadnessCast { .. } => Axes::NONE,
        Effect::PutAtLibraryPosition { .. } => Axes::CONSERVATIVE,
        Effect::ChooseDrawnThisTurnPayOrTopdeck {
            count,
            life_payment,
            player,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc = acc.or(scan_quantity_expr(life_payment));
            acc = acc.or(scan_target_filter(player));
            acc
        }
        Effect::PutOnTopOrBottom { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::GiftDelivery { .. } => Axes::NONE,
        Effect::Goad { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::GoadAll { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Detain { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::SetRoomDoorLock { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ExchangeControl {
            target_a, target_b, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target_a));
            acc = acc.or(scan_target_filter(target_b));
            acc
        }
        Effect::ChangeTargets {
            target, forced_to, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            if let Some(x) = forced_to {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        Effect::Manifest {
            target,
            count,
            enters_under,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(count));
            if let Some(x) = enters_under {
                acc = acc.or(scan_controller_ref(x));
            }
            acc
        }
        Effect::ManifestDread => Axes::NONE,
        Effect::Cloak { target, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::TurnFaceUp { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::TurnFaceDown { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::ExtraTurn { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::GrantExtraLoyaltyActivations { amount, target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::SkipNextTurn { target, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::SkipNextStep { target, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::AdditionalPhase { target, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::Double { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::RuntimeHandled { .. } => Axes::NONE,
        Effect::Incubate { count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::Amass { count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::Monstrosity { count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::Specialize => Axes::NONE,
        Effect::Renown { count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::Bolster { count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::Adapt { count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::Learn => Axes::NONE,
        Effect::Forage => Axes::NONE,
        Effect::Harness => Axes::NONE,
        Effect::CollectEvidence { .. } => Axes::NONE,
        Effect::Endure {
            amount, subject, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            acc = acc.or(scan_target_filter(subject));
            acc
        }
        Effect::BlightEffect { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(player));
            acc
        }
        Effect::Seek { filter, count, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        Effect::SetLifeTotal { target, amount, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_quantity_expr(amount));
            acc
        }
        Effect::ExchangeLifeWithStat { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(player));
            acc
        }
        Effect::ExchangeLifeTotals {
            player_a, player_b, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(player_a));
            acc = acc.or(scan_target_filter(player_b));
            acc
        }
        Effect::SetDayNight { .. } => Axes::NONE,
        Effect::GiveControl {
            target, recipient, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc = acc.or(scan_target_filter(recipient));
            acc
        }
        Effect::RemoveFromCombat { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Conjure { .. } => Axes::CONSERVATIVE,
        Effect::ApplyPerpetual { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        Effect::Intensify { amount, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(amount));
            acc
        }
        Effect::DraftFromSpellbook { .. } => Axes::NONE,
        Effect::ChooseOneOf { .. } => Axes::CONSERVATIVE,
        Effect::Unimplemented { .. } => Axes::NONE,
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_quantity_ref(x: &QuantityRef) -> Axes {
    match x {
        QuantityRef::HandSize { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::LifeTotal { player, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::GraveyardSize { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::LifeAboveStarting => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        QuantityRef::StartingLifeTotal => Axes::NONE,
        QuantityRef::ObjectCount { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::ObjectCountDistinct { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::ObjectCountBySharedQuality { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::PlayerCount { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_filter(filter));
            acc
        }
        QuantityRef::CountersOn { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::CountersOnObjects { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::PlayerCounter { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_count_scope(scope));
            acc
        }
        QuantityRef::Variable { .. } => Axes::NONE,
        QuantityRef::Power { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::Intensity { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::Toughness { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::ObjectManaValue { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::TargetObjectManaValue { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::ObjectColorCount { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::ObjectNameWordCount { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::ObjectTypelineComponentCount { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::ManaSymbolsInManaCost { scope, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        QuantityRef::SelfManaValue => Axes::NONE,
        QuantityRef::Aggregate { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::ControlledByEachPlayer { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::TargetZoneCardCount { .. } => Axes::NONE,
        QuantityRef::Devotion { .. } => Axes {
            event: false,
            sibling: true,
            projected: false,
        },
        QuantityRef::DistinctCardTypes { .. } => Axes::CONSERVATIVE,
        QuantityRef::CardsExiledBySource => Axes::NONE,
        QuantityRef::ExiledCardPower { .. } => Axes::NONE,
        QuantityRef::ZoneCardCount { filter, scope, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc = acc.or(scan_count_scope(scope));
            acc
        }
        QuantityRef::BasicLandTypeCount { controller, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_controller_ref(controller));
            acc
        }
        QuantityRef::TrackedSetSize => Axes::NONE,
        QuantityRef::FilteredTrackedSetSize { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::TrackedSetAggregate { .. } => Axes::NONE,
        QuantityRef::ExiledFromHandThisResolution => Axes::NONE,
        QuantityRef::PreviousEffectAmount => Axes::NONE,
        QuantityRef::LifeLostThisTurn { player, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::PartySize { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::UnspentMana { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        QuantityRef::Speed { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::EventContextAmount => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        QuantityRef::AttachmentsOnLeavingObject { controller, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = controller {
                acc = acc.or(scan_controller_ref(x));
            }
            acc
        }
        QuantityRef::EventContextSourceCostX => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        QuantityRef::SpellsCastThisTurn { scope, filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_count_scope(scope));
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        QuantityRef::EnteredThisTurn { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: true,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::SacrificedThisTurn { player, filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::CrimesCommittedThisTurn => Axes::NONE,
        QuantityRef::LifeGainedThisTurn { player, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::CardsDrawnThisTurn { player, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::BattlefieldEntriesThisTurn { player, filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::LandsPlayedThisTurn { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::TurnsTaken => Axes::NONE,
        QuantityRef::ZoneChangeCountThisTurn { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::ZoneChangeAggregateThisTurn { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::DamageDealtThisTurn { source, target, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_target_filter(source));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        QuantityRef::ChosenNumber => Axes::NONE,
        QuantityRef::AttackedThisTurn { scope, filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_count_scope(scope));
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        QuantityRef::DescendedThisTurn => Axes::NONE,
        QuantityRef::LoyaltyAbilitiesActivatedThisTurn { player, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::SpellsCastLastTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        QuantityRef::SpellsCastThisGame { scope, filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_count_scope(scope));
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        QuantityRef::CounterAddedThisTurn { actor, target, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_count_scope(actor));
            acc = acc.or(scan_target_filter(target));
            acc
        }
        QuantityRef::CardsDiscardedThisTurn { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::TokensCreatedThisTurn { player, filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::PlayerActionsThisTurn { player, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_player_scope(player));
            acc
        }
        QuantityRef::DungeonsCompleted => Axes::NONE,
        QuantityRef::CostXPaid => Axes::NONE,
        QuantityRef::KickerCount => Axes::NONE,
        QuantityRef::AdditionalCostPaymentCount => Axes::NONE,
        QuantityRef::AdditionalCostPaymentCountFor { .. } => Axes::NONE,
        QuantityRef::ConvokedCreatureCount => Axes::NONE,
        QuantityRef::TimesCostPaidThisResolution => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        QuantityRef::ManaSpentToCast { .. } => Axes::CONSERVATIVE,
        QuantityRef::ColorsInCommandersColorIdentity => Axes::NONE,
        QuantityRef::CommanderCastFromCommandZoneCount => Axes::NONE,
        QuantityRef::CommanderManaValue { owner, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_controller_ref(owner));
            acc
        }
        QuantityRef::DistinctColorsAmongPermanents { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::DistinctCounterKindsAmong { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        QuantityRef::VoteCount { .. } => Axes::NONE,
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_quantity_expr(x: &QuantityExpr) -> Axes {
    match x {
        QuantityExpr::Ref { qty, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_ref(qty));
            acc
        }
        QuantityExpr::Fixed { .. } => Axes::NONE,
        QuantityExpr::DivideRounded { inner, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(inner));
            acc
        }
        QuantityExpr::Offset { inner, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(inner));
            acc
        }
        QuantityExpr::ClampMin { inner, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(inner));
            acc
        }
        QuantityExpr::Multiply { inner, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(inner));
            acc
        }
        QuantityExpr::Sum { exprs, .. } => {
            let mut acc = Axes::NONE;
            for x in exprs {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc
        }
        QuantityExpr::UpTo { max, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(max));
            acc
        }
        QuantityExpr::Power { exponent, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(exponent));
            acc
        }
        QuantityExpr::Difference { left, right, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(left));
            acc = acc.or(scan_quantity_expr(right));
            acc
        }
        QuantityExpr::Max { exprs, .. } => {
            let mut acc = Axes::NONE;
            for x in exprs {
                acc = acc.or(scan_quantity_expr(x));
            }
            acc
        }
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_ability_condition(x: &AbilityCondition) -> Axes {
    match x {
        AbilityCondition::AdditionalCostPaid { subject, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_object_scope(subject));
            acc
        }
        AbilityCondition::AdditionalCostPaidInstead => Axes::NONE,
        AbilityCondition::AlternativeManaCostPaid => Axes::NONE,
        AbilityCondition::EffectOutcome { .. } => Axes::NONE,
        AbilityCondition::EventOutcomeWon => Axes::NONE,
        AbilityCondition::WhenYouDo => Axes::NONE,
        AbilityCondition::CastFromZone { .. } => Axes::NONE,
        AbilityCondition::CastDuringPhase { .. } => Axes::NONE,
        AbilityCondition::CurrentPhaseIs { .. } => Axes::NONE,
        AbilityCondition::CastTimingPermission { .. } => Axes::NONE,
        AbilityCondition::ManaColorSpent { .. } => Axes::NONE,
        AbilityCondition::RevealedHasCardType { .. } => Axes::CONSERVATIVE,
        AbilityCondition::ObjectsShareQuality {
            subject, reference, ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(subject));
            acc = acc.or(scan_target_filter(reference));
            acc
        }
        AbilityCondition::TargetSharesNameWithOtherExiledThisWay { target, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(target));
            acc
        }
        AbilityCondition::SourceEnteredThisTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        AbilityCondition::CastVariantPaid { subject, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_object_scope(subject));
            acc
        }
        AbilityCondition::CastVariantPaidInstead { .. } => Axes::NONE,
        AbilityCondition::QuantityCheck { lhs, rhs, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(lhs));
            acc = acc.or(scan_quantity_expr(rhs));
            acc
        }
        AbilityCondition::PreviousEffectAmount { rhs, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(rhs));
            acc
        }
        AbilityCondition::HasMaxSpeed => Axes::NONE,
        AbilityCondition::IsMonarch => Axes::NONE,
        AbilityCondition::IsInitiative => Axes::NONE,
        AbilityCondition::HasCityBlessing => Axes::NONE,
        AbilityCondition::IsRingBearer => Axes::NONE,
        AbilityCondition::TargetHasKeywordInstead { .. } => Axes::NONE,
        AbilityCondition::TargetMatchesFilter { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        AbilityCondition::HasObjectTarget => Axes::NONE,
        AbilityCondition::TriggeringSpellTargetsFilter { filter, .. } => {
            let mut acc = Axes {
                event: true,
                sibling: false,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        AbilityCondition::SourceMatchesFilter { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        AbilityCondition::ZoneChangeObjectMatchesFilter { filter, .. } => {
            let mut acc = Axes {
                event: true,
                sibling: false,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        AbilityCondition::ControllerControlsMatching { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        AbilityCondition::ControllerControlledMatchingAsCast { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        AbilityCondition::IsYourTurn => Axes::NONE,
        AbilityCondition::WasStartingPlayer { controller, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_controller_ref(controller));
            acc
        }
        AbilityCondition::SpellCastWithVariantThisTurn { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        AbilityCondition::FirstCombatPhaseOfTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        AbilityCondition::FirstEndStepOfTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        AbilityCondition::ZoneChangedThisWay { filter, .. } => {
            let mut acc = Axes {
                event: true,
                sibling: false,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        AbilityCondition::CostPaidObjectMatchesFilter { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        AbilityCondition::SourceIsTapped => Axes::NONE,
        AbilityCondition::SourceAttachedToCreature => Axes::NONE,
        AbilityCondition::ConditionInstead { inner, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_ability_condition(inner));
            acc
        }
        AbilityCondition::And { conditions, .. } => {
            let mut acc = Axes::NONE;
            for x in conditions {
                acc = acc.or(scan_ability_condition(x));
            }
            acc
        }
        AbilityCondition::Or { conditions, .. } => {
            let mut acc = Axes::NONE;
            for x in conditions {
                acc = acc.or(scan_ability_condition(x));
            }
            acc
        }
        AbilityCondition::Not { condition, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_ability_condition(condition));
            acc
        }
        AbilityCondition::DayNightIsNeither => Axes::NONE,
        AbilityCondition::DayNightIs { .. } => Axes::NONE,
        AbilityCondition::NthResolutionThisTurn { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        AbilityCondition::SourceLacksKeyword { .. } => Axes::NONE,
        AbilityCondition::ScopedPlayerMatches { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_filter(filter));
            acc
        }
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_target_filter(x: &TargetFilter) -> Axes {
    match x {
        TargetFilter::None => Axes::NONE,
        TargetFilter::Any => Axes::NONE,
        TargetFilter::Player => Axes::NONE,
        TargetFilter::Controller => Axes::NONE,
        TargetFilter::SelfRef => Axes::NONE,
        TargetFilter::SourceOrPaired => Axes::NONE,
        TargetFilter::Typed(..) => Axes::CONSERVATIVE,
        TargetFilter::Not { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TargetFilter::Or { filters, .. } => {
            let mut acc = Axes::NONE;
            for x in filters {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        TargetFilter::And { filters, .. } => {
            let mut acc = Axes::NONE;
            for x in filters {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        TargetFilter::StackAbility { controller, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = controller {
                acc = acc.or(scan_controller_ref(x));
            }
            acc
        }
        TargetFilter::StackSpell => Axes::NONE,
        TargetFilter::SpecificObject { .. } => Axes::NONE,
        TargetFilter::SpecificPlayer { .. } => Axes::NONE,
        TargetFilter::Neighbor { .. } => Axes::NONE,
        TargetFilter::ScopedPlayer => Axes::NONE,
        TargetFilter::AttachedTo => Axes::NONE,
        TargetFilter::LastCreated => Axes::NONE,
        TargetFilter::LastRevealed => Axes::NONE,
        TargetFilter::CostPaidObject => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::ChosenCard => Axes::NONE,
        TargetFilter::TrackedSet { .. } => Axes::NONE,
        TargetFilter::TrackedSetFiltered { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TargetFilter::ExiledBySource => Axes::NONE,
        TargetFilter::ExiledCardByIndex { .. } => Axes::NONE,
        TargetFilter::TriggeringSpellController => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::TriggeringSpellOwner => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::TriggeringPlayer => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::TriggeringSource => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::EventTarget => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::TriggeringSourceController => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::ParentTarget => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::ParentTargetSlot { .. } => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::ParentTargetController => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::ParentTargetOwner => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::SourceChosenPlayer => Axes::NONE,
        TargetFilter::OriginalController => Axes::NONE,
        TargetFilter::PostReplacementSourceController => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::PostReplacementDamageTarget => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::PostReplacementDamageTargetOwner => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::DefendingPlayer => Axes::NONE,
        TargetFilter::HasChosenName => Axes::NONE,
        TargetFilter::ChosenDamageSource => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TargetFilter::Named { .. } => Axes::NONE,
        TargetFilter::Owner => Axes::NONE,
        TargetFilter::AllPlayers => Axes::NONE,
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_object_scope(x: &ObjectScope) -> Axes {
    match x {
        ObjectScope::Source => Axes::NONE,
        ObjectScope::Target => Axes::NONE,
        ObjectScope::Recipient => Axes::NONE,
        ObjectScope::EventSource => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        ObjectScope::CostPaidObject => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        ObjectScope::Anaphoric => Axes::NONE,
        ObjectScope::Demonstrative => Axes::NONE,
        ObjectScope::EventTarget => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_trigger_condition(x: &TriggerCondition) -> Axes {
    match x {
        TriggerCondition::GainedLife { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::LostLife => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::Descended => Axes::NONE,
        TriggerCondition::ControlsType { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TriggerCondition::NoSpellsCastLastTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::TwoOrMoreSpellsCastLastTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::DuringPlayersTurn { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_filter(player));
            acc
        }
        TriggerCondition::SourceEnteredThisTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::EchoDue => Axes::NONE,
        TriggerCondition::MinCoAttackers { filter, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        TriggerCondition::SolveConditionMet => Axes::NONE,
        TriggerCondition::ClassLevelGE { .. } => Axes::NONE,
        TriggerCondition::SourceIsHarnessed => Axes::NONE,
        TriggerCondition::AttractionVisitRoll { .. } => Axes::NONE,
        TriggerCondition::WasCast {
            controller, owner, ..
        } => {
            let mut acc = Axes::NONE;
            if let Some(x) = controller {
                acc = acc.or(scan_controller_ref(x));
            }
            if let Some(x) = owner {
                acc = acc.or(scan_controller_ref(x));
            }
            acc
        }
        TriggerCondition::WasPlayed => Axes::NONE,
        TriggerCondition::AdditionalCostPaid { .. } => Axes::NONE,
        TriggerCondition::SourceIsAttacking => Axes::NONE,
        TriggerCondition::CastVariantPaid { .. } => Axes::NONE,
        TriggerCondition::CastVariantPaidPersistent { .. } => Axes::NONE,
        TriggerCondition::ActivatedAbilityIsNonMana => Axes::NONE,
        TriggerCondition::DealtDamageBySourceThisTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::DealtDamageThisTurnBySource { source, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_target_filter(source));
            acc
        }
        TriggerCondition::FirstTimeObjectTappedThisTurn => Axes::NONE,
        TriggerCondition::WasType { .. } => Axes::NONE,
        TriggerCondition::LifeTotalGE { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::ControlCount { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TriggerCondition::ControlsNone { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TriggerCondition::AttackedThisTurn => Axes::NONE,
        TriggerCondition::FirstCombatPhaseOfTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::CastSpellThisTurn { filter, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        TriggerCondition::QuantityComparison { lhs, rhs, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(lhs));
            acc = acc.or(scan_quantity_expr(rhs));
            acc
        }
        TriggerCondition::HasMaxSpeed => Axes::NONE,
        TriggerCondition::IsMonarch => Axes::NONE,
        TriggerCondition::IsInitiative => Axes::NONE,
        TriggerCondition::NoMonarch => Axes::NONE,
        TriggerCondition::WasStartingPlayer { controller, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_controller_ref(controller));
            acc
        }
        TriggerCondition::SpellCastWithVariantThisTurn { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::HasCityBlessing => Axes::NONE,
        TriggerCondition::CompletedDungeon { .. } => Axes::NONE,
        TriggerCondition::SourceIsTapped => Axes::NONE,
        TriggerCondition::SourceIsTransformed => Axes::NONE,
        TriggerCondition::SourceIsFaceUp => Axes::NONE,
        TriggerCondition::SourceIsFaceDown => Axes::NONE,
        TriggerCondition::SourceInZone { .. } => Axes::NONE,
        TriggerCondition::CounterAddedThisTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::LostLifeLastTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::DefendingPlayerControlsNone { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TriggerCondition::TributeNotPaid => Axes::NONE,
        TriggerCondition::CastDuringPhase { .. } => Axes::NONE,
        TriggerCondition::CastTimingPermission { .. } => Axes::NONE,
        TriggerCondition::ManaColorSpent { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::ManaSpentCondition { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        TriggerCondition::HadCounters { .. } => Axes {
            event: false,
            sibling: true,
            projected: false,
        },
        TriggerCondition::ControlsCommander { .. } => Axes::NONE,
        TriggerCondition::IsRenowned { .. } => Axes::NONE,
        TriggerCondition::HasCounters { .. } => Axes {
            event: false,
            sibling: true,
            projected: false,
        },
        TriggerCondition::ZoneChangeObjectMatchesFilter { filter, .. } => {
            let mut acc = Axes {
                event: true,
                sibling: false,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TriggerCondition::ZoneChangeObjectIsTapped => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TriggerCondition::SourceMatchesFilter { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TriggerCondition::EventDamageSourceMatchesFilter { filter, .. } => {
            let mut acc = Axes {
                event: true,
                sibling: false,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TriggerCondition::DamagedPlayerIsEventSourceOwner => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        TriggerCondition::ChosenLabelIs { .. } => Axes::NONE,
        TriggerCondition::AttackersDeclaredCount { .. } => Axes::CONSERVATIVE,
        TriggerCondition::ExceptFirstDrawInDrawStep => Axes::NONE,
        TriggerCondition::PlacedByAbilitySource => Axes::NONE,
        TriggerCondition::TriggeringSpellTargetsFilter { filter, .. } => {
            let mut acc = Axes {
                event: true,
                sibling: false,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        TriggerCondition::And { conditions, .. } => {
            let mut acc = Axes::NONE;
            for x in conditions {
                acc = acc.or(scan_trigger_condition(x));
            }
            acc
        }
        TriggerCondition::Or { conditions, .. } => {
            let mut acc = Axes::NONE;
            for x in conditions {
                acc = acc.or(scan_trigger_condition(x));
            }
            acc
        }
        TriggerCondition::Not { condition, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_trigger_condition(condition));
            acc
        }
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_duration(x: &Duration) -> Axes {
    match x {
        Duration::UntilEndOfTurn => Axes::NONE,
        Duration::UntilEndOfCombat => Axes::NONE,
        Duration::UntilNextTurnOf { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        Duration::UntilEndOfNextTurnOf { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        Duration::UntilHostLeavesPlay => Axes::NONE,
        Duration::UntilNextStepOf { player, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_scope(player));
            acc
        }
        Duration::ForAsLongAs { condition, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_static_condition(condition));
            acc
        }
        Duration::Permanent => Axes::NONE,
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_static_condition(x: &StaticCondition) -> Axes {
    match x {
        StaticCondition::DevotionGE { .. } => Axes {
            event: false,
            sibling: true,
            projected: false,
        },
        StaticCondition::IsPresent { filter, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = filter {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        StaticCondition::ChosenColorIs { .. } => Axes::NONE,
        StaticCondition::ChosenLabelIs { .. } => Axes::NONE,
        StaticCondition::QuantityComparison { lhs, rhs, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(lhs));
            acc = acc.or(scan_quantity_expr(rhs));
            acc
        }
        StaticCondition::HasMaxSpeed => Axes::NONE,
        StaticCondition::SpeedGE { .. } => Axes::NONE,
        StaticCondition::And { conditions, .. } => {
            let mut acc = Axes::NONE;
            for x in conditions {
                acc = acc.or(scan_static_condition(x));
            }
            acc
        }
        StaticCondition::Or { conditions, .. } => {
            let mut acc = Axes::NONE;
            for x in conditions {
                acc = acc.or(scan_static_condition(x));
            }
            acc
        }
        StaticCondition::Not { condition, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_static_condition(condition));
            acc
        }
        StaticCondition::DayNightIs { .. } => Axes::NONE,
        StaticCondition::HasCounters { .. } => Axes {
            event: false,
            sibling: true,
            projected: false,
        },
        StaticCondition::CastVariantPaid { .. } => Axes::NONE,
        StaticCondition::RecipientHasCounters { .. } => Axes {
            event: false,
            sibling: true,
            projected: false,
        },
        StaticCondition::ClassLevelGE { .. } => Axes::NONE,
        StaticCondition::DefendingPlayerControls { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        StaticCondition::SourceAttackingAlone => Axes::NONE,
        StaticCondition::SourceIsAttacking => Axes::NONE,
        StaticCondition::SourceIsBlocking => Axes::NONE,
        StaticCondition::SourceIsBlocked => Axes::NONE,
        StaticCondition::IsMonarch => Axes::NONE,
        StaticCondition::IsInitiative => Axes::NONE,
        StaticCondition::NoMonarch => Axes::NONE,
        StaticCondition::HasCityBlessing => Axes::NONE,
        StaticCondition::CompletedADungeon => Axes::NONE,
        StaticCondition::WasStartingPlayer { controller, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_controller_ref(controller));
            acc
        }
        StaticCondition::SpellCastWithVariantThisTurn { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        StaticCondition::OpponentPoisonAtLeast { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        StaticCondition::UnlessPay { .. } => Axes::CONSERVATIVE,
        StaticCondition::Unrecognized { .. } => Axes::NONE,
        StaticCondition::DuringYourTurn => Axes::NONE,
        StaticCondition::SharesColorWithMostCommonColorAmongPermanents => Axes::NONE,
        StaticCondition::SourceEnteredThisTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        StaticCondition::SourceHasDealtDamage => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        StaticCondition::WasCast { .. } => Axes::NONE,
        StaticCondition::IsRingBearer => Axes::NONE,
        StaticCondition::RingLevelAtLeast { .. } => Axes::NONE,
        StaticCondition::ControlsCommander { .. } => Axes::NONE,
        StaticCondition::SourceIsTapped => Axes::NONE,
        StaticCondition::IsTapped { scope, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_object_scope(scope));
            acc
        }
        StaticCondition::SourceIsSaddled => Axes::NONE,
        StaticCondition::SourceControllerEquals { .. } => Axes::NONE,
        StaticCondition::SourceIsEquipped => Axes::NONE,
        StaticCondition::SourceIsEnchanted => Axes::NONE,
        StaticCondition::SourceIsMonstrous => Axes::NONE,
        StaticCondition::SourceIsHarnessed => Axes::NONE,
        StaticCondition::SourceAttachedToCreature => Axes::NONE,
        StaticCondition::SourceMatchesFilter { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        StaticCondition::RecipientMatchesFilter { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        StaticCondition::RecipientAttackingOwnerTarget { .. } => Axes::NONE,
        StaticCondition::SourceIsPaired => Axes::NONE,
        StaticCondition::SourceInZone { .. } => Axes::NONE,
        StaticCondition::EnchantedIsFaceDown => Axes::NONE,
        StaticCondition::AdditionalCostPaid => Axes::NONE,
        StaticCondition::CastingAsVariant { .. } => Axes::NONE,
        StaticCondition::None => Axes::NONE,
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_player_filter(x: &PlayerFilter) -> Axes {
    match x {
        PlayerFilter::Controller => Axes::NONE,
        PlayerFilter::Opponent => Axes::NONE,
        PlayerFilter::DefendingPlayer => Axes::NONE,
        PlayerFilter::OpponentLostLife => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        PlayerFilter::OpponentGainedLife => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        PlayerFilter::HasLostTheGame => Axes::NONE,
        PlayerFilter::OpponentDealtCombatDamage { source, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            if let Some(x) = source {
                acc = acc.or(scan_target_filter(x));
            }
            acc
        }
        PlayerFilter::OpponentAttacked { .. } => Axes::NONE,
        PlayerFilter::All => Axes::NONE,
        PlayerFilter::AllExcept { exclude, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_player_filter(exclude));
            acc
        }
        PlayerFilter::HighestSpeed => Axes::NONE,
        PlayerFilter::ZoneChangedThisWay => Axes::NONE,
        PlayerFilter::PerformedActionThisWay { .. } => Axes::NONE,
        PlayerFilter::OwnersOfCardsExiledBySource => Axes::NONE,
        PlayerFilter::TriggeringPlayer => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        PlayerFilter::OpponentOtherThanTriggering => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        PlayerFilter::OpponentOfTriggeringPlayer => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        PlayerFilter::OpponentOfTriggeringPlayerNotAttacked => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        PlayerFilter::VotedFor { .. } => Axes::NONE,
        PlayerFilter::ParentObjectTargetController => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        PlayerFilter::ControlsCount { filter, count, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: true,
                projected: false,
            };
            acc = acc.or(scan_target_filter(filter));
            acc = acc.or(scan_quantity_expr(count));
            acc
        }
        PlayerFilter::PlayerAttribute { attr, value, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_quantity_ref(attr));
            acc = acc.or(scan_quantity_expr(value));
            acc
        }
        PlayerFilter::ChosenPlayer { .. } => Axes::NONE,
        PlayerFilter::ParentObjectTargetOwner => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_replacement_condition(x: &ReplacementCondition) -> Axes {
    match x {
        ReplacementCondition::And { conditions, .. } => {
            let mut acc = Axes::NONE;
            for x in conditions {
                acc = acc.or(scan_replacement_condition(x));
            }
            acc
        }
        ReplacementCondition::UnlessControlsSubtype { .. } => Axes::NONE,
        ReplacementCondition::UnlessControlsOtherLeq { .. } => Axes::CONSERVATIVE,
        ReplacementCondition::UnlessControlsMatching { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        ReplacementCondition::UnlessControlsCountMatching { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        ReplacementCondition::UnlessPlayerLifeAtMost { .. } => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        ReplacementCondition::UnlessMultipleOpponents => Axes::NONE,
        ReplacementCondition::UnlessYourTurn => Axes::NONE,
        ReplacementCondition::UnlessQuantity {
            lhs,
            rhs,
            active_player_req,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(lhs));
            acc = acc.or(scan_quantity_expr(rhs));
            if let Some(x) = active_player_req {
                acc = acc.or(scan_controller_ref(x));
            }
            acc
        }
        ReplacementCondition::OnlyIfQuantity {
            lhs,
            rhs,
            active_player_req,
            ..
        } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_quantity_expr(lhs));
            acc = acc.or(scan_quantity_expr(rhs));
            if let Some(x) = active_player_req {
                acc = acc.or(scan_controller_ref(x));
            }
            acc
        }
        ReplacementCondition::HasMaxSpeed => Axes::NONE,
        ReplacementCondition::CastViaEscape => Axes::NONE,
        ReplacementCondition::CastVariantPaid { .. } => Axes::NONE,
        ReplacementCondition::CastFromZone { .. } => Axes::NONE,
        ReplacementCondition::EnteredFromZone { .. } => Axes::NONE,
        ReplacementCondition::YouAttackedThisTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        ReplacementCondition::OpponentDamagedThisTurn => Axes {
            event: false,
            sibling: false,
            projected: true,
        },
        ReplacementCondition::CastViaKicker { .. } => Axes::NONE,
        ReplacementCondition::SourceTappedState { .. } => Axes::NONE,
        ReplacementCondition::DealtDamageThisTurnBySource { source, .. } => {
            let mut acc = Axes {
                event: false,
                sibling: false,
                projected: true,
            };
            acc = acc.or(scan_target_filter(source));
            acc
        }
        ReplacementCondition::EventSourceControlledBy { controller, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_controller_ref(controller));
            acc
        }
        ReplacementCondition::EffectCausedDiscard => Axes::NONE,
        ReplacementCondition::OnlyExtraTurn => Axes::NONE,
        ReplacementCondition::TokenSubtypeMatches { .. } => Axes::NONE,
        ReplacementCondition::TokenCoreTypeMatches { .. } => Axes::NONE,
        ReplacementCondition::ExceptFirstDrawInDrawStep => Axes::NONE,
        ReplacementCondition::IfControlsMatching { filter, .. } => {
            let mut acc = Axes::NONE;
            acc = acc.or(scan_target_filter(filter));
            acc
        }
        ReplacementCondition::ClassLevelGE { .. } => Axes::NONE,
        ReplacementCondition::DuringUntapStep => Axes::NONE,
        ReplacementCondition::ControllerControlsSource { .. } => Axes::NONE,
        ReplacementCondition::Unrecognized { .. } => Axes::NONE,
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_player_scope(x: &PlayerScope) -> Axes {
    match x {
        PlayerScope::Controller => Axes::NONE,
        PlayerScope::ScopedPlayer => Axes::NONE,
        PlayerScope::Target => Axes::NONE,
        PlayerScope::Opponent { .. } => Axes::NONE,
        PlayerScope::AllPlayers { exclude, .. } => {
            let mut acc = Axes::NONE;
            if let Some(x) = exclude {
                acc = acc.or(scan_player_scope(x));
            }
            acc
        }
        PlayerScope::RecipientController => Axes::NONE,
        PlayerScope::DefendingPlayer => Axes::NONE,
        PlayerScope::ParentObjectTargetController => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        PlayerScope::SourceChosenPlayer => Axes::NONE,
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_controller_ref(x: &ControllerRef) -> Axes {
    match x {
        ControllerRef::You => Axes::NONE,
        ControllerRef::Opponent => Axes::NONE,
        ControllerRef::ScopedPlayer => Axes::NONE,
        ControllerRef::TargetPlayer => Axes::NONE,
        ControllerRef::ParentTargetController => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        ControllerRef::ParentTargetOwner => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        ControllerRef::DefendingPlayer => Axes::NONE,
        ControllerRef::ChosenPlayer { .. } => Axes::NONE,
        ControllerRef::SourceChosenPlayer => Axes::NONE,
        ControllerRef::TriggeringPlayer => Axes {
            event: true,
            sibling: false,
            projected: false,
        },
        ControllerRef::EnchantedPlayer => Axes::NONE,
    }
}

#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
fn scan_count_scope(x: &CountScope) -> Axes {
    match x {
        CountScope::Controller => Axes::NONE,
        CountScope::Owner => Axes::NONE,
        CountScope::ScopedPlayer => Axes::NONE,
        CountScope::SourceChosenPlayer => Axes::NONE,
        CountScope::All => Axes::NONE,
        CountScope::Opponents => Axes::NONE,
    }
}

// ---------------------------------------------------------------------------
// Public classification API (consumed by `game::triggers` ordering and
// `analysis::resource` coverability). Each is a thin projection of one axis.
// ---------------------------------------------------------------------------

/// Axis 3: does this resolved ability (and its chain/conditions) read a
/// projected player-level resource or journal? (`analysis::resource` item 4.)
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
pub(crate) fn ability_reads_projected_resource(ability: &ResolvedAbility) -> bool {
    resolved_ability_axes(ability).projected
}

/// Axis 1: does this resolved ability read the concrete triggering-event /
/// cost-paid-object context? (CR 603.4; `game::triggers` ordering.)
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
pub(crate) fn ability_uses_event_context(ability: &ResolvedAbility) -> bool {
    resolved_ability_axes(ability).event
}

/// Axis 2: does this resolved ability read a source/recipient or board-scoped
/// mutable aggregate a sibling copy could change? (CR 603.3b; `game::triggers`
/// C2 distinct-event auto-resolve gate — the Rubblebelt Rioters / Orcish
/// Siegemaster exclusion.)
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
pub(crate) fn ability_reads_sibling_mutable(ability: &ResolvedAbility) -> bool {
    resolved_ability_axes(ability).sibling
}

/// Axis 3 on a bare trigger fire-time `condition` (CR 603.4 intervening-if) —
/// the off-stack scan surface (`analysis::resource` item 5).
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
pub(crate) fn trigger_condition_reads_projected_resource(condition: &TriggerCondition) -> bool {
    scan_trigger_condition(condition).projected
}

/// Axis 3 on a condition-gated static's `condition` (CR 604.1/613.1) — the
/// dormant-static off-stack scan surface.
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
pub(crate) fn static_condition_reads_projected_resource(condition: &StaticCondition) -> bool {
    scan_static_condition(condition).projected
}

/// Axis 3 on a replacement effect's `condition`/body (CR 614.1) — the
/// off-stack replacement scan surface.
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
pub(crate) fn replacement_condition_reads_projected_resource(
    condition: &ReplacementCondition,
) -> bool {
    scan_replacement_condition(condition).projected
}

/// Axis 3 on a bare `AbilityCondition` (resolution-time branch selector).
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
pub(crate) fn ability_condition_reads_projected_resource(condition: &AbilityCondition) -> bool {
    scan_ability_condition(condition).projected
}

/// Axis 3 on a transient `Duration::ForAsLongAs` condition (CR 604.1) — the
/// `transient_continuous_effects` off-stack scan surface.
#[allow(dead_code)] // TODO(PR-6.5 inc2): remove — consumed once wired via analysis::resource stack_entry_reads_projected_resource / fire_time_conditions_read_projected_resource + game::triggers C0 classifier.
pub(crate) fn duration_reads_projected_resource(duration: &Duration) -> bool {
    scan_duration(duration).projected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ability::{
        AggregateFunction, CastManaObjectScope, CastManaSpentMetric, Comparator,
    };
    use crate::types::identifiers::ObjectId;
    use crate::types::player::{PlayerCounterKind, PlayerId};

    fn ability_with_amount(qty: QuantityRef) -> ResolvedAbility {
        ResolvedAbility::new(
            Effect::GainLife {
                amount: QuantityExpr::Ref { qty },
                player: TargetFilter::Controller,
            },
            Vec::new(),
            ObjectId(1),
            PlayerId(0),
        )
    }

    fn fixed_drain() -> ResolvedAbility {
        ResolvedAbility::new(
            Effect::GainLife {
                amount: QuantityExpr::Fixed { value: 1 },
                player: TargetFilter::Controller,
            },
            Vec::new(),
            ObjectId(1),
            PlayerId(0),
        )
    }

    // ---- Axis 3: projected-resource readers (must classify TRUE) ----
    #[test]
    fn projected_readers_classify_as_reading() {
        // Life axis (CR 119).
        assert!(ability_reads_projected_resource(&ability_with_amount(
            QuantityRef::LifeTotal {
                player: PlayerScope::Controller
            }
        )));
        // Player-counter axis (CR 122.1) — N1(n) walker pairing; experience has NO
        // winner-predicate firewall, so this classification is the only rejection.
        assert!(ability_reads_projected_resource(&ability_with_amount(
            QuantityRef::PlayerCounter {
                kind: PlayerCounterKind::Experience,
                scope: CountScope::Controller
            }
        )));
        // Per-turn life-gained journal.
        assert!(ability_reads_projected_resource(&ability_with_amount(
            QuantityRef::LifeGainedThisTurn {
                player: PlayerScope::Controller
            }
        )));
        // Cast journal (spells cast this turn, cleared by project_out_resources).
        assert!(ability_reads_projected_resource(&ability_with_amount(
            QuantityRef::SpellsCastThisTurn {
                scope: CountScope::Controller,
                filter: None
            }
        )));
        // Damage journal (damage dealt this turn).
        assert!(ability_reads_projected_resource(&ability_with_amount(
            QuantityRef::DamageDealtThisTurn {
                source: Box::new(TargetFilter::Any),
                target: Box::new(TargetFilter::Any),
                aggregate: AggregateFunction::Sum,
                group_by: None,
                damage_kind: crate::types::ability::DamageKindFilter::Any,
                channel: crate::types::ability::DamageChannel::Total,
            }
        )));
        // Trigger fire-time intervening-if readers.
        assert!(trigger_condition_reads_projected_resource(
            &TriggerCondition::GainedLife { minimum: 30 }
        ));
        assert!(trigger_condition_reads_projected_resource(
            &TriggerCondition::LifeTotalGE { minimum: 6 }
        ));
        // Ability-condition branch selector reading the per-ability resolution count.
        assert!(ability_condition_reads_projected_resource(
            &AbilityCondition::NthResolutionThisTurn { n: 10 }
        ));
        // Static-condition dormant reader (poison).
        assert!(static_condition_reads_projected_resource(
            &StaticCondition::OpponentPoisonAtLeast { count: 1 }
        ));
        // Replacement-condition dormant reader (life).
        assert!(replacement_condition_reads_projected_resource(
            &ReplacementCondition::UnlessPlayerLifeAtMost { amount: 5 }
        ));
        // Transient ForAsLongAs duration wrapping a life-reading static condition.
        assert!(duration_reads_projected_resource(&Duration::ForAsLongAs {
            condition: StaticCondition::OpponentPoisonAtLeast { count: 1 }
        }));
    }

    // ---- Axis 3: object/board readers are NON-reading (R5-B1 negative) ----
    #[test]
    fn object_and_board_readers_are_not_projected() {
        // Object counter / P/T reads are strict-compared by gate (1), not projected.
        for qty in [
            QuantityRef::Power {
                scope: ObjectScope::Source,
            },
            QuantityRef::CountersOn {
                scope: ObjectScope::Source,
                counter_type: None,
            },
            QuantityRef::ObjectCount {
                filter: TargetFilter::Any,
            },
        ] {
            assert!(!ability_reads_projected_resource(&ability_with_amount(qty)));
        }
        // Structural conditions do not read a projected axis.
        assert!(!trigger_condition_reads_projected_resource(
            &TriggerCondition::SourceIsTapped
        ));
        assert!(!static_condition_reads_projected_resource(
            &StaticCondition::SourceIsTapped
        ));
        assert!(!ability_condition_reads_projected_resource(
            &AbilityCondition::IsYourTurn
        ));
        assert!(!replacement_condition_reads_projected_resource(
            &ReplacementCondition::CastFromZone {
                zone: crate::types::zones::Zone::Graveyard
            }
        ));
        assert!(!duration_reads_projected_resource(
            &Duration::UntilEndOfTurn
        ));
        // The plain fixed drain reads nothing on any axis.
        assert!(!ability_reads_projected_resource(&fixed_drain()));
    }

    // ---- Axis 1: event-context ----
    #[test]
    fn event_context_axis_discriminates() {
        // "gain THAT MUCH life" reads the triggering event amount.
        assert!(ability_uses_event_context(&ability_with_amount(
            QuantityRef::EventContextAmount
        )));
        // Fixed drain does not.
        assert!(!ability_uses_event_context(&fixed_drain()));

        // Each of the 5 event-context escapees, reached through a carrier the walk
        // actually traverses, must classify event == true.
        // (1) ObjectScope::EventSource via QuantityRef::Power.
        assert!(ability_uses_event_context(&ability_with_amount(
            QuantityRef::Power {
                scope: ObjectScope::EventSource,
            }
        )));
        // (2) TargetFilter::TriggeringSourceController via QuantityRef::ObjectCount filter.
        assert!(ability_uses_event_context(&ability_with_amount(
            QuantityRef::ObjectCount {
                filter: TargetFilter::TriggeringSourceController,
            }
        )));
        // (3) TargetFilter::ParentTargetSlot via QuantityRef::ObjectCount filter.
        assert!(ability_uses_event_context(&ability_with_amount(
            QuantityRef::ObjectCount {
                filter: TargetFilter::ParentTargetSlot { index: 0 },
            }
        )));
        // (4) QuantityRef::TimesCostPaidThisResolution directly.
        assert!(ability_uses_event_context(&ability_with_amount(
            QuantityRef::TimesCostPaidThisResolution
        )));
        // (5) CastManaObjectScope::TriggeringSpell via QuantityRef::ManaSpentToCast,
        //     whose whole arm is Axes::CONSERVATIVE (fail-closed ⇒ event == true).
        assert!(ability_uses_event_context(&ability_with_amount(
            QuantityRef::ManaSpentToCast {
                scope: CastManaObjectScope::TriggeringSpell,
                metric: CastManaSpentMetric::Total,
            }
        )));

        // Cross-axis negative: a purely projected-resource reader (life, CR 119)
        // does NOT read event context — the axes are independent.
        assert!(!ability_uses_event_context(&ability_with_amount(
            QuantityRef::LifeTotal {
                player: PlayerScope::Controller,
            }
        )));
    }

    // ---- BLOCKER 1 regression: multi_target bounds are traversed ----
    #[test]
    fn multi_target_bound_event_read_classifies() {
        // Base effect reads nothing; the ONLY event read is the multi_target min.
        // Revert-fail: drop the `multi_target` traversal ⇒ this flips to inert.
        let mut a = fixed_drain();
        a.multi_target = Some(MultiTargetSpec {
            min: QuantityExpr::Ref {
                qty: QuantityRef::EventContextAmount,
            },
            max: None,
        });
        assert!(ability_uses_event_context(&a));
        // Sanity: without the multi_target it is inert (isolates the min bound).
        assert!(!ability_uses_event_context(&fixed_drain()));
    }

    // ---- BLOCKER 2 regression: target_constraints are traversed ----
    #[test]
    fn target_constraint_event_read_classifies() {
        // The ONLY read is the TotalManaValue where-X bound (EventContextAmount).
        // Revert-fail: drop the `target_constraints` traversal ⇒ this flips to inert.
        let mut a = fixed_drain();
        a.target_constraints = vec![TargetSelectionConstraint::TotalManaValue {
            comparator: Comparator::LE,
            value: QuantityExpr::Ref {
                qty: QuantityRef::EventContextAmount,
            },
        }];
        assert!(ability_uses_event_context(&a));
        // Sanity: the Different* constraints carry no read.
        let mut b = fixed_drain();
        b.target_constraints = vec![TargetSelectionConstraint::DifferentTargetPlayers];
        assert!(!ability_uses_event_context(&b));
    }

    // ---- Axis 2: sibling-mutable board read (Rubblebelt / Orcish class) ----
    #[test]
    fn sibling_mutable_axis_discriminates() {
        // A board-count-scaled pump reads a mutable aggregate a sibling could change.
        assert!(ability_reads_sibling_mutable(&ability_with_amount(
            QuantityRef::ObjectCount {
                filter: TargetFilter::Any
            }
        )));
        // Source power (Orcish Siegemaster class) is a sibling-mutable read.
        assert!(ability_reads_sibling_mutable(&ability_with_amount(
            QuantityRef::Power {
                scope: ObjectScope::Source
            }
        )));
        // Fixed drain reads no sibling-mutable state — safe to auto-resolve.
        assert!(!ability_reads_sibling_mutable(&fixed_drain()));
    }
}
