//! A mana ability's own mana cost that neither the pool nor auto-tap covers opens a payment
//! window (CR 605.3a + CR 117.1d + CR 601.2g via CR 602.2b), driven through `apply()` on
//! Grand Architect + Pili-Pala.
use engine::ai_support::{legal_actions, legal_actions_full};
use engine::game::mana_abilities::is_mana_ability;
use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::ability::{AbilityCost, AbilityKind};
use engine::types::actions::GameAction;
use engine::types::counter::CounterType;
use engine::types::game_state::{GameState, ManaAbilityResume, ManaChoice, WaitingFor};
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

/// The index of `id`'s mana ability whose cost `pick` accepts.
pub(crate) fn mana_ability_costing(
    state: &GameState,
    id: ObjectId,
    pick: impl Fn(&AbilityCost) -> bool,
) -> usize {
    state.objects[&id]
        .abilities
        .iter()
        .position(|a| is_mana_ability(a) && a.cost.as_ref().is_some_and(&pick))
        .expect("a mana ability with that cost")
}

/// The mana ability of `id` whose cost is {T} alone.
fn tap_costed(state: &GameState, id: ObjectId) -> GameAction {
    GameAction::ActivateAbility {
        source_id: id,
        ability_index: mana_ability_costing(state, id, |cost| *cost == AbilityCost::Tap),
    }
}

/// The mana ability of `id` whose cost has several parts ("{1}, {T}", "{1}, Remove X counters").
fn composite_costed(state: &GameState, id: ObjectId) -> GameAction {
    GameAction::ActivateAbility {
        source_id: id,
        ability_index: mana_ability_costing(state, id, |cost| {
            matches!(cost, AbilityCost::Composite { .. })
        }),
    }
}

/// The activations suspended at the standing payment window, innermost first.
fn suspended(state: &GameState) -> Vec<GameAction> {
    let WaitingFor::ManaAbilityManaPayment {
        pending_mana_ability,
        ..
    } = &state.waiting_for
    else {
        return Vec::new();
    };
    std::iter::successors(Some(pending_mana_ability), |pending| {
        match &pending.resume {
            ManaAbilityResume::ManaAbilityManaPayment {
                pending_mana_ability,
            } => Some(pending_mana_ability),
            _ => None,
        }
    })
    .map(|pending| GameAction::ActivateAbility {
        source_id: pending.source_id,
        ability_index: pending.ability_index.expect("an enumerated ability"),
    })
    .collect()
}

/// Submits `action` and requires the engine to refuse it and leave the game as it stood.
fn assert_refused(runner: &mut GameRunner, action: &GameAction) {
    let before = runner.state().clone();
    assert!(
        runner.act(action.clone()).is_err(),
        "{action:?} is refused: {:?}",
        suspended(runner.state())
    );
    assert!(
        *runner.state() == before,
        "a refused {action:?} changes nothing"
    );
}

/// Grand Architect taps itself for {C}{C}, spendable only on artifacts.
fn tap_architect(runner: &mut GameRunner, architect: ObjectId) {
    activate(runner, architect, true);
    act(
        runner,
        GameAction::SelectCards {
            cards: vec![architect],
        },
    );
}

/// The tap the engine authors for `land` at this prompt, as a human seat is offered it.
fn land_tap(state: &GameState, land: ObjectId) -> Option<GameAction> {
    legal_actions_full(state)
        .2
        .get(&land)
        .into_iter()
        .flatten()
        .find(|action| matches!(action, GameAction::TapLandForMana { .. }))
        .cloned()
}

/// Whether the display sweep reports a mana ability of `id` as available.
fn swept_ready(state: &GameState, id: ObjectId) -> bool {
    let mut shown = state.clone();
    engine::game::public_state::mark_mana_display_dirty(&mut shown);
    engine::game::derived::derive_display_state(&mut shown);
    shown.objects[&id].has_mana_ability
}

/// Prismite: "{2}: Add one mana of any color." (CR 605.3c)
#[test]
fn a_suspended_mana_ability_cannot_be_activated_again() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let prismite = scenario.add_real_card(P0, "Prismite", Zone::Battlefield, db);
    let architect = scenario.add_real_card(P0, "Grand Architect", Zone::Battlefield, db);
    let mut runner = scenario.build();
    let begun = mana_costed(runner.state(), prismite);
    act(&mut runner, begun.clone());
    assert_eq!(
        suspended(runner.state()),
        [begun.clone()],
        "reach: Prismite's {{2}} opens its payment window"
    );

    assert_refused(&mut runner, &begun);
    assert!(!legal_actions(runner.state()).contains(&begun));

    tap_architect(&mut runner, architect);
    assert!(
        runner.state().objects[&architect].tapped
            && matches!(
                runner.state().waiting_for,
                WaitingFor::ChooseManaColor { .. }
            ),
        "another source pays the window and Prismite asks its color: {:?}",
        runner.state().waiting_for
    );
    act(
        &mut runner,
        GameAction::ChooseManaColor {
            choice: ManaChoice::SingleColor(ManaType::Red),
            count: 1,
        },
    );
    assert!(matches!(
        runner.state().waiting_for,
        WaitingFor::Priority { .. }
    ));
    assert_eq!(pool_total(runner.state()), 1);
}

/// Skyshroud Elf: "{T}: Add {G}." and "{1}: Add {R} or {W}." CR 605.3c names the ability, so the
/// Elf's other ability and Phyrexian Altar ("Sacrifice a creature: Add one mana of any color.")
/// stay legal while its {1} ability is suspended.
#[test]
fn a_suspended_mana_abilitys_other_ability_and_other_sources_still_pay() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let elf = scenario.add_real_card(P0, "Skyshroud Elf", Zone::Battlefield, db);
    let altar = scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    let bears = scenario.add_real_card(P0, "Grizzly Bears", Zone::Battlefield, db);
    let mut runner = scenario.build();
    let begun = mana_costed(runner.state(), elf);
    act(&mut runner, begun.clone());
    assert_eq!(
        suspended(runner.state()),
        [begun.clone()],
        "reach: the Elf's {{1}} opens its payment window"
    );

    assert_refused(&mut runner, &begun);

    let mut other_ability = GameRunner::from_state(runner.state().clone());
    let tap = tap_costed(other_ability.state(), elf);
    act(&mut other_ability, tap);
    assert!(
        other_ability.state().objects[&elf].tapped
            && matches!(
                other_ability.state().waiting_for,
                WaitingFor::ChooseManaColor { .. }
            ),
        "the Elf's own {{G}} pays its {{1}}: {:?}",
        other_ability.state().waiting_for
    );

    activate(&mut runner, altar, true);
    act(&mut runner, GameAction::SelectCards { cards: vec![bears] });
    assert_eq!(runner.state().objects[&bears].zone, Zone::Graveyard);
}

/// CR 605.3c: an ability suspended beneath the standing window has not resolved either.
#[test]
fn a_suspended_ancestor_mana_ability_cannot_be_activated_again() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Grizzly Bears", Zone::Battlefield, db);
    let prismite = scenario.add_real_card(P0, "Prismite", Zone::Battlefield, db);
    let elf = scenario.add_real_card(P0, "Skyshroud Elf", Zone::Battlefield, db);
    let mut runner = scenario.build();
    let ancestor = mana_costed(runner.state(), prismite);
    let inner = mana_costed(runner.state(), elf);
    act(&mut runner, ancestor.clone());
    act(&mut runner, inner.clone());
    assert_eq!(
        suspended(runner.state()),
        [inner, ancestor.clone()],
        "reach: the Elf's window stands inside Prismite's"
    );

    assert_refused(&mut runner, &ancestor);
}

struct ArchitectBoard {
    runner: GameRunner,
    altar: ObjectId,
    architect: ObjectId,
    /// The permanents named to the builder, in the order they were created.
    outer: Vec<ObjectId>,
    elf: ObjectId,
}

/// Phyrexian Altar, Grizzly Bears, Grand Architect, each of `outer` in order, and Skyshroud Elf.
fn architect_board(outer: &[&str], elf_tapped: bool) -> Option<ArchitectBoard> {
    let db = shared_card_db()?;
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let altar = scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Grizzly Bears", Zone::Battlefield, db);
    let architect = scenario.add_real_card(P0, "Grand Architect", Zone::Battlefield, db);
    let outer: Vec<ObjectId> = outer
        .iter()
        .map(|name| scenario.add_real_card(P0, name, Zone::Battlefield, db))
        .collect();
    let elf = scenario.add_real_card(P0, "Skyshroud Elf", Zone::Battlefield, db);
    let mut runner = scenario.build();
    runner.state_mut().objects.get_mut(&elf).unwrap().tapped = elf_tapped;
    Some(ArchitectBoard {
        runner,
        altar,
        architect,
        outer,
        elf,
    })
}

/// Whether the pool is exactly Grand Architect's two artifact-only mana.
fn holds_only_architect_mana(state: &GameState) -> bool {
    let pool = &state.players[0].mana_pool;
    pool.total() == 2 && pool.units().all(|unit| !unit.restrictions.is_empty())
}

/// Suspends `outer`, nests the Elf's "{1}" window in it, then taps Grand Architect for mana.
/// Returns the two suspended activations, innermost first.
fn nest_then_tap_architect(board: &mut ArchitectBoard, outer: GameAction) -> [GameAction; 2] {
    let inner = mana_costed(board.runner.state(), board.elf);
    act(&mut board.runner, outer.clone());
    act(&mut board.runner, inner.clone());
    let chain = [inner, outer];
    assert_eq!(
        suspended(board.runner.state()),
        chain,
        "reach: the Elf's window stands inside the outer one"
    );
    tap_architect(&mut board.runner, board.architect);
    chain
}

/// With Prismite's "{2}" suspended beneath the Elf's window and Grand Architect's {C}{C} in the
/// pool, the pool could pay Prismite again; no reader of readiness may say so (CR 605.3c).
#[test]
fn a_suspended_ancestor_is_not_ready_once_the_pool_could_pay_it() {
    let Some(mut board) = architect_board(&["Prismite"], false) else {
        return;
    };
    let (prismite, elf, altar) = (board.outer[0], board.elf, board.altar);
    let ancestor = mana_costed(board.runner.state(), prismite);
    let chain = nest_then_tap_architect(&mut board, ancestor.clone());
    let state = board.runner.state();
    assert!(
        suspended(state) == chain && holds_only_architect_mana(state),
        "reach: artifact-only mana cannot pay the Elf, so its window stands: {:?}",
        state.waiting_for
    );

    let view = viewer_projection(state).1;
    assert_eq!(
        (
            legal_actions(state).contains(&ancestor),
            swept_ready(state, prismite),
            offers(&view, &ancestor),
        ),
        (false, false, false),
        "legal actions, the display sweep and the viewer's moves all leave the suspended \
         Prismite out"
    );
    let still_legal = [
        tap_costed(state, elf),
        GameAction::ActivateAbility {
            source_id: altar,
            ability_index: ability(state, altar, true),
        },
    ];
    for action in &still_legal {
        assert!(legal_actions(state).contains(action), "{action:?}");
        assert!(offers(&view, action), "{action:?}");
    }
    assert!(offers(&view, &GameAction::CancelCast));
    assert!(swept_ready(state, elf) && swept_ready(state, altar));
    assert_refused(&mut board.runner, &ancestor);

    // With no window open, both readers show a pool-funded Prismite as ready.
    let Some(mut free) = architect_board(&["Prismite"], false) else {
        return;
    };
    tap_architect(&mut free.runner, free.architect);
    let state = free.runner.state();
    assert!(swept_ready(state, free.outer[0]));
    assert!(offers(
        &viewer_projection(state).1,
        &mana_costed(state, free.outer[0])
    ));
}

/// Dimir Signet: "{1}, {T}: Add {U}{B}." Auto-tap picks its own sources, and a Signet suspended
/// beneath the Elf's window is not one of them (CR 605.3c).
#[test]
fn the_auto_payer_does_not_tap_a_suspended_ancestor() {
    let Some(mut board) = architect_board(&["Dimir Signet"], true) else {
        return;
    };
    let signet = board.outer[0];
    let outer = composite_costed(board.runner.state(), signet);
    let chain = nest_then_tap_architect(&mut board, outer);
    let state = board.runner.state();
    assert_eq!(
        suspended(state),
        chain,
        "the Elf's window stands over the Signet"
    );
    assert!(!state.objects[&signet].tapped);
    assert!(holds_only_architect_mana(state), "nothing was spent");

    // Not suspended, the same Signet is the payer's choice.
    let Some(mut free) = architect_board(&["Dimir Signet"], true) else {
        return;
    };
    tap_architect(&mut free.runner, free.architect);
    let inner = mana_costed(free.runner.state(), free.elf);
    act(&mut free.runner, inner);
    assert!(
        free.runner.state().objects[&free.outer[0]].tapped
            && matches!(
                free.runner.state().waiting_for,
                WaitingFor::ChooseManaColor { .. }
            ),
        "auto-tap pays the Elf with the Signet: {:?}",
        free.runner.state().waiting_for
    );

    // Prismite has no {T} in its cost and is never the payer's choice.
    let Some(mut board) = architect_board(&["Prismite"], true) else {
        return;
    };
    let outer = mana_costed(board.runner.state(), board.outer[0]);
    let chain = nest_then_tap_architect(&mut board, outer);
    assert_eq!(suspended(board.runner.state()), chain);
    assert!(holds_only_architect_mana(board.runner.state()));

    // Izzet Signet: "{1}, {T}: Add {U}{R}." Created after the suspended Dimir Signet, it is the
    // one the payer taps.
    let Some(mut board) = architect_board(&["Dimir Signet", "Izzet Signet"], true) else {
        return;
    };
    let (dimir, izzet) = (board.outer[0], board.outer[1]);
    let outer = composite_costed(board.runner.state(), dimir);
    nest_then_tap_architect(&mut board, outer);
    let state = board.runner.state();
    assert!(
        matches!(state.waiting_for, WaitingFor::ChooseManaColor { .. }),
        "the Elf is paid and asks its color: {:?}",
        state.waiting_for
    );
    assert!(state.objects[&izzet].tapped && !state.objects[&dimir].tapped);
}

/// Calciform Pools: "{T}: Add {C}." and "{1}, Remove X storage counters from this land: Add X
/// mana in any combination of {W} and/or {U}." Celestial Prism: "{2}, {T}: Add one mana of any
/// color." With the Pools' third ability suspended beneath the Prism's window, auto-tap leaves
/// the Pools alone and its "{T}: Add {C}" is the player's to activate (CR 605.3c).
#[test]
fn a_suspended_ancestors_other_mana_ability_is_left_for_the_player_to_tap() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let altar = scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    let bears = scenario.add_real_card(P0, "Grizzly Bears", Zone::Battlefield, db);
    let pools = scenario.add_real_card(P0, "Calciform Pools", Zone::Battlefield, db);
    let prism = scenario.add_real_card(P0, "Celestial Prism", Zone::Battlefield, db);
    let mut runner = scenario.build();
    let storage = CounterType::Generic("storage".to_string());
    runner
        .state_mut()
        .objects
        .get_mut(&pools)
        .unwrap()
        .counters
        .insert(storage.clone(), 2);
    let outer = composite_costed(runner.state(), pools);
    let inner = composite_costed(runner.state(), prism);
    act(&mut runner, outer.clone());
    act(&mut runner, GameAction::SubmitPayAmount { amount: 1 });
    act(&mut runner, inner.clone());
    let chain = [inner, outer];
    assert_eq!(
        suspended(runner.state()),
        chain,
        "reach: the Prism's window stands inside the Pools'"
    );
    activate(&mut runner, altar, true);
    act(&mut runner, GameAction::SelectCards { cards: vec![bears] });
    assert_eq!(
        runner.state().objects[&bears].zone,
        Zone::Graveyard,
        "reach: the Altar's cost was paid"
    );

    let color = runner.act(GameAction::ChooseManaColor {
        choice: ManaChoice::SingleColor(ManaType::Red),
        count: 1,
    });
    assert!(color.is_ok(), "the Altar's color is accepted: {color:?}");
    let state = runner.state();
    assert_eq!(
        suspended(state),
        chain,
        "one mana of the Prism's {{2}}: its window stands"
    );
    assert!(!state.objects[&pools].tapped);
    let tap = land_tap(state, pools).expect("the window offers the Pools' {T}: Add {C}");

    act(&mut runner, tap);
    assert!(
        runner.state().objects[&prism].tapped
            && matches!(
                runner.state().waiting_for,
                WaitingFor::ChooseManaColor { .. }
            ),
        "the Prism is paid and asks its color: {:?}",
        runner.state().waiting_for
    );
    act(
        &mut runner,
        GameAction::ChooseManaColor {
            choice: ManaChoice::SingleColor(ManaType::White),
            count: 1,
        },
    );
    // The Prism's mana pays the Pools' {1}; X is 1, so the Pools adds one of {W} or {U}.
    let combination = legal_actions(runner.state())
        .into_iter()
        .find(|action| matches!(action, GameAction::ChooseManaColor { .. }))
        .expect("the Pools asks which mana it adds");
    act(&mut runner, combination);
    let state = runner.state();
    assert!(matches!(state.waiting_for, WaitingFor::Priority { .. }));
    assert_eq!(pool_total(state), 1);
    assert_eq!(state.objects[&pools].counters.get(&storage), Some(&1));
}
