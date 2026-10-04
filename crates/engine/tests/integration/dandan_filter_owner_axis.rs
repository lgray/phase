//! Dandân shared pile: a filter that names the graveyard or library is
//! satisfied for every seat that reads the pile, and a filter that does not
//! name it keeps its single player-axis comparison.

use engine::database::card_db::CardDatabase;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::counter::CounterType;
use engine::types::format::FormatConfig;
use engine::types::game_state::{PayCostKind, StackEntryKind, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

use crate::support::shared_card_db;

pub(super) fn plenty_of_mana() -> Vec<ManaUnit> {
    [
        ManaType::White,
        ManaType::Blue,
        ManaType::Black,
        ManaType::Red,
        ManaType::Green,
        ManaType::Colorless,
    ]
    .into_iter()
    .flat_map(|color| (0..6).map(move |_| ManaUnit::new(color, ObjectId(0), false, vec![])))
    .collect()
}

pub(super) fn scenario(format: FormatConfig) -> GameScenario {
    let mut scenario = GameScenario::new_with_format(format, 2, 11);
    scenario.at_phase(Phase::PreCombatMain);
    scenario
}

pub(super) fn stage(
    scenario: &mut GameScenario,
    db: &CardDatabase,
    zone: Zone,
    cards: &[(PlayerId, &str)],
) -> Vec<ObjectId> {
    cards
        .iter()
        .map(|&(owner, name)| scenario.add_real_card(owner, name, zone, db))
        .collect()
}

/// Build the runner with `actor` holding priority in their precombat main phase.
pub(super) fn start(mut scenario: GameScenario, actor: PlayerId) -> GameRunner {
    scenario.with_mana_pool(actor, plenty_of_mana());
    let mut runner = scenario.build();
    let state = runner.state_mut();
    state.active_player = actor;
    state.priority_player = actor;
    state.waiting_for = WaitingFor::Priority { player: actor };
    runner
}

pub(super) fn dandan() -> FormatConfig {
    FormatConfig::dandan()
}

fn gy(runner: &GameRunner, seat: PlayerId) -> Vec<ObjectId> {
    runner.state().graveyard_of(seat).iter().copied().collect()
}

/// Pass priority until the ETB trigger's targets are known: the legal targets of an open prompt,
/// or the lone legal target the engine chose automatically.
fn pass_to_trigger_targets(runner: &mut GameRunner) -> Vec<TargetRef> {
    for _ in 0..12 {
        match runner.state().waiting_for.clone() {
            WaitingFor::TriggerTargetSelection {
                target_slots,
                selection,
                ..
            } => return target_slots[selection.current_slot].legal_targets.clone(),
            _ => {
                if let Some(entry) = runner.state().stack.last() {
                    if matches!(entry.kind, StackEntryKind::TriggeredAbility { .. }) {
                        let ability = entry.ability().expect("a triggered ability");
                        return ability.targets.clone();
                    }
                }
                runner.act(GameAction::PassPriority).expect("pass priority");
            }
        }
    }
    panic!("the trigger never reached the stack");
}

fn sacrifice_ability_index(runner: &GameRunner, source: ObjectId) -> usize {
    runner.state().objects[&source]
        .abilities
        .iter()
        .position(|ability| format!("{:?}", ability.cost).contains("Sacrifice"))
        .expect("a sacrifice ability")
}

// ---------------------------------------------------------------------------
// V1: FilterProp::Owned (Haunted Fengraf)
// ---------------------------------------------------------------------------

/// P1 activates Haunted Fengraf over a graveyard holding `victims`; returns the runner,
/// the Fengraf and the staged victims.
fn fengraf_run(
    format: FormatConfig,
    victims: &[(PlayerId, &str)],
    controller_override: Option<PlayerId>,
) -> (GameRunner, ObjectId, Vec<ObjectId>) {
    let db = shared_card_db().expect("card db");
    let mut sc = scenario(format);
    let fengraf = sc.add_real_card(P1, "Haunted Fengraf", Zone::Battlefield, db);
    let cards = stage(&mut sc, db, Zone::Graveyard, victims);
    let mut runner = start(sc, P1);
    runner.state_mut().objects.get_mut(&fengraf).unwrap().tapped = false;
    if let Some(controller) = controller_override {
        for id in &cards {
            runner.state_mut().objects.get_mut(id).unwrap().controller = controller;
        }
    }
    let index = sacrifice_ability_index(&runner, fengraf);
    runner.activate(fengraf, index).resolve();
    (runner, fengraf, cards)
}

#[test]
fn v1_fengraf_returns_the_other_seats_creature_from_the_shared_graveyard() {
    if shared_card_db().is_none() {
        return;
    }
    // Reach: the cost was paid, so Fengraf itself sits in the pile.
    let (runner, fengraf, cards) = fengraf_run(dandan(), &[(P0, "Grizzly Bears")], None);
    assert_eq!(runner.state().objects[&fengraf].zone, Zone::Graveyard);
    assert_eq!(
        runner.state().objects[&cards[0]].zone,
        Zone::Hand,
        "a P0-owned creature in the shared graveyard satisfies P1's 'your graveyard'"
    );

    // Paired: a P1-owned creature leaves the pile as well.
    let (runner, _, cards) = fengraf_run(dandan(), &[(P1, "Grizzly Bears")], None);
    assert_eq!(runner.state().objects[&cards[0]].zone, Zone::Hand);

    // Hostile: a pile card whose stored controller is P1 is still returned.
    let (runner, _, cards) = fengraf_run(dandan(), &[(P0, "Grizzly Bears")], Some(P1));
    assert_eq!(runner.state().objects[&cards[0]].zone, Zone::Hand);
}

#[test]
fn v1_fengraf_in_standard_returns_only_the_activators_own_graveyard_card() {
    if shared_card_db().is_none() {
        return;
    }
    let (runner, fengraf, cards) = fengraf_run(
        FormatConfig::standard(),
        &[(P0, "Grizzly Bears"), (P1, "Grizzly Bears")],
        None,
    );
    assert_eq!(runner.state().objects[&fengraf].zone, Zone::Graveyard);
    assert_eq!(runner.state().objects[&cards[1]].zone, Zone::Hand);
    assert_eq!(
        runner.state().objects[&cards[0]].zone,
        Zone::Graveyard,
        "the other seat's own graveyard card stays"
    );
    assert!(gy(&runner, P0).contains(&cards[0]));
}

// ---------------------------------------------------------------------------
// V2: typed-filter controller axis (Mystic Sanctuary)
// ---------------------------------------------------------------------------

/// P1 plays Mystic Sanctuary (enters untapped) over a graveyard holding a P0-owned instant and
/// creature plus a P1-owned instant; returns the runner, staged cards and the trigger's targets.
fn sanctuary_run(format: FormatConfig) -> (GameRunner, Vec<ObjectId>, Vec<TargetRef>) {
    let db = shared_card_db().expect("card db");
    let mut sc = scenario(format);
    for _ in 0..3 {
        sc.add_real_card(P1, "Island", Zone::Battlefield, db);
    }
    let sanctuary = sc.add_real_card(P1, "Mystic Sanctuary", Zone::Hand, db);
    let cards = stage(
        &mut sc,
        db,
        Zone::Graveyard,
        &[(P0, "Lightning Bolt"), (P0, "Grizzly Bears"), (P1, "Shock")],
    );
    let mut runner = start(sc, P1);
    let card_id = runner.state().objects[&sanctuary].card_id;
    runner
        .act(GameAction::PlayLand {
            object_id: sanctuary,
            card_id,
        })
        .expect("Mystic Sanctuary is playable");
    assert!(
        !runner.state().objects[&sanctuary].tapped,
        "reach: three other Islands keep it untapped, so the trigger is on the stack"
    );
    let targets = pass_to_trigger_targets(&mut runner);
    (runner, cards, targets)
}

#[test]
fn v2_mystic_sanctuary_targets_the_other_seats_instant_in_the_shared_graveyard() {
    if shared_card_db().is_none() {
        return;
    }
    let (mut runner, cards, targets) = sanctuary_run(dandan());
    let (bolt, bears, shock) = (cards[0], cards[1], cards[2]);
    assert!(targets.contains(&TargetRef::Object(shock)), "paired: own");
    assert!(targets.contains(&TargetRef::Object(bolt)), "other seat's");
    assert!(
        !targets.contains(&TargetRef::Object(bears)),
        "a creature is neither an instant nor a sorcery"
    );

    runner
        .act(GameAction::ChooseTarget {
            target: Some(TargetRef::Object(bolt)),
        })
        .expect("target chosen");
    for _ in 0..6 {
        match runner.state().waiting_for.clone() {
            WaitingFor::OptionalEffectChoice { .. } => {
                runner
                    .act(GameAction::DecideOptionalEffect { accept: true })
                    .expect("accept the may");
            }
            _ => {
                if runner.state().stack.is_empty() {
                    break;
                }
                runner.act(GameAction::PassPriority).expect("resolve");
            }
        }
    }
    assert_eq!(
        runner.state().library_of(P0).front().copied(),
        Some(bolt),
        "the instant sits on top of the shared library"
    );
}

#[test]
fn v2_mystic_sanctuary_in_standard_cannot_reach_the_other_seats_graveyard() {
    if shared_card_db().is_none() {
        return;
    }
    let (_, cards, targets) = sanctuary_run(FormatConfig::standard());
    assert!(targets.contains(&TargetRef::Object(cards[2])), "own: reach");
    assert!(!targets.contains(&TargetRef::Object(cards[0])));
}

// ---------------------------------------------------------------------------
// V3: Owned{Opponent} (Disposal Mummy)
// ---------------------------------------------------------------------------

fn mummy_targets(format: FormatConfig) -> (Vec<ObjectId>, Vec<TargetRef>) {
    let db = shared_card_db().expect("card db");
    let mut sc = scenario(format);
    let mummy = sc.add_real_card(P1, "Disposal Mummy", Zone::Hand, db);
    let cards = stage(
        &mut sc,
        db,
        Zone::Graveyard,
        &[(P0, "Grizzly Bears"), (P1, "Shock")],
    );
    let mut runner = start(sc, P1);
    runner.cast(mummy).commit();
    let targets = pass_to_trigger_targets(&mut runner);
    (cards, targets)
}

#[test]
fn v3_disposal_mummy_treats_the_shared_graveyard_as_an_opponents() {
    if shared_card_db().is_none() {
        return;
    }
    let (cards, targets) = mummy_targets(dandan());
    assert!(targets.contains(&TargetRef::Object(cards[0])), "reach");
    assert!(
        targets.contains(&TargetRef::Object(cards[1])),
        "the pile is an opponent's graveyard for every seat"
    );
}

#[test]
fn v3_disposal_mummy_in_standard_excludes_the_casters_own_graveyard() {
    if shared_card_db().is_none() {
        return;
    }
    let (cards, targets) = mummy_targets(FormatConfig::standard());
    assert!(targets.contains(&TargetRef::Object(cards[0])), "reach");
    assert!(!targets.contains(&TargetRef::Object(cards[1])));
}

// ---------------------------------------------------------------------------
// V14 / V16: observers whose filter does not name the graveyard keep one comparison
// ---------------------------------------------------------------------------

/// P0 controls `observer`; `sacrificer` controls Viscera Seer and a Grizzly Bears and sacrifices
/// the Bears, which lands in the (shared) graveyard.
fn sacrifice_run(
    format: FormatConfig,
    observer: &str,
    sacrificer: PlayerId,
) -> (GameRunner, ObjectId) {
    let db = shared_card_db().expect("card db");
    let mut sc = scenario(format);
    let watcher = sc.add_real_card(P0, observer, Zone::Battlefield, db);
    let seer = sc.add_real_card(sacrificer, "Viscera Seer", Zone::Battlefield, db);
    let victim = sc.add_real_card(sacrificer, "Grizzly Bears", Zone::Battlefield, db);
    stage(
        &mut sc,
        db,
        Zone::Library,
        &[(P0, "Island"), (P0, "Island")],
    );
    let mut runner = start(sc, sacrificer);
    runner
        .act(GameAction::ActivateAbility {
            source_id: seer,
            ability_index: 0,
        })
        .expect("Viscera Seer activates");
    for _ in 0..12 {
        match runner.state().waiting_for.clone() {
            WaitingFor::PayCost {
                kind: PayCostKind::Sacrifice,
                ..
            } => {
                runner
                    .act(GameAction::SelectCards {
                        cards: vec![victim],
                    })
                    .expect("sacrifice the Bears");
            }
            WaitingFor::ScryChoice { .. } => {
                runner
                    .act(GameAction::SelectCards { cards: vec![] })
                    .expect("scry");
            }
            _ if !runner.state().stack.is_empty() => {
                runner.act(GameAction::PassPriority).expect("resolve");
            }
            _ => break,
        }
    }
    assert!(
        gy(&runner, P0).contains(&victim),
        "reach: the sacrificed card is in the shared pile"
    );
    (runner, watcher)
}

fn counters(runner: &GameRunner, id: ObjectId) -> u32 {
    runner.state().objects[&id]
        .counters
        .get(&CounterType::Plus1Plus1)
        .copied()
        .unwrap_or(0)
}

#[test]
fn v14_a_sacrifice_observer_ignores_the_other_seats_sacrifice_into_the_shared_graveyard() {
    if shared_card_db().is_none() {
        return;
    }
    let (runner, watcher) = sacrifice_run(dandan(), "Bloodbriar", P0);
    assert_eq!(
        counters(&runner, watcher),
        1,
        "reach: its own sacrifice counts"
    );

    let (runner, watcher) = sacrifice_run(dandan(), "Bloodbriar", P1);
    assert_eq!(
        counters(&runner, watcher),
        0,
        "'whenever you sacrifice' never sees an opponent's sacrifice"
    );
}

#[test]
fn v16_a_dies_observer_ignores_the_other_seats_creature_in_the_shared_graveyard() {
    if shared_card_db().is_none() {
        return;
    }
    let (runner, _) = sacrifice_run(dandan(), "Zulaport Cutthroat", P0);
    assert_eq!(runner.life(P1), 19, "reach: its own creature dying drains");
    assert_eq!(runner.life(P0), 21);

    let (runner, _) = sacrifice_run(dandan(), "Zulaport Cutthroat", P1);
    assert_eq!(runner.life(P1), 20);
    assert_eq!(runner.life(P0), 20);
}
