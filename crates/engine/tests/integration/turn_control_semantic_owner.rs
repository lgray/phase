//! CR 723: the action boundary authorizes the player allowed to submit an
//! action while keying player-scoped state to the seat whose decision it is.
//!
//! CR 723.5 sends the controlled player's choices to the controller, CR 723.3
//! leaves the decision seat its own, and CR 723.5a lets the controller spend
//! only that seat's resources. `apply_as_current` derived one player and used
//! it for both, and under control the `state.priority_player` sites wrote the
//! controller's key while their paired sites wrote the seat's. CR 723.5b
//! bounds the redirect: an action that names the submitting seat is not a
//! choice control redirects.

use engine::game::engine::{apply_as_current, apply_interaction_for_simulation};
use engine::game::mana_sources::activatable_mana_actions_for_player;
use engine::game::public_state::sync_waiting_for;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::turn_control::authorized_submitter_for_player;
use engine::game::EngineError;
use engine::types::ability::{AbilityCost, Effect, ResolvedAbility};
use engine::types::actions::{DebugAction, GameAction};
use engine::types::game_state::{AutoPassMode, TurnBoundary, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard};
use engine::types::phase::{Phase, PhaseStop, PhaseStopScope};
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);

const OPPOSITION_AGENT: &str = "Flash\n\
You control your opponents while they're searching their libraries.\n\
While an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.";
const TEST_TUTOR: &str =
    "Search your library for a card, put that card into your hand, then shuffle.";

/// Put `seat` on a priority window, optionally under `controller`'s CR 723
/// player control, re-deriving `priority_player` the way the engine does.
fn install_priority(runner: &mut GameRunner, seat: PlayerId, controller: Option<PlayerId>) {
    let state = runner.state_mut();
    state.active_player = seat;
    state.turn_decision_controller = controller;
    state.priority_passes.clear();
    sync_waiting_for(state, &WaitingFor::Priority { player: seat });
}

/// Non-vacuity guard: a row exercises the split only while the seat's decisions
/// are submittable by a different player.
fn assert_seat_submits_through(runner: &GameRunner, seat: PlayerId, submitter: PlayerId) {
    assert_eq!(
        authorized_submitter_for_player(runner.state(), seat),
        submitter,
        "the seat's authorized submitter decides whether this row discriminates"
    );
}

fn tracked_seats(runner: &GameRunner) -> Vec<PlayerId> {
    let mut seats: Vec<PlayerId> = runner
        .state()
        .lands_tapped_for_mana
        .keys()
        .copied()
        .collect();
    seats.sort_by_key(|player| player.0);
    seats
}

fn tracked_lands(runner: &GameRunner, seat: PlayerId) -> Vec<ObjectId> {
    runner
        .state()
        .lands_tapped_for_mana
        .get(&seat)
        .cloned()
        .unwrap_or_default()
}

/// Four seats, each with one Forest already booked as manually tapped for mana.
fn four_player_board_with_every_seat_tracked(controller: Option<PlayerId>) -> GameRunner {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    let lands: Vec<(PlayerId, ObjectId)> = [P0, P1, P2, P3]
        .into_iter()
        .map(|seat| (seat, scenario.add_basic_land(seat, ManaColor::Green)))
        .collect();
    let mut runner = scenario.build();
    install_priority(&mut runner, P1, controller);
    for (seat, land) in lands {
        runner
            .state_mut()
            .lands_tapped_for_mana
            .insert(seat, vec![land]);
    }
    runner
}

/// One Forest on the acting seat's battlefield, at that seat's priority window.
fn land_board(seats: u8, controller: Option<PlayerId>) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(seats, 42);
    scenario.at_phase(Phase::PreCombatMain);
    let forest = scenario.add_basic_land(P1, ManaColor::Green);
    let mut runner = scenario.build();
    install_priority(&mut runner, P1, controller);
    (runner, forest)
}

/// The tap action the engine itself enumerates for `seat` — never hand-built.
fn engine_authored_tap(runner: &GameRunner, seat: PlayerId, land: ObjectId) -> GameAction {
    activatable_mana_actions_for_player(runner.state(), seat)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                GameAction::TapLandForMana { selection } if selection.source.object_id == land
            )
        })
        .expect("the engine enumerates a land tap for the seat")
}

/// CR 723.5a: a `PassPriority` clears the manual mana tracking of the seat that
/// passed, not of the player authorized to submit that pass.
#[test]
fn a_pass_clears_the_controlled_seats_mana_tracking() {
    let mut runner = four_player_board_with_every_seat_tracked(Some(P0));
    assert_seat_submits_through(&runner, P1, P0);
    assert_eq!(tracked_seats(&runner), vec![P0, P1, P2, P3]);

    apply_as_current(runner.state_mut(), GameAction::PassPriority)
        .expect("the controller passes the controlled seat's priority");

    assert_eq!(
        tracked_seats(&runner),
        vec![P0, P2, P3],
        "only the passing seat's tracking clears"
    );
}

/// Reach guard for the row above: with no control effect the same board, the
/// same call and the same expected set must hold.
#[test]
fn a_pass_clears_its_own_seats_mana_tracking_without_control() {
    let mut runner = four_player_board_with_every_seat_tracked(None);
    assert_seat_submits_through(&runner, P1, P1);

    apply_as_current(runner.state_mut(), GameAction::PassPriority)
        .expect("the seat passes its own priority");

    assert_eq!(tracked_seats(&runner), vec![P0, P2, P3]);
}

/// CR 723.9: an effect may give a player control of themselves, and that player
/// then makes their own decisions — indistinguishable from no control at all.
#[test]
fn self_control_clears_the_same_seat_as_no_control() {
    let mut runner = four_player_board_with_every_seat_tracked(Some(P1));
    assert_seat_submits_through(&runner, P1, P1);

    apply_as_current(runner.state_mut(), GameAction::PassPriority)
        .expect("a self-controlling seat passes its own priority");

    assert_eq!(tracked_seats(&runner), vec![P0, P2, P3]);
}

/// CR 723.5 + CR 723.5a: the controller may submit the controlled seat's land
/// tap through either boundary form, and the mana books to that seat.
#[test]
fn the_controllers_land_tap_is_accepted_and_books_to_the_controlled_seat() {
    let (mut runner, forest) = land_board(2, Some(P0));
    assert_seat_submits_through(&runner, P1, P0);
    let tap = engine_authored_tap(&runner, P1, forest);
    apply_as_current(runner.state_mut(), tap).expect("the controller taps the seat's land");
    assert_eq!(tracked_lands(&runner, P1), vec![forest]);
    assert_eq!(tracked_seats(&runner), vec![P1]);

    let (mut runner, forest) = land_board(2, Some(P0));
    let tap = engine_authored_tap(&runner, P1, forest);
    apply_interaction_for_simulation(runner.state_mut(), P0, P1, tap)
        .expect("the split boundary accepts the same tap");
    assert_eq!(tracked_lands(&runner, P1), vec![forest]);
}

/// CR 723.5: while controlled, the seat itself is not an authorized submitter,
/// so its own tap must be refused.
#[test]
fn the_controlled_seat_may_not_submit_its_own_land_tap() {
    let (mut runner, forest) = land_board(2, Some(P0));
    let tap = engine_authored_tap(&runner, P1, forest);

    assert!(
        matches!(
            apply_interaction_for_simulation(runner.state_mut(), P1, P1, tap),
            Err(EngineError::WrongPlayer)
        ),
        "the controlled seat is not its own authorized submitter"
    );
    assert!(runner.state().lands_tapped_for_mana.is_empty());
}

/// Reach guard: uncontrolled, the seat's own tap is accepted through both forms
/// and books to the seat.
#[test]
fn an_uncontrolled_seat_taps_its_own_land_through_both_boundary_forms() {
    let (mut runner, forest) = land_board(2, None);
    assert_seat_submits_through(&runner, P1, P1);
    let tap = engine_authored_tap(&runner, P1, forest);
    apply_as_current(runner.state_mut(), tap).expect("the seat taps its own land");
    assert_eq!(tracked_lands(&runner, P1), vec![forest]);

    let (mut runner, forest) = land_board(2, None);
    let tap = engine_authored_tap(&runner, P1, forest);
    apply_interaction_for_simulation(runner.state_mut(), P1, P1, tap)
        .expect("the split boundary accepts the seat's own tap");
    assert_eq!(tracked_lands(&runner, P1), vec![forest]);
}

/// CR 723.5a: an undoable land mana ability activated through `ActivateAbility`
/// books to the seat holding priority, never to its submitter or a bystander.
#[test]
fn a_controlled_seats_mana_ability_books_its_land_to_the_seat() {
    let (mut runner, forest) = land_board(4, Some(P0));
    assert_seat_submits_through(&runner, P1, P0);

    apply_as_current(
        runner.state_mut(),
        GameAction::ActivateAbility {
            source_id: forest,
            ability_index: 0,
        },
    )
    .expect("the controller activates the seat's land mana ability");

    assert_eq!(tracked_seats(&runner), vec![P1]);
    assert_eq!(tracked_lands(&runner, P1), vec![forest]);
}

/// Reach guard for the row above.
#[test]
fn an_uncontrolled_seats_mana_ability_books_its_land_to_the_seat() {
    let (mut runner, forest) = land_board(4, None);
    assert_seat_submits_through(&runner, P1, P1);

    apply_as_current(
        runner.state_mut(),
        GameAction::ActivateAbility {
            source_id: forest,
            ability_index: 0,
        },
    )
    .expect("the seat activates its own land mana ability");

    assert_eq!(tracked_seats(&runner), vec![P1]);
}

/// CR 723.5a + CR 605.3b: the undo of a manual land tap looks the tap up under
/// the seat it booked to, so the round trip closes at a priority window.
#[test]
fn a_controlled_seats_land_tap_undoes_at_priority() {
    let (mut runner, forest) = land_board(2, Some(P0));
    assert_seat_submits_through(&runner, P1, P0);
    let tap = engine_authored_tap(&runner, P1, forest);
    apply_as_current(runner.state_mut(), tap).expect("the controller taps the seat's land");
    assert_eq!(tracked_lands(&runner, P1), vec![forest]);

    apply_as_current(
        runner.state_mut(),
        GameAction::UntapLandForMana { object_id: forest },
    )
    .expect("the controller undoes the seat's own tap");

    assert!(runner.state().lands_tapped_for_mana.is_empty());
    assert!(!runner.state().objects[&forest].tapped);
}

/// The same round trip at a mana-payment window, whose tap arm already keyed
/// the seat while its undo arm did not.
#[test]
fn a_controlled_seats_land_tap_undoes_during_mana_payment() {
    let (mut runner, forest) = land_board(2, Some(P0));
    runner.enter_mana_payment(P1, None);
    assert_seat_submits_through(&runner, P1, P0);
    let tap = engine_authored_tap(&runner, P1, forest);
    apply_as_current(runner.state_mut(), tap).expect("the controller taps the seat's land");
    assert_eq!(tracked_lands(&runner, P1), vec![forest]);

    apply_as_current(
        runner.state_mut(),
        GameAction::UntapLandForMana { object_id: forest },
    )
    .expect("the controller undoes the seat's own tap mid-payment");

    assert!(runner.state().lands_tapped_for_mana.is_empty());
    assert!(!runner.state().objects[&forest].tapped);
}

/// The already-correct end of the same key class: an unless-payment window
/// destructures its own seat, so its round trip closes under control with no
/// source line changed for it.
#[test]
fn a_controlled_seats_land_tap_undoes_during_an_unless_payment() {
    let (mut runner, forest) = land_board(2, Some(P0));
    let pending_effect = ResolvedAbility::new(
        Effect::Unimplemented {
            name: "Unless Payment Witness".to_string(),
            description: None,
        },
        vec![],
        forest,
        P1,
    );
    runner.state_mut().waiting_for = WaitingFor::UnlessPayment {
        player: P1,
        cost: AbilityCost::Mana {
            cost: ManaCost::Cost {
                shards: vec![ManaCostShard::Green],
                generic: 0,
            },
        },
        pending_effect: Box::new(pending_effect),
        trigger_event: None,
        effect_description: Some("unless payment witness".to_string()),
        remaining: vec![],
    };
    assert_seat_submits_through(&runner, P1, P0);

    let tap = engine_authored_tap(&runner, P1, forest);
    apply_as_current(runner.state_mut(), tap).expect("the controller taps the seat's land");
    assert_eq!(tracked_lands(&runner, P1), vec![forest]);

    apply_as_current(
        runner.state_mut(),
        GameAction::UntapLandForMana { object_id: forest },
    )
    .expect("the controller undoes the seat's own tap during the unless payment");

    assert!(runner.state().lands_tapped_for_mana.is_empty());
}

/// A land the seat never tapped for mana stays untrackable, so the undo guard
/// still refuses it after the key moved to the seat.
#[test]
fn an_untapped_second_land_is_still_refused_by_the_undo_guard() {
    let mut scenario = GameScenario::new_n_player(2, 42);
    scenario.at_phase(Phase::PreCombatMain);
    let tapped = scenario.add_basic_land(P1, ManaColor::Green);
    let untouched = scenario.add_basic_land(P1, ManaColor::Green);
    let mut runner = scenario.build();
    install_priority(&mut runner, P1, Some(P0));
    let tap = engine_authored_tap(&runner, P1, tapped);
    apply_as_current(runner.state_mut(), tap).expect("the controller taps one of the seat's lands");

    assert!(matches!(
        apply_as_current(
            runner.state_mut(),
            GameAction::UntapLandForMana {
                object_id: untouched
            },
        ),
        Err(EngineError::InvalidAction(_))
    ));
    assert_eq!(tracked_lands(&runner, P1), vec![tapped]);
    assert!(!runner.state().objects[&untouched].tapped);
}

/// Build the Opposition Agent latch: an opponent's own-library search that
/// CR 723.2 hands to the agent's controller, with no `turn_decision_controller`.
fn opposition_agent_search(with_agent: bool) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    if with_agent {
        scenario.add_creature_from_oracle(P0, "Opposition Agent", 3, 2, OPPOSITION_AGENT);
    }
    let tutor = scenario
        .add_spell_to_hand_from_oracle(P1, "Test Tutor", false, TEST_TUTOR)
        .with_mana_cost(ManaCost::zero())
        .id();
    let found = scenario
        .add_spell_to_library_top(P1, "Found Card", true)
        .id();
    scenario.add_card_to_library_top(P1, "Library Filler");
    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }
    let outcome = runner.cast(tutor).resolve();
    assert!(
        matches!(
            outcome.final_waiting_for(),
            WaitingFor::SearchChoice { player: P1, .. }
        ),
        "the tutor must halt on the searcher's own search choice, got {:?}",
        outcome.final_waiting_for()
    );
    (runner, found)
}

fn seed_auto_pass(runner: &mut GameRunner) {
    for seat in [P0, P1] {
        runner.state_mut().auto_pass.insert(
            seat,
            AutoPassMode::UntilTurnBoundary {
                until: TurnBoundary::EndOfCurrentTurn,
            },
        );
    }
    assert!(runner.state().auto_pass.contains_key(&P0));
    assert!(runner.state().auto_pass.contains_key(&P1));
}

/// CR 723.2 + CR 723.5: the search latch redirects the submitter with no
/// `turn_decision_controller` set at all, and the searcher's own preference is
/// what the search choice discharges.
#[test]
fn a_latched_search_choice_discharges_the_searchers_preference() {
    let (mut runner, found) = opposition_agent_search(true);
    assert!(
        runner.state().turn_decision_controller.is_none(),
        "the latch must redirect with no turn-control field set"
    );
    assert_seat_submits_through(&runner, P1, P0);
    seed_auto_pass(&mut runner);

    runner
        .act(GameAction::SelectCards { cards: vec![found] })
        .expect("the agent's controller submits the searcher's choice");

    assert!(
        !runner.state().auto_pass.contains_key(&P1),
        "the searcher's own preference is the one the choice discharges"
    );
    assert!(
        runner.state().auto_pass.contains_key(&P0),
        "the submitting controller's preference is untouched"
    );
}

/// Reach guard: the same tutor with no agent on the battlefield discharges the
/// same seat's preference, so the row above pins the latch, not the tutor.
#[test]
fn an_unlatched_search_choice_discharges_the_searchers_preference() {
    let (mut runner, found) = opposition_agent_search(false);
    assert_seat_submits_through(&runner, P1, P1);
    seed_auto_pass(&mut runner);

    runner
        .act(GameAction::SelectCards { cards: vec![found] })
        .expect("the searcher submits its own choice");

    assert!(!runner.state().auto_pass.contains_key(&P1));
    assert!(runner.state().auto_pass.contains_key(&P0));
}

fn one_phase_stop() -> Vec<PhaseStop> {
    vec![PhaseStop {
        phase: Phase::End,
        scope: PhaseStopScope::AllTurns,
    }]
}

/// CR 723.5b: a phase-stop preference is not a choice called for by the rules
/// or by an object, so it writes the submitting seat's slot and never the
/// controlled seat's.
#[test]
fn a_phase_stop_preference_writes_the_submitters_slot_under_control() {
    let mut runner = four_player_board_with_every_seat_tracked(Some(P0));
    assert_seat_submits_through(&runner, P1, P0);

    apply_as_current(
        runner.state_mut(),
        GameAction::SetPhaseStops {
            stops: one_phase_stop(),
        },
    )
    .expect("a preference is legal in every state");

    assert_eq!(
        runner.state().phase_stops.get(&P0),
        Some(&one_phase_stop()),
        "the submitting connection owns the preference"
    );
    assert!(
        !runner.state().phase_stops.contains_key(&P1),
        "the controlled seat has no preference of its own to gain"
    );
}

/// Reach guard: uncontrolled, the same call reaches the same live handler and
/// writes the acting seat's own slot.
#[test]
fn a_phase_stop_preference_writes_the_acting_seats_slot_without_control() {
    let mut runner = four_player_board_with_every_seat_tracked(None);
    assert_seat_submits_through(&runner, P1, P1);

    apply_as_current(
        runner.state_mut(),
        GameAction::SetPhaseStops {
            stops: one_phase_stop(),
        },
    )
    .expect("a preference is legal in every state");

    assert_eq!(runner.state().phase_stops.get(&P1), Some(&one_phase_stop()));
}

/// CR 723.5b bounds the exemption: `SetAutoPass` stores a mode for the priority
/// seat and immediately passes that priority, so it stays a rules choice and
/// the controlled seat may not submit it.
#[test]
fn the_controlled_seat_may_not_submit_an_auto_pass_request() {
    let mut runner = four_player_board_with_every_seat_tracked(Some(P0));
    assert_seat_submits_through(&runner, P1, P0);

    assert!(
        matches!(
            apply_interaction_for_simulation(
                runner.state_mut(),
                P1,
                P1,
                GameAction::SetAutoPass {
                    mode: engine::types::game_state::AutoPassRequest::UntilStackEmpty,
                },
            ),
            Err(EngineError::WrongPlayer)
        ),
        "an auto-pass request is not exempt from submitter authorization"
    );
}

/// Live positive control for the refusal above: the same unauthorized actor
/// submitting an actually-exempt action is accepted, so the exemption decides
/// the verdict rather than the board.
#[test]
fn the_controlled_seat_may_submit_its_own_phase_stop_preference() {
    let mut runner = four_player_board_with_every_seat_tracked(Some(P0));

    apply_interaction_for_simulation(
        runner.state_mut(),
        P1,
        P1,
        GameAction::SetPhaseStops {
            stops: one_phase_stop(),
        },
    )
    .expect("a preference names its own submitter, so any seat may set its own");

    assert_eq!(runner.state().phase_stops.get(&P1), Some(&one_phase_stop()));
}

/// A sandbox-enabled controlled board. Both preconditions are load-bearing:
/// `debug_mode` gates every debug action, and `allow_debug_actions` gates the
/// host grant/revoke guard ahead of its host check.
fn sandbox_board(permitted: PlayerId) -> GameRunner {
    let mut runner = four_player_board_with_every_seat_tracked(Some(P0));
    let state = runner.state_mut();
    state.debug_mode = true;
    state.format_config.allow_debug_actions = true;
    state.debug_permitted.clear();
    state.debug_permitted.insert(permitted);
    runner
}

fn zero_count_create() -> GameAction {
    GameAction::Debug(DebugAction::CreateCard {
        card_name: "Forest".to_string(),
        owner: P1,
        zone: Zone::Hand,
        count: 0,
        attach_to: None,
        run_etb: false,
        nonlegendary: false,
    })
}

/// CR 723.5b: a sandbox capability authorizes the submitting connection, so the
/// gate reads the submitter even while that submitter controls another player.
#[test]
fn a_debug_action_is_gated_on_the_submitting_connection() {
    let mut runner = sandbox_board(P0);
    assert_seat_submits_through(&runner, P1, P0);

    apply_as_current(
        runner.state_mut(),
        GameAction::Debug(DebugAction::SetLife {
            player_id: P1,
            life: 15,
        }),
    )
    .expect("the permitted submitter's debug action is accepted");

    assert_eq!(runner.state().players[P1.0 as usize].life, 15);
}

/// Hostile fixture for the row above: permitting the controlled seat instead of
/// the submitter must refuse the same call, proving the gate is live.
#[test]
fn a_debug_action_is_refused_when_only_the_controlled_seat_is_permitted() {
    let mut runner = sandbox_board(P1);
    let life_before = runner.state().players[P1.0 as usize].life;

    assert!(matches!(
        apply_as_current(
            runner.state_mut(),
            GameAction::Debug(DebugAction::SetLife {
                player_id: P1,
                life: 15,
            }),
        ),
        Err(EngineError::InvalidAction(_))
    ));
    assert_eq!(runner.state().players[P1.0 as usize].life, life_before);
}

/// The zero-count debug no-op takes the boundary's own early return, so its
/// preflight call site is covered against both permission sets.
#[test]
fn a_zero_count_debug_action_is_gated_on_the_submitting_connection() {
    let mut runner = sandbox_board(P0);
    apply_as_current(runner.state_mut(), zero_count_create())
        .expect("the permitted submitter's zero-count debug no-op is accepted");

    let mut runner = sandbox_board(P1);
    assert!(matches!(
        apply_as_current(runner.state_mut(), zero_count_create()),
        Err(EngineError::InvalidAction(_))
    ));
}

/// CR 723.5b: the host-only grant guard compares the submitting connection, so
/// controlling another player must not move the comparison onto that player.
#[test]
fn a_debug_permission_grant_is_authorized_against_the_submitting_host() {
    let mut runner = sandbox_board(P0);
    assert_seat_submits_through(&runner, P1, P0);

    apply_as_current(
        runner.state_mut(),
        GameAction::GrantDebugPermission { player_id: P1 },
    )
    .expect("the host grants debug permission while controlling another player");

    assert!(runner.state().debug_permitted.contains(&P1));
}
