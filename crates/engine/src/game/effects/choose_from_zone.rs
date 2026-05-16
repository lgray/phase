use crate::game::players;
use crate::types::ability::{
    ChooseFromZoneConstraint, Chooser, Effect, EffectError, EffectKind, ResolvedAbility, TargetRef,
};
use crate::types::card_type::CoreType;
use crate::types::events::GameEvent;
use crate::types::game_state::{GameState, WaitingFor};
use crate::types::identifiers::ObjectId;
use crate::types::player::PlayerId;

/// CR 700.2: Choose card(s) from a tracked set — player selects from exiled/revealed cards.
/// The available cards come from the most recent tracked set recorded by the parent effect
/// (e.g., ChangeZone to exile). The `chooser` field determines whether the controller or
/// an opponent makes the selection.
pub fn resolve(
    state: &mut GameState,
    ability: &ResolvedAbility,
    events: &mut Vec<GameEvent>,
) -> Result<(), EffectError> {
    let (count, chooser, up_to, constraint) = match &ability.effect {
        Effect::ChooseFromZone {
            count,
            chooser,
            up_to,
            constraint,
            ..
        } => (*count as usize, *chooser, *up_to, constraint.clone()),
        _ => return Err(EffectError::MissingParam("ChooseFromZone".to_string())),
    };

    // Read available cards from the most recent tracked set (same pattern as delayed_trigger.rs).
    // The tracked set was recorded by the preceding ChangeZone effect via next_sub_needs_tracked_set.
    let cards: Vec<ObjectId> = crate::game::targeting::latest_tracked_set_id(state)
        .and_then(|id| state.tracked_object_sets.get(&id).cloned())
        .unwrap_or_default();

    // Fallback: if tracked set is empty, try ability targets filtered to objects.
    let cards = if cards.is_empty() {
        ability
            .targets
            .iter()
            .filter_map(|t| match t {
                TargetRef::Object(id) => Some(*id),
                _ => None,
            })
            .collect()
    } else {
        cards
    };

    // CR 700.2: If there are no objects to choose from, skip the choice.
    if cards.is_empty() || count == 0 {
        events.push(GameEvent::EffectResolved {
            kind: EffectKind::ChooseFromZone,
            source_id: ability.source_id,
        });
        return Ok(());
    }

    let clamped_count = count.min(cards.len());

    // CR 700.2: Determine who makes the choice.
    let choosing_player = resolve_chooser(state, ability, chooser);

    state.waiting_for = WaitingFor::ChooseFromZoneChoice {
        player: choosing_player,
        cards,
        count: clamped_count,
        up_to,
        constraint,
        source_id: ability.source_id,
    };

    events.push(GameEvent::EffectResolved {
        kind: EffectKind::ChooseFromZone,
        source_id: ability.source_id,
    });

    Ok(())
}

/// CR 700.2: Resolve the `Chooser` enum to an actual `PlayerId`.
/// For `Opponent`, first checks ability targets for a pre-targeted opponent player
/// (handles "target opponent chooses"), then falls back to the first opponent in APNAP order.
fn resolve_chooser(state: &GameState, ability: &ResolvedAbility, chooser: Chooser) -> PlayerId {
    match chooser {
        Chooser::Controller => ability.controller,
        Chooser::Opponent => {
            // Check if an opponent was already targeted by the spell.
            if let Some(targeted_opponent) = ability.targets.iter().find_map(|t| match t {
                TargetRef::Player(id) if *id != ability.controller => Some(*id),
                _ => None,
            }) {
                return targeted_opponent;
            }
            // Fallback: first opponent in APNAP order (CR-correct for 2-player).
            players::opponents(state, ability.controller)
                .into_iter()
                .next()
                .unwrap_or(ability.controller)
        }
    }
}

pub fn selection_satisfies_constraint(
    state: &GameState,
    chosen: &[ObjectId],
    constraint: Option<&ChooseFromZoneConstraint>,
) -> bool {
    match constraint {
        None => true,
        Some(ChooseFromZoneConstraint::DistinctCardTypes { categories }) => {
            selected_cards_cover_distinct_card_types(state, chosen, categories)
        }
    }
}

fn selected_cards_cover_distinct_card_types(
    state: &GameState,
    chosen: &[ObjectId],
    categories: &[CoreType],
) -> bool {
    if chosen.is_empty() {
        return true;
    }
    if chosen.len() > categories.len() {
        return false;
    }

    let card_options: Option<Vec<Vec<usize>>> = chosen
        .iter()
        .map(|id| {
            state.objects.get(id).map(|obj| {
                categories
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, category)| {
                        obj.card_types.core_types.contains(category).then_some(idx)
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();

    let mut card_options = match card_options {
        Some(options) => options,
        None => return false,
    };
    if card_options.iter().any(Vec::is_empty) {
        return false;
    }

    card_options.sort_by_key(Vec::len);
    let mut used = vec![false; categories.len()];
    assign_distinct_categories(&card_options, &mut used, 0)
}

fn assign_distinct_categories(card_options: &[Vec<usize>], used: &mut [bool], idx: usize) -> bool {
    if idx == card_options.len() {
        return true;
    }
    for &category_idx in &card_options[idx] {
        if used[category_idx] {
            continue;
        }
        used[category_idx] = true;
        if assign_distinct_categories(card_options, used, idx + 1) {
            return true;
        }
        used[category_idx] = false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::zones::create_object;
    use crate::types::identifiers::{CardId, TrackedSetId};
    use crate::types::zones::Zone;

    #[test]
    fn resolve_with_controller_chooser() {
        let mut state = GameState::new_two_player(42);
        let card1 = create_object(
            &mut state,
            CardId(1),
            PlayerId(0),
            "Card A".to_string(),
            Zone::Exile,
        );
        let card2 = create_object(
            &mut state,
            CardId(2),
            PlayerId(0),
            "Card B".to_string(),
            Zone::Exile,
        );

        // Simulate tracked set from parent ChangeZone
        state
            .tracked_object_sets
            .insert(TrackedSetId(1), vec![card1, card2]);
        state.next_tracked_set_id = 2;

        let ability = ResolvedAbility::new(
            Effect::ChooseFromZone {
                count: 1,
                zone: Zone::Exile,
                chooser: Chooser::Controller,
                up_to: false,
                constraint: None,
            },
            vec![],
            ObjectId(100),
            PlayerId(0),
        );
        let mut events = Vec::new();

        resolve(&mut state, &ability, &mut events).unwrap();

        match &state.waiting_for {
            WaitingFor::ChooseFromZoneChoice {
                player,
                cards,
                count,
                up_to,
                constraint,
                ..
            } => {
                assert_eq!(*player, PlayerId(0), "Controller should be the chooser");
                assert_eq!(cards.len(), 2);
                assert_eq!(*count, 1);
                assert!(!up_to);
                assert!(constraint.is_none());
            }
            other => panic!("Expected ChooseFromZoneChoice, got {:?}", other),
        }
    }

    #[test]
    fn resolve_with_opponent_chooser() {
        let mut state = GameState::new_two_player(42);
        let card1 = create_object(
            &mut state,
            CardId(1),
            PlayerId(0),
            "Card A".to_string(),
            Zone::Exile,
        );

        state
            .tracked_object_sets
            .insert(TrackedSetId(1), vec![card1]);
        state.next_tracked_set_id = 2;

        let ability = ResolvedAbility::new(
            Effect::ChooseFromZone {
                count: 1,
                zone: Zone::Exile,
                chooser: Chooser::Opponent,
                up_to: false,
                constraint: None,
            },
            vec![],
            ObjectId(100),
            PlayerId(0),
        );
        let mut events = Vec::new();

        resolve(&mut state, &ability, &mut events).unwrap();

        match &state.waiting_for {
            WaitingFor::ChooseFromZoneChoice { player, count, .. } => {
                assert_eq!(*player, PlayerId(1), "Opponent should be the chooser");
                assert_eq!(*count, 1);
            }
            other => panic!("Expected ChooseFromZoneChoice, got {:?}", other),
        }
    }

    #[test]
    fn resolve_with_targeted_opponent() {
        let mut state = GameState::new_two_player(42);
        let card1 = create_object(
            &mut state,
            CardId(1),
            PlayerId(0),
            "Card A".to_string(),
            Zone::Exile,
        );

        state
            .tracked_object_sets
            .insert(TrackedSetId(1), vec![card1]);
        state.next_tracked_set_id = 2;

        // Simulate a targeted opponent (e.g., Gifts Ungiven targeting PlayerId(1))
        let ability = ResolvedAbility::new(
            Effect::ChooseFromZone {
                count: 1,
                zone: Zone::Exile,
                chooser: Chooser::Opponent,
                up_to: false,
                constraint: None,
            },
            vec![TargetRef::Player(PlayerId(1))],
            ObjectId(100),
            PlayerId(0),
        );
        let mut events = Vec::new();

        resolve(&mut state, &ability, &mut events).unwrap();

        match &state.waiting_for {
            WaitingFor::ChooseFromZoneChoice { player, .. } => {
                assert_eq!(
                    *player,
                    PlayerId(1),
                    "Targeted opponent should be the chooser"
                );
            }
            other => panic!("Expected ChooseFromZoneChoice, got {:?}", other),
        }
    }

    #[test]
    fn empty_tracked_set_skips_choice() {
        let mut state = GameState::new_two_player(42);

        let ability = ResolvedAbility::new(
            Effect::ChooseFromZone {
                count: 1,
                zone: Zone::Exile,
                chooser: Chooser::Opponent,
                up_to: false,
                constraint: None,
            },
            vec![],
            ObjectId(100),
            PlayerId(0),
        );
        let mut events = Vec::new();

        resolve(&mut state, &ability, &mut events).unwrap();

        // Should not set ChooseFromZoneChoice — no cards to choose from
        assert!(
            !matches!(state.waiting_for, WaitingFor::ChooseFromZoneChoice { .. }),
            "Should skip choice when tracked set is empty"
        );
    }

    #[test]
    fn count_clamped_to_available_cards() {
        let mut state = GameState::new_two_player(42);
        let card1 = create_object(
            &mut state,
            CardId(1),
            PlayerId(0),
            "Card A".to_string(),
            Zone::Exile,
        );

        state
            .tracked_object_sets
            .insert(TrackedSetId(1), vec![card1]);
        state.next_tracked_set_id = 2;

        // Request 3 but only 1 card available
        let ability = ResolvedAbility::new(
            Effect::ChooseFromZone {
                count: 3,
                zone: Zone::Exile,
                chooser: Chooser::Controller,
                up_to: false,
                constraint: None,
            },
            vec![],
            ObjectId(100),
            PlayerId(0),
        );
        let mut events = Vec::new();

        resolve(&mut state, &ability, &mut events).unwrap();

        match &state.waiting_for {
            WaitingFor::ChooseFromZoneChoice { count, .. } => {
                assert_eq!(*count, 1, "Count should be clamped to available cards");
            }
            other => panic!("Expected ChooseFromZoneChoice, got {:?}", other),
        }
    }

    #[test]
    fn distinct_card_type_constraint_accepts_valid_assignment() {
        let mut state = GameState::new_two_player(42);
        let artifact_creature = create_object(
            &mut state,
            CardId(1),
            PlayerId(0),
            "Patchwork Automaton".to_string(),
            Zone::Library,
        );
        state
            .objects
            .get_mut(&artifact_creature)
            .unwrap()
            .card_types
            .core_types = vec![CoreType::Artifact, CoreType::Creature];
        let creature = create_object(
            &mut state,
            CardId(2),
            PlayerId(0),
            "Elvish Mystic".to_string(),
            Zone::Library,
        );
        state
            .objects
            .get_mut(&creature)
            .unwrap()
            .card_types
            .core_types = vec![CoreType::Creature];

        assert!(selection_satisfies_constraint(
            &state,
            &[artifact_creature, creature],
            Some(&ChooseFromZoneConstraint::DistinctCardTypes {
                categories: vec![CoreType::Artifact, CoreType::Creature],
            }),
        ));
    }

    #[test]
    fn distinct_card_type_constraint_rejects_duplicate_assignment_only() {
        let mut state = GameState::new_two_player(42);
        let creature_a = create_object(
            &mut state,
            CardId(1),
            PlayerId(0),
            "Elvish Mystic".to_string(),
            Zone::Library,
        );
        state
            .objects
            .get_mut(&creature_a)
            .unwrap()
            .card_types
            .core_types = vec![CoreType::Creature];
        let creature_b = create_object(
            &mut state,
            CardId(2),
            PlayerId(0),
            "Llanowar Elves".to_string(),
            Zone::Library,
        );
        state
            .objects
            .get_mut(&creature_b)
            .unwrap()
            .card_types
            .core_types = vec![CoreType::Creature];

        assert!(!selection_satisfies_constraint(
            &state,
            &[creature_a, creature_b],
            Some(&ChooseFromZoneConstraint::DistinctCardTypes {
                categories: vec![CoreType::Artifact, CoreType::Creature],
            }),
        ));
    }
}
