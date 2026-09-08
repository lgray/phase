//! CR 732.2b — which places a responder may name when shortening a proposed loop shortcut.
//!
//! The proposal owns the range (`ShortcutProposal::shortening_places`); `apply()` enforces it and
//! the interaction projection publishes its two ends. Every row drives real `apply()` calls on a
//! board reached through a production path — the restored four-seat drain dump, or the driven
//! two-player optional-drain rig — except the zero-count proposal, which no offer path mints and
//! which is therefore declared raw.

use engine::analysis::decision_template::IterationCount;
use engine::analysis::loop_check::ShortcutResponse;
use engine::game::engine::{apply, EngineError};
use engine::game::interaction::{
    bind_interaction_authority, derive_viewer_interaction, resolve_interaction_response,
};
use engine::game::visibility::filter_state_for_viewer;
use engine::types::actions::GameAction;
use engine::types::game_state::{GameState, WaitingFor};
use engine::types::interaction::{
    InteractionOpportunityResponse, InteractionResponse, InteractionResponseSpec,
    InteractionSessionId, InteractionShortcutReply, InteractionSubmission,
};
use engine::types::player::PlayerId;

use crate::loop_shortcut::reach_2p_optional_drain_offer;
use crate::loop_shortcut_drain_boards::weird_drain_board;

/// The seat now being polled, and the seats still queued behind it.
fn window(state: &GameState) -> (PlayerId, Vec<PlayerId>) {
    let WaitingFor::RespondToShortcut {
        player,
        remaining_players,
        ..
    } = &state.waiting_for
    else {
        panic!(
            "expected a live responder window, got {:?}",
            state.waiting_for
        );
    };
    (*player, remaining_players.clone())
}

/// The range the proposal on this window admits.
fn admitted_places(state: &GameState) -> std::ops::RangeInclusive<u32> {
    let WaitingFor::RespondToShortcut { proposal, .. } = &state.waiting_for else {
        panic!(
            "expected a live responder window, got {:?}",
            state.waiting_for
        );
    };
    proposal.shortening_places()
}

/// Submit a raw `GameAction` as the seat the window is polling — the boundary every emitter
/// crosses, and the one the browser and a restored save reach without the interaction layer.
fn respond(state: &mut GameState, response: ShortcutResponse) -> Result<(), EngineError> {
    let (player, _) = window(state);
    apply(state, player, GameAction::RespondToShortcut { response }).map(|_| ())
}

fn shorten(state: &mut GameState, at_iteration: u32) -> Result<(), EngineError> {
    respond(state, ShortcutResponse::Shorten { at_iteration })
}

/// The restored four-seat drain dump, declared at `count` and parked at its first responder.
/// Its offer is directly declarable, so no drive stands between the restore and the window.
fn weird_window(count: IterationCount) -> GameState {
    let mut state = weird_drain_board();
    let WaitingFor::LoopShortcut { proposer, .. } = state.waiting_for.clone() else {
        panic!(
            "the weird-drain fixture restores at its offer, got {:?}",
            state.waiting_for
        );
    };
    apply(
        &mut state,
        proposer,
        GameAction::DeclareShortcut {
            count: count.clone(),
            template: None,
        },
    )
    .unwrap_or_else(|error| panic!("declaring {count:?} on the drain dump: {error:?}"));
    let WaitingFor::RespondToShortcut { proposal, .. } = &state.waiting_for else {
        panic!(
            "declaring {count:?} must open a responder window, got {:?}",
            state.waiting_for
        );
    };
    assert_eq!(
        proposal.count, count,
        "the window carries the count that was declared"
    );
    state
}

/// The largest `Fixed` count this board's offer will accept, read off the offer itself so no row
/// transcribes a bound.
fn weird_offer_bound() -> u32 {
    let state = weird_drain_board();
    let WaitingFor::LoopShortcut { schema, .. } = &state.waiting_for else {
        panic!(
            "the weird-drain fixture restores at its offer, got {:?}",
            state.waiting_for
        );
    };
    schema.max_iterations
}

/// The driven two-player optional-drain rig, declared at `count` and parked at its responder.
/// This is the only reachable board whose schema admits `UntilLethal`: both committed dumps
/// refuse that declaration against their bounded offers.
fn rig_window(count: IterationCount) -> GameState {
    let (mut runner, _life, _cleric) = reach_2p_optional_drain_offer();
    runner
        .act(GameAction::DeclareShortcut {
            count: count.clone(),
            template: None,
        })
        .unwrap_or_else(|error| panic!("declaring {count:?} on the 2p rig: {error:?}"));
    let state = runner.state().clone();
    let WaitingFor::RespondToShortcut { proposal, .. } = &state.waiting_for else {
        panic!(
            "declaring {count:?} must open a responder window, got {:?}",
            state.waiting_for
        );
    };
    assert_eq!(
        proposal.count, count,
        "the window carries the count that was declared"
    );
    state
}

/// CR 732.2b: a place at or past the proposed count names no choice different from what was
/// proposed, so `apply()` refuses it — while the last place in the sequence is answered on the
/// same board, from the state the refusal left behind.
#[test]
fn a_place_at_the_proposed_count_is_refused_and_the_place_below_it_is_answered() {
    let count = weird_offer_bound();
    let mut state = weird_window(IterationCount::Fixed(count));
    let (responder, queued) = window(&state);

    let refused = shorten(&mut state, count);
    assert!(
        matches!(refused, Err(EngineError::InvalidAction(_))),
        "place {count} is outside a {count}-repetition proposal: {refused:?}"
    );
    assert_eq!(
        window(&state),
        (responder, queued),
        "a refused response leaves the poll exactly as it found it"
    );

    shorten(&mut state, count - 1).expect("the last place in the sequence is a legal answer");
    assert_eq!(
        state.waiting_for,
        WaitingFor::Priority { player: responder },
        "CR 732.2c: the shortening responder receives priority"
    );
}

/// CR 704.5a: an `UntilLethal` proposal is ended by the loss state-based action rather than by
/// a count, so it names no place past its end.
#[test]
fn every_place_is_in_range_on_an_until_lethal_proposal() {
    let mut state = rig_window(IterationCount::UntilLethal);
    let (responder, _) = window(&state);

    shorten(&mut state, u32::MAX).expect("no place is past an unbounded proposal's end");
    assert_eq!(
        state.waiting_for,
        WaitingFor::Priority { player: responder },
        "CR 732.2c: the shortening responder receives priority"
    );
}

/// The member the class must refuse. A zero-repetition proposal offers nothing to diverge from,
/// so it admits no place at all — while `Accept` still answers, which is what proves the refusal
/// is scoped to the shortening response and did not swallow the reply path.
#[test]
fn a_zero_count_proposal_admits_no_place_while_accept_still_answers() {
    let mut state = weird_window(IterationCount::Fixed(0));
    let (responder, queued) = window(&state);
    let next = *queued.first().expect("two seats are polled on this board");
    assert!(
        admitted_places(&state).is_empty(),
        "a zero-count proposal admits no place"
    );

    for place in [0u32, 1] {
        let refused = shorten(&mut state, place);
        assert!(
            matches!(refused, Err(EngineError::InvalidAction(_))),
            "place {place} on a zero-count proposal: {refused:?}"
        );
        assert_eq!(
            window(&state),
            (responder, queued.clone()),
            "the window survives the refusal of place {place}"
        );
    }

    respond(&mut state, ShortcutResponse::Accept).expect("Accept answers a zero-count proposal");
    assert_eq!(
        window(&state).0,
        next,
        "CR 732.2b: accepting advances the poll to the queued seat"
    );
    respond(&mut state, ShortcutResponse::Accept).expect("the last seat accepts");
    assert!(
        matches!(state.waiting_for, WaitingFor::Priority { .. }),
        "CR 732.2c: the last acceptance takes the shortcut and lands on a priority point, got {:?}",
        state.waiting_for
    );
}

/// Place 0 is in range on every proposal a responder can be shown, which is what every emitter
/// that names a constant place emits. `Fixed(1)` carries the boundary: there 0 is simultaneously
/// the only legal place and the last one.
#[test]
fn place_zero_is_answered_on_every_proposal_a_responder_can_be_shown() {
    let bounded = IterationCount::Fixed(weird_offer_bound());
    for mut state in [
        weird_window(bounded),
        rig_window(IterationCount::Fixed(1)),
        rig_window(IterationCount::UntilLethal),
    ] {
        let (responder, _) = window(&state);
        let places = admitted_places(&state);
        shorten(&mut state, 0)
            .unwrap_or_else(|error| panic!("place 0 against {places:?}: {error:?}"));
        assert_eq!(
            state.waiting_for,
            WaitingFor::Priority { player: responder },
            "CR 732.2c: the shortening responder receives priority"
        );
    }
}

/// The `[min, max]` the interaction layer publishes to the seat it is polling.
fn published_ends(state: &GameState) -> (u32, u32) {
    let (responder, _) = window(state);
    let filtered = filter_state_for_viewer(state, responder);
    derive_viewer_interaction(state, &filtered, responder)
        .opportunities
        .iter()
        .find_map(|opportunity| match &opportunity.response {
            InteractionOpportunityResponse::Schema {
                spec:
                    InteractionResponseSpec::ShortcutReply {
                        min_iteration,
                        max_iteration,
                        ..
                    },
                ..
            } => Some((*min_iteration, *max_iteration)),
            _ => None,
        })
        .expect("the responder window publishes a shortcut-reply opportunity")
}

/// Does the interaction layer let this seat submit this place?
fn interaction_admits(state: &GameState, at_iteration: u32) -> bool {
    let (responder, _) = window(state);
    let filtered = filter_state_for_viewer(state, responder);
    let interaction_id = derive_viewer_interaction(state, &filtered, responder)
        .opportunities
        .iter()
        .find(|opportunity| {
            matches!(
                &opportunity.response,
                InteractionOpportunityResponse::Schema {
                    spec: InteractionResponseSpec::ShortcutReply { .. },
                    ..
                }
            )
        })
        .map(|opportunity| opportunity.interaction_id.clone())
        .expect("the responder window publishes a shortcut-reply opportunity");
    resolve_interaction_response(
        state,
        responder,
        &InteractionSubmission {
            interaction_id,
            response: InteractionResponse::ShortcutReply {
                reply: InteractionShortcutReply::Shorten { at_iteration },
            },
        },
    )
    .is_ok()
}

/// A range the responder is shown is a range `apply()` honors: the projection publishes the
/// proposal's own two ends, and the two layers agree on every place around them. Reds if either
/// layer derives its ends a second time.
#[test]
fn the_published_range_and_the_reducer_agree_on_every_proposal_shape() {
    let bound = weird_offer_bound();
    let mut admitted = 0usize;
    let mut refused = 0usize;

    for mut state in [
        weird_window(IterationCount::Fixed(0)),
        weird_window(IterationCount::Fixed(bound)),
        rig_window(IterationCount::Fixed(1)),
        rig_window(IterationCount::UntilLethal),
    ] {
        bind_interaction_authority(&mut state, InteractionSessionId("shortening".to_string()))
            .expect("the responder window binds interaction authority");
        let places = admitted_places(&state);
        assert_eq!(
            published_ends(&state),
            (*places.start(), *places.end()),
            "the projection publishes the proposal's own ends for {places:?}"
        );

        let probes = [
            places.start().saturating_sub(1),
            *places.start(),
            places.end().saturating_sub(1),
            *places.end(),
            places.end().saturating_add(1),
        ];
        for place in probes {
            let mut board = state.clone();
            let honored = shorten(&mut board, place).is_ok();
            assert_eq!(
                interaction_admits(&state, place),
                honored,
                "the two layers disagree on place {place} against {places:?}"
            );
            if honored {
                admitted += 1;
            } else {
                refused += 1;
            }
        }
    }

    assert!(
        admitted > 0 && refused > 0,
        "reach-guard on the table itself: {admitted} admitted and {refused} refused, so neither a \
         dead projection nor a blanket refusal can satisfy the agreement"
    );
}

/// One proposal, two responders queued behind it: the refusal binds to the proposal, not to
/// whoever is answering, so it must leave the poll untouched — against an `Accept` on the same
/// board, which does advance it.
#[test]
fn a_refusal_does_not_advance_the_poll() {
    let count = weird_offer_bound();
    let mut refusal_board = weird_window(IterationCount::Fixed(count));
    let before = window(&refusal_board);
    assert!(
        !before.1.is_empty(),
        "reach-guard: this board queues a second responder behind the one being polled"
    );

    assert!(
        shorten(&mut refusal_board, count).is_err(),
        "place {count} is outside a {count}-repetition proposal"
    );
    assert_eq!(
        window(&refusal_board),
        before,
        "the refused response advanced neither the acting responder nor the queue"
    );

    let mut accept_board = weird_window(IterationCount::Fixed(count));
    respond(&mut accept_board, ShortcutResponse::Accept).expect("Accept answers");
    assert_ne!(
        window(&accept_board).0,
        before.0,
        "positive control: an answered response does advance the poll on this same board"
    );
}
