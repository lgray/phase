//! CR 732.2a: the confirmer's replay of a trace candidate, and the cover it certifies on.

use engine::analysis::decision_template::IterationCount;
use engine::analysis::loop_check::ShortcutResponse;
use engine::analysis::resource::{FodderCoverRefusal, ObjectGrowthVerdict, ResourceAxis};
use engine::game::engine::certify_object_growth_frames_for_tests;
use engine::game::period_confirm::{confirm_for_tests, performed_for_tests, OfferRefusal};
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::ability::{AbilityKind, ResolvedAbility, TargetRef};
use engine::types::actions::GameAction;
use engine::types::game_state::{
    CastPaymentMode, GameState, LoopDetectionMode, StackEntryKind, WaitingFor,
};
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaType;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

use crate::food_chain_board::{self, BoardCMember};
use crate::play_trace::{
    ability, activate, altar_board, cast, chooses_color, is_offer, names, place, settle,
};
use crate::support::shared_card_db;

fn act(runner: &mut GameRunner, action: GameAction) {
    let shown = format!("{action:?}");
    runner
        .act(action)
        .unwrap_or_else(|error| panic!("{shown} rejected: {error:?}"));
}

fn activated_index(state: &GameState, id: ObjectId) -> usize {
    state.objects[&id]
        .abilities
        .iter()
        .position(|a| a.kind == AbilityKind::Activated)
        .expect("an activated ability")
}

/// Passes priority until the stack is empty, answering every target prompt with `target`.
fn resolve_stack(runner: &mut GameRunner, target: ObjectId) {
    for _ in 0..40 {
        match &runner.state().waiting_for {
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => return,
            WaitingFor::Priority { .. } => act(runner, GameAction::PassPriority),
            WaitingFor::TargetSelection { .. } | WaitingFor::TriggerTargetSelection { .. } => act(
                runner,
                GameAction::ChooseTarget {
                    target: Some(TargetRef::Object(target)),
                },
            ),
            other => panic!("unexpected prompt {other:?}"),
        }
    }
    panic!("the stack did not empty");
}

/// CR 701.21a: Kiki-Jiki's "Sacrifice it at the beginning of the next end step" sacrifices the
/// copy only while its creator controls it; Ray of Command's controller keeps it.
#[test]
fn a_control_changed_kiki_copy_survives_its_delayed_sacrifice() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let kiki = scenario.add_real_card(P0, "Kiki-Jiki, Mirror Breaker", Zone::Battlefield, db);
    let bears = scenario.add_real_card(P0, "Grizzly Bears", Zone::Battlefield, db);
    let ray = scenario.add_real_card(P1, "Ray of Command", Zone::Hand, db);
    for _ in 0..4 {
        scenario.add_real_card(P1, "Island", Zone::Battlefield, db);
    }
    let mut runner = scenario.build();
    let index = activated_index(runner.state(), kiki);
    act(
        &mut runner,
        GameAction::ActivateAbility {
            source_id: kiki,
            ability_index: index,
        },
    );
    resolve_stack(&mut runner, bears);
    let copy = runner
        .state()
        .battlefield
        .iter()
        .copied()
        .find(|id| runner.state().objects[id].is_token)
        .expect("Kiki-Jiki made a copy");
    assert_eq!(runner.state().delayed_triggers.len(), 1);

    act(&mut runner, GameAction::PassPriority);
    assert!(matches!(runner.state().waiting_for, WaitingFor::Priority { player } if player == P1));
    let card_id = runner.state().objects[&ray].card_id;
    act(
        &mut runner,
        GameAction::CastSpell {
            object_id: ray,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        },
    );
    resolve_stack(&mut runner, copy);
    assert_eq!(runner.state().objects[&copy].controller, P1);

    for _ in 0..40 {
        let state = runner.state();
        if state.phase == Phase::End
            && state.delayed_triggers.is_empty()
            && !state
                .stack
                .iter()
                .any(|e| matches!(e.kind, StackEntryKind::TriggeredAbility { .. }))
        {
            break;
        }
        let action = match state.waiting_for {
            WaitingFor::DeclareAttackers { .. } => GameAction::DeclareAttackers {
                attacks: vec![],
                bands: vec![],
            },
            _ => GameAction::PassPriority,
        };
        act(&mut runner, action);
    }
    let state = runner.state();
    assert!(
        state.phase == Phase::End && state.delayed_triggers.is_empty() && state.stack.is_empty(),
        "reach: the delayed sacrifice resolved in the end step"
    );
    assert!(state.battlefield.contains(&copy));
    assert_eq!(state.objects[&copy].controller, P1);
    assert!(state.players.iter().all(|p| !p.graveyard.contains(&copy)));
}

/// The confirmer's verdict on the latest span the trace names at `state`.
fn latest_verdict(state: &GameState) -> Result<Vec<String>, OfferRefusal> {
    confirm_for_tests(state)
        .pop()
        .expect("the trace names a span")
        .1
}

/// Kiki-Jiki, Mirror Breaker ("{T}: Create a token that's a copy of target nonlegendary creature
/// you control, except it has haste. Sacrifice it at the beginning of the next end step.") beside
/// `copied`, whose copies' enters trigger untaps Kiki-Jiki.
fn kiki_copy_board(copied: &str) -> Option<(GameRunner, ObjectId, ObjectId)> {
    let db = shared_card_db()?;
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let kiki = scenario.add_real_card(P0, "Kiki-Jiki, Mirror Breaker", Zone::Battlefield, db);
    let copied = scenario.add_real_card(P0, copied, Zone::Battlefield, db);
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Mountain", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    Some((runner, kiki, copied))
}

/// One Kiki-Jiki cycle, declining an offer it meets; what the offered period's replay performs.
fn kiki_copy_cycle(
    runner: &mut GameRunner,
    kiki: ObjectId,
    copied: ObjectId,
) -> Option<Result<Vec<String>, OfferRefusal>> {
    let index = ability(runner.state(), kiki, false);
    activate(runner, kiki, index);
    let score = |action: &GameAction| match action {
        GameAction::ChooseTarget {
            target: Some(TargetRef::Object(id)),
        } if *id == kiki => 3,
        GameAction::ChooseTarget {
            target: Some(TargetRef::Object(id)),
        } if *id == copied => 2,
        GameAction::SelectModes { indices } if indices == &[0] => 2,
        GameAction::DecideOptionalEffect { accept: true } => 2,
        // Pestermite's "tap or untap": the untap branch.
        GameAction::ChooseBranch { index: 1 } => 2,
        _ => 0,
    };
    settle(runner, &score);
    let performed = is_offer(runner.state()).then(|| {
        let performed = performed_for_tests(runner.state()).expect("an offered span");
        act(runner, GameAction::DeclineShortcut);
        settle(runner, &score);
        performed
    });
    assert!(
        !runner.state().objects[&kiki].tapped,
        "reach: the copy's trigger untapped Kiki-Jiki"
    );
    performed
}

fn grown_not_inert(verdict: &Result<Vec<String>, OfferRefusal>) -> bool {
    matches!(
        verdict,
        Err(OfferRefusal::Cover(ObjectGrowthVerdict::FodderGrowth(pairs)))
            if pairs.iter().all(|refusals| refusals.contains(&FodderCoverRefusal::GrownNotInert))
    )
}

/// CR 702.8a + CR 702.10c: Kiki-Jiki copying Deceiver Exarch ("Flash … When this creature enters,
/// choose one — • Untap target permanent you control. …") confirms, its copies' flash and haste
/// and their delayed "sacrifice it" each grown; Pestermite's copies ("Flash Flying When this
/// creature enters, you may tap or untap target permanent.") carry flying and are not inert.
#[test]
fn kiki_jiki_confirms_with_deceiver_exarch_and_not_with_pestermite() {
    let Some((mut runner, kiki, exarch)) = kiki_copy_board("Deceiver Exarch") else {
        return;
    };
    let performed = (0..4).find_map(|_| kiki_copy_cycle(&mut runner, kiki, exarch));
    assert!(
        matches!(&performed, Some(Ok(performed)) if crate::loop_period_performs::same_up_to_rotation(
            performed,
            &["Deceiver Exarch"]
        )),
        "{performed:?}"
    );

    let Some((mut runner, kiki, pestermite)) = kiki_copy_board("Pestermite") else {
        return;
    };
    for cycle in 0..4 {
        assert_eq!(
            kiki_copy_cycle(&mut runner, kiki, pestermite),
            None,
            "cycle {cycle}"
        );
    }
    let verdict = latest_verdict(runner.state());
    assert!(grown_not_inert(&verdict), "{verdict:?}");
}

/// Phyrexian Altar recurring Gravecrawler beside Samwise Gamgee ("Whenever another nontoken
/// creature you control enters, create a Food token.") grows Foods, whose "{2}, {T}, Sacrifice
/// this token: You gain 3 life." keeps them from being inert.
#[test]
fn an_altar_loop_growing_foods_is_refused_at_the_cover() {
    let Some(db) = shared_card_db() else { return };
    let (mut runner, altar, gravecrawler) = altar_board(Some("Samwise Gamgee"), db);
    let score = |action: &GameAction| {
        2 * names(&[gravecrawler])(action) + chooses_color(ManaType::Black)(action)
    };
    for cycle in 0..3 {
        cast(&mut runner, gravecrawler, vec![], CastPaymentMode::Auto);
        settle(&mut runner, &score);
        let index = ability(runner.state(), altar, true);
        activate(&mut runner, altar, index);
        settle(&mut runner, &score);
        assert!(!is_offer(runner.state()), "cycle {cycle}");
    }
    let foods = runner
        .state()
        .battlefield
        .iter()
        .filter(|id| runner.state().objects[id].name == "Food")
        .count();
    assert_eq!(foods, 3, "reach: one Food per entry");
    let verdict = latest_verdict(runner.state());
    assert!(grown_not_inert(&verdict), "{verdict:?}");
}

/// Kiki-Jiki copying Deceiver Exarch, three cycles after the first: three priority frames.
fn kiki_exarch_frames() -> Option<(ObjectId, [GameState; 3])> {
    let (mut runner, kiki, exarch) = kiki_copy_board("Deceiver Exarch")?;
    let mut frames = Vec::new();
    for _ in 0..4 {
        kiki_copy_cycle(&mut runner, kiki, exarch);
        frames.push(runner.state().clone());
    }
    let [_, a, b, c]: [GameState; 4] = frames.try_into().ok()?;
    Some((kiki, [a, b, c]))
}

/// CR 603.7a + CR 603.7c: the newest "sacrifice it" delayed trigger is growth only while it acts
/// on the grown copy alone through a reference to its snapshotted target; each altered form stays
/// in the compared remainder.
#[test]
fn a_delayed_trigger_is_stripped_only_while_it_acts_on_grown_objects_alone() {
    use engine::types::ability::{Effect, QuantityExpr, TargetFilter, TypedFilter};

    let Some((kiki, frames)) = kiki_exarch_frames() else {
        return;
    };
    let verdict = |frames: &[GameState; 3]| {
        certify_object_growth_frames_for_tests([&frames[0], &frames[1], &frames[2]], &[], P0)
    };
    assert!(verdict(&frames).certifies(), "{:?}", verdict(&frames));

    let effect = |json: serde_json::Value| -> Effect { serde_json::from_value(json).unwrap() };
    type Alteration = Box<dyn Fn(&mut ResolvedAbility)>;
    let alterations: Vec<(&str, Alteration)> = vec![
        (
            "references Kiki-Jiki",
            Box::new(move |a| a.targets = vec![TargetRef::Object(kiki)]),
        ),
        ("references nothing", Box::new(|a| a.targets.clear())),
        (
            "puts it onto the battlefield",
            Box::new(move |a| {
                a.effect = effect(serde_json::json!({"type": "ChangeZone",
                    "destination": "Battlefield", "target": {"type": "LastCreated"}}))
            }),
        ),
        (
            "puts it into its library",
            Box::new(move |a| {
                a.effect = effect(serde_json::json!({"type": "Bounce",
                    "destination": "Library", "target": {"type": "LastCreated"}}))
            }),
        ),
        (
            "sacrifices through a filter",
            Box::new(|a| {
                a.effect = Effect::Sacrifice {
                    target: TargetFilter::Typed(TypedFilter::creature()),
                    count: QuantityExpr::Fixed { value: 1 },
                    min_count: 0,
                }
            }),
        ),
    ];
    for (what, alter) in alterations {
        let mut altered = frames.clone();
        let newest = altered[2]
            .delayed_triggers
            .last_mut()
            .expect("reach: the latest cycle's delayed sacrifice");
        assert!(matches!(newest.ability.effect, Effect::Sacrifice { .. }));
        alter(&mut newest.ability);
        let ObjectGrowthVerdict::FodderGrowth([first, second]) = verdict(&altered) else {
            panic!("{what}: {:?}", verdict(&altered));
        };
        assert!(first.is_empty(), "{what}: {first:?}");
        assert!(
            second.contains(&FodderCoverRefusal::NonObjectRemainder),
            "{what}: {second:?}"
        );
    }
}

/// CR 705.1 + CR 732.2a: Food Chain recasting Eternal Scourge beside Mirror March ("Whenever a
/// nontoken creature you control enters, flip a coin until you lose a flip. …") draws a random
/// outcome inside the period, so the replay refuses it.
#[test]
fn a_period_that_flips_a_coin_is_refused_as_random() {
    let Some(db) = shared_card_db() else { return };
    let member = BoardCMember::C1EternalScourge;
    let Some(mut board) = food_chain_board::build(member) else {
        return;
    };
    place(board.runner.state_mut(), P0, "Mirror March", db);
    board.runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    for _ in 0..2 {
        food_chain_board::exile_with_food_chain(
            &mut board.runner,
            board.food_chain,
            board.creature,
            member.mana(),
        );
        food_chain_board::cast_spell(&mut board.runner, board.creature).expect("cast from exile");
        settle(&mut board.runner, &|_| 0);
        assert_eq!(
            board.runner.state().objects[&board.creature].zone,
            Zone::Battlefield,
            "reach: Eternal Scourge resolved"
        );
    }
    let verdict = latest_verdict(board.runner.state());
    assert!(
        matches!(verdict, Err(OfferRefusal::Randomness)),
        "{verdict:?}"
    );
}

/// Phyrexian Altar ("Sacrifice a creature: Add one mana of any color.") beside `creature` in hand,
/// eight Swamps and Mountains, and `extra`; the creature is cast and sacrificed once for `color`.
fn altar_sacrifices_once(
    creature: &str,
    extra: Option<&str>,
    color: ManaType,
) -> Option<GameRunner> {
    let db = shared_card_db()?;
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let altar = scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    let creature = scenario.add_real_card(P0, creature, Zone::Hand, db);
    if let Some(extra) = extra {
        scenario.add_real_card(P0, extra, Zone::Battlefield, db);
    }
    for land in ["Swamp", "Mountain"] {
        for _ in 0..8 {
            scenario.add_real_card(P0, land, Zone::Battlefield, db);
        }
    }
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Swamp", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let score = |action: &GameAction| 2 * names(&[creature])(action) + chooses_color(color)(action);
    cast(&mut runner, creature, vec![], CastPaymentMode::Auto);
    settle(&mut runner, &score);
    let index = ability(runner.state(), altar, true);
    activate(&mut runner, altar, index);
    settle(&mut runner, &score);
    Some(runner)
}

/// CR 400.7: Hundred-Battle Veteran ("You may cast this card from your graveyard. If you do, it
/// enters with a finality counter on it.") reached the graveyard when sacrificed after a cast from
/// hand; the replay casts it from the graveyard, and the same sacrifice exiles it.
#[test]
fn a_sacrifice_arriving_elsewhere_than_recorded_is_refused() {
    let Some(runner) = altar_sacrifices_once("Hundred-Battle Veteran", None, ManaType::Black)
    else {
        return;
    };
    let veteran = runner.state().players[0]
        .graveyard
        .iter()
        .copied()
        .find(|id| runner.state().objects[id].name == "Hundred-Battle Veteran");
    assert!(
        veteran.is_some(),
        "reach: the sacrifice put it into the graveyard"
    );
    let verdict = latest_verdict(runner.state());
    assert!(
        matches!(verdict, Err(OfferRefusal::ArrivalDiverged)),
        "{verdict:?}"
    );
}

/// CR 614.6: under Rest in Peace ("If a card or token would be put into a graveyard from anywhere,
/// exile it instead.") the sacrificed Squee, the Immortal ("You may cast this card from your
/// graveyard or from exile.") arrives in exile on both sides, so the replay reaches the cover.
#[test]
fn a_replaced_sacrifice_arriving_where_recorded_is_admitted() {
    let Some(runner) =
        altar_sacrifices_once("Squee, the Immortal", Some("Rest in Peace"), ManaType::Red)
    else {
        return;
    };
    let squee_exiled = runner
        .state()
        .objects
        .values()
        .any(|o| o.name == "Squee, the Immortal" && o.zone == Zone::Exile);
    assert!(
        squee_exiled,
        "reach: Rest in Peace exiled the sacrificed Squee"
    );
    let verdict = latest_verdict(runner.state());
    assert!(
        !matches!(
            verdict,
            Err(OfferRefusal::ArrivalDiverged
                | OfferRefusal::IllegalReplayedPlay
                | OfferRefusal::UnanswerablePrompt
                | OfferRefusal::NoRecurrence
                | OfferRefusal::Randomness
                | OfferRefusal::Fragmented { .. })
        ),
        "{verdict:?}"
    );
}

/// Basalt Monolith ("{T}: Add {C}{C}{C}. {3}: Untap this artifact.") enchanted by Power Artifact,
/// tapped and untapped `cycles` times beside Llanowar Elves, six Mountains, and five Mountains in
/// hand; with `helm`, each cycle also activates Coral Helm ("{3}, Discard a card at random: Target
/// creature gets +2/+2 until end of turn.") on the Elves.
fn basalt_cycles(pyromancy: bool, cycles: usize) -> Option<GameRunner> {
    let db = shared_card_db()?;
    let mut rig = crate::loop_shortcut_mana_engine::setup(true, LoopDetectionMode::Interactive, db);
    let basalt = rig.basalt;
    let state = rig.runner.state_mut();
    let elves = place(state, P0, "Llanowar Elves", db);
    let pyromancy = pyromancy.then(|| place(state, P0, "Pyromancy", db));
    for _ in 0..6 {
        place(state, P0, "Mountain", db);
    }
    for _ in 0..5 {
        let face = db.get_face_by_name("Mountain").expect("card in fixture");
        let id = engine::game::deck_loading::create_object_from_card_face(state, face, P0);
        engine::game::zones::remove_from_zone(state, id, Zone::Library, P0);
        engine::game::zones::add_to_zone(state, id, Zone::Hand, P0);
        state.objects.get_mut(&id).expect("created").zone = Zone::Hand;
    }
    let runner = &mut rig.runner;
    let mana = ability(runner.state(), basalt, true);
    let untap = ability(runner.state(), basalt, false);
    for _ in 0..cycles {
        if is_offer(runner.state()) {
            break;
        }
        activate(runner, basalt, mana);
        settle(runner, &|_| 0);
        if let Some(pyromancy) = pyromancy {
            let index = ability(runner.state(), pyromancy, false);
            activate(runner, pyromancy, index);
            settle(runner, &names(&[elves]));
        }
        activate(runner, basalt, untap);
        settle(runner, &|_| 0);
    }
    Some(rig.runner)
}

/// CR 701.9b + CR 732.2a: a period that activates Pyromancy discards a card at random as its
/// cost, so the replay refuses it at that draw, before any cover; the same Basalt Monolith period
/// without it is offered.
#[test]
fn a_period_paying_a_random_discard_cost_is_refused_as_random() {
    let Some(plain) = basalt_cycles(false, 2) else {
        return;
    };
    assert!(
        is_offer(plain.state()),
        "reach: the plain Basalt Monolith period is offered"
    );
    let random = basalt_cycles(true, 2).expect("the fixture holds Pyromancy");
    let state = random.state();
    assert!(
        state.players[0].graveyard.len() >= 2,
        "reach: each Pyromancy activation discarded a card"
    );
    let verdicts = confirm_for_tests(state);
    assert!(
        verdicts
            .iter()
            .any(|(_, verdict)| matches!(verdict, Err(OfferRefusal::Randomness))),
        "{verdicts:?}"
    );
    assert!(!is_offer(state), "no offer");
}

/// Declares the standing offer at `count`, and every seat asked accepts it.
fn take(runner: &mut GameRunner, count: u32) {
    act(
        runner,
        GameAction::DeclareShortcut {
            count: IterationCount::Fixed(count),
            template: None,
        },
    );
    while matches!(
        runner.state().waiting_for,
        WaitingFor::RespondToShortcut { .. }
    ) {
        act(
            runner,
            GameAction::RespondToShortcut {
                response: ShortcutResponse::Accept,
            },
        );
    }
}

fn marks_mana(state: &GameState) -> bool {
    state.unbounded_resources.get(&P0).is_some_and(|axes| {
        axes.iter()
            .any(|axis| matches!(axis, ResourceAxis::Mana(_)))
    })
}

/// CR 106.6 + CR 732.2c: Food Chain's mana may be spent only to cast creature spells, so a take of
/// the Food Chain + Eternal Scourge period performs its cycles and leaves that restricted mana,
/// while Basalt Monolith's unrestricted period takes the ∞ mark.
#[test]
fn a_take_whose_period_adds_restricted_mana_performs_it() {
    let member = BoardCMember::C1EternalScourge;
    let Some(mut board) = food_chain_board::build(member) else {
        return;
    };
    board.runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    for _ in 0..3 {
        if is_offer(board.runner.state()) {
            break;
        }
        food_chain_board::exile_with_food_chain(
            &mut board.runner,
            board.food_chain,
            board.creature,
            member.mana(),
        );
        food_chain_board::cast_spell(&mut board.runner, board.creature).expect("cast from exile");
        settle(&mut board.runner, &|_| 0);
    }
    assert!(
        is_offer(board.runner.state()),
        "reach: the Food Chain period is offered"
    );
    let pool = |state: &GameState, restricted: bool| {
        state.players[0]
            .mana_pool
            .units()
            .filter(|unit| unit.restrictions.is_empty() != restricted)
            .count()
    };
    let before = pool(board.runner.state(), true);
    take(&mut board.runner, 3);
    let state = board.runner.state();
    assert!(
        pool(state, true) >= before + 3,
        "each performed cycle nets one restricted mana: {before} -> {}",
        pool(state, true)
    );
    assert_eq!(pool(state, false), 0, "no unrestricted mana is added");
    assert!(!marks_mana(state), "the restricted period takes no ∞ mark");

    let mut basalt = basalt_cycles(false, 2).expect("the fixture holds Basalt Monolith");
    assert!(
        is_offer(basalt.state()),
        "reach: the Basalt Monolith period is offered"
    );
    take(&mut basalt, 3);
    assert!(
        marks_mana(basalt.state()),
        "the unrestricted period takes the ∞ mark"
    );
}
