use engine::game::game_object::GameObject;
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;

#[test]
fn game_action_fixture_matches_curated_client_contract() {
    let parsed: GameAction = serde_json::from_str(include_str!(
        "../../../../fixtures/adapter-contract/game_action.json"
    ))
    .unwrap();
    match parsed {
        GameAction::ChooseLegend { keep } => assert_eq!(keep.0, 1),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn waiting_for_fixture_matches_curated_client_contract() {
    let parsed: WaitingFor = serde_json::from_str(include_str!(
        "../../../../fixtures/adapter-contract/waiting_for.json"
    ))
    .unwrap();
    match parsed {
        WaitingFor::EffectZoneChoice {
            player,
            cards,
            count,
            source_id,
            ..
        } => {
            assert_eq!(player.0, 0);
            assert_eq!(cards.len(), 2);
            assert_eq!(count, 1);
            assert_eq!(source_id.0, 99);
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn waiting_for_priority_fixture_matches_curated_client_contract() {
    let parsed: WaitingFor = serde_json::from_str(include_str!(
        "../../../../fixtures/adapter-contract/waiting_for_priority.json"
    ))
    .unwrap();
    match parsed {
        WaitingFor::Priority { player } => assert_eq!(player.0, 0),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn game_object_fixture_matches_curated_client_contract() {
    let parsed: GameObject = serde_json::from_str(include_str!(
        "../../../../fixtures/adapter-contract/game_object.json"
    ))
    .unwrap();
    assert_eq!(parsed.name, "Fixture Bear");
    assert_eq!(parsed.id.0, 1);
    assert_eq!(parsed.card_id.0, 100);
    assert_eq!(parsed.power, Some(2));
    assert_eq!(parsed.toughness, Some(2));
}
