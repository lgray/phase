//! A mana ability's own mana cost that neither the pool nor auto-tap covers opens a payment
//! window (CR 605.3a + CR 117.1d + CR 601.2g via CR 602.2b), driven through `apply()` on
//! Grand Architect + Pili-Pala.
use engine::ai_support::{legal_actions, legal_actions_full};
use engine::game::mana_abilities::is_mana_ability;
use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::ability::AbilityKind;
use engine::types::actions::GameAction;
use engine::types::game_state::{GameState, ManaChoice, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaType;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

use crate::support::shared_card_db;

fn ability(state: &GameState, id: ObjectId, mana: bool) -> usize {
    state.objects[&id]
        .abilities
        .iter()
        .position(|a| a.kind == AbilityKind::Activated && is_mana_ability(a) == mana)
        .expect("the ability")
}

fn act(runner: &mut GameRunner, action: GameAction) {
    let shown = format!("{action:?}");
    runner
        .act(action)
        .unwrap_or_else(|error| panic!("{shown} rejected: {error:?}"));
}

fn activate(runner: &mut GameRunner, source: ObjectId, mana: bool) {
    let ability_index = ability(runner.state(), source, mana);
    act(
        runner,
        GameAction::ActivateAbility {
            source_id: source,
            ability_index,
        },
    );
}

fn pool_total(state: &GameState) -> usize {
    state.players[0].mana_pool.total()
}

struct Board {
    runner: GameRunner,
    architect: ObjectId,
    pili: ObjectId,
}

/// Grand Architect: "{U}: Target artifact creature becomes blue until end of turn." and "Tap an
/// untapped blue creature you control: Add {C}{C}. Spend this mana only to cast artifact spells
/// or activate abilities of artifacts." Pili-Pala: "{2}, {Q}: Add one mana of any color."
/// With `blue`, the Island pays Grand Architect's {U} to make Pili-Pala blue, leaving the pool
/// empty and Pili-Pala untapped.
fn board(blue: bool) -> Option<Board> {
    let db = shared_card_db()?;
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let architect = scenario.add_real_card(P0, "Grand Architect", Zone::Battlefield, db);
    let pili = scenario.add_real_card(P0, "Pili-Pala", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Island", Zone::Battlefield, db);
    let mut runner = scenario.build();
    if blue {
        activate(&mut runner, architect, false);
        while !runner.state().stack.is_empty() {
            act(&mut runner, GameAction::PassPriority);
        }
    }
    assert_eq!(
        pool_total(runner.state()),
        0,
        "reach: the pool starts empty"
    );
    assert!(
        !runner.state().objects[&pili].tapped,
        "reach: Pili-Pala untapped"
    );
    Some(Board {
        runner,
        architect,
        pili,
    })
}

fn open_window(board: &mut Board) {
    activate(&mut board.runner, board.pili, true);
    assert!(
        matches!(
            board.runner.state().waiting_for,
            WaitingFor::ManaAbilityManaPayment { .. }
        ),
        "Pili-Pala's {{2}} opens its payment window: {:?}",
        board.runner.state().waiting_for
    );
}

fn architect_activation(board: &Board) -> GameAction {
    GameAction::ActivateAbility {
        source_id: board.architect,
        ability_index: ability(board.runner.state(), board.architect, true),
    }
}

#[test]
fn grand_architect_pays_pili_pala_inside_its_payment_window() {
    let Some(mut board) = board(true) else { return };
    open_window(&mut board);
    let architect = architect_activation(&board);
    act(&mut board.runner, architect);
    // Grand Architect's cost taps Pili-Pala itself; the {Q} then untaps it (CR 601.2h).
    act(
        &mut board.runner,
        GameAction::SelectCards {
            cards: vec![board.pili],
        },
    );
    act(
        &mut board.runner,
        GameAction::ChooseManaColor {
            choice: ManaChoice::SingleColor(ManaType::Blue),
            count: 1,
        },
    );
    let state = board.runner.state();
    assert!(matches!(state.waiting_for, WaitingFor::Priority { .. }));
    assert_eq!(state.players[0].mana_pool.count_color(ManaType::Blue), 1);
    assert_eq!(
        pool_total(state),
        1,
        "the {{C}}{{C}} paid Pili-Pala's {{2}}"
    );
    assert!(
        !state.objects[&board.pili].tapped,
        "{{Q}} untapped Pili-Pala"
    );
}

#[test]
fn the_ai_drive_pays_pili_pala_from_its_window_candidates() {
    let Some(mut board) = board(true) else { return };
    open_window(&mut board);
    let architect = architect_activation(&board);
    assert!(
        legal_actions(board.runner.state()).contains(&architect),
        "the window offers Grand Architect's mana ability"
    );
    act(&mut board.runner, architect);
    for _ in 0..8 {
        if matches!(
            board.runner.state().waiting_for,
            WaitingFor::Priority { .. }
        ) {
            break;
        }
        let action = legal_actions(board.runner.state())
            .into_iter()
            .find(|action| !matches!(action, GameAction::CancelCast))
            .expect("an answer");
        act(&mut board.runner, action);
    }
    let state = board.runner.state();
    assert!(matches!(state.waiting_for, WaitingFor::Priority { .. }));
    assert_eq!(pool_total(state), 1, "Pili-Pala's mana was added");
}

#[test]
fn cancelling_the_window_withdraws_pili_palas_activation() {
    let Some(mut board) = board(true) else { return };
    open_window(&mut board);
    assert!(legal_actions(board.runner.state()).contains(&GameAction::CancelCast));
    act(&mut board.runner, GameAction::CancelCast);
    let state = board.runner.state();
    assert!(matches!(state.waiting_for, WaitingFor::Priority { player } if player == P0));
    assert_eq!(pool_total(state), 0);
    assert!(!state.objects[&board.pili].tapped);
    assert!(state.stack.is_empty());
}

#[test]
fn no_window_opens_without_a_mana_ability_that_could_pay() {
    // Pili-Pala is not blue and Grand Architect is tapped, so no untapped blue creature can pay
    // Grand Architect's cost; the Island alone cannot pay {2}.
    let Some(mut board) = board(false) else {
        return;
    };
    let (architect, pili) = (board.architect, board.pili);
    board
        .runner
        .state_mut()
        .objects
        .get_mut(&architect)
        .unwrap()
        .tapped = true;
    let ability_index = ability(board.runner.state(), pili, true);
    let refused = board.runner.act(GameAction::ActivateAbility {
        source_id: pili,
        ability_index,
    });
    assert!(
        refused.is_err(),
        "no window: {:?}",
        board.runner.state().waiting_for
    );
    assert!(matches!(
        board.runner.state().waiting_for,
        WaitingFor::Priority { .. }
    ));
    // Reach guard: with Grand Architect untapped it can tap itself, so the window opens.
    let Some(mut untapped) = self::board(false) else {
        return;
    };
    open_window(&mut untapped);
}

struct AltarBoard {
    runner: GameRunner,
    altar: ObjectId,
    bears: ObjectId,
    pili: ObjectId,
    island: ObjectId,
}

/// Phyrexian Altar: "Sacrifice a creature: Add one mana of any color." Pili-Pala is tapped and
/// the pool empty, so the Island alone cannot pay its {2} and the window opens.
fn altar_board() -> Option<AltarBoard> {
    let db = shared_card_db()?;
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let altar = scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    let bears = scenario.add_real_card(P0, "Grizzly Bears", Zone::Battlefield, db);
    let pili = scenario.add_real_card(P0, "Pili-Pala", Zone::Battlefield, db);
    let island = scenario.add_real_card(P0, "Island", Zone::Battlefield, db);
    let mut runner = scenario.build();
    runner.state_mut().objects.get_mut(&pili).unwrap().tapped = true;
    activate(&mut runner, pili, true);
    assert!(matches!(
        runner.state().waiting_for,
        WaitingFor::ManaAbilityManaPayment { .. }
    ));
    Some(AltarBoard {
        runner,
        altar,
        bears,
        pili,
        island,
    })
}

fn tap_island(board: &mut AltarBoard) {
    let action = legal_actions_full(board.runner.state())
        .2
        .get(&board.island)
        .and_then(|actions| {
            actions
                .iter()
                .find(|action| matches!(action, GameAction::TapLandForMana { .. }))
                .cloned()
        })
        .expect("the window offers the Island");
    act(&mut board.runner, action);
}

fn sacrifice_bears_for_red(board: &mut AltarBoard) {
    activate(&mut board.runner, board.altar, true);
    act(
        &mut board.runner,
        GameAction::SelectCards {
            cards: vec![board.bears],
        },
    );
    act(
        &mut board.runner,
        GameAction::ChooseManaColor {
            choice: ManaChoice::SingleColor(ManaType::Red),
            count: 1,
        },
    );
}

fn assert_pili_pala_paid(board: &mut AltarBoard) {
    assert!(
        matches!(
            board.runner.state().waiting_for,
            WaitingFor::ChooseManaColor { .. }
        ),
        "the {{2}} is paid and Pili-Pala asks its color: {:?}",
        board.runner.state().waiting_for
    );
    act(
        &mut board.runner,
        GameAction::ChooseManaColor {
            choice: ManaChoice::SingleColor(ManaType::Green),
            count: 1,
        },
    );
    let state = board.runner.state();
    assert!(matches!(state.waiting_for, WaitingFor::Priority { .. }));
    assert_eq!(state.players[0].mana_pool.count_color(ManaType::Green), 1);
    assert_eq!(pool_total(state), 1);
    assert!(!state.objects[&board.pili].tapped);
    assert_eq!(state.objects[&board.bears].zone, Zone::Graveyard);
}

#[test]
fn a_sacrifice_mana_ability_completes_the_window_after_a_land() {
    let Some(mut board) = altar_board() else {
        return;
    };
    tap_island(&mut board);
    assert!(
        matches!(
            board.runner.state().waiting_for,
            WaitingFor::ManaAbilityManaPayment { .. }
        ),
        "one mana of the {{2}}: the window reopens"
    );
    assert_eq!(pool_total(board.runner.state()), 1);
    sacrifice_bears_for_red(&mut board);
    assert_pili_pala_paid(&mut board);
}

#[test]
fn after_a_sacrifice_mana_ability_auto_tap_pays_the_rest() {
    let Some(mut board) = altar_board() else {
        return;
    };
    sacrifice_bears_for_red(&mut board);
    // CR 601.2g: the Island now covers the rest, so the re-entered activation auto-taps it.
    assert!(board.runner.state().objects[&board.island].tapped);
    assert_pili_pala_paid(&mut board);
}

/// What a human seat is shown: the interaction authority bound, the state filtered for the
/// viewer, and the projection derived from that copy.
fn viewer_projection(
    state: &GameState,
) -> (GameState, engine::types::interaction::ViewerInteraction) {
    use engine::game::interaction::{bind_interaction_authority, derive_viewer_interaction};
    use engine::types::interaction::InteractionSessionId;
    let mut bound = state.clone();
    bind_interaction_authority(&mut bound, InteractionSessionId("viewer".to_string()))
        .expect("the interaction authority binds");
    let filtered = engine::game::visibility::filter_state_for_viewer(&bound, P0);
    let view = derive_viewer_interaction(&bound, &filtered, P0);
    (filtered, view)
}

fn offers(view: &engine::types::interaction::ViewerInteraction, action: &GameAction) -> bool {
    use engine::types::interaction::{
        InteractionOpportunityResponse, InteractionPresentationSurface,
    };
    let wanted = engine::game::interaction::interaction_action_id(action);
    view.opportunities
        .iter()
        .flat_map(|opportunity| match &opportunity.response {
            InteractionOpportunityResponse::ExactChoices { choices } => choices.iter(),
            InteractionOpportunityResponse::Schema { candidates, .. } => candidates.iter(),
        })
        .flat_map(|choice| choice.surfaces.iter())
        .any(|surface| {
            matches!(
                surface,
                InteractionPresentationSurface::Action { action_id: Some(id), .. } if *id == wanted
            )
        })
}

/// The mana ability of `id` whose cost is mana alone.
fn mana_costed(state: &GameState, id: ObjectId) -> GameAction {
    use engine::types::ability::AbilityCost;
    let ability_index = state.objects[&id]
        .abilities
        .iter()
        .position(|a| is_mana_ability(a) && matches!(a.cost, Some(AbilityCost::Mana { .. })))
        .expect("a mana ability with a mana cost");
    GameAction::ActivateAbility {
        source_id: id,
        ability_index,
    }
}

/// The floating pip is journaled and verified in the game's own state, and the viewer's copy
/// holds neither the producer record nor the verification.
fn assert_the_viewers_copy_drops_the_pools_verification(state: &GameState, filtered: &GameState) {
    let pips: Vec<_> = state.players[0]
        .mana_pool
        .units()
        .map(|unit| unit.pip_id)
        .collect();
    assert!(!pips.is_empty(), "reach: mana floats");
    assert!(
        pips.iter()
            .all(|pip| state.resolved_rules_journal.has_produced_pip(*pip)),
        "reach: the game's journal produced the floating mana"
    );
    assert!(
        state.players[0].mana_pool.is_verified(),
        "the game's own pool stays verified"
    );
    assert!(
        pips.iter()
            .all(|pip| !filtered.resolved_rules_journal.has_produced_pip(*pip)),
        "reach: the viewer's copy carries no producer record"
    );
    assert!(
        !filtered.players[0].mana_pool.is_verified(),
        "a copy without the journal does not claim its pips are journaled"
    );
}

/// Gruul Signet: "{1}, {T}: Add {R}{G}." While Grizzly Bears is being paid for with a Forest's
/// {G} floating, the Signet's {1} is payable from the pool (CR 605.3a).
#[test]
fn a_viewer_is_offered_a_costed_mana_ability_while_mana_floats_in_a_spell_payment() {
    use engine::types::game_state::CastPaymentMode;
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let forest = scenario.add_real_card(P0, "Forest", Zone::Battlefield, db);
    let signet = scenario.add_real_card(P0, "Gruul Signet", Zone::Battlefield, db);
    let bears = scenario.add_real_card(P0, "Grizzly Bears", Zone::Hand, db);
    let mut runner = scenario.build();
    let signet_ability = GameAction::ActivateAbility {
        source_id: signet,
        ability_index: ability(runner.state(), signet, true),
    };

    // With the same {G} floating and no payment in progress the Signet is offered.
    let mut at_priority = GameRunner::from_state(runner.state().clone());
    activate(&mut at_priority, forest, true);
    assert_eq!(pool_total(at_priority.state()), 1, "reach: {{G}} floats");
    assert!(offers(
        &viewer_projection(at_priority.state()).1,
        &signet_ability
    ));

    let card_id = runner.state().objects[&bears].card_id;
    act(
        &mut runner,
        GameAction::CastSpell {
            object_id: bears,
            card_id,
            targets: Vec::new(),
            payment_mode: CastPaymentMode::Manual,
        },
    );
    assert!(matches!(
        runner.state().waiting_for,
        WaitingFor::ManaPayment { .. }
    ));
    // With the pool empty the Forest can pay the Signet's {1}.
    assert!(offers(
        &viewer_projection(runner.state()).1,
        &signet_ability
    ));

    activate(&mut runner, forest, true);
    assert!(matches!(
        runner.state().waiting_for,
        WaitingFor::ManaPayment { .. }
    ));
    let (filtered, view) = viewer_projection(runner.state());
    assert!(offers(&view, &signet_ability));
    assert_the_viewers_copy_drops_the_pools_verification(runner.state(), &filtered);
}

/// Skyshroud Elf: "{T}: Add {G}." and "{1}: Add {R} or {W}." In Pili-Pala's payment window with
/// the Island's {U} floating, the tapped Elf's {1} is payable from the pool (CR 605.3a).
#[test]
fn a_viewer_is_offered_a_costed_mana_ability_while_mana_floats_in_a_mana_abilitys_payment() {
    let Some(db) = shared_card_db() else { return };
    for with_elf in [false, true] {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::PreCombatMain);
        scenario.add_real_card(P0, "Grand Architect", Zone::Battlefield, db);
        let pili = scenario.add_real_card(P0, "Pili-Pala", Zone::Battlefield, db);
        let island = scenario.add_real_card(P0, "Island", Zone::Battlefield, db);
        let elf =
            with_elf.then(|| scenario.add_real_card(P0, "Skyshroud Elf", Zone::Battlefield, db));
        let mut runner = scenario.build();
        if let Some(elf) = elf {
            runner.state_mut().objects.get_mut(&elf).unwrap().tapped = true;
        }
        activate(&mut runner, pili, true);
        activate(&mut runner, island, true);
        assert!(
            matches!(
                runner.state().waiting_for,
                WaitingFor::ManaAbilityManaPayment { .. }
            ),
            "reach: Pili-Pala's window stands: {:?}",
            runner.state().waiting_for
        );
        assert_eq!(pool_total(runner.state()), 1, "reach: {{U}} floats");
        let (filtered, view) = viewer_projection(runner.state());
        assert!(offers(&view, &GameAction::CancelCast));
        // Without the Elf no mana ability's cost is payable from the pool.
        let Some(elf) = elf else { continue };
        assert!(offers(&view, &mana_costed(runner.state(), elf)));
        assert_the_viewers_copy_drops_the_pools_verification(runner.state(), &filtered);
    }
}
