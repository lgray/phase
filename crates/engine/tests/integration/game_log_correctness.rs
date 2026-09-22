//! Game log privacy, naming and dedupe across the cast, activation and elimination pipelines.

use engine::game::game_object::{AttachTarget, BackFaceData};
use engine::game::log::resolve_log_entries;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::ability::{AbilityDefinition, AbilityKind, Effect, QuantityExpr, TargetFilter};
use engine::types::actions::GameAction;
use engine::types::card_type::{CardType, CoreType};
use engine::types::events::GameEvent;
use engine::types::game_state::{ActionResult, AutoPassRequest, TurnBoundary, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::log::{GameLogEntry, LogSegment, LogVisibility};
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard, ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const P2: PlayerId = PlayerId(2);
const EXILE_HAND_THEN_DRAW: &str = "Exile all cards from target player's hand, then that player draws a card. That player loses 1 life.";

fn names(entry: &GameLogEntry, id: ObjectId) -> bool {
    entry.segments.iter().any(
        |segment| matches!(segment, LogSegment::CardName { object_id, .. } if *object_id == id),
    )
}

fn entries_naming(entries: &[GameLogEntry], id: ObjectId) -> Vec<&GameLogEntry> {
    entries.iter().filter(|entry| names(entry, id)).collect()
}

fn is_move_line(entry: &GameLogEntry, id: ObjectId, from: Zone, to: Zone) -> bool {
    matches!(
        entry.segments.as_slice(),
        [
            LogSegment::CardName { object_id, .. },
            LogSegment::Text(moves),
            LogSegment::Zone(logged_from),
            LogSegment::Text(_),
            LogSegment::Zone(logged_to),
        ] if *object_id == id && moves == " moves from " && *logged_from == from && *logged_to == to
    )
}

fn has_elimination_line(entries: &[GameLogEntry], player: PlayerId) -> bool {
    entries.iter().any(|entry| {
        matches!(
            entry.segments.as_slice(),
            [LogSegment::PlayerName { player_id, .. }, LogSegment::Text(text)]
                if *player_id == player && text == " is eliminated"
        )
    })
}

fn event_position(result: &ActionResult, predicate: impl Fn(&GameEvent) -> bool) -> Option<usize> {
    result.events.iter().position(predicate)
}

fn move_position(result: &ActionResult, id: ObjectId, from: Zone, to: Zone) -> Option<usize> {
    event_position(result, |event| {
        matches!(event, GameEvent::ZoneChanged { object_id, from: Some(f), to: t, .. }
            if *object_id == id && *f == from && *t == to)
    })
}

fn eliminated_position(result: &ActionResult, player: PlayerId) -> Option<usize> {
    event_position(
        result,
        |event| matches!(event, GameEvent::PlayerEliminated { player_id } if *player_id == player),
    )
}

fn turn_started_position(result: &ActionResult) -> Option<usize> {
    event_position(result, |event| {
        matches!(event, GameEvent::TurnStarted { .. })
    })
}

fn pass_until_elimination(runner: &mut GameRunner, mut result: ActionResult) -> ActionResult {
    for _ in 0..6 {
        if eliminated_position(&result, P2).is_some() {
            return result;
        }
        result = runner.act(GameAction::PassPriority).unwrap();
    }
    panic!("no elimination batch; waiting for {:?}", result.waiting_for);
}

/// P1 and P2 auto-pass until their own next turn, then P0 passes into its end step.
fn arm_auto_pass_and_reach_end_step(runner: &mut GameRunner) {
    runner.act(GameAction::PassPriority).unwrap();
    for player in [P1, P2] {
        assert!(matches!(
            runner.state().waiting_for,
            WaitingFor::Priority { player: holder } if holder == player
        ));
        runner
            .act(GameAction::SetAutoPass {
                mode: AutoPassRequest::UntilTurnBoundary {
                    until: TurnBoundary::MyNextTurnStart,
                },
            })
            .unwrap();
    }
    while runner.state().phase != Phase::End {
        assert!(matches!(
            runner.state().waiting_for,
            WaitingFor::Priority { player } if player == P0
        ));
        runner.act(GameAction::PassPriority).unwrap();
    }
}

/// CR 400.2 + CR 701.17c: a card leaving the library face up into a public zone is public.
#[test]
fn library_departures_into_public_zones_are_logged() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let milled = scenario.add_card_to_library_top(P0, "Probe Milled");
    let mill = scenario
        .add_spell_to_hand_from_oracle(P0, "Probe Mill", false, "Mill a card.")
        .id();
    let mut runner = scenario.build();
    let before = runner.state().clone();
    let outcome = runner.cast(mill).resolve();
    outcome.assert_zone(&[milled], Zone::Graveyard);
    let entries = resolve_log_entries(outcome.events(), &before, outcome.state());
    let milled_entries = entries_naming(&entries, milled);
    assert_eq!(milled_entries.len(), 1, "{entries:?}");
    assert!(is_move_line(
        milled_entries[0],
        milled,
        Zone::Library,
        Zone::Graveyard
    ));

    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let found = scenario.add_card_to_library_top(P0, "Probe Found");
    let search = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Probe Search",
            false,
            "Search your library for a card, put it onto the battlefield, then shuffle.",
        )
        .id();
    let mut runner = scenario.build();
    let before = runner.state().clone();
    let outcome = runner.cast(search).search_first_legal().resolve();
    outcome.assert_zone(&[found], Zone::Battlefield);
    let entries = resolve_log_entries(outcome.events(), &before, outcome.state());
    assert!(entries.iter().any(|entry| is_move_line(
        entry,
        found,
        Zone::Library,
        Zone::Battlefield
    )));
}

/// CR 406.3 + CR 400.2: face-down exile and a draw keep the card's identity out of the public log.
#[test]
fn face_down_exile_and_draw_stay_unnamed() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let hidden = scenario.add_card_to_library_top(P0, "Probe Hidden Top");
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Probe Hideaway",
            false,
            "Exile the top card of your library face down.",
        )
        .id();
    let mut runner = scenario.build();
    let before = runner.state().clone();
    let outcome = runner.cast(spell).resolve();
    outcome.assert_zone(&[hidden], Zone::Exile);
    assert!(outcome.state().objects[&hidden].face_down);
    let entries = resolve_log_entries(outcome.events(), &before, outcome.state());
    assert!(entries.iter().any(|entry| matches!(
        entry.segments.as_slice(),
        [LogSegment::CardName { object_id, .. }, LogSegment::Text(text)]
            if *object_id == spell && text == "'s effect resolves"
    )));
    assert!(entries_naming(&entries, hidden).is_empty(), "{entries:?}");

    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let drawn = scenario.add_card_to_library_top(P0, "Probe Drawn");
    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Probe Draw", false, "Draw a card.")
        .id();
    let mut runner = scenario.build();
    let before = runner.state().clone();
    let outcome = runner.cast(spell).resolve();
    outcome.assert_zone(&[drawn], Zone::Hand);
    let entries = resolve_log_entries(outcome.events(), &before, outcome.state());
    assert!(entries
        .iter()
        .filter(|entry| entry.presentation.visibility == LogVisibility::Public)
        .all(|entry| !names(entry, drawn)));
}

/// CR 800.4a: the leaving player's hidden cards leave the game without being revealed.
#[test]
fn eliminated_players_hidden_cards_stay_unnamed() {
    let mut scenario = GameScenario::new_n_player(3, 7);
    scenario.at_phase(Phase::PreCombatMain);
    let hand = scenario.add_card_to_hand(P2, "Probe Hand");
    let library = scenario.add_card_to_library_top(P2, "Probe Library");
    let face_down = scenario
        .add_creature_to_exile(P2, "Probe Face Down", 2, 2)
        .id();
    let graveyard = scenario
        .add_creature_to_graveyard(P2, "Probe Graveyard", 2, 2)
        .id();
    let mut runner = scenario.build();
    runner
        .state_mut()
        .objects
        .get_mut(&face_down)
        .unwrap()
        .face_down = true;

    let result = runner.act(GameAction::Concede { player_id: P2 }).unwrap();

    for id in [hand, library] {
        assert_eq!(runner.state().objects[&id].zone, Zone::Exile);
    }
    let entries = &result.log_entries;
    assert!(entries.iter().any(|entry| is_move_line(
        entry,
        graveyard,
        Zone::Graveyard,
        Zone::Exile
    )));
    for id in [hand, library, face_down] {
        assert!(entries_naming(entries, id).is_empty(), "{entries:?}");
    }
    assert!(entries.iter().all(|entry| {
        let zones: Vec<_> = entry
            .segments
            .iter()
            .filter_map(|segment| match segment {
                LogSegment::Zone(zone) => Some(zone),
                _ => None,
            })
            .collect();
        !(zones.len() == 2 && zones[0] == zones[1])
    }));
    assert!(has_elimination_line(entries, P2));
}

/// CR 800.4a: only the sweep's moves are hidden; the leaver's own face-up exile earlier in the
/// same batch stays public.
#[test]
fn leavers_same_batch_face_up_exile_is_logged() {
    let mut scenario = GameScenario::new_n_player(3, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P2, 1);
    let kept = scenario.add_card_to_hand(P2, "Probe Kept");
    let deep = scenario.add_card_to_library_top(P2, "Probe Deep");
    let drawn = scenario.add_card_to_library_top(P2, "Probe Drawn");
    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Probe Exile Hand", false, EXILE_HAND_THEN_DRAW)
        .id();
    let mut runner = scenario.build();
    let _ = runner.cast(spell).target_player(P2).commit();
    let first = runner.act(GameAction::PassPriority).unwrap();
    let result = pass_until_elimination(&mut runner, first);

    assert!(move_position(&result, kept, Zone::Hand, Zone::Exile).is_some());
    assert!(move_position(&result, drawn, Zone::Hand, Zone::Exile).is_some());
    assert!(move_position(&result, deep, Zone::Library, Zone::Exile).is_some());
    assert_eq!(turn_started_position(&result), None);

    let entries = &result.log_entries;
    let kept_entries = entries_naming(entries, kept);
    assert_eq!(kept_entries.len(), 1, "{entries:?}");
    assert!(is_move_line(kept_entries[0], kept, Zone::Hand, Zone::Exile));
    assert!(entries_naming(entries, drawn).is_empty(), "{entries:?}");
    assert!(entries_naming(entries, deep).is_empty(), "{entries:?}");
    assert!(has_elimination_line(entries, P2));
}

/// CR 800.4a + CR 800.4j: the sweep stays hidden when the same action carries play into the
/// next turn.
#[test]
fn turn_crossing_elimination_keeps_hidden_cards_unnamed() {
    let mut scenario = GameScenario::new_n_player(3, 7);
    scenario.at_phase(Phase::PreCombatMain);
    let hand = scenario.add_card_to_hand(P0, "Probe Hand");
    let library = scenario.add_card_to_library_top(P0, "Probe Library");
    let mut runner = scenario.build();
    arm_auto_pass_and_reach_end_step(&mut runner);

    let result = runner.act(GameAction::Concede { player_id: P0 }).unwrap();

    assert!(move_position(&result, hand, Zone::Hand, Zone::Exile).is_some());
    assert!(move_position(&result, library, Zone::Library, Zone::Exile).is_some());
    let eliminated = eliminated_position(&result, P0).expect("P0 eliminated in this batch");
    assert!(turn_started_position(&result).is_some_and(|turn_start| eliminated < turn_start));

    let entries = &result.log_entries;
    assert!(entries_naming(entries, hand).is_empty(), "{entries:?}");
    assert!(entries_naming(entries, library).is_empty(), "{entries:?}");
    assert!(has_elimination_line(entries, P0));
}

/// Pins a known limit: in a turn segment the batch leaves through a turn start, the leaver's own
/// face-up exile before their elimination is hidden too, because the journal that tells it apart
/// from the sweep is cleared at turn start and one batch may cross it. This assertion is expected
/// to fail once the log is resolved per turn segment.
#[test]
fn turn_crossing_elimination_hides_leavers_face_up_exile() {
    let mut scenario = GameScenario::new_n_player(3, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P2, 1);
    let kept = scenario.add_card_to_hand(P2, "Probe Kept");
    scenario.add_card_to_library_top(P2, "Probe Deep");
    scenario.add_card_to_library_top(P2, "Probe Drawn");
    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Probe Exile Hand", true, EXILE_HAND_THEN_DRAW)
        .id();
    let mut runner = scenario.build();
    arm_auto_pass_and_reach_end_step(&mut runner);
    let _ = runner.cast(spell).target_player(P2).commit();
    let armed = runner
        .act(GameAction::SetAutoPass {
            mode: AutoPassRequest::UntilTurnBoundary {
                until: TurnBoundary::EndOfCurrentTurn,
            },
        })
        .unwrap();
    let result = pass_until_elimination(&mut runner, armed);

    let kept_move = move_position(&result, kept, Zone::Hand, Zone::Exile).expect("kept exiled");
    let eliminated = eliminated_position(&result, P2).unwrap();
    let turn_start = turn_started_position(&result).expect("batch crosses a turn start");
    assert!(kept_move < eliminated && eliminated < turn_start);

    assert!(
        entries_naming(&result.log_entries, kept).is_empty(),
        "{:?}",
        result.log_entries
    );
}

fn adventure_face() -> BackFaceData {
    BackFaceData {
        is_swap_snapshot: false,
        trigger_printed_origins: Vec::new(),
        name: "Probe Adventure Face".to_string(),
        power: None,
        toughness: None,
        loyalty: None,
        printed_loyalty: None,
        defense: None,
        card_types: {
            let mut card_type = CardType::default();
            card_type.core_types.push(CoreType::Instant);
            card_type.subtypes.push("Adventure".to_string());
            card_type
        },
        mana_cost: ManaCost::Cost {
            shards: vec![ManaCostShard::Red],
            generic: 1,
        },
        keywords: Vec::new(),
        abilities: vec![AbilityDefinition::new(
            AbilityKind::Spell,
            Effect::DealDamage {
                amount: QuantityExpr::Fixed { value: 2 },
                target: TargetFilter::Any,
                damage_source: None,
                excess: None,
            },
        )],
        trigger_definitions: Default::default(),
        replacement_definitions: Default::default(),
        static_definitions: Default::default(),
        color: vec![ManaColor::Red],
        printed_ref: None,
        modal: None,
        additional_cost: None,
        strive_cost: None,
        casting_restrictions: Vec::new(),
        casting_options: Vec::new(),
        layout_kind: None,
        parse_warnings: vec![],
    }
}

/// CR 400.7 + CR 715.4: lines about the Adventure spell name the face it had on the stack, not
/// the creature it becomes in exile.
#[test]
fn adventure_resolution_names_the_adventure_face() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let card = scenario
        .add_creature_to_hand(P0, "Probe Creature Face", 2, 2)
        .with_mana_cost(ManaCost::Cost {
            shards: vec![ManaCostShard::Red],
            generic: 2,
        })
        .id();
    scenario.with_mana_pool(
        P0,
        vec![ManaUnit::new(ManaType::Red, ObjectId(0), false, Vec::new()); 2],
    );
    let mut runner = scenario.build();
    runner.state_mut().objects.get_mut(&card).unwrap().back_face = Some(adventure_face());
    let before = runner.state().clone();
    let outcome = runner
        .cast(card)
        .adventure_face(false)
        .target_player(P1)
        .resolve();
    outcome.assert_zone(&[card], Zone::Exile);
    let entries = resolve_log_entries(outcome.events(), &before, outcome.state());
    let card_name = |entry: &GameLogEntry| match entry.segments.first() {
        Some(LogSegment::CardName { name, object_id }) if *object_id == card => Some(name.clone()),
        _ => None,
    };
    let text_after_card = |entry: &GameLogEntry, expected: &str| matches!(entry.segments.get(1), Some(LogSegment::Text(text)) if text == expected);

    let damage = entries
        .iter()
        .find(|entry| text_after_card(entry, " deals "))
        .and_then(card_name);
    let effect = entries
        .iter()
        .find(|entry| text_after_card(entry, "'s effect resolves"))
        .and_then(card_name);
    let exile = entries
        .iter()
        .find(|entry| is_move_line(entry, card, Zone::Stack, Zone::Exile))
        .and_then(card_name);
    assert_eq!(
        damage.as_deref(),
        Some("Probe Adventure Face"),
        "{entries:?}"
    );
    assert_eq!(
        effect.as_deref(),
        Some("Probe Adventure Face"),
        "{entries:?}"
    );
    assert_eq!(exile.as_deref(), Some("Probe Creature Face"), "{entries:?}");
}

/// CR 702.6a: one equip activation is one log line.
#[test]
fn tagged_activation_logs_one_line() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let bear = scenario.add_creature(P0, "Probe Bear", 2, 2).id();
    let sword = scenario
        .add_artifact_from_oracle(P0, "Probe Sword", "Equip {0}")
        .with_subtypes(vec!["Equipment"])
        .id();
    let mut runner = scenario.build();
    let before = runner.state().clone();
    let outcome = runner.activate(sword, 0).target_object(bear).resolve();

    assert_eq!(
        outcome.state().objects[&sword].attached_to,
        Some(AttachTarget::Object(bear))
    );
    assert!(outcome
        .events()
        .iter()
        .any(|event| matches!(event, GameEvent::AbilityActivated { source_id, .. } if *source_id == sword)));
    assert!(outcome.events().iter().any(|event| matches!(
        event,
        GameEvent::KeywordAbilityActivated { source_id, .. } if *source_id == sword
    )));

    let entries = resolve_log_entries(outcome.events(), &before, outcome.state());
    let activation_lines = |label: &str| {
        entries
            .iter()
            .filter(|entry| {
                names(entry, sword)
                    && entry
                        .segments
                        .iter()
                        .any(|segment| matches!(segment, LogSegment::Text(text) if text == label))
            })
            .count()
    };
    assert_eq!(activation_lines(" activates equip: "), 1, "{entries:?}");
    assert_eq!(activation_lines(" activates ability: "), 0, "{entries:?}");
}
