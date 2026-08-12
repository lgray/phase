//! Cross-episode CR 732.2a ranking rows that need a whole board rather than a resolver fixture.
//!
//! **Row R2-d** lives here: a ranked `AnnouncementSubject::Seat` is judged as a CR 601.2c
//! TARGET, not merely as PRESENT — and the CR 115.10a CHOICE class keeps its existence-only
//! authority on the very same board. The resolver-level zone sampling for the same arm is
//! `analysis::decision_template`'s `r1ghi_*`; this file is the board-level statement, which is
//! where the two authorities can be contrasted on ONE state.

use engine::analysis::decision_template::{
    resolve, AnnouncementSubject, ConcreteDecision, ConcreteTarget, DecisionGroupKey, DecisionKind,
    DecisionSlot, DecisionTemplate, IterationCount, PinnedDecision, Ranking, ReplayFailure,
    ReplayMode, TargetPin, TargetSchedule,
};
use engine::game::scenario::GameScenario;
use engine::types::ability::{ControllerRef, StaticDefinition, TargetFilter, TypedFilter};
use engine::types::game_state::{GameState, LayersDirty, YieldTarget};
use engine::types::identifiers::{CardId, ObjectId};
use engine::types::player::PlayerId;
use engine::types::statics::StaticMode;
use engine::types::zones::Zone;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

/// A 3-player board carrying one P0-controlled ability source on the battlefield.
///
/// The source's ZONE is load-bearing, not scenery: the `Seat` arm asks
/// `player_is_legal_target(state, seat, src_id, src_controller)`, and both of its trailing
/// arguments come from re-binding the SLOT's source through `resolve_ability_instance`. A slot
/// whose source does not resolve fails closed before any hexproof question is asked, which would
/// make every arm below refuse for the wrong reason.
fn board_with_source() -> (GameState, ObjectId) {
    let mut state = GameScenario::new_n_player(3, 7).build().state().clone();
    // Production `zones::create_object`, never a raw `objects.insert`: a raw insert never joins
    // `state.battlefield`, so `game_functioning_statics` would not see it and the grant applied
    // in `grant_hexproof` below would silently never apply.
    let source = engine::game::zones::create_object(
        &mut state,
        CardId(950),
        P0,
        "Ranked Seat Ability Source".to_string(),
        Zone::Battlefield,
    );
    (state, source)
}

/// "You have hexproof" (the Leyline of Sanctity shape), on a permanent `player` controls.
fn grant_hexproof(state: &mut GameState, player: PlayerId) {
    let grantor = engine::game::zones::create_object(
        state,
        CardId(951),
        player,
        "You Have Hexproof Source".to_string(),
        Zone::Battlefield,
    );
    state
        .objects
        .get_mut(&grantor)
        .expect("the grantor was just created")
        .static_definitions =
        vec![
            StaticDefinition::new(StaticMode::Hexproof).affected(TargetFilter::Typed(
                TypedFilter::default().controller(ControllerRef::You),
            )),
        ]
        .into();
    // Fixture bookkeeping, not a rule: a new continuous-effect source needs a layer pass to be
    // seen, which an ETB would have requested. MEASURED on this lane:
    // `create_object` does NOT re-dirty `layers_dirty`, so on a board whose pass has already run
    // (`Clean`) a bare `flush_layers` returns immediately, `refresh_static_mode_presence` never
    // runs, and the O(1) `static_mode_presence` gate answers `false` for `Hexproof` regardless of
    // what the grantor carries. Marking `Full` is what an ETB would have requested.
    state.layers_dirty = LayersDirty::Full;
    engine::game::layers::flush_layers(state);
}

fn slot_for(source: ObjectId, state: &GameState) -> DecisionSlot {
    DecisionSlot {
        source: YieldTarget::ThisObject {
            source_id: source,
            // CR 400.7: bind the LIVE incarnation, read from state rather than hard-coded — a
            // hard-coded one would fail closed the moment the harness re-enters the object and
            // the row would refuse for a bookkeeping reason instead of the rules one.
            incarnation: Some(state.objects[&source].incarnation),
            trigger_description: None,
        },
        index: 0,
    }
}

fn one_pin_template(slot: DecisionSlot, pin: TargetPin) -> DecisionTemplate {
    let sources = vec![slot.source.clone()];
    DecisionTemplate {
        owner: P0,
        decisions: vec![PinnedDecision::Targets {
            slot,
            targets: vec![pin],
        }],
        replay: ReplayMode::Scheduled {
            count: IterationCount::Fixed(1),
        },
        key: DecisionGroupKey::from_sources(&sources, DecisionKind::LoopChoice),
    }
}

fn ranked_seat(player: PlayerId) -> TargetPin {
    TargetPin::Scheduled(TargetSchedule::Constant(Ranking::one(
        AnnouncementSubject::Seat(player),
    )))
}

/// **Row R2-d.** CR 601.2c + CR 702.11c: a ranked `Seat` is judged as a TARGET. The same seat,
/// on the same board, at the same slot, is REFUSED in the TARGET-class spelling and ADMITTED in
/// the CR 115.10a CHOICE-class one — which is the whole content of the provenance split, stated
/// as one measurement.
///
/// # The three arms, and why none of them is redundant
///
/// * **(a) paired positive** — the ranked seat resolves to `ConcreteTarget::Player(P1)` on the
///   board with NO hexproof. Without it, arm (b) is equally explained by a `Seat` arm that
///   never resolves anything (a dead accessor, a slot source that does not re-bind, a
///   fail-closed branch taken for a bookkeeping reason).
/// * **(b) the claim** — the identical template on the identical board plus one "You have
///   hexproof" permanent P1 controls is `ReplayFailure::IllegalTarget`.
/// * **(c) the CHOICE-class sibling, on the SAME hexproofed board** — a `TargetPin::Player(P1)`
///   at the same slot still resolves. This is the arm that proves (b) is AUTHORITY SELECTION
///   and not a newly-strict engine: CR 115.10a says a player who is not identified by the word
///   "target" is not a target, so applying hexproof to a merely CHOSEN seat would be an
///   over-veto that refuses legal CR 732.2a proposals. Losing arm (c) is the failure mode the
///   shipped `a_shrouded_player_pin_is_still_published_by_the_offer_builder` guards from the
///   other side.
///
/// # Discrimination
///
/// * swap `evaluate_schedule`'s `Seat` arm from `targeting::player_is_legal_target` to
///   `players::player_exists_for_choice` ⇒ the hexproofed seat resolves ⇒ **(b) FAILS** while
///   (a) and (c) stay green — the exact asymmetry that makes this row about the AUTHORITY and
///   not about the board;
/// * conversely, route `resolve_target`'s `TargetPin::Player` arm through
///   `player_is_legal_target` ⇒ **(c) FAILS** ⇒ the over-veto is caught here too.
///
/// # Reach-guards
///
/// The hexproof is asserted to bite at the TARGET seam for THIS source and controller before
/// (b) is claimed, and asserted NOT to bite for a third seat on the same board — so the
/// exclusion is the hexproof rather than a blanket refusal. CR 702.11c is opponent-scoped, so
/// the source's controller is asserted to be P1's opponent.
#[test]
fn r2d_a_ranked_seat_is_judged_as_a_target_while_the_choice_class_keeps_existence_only() {
    let (clean, source) = board_with_source();
    let slot = slot_for(source, &clean);

    // ── (a) PAIRED POSITIVE: no hexproof ⇒ the ranked seat resolves ──
    let ranked = one_pin_template(slot.clone(), ranked_seat(P1));
    assert_eq!(
        resolve(&ranked, 0, &clean),
        Ok(vec![ConcreteDecision::Targets {
            slot: slot.clone(),
            targets: vec![ConcreteTarget::Player(P1)],
        }]),
        "CR 601.2c: a ranked seat whose slot source is a live battlefield object RESOLVES — \
         without this arm the refusal below is satisfied by a `Seat` arm that never resolves \
         at all"
    );

    // ── the hostile board: "You have hexproof" on a permanent P1 controls ──
    let mut hostile = clean.clone();
    grant_hexproof(&mut hostile, P1);
    let controller = hostile.objects[&source].controller;
    assert!(
        engine::game::players::is_opponent(&hostile, P1, controller),
        "reach-guard: CR 702.11c excludes only OPPONENTS' spells and abilities, so the ability \
         source's controller {controller:?} must be P1's opponent"
    );
    assert!(
        engine::game::static_abilities::player_cannot_be_targeted_by(
            &hostile, P1, source, controller
        ),
        "reach-guard: the grant must actually bite at the TARGET seam for THIS source, else \
         arm (b) proves nothing"
    );
    assert!(
        !engine::game::static_abilities::player_cannot_be_targeted_by(
            &hostile, P2, source, controller
        ),
        "reach-guard: a third seat on the same board is still targetable, so the exclusion is \
         the hexproof and not an empty legal space"
    );

    // ── (b) THE CLAIM: the TARGET-class spelling is refused ──
    assert_eq!(
        resolve(&ranked, 0, &hostile),
        Err(ReplayFailure::IllegalTarget {
            slot: slot.clone(),
            pin: ranked_seat(P1),
        }),
        "CR 601.2c + CR 702.11c: an ANNOUNCED seat is a target, so hexproof makes it an \
         ILLEGAL one. `player_exists_for_choice` would say yes here — that is the authority \
         this spelling exists to move off"
    );

    // ── (c) THE CHOICE-CLASS SIBLING on the SAME board: existence only, still admitted ──
    let chosen = one_pin_template(slot.clone(), TargetPin::Player(P1));
    assert_eq!(
        resolve(&chosen, 0, &hostile),
        Ok(vec![ConcreteDecision::Targets {
            slot,
            targets: vec![ConcreteTarget::Player(P1)],
        }]),
        "CR 115.10a: a seat that is CHOSEN rather than targeted is not subject to CR 702.11c, \
         so the choice class still resolves on the very board the target class refuses. \
         Applying the targeting exclusions here would be the over-veto — it would refuse legal \
         CR 732.2a proposals, e.g. a CR 701.34a proliferate choice"
    );
}
