use crate::game::quantity::resolve_quantity_with_targets;
use crate::types::ability::{Effect, EffectError, EffectKind, ResolvedAbility};
use crate::types::events::GameEvent;
use crate::types::game_state::GameState;

/// CR 701.58a: Cloak — put the top card of a player's library onto the
/// battlefield face down as a 2/2 creature **with ward {2}**. Like manifest
/// (CR 701.40a), a cloaked creature card can later be turned face up for its
/// mana cost; the sole behavioral difference is the ward {2} the cloaked
/// permanent enters with (granted via `FaceDownProfile::cloaked_2_2`).
///
/// `target` selects whose library is cloaked from (mirrors `Effect::Manifest`):
/// `Controller` for "you cloak the top card of your library",
/// `ParentTargetController` / `TriggeringPlayer` for relative-player bodies.
///
/// `object_source` selects WHICH cards are cloaked. `None` is the CR 701.58e
/// library-top source (Cryptic Coat, Ransom Note). `Some(filter)` names
/// explicit objects a preceding `Effect::ChooseFromZone` chose and forwarded
/// onto this ability's `targets` — Vannifar's "cloak a card from your hand".
pub fn resolve(
    state: &mut GameState,
    ability: &ResolvedAbility,
    events: &mut Vec<GameEvent>,
) -> Result<(), EffectError> {
    let (target, count, object_source) = match &ability.effect {
        Effect::Cloak {
            target,
            count,
            object_source,
        } => (
            target.clone(),
            resolve_quantity_with_targets(state, count, ability).max(0) as usize,
            object_source.clone(),
        ),
        _ => return Err(EffectError::MissingParam("count".to_string())),
    };

    let player = super::resolve_player_for_context_ref(state, ability, &target);

    match object_source {
        // CR 701.58a: Cloak explicit objects chosen upstream (Vannifar's "cloak
        // a card from your hand"). The parent `ChooseFromZone` wrote the chosen
        // cards onto `ability.targets`; resolve them the same way every
        // object-forwarding handler does (CR 608.2c — later instructions read
        // the earlier selection). Each card is turned face down as a 2/2 with
        // ward {2} in its own zone (CR 701.58a), sourced by this ability.
        Some(filter) => {
            let object_ids = crate::game::effects::effect_object_targets(&filter, &ability.targets);
            for object_id in object_ids {
                crate::game::morph::manifest_card(
                    state,
                    player,
                    object_id,
                    ability.source_id,
                    crate::types::ability::FaceDownProfile::cloaked_2_2(),
                    None,
                    events,
                )
                .map_err(|e| EffectError::MissingParam(format!("{e}")))?;
            }
        }
        // CR 701.58e: If an effect instructs a player to cloak multiple cards
        // from a single library, those cards are cloaked one at a time.
        None => {
            for _ in 0..count {
                let has_cards = state
                    .players
                    .iter()
                    .find(|p| p.id == player)
                    .map(|p| !p.library.is_empty())
                    .unwrap_or(false);

                if !has_cards {
                    break;
                }

                crate::game::morph::cloak(state, player, events)
                    .map_err(|e| EffectError::MissingParam(format!("{e}")))?;
            }
        }
    }

    events.push(GameEvent::EffectResolved {
        kind: EffectKind::from(&ability.effect),
        source_id: ability.source_id,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::zones::create_object;
    use crate::types::ability::{QuantityExpr, TargetFilter};
    use crate::types::identifiers::{CardId, ObjectId};
    use crate::types::keywords::{Keyword, WardCost};
    use crate::types::mana::ManaCost;
    use crate::types::player::PlayerId;
    use crate::types::zones::Zone;

    #[test]
    fn cloak_top_card_enters_face_down_with_ward_two() {
        let mut state = GameState::new_two_player(42);
        let player = PlayerId(0);
        let card = create_object(
            &mut state,
            CardId(70158),
            player,
            "Cloaked Card".to_string(),
            Zone::Library,
        );
        let ability = ResolvedAbility::new(
            Effect::Cloak {
                target: TargetFilter::Controller,
                count: QuantityExpr::Fixed { value: 1 },
                object_source: None,
            },
            vec![],
            ObjectId(999),
            player,
        );

        let mut events = Vec::new();
        resolve(&mut state, &ability, &mut events).unwrap();

        let obj = &state.objects[&card];
        assert_eq!(obj.zone, Zone::Battlefield);
        assert!(obj.face_down);
        assert_eq!(obj.power, Some(2));
        assert_eq!(obj.toughness, Some(2));
        // allow-raw-authority: unit test asserts the exact Ward {2} cost the cloak profile grants on the raw keyword vec
        assert!(obj.keywords.iter().any(|keyword| matches!(
            keyword,
            Keyword::Ward(WardCost::Mana(cost)) if *cost == ManaCost::generic(2)
        )));
        assert!(events
            .iter()
            .any(|event| matches!(event, GameEvent::ZoneChanged { object_id, to, .. } if *object_id == card && *to == Zone::Battlefield)));
    }
}
