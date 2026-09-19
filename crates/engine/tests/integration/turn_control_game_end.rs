//! CR 104.1 + CR 723.1: a game that ends takes no further turn and no further
//! combat phase, so every player-control effect over it is over too. Driven from
//! the CR 104.4b mandatory-loop draw, the one game end that eliminates nobody
//! and therefore never reaches CR 800.4a's leave-game teardown.

use std::sync::Arc;

use engine::game::deck_loading::DeckEntry;
use engine::game::engine::apply;
use engine::game::interaction::{bind_interaction_authority, derive_viewer_interaction};
use engine::game::scenario::{GameScenario, P0, P1};
use engine::game::turn_control::{authorized_submitters, turn_decision_maker};
use engine::game::turns::advance_phase;
use engine::game::visibility::filter_state_for_viewer;
use engine::game::EngineError;
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::card::CardFace;
use engine::types::format::{FormatConfig, SideboardPolicy};
use engine::types::game_state::{CastPaymentMode, GameState, PlayerDeckPool, WaitingFor};
use engine::types::interaction::{
    InteractionOpportunityResponse, InteractionResponseSpec, InteractionSessionId,
};
use engine::types::mana::ManaCost;
use engine::types::match_config::{DeckCardCount, MatchConfig, MatchPhase, MatchType};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const P2: PlayerId = PlayerId(2);

const WORST_FEARS: &str = "You control target player during that player's next turn. Exile Worst Fears. (You see all cards that player could see and make all decisions for them.)";
const TAINTED_PACT: &str = "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.";

const MAIN_CARD: &str = "Registered Main Card";
const SIDE_CARD: &str = "Registered Sideboard Card";

fn entry(name: &str, count: u32) -> DeckEntry {
    DeckEntry {
        card: CardFace {
            name: name.to_string(),
            ..Default::default()
        },
        count,
    }
}

/// Register the same four-plus-three pool for every seat and put the game in a
/// match, so the game's end has a between-games step to route.
fn install_match(state: &mut GameState, match_type: MatchType, seats: u8) {
    state.set_match_config(MatchConfig {
        match_type,
        ..Default::default()
    });
    state.match_phase = MatchPhase::InGame;
    state.game_number = 1;
    state.deck_pools = (0..seats)
        .map(|seat| PlayerDeckPool {
            player: PlayerId(seat),
            registered_main: Arc::new(vec![entry(MAIN_CARD, 4)]),
            registered_sideboard: Arc::new(vec![entry(SIDE_CARD, 3)]),
            current_main: Arc::new(vec![entry(MAIN_CARD, 4)]),
            current_sideboard: Arc::new(vec![entry(SIDE_CARD, 3)]),
            ..Default::default()
        })
        .collect();
}

/// The whole registered pool in the main deck: legal under every sideboard
/// policy, including one that permits no sideboard at all.
fn submit_whole_pool() -> GameAction {
    GameAction::SubmitSideboard {
        main: vec![
            DeckCardCount {
                name: MAIN_CARD.to_string(),
                count: 4,
            },
            DeckCardCount {
                name: SIDE_CARD.to_string(),
                count: 3,
            },
        ],
        sideboard: vec![],
    }
}

/// `controller` casts Worst Fears at `target` (CR 723.1), the game advances into
/// the controlled turn, and the controller's draw from an empty library ends the
/// game as a CR 104.4b draw with nobody eliminated. Returns the post-draw state.
fn game_ended_by_a_loop_draw_under_control(
    format: FormatConfig,
    seats: u8,
    match_type: MatchType,
    controller: PlayerId,
    target: PlayerId,
) -> GameState {
    let mut scenario = GameScenario::new_with_format(format, seats, 42);
    scenario.at_phase(Phase::PreCombatMain);
    let worst_fears = scenario
        .add_spell_to_hand_from_oracle(controller, "Worst Fears", false, WORST_FEARS)
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let pact = scenario
        .add_spell_to_hand_from_oracle(controller, "Tainted Pact", true, TAINTED_PACT)
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner = scenario.build();
    install_match(runner.state_mut(), match_type, seats);
    {
        let state = runner.state_mut();
        state.active_player = controller;
        state.priority_player = controller;
        state.waiting_for = WaitingFor::Priority { player: controller };
    }

    let card_id = runner.state().objects[&worst_fears].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: worst_fears,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("the controller casts Worst Fears");
    for _ in 0..48 {
        match &runner.state().waiting_for {
            WaitingFor::TargetSelection { .. } => {
                runner
                    .act(GameAction::ChooseTarget {
                        target: Some(TargetRef::Player(target)),
                    })
                    .expect("Worst Fears targets the controlled player");
            }
            WaitingFor::ManaPayment { .. } => {
                runner.act(GameAction::PassPriority).expect("pay");
            }
            WaitingFor::Priority { .. } => {
                if runner.state().stack.is_empty() {
                    break;
                }
                runner.act(GameAction::PassPriority).expect("resolve");
            }
            other => panic!("unexpected cast window: {other:?}"),
        }
    }

    // CR 723.1: control activates when the affected player's turn begins. Stop at
    // a main phase, the first window of that turn a spell can be cast in.
    let mut events = Vec::new();
    for _ in 0..60 {
        advance_phase(runner.state_mut(), &mut events);
        let state = runner.state();
        if state.turn_decision_controller.is_some() && state.phase == Phase::PreCombatMain {
            break;
        }
    }
    assert_eq!(
        turn_decision_maker(runner.state()),
        controller,
        "reach guard: CR 723.1 control is active over the turn the draw ends"
    );
    assert_eq!(
        runner.state().scheduled_turn_controls.len(),
        1,
        "reach guard: the control effect is still scheduled when the game ends"
    );
    assert!(
        runner.state().players[controller.0 as usize]
            .library
            .is_empty(),
        "reach guard: the CR 104.4b loop needs a draw with no card to draw"
    );

    let outcome = runner.cast(pact).accept_optional().resolve();
    let state = outcome.state().clone();
    assert!(
        state.game_end.is_some(),
        "reach guard: the mandatory-loop draw ended the game"
    );
    assert!(
        state.eliminated_players.is_empty(),
        "reach guard: nobody was eliminated, so CR 800.4a's teardown never ran"
    );
    state
}

/// CR 104.1 + CR 723.1: no player-control effect survives the game, in any of the
/// places one is recorded. `clause` names what the calling row discriminates.
fn assert_no_control_survives(state: &GameState, clause: &str) {
    assert_eq!(
        state.turn_decision_controller, None,
        "a decision controller outlived the game (clause: {clause})"
    );
    assert_eq!(
        state.turn_decision_control_timestamp, None,
        "a control timestamp outlived the game (clause: {clause})"
    );
    assert_eq!(
        state.active_full_turn_control, None,
        "a full-turn control window outlived the game (clause: {clause})"
    );
    assert_eq!(
        state.active_combat_phase_control, None,
        "a combat-phase control window outlived the game (clause: {clause})"
    );
    assert!(
        state.scheduled_turn_controls.is_empty(),
        "a scheduled control outlived the game (clause: {clause})"
    );
}

/// The interaction projection is only populated behind a bound session, so a
/// prompt read without one publishes nothing for anybody.
fn with_interaction_session(state: &GameState) -> GameState {
    let mut bound = state.clone();
    bind_interaction_authority(
        &mut bound,
        InteractionSessionId("turn-control-game-end".to_string()),
    )
    .expect("interaction authority binds");
    bound
}

/// `(min_main_total, max_main_total, candidate count)` of a viewer's sideboard
/// opportunity, or `None` when the viewer is offered nothing to answer.
fn sideboard_partition(state: &GameState, viewer: PlayerId) -> Option<(u32, u32, usize)> {
    let filtered = filter_state_for_viewer(state, viewer);
    let view = derive_viewer_interaction(state, &filtered, viewer);
    let opportunity = view.opportunities.first()?;
    let InteractionOpportunityResponse::Schema { spec, candidates } = &opportunity.response else {
        return None;
    };
    let InteractionResponseSpec::DeckPartition {
        min_main_total,
        max_main_total,
        ..
    } = spec
    else {
        return None;
    };
    Some((*min_main_total, *max_main_total, candidates.len()))
}

fn can_submit(state: &GameState, viewer: PlayerId) -> bool {
    let filtered = filter_state_for_viewer(state, viewer);
    derive_viewer_interaction(state, &filtered, viewer).can_submit
}

#[test]
fn control_ends_when_a_loop_draw_ends_the_game() {
    let state = game_ended_by_a_loop_draw_under_control(
        FormatConfig::standard(),
        2,
        MatchType::Bo3,
        P1,
        P0,
    );

    assert_eq!(
        state.waiting_for.variant_name(),
        "BetweenGamesSideboard",
        "reach guard: the draw moved the match to its between-games step"
    );
    assert_no_control_survives(&state, "the teardown's placement at `end_game`");
}

#[test]
fn the_sideboard_prompt_routes_to_the_seat_that_owns_the_cards() {
    let state = game_ended_by_a_loop_draw_under_control(
        FormatConfig::standard(),
        2,
        MatchType::Bo3,
        P1,
        P0,
    );

    assert_eq!(
        authorized_submitters(&state),
        vec![P0],
        "CR 100.4: the sideboarding seat answers its own prompt"
    );
    let bound = with_interaction_session(&state);
    assert!(can_submit(&bound, P0));
    let (_, max_main_total, candidates) = sideboard_partition(&bound, P0)
        .expect("the owner's prompt publishes a deck partition to answer");
    assert!(max_main_total > 0);
    assert!(candidates > 0);
}

#[test]
fn the_former_controller_cannot_answer_the_owners_prompt() {
    let state = game_ended_by_a_loop_draw_under_control(
        FormatConfig::standard(),
        2,
        MatchType::Bo3,
        P1,
        P0,
    );

    let mut by_owner = state.clone();
    apply(&mut by_owner, P0, submit_whole_pool()).expect("the owner submits its own sideboard");

    let mut by_controller = state.clone();
    assert!(
        matches!(
            apply(&mut by_controller, P1, submit_whole_pool()),
            Err(EngineError::WrongPlayer)
        ),
        "the CR 723 controller has no say once the game is over"
    );

    let bound = with_interaction_session(&state);
    // Paired with the owner's populated projection: without it, an empty
    // projection for the controller would read the same whether the prompt is
    // routed correctly or published to nobody at all.
    assert!(sideboard_partition(&bound, P0).is_some());
    assert!(sideboard_partition(&bound, P1).is_none());
    assert!(!can_submit(&bound, P1));
}

#[test]
fn the_play_draw_prompt_routes_to_its_own_seat() {
    let mut state = game_ended_by_a_loop_draw_under_control(
        FormatConfig::standard(),
        2,
        MatchType::Bo3,
        P1,
        P0,
    );

    apply(&mut state, P0, submit_whole_pool()).expect("the first seat submits");
    apply(&mut state, P1, submit_whole_pool()).expect("the second seat submits");

    assert_eq!(
        state.waiting_for.variant_name(),
        "BetweenGamesChoosePlayDraw",
        "reach guard: both seats submitted, so the match moved to the CR 103.1 choice"
    );
    assert_eq!(
        authorized_submitters(&state),
        vec![P0],
        "CR 103.1: the chooser answers its own prompt"
    );
    assert_no_control_survives(
        &state,
        "the teardown's placement at `end_game`, sibling prompt",
    );
}

/// CR 100.4: sideboard restrictions are per-format, and a format that permits
/// none pins the whole pool in the main deck — degenerate bounds that must still
/// route to their seat rather than read as "nobody may act".
#[test]
fn a_format_without_a_sideboard_still_routes_to_the_owner() {
    let state = game_ended_by_a_loop_draw_under_control(
        FormatConfig {
            sideboard_policy: SideboardPolicy::Forbidden,
            ..FormatConfig::standard()
        },
        2,
        MatchType::Bo3,
        P1,
        P0,
    );

    let WaitingFor::BetweenGamesSideboard {
        max_sideboard_size, ..
    } = state.waiting_for
    else {
        panic!("reach guard: the draw opened a between-games sideboard prompt");
    };
    assert_eq!(
        max_sideboard_size,
        Some(0),
        "reach guard: the row rides on the `Forbidden` bounds arm"
    );

    assert_eq!(authorized_submitters(&state), vec![P0]);
    let bound = with_interaction_session(&state);
    let (min_main_total, max_main_total, _) = sideboard_partition(&bound, P0)
        .expect("the owner's prompt publishes a deck partition to answer");
    assert_eq!(
        min_main_total, max_main_total,
        "with no sideboard the whole pool is pinned in the main deck"
    );
    let mut submitted = state.clone();
    apply(&mut submitted, P0, submit_whole_pool()).expect("the owner submits the pinned pool");
}

/// CR 805.8 + CR 723.5: control over one seat of a shared-turn team delegates
/// every teammate's decisions, so a multi-seat match has more prompts to strand
/// than a duel does. The teardown names no seat, so each answers its own.
#[test]
fn every_seat_answers_its_own_prompt_in_a_multi_seat_match() {
    let mut state = game_ended_by_a_loop_draw_under_control(
        FormatConfig::archenemy(),
        4,
        MatchType::Bo3,
        P0,
        P2,
    );

    assert_eq!(
        state.waiting_for.variant_name(),
        "BetweenGamesSideboard",
        "reach guard: the draw moved the match to its between-games step"
    );
    assert_no_control_survives(&state, "the teardown's placement at `end_game`, four seats");
    for seat in [P0, P1, P2, PlayerId(3)] {
        assert_eq!(
            authorized_submitters(&state),
            vec![seat],
            "each seat in turn answers its own sideboard prompt"
        );
        apply(&mut state, seat, submit_whole_pool()).expect("the prompted seat submits");
    }
}

/// A best-of-one never reaches `handle_game_over_transition`'s between-games
/// path, so the teardown has to belong to the game layer: the terminal snapshot
/// carries no live control either.
#[test]
fn control_ends_when_a_bo1_game_ends_in_a_draw() {
    let state = game_ended_by_a_loop_draw_under_control(
        FormatConfig::standard(),
        2,
        MatchType::Bo1,
        P1,
        P0,
    );

    assert_eq!(
        state.match_phase,
        MatchPhase::Completed,
        "reach guard: a best-of-one draw completes the match"
    );
    assert_eq!(state.waiting_for.variant_name(), "GameOver");
    assert_no_control_survives(
        &state,
        "the teardown's placement at `end_game`, past the match",
    );
}
