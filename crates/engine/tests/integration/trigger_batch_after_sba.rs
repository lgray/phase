//! CR 603.3b + CR 607.2a: a creature that dies to a state-based action right
//! after entering puts its enters and leaves triggers on the stack together;
//! when the enters trigger is ordered on top, the leaves trigger refers to the
//! card the enters trigger exiled.
//!
//! Oracle text (Scryfall):
//!   Wormfang Newt: "When this creature enters, exile a land you control.
//!    When this creature leaves the battlefield, return the exiled card to the
//!    battlefield under its owner's control."
//!   Skyclave Apparition: "When this creature enters, exile up to one target
//!    nonland, nontoken permanent you don't control with mana value 4 or less.
//!    When this creature leaves the battlefield, the exiled card's owner
//!    creates an X/X blue Illusion creature token, where X is the mana value of
//!    the exiled card."
//!   Collected Company: "Look at the top six cards of your library. Put up to
//!    two creature cards with mana value 3 or less from among them onto the
//!    battlefield. Put the rest on the bottom of your library in any order."
//!   Elesh Norn, Grand Cenobite: "Vigilance / Other creatures you control get
//!    +2/+2. / Creatures your opponents control get -2/-2."
//! Ruling (Fiend Hunter, 2018-12-07): if Fiend Hunter leaves the battlefield
//! before its first ability resolves, its second ability "will trigger and do
//! nothing. Then its first ability will resolve and exile the target creature
//! indefinitely."

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::ability::{Effect, TargetRef};
use engine::types::actions::GameAction;
use engine::types::game_state::{GameState, LoopDetectionMode, StackEntryKind, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::triggers::TriggerMode;
use engine::types::zones::Zone;

use crate::support::shared_card_db;

const MODES: [LoopDetectionMode; 2] = [LoopDetectionMode::Off, LoopDetectionMode::Interactive];
const NEWT_LAND: (PlayerId, &str) = (P0, "Island");
const SKYCLAVE_TARGET: (PlayerId, &str) = (P1, "Grizzly Bears");

fn pool(n: usize, mana: ManaType) -> Vec<ManaUnit> {
    (0..n)
        .map(|_| ManaUnit::new(mana, ObjectId(0), false, vec![]))
        .collect()
}

/// P0 casts Collected Company and puts `card` onto the battlefield under P1's
/// Elesh Norn, where it dies to the state-based action; `exiled` is the
/// permanent its enters trigger exiles.
struct Board {
    runner: GameRunner,
    card: ObjectId,
    exiled: ObjectId,
}

fn board(mode: LoopDetectionMode, card_name: &str, exiled: (PlayerId, &str)) -> Board {
    let db = shared_card_db().expect("card db");
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let card = scenario.add_real_card(P0, card_name, Zone::Library, db);
    for _ in 0..5 {
        scenario.add_real_card(P0, "Forest", Zone::Library, db);
    }
    let coco = scenario.add_real_card(P0, "Collected Company", Zone::Hand, db);
    scenario.add_real_card(P1, "Elesh Norn, Grand Cenobite", Zone::Battlefield, db);
    let exiled = scenario.add_real_card(exiled.0, exiled.1, Zone::Battlefield, db);
    let mut mana = pool(1, ManaType::Green);
    mana.extend(pool(3, ManaType::Colorless));
    scenario.with_mana_pool(P0, mana);
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    drop(runner.cast(coco).commit());
    Board {
        runner,
        card,
        exiled,
    }
}

fn leaves_description(state: &GameState, card: ObjectId) -> String {
    state.objects[&card]
        .trigger_definitions
        .iter_unchecked()
        .find(|entry| matches!(entry.definition.mode, TriggerMode::LeavesBattlefield))
        .and_then(|entry| entry.definition.description.clone())
        .expect("the card has a described leaves trigger")
}

/// What the run saw on the way to an empty stack.
struct Run {
    runner: GameRunner,
    exiled: ObjectId,
    /// Whether the top stack entry after ordering is the enters exile.
    enters_on_top_after_order: bool,
    /// The exiled object's zone while the leaves trigger is on top.
    exiled_zone_at_leaves: Zone,
    target_prompted: bool,
}

fn run(
    mode: LoopDetectionMode,
    card_name: &str,
    exiled: (PlayerId, &str),
    enters_on_top: bool,
) -> Run {
    let Board {
        mut runner,
        card,
        exiled,
    } = board(mode, card_name, exiled);
    let leaves = leaves_description(runner.state(), card);
    let mut ordered = false;
    let mut enters_on_top_after_order = None;
    let mut exiled_zone_at_leaves = None;
    let mut target_prompted = false;
    for _ in 0..60 {
        let state = runner.state();
        let action = match &state.waiting_for {
            WaitingFor::Priority { .. } => {
                let top = state.stack.back().map(|entry| &entry.kind);
                if let Some(StackEntryKind::TriggeredAbility {
                    source_id,
                    ability,
                    description,
                    ..
                }) = top
                {
                    if ordered && enters_on_top_after_order.is_none() {
                        enters_on_top_after_order = Some(
                            *source_id == card
                                && matches!(
                                    ability.effect,
                                    Effect::ChangeZone {
                                        destination: Zone::Exile,
                                        ..
                                    }
                                ),
                        );
                    }
                    if *source_id == card && description.as_deref() == Some(leaves.as_str()) {
                        exiled_zone_at_leaves.get_or_insert(state.objects[&exiled].zone);
                    }
                }
                if state.stack.is_empty() {
                    break;
                }
                GameAction::PassPriority
            }
            WaitingFor::OrderTriggers { player, triggers } => {
                assert_eq!(*player, P0, "{mode:?}: the controller orders");
                assert_eq!(triggers.len(), 2, "{mode:?}: both triggers wait together");
                assert!(
                    triggers.iter().all(|t| t.source_id == card),
                    "{mode:?}: both are the card's triggers"
                );
                ordered = true;
                // Index 0 is placed first, so the last listed trigger is on top.
                let mut order: Vec<usize> = (0..triggers.len()).collect();
                order.sort_by_key(|&i| (triggers[i].description == leaves) != enters_on_top);
                GameAction::OrderTriggers { order }
            }
            WaitingFor::DigChoice { .. } => GameAction::SelectCards { cards: vec![card] },
            WaitingFor::TriggerTargetSelection { .. } => {
                target_prompted = true;
                GameAction::SelectTargets {
                    targets: vec![TargetRef::Object(exiled)],
                }
            }
            other => panic!("{mode:?}: unhandled prompt {other:?}"),
        };
        runner.act(action).expect("action accepted");
    }
    let state = runner.state();
    assert!(state.stack.is_empty(), "{mode:?}: the stack settled");
    assert!(
        matches!(state.waiting_for, WaitingFor::Priority { .. }),
        "{mode:?}: ends at priority"
    );
    assert!(ordered, "{mode:?}: the ordering prompt was reached");
    assert_eq!(state.objects[&card].zone, Zone::Graveyard, "{mode:?}");
    Run {
        runner,
        exiled,
        enters_on_top_after_order: enters_on_top_after_order.expect("a trigger was on top"),
        exiled_zone_at_leaves: exiled_zone_at_leaves.expect("the leaves trigger was on top"),
        target_prompted,
    }
}

/// Illusion tokens on the battlefield, as `(controller, power, toughness)`.
fn illusions(state: &GameState) -> Vec<(PlayerId, Option<i32>, Option<i32>)> {
    state
        .battlefield
        .iter()
        .filter_map(|id| state.objects.get(id))
        .filter(|object| object.name == "Illusion")
        .map(|object| (object.controller, object.power, object.toughness))
        .collect()
}

#[test]
fn newt_enters_on_top_returns_the_land_it_exiled() {
    for mode in MODES {
        let r = run(mode, "Wormfang Newt", NEWT_LAND, true);
        assert!(r.enters_on_top_after_order, "{mode:?}");
        assert_eq!(r.exiled_zone_at_leaves, Zone::Exile, "{mode:?}");
        let island = &r.runner.state().objects[&r.exiled];
        // CR 607.2a: the leaves trigger refers to the card its linked ability exiled.
        assert_eq!(island.zone, Zone::Battlefield, "{mode:?}");
        assert_eq!(island.controller, P0, "{mode:?}");
    }
}

#[test]
fn newt_leaves_on_top_returns_nothing() {
    for mode in MODES {
        let r = run(mode, "Wormfang Newt", NEWT_LAND, false);
        assert!(!r.enters_on_top_after_order, "{mode:?}");
        assert_eq!(r.exiled_zone_at_leaves, Zone::Battlefield, "{mode:?}");
        assert_eq!(
            r.runner.state().objects[&r.exiled].zone,
            Zone::Exile,
            "{mode:?}"
        );
    }
}

#[test]
fn skyclave_enters_on_top_gives_the_owner_a_token_of_its_mana_value() {
    for mode in MODES {
        let r = run(mode, "Skyclave Apparition", SKYCLAVE_TARGET, true);
        assert!(r.target_prompted, "{mode:?}");
        assert!(r.enters_on_top_after_order, "{mode:?}");
        assert_eq!(r.exiled_zone_at_leaves, Zone::Exile, "{mode:?}");
        let state = r.runner.state();
        assert_eq!(state.objects[&r.exiled].zone, Zone::Exile, "{mode:?}");
        // CR 607.2a: X is the exiled Bears' mana value 2; P1's Norn adds +2/+2.
        assert_eq!(illusions(state), vec![(P1, Some(4), Some(4))], "{mode:?}");
    }
}

#[test]
fn skyclave_leaves_on_top_creates_no_token() {
    for mode in MODES {
        let r = run(mode, "Skyclave Apparition", SKYCLAVE_TARGET, false);
        assert!(r.target_prompted, "{mode:?}");
        assert!(!r.enters_on_top_after_order, "{mode:?}");
        assert_eq!(r.exiled_zone_at_leaves, Zone::Battlefield, "{mode:?}");
        let state = r.runner.state();
        assert_eq!(state.objects[&r.exiled].zone, Zone::Exile, "{mode:?}");
        assert!(illusions(state).is_empty(), "{mode:?}");
    }
}
