//! CR 613.8b: when effects form a dependency loop, only the loop members fall back to
//! timestamp order; an effect that depends on the loop still waits until just after it.
//!
//! Board: Curse of Conformity ("nonlegendary creatures enchanted player controls ... lose all
//! creature types") depends on March of the Machines (which makes noncreature artifacts
//! creatures). March of the Machines and Prismatic Omen form a loop only in the engine's
//! shape-based dependency model (CR 613.8a does not make either depend on the other); the
//! expected result is the same under the precise reading, and the loop-only stall path is
//! pinned at unit level in `layers.rs`.

use super::support::shared_card_db;
use engine::database::card_db::CardDatabase;
use engine::game::deck_loading::create_object_from_card_face;
use engine::game::effects::attach::attach_to_player;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::game::zones::{add_to_zone, remove_from_zone};
use engine::types::card_type::CoreType;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const BASIC_LAND_TYPES: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];

/// Which of the three layered enchantments is placed first; the oldest timestamp goes to the
/// first-placed card.
#[derive(Clone, Copy)]
enum Order {
    CurseOldest,
    CurseNewest,
}

struct Board {
    runner: GameRunner,
    plains: ObjectId,
    equipment: ObjectId,
    curse: Option<(ObjectId, u64)>,
    march_ts: u64,
    omen_ts: u64,
}

/// Place a real card after `build()`; `add_real_card(Battlefield)` panics on an Aura.
fn place(runner: &mut GameRunner, name: &str, db: &CardDatabase) -> (ObjectId, u64) {
    let state = runner.state_mut();
    let face = db
        .get_face_by_name(name)
        .unwrap_or_else(|| panic!("card '{name}' not found in fixture"));
    let id = create_object_from_card_face(state, face, P0);
    remove_from_zone(state, id, Zone::Library, P0);
    add_to_zone(state, id, Zone::Battlefield, P0);
    let ts = state.next_timestamp();
    let obj = state.objects.get_mut(&id).unwrap();
    obj.zone = Zone::Battlefield;
    obj.timestamp = ts;
    (id, ts)
}

fn setup(db: &CardDatabase, curse_host: Option<PlayerId>, order: Order) -> Board {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let plains = scenario.add_real_card(P0, "Plains", Zone::Battlefield, db);
    let mut runner = scenario.build();
    runner.state_mut().all_creature_types = db.creature_type_vocabulary().to_vec();

    let (equipment, _) = place(&mut runner, "Cloak and Dagger", db);
    let mut curse = None;
    let mut place_curse = |runner: &mut GameRunner| {
        if let Some(host) = curse_host {
            let placed = place(runner, "Curse of Conformity", db);
            attach_to_player(runner.state_mut(), placed.0, host);
            curse = Some(placed);
        }
    };
    if matches!(order, Order::CurseOldest) {
        place_curse(&mut runner);
    }
    let (_, march_ts) = place(&mut runner, "March of the Machines", db);
    let (_, omen_ts) = place(&mut runner, "Prismatic Omen", db);
    if matches!(order, Order::CurseNewest) {
        place_curse(&mut runner);
    }

    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());
    Board {
        runner,
        plains,
        equipment,
        curse,
        march_ts,
        omen_ts,
    }
}

/// Reach-guards shared by every row: the vocabulary seeds `Rogue`, March of the Machines made
/// the Equipment a creature, Prismatic Omen gave the Plains every basic land type, and the
/// timestamps ordered as intended.
fn assert_reached(board: &Board, db: &CardDatabase, order: Order) {
    let state = board.runner.state();
    let vocabulary = db.creature_type_vocabulary();
    assert!(vocabulary.iter().any(|t| t == "Rogue"));
    assert!(!vocabulary.iter().any(|t| t == "Equipment"));
    assert!(state.objects[&board.equipment]
        .card_types
        .core_types
        .contains(&CoreType::Creature));
    let plains = &state.objects[&board.plains].card_types.subtypes;
    for land_type in BASIC_LAND_TYPES {
        assert!(plains.iter().any(|t| t == land_type), "missing {land_type}");
    }
    assert!(board.march_ts < board.omen_ts);
    if let Some((_, curse_ts)) = board.curse {
        match order {
            Order::CurseOldest => assert!(curse_ts < board.march_ts),
            Order::CurseNewest => assert!(board.omen_ts < curse_ts),
        }
    }
}

fn equipment_subtypes(board: &Board) -> Vec<String> {
    board.runner.state().objects[&board.equipment]
        .card_types
        .subtypes
        .clone()
}

#[test]
fn curse_older_than_the_loop_still_applies_after_it() {
    let Some(db) = shared_card_db() else { return };
    let board = setup(db, Some(P0), Order::CurseOldest);
    assert_reached(&board, db, Order::CurseOldest);
    assert_eq!(equipment_subtypes(&board), vec!["Equipment".to_string()]);
}

#[test]
fn curse_newer_than_the_loop_applies_after_it() {
    let Some(db) = shared_card_db() else { return };
    let board = setup(db, Some(P0), Order::CurseNewest);
    assert_reached(&board, db, Order::CurseNewest);
    assert_eq!(equipment_subtypes(&board), vec!["Equipment".to_string()]);
}

#[test]
fn loop_alone_applies_every_member() {
    let Some(db) = shared_card_db() else { return };
    let board = setup(db, None, Order::CurseOldest);
    assert_reached(&board, db, Order::CurseOldest);
    assert!(equipment_subtypes(&board).iter().any(|t| t == "Rogue"));
}

#[test]
fn curse_on_another_player_leaves_the_loop_recipients_untouched() {
    let Some(db) = shared_card_db() else { return };
    let board = setup(db, Some(P1), Order::CurseOldest);
    assert_reached(&board, db, Order::CurseOldest);
    assert!(equipment_subtypes(&board).iter().any(|t| t == "Rogue"));
}
