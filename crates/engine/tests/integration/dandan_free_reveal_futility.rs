//! The Dandan free-reveal futility fact (CR 103.5 as modified by the Dandan
//! rule): a seat offered the free reveal is futile exactly when the redraw
//! pool, the shared library plus every hand still in the round, cannot deal
//! two lands and two nonland cards.

use engine::ai_support::legal_actions_for_viewer;
use engine::database::card_db::CardDatabase;
use engine::game::deck_loading::{
    load_and_hydrate_decks, resolve_deck_list, DeckList, DeckPayload, PlayerDeckList,
};
use engine::game::engine::{apply, start_game_with_starting_player};
use engine::game::mulligan::free_reveal_futile_for;
use engine::game::scenario::{P0, P1};
use engine::game::zones::{add_to_zone, remove_from_zone};
use engine::types::actions::{GameAction, MulliganChoice};
use engine::types::format::FormatConfig;
use engine::types::game_state::{GameState, MulliganDecisionPhase, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

use crate::support::shared_card_db;

const ISLANDS: [&str; 7] = ["Island"; 7];
const SIX_ISLANDS_ONE_OPT: [&str; 7] = [
    "Island", "Island", "Island", "Island", "Island", "Island", "Opt",
];
const FIVE_ISLANDS_TWO_OPTS: [&str; 7] = [
    "Island", "Island", "Island", "Island", "Island", "Opt", "Opt",
];
const TWO_OPTS: [(&str, usize); 2] = [("Island", 78), ("Opt", 2)];

fn payload(db: &CardDatabase, pile: &[(&str, usize)]) -> DeckPayload {
    let main_deck = pile
        .iter()
        .flat_map(|&(name, copies)| std::iter::repeat_n(name.to_string(), copies))
        .collect();
    resolve_deck_list(
        db,
        &DeckList {
            player: PlayerDeckList {
                main_deck,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}

fn dealt(db: &CardDatabase, pile: &DeckPayload, seed: u64, starting: PlayerId) -> GameState {
    let mut state = GameState::new(FormatConfig::dandan(), 2, seed);
    load_and_hydrate_decks(&mut state, pile, Some(db));
    start_game_with_starting_player(&mut state, starting);
    state
}

fn other(seat: PlayerId) -> PlayerId {
    if seat == P0 {
        P1
    } else {
        P0
    }
}

fn hand(state: &GameState, seat: PlayerId) -> Vec<ObjectId> {
    state.players[seat.0 as usize]
        .hand
        .iter()
        .copied()
        .collect()
}

/// Make the named cards `seat`'s whole hand, swapping with the shared pile.
fn set_hand(state: &mut GameState, seat: PlayerId, names: &[&str]) {
    for id in hand(state, seat) {
        remove_from_zone(state, id, Zone::Hand, seat);
        add_to_zone(state, id, Zone::Library, P0);
        state.objects.get_mut(&id).unwrap().zone = Zone::Library;
    }
    for name in names {
        let id = state
            .library_of(P0)
            .iter()
            .copied()
            .find(|id| state.objects[id].name == *name)
            .unwrap_or_else(|| panic!("{name} is in the pile"));
        remove_from_zone(state, id, Zone::Library, P0);
        add_to_zone(state, id, Zone::Hand, seat);
        let obj = state.objects.get_mut(&id).unwrap();
        obj.zone = Zone::Hand;
        obj.owner = seat;
    }
}

fn free_reveal() -> GameAction {
    GameAction::MulliganDecision {
        choice: MulliganChoice::FreeReveal,
    }
}

fn offered(state: &GameState, seat: PlayerId) -> bool {
    legal_actions_for_viewer(state, seat)
        .0
        .contains(&free_reveal())
}

fn act(state: &mut GameState, seat: PlayerId, choice: MulliganChoice) {
    let label = format!("{seat:?} {choice:?}");
    apply(state, seat, GameAction::MulliganDecision { choice })
        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
}

fn accepts_free_reveal(state: &GameState, seat: PlayerId) -> bool {
    apply(&mut state.clone(), seat, free_reveal()).is_ok()
}

/// (pending seats, declared seats) of the open round.
fn round(state: &GameState) -> (Vec<PlayerId>, Vec<PlayerId>) {
    let WaitingFor::MulliganDecision {
        pending, declared, ..
    } = &state.waiting_for
    else {
        panic!("expected MulliganDecision, got {:?}", state.waiting_for);
    };
    (
        pending.iter().map(|e| e.player).collect(),
        declared.iter().map(|d| d.player).collect(),
    )
}

/// Every orientation: (starting player, futile seat).
fn orientations() -> impl Iterator<Item = (PlayerId, PlayerId)> {
    [P0, P1]
        .into_iter()
        .flat_map(|starting| [P0, P1].map(|seat| (starting, seat)))
}

#[test]
fn a_declared_seats_hand_counts_toward_the_pool() {
    let Some(db) = shared_card_db() else { return };
    let pile = payload(db, &TWO_OPTS);
    for (starting, seat) in orientations() {
        let mut state = dealt(db, &pile, 1, starting);
        set_hand(&mut state, seat, &ISLANDS);
        set_hand(&mut state, other(seat), &SIX_ISLANDS_ONE_OPT);
        act(&mut state, other(seat), MulliganChoice::FreeReveal);

        assert_eq!(
            round(&state),
            (vec![seat], vec![other(seat)]),
            "reach: the other seat declared"
        );
        assert!(offered(&state, seat), "reach: {seat:?} is offered");
        assert!(
            !free_reveal_futile_for(&state, seat),
            "{starting:?} {seat:?}: the declared Opt returns before the deal"
        );
    }
}

#[test]
fn a_pending_seats_hand_counts_until_it_keeps() {
    let Some(db) = shared_card_db() else { return };
    let pile = payload(db, &TWO_OPTS);
    for (starting, seat) in orientations() {
        let mut state = dealt(db, &pile, 1, starting);
        set_hand(&mut state, seat, &ISLANDS);
        set_hand(&mut state, other(seat), &FIVE_ISLANDS_TWO_OPTS);
        assert!(
            round(&state).0.contains(&other(seat)),
            "reach: the other seat is pending"
        );
        assert!(offered(&state, seat), "reach: {seat:?} is offered");
        assert!(
            !free_reveal_futile_for(&state, seat),
            "{starting:?} {seat:?}: the pending Opts may still return"
        );

        act(&mut state, other(seat), MulliganChoice::Keep);
        assert_eq!(
            round(&state),
            (vec![seat], vec![]),
            "reach: the other seat kept"
        );
        assert!(
            free_reveal_futile_for(&state, seat),
            "{starting:?} {seat:?}: both Opts are kept"
        );
        assert!(offered(&state, seat), "the reveal stays offered");
        assert!(accepts_free_reveal(&state, seat), "the reveal stays legal");
    }
}

#[test]
fn the_deal_is_futile_only_when_the_whole_pile_cannot_clear() {
    let Some(db) = shared_card_db() else { return };
    for (pile, futile) in [
        (&[("Island", 80)][..], true),
        (&[("Island", 79), ("Opt", 1)][..], true),
        (&[("Island", 1), ("Opt", 79)][..], true),
        (&[("Island", 78), ("Opt", 2)][..], false),
        (&[("Island", 2), ("Opt", 78)][..], false),
    ] {
        let payload = payload(db, pile);
        for seed in 0..3 {
            let state = dealt(db, &payload, seed, P0);
            for seat in [P0, P1] {
                assert!(
                    offered(&state, seat),
                    "reach: {pile:?} seed {seed} {seat:?}"
                );
                assert_eq!(
                    free_reveal_futile_for(&state, seat),
                    futile,
                    "{pile:?} seed {seed} {seat:?}"
                );
                if futile {
                    assert!(accepts_free_reveal(&state, seat), "the reveal stays legal");
                }
            }
        }
    }
}

#[test]
fn a_seat_not_offered_the_reveal_is_never_futile() {
    let Some(db) = shared_card_db() else { return };
    let mut state = dealt(db, &payload(db, &[("Island", 80)]), 1, P0);
    assert!(
        free_reveal_futile_for(&state, P0),
        "reach: the deal is futile"
    );

    let mut standard = state.clone();
    standard.format_config = FormatConfig::standard();
    assert!(!free_reveal_futile_for(&standard, P0));

    act(&mut state, P1, MulliganChoice::Mulligan);
    act(&mut state, P0, MulliganChoice::Keep);
    let WaitingFor::MulliganDecision { pending, .. } = &state.waiting_for else {
        panic!("expected MulliganDecision, got {:?}", state.waiting_for);
    };
    assert!(
        pending.iter().any(|e| e.player == P1
            && e.mulligan_count == 1
            && matches!(e.phase, MulliganDecisionPhase::Declare)),
        "reach: P1 redrew after a regular mulligan"
    );
    assert!(!free_reveal_futile_for(&state, P1));
}

#[test]
fn the_default_list_is_never_futile_at_the_deal() {
    let Some(db) = shared_card_db() else { return };
    let mut offered_hands = 0;
    for seed in 0..60 {
        let state = dealt(db, &DeckPayload::default(), seed, P0);
        for seat in [P0, P1] {
            offered_hands += usize::from(offered(&state, seat));
            assert!(
                !free_reveal_futile_for(&state, seat),
                "seed {seed} {seat:?}"
            );
        }
    }
    assert!(offered_hands > 0, "reach: some deal offers the reveal");
}
