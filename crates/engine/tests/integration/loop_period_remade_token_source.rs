//! CR 732.2a: a loop whose mana source is a token it sacrifices and makes again every cycle
//! (CR 111.7, CR 111.1) is offered at its first recurrence, and its take performs the cycles.
//!
//! Every board is driven through `apply()` in each mode that offers. A reference is the same
//! board with detection off, driven by hand for as many cycles as the live board performed.

use std::path::Path;

use engine::analysis::decision_template::{IterationCount, ShortcutDecisionSchema};
use engine::analysis::loop_check::{LoopCertificate, OfferRoad, ShortcutResponse};
use engine::analysis::resource::{CounterClass, ObjectClass, ResourceAxis};
use engine::database::card_db::CardDatabase;
use engine::game::engine::apply;
use engine::game::mana_abilities::is_mana_ability;
use engine::game::scenario::{GameScenario, P0};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::game::{play_trace_view, NamingCause};
use engine::types::ability::{AbilityKind, TargetRef};
use engine::types::actions::{DebugAction, DebugTokenRequest, GameAction};
use engine::types::counter::CounterType;
use engine::types::game_state::{
    CastPaymentMode, GameState, LoopDetectionMode, ManaChoice, WaitingFor,
};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaColor, ManaType};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

use crate::support::shared_card_db;

/// Bounds a runaway drive; no assertion reads it.
const BEAT_CAP: usize = 400;

const MODES: [LoopDetectionMode; 2] = [LoopDetectionMode::On, LoopDetectionMode::Interactive];

/// The cycles a fixed-count take is declared for.
const TAKEN: u32 = 5;

const POOL: [ManaType; 6] = [
    ManaType::White,
    ManaType::Blue,
    ManaType::Black,
    ManaType::Red,
    ManaType::Green,
    ManaType::Colorless,
];

/// How a drive answers the prompts a board's loop asks.
#[derive(Clone)]
struct Answers {
    color: ManaType,
    /// What every target prompt and every sacrifice cost names.
    aim: Option<TargetRef>,
    /// The answer to a "you may".
    accept: bool,
}

fn act(state: &mut GameState, action: GameAction) {
    let actor = state
        .waiting_for
        .acting_player()
        .unwrap_or_else(|| panic!("nobody acts at {:?}", state.waiting_for));
    let shown = format!("{action:?}");
    if let Err(error) = apply(state, actor, action) {
        panic!("{shown} rejected: {error:?}");
    }
}

/// Answers prompts until a player has priority, an offer is made or the game ends.
fn answer_prompts(state: &mut GameState, answers: &Answers) {
    for _ in 0..BEAT_CAP {
        let action = match &state.waiting_for {
            WaitingFor::Priority { .. }
            | WaitingFor::LoopShortcut { .. }
            | WaitingFor::GameOver { .. } => return,
            WaitingFor::ChooseManaColor { .. } => GameAction::ChooseManaColor {
                choice: ManaChoice::SingleColor(answers.color),
                count: 1,
            },
            WaitingFor::TriggerTargetSelection { .. } | WaitingFor::TargetSelection { .. } => {
                GameAction::ChooseTarget {
                    target: answers.aim.clone(),
                }
            }
            WaitingFor::PayCost { .. } => match answers.aim {
                Some(TargetRef::Object(paid)) => GameAction::SelectCards { cards: vec![paid] },
                _ => panic!("no object to pay {:?} with", state.waiting_for),
            },
            // The pool holds the payment.
            WaitingFor::ManaPayment { .. } => GameAction::PassPriority,
            WaitingFor::OrderTriggers { triggers, .. } => GameAction::OrderTriggers {
                order: (0..triggers.len()).collect(),
            },
            WaitingFor::OptionalEffectChoice { .. } => GameAction::DecideOptionalEffect {
                accept: answers.accept,
            },
            other => panic!("the drive has no answer to {other:?}"),
        };
        act(state, action);
    }
    panic!("the prompts did not end");
}

/// [`answer_prompts`], with every player passing priority until the stack is empty.
fn settle(state: &mut GameState, answers: &Answers) {
    for _ in 0..BEAT_CAP {
        answer_prompts(state, answers);
        if !matches!(state.waiting_for, WaitingFor::Priority { .. }) || state.stack.is_empty() {
            return;
        }
        act(state, GameAction::PassPriority);
    }
    panic!("the stack did not empty");
}

fn activate(state: &mut GameState, source_id: ObjectId, ability_index: usize) {
    act(
        state,
        GameAction::ActivateAbility {
            source_id,
            ability_index,
        },
    );
}

/// Activates `source`'s first mana ability (CR 605.1a).
fn tap_for_mana(state: &mut GameState, source: ObjectId) {
    let index = state.objects[&source]
        .abilities
        .iter()
        .position(is_mana_ability)
        .expect("a mana ability");
    activate(state, source, index);
}

/// Activates `source`'s first activated ability that is not a mana ability.
fn activate_ability(state: &mut GameState, source: ObjectId) {
    let index = state.objects[&source]
        .abilities
        .iter()
        .position(|ability| ability.kind == AbilityKind::Activated && !is_mana_ability(ability))
        .expect("an activated ability");
    activate(state, source, index);
}

/// P0's untapped tokens named `name`, oldest first.
fn tokens(state: &GameState, name: &str) -> Vec<ObjectId> {
    let mut ids: Vec<ObjectId> = state
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            let object = &state.objects[id];
            object.is_token && object.name == name && object.controller == P0 && !object.tapped
        })
        .collect();
    ids.sort_unstable();
    ids
}

/// Makes `count` Treasures under `owner` with the debug action a live game makes them with.
pub(crate) fn make_treasures(state: &mut GameState, owner: PlayerId, count: u32) {
    let preset_id = engine::game::token_presets::known_token_presets()
        .iter()
        .find(|preset| preset.body.display_name == "Treasure")
        .expect("the Treasure preset")
        .id
        .clone();
    let debug = std::mem::replace(&mut state.debug_mode, true);
    act(
        state,
        GameAction::Debug(DebugAction::CreateToken {
            request: DebugTokenRequest::Preset {
                preset_id,
                owner,
                power_override: None,
                toughness_override: None,
                enter_with_counters: vec![],
            },
            count,
            run_etb: false,
        }),
    );
    state.debug_mode = debug;
}

fn in_mode(board: &GameState, mode: LoopDetectionMode) -> GameState {
    let mut state = board.clone();
    state.loop_detection = mode;
    state
}

fn is_offer(state: &GameState) -> bool {
    matches!(state.waiting_for, WaitingFor::LoopShortcut { .. })
}

/// The standing offer, which must be the recorded period's.
fn offer(state: &GameState) -> (&LoopCertificate, &ShortcutDecisionSchema) {
    let WaitingFor::LoopShortcut {
        certificate,
        schema,
        road,
        ..
    } = &state.waiting_for
    else {
        panic!("no offer at {:?}", state.waiting_for);
    };
    assert_eq!(*road, OfferRoad::RecordedPeriod);
    (certificate, schema)
}

/// Declares the standing offer at `count`, and every seat asked accepts it.
fn take(state: &mut GameState, count: u32) {
    act(
        state,
        GameAction::DeclareShortcut {
            count: IterationCount::Fixed(count),
            template: None,
        },
    );
    while matches!(state.waiting_for, WaitingFor::RespondToShortcut { .. }) {
        act(
            state,
            GameAction::RespondToShortcut {
                response: ShortcutResponse::Accept,
            },
        );
    }
}

/// What a loop moves: every seat's life and standing, P0's pool by mana type and token count,
/// and the damage and +1/+1 counters on one permanent.
#[derive(Debug, PartialEq)]
struct Reading {
    life: Vec<i32>,
    eliminated: Vec<bool>,
    mana: Vec<usize>,
    tokens: usize,
    damage: u32,
    counters: u32,
}

fn reading(state: &GameState, tracked: ObjectId) -> Reading {
    let object = &state.objects[&tracked];
    Reading {
        life: state.players.iter().map(|player| player.life).collect(),
        eliminated: state.players.iter().map(|p| p.is_eliminated).collect(),
        mana: POOL
            .iter()
            .map(|color| state.players[0].mana_pool.count_color(*color))
            .collect(),
        tokens: state
            .battlefield
            .iter()
            .filter(|id| state.objects[*id].is_token && state.objects[*id].controller == P0)
            .count(),
        damage: object.damage_marked,
        counters: object
            .counters
            .get(&CounterType::Plus1Plus1)
            .copied()
            .unwrap_or(0),
    }
}

fn mana_of(color: ManaType, amount: usize) -> Vec<usize> {
    POOL.iter()
        .map(|pooled| if *pooled == color { amount } else { 0 })
        .collect()
}

/// The opponents of P0 still in the game.
fn living_opponents(state: &GameState) -> Vec<PlayerId> {
    state
        .players
        .iter()
        .filter(|player| player.id != P0 && !player.is_eliminated)
        .map(|player| player.id)
        .collect()
}

/// CR 704.5a: the cycles until the last living opponent is at 0 or less life, each losing
/// `per_cycle` life a cycle.
fn cycles_to_the_last_loss(state: &GameState, per_cycle: i32) -> u32 {
    living_opponents(state)
        .into_iter()
        .map(|seat| (state.players[seat.0 as usize].life + per_cycle - 1) / per_cycle)
        .max()
        .expect("a living opponent") as u32
}

/// Four players at 40 life in P0's precombat main, with Smaug the Impenetrable ("Whenever Smaug
/// is dealt noncombat damage, create that many Treasure tokens.") and Mayhem Devil ("Whenever a
/// player sacrifices a permanent, this creature deals 1 damage to any target.") under P0.
fn smaug_scenario(db: &CardDatabase) -> (GameScenario, ObjectId) {
    let mut scenario = GameScenario::new_n_player(4, 7);
    scenario.at_phase(Phase::PreCombatMain);
    for seat in 0..4 {
        scenario.with_life(PlayerId(seat), 40);
    }
    let smaug = scenario.add_real_card(P0, "Smaug the Impenetrable", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Mayhem Devil", Zone::Battlefield, db);
    (scenario, smaug)
}

/// The built scenario with `treasures` Treasures under P0.
fn built(scenario: GameScenario, treasures: u32) -> GameState {
    let mut state = scenario.build().state().clone();
    make_treasures(&mut state, P0, treasures);
    state
}

/// One cycle of a Treasure engine: P0's oldest Treasure sacrificed for mana, then the stack.
fn treasure_cycle(state: &mut GameState, answers: &Answers) {
    let treasure = tokens(state, "Treasure")[0];
    tap_for_mana(state, treasure);
    settle(state, answers);
}

/// CR 111.7 + CR 702.12b: the sacrificed Treasure ceases to exist, the Devil's damage on
/// indestructible Smaug makes another, and each cycle nets one mana.
#[test]
fn a_treasure_remade_by_damage_on_smaug_is_offered_and_taken() {
    let db = shared_card_db().expect("the card fixture");
    let (scenario, smaug) = smaug_scenario(db);
    let board = built(scenario, 1);
    let answers = Answers {
        color: ManaType::Red,
        aim: Some(TargetRef::Object(smaug)),
        accept: true,
    };
    let mut reference = in_mode(&board, LoopDetectionMode::Off);
    for _ in 0..=TAKEN {
        treasure_cycle(&mut reference, &answers);
    }
    assert_eq!(
        reading(&reference, smaug),
        Reading {
            life: vec![40; 4],
            eliminated: vec![false; 4],
            mana: mana_of(ManaType::Red, 6),
            tokens: 1,
            damage: 6,
            counters: 0,
        }
    );
    for mode in MODES {
        let mut live = in_mode(&board, mode);
        treasure_cycle(&mut live, &answers);
        let (certificate, _) = offer(&live);
        assert_eq!(
            certificate.unbounded,
            [ResourceAxis::Mana(ManaType::Red)],
            "{mode:?}"
        );
        take(&mut live, TAKEN);
        assert_eq!(
            reading(&live, smaug),
            reading(&reference, smaug),
            "{mode:?}"
        );
    }
}

/// A land tapped through its mana ability earlier in the step is a consumed play, and the
/// Treasure made again is still offered at its first recurrence, as a play restored.
#[test]
fn a_land_tapped_earlier_in_the_step_does_not_delay_the_offer() {
    let db = shared_card_db().expect("the card fixture");
    let (mut scenario, smaug) = smaug_scenario(db);
    let land = scenario.add_basic_land(P0, ManaColor::Red);
    let board = built(scenario, 1);
    let answers = Answers {
        color: ManaType::Red,
        aim: Some(TargetRef::Object(smaug)),
        accept: true,
    };
    for mode in MODES {
        let mut live = in_mode(&board, mode);
        tap_for_mana(&mut live, land);
        settle(&mut live, &answers);
        assert!(live.objects[&land].tapped, "{mode:?}");
        treasure_cycle(&mut live, &answers);
        offer(&live);
        let offered = play_trace_view(&live)
            .and_then(|view| view.offered)
            .expect("the offered span");
        assert_eq!(offered.cause, NamingCause::Restored, "{mode:?}");
    }
}

/// CR 111.7: a Treasure spent and never made again ceases to exist, and Phyrexian Altar
/// ("Sacrifice a creature: Add one mana of any color.") sacrificing Kitchen Finks ("When this
/// creature enters, you gain 2 life." Persist) beside Vizier of Remedies ("If one or more -1/-1
/// counters would be put on a creature you control, that many -1/-1 counters minus one are put
/// on it instead.") is offered after its first activation all the same.
#[test]
fn a_treasure_spent_earlier_in_the_step_does_not_delay_another_plays_offer() {
    let db = shared_card_db().expect("the card fixture");
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let altar = scenario.add_real_card(P0, "Phyrexian Altar", Zone::Battlefield, db);
    let finks = scenario.add_real_card(P0, "Kitchen Finks", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Vizier of Remedies", Zone::Battlefield, db);
    let board = built(scenario, 1);
    let answers = Answers {
        color: ManaType::Black,
        aim: Some(TargetRef::Object(finks)),
        accept: true,
    };
    for mode in MODES {
        let mut live = in_mode(&board, mode);
        let treasure = tokens(&live, "Treasure")[0];
        treasure_cycle(&mut live, &answers);
        assert!(!live.objects.contains_key(&treasure), "{mode:?}");
        assert_eq!(live.players[0].mana_pool.total(), 1, "{mode:?}");
        let life = live.players[0].life;

        tap_for_mana(&mut live, altar);
        settle(&mut live, &answers);
        assert!(is_offer(&live), "{mode:?}: {:?}", live.waiting_for);
        assert_eq!(live.players[0].life, life + 2, "{mode:?}");
        let returned = &live.objects[&finks];
        assert_eq!(returned.zone, Zone::Battlefield, "{mode:?}");
        assert!(returned.counters.is_empty(), "{mode:?}");
    }
}

/// The captured four-player game, one seat already eliminated, with Mirkwood Bats ("Whenever
/// you create or sacrifice a token, each opponent loses 1 life.") beside Smaug and Mayhem Devil,
/// resolved with detection off to the empty stack its loop starts from.
fn bats_capture() -> (GameState, ObjectId) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/smaug_devil_bats_4p.json.gz");
    let mut state = crate::committed_dump_walk::restore_committed(&path).expect("the capture");
    state.loop_detection = LoopDetectionMode::Off;
    let smaug = state
        .battlefield
        .iter()
        .copied()
        .find(|id| state.objects[id].name == "Smaug the Impenetrable")
        .expect("Smaug");
    assert_eq!(state.objects[&smaug].controller, P0);
    let answers = Answers {
        color: ManaType::Red,
        aim: Some(TargetRef::Object(smaug)),
        accept: true,
    };
    settle(&mut state, &answers);
    (state, smaug)
}

/// CR 704.5a + CR 732.2a: each cycle of the capture costs every living opponent 2 life, so the
/// offer is bounded by the last of them; `pick` names the Treasure of the stock each cycle
/// sacrifices.
fn bats_capture_offered_and_taken(pick: fn(&[ObjectId]) -> Option<&ObjectId>) {
    let (board, smaug) = bats_capture();
    let answers = Answers {
        color: ManaType::Red,
        aim: Some(TargetRef::Object(smaug)),
        accept: true,
    };
    let cycle = |state: &mut GameState| {
        let stock = tokens(state, "Treasure");
        tap_for_mana(state, *pick(&stock).expect("a Treasure"));
        settle(state, &answers);
    };
    let mut reference = in_mode(&board, LoopDetectionMode::Off);
    cycle(&mut reference);
    let bound = cycles_to_the_last_loss(&reference, 2);
    assert!(bound > TAKEN);
    let mut references = Vec::new();
    for count in 1..=bound {
        cycle(&mut reference);
        if count == TAKEN || count == bound {
            references.push((count, reference.clone()));
        }
    }
    for mode in MODES {
        let mut live = in_mode(&board, mode);
        assert!(tokens(&live, "Treasure").len() > 1, "{mode:?}");
        cycle(&mut live);
        let (_, schema) = offer(&live);
        assert_eq!(schema.measured_repetition_bound, Some(bound), "{mode:?}");

        for (count, reference) in &references {
            let mut taken = live.clone();
            take(&mut taken, *count);
            assert_eq!(
                reading(&taken, smaug),
                reading(reference, smaug),
                "{mode:?}, {count} cycles"
            );
            assert_eq!(
                taken.waiting_for, reference.waiting_for,
                "{mode:?}, {count} cycles"
            );
            assert_eq!(
                matches!(taken.waiting_for, WaitingFor::GameOver { .. }),
                *count == bound,
                "{mode:?}, {count} cycles"
            );
        }
    }
}

/// The sacrificed Treasure is older than the trace.
#[test]
fn the_bats_capture_sacrificing_its_oldest_treasure_is_offered_a_bounded_shortcut_and_taken() {
    bats_capture_offered_and_taken(<[ObjectId]>::first);
}

/// The sacrificed Treasure is the stock's newest.
#[test]
fn the_bats_capture_sacrificing_its_newest_treasure_is_offered_a_bounded_shortcut_and_taken() {
    bats_capture_offered_and_taken(<[ObjectId]>::last);
}

/// CR 111.7: with the Devil's damage aimed at an opponent nothing makes a Treasure, so each
/// cycle spends one of the stock and no shortcut is offered.
#[test]
fn a_treasure_stock_spent_without_remaking_is_not_offered() {
    let (board, _) = bats_capture();
    let victim = living_opponents(&board)[0];
    let answers = Answers {
        color: ManaType::Red,
        aim: Some(TargetRef::Player(victim)),
        accept: true,
    };
    let lives = |state: &GameState| -> Vec<i32> {
        living_opponents(state)
            .into_iter()
            .map(|seat| state.players[seat.0 as usize].life)
            .collect()
    };
    for mode in MODES {
        let mut live = in_mode(&board, mode);
        for cycle in 0..6 {
            let (stock, before) = (tokens(&live, "Treasure").len(), lives(&live));
            treasure_cycle(&mut live, &answers);
            assert!(!is_offer(&live), "{mode:?}, cycle {cycle}");
            assert_eq!(tokens(&live, "Treasure").len(), stock - 1, "{mode:?}");
            let after = lives(&live);
            assert_eq!(after.len(), before.len(), "{mode:?}");
            assert!(
                after.iter().zip(&before).all(|(now, then)| now < then),
                "{mode:?}: {before:?} -> {after:?}"
            );
        }
    }
}

/// Four players in P0's precombat main, with Basking Broodscale ("{1}{G}: Adapt 1. Whenever one
/// or more +1/+1 counters are put on this creature, you may create a 0/1 colorless Eldrazi Spawn
/// creature token with 'Sacrifice this token: Add {C}.'"), Blade of the Bloodchief ("Whenever a
/// creature dies, put a +1/+1 counter on equipped creature." Equip {1}) and three Forests.
fn broodscale_board(db: &CardDatabase) -> (GameState, ObjectId, ObjectId, [ObjectId; 3]) {
    let mut scenario = GameScenario::new_n_player(4, 7);
    scenario.at_phase(Phase::PreCombatMain);
    let broodscale = scenario.add_real_card(P0, "Basking Broodscale", Zone::Battlefield, db);
    let blade = scenario.add_real_card(P0, "Blade of the Bloodchief", Zone::Battlefield, db);
    let forests = [(); 3].map(|()| scenario.add_basic_land(P0, ManaColor::Green));
    (scenario.build().state().clone(), broodscale, blade, forests)
}

/// One cycle of the Spawn engine: the Spawn sacrificed for mana, then the stack.
fn spawn_cycle(state: &mut GameState, answers: &Answers) {
    let spawn = tokens(state, "Eldrazi Spawn")[0];
    tap_for_mana(state, spawn);
    settle(state, answers);
}

/// CR 702.6a + CR 701.46a + CR 603.5: with its Forests tapped, the Blade equipped and Adapt
/// paid under detection, the Spawn sacrificed for {C} dies, the Blade's counter makes Broodscale
/// offer another, and the cycle is offered with that "may" as its one decision.
#[test]
fn a_spawn_remade_by_broodscale_is_offered_and_taken() {
    let db = shared_card_db().expect("the card fixture");
    let (board, broodscale, blade, forests) = broodscale_board(db);
    let answers = Answers {
        color: ManaType::Green,
        aim: Some(TargetRef::Object(broodscale)),
        accept: true,
    };
    let set_up = |mode: LoopDetectionMode| {
        let mut state = in_mode(&board, mode);
        for forest in forests {
            tap_for_mana(&mut state, forest);
            settle(&mut state, &answers);
        }
        for source in [blade, broodscale] {
            activate_ability(&mut state, source);
            settle(&mut state, &answers);
        }
        assert_eq!(
            state.objects[&blade].attached_to,
            Some(broodscale.into()),
            "{mode:?}"
        );
        assert_eq!(tokens(&state, "Eldrazi Spawn").len(), 1, "{mode:?}");
        state
    };
    let mut reference = set_up(LoopDetectionMode::Off);
    for _ in 0..=TAKEN {
        spawn_cycle(&mut reference, &answers);
    }
    assert_eq!(
        reading(&reference, broodscale),
        Reading {
            life: vec![20; 4],
            eliminated: vec![false; 4],
            mana: mana_of(ManaType::Colorless, 6),
            tokens: 1,
            damage: 0,
            counters: 7,
        }
    );
    for mode in MODES {
        let mut live = set_up(mode);
        spawn_cycle(&mut live, &answers);
        let (certificate, schema) = offer(&live);
        assert_eq!(
            certificate.unbounded,
            [
                ResourceAxis::Mana(ManaType::Colorless),
                ResourceAxis::Counter(CounterClass::Plus1Plus1, ObjectClass::Creature),
            ],
            "{mode:?}"
        );
        assert_eq!(schema.points.len(), 1, "{mode:?}");
        take(&mut live, TAKEN);
        assert_eq!(
            reading(&live, broodscale),
            reading(&reference, broodscale),
            "{mode:?}"
        );

        let mut declined = set_up(mode);
        let decline = Answers {
            accept: false,
            ..answers.clone()
        };
        spawn_cycle(&mut declined, &decline);
        assert!(!is_offer(&declined), "{mode:?}");
        assert_eq!(
            reading(&declined, broodscale),
            Reading {
                life: vec![20; 4],
                eliminated: vec![false; 4],
                mana: mana_of(ManaType::Colorless, 1),
                tokens: 0,
                damage: 0,
                counters: 2,
            },
            "{mode:?}"
        );
    }
}

/// CR 111.7 + CR 111.1: two Treasures sacrificed while the first's triggers are on the stack
/// are two plays of one ability, each made again from its own Treasure; Adapt on a Broodscale
/// that has its counter spends the mana, and Mirkwood Bats costs each opponent 4 life a cycle.
#[test]
fn two_treasures_sacrificed_before_the_stack_empties_are_offered_and_taken() {
    let db = shared_card_db().expect("the card fixture");
    let (mut scenario, smaug) = smaug_scenario(db);
    scenario.add_real_card(P0, "Mirkwood Bats", Zone::Battlefield, db);
    let broodscale = scenario.add_real_card(P0, "Basking Broodscale", Zone::Battlefield, db);
    let mut board = in_mode(&built(scenario, 2), LoopDetectionMode::Off);
    let answers = Answers {
        color: ManaType::Green,
        aim: Some(TargetRef::Object(smaug)),
        accept: true,
    };
    let cycle = |state: &mut GameState| {
        for _ in 0..2 {
            let treasure = tokens(state, "Treasure")[0];
            tap_for_mana(state, treasure);
            answer_prompts(state, &answers);
            assert!(!state.stack.is_empty());
        }
        activate_ability(state, broodscale);
        settle(state, &answers);
    };
    cycle(&mut board);
    assert_eq!(
        board.objects[&broodscale].counters[&CounterType::Plus1Plus1],
        1
    );
    for mode in MODES {
        let mut live = in_mode(&board, mode);
        let mut reference = in_mode(&board, LoopDetectionMode::Off);
        cycle(&mut live);
        cycle(&mut reference);
        let (_, schema) = offer(&live);
        assert_eq!(
            schema.measured_repetition_bound,
            Some(cycles_to_the_last_loss(&live, 4)),
            "{mode:?}"
        );
        let before = reading(&reference, smaug);
        take(&mut live, TAKEN);
        for _ in 0..TAKEN {
            cycle(&mut reference);
        }
        let after = reading(&reference, smaug);
        assert_eq!(reading(&live, smaug), after, "{mode:?}");
        assert_eq!(
            (after.mana, after.tokens, after.damage),
            (before.mana, before.tokens, before.damage + 10)
        );
        assert_eq!(
            after.life[1..],
            before.life[1..]
                .iter()
                .map(|life| life - 20)
                .collect::<Vec<_>>()
        );
    }
}

/// CR 601.2g + CR 605.3a: Warren Soultrader ("Pay 1 life, Sacrifice another creature: Create a
/// Treasure token.") sacrificing Gravecrawler ("You may cast this card from your graveyard as
/// long as you control a Zombie.") beside Blood Artist ("Whenever this creature or another
/// creature dies, target player loses 1 life and you gain 1 life.") makes again the Treasure
/// that pays for Gravecrawler, however `payment` has that Treasure spent.
fn a_treasure_remade_after_paying_for_a_cast_is_offered_and_taken(payment: CastPaymentMode) {
    let db = shared_card_db().expect("the card fixture");
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let trader = scenario.add_real_card(P0, "Warren Soultrader", Zone::Battlefield, db);
    scenario.add_real_card(P0, "Blood Artist", Zone::Battlefield, db);
    let crawler = scenario.add_real_card(P0, "Gravecrawler", Zone::Graveyard, db);
    let board = built(scenario, 1);
    let answers = Answers {
        color: ManaType::Black,
        aim: Some(TargetRef::Player(PlayerId(1))),
        accept: true,
    };
    let cycle = |state: &mut GameState| {
        let treasure = tokens(state, "Treasure")[0];
        let card_id = state.objects[&crawler].card_id;
        act(
            state,
            GameAction::CastSpell {
                object_id: crawler,
                card_id,
                targets: vec![],
                payment_mode: payment,
            },
        );
        match payment {
            CastPaymentMode::Auto => {}
            CastPaymentMode::Manual => {
                assert!(matches!(state.waiting_for, WaitingFor::ManaPayment { .. }));
                tap_for_mana(state, treasure);
            }
            CastPaymentMode::AutoExceptSacrificialMana => {
                let WaitingFor::ManaSourceSelection { options, .. } = &state.waiting_for else {
                    panic!("no mana source asked for at {:?}", state.waiting_for);
                };
                let [selection] = &options[..] else {
                    panic!("one Treasure offers {options:?}");
                };
                assert_eq!(selection.source.object_id, treasure);
                let selection = selection.clone();
                act(state, GameAction::ActivateManaSource { selection });
            }
        }
        settle(state, &answers);
        assert!(!state.objects.contains_key(&treasure));
        assert_eq!(state.objects[&crawler].zone, Zone::Battlefield);
        activate_ability(state, trader);
        act(
            state,
            GameAction::SelectCards {
                cards: vec![crawler],
            },
        );
        settle(state, &answers);
    };
    let mut reference = in_mode(&board, LoopDetectionMode::Off);
    for _ in 0..4 {
        cycle(&mut reference);
    }
    assert_eq!(
        reading(&reference, trader),
        Reading {
            life: vec![20, 16],
            eliminated: vec![false; 2],
            mana: mana_of(ManaType::Black, 0),
            tokens: 1,
            damage: 0,
            counters: 0,
        }
    );
    assert_eq!(reference.objects[&crawler].zone, Zone::Graveyard);
    for mode in MODES {
        let mut live = in_mode(&board, mode);
        cycle(&mut live);
        let (certificate, schema) = offer(&live);
        assert_eq!(
            certificate.unbounded,
            [ResourceAxis::Life(PlayerId(1))],
            "{mode:?}"
        );
        assert_eq!(schema.measured_repetition_bound, Some(19), "{mode:?}");
        take(&mut live, 3);
        assert_eq!(
            reading(&live, trader),
            reading(&reference, trader),
            "{mode:?}"
        );
        assert_eq!(live.objects[&crawler].zone, Zone::Graveyard, "{mode:?}");
        assert_eq!(live.waiting_for, reference.waiting_for, "{mode:?}");
    }
}

/// The engine spends the Treasure, and no play records it.
#[test]
fn a_treasure_the_engine_pays_a_cast_with_is_offered_and_taken() {
    a_treasure_remade_after_paying_for_a_cast_is_offered_and_taken(CastPaymentMode::Auto);
}

/// The Treasure's ability is activated at the payment.
#[test]
fn a_treasure_activated_at_a_casts_payment_is_offered_and_taken() {
    a_treasure_remade_after_paying_for_a_cast_is_offered_and_taken(CastPaymentMode::Manual);
}

/// The Treasure is the mana source selected when the engine asks before sacrificing one.
#[test]
fn a_treasure_selected_as_a_casts_sacrificial_mana_is_offered_and_taken() {
    a_treasure_remade_after_paying_for_a_cast_is_offered_and_taken(
        CastPaymentMode::AutoExceptSacrificialMana,
    );
}
