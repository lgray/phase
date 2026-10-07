//! CR 732.2a: the confirmer's replay of a trace candidate, and the cover it certifies on.

use engine::analysis::decision_template::IterationCount;
use engine::analysis::loop_check::{OfferRoad, ShortcutResponse};
use engine::analysis::resource::{FodderCoverRefusal, ObjectGrowthVerdict, ResourceAxis};
use engine::game::effects::attach::attach_to;
use engine::game::engine::certify_object_growth_frames_for_tests;
use engine::game::perf_counters::play_trace_counters;
use engine::game::period_confirm::{confirm_for_tests, performed_for_tests, OfferRefusal};
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::game::{play_trace_view, NamingCause};
use engine::types::ability::{
    AbilityKind, DelayedTriggerCondition, Effect, ResolvedAbility, TargetRef,
};
use engine::types::actions::GameAction;
use engine::types::game_state::{
    CastPaymentMode, GameState, LoopDetectionMode, ManaChoice, StackEntryKind, WaitingFor,
};
use engine::types::identifiers::ObjectId;
use engine::types::keywords::Keyword;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
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
    kiki_board(P0, copied, None)
}

/// [`kiki_copy_board`] on P0's turn with Kiki-Jiki and `copied` under `seat`'s control, and Soul
/// Warden ("Whenever another creature enters, you gain 1 life.") under `warden`'s.
fn kiki_board(
    seat: PlayerId,
    copied: &str,
    warden: Option<PlayerId>,
) -> Option<(GameRunner, ObjectId, ObjectId)> {
    let db = shared_card_db()?;
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let kiki = scenario.add_real_card(seat, "Kiki-Jiki, Mirror Breaker", Zone::Battlefield, db);
    let copied = scenario.add_real_card(seat, copied, Zone::Battlefield, db);
    if let Some(warden) = warden {
        scenario.add_real_card(warden, "Soul Warden", Zone::Battlefield, db);
    }
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Mountain", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    Some((runner, kiki, copied))
}

/// Kiki-Jiki's answers: Kiki-Jiki untapped by the copy of `copied` it targets.
fn kiki_score(kiki: ObjectId, copied: ObjectId) -> impl Fn(&GameAction) -> i32 {
    move |action| match action {
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
    }
}

fn kiki_activates(runner: &mut GameRunner, kiki: ObjectId, score: &dyn Fn(&GameAction) -> i32) {
    let index = ability(runner.state(), kiki, false);
    activate(runner, kiki, index);
    settle(runner, score);
}

/// One Kiki-Jiki cycle, declining an offer it meets; what the offered period's replay performs.
fn kiki_copy_cycle(
    runner: &mut GameRunner,
    kiki: ObjectId,
    copied: ObjectId,
) -> Option<Result<Vec<String>, OfferRefusal>> {
    let score = kiki_score(kiki, copied);
    kiki_activates(runner, kiki, &score);
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
/// hand; with `pyromancy`, each cycle also activates Pyromancy at the Elves.
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

/// [`take`], then the passes to the step end.
fn take_to_step_end(runner: &mut GameRunner, count: u32) {
    take(runner, count);
    take_to_step_end_after_take(runner, count);
}

/// Every seat passes until the step ends, where a collapse prompt is answered `count`.
fn take_to_step_end_after_take(runner: &mut GameRunner, count: u32) {
    let phase = runner.state().phase;
    while matches!(runner.state().waiting_for, WaitingFor::Priority { .. })
        && runner.state().phase == phase
    {
        act(runner, GameAction::PassPriority);
    }
    if matches!(
        runner.state().waiting_for,
        WaitingFor::PayAmountChoice { .. }
    ) {
        act(runner, GameAction::SubmitPayAmount { amount: count });
    }
}

fn marks_tokens(state: &GameState, seat: PlayerId) -> bool {
    state
        .unbounded_resources
        .get(&seat)
        .is_some_and(|axes| axes.contains(&ResourceAxis::TokensCreated))
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
    let Some(mut board) = board_c_offered(BoardCMember::C1EternalScourge) else {
        return;
    };
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

/// Board C driven until its period is offered, at most three cycles.
pub(crate) fn board_c_offered(member: BoardCMember) -> Option<food_chain_board::FoodChainBoard> {
    let mut board = food_chain_board::build(member)?;
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
    Some(board)
}

fn road(state: &GameState) -> Option<OfferRoad> {
    match &state.waiting_for {
        WaitingFor::LoopShortcut { road, .. } => Some(*road),
        _ => None,
    }
}

fn incarnations(state: &GameState, ids: &[ObjectId]) -> u64 {
    ids.iter().map(|id| state.objects[id].incarnation).sum()
}

/// CR 732.2a + CR 732.2c: each Board C member is offered on the recorded road, and a take of it
/// moves the creature through exile and back once per cycle (CR 400.7).
#[test]
fn board_c_members_are_offered_on_the_recorded_road_and_taken() {
    for member in [
        BoardCMember::C1EternalScourge,
        BoardCMember::C2SqueeTheImmortal,
    ] {
        let Some(mut board) = board_c_offered(member) else {
            return;
        };
        assert_eq!(
            road(board.runner.state()),
            Some(OfferRoad::RecordedPeriod),
            "{member:?}"
        );
        let before = incarnations(board.runner.state(), &[board.creature]);
        take(&mut board.runner, 3);
        let state = board.runner.state();
        assert!(
            matches!(state.waiting_for, WaitingFor::Priority { .. }),
            "{member:?}: {}",
            state.waiting_for.variant_name()
        );
        let after = incarnations(state, &[board.creature]);
        assert!(
            after > before,
            "{member:?}: the take moved the creature ({before} -> {after})"
        );
    }
}

/// Board C's take: its per-cycle history work and pool walk do not grow with its count.
#[test]
fn board_c_take_work_is_flat_per_cycle() {
    use crate::loop_shortcut::{
        assert_take_history_work_is_flat, assert_take_pool_walk_is_flat, TakeHistoryVector,
    };

    let metered = |n: u32| {
        let mut board =
            board_c_offered(BoardCMember::C1EternalScourge).expect("the fixture holds Board C");
        assert!(is_offer(board.runner.state()), "reach: Board C is offered");
        engine::game::perf_counters::reset();
        take(&mut board.runner, n);
        board.runner.state().clone()
    };
    assert_take_history_work_is_flat(
        32,
        &[
            TakeHistoryVector::JournalEntries,
            TakeHistoryVector::ProducedMana,
            TakeHistoryVector::SpentMana,
            TakeHistoryVector::BattlefieldEntries,
        ],
        &[],
        metered,
    );
    assert_take_pool_walk_is_flat(32, metered);
}

fn tokens(state: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    state
        .battlefield
        .iter()
        .copied()
        .filter(|id| state.objects[id].is_token && state.objects[id].controller == controller)
        .collect()
}

/// Passes to the next turn, declaring no attackers and ordering triggers as they are listed.
fn pass_to_next_turn(runner: &mut GameRunner) {
    let turn = runner.state().turn_number;
    for _ in 0..80 {
        let action = match &runner.state().waiting_for {
            _ if runner.state().turn_number != turn => return,
            WaitingFor::Priority { .. } => GameAction::PassPriority,
            WaitingFor::DeclareAttackers { .. } => GameAction::DeclareAttackers {
                attacks: vec![],
                bands: vec![],
            },
            WaitingFor::OrderTriggers { triggers, .. } => GameAction::OrderTriggers {
                order: (0..triggers.len()).collect(),
            },
            other => panic!("unexpected prompt {}", other.variant_name()),
        };
        act(runner, action);
    }
    panic!("the turn did not end");
}

/// CR 732.2a + CR 732.2c: Kiki-Jiki's first span refuses at its target prompt; the window after the
/// second activation offers the period on the recorded road, and the take makes what its cycles
/// would: each copy untapped with haste under its own "Sacrifice it at the beginning of the next
/// end step", its enters trigger already resolved, and none left after that step.
#[test]
fn kiki_jiki_is_refused_at_its_first_span_then_offered_and_taken() {
    let Some((mut runner, kiki, exarch)) = kiki_copy_board("Deceiver Exarch") else {
        return;
    };
    let score = kiki_score(kiki, exarch);
    kiki_activates(&mut runner, kiki, &score);
    assert!(!is_offer(runner.state()), "the first span is not offered");
    assert_eq!(
        latest_verdict(runner.state()),
        Err(OfferRefusal::UnanswerablePrompt)
    );
    kiki_activates(&mut runner, kiki, &score);
    assert_eq!(road(runner.state()), Some(OfferRoad::RecordedPeriod));
    let before = tokens(runner.state(), P0);
    take_to_step_end(&mut runner, 2);
    let state = runner.state();
    assert!(
        matches!(state.waiting_for, WaitingFor::Priority { .. }),
        "no copy's enters trigger is owed once the step has ended: {}",
        state.waiting_for.variant_name()
    );
    let minted: Vec<ObjectId> = tokens(state, P0)
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(minted.len(), 2, "the take made the copies it counted");
    for id in &minted {
        let copy = &state.objects[id];
        assert!(
            !copy.tapped && copy.keywords.contains(&Keyword::Haste),
            "{id:?}: tapped={} keywords={:?}",
            copy.tapped,
            copy.keywords
        );
        assert!(
            state.delayed_triggers.iter().any(|trigger| {
                trigger.condition == DelayedTriggerCondition::AtNextPhase { phase: Phase::End }
                    && matches!(trigger.ability.effect, Effect::Sacrifice { .. })
                    && trigger.ability.targets == [TargetRef::Object(*id)]
            }),
            "{id:?} has no delayed sacrifice: {:?}",
            state.delayed_triggers
        );
    }
    pass_to_next_turn(&mut runner);
    assert_eq!(
        tokens(runner.state(), P0),
        Vec::<ObjectId>::new(),
        "no copy outlives the end step"
    );
}

/// CR 732.1b: Sprout Swarm's Saprolings carry no keyword and no delayed trigger, so the take of
/// its period stands on the ∞ mark.
#[test]
fn a_take_of_a_period_growing_bare_tokens_stands_on_the_mark() {
    let Some((mut runner, sprout, _, _)) = sprout_swarm_board() else {
        return;
    };
    for _ in 0..3 {
        runner.cast(sprout).accept_optional().commit();
        settle(&mut runner, &|_| 0);
    }
    assert!(is_offer(runner.state()), "reach: the recast is offered");
    take(&mut runner, 3);
    let marked = runner.state().unbounded_resources.get(&P0);
    assert!(
        marked.is_some_and(|axes| axes.contains(&ResourceAxis::TokensCreated)),
        "{marked:?}"
    );
}

/// CR 732.2c + CR 111.2: Forbidden Orchard ("Whenever you tap this land for mana, target opponent
/// creates a 1/1 colorless Spirit creature token.") untapped by Voyaging Satyr ("{T}: Untap target
/// land.") under Freed from the Real ("{U}: Untap enchanted creature.") grows Spirits its opponent
/// controls, so the take performs the period: the opponent gets each Spirit, and nothing is marked
/// or minted for the player who looped.
#[test]
fn a_take_of_a_period_growing_an_opponents_tokens_gives_them_to_that_opponent() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let orchard = scenario.add_real_card(P0, "Forbidden Orchard", Zone::Battlefield, db);
    let satyr = scenario.add_real_card(P0, "Voyaging Satyr", Zone::Battlefield, db);
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Swamp", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let freed = place(runner.state_mut(), P0, "Freed from the Real", db);
    attach_to(runner.state_mut(), freed, satyr);
    let untaps = runner.state().objects[&freed]
        .abilities
        .iter()
        .rposition(|ability| ability.kind == AbilityKind::Activated)
        .expect("Freed from the Real's untap ability");
    let score = |action: &GameAction| match action {
        GameAction::ChooseTarget {
            target: Some(TargetRef::Object(id)),
        } if *id == orchard => 3,
        GameAction::ChooseTarget {
            target: Some(TargetRef::Player(P1)),
        } => 3,
        other => chooses_color(ManaType::Blue)(other),
    };
    for cycle in 0.. {
        assert!(cycle < 4, "no offer in four cycles");
        let (taps, untaps_land) = (
            ability(runner.state(), orchard, true),
            ability(runner.state(), satyr, false),
        );
        for (source, index) in [(orchard, taps), (satyr, untaps_land), (freed, untaps)] {
            if !is_offer(runner.state()) {
                activate(&mut runner, source, index);
                settle(&mut runner, &score);
            }
        }
        if is_offer(runner.state()) {
            break;
        }
    }
    let spirits = tokens(runner.state(), P1).len();
    assert!(spirits > 0, "reach: the period made the opponent a Spirit");
    take(&mut runner, 3);
    assert!(!marks_tokens(runner.state(), P0), "at the take");
    take_to_step_end_after_take(&mut runner, 3);
    let state = runner.state();
    assert!(!marks_tokens(state, P0), "at the step end");
    assert_eq!(tokens(state, P1).len(), spirits + 3);
    assert_eq!(tokens(state, P0), Vec::<ObjectId>::new());
}

/// CR 732.2c + CR 111.2: Dragonlair Spider ("Whenever an opponent casts a spell, create a 1/1 green
/// Insect creature token.") makes its controller an Insect each time the Altar + Gravecrawler
/// period recasts, so the take performs the period and the Spider's controller gets each Insect.
#[test]
fn a_take_of_a_period_feeding_an_opponents_token_trigger_gives_them_its_tokens() {
    let Some(db) = shared_card_db() else { return };
    let (mut runner, altar, gravecrawler) = altar_board(None, db);
    place(runner.state_mut(), P1, "Dragonlair Spider", db);
    let score = |action: &GameAction| {
        2 * names(&[gravecrawler])(action) + chooses_color(ManaType::Black)(action)
    };
    for cycle in 0.. {
        assert!(cycle < 4, "no offer in four cycles");
        cast(&mut runner, gravecrawler, vec![], CastPaymentMode::Auto);
        settle(&mut runner, &score);
        if is_offer(runner.state()) {
            break;
        }
        let index = ability(runner.state(), altar, true);
        activate(&mut runner, altar, index);
        settle(&mut runner, &score);
        if is_offer(runner.state()) {
            break;
        }
    }
    let insects = tokens(runner.state(), P1).len();
    assert!(insects > 0, "reach: the period made the opponent an Insect");
    take(&mut runner, 3);
    assert!(!marks_tokens(runner.state(), P0), "at the take");
    take_to_step_end_after_take(&mut runner, 3);
    let state = runner.state();
    assert!(!marks_tokens(state, P0), "at the step end");
    assert_eq!(tokens(state, P1).len(), insects + 3);
    assert_eq!(tokens(state, P0), Vec::<ObjectId>::new());
}

/// CR 732.2a: P1's Kiki-Jiki period on P0's turn is offered to P1, at P1's own priority, and never
/// to P0, who makes none of its plays; P1's decline returns to priority, and P1's take makes P1's
/// copies and marks nothing for P0. P0's Soul Warden triggering inside the period changes none of
/// it.
#[test]
fn a_period_is_offered_only_to_the_seat_that_made_its_plays() {
    for warden in [None, Some(P0)] {
        let Some((mut runner, kiki, exarch)) = kiki_board(P1, "Deceiver Exarch", warden) else {
            return;
        };
        let score = kiki_score(kiki, exarch);
        for activation in 0.. {
            if is_offer(runner.state()) {
                break;
            }
            assert!(activation < 4, "{warden:?}: no offer in four activations");
            assert!(
                matches!(runner.state().waiting_for, WaitingFor::Priority { player } if player == P0),
                "{warden:?}: {}",
                runner.state().waiting_for.variant_name()
            );
            act(&mut runner, GameAction::PassPriority);
            if is_offer(runner.state()) {
                break;
            }
            kiki_activates(&mut runner, kiki, &score);
        }
        let WaitingFor::LoopShortcut { proposer, .. } = runner.state().waiting_for else {
            unreachable!("the drive ends on an offer");
        };
        assert_eq!(proposer, P1, "{warden:?}");

        let mut declined = runner.state().clone();
        engine::game::engine::apply(&mut declined, P1, GameAction::DeclineShortcut)
            .expect("the proposer declines");
        assert!(
            matches!(declined.waiting_for, WaitingFor::Priority { .. }),
            "{warden:?}: {}",
            declined.waiting_for.variant_name()
        );

        let before = tokens(runner.state(), P1).len();
        take(&mut runner, 3);
        let state = runner.state();
        assert_eq!(tokens(state, P1).len(), before + 3, "{warden:?}");
        assert_eq!(tokens(state, P0), Vec::<ObjectId>::new(), "{warden:?}");
        assert_eq!(state.unbounded_resources.get(&P0), None, "{warden:?}");
    }
}

/// Grand Architect ("{U}: Target artifact creature becomes blue until end of turn. Tap an untapped
/// blue creature you control: Add {C}{C}. …") and Pili-Pala ("{2}, {Q}: Add one mana of any
/// color."), with Pili-Pala recolored first when `blue`, cycled until an offer; whether one came.
pub(crate) fn grand_architect_pili_pala(
    db: &engine::database::card_db::CardDatabase,
    blue: bool,
) -> (GameRunner, ObjectId, bool) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let architect = scenario.add_real_card(P0, "Grand Architect", Zone::Battlefield, db);
    let pili = scenario.add_real_card(P0, "Pili-Pala", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Island", Zone::Battlefield, db);
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let prefer_pili =
        |action: &GameAction| 2 * names(&[pili])(action) + chooses_color(ManaType::Blue)(action);
    if blue {
        let recolor = ability(runner.state(), architect, false);
        activate(&mut runner, architect, recolor);
        settle(&mut runner, &prefer_pili);
    }
    let (tap_blue, untap) = (
        ability(runner.state(), architect, true),
        ability(runner.state(), pili, true),
    );
    for _ in 0..3 {
        activate(&mut runner, architect, tap_blue);
        settle(&mut runner, &prefer_pili);
        if is_offer(runner.state()) {
            return (runner, pili, true);
        }
        let untapped = runner.act(GameAction::ActivateAbility {
            source_id: pili,
            ability_index: untap,
        });
        if untapped.is_err() {
            break;
        }
        settle(&mut runner, &prefer_pili);
    }
    (runner, pili, false)
}

/// Before the recolor Pili-Pala cannot be tapped for {C}{C}, so there is no cycle and no offer;
/// once it is blue, the window after the second tap offers the two-play period on the recorded
/// road, and the take adds the mana its cycles net.
#[test]
fn grand_architect_pili_pala_is_offered_only_once_pili_pala_is_blue() {
    let Some(db) = shared_card_db() else { return };
    let (runner, pili, offered) = grand_architect_pili_pala(db, false);
    assert!(!offered, "no offer before the recolor");
    assert!(
        !runner.state().objects[&pili].tapped,
        "Pili-Pala was never tapped"
    );

    let (mut runner, _, offered) = grand_architect_pili_pala(db, true);
    assert!(offered, "reach: the recolored board offers");
    assert_eq!(road(runner.state()), Some(OfferRoad::RecordedPeriod));
    let pool = |state: &GameState| state.players[0].mana_pool.units().count();
    let before = pool(runner.state());
    take(&mut runner, 3);
    assert!(
        pool(runner.state()) > before,
        "the take added the cycles' mana ({before} -> {})",
        pool(runner.state())
    );
}

/// CR 732.1b: on Food Chain + Eternal Scourge + Misthollow Griffin, the first window's span begins
/// at Food Chain's activation inside the Griffin's payment, which a replay from the priority frame
/// never makes, so the step ends first; the next cycle's window offers on the recorded road, and
/// the take moves the creatures through exile.
#[test]
fn griffin_board_is_refused_at_its_first_window_and_offered_at_the_next() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    let food_chain = scenario.add_real_card(P0, "Food Chain", Zone::Battlefield, db);
    let scourge = scenario.add_real_card(P0, "Eternal Scourge", Zone::Battlefield, db);
    let griffin = scenario.add_real_card(P0, "Misthollow Griffin", Zone::Exile, db);
    for seat in [P0, P1, PlayerId(2), PlayerId(3)] {
        for _ in 0..8 {
            scenario.add_real_card(seat, "Swamp", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let cast_paying_with_food_chain =
        |runner: &mut GameRunner, creature: ObjectId, other: ObjectId| {
            cast(runner, creature, vec![], CastPaymentMode::Manual);
            activate(runner, food_chain, 0);
            act(runner, GameAction::SelectCards { cards: vec![other] });
            act(
                runner,
                GameAction::ChooseManaColor {
                    choice: ManaChoice::SingleColor(ManaType::Blue),
                    count: 1,
                },
            );
            act(runner, GameAction::PassPriority);
            settle(runner, &|_| 0);
        };
    cast_paying_with_food_chain(&mut runner, griffin, scourge);
    assert!(!is_offer(runner.state()));
    assert_eq!(
        latest_verdict(runner.state()),
        Err(OfferRefusal::NoRecurrence)
    );
    cast_paying_with_food_chain(&mut runner, scourge, griffin);
    assert_eq!(road(runner.state()), Some(OfferRoad::RecordedPeriod));
    let before = incarnations(runner.state(), &[griffin, scourge]);
    take(&mut runner, 2);
    let after = incarnations(runner.state(), &[griffin, scourge]);
    assert!(
        after > before,
        "the take moved the creatures ({before} -> {after})"
    );
}

/// CR 732.1b + CR 704.5a: Phyrexian Altar recurring Gravecrawler is refused by name: with no
/// payoff nothing grows, Altar of the Brood's mill keeps the frames from covering, and Zulaport
/// Cutthroat's "each opponent loses 1 life" moves the opponents toward losing.
#[test]
fn an_altar_gravecrawler_period_is_refused_at_its_payoffs_stage() {
    let Some(db) = shared_card_db() else { return };
    let Some(board) = board_c_offered(BoardCMember::C2SqueeTheImmortal) else {
        return;
    };
    assert!(is_offer(board.runner.state()), "reach: Board C is offered");
    type Expected = fn(&Result<Vec<String>, OfferRefusal>) -> bool;
    let refused: [(Option<&str>, Expected); 3] = [
        (None, |v| *v == Err(OfferRefusal::NoAxis)),
        (Some("Altar of the Brood"), |v| {
            matches!(
                v,
                Err(OfferRefusal::Cover(
                    ObjectGrowthVerdict::ResourceRecurrence(false)
                ))
            )
        }),
        (Some("Zulaport Cutthroat"), |v| {
            *v == Err(OfferRefusal::LossAxis)
        }),
    ];
    for (payoff, expected) in refused {
        let (mut runner, altar, gravecrawler) = altar_board(payoff, db);
        let score = |action: &GameAction| {
            2 * names(&[gravecrawler])(action) + chooses_color(ManaType::Black)(action)
        };
        for cycle in 0..3 {
            cast(&mut runner, gravecrawler, vec![], CastPaymentMode::Auto);
            settle(&mut runner, &score);
            let index = ability(runner.state(), altar, true);
            activate(&mut runner, altar, index);
            settle(&mut runner, &score);
            assert!(!is_offer(runner.state()), "{payoff:?} cycle {cycle}");
        }
        let verdict = latest_verdict(runner.state());
        assert!(expected(&verdict), "{payoff:?}: {verdict:?}");
    }
}

/// CR 732.1b: once both rotations of the Altar + Gravecrawler period are named, each later cycle
/// asks and drives the same number of spans, every span a window names being confirmed afresh.
#[test]
fn an_altar_gravecrawler_meter_is_flat_once_both_rotations_are_named() {
    let Some(db) = shared_card_db() else { return };
    let (mut runner, altar, gravecrawler) = altar_board(None, db);
    let score = |action: &GameAction| {
        2 * names(&[gravecrawler])(action) + chooses_color(ManaType::Black)(action)
    };
    let mut per_cycle = Vec::new();
    for _ in 0..6 {
        let before = play_trace_counters();
        cast(&mut runner, gravecrawler, vec![], CastPaymentMode::Auto);
        settle(&mut runner, &score);
        let index = ability(runner.state(), altar, true);
        activate(&mut runner, altar, index);
        settle(&mut runner, &score);
        let run = play_trace_counters().since(before);
        per_cycle.push((run.confirm_asks, run.confirm_drives));
    }
    let rotations = play_trace_view(runner.state())
        .expect("a trace")
        .named
        .iter()
        .filter(|span| span.cause == NamingCause::Repeat)
        .count();
    assert!(rotations >= 2, "reach: both rotations are named");
    assert!(per_cycle[0].1 > 0, "reach: the first span is driven");
    assert_eq!(per_cycle[2..], [(1, 1); 4], "{per_cycle:?}");
}

/// CR 732.2a: Aetherflux Reservoir ("Whenever you cast a spell, you gain 1 life for each spell
/// you've cast this turn.") against two Eidolon of the Great Revel ("Whenever a player casts a
/// spell with mana value 3 or less, Eidolon of the Great Revel deals 2 damage to that player.")
/// costs the Altar + Gravecrawler period life until the turn's fifth cast, so the period refused
/// for want of an axis is offered by the third cycle.
#[test]
fn a_period_refused_for_no_axis_is_offered_once_it_nets_life() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let altar = scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Walking Corpse", Zone::Battlefield, db);
    let gravecrawler = scenario.add_real_card(P0, "Gravecrawler", Zone::Graveyard, db);
    for card in ["Swamp", "Aetherflux Reservoir"] {
        scenario.add_real_card(P0, card, Zone::Battlefield, db);
    }
    for _ in 0..2 {
        scenario.add_real_card(P1, "Eidolon of the Great Revel", Zone::Battlefield, db);
    }
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Swamp", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let score = |action: &GameAction| {
        2 * names(&[gravecrawler])(action) + chooses_color(ManaType::Black)(action)
    };
    let mut refused_for_no_axis = false;
    for _ in 0..3 {
        cast(&mut runner, gravecrawler, vec![], CastPaymentMode::Auto);
        settle(&mut runner, &score);
        if is_offer(runner.state()) {
            break;
        }
        let index = ability(runner.state(), altar, true);
        activate(&mut runner, altar, index);
        settle(&mut runner, &score);
        if is_offer(runner.state()) {
            break;
        }
        refused_for_no_axis |= latest_verdict(runner.state()) == Err(OfferRefusal::NoAxis);
    }
    assert!(
        refused_for_no_axis,
        "reach: the period was refused while it cost life"
    );
    assert_eq!(road(runner.state()), Some(OfferRoad::RecordedPeriod));
}

/// CR 732.2a + CR 614.1a: Burning-Tree Shaman ("Whenever a player activates an ability that isn't
/// a mana ability, this creature deals 1 damage to that player.") costs the Basalt Monolith +
/// Power Artifact period 1 life a cycle, until Angel's Grace ("... Until end of turn, damage that
/// would reduce your life total to less than 1 reduces it to 1 instead.") holds the total at 1; the
/// period refused while it costs life is offered from the first frame whose repeat costs none.
#[test]
fn a_period_costing_life_is_offered_once_a_floor_holds_the_life_total() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 5);
    let basalt = scenario.add_real_card(P0, "Basalt Monolith", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Plains", Zone::Battlefield, db);
    let grace = scenario.add_real_card(P0, "Angel's Grace", Zone::Hand, db);
    scenario.add_real_card(P1, "Burning-Tree Shaman", Zone::Battlefield, db);
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Wastes", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let power = place(runner.state_mut(), P0, "Power Artifact", db);
    attach_to(runner.state_mut(), power, basalt);
    cast(&mut runner, grace, vec![], CastPaymentMode::Auto);
    settle(&mut runner, &|_| 0);
    let mana = ability(runner.state(), basalt, true);
    let untap = ability(runner.state(), basalt, false);
    let life = |state: &GameState| state.players[0].life;
    let mut refused_at = Vec::new();
    for _ in 0..6 {
        activate(&mut runner, basalt, mana);
        settle(&mut runner, &|_| 0);
        if is_offer(runner.state()) {
            break;
        }
        activate(&mut runner, basalt, untap);
        settle(&mut runner, &|_| 0);
        if is_offer(runner.state()) {
            break;
        }
        if latest_verdict(runner.state()) == Err(OfferRefusal::NoAxis) {
            refused_at.push(life(runner.state()));
        }
    }
    assert_eq!(
        refused_at,
        [4, 3],
        "reach: the period was refused while it cost life"
    );
    assert_eq!(road(runner.state()), Some(OfferRoad::RecordedPeriod));
    assert_eq!(life(runner.state()), 2);
}

/// CR 732.1b: while Altar of the Brood ("Whenever another permanent you control enters, each
/// opponent mills a card.") has a library to mill, the Altar + Gravecrawler period does not come
/// round; once the library is empty it does, and Soul Warden ("Whenever another creature enters,
/// you gain 1 life.") makes it worth repeating, so the same plays are then offered.
#[test]
fn an_altar_gravecrawler_period_is_offered_once_the_milled_library_is_empty() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let altar = scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Walking Corpse", Zone::Battlefield, db);
    let gravecrawler = scenario.add_real_card(P0, "Gravecrawler", Zone::Graveyard, db);
    for payoff in ["Swamp", "Altar of the Brood", "Soul Warden"] {
        scenario.add_real_card(P0, payoff, Zone::Battlefield, db);
    }
    for (seat, cards) in [(P0, 10), (P1, 3)] {
        for _ in 0..cards {
            scenario.add_real_card(seat, "Swamp", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let score = |action: &GameAction| {
        2 * names(&[gravecrawler])(action) + chooses_color(ManaType::Black)(action)
    };
    let library = |state: &GameState| state.players[1].library.len();
    let mut refused_while_milling = false;
    for _ in 0..6 {
        cast(&mut runner, gravecrawler, vec![], CastPaymentMode::Auto);
        settle(&mut runner, &score);
        if is_offer(runner.state()) {
            break;
        }
        let index = ability(runner.state(), altar, true);
        activate(&mut runner, altar, index);
        settle(&mut runner, &score);
        if is_offer(runner.state()) {
            break;
        }
        refused_while_milling |= library(runner.state()) > 0
            && latest_verdict(runner.state())
                == Err(OfferRefusal::Cover(
                    ObjectGrowthVerdict::ResourceRecurrence(false),
                ));
    }
    assert!(
        refused_while_milling,
        "reach: the cover refused it mid-mill"
    );
    assert_eq!(road(runner.state()), Some(OfferRoad::RecordedPeriod));
    assert_eq!(library(runner.state()), 0);
}

/// CR 732.3: two seats each tapping and untapping their own Basalt Monolith under Power Artifact
/// ("Enchanted artifact's activated abilities cost {2} less to activate. …") name a span holding
/// both seats' plays, which is refused before any replay; nothing is offered.
#[test]
fn a_span_holding_two_seats_plays_is_refused_as_fragmented_without_a_drive() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let monoliths =
        [P0, P1].map(|seat| scenario.add_real_card(seat, "Basalt Monolith", Zone::Battlefield, db));
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Wastes", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    for (seat, monolith) in [P0, P1].into_iter().zip(monoliths) {
        let power = place(runner.state_mut(), seat, "Power Artifact", db);
        attach_to(runner.state_mut(), power, monolith);
    }
    let indices = monoliths.map(|monolith| {
        (
            ability(runner.state(), monolith, true),
            ability(runner.state(), monolith, false),
        )
    });
    let mut fragmented = None;
    for _ in 0..3 {
        for ((seat, monolith), (mana, untap)) in [P0, P1].into_iter().zip(monoliths).zip(indices) {
            assert!(
                matches!(runner.state().waiting_for, WaitingFor::Priority { player } if player == seat)
            );
            activate(&mut runner, monolith, mana);
            let before = play_trace_counters();
            activate(&mut runner, monolith, untap);
            let run = play_trace_counters().since(before);
            let last = play_trace_view(runner.state()).and_then(|view| view.named.last().copied());
            if let (Some(span), true, true) = (last, seat == P1, fragmented.is_none()) {
                let verdict = confirm_for_tests(runner.state())
                    .into_iter()
                    .find(|(named, _)| *named == span)
                    .map(|(_, verdict)| verdict);
                fragmented = Some((run.confirm_asks, run.confirm_drives, verdict));
            }
            act(&mut runner, GameAction::PassPriority);
        }
        settle(&mut runner, &|_| 0);
        assert!(!is_offer(runner.state()));
    }
    let view = play_trace_view(runner.state()).expect("a trace");
    assert!(
        view.named
            .iter()
            .any(|span| span.cause == NamingCause::Repeat),
        "reach: repeats name spans"
    );
    assert_eq!(
        fragmented,
        Some((
            1,
            0,
            Some(Err(OfferRefusal::Fragmented {
                seats: vec![P0, P1]
            }))
        ))
    );
}

/// Presence of Gond on Grizzly Bears with Intruder Alarm and Llanowar Elves; `extra` is a card in
/// P0's hand, or a Forest on the battlefield.
fn gond_board(
    db: &engine::database::card_db::CardDatabase,
    extra: Option<&str>,
) -> (GameRunner, ObjectId, ObjectId, Option<ObjectId>) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let bears = scenario.add_real_card(P0, "Grizzly Bears", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Intruder Alarm", Zone::Battlefield, db);
    let elves = scenario.add_real_card(P0, "Llanowar Elves", Zone::Battlefield, db);
    let extra = extra.map(|name| {
        let zone = if name == "Forest" {
            Zone::Battlefield
        } else {
            Zone::Hand
        };
        scenario.add_real_card(P0, name, zone, db)
    });
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Forest", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let gond = place(runner.state_mut(), P0, "Presence of Gond", db);
    attach_to(runner.state_mut(), gond, bears);
    (runner, bears, elves, extra)
}

/// One Gond cycle: Llanowar Elves taps for {G}, `between` runs, the Bears make an Elf, and the
/// stack settles. The meter's asks at the Bears' window.
fn gond_cycle(
    runner: &mut GameRunner,
    bears: ObjectId,
    elves: ObjectId,
    between: impl FnOnce(&mut GameRunner),
) -> u64 {
    let green = chooses_color(ManaType::Green);
    let tap = ability(runner.state(), elves, true);
    activate(runner, elves, tap);
    settle(runner, &green);
    between(runner);
    let before = play_trace_counters();
    let make = ability(runner.state(), bears, false);
    activate(runner, bears, make);
    settle(runner, &green);
    play_trace_counters().since(before).confirm_asks
}

/// CR 732.2a + CR 117.3c: the Gond board's first span, Llanowar Elves' tap and Mobilize ("Untap
/// all creatures you control."), is refused, and the retry offers the Bears' span at the same
/// window after two asks; without Mobilize the first span offers and nothing is retried.
#[test]
fn the_gond_board_offers_the_bears_span_after_refusing_the_first() {
    let Some(db) = shared_card_db() else { return };
    let (mut runner, bears, elves, mobilize) = gond_board(db, Some("Mobilize"));
    let mobilize = mobilize.expect("Mobilize");
    let asks = gond_cycle(&mut runner, bears, elves, |runner| {
        cast(runner, mobilize, vec![], CastPaymentMode::Auto);
        settle(runner, &|_| 0);
    });
    let view = play_trace_view(runner.state()).expect("a trace");
    let offered = view.offered.expect("the Bears' span is offered");
    let first = view
        .named
        .iter()
        .find(|span| span.end == offered.end)
        .copied()
        .expect("the window's first span");
    assert!(first.start < offered.start, "{:?}", view.named);
    assert_eq!(asks, 2);

    let (mut runner, bears, elves, _) = gond_board(db, None);
    let asks = gond_cycle(&mut runner, bears, elves, |_| {});
    assert!(
        is_offer(runner.state()),
        "reach: the board without Mobilize offers"
    );
    assert_eq!(asks, 1);
}

/// Hostiles on the Gond board without Mobilize: Giant Growth ("Target creature gets +3/+3 until
/// end of turn.") cast while the Bears' ability is on the stack is not replayable, so that window
/// offers nothing and the next cycle's does; a Forest tapped between the Elves and the Bears is
/// left out of the span the retry offers.
#[test]
fn the_gond_board_hostiles_are_offered_where_the_period_recurs() {
    let Some(db) = shared_card_db() else { return };
    let mut canary =
        crate::loop_shortcut_activation::setup(true, true, LoopDetectionMode::Interactive, db);
    let index =
        crate::loop_shortcut_activation::token_ability_index(canary.runner.state(), canary.host)
            .expect("Gond's granted ability");
    crate::loop_shortcut_activation::activate_and_drive(&mut canary.runner, canary.host, index);
    assert!(is_offer(canary.runner.state()), "reach: the canary offers");

    let (mut runner, bears, elves, growth) = gond_board(db, Some("Giant Growth"));
    let growth = growth.expect("Giant Growth");
    let green = chooses_color(ManaType::Green);
    let tap = ability(runner.state(), elves, true);
    activate(&mut runner, elves, tap);
    settle(&mut runner, &green);
    let make = ability(runner.state(), bears, false);
    activate(&mut runner, bears, make);
    cast(&mut runner, growth, vec![bears], CastPaymentMode::Auto);
    settle(&mut runner, &green);
    assert!(
        !is_offer(runner.state()),
        "the Giant Growth window offers nothing"
    );
    assert!(
        confirm_for_tests(runner.state())
            .iter()
            .all(|(_, verdict)| *verdict == Err(OfferRefusal::IllegalReplayedPlay)),
        "{:?}",
        confirm_for_tests(runner.state())
    );
    gond_cycle(&mut runner, bears, elves, |_| {});
    assert!(is_offer(runner.state()), "the next cycle's window offers");

    let (mut runner, bears, elves, forest) = gond_board(db, Some("Forest"));
    let forest = forest.expect("Forest");
    gond_cycle(&mut runner, bears, elves, |runner| {
        activate(runner, forest, 0);
        settle(runner, &green);
    });
    let view = play_trace_view(runner.state()).expect("a trace");
    let offered = view.offered.expect("the retry offers");
    assert!(view.named[0].start < offered.start, "{:?}", view.named);
}

/// Witherbloom, the Balancer ("Instant and sorcery spells you cast have affinity for creatures.")
/// with Sprout Swarm ("Convoke", "Buyback {3}", "Create a 1/1 green Saproling creature token.") in
/// hand and nine Forests; the opponent holds Murder ("Destroy target creature.") and three Swamps.
fn sprout_swarm_board() -> Option<(GameRunner, ObjectId, ObjectId, ObjectId)> {
    let db = shared_card_db()?;
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let witherbloom =
        scenario.add_real_card(P0, "Witherbloom, the Balancer", Zone::Battlefield, db);
    let sprout = scenario.add_real_card(P0, "Sprout Swarm", Zone::Hand, db);
    let murder = scenario.add_real_card(P1, "Murder", Zone::Hand, db);
    for _ in 0..9 {
        scenario.add_real_card(P0, "Forest", Zone::Battlefield, db);
    }
    for _ in 0..3 {
        scenario.add_real_card(P1, "Swamp", Zone::Battlefield, db);
    }
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Forest", Zone::Library, db);
        }
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    Some((runner, sprout, witherbloom, murder))
}

fn saprolings(state: &GameState) -> usize {
    state
        .battlefield
        .iter()
        .filter(|id| state.objects[id].name == "Saproling")
        .count()
}

/// CR 732.2a + CR 702.51a: three Sprout Swarm casts spend the Forests and leave three Saprolings,
/// and the window after the third offers the recast: its replay pays by convoke what the recorded
/// cast paid with Forests.
#[test]
fn a_sprout_swarm_ramp_up_is_offered_once_its_forests_are_spent() {
    let Some((mut runner, sprout, _, _)) = sprout_swarm_board() else {
        return;
    };
    for cast in 0..3 {
        assert!(!is_offer(runner.state()), "before cast {cast}");
        runner.cast(sprout).accept_optional().commit();
        settle(&mut runner, &|_| 0);
    }
    assert_eq!(saprolings(runner.state()), 3, "reach: each cast resolved");
    assert_eq!(road(runner.state()), Some(OfferRoad::RecordedPeriod));
}

/// CR 732.3: the same board with Murder cast in response to the third Sprout Swarm: the span
/// holds both seats' plays and is refused before any replay.
#[test]
fn an_opponents_response_fragments_the_sprout_swarm_span() {
    let Some((mut runner, sprout, witherbloom, murder)) = sprout_swarm_board() else {
        return;
    };
    for _ in 0..2 {
        runner.cast(sprout).accept_optional().commit();
        settle(&mut runner, &|_| 0);
        assert!(!is_offer(runner.state()));
    }
    assert_eq!(
        saprolings(runner.state()),
        2,
        "reach: each cast made a Saproling"
    );
    let before = play_trace_counters();
    runner.cast(sprout).accept_optional().commit();
    act(&mut runner, GameAction::PassPriority);
    cast(
        &mut runner,
        murder,
        vec![witherbloom],
        CastPaymentMode::Auto,
    );
    settle(&mut runner, &names(&[witherbloom]));
    let run = play_trace_counters().since(before);
    let state = runner.state();
    assert_eq!(
        state.objects[&witherbloom].zone,
        Zone::Graveyard,
        "reach: Murder resolved"
    );
    assert_eq!(saprolings(state), 3, "reach: Sprout Swarm resolved");
    assert!(!is_offer(state));
    assert_eq!(
        latest_verdict(state),
        Err(OfferRefusal::Fragmented {
            seats: vec![P0, P1]
        })
    );
    assert_eq!((run.confirm_asks, run.confirm_drives), (1, 0));

    let offered = board_c_offered(BoardCMember::C2SqueeTheImmortal).expect("Board C");
    assert!(
        is_offer(offered.runner.state()),
        "reach: Board C is offered"
    );
}

/// CR 732.2a: Food Chain ("Add X mana of any one color ... Spend this mana only to cast creature
/// spells.") making green cannot recast Squee, the Immortal ({1}{R}{R}) once the red in the pool
/// is spent, so the replay refuses; the same plays with red chosen are offered.
#[test]
fn a_replay_refused_for_want_of_red_mana_is_asked_again_once_food_chain_makes_red() {
    let Some(db) = shared_card_db() else { return };
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let food_chain = scenario.add_real_card(P0, "Food Chain", Zone::Battlefield, db);
    let squee = scenario.add_real_card(P0, "Squee, the Immortal", Zone::Battlefield, db);
    scenario.with_mana_pool(
        P0,
        (0..6)
            .map(|unit| ManaUnit::new(ManaType::Red, ObjectId(9_900 + unit), false, Vec::new()))
            .collect(),
    );
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let exile_for = |runner: &mut GameRunner, color: ManaType| {
        activate(runner, food_chain, 0);
        act(runner, GameAction::SelectCards { cards: vec![squee] });
        act(
            runner,
            GameAction::ChooseManaColor {
                choice: ManaChoice::SingleColor(color),
                count: 1,
            },
        );
    };
    for _ in 0..2 {
        exile_for(&mut runner, ManaType::Green);
        assert!(!is_offer(runner.state()), "no offer while green is made");
        cast(&mut runner, squee, vec![], CastPaymentMode::Auto);
        settle(&mut runner, &|_| 0);
        assert!(!is_offer(runner.state()), "no offer while green is made");
    }
    assert_eq!(
        latest_verdict(runner.state()),
        Err(OfferRefusal::IllegalReplayedPlay),
        "reach: the green cycle was refused by its replay"
    );
    exile_for(&mut runner, ManaType::Red);
    assert_eq!(road(runner.state()), Some(OfferRoad::RecordedPeriod));
}
