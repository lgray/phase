//! #7968 — a per-seat projection carries deck-pool contents only while that player's own
//! between-games sideboarding prompt is live (CR 100.4). Outside that prompt every pool's
//! nine registration lists are blanked for every viewer, including the viewer's own.

use std::io::Read;
use std::sync::Arc;

use engine::game::deck_loading::DeckEntry;
use engine::game::interaction::{bind_interaction_authority, derive_viewer_interaction};
use engine::game::visibility::filter_state_for_viewer;
use engine::types::card::CardFace;
use engine::types::format::FormatConfig;
use engine::types::game_state::{GameState, PersistedGameState, PlayerDeckPool, WaitingFor};
use engine::types::interaction::{
    InteractionOpportunityResponse, InteractionPresentationSurface, InteractionResponseSpec,
    InteractionSessionId,
};
use engine::types::match_config::MatchScore;
use engine::types::player::PlayerId;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const SEATS: u8 = 4;

/// Three names that isolate the three lists `sideboard_projection` consumes: one only in
/// `registered_main`, one only in `registered_sideboard`, one in `registered_main` and
/// `current_main` both.
const MAIN_ONLY: &str = "Main Only Card";
const SIDE_ONLY: &str = "Sideboard Only Card";
const IN_BOTH: &str = "Registered And Current Card";

fn load_dump() -> GameState {
    let mut json = String::new();
    flate2::read::GzDecoder::new(
        include_bytes!("../fixtures/dina_conqueror_4p.json.gz").as_slice(),
    )
    .read_to_string(&mut json)
    .expect("fixture .json.gz inflates to UTF-8 JSON");
    let envelope: serde_json::Value =
        serde_json::from_str(&json).expect("dump envelope parses as JSON");
    serde_json::from_value::<PersistedGameState>(envelope["gameState"].clone())
        .expect("gameState deserializes through the production decoder")
        .into_game_state()
        .expect("persisted snapshot satisfies the checked restore contract")
}

fn pool_of(pools: &[PlayerDeckPool], player: PlayerId) -> &PlayerDeckPool {
    pools
        .iter()
        .find(|pool| pool.player == player)
        .expect("every seat has a deck pool")
}

fn json_len(state: &GameState) -> usize {
    serde_json::to_string(state)
        .expect("a projected state serializes")
        .len()
}

fn blank_registration_lists(pool: &mut PlayerDeckPool) {
    pool.registered_main = Arc::new(Vec::new());
    pool.registered_sideboard = Arc::new(Vec::new());
    pool.current_main = Arc::new(Vec::new());
    pool.current_sideboard = Arc::new(Vec::new());
    pool.registered_companion = Arc::new(Vec::new());
    pool.current_companion = Arc::new(Vec::new());
    pool.registered_planar_deck = Arc::new(Vec::new());
    pool.registered_scheme_deck = Arc::new(Vec::new());
    pool.current_scheme_deck = Arc::new(Vec::new());
}

#[test]
fn projection_drops_every_deck_list() {
    let state = load_dump();
    let mut seats_exercised = 0;
    for seat in 0..SEATS {
        let viewer = PlayerId(seat);
        let source = pool_of(&state.deck_pools, viewer);
        // Reach guard: a fixture that lost its pools must fail here rather than
        // satisfy every emptiness assertion below vacuously.
        assert!(!source.registered_main.is_empty());
        assert!(!source.current_main.is_empty());
        assert!(!source.registered_commander.is_empty());

        let projected = filter_state_for_viewer(&state, viewer);
        assert_eq!(projected.deck_pools.len(), state.deck_pools.len());
        for pool in projected.deck_pools.iter() {
            assert!(pool.registered_main.is_empty());
            assert!(pool.registered_sideboard.is_empty());
            assert!(pool.current_main.is_empty());
            assert!(pool.current_sideboard.is_empty());
            assert!(pool.registered_companion.is_empty());
            assert!(pool.current_companion.is_empty());
            assert!(pool.registered_planar_deck.is_empty());
            assert!(pool.registered_scheme_deck.is_empty());
            assert!(pool.current_scheme_deck.is_empty());
            // Commander identity is read off the projection by
            // `derived::derive_display_state`, so an over-broad blanking is red here.
            assert!(!pool.registered_commander.is_empty());
            assert!(!pool.current_commander.is_empty());
        }
        seats_exercised += 1;
    }
    assert_eq!(seats_exercised, SEATS);
}

#[test]
fn projection_sheds_the_viewers_own_pool_payload() {
    let state = load_dump();
    let mut seats_exercised = 0;
    for seat in 0..SEATS {
        let viewer = PlayerId(seat);
        let projected = filter_state_for_viewer(&state, viewer);

        // What the projection sent before this change: the same frame with the
        // viewer's own pool restored verbatim.
        let mut base_like = projected.clone();
        let own = pool_of(&state.deck_pools, viewer).clone();
        *base_like
            .deck_pools
            .iter_mut()
            .find(|pool| pool.player == viewer)
            .expect("the viewer has a pool") = own;

        let mut without_own_pool = base_like.clone();
        blank_registration_lists(
            without_own_pool
                .deck_pools
                .iter_mut()
                .find(|pool| pool.player == viewer)
                .expect("the viewer has a pool"),
        );

        let payload = json_len(&base_like) - json_len(&without_own_pool);
        let shed = json_len(&base_like) - json_len(&projected);
        assert!(
            payload > 0,
            "the fixture carries an own-pool payload to shed"
        );
        assert!(
            shed * 2 > payload,
            "seat {seat} still ships its own deck lists: shed {shed} of {payload}"
        );
        seats_exercised += 1;
    }
    assert_eq!(seats_exercised, SEATS);
}

fn entry(name: &str, count: u32) -> DeckEntry {
    DeckEntry {
        card: CardFace {
            name: name.to_string(),
            ..Default::default()
        },
        count,
    }
}

/// A four-seat state whose live prompt is P1's sideboarding step. Built rather than
/// loaded: every committed dump sits at `Priority` with an empty `registered_sideboard`,
/// so a dump-driven carve-out leg would be red at base and at tip alike.
fn sideboarding_state() -> GameState {
    let mut state = GameState::new(FormatConfig::standard(), SEATS, 7);
    state.deck_pools = (0..SEATS)
        .map(|seat| PlayerDeckPool {
            player: PlayerId(seat),
            registered_main: Arc::new(vec![entry(MAIN_ONLY, 4), entry(IN_BOTH, 2)]),
            registered_sideboard: Arc::new(vec![entry(SIDE_ONLY, 3)]),
            current_main: Arc::new(vec![entry(IN_BOTH, 2)]),
            current_sideboard: Arc::new(vec![entry(MAIN_ONLY, 4), entry(SIDE_ONLY, 3)]),
            ..Default::default()
        })
        .collect();
    state.waiting_for = WaitingFor::BetweenGamesSideboard {
        player: P1,
        game_number: 2,
        score: MatchScore::default(),
        min_main_deck_size: 0,
        max_sideboard_size: Some(15),
    };
    bind_interaction_authority(
        &mut state,
        InteractionSessionId("deck-pool-projection".to_string()),
    )
    .expect("interaction authority binds");
    state
}

/// `(max, total)` of a candidate's `Amount` surface, keyed by the card name its `Value`
/// surface carries. `max` is that name's `registered_main + registered_sideboard`;
/// `total` is its `current_main` count.
fn candidate_amount(
    candidates: &[engine::types::interaction::InteractionChoice],
    name: &str,
) -> (u32, Option<u32>) {
    let choice = candidates
        .iter()
        .find(|choice| {
            choice.surfaces.iter().any(|surface| {
                matches!(surface, InteractionPresentationSurface::Value { value, .. } if value == name)
            })
        })
        .unwrap_or_else(|| panic!("the prompt publishes {name} among its candidates"));
    choice
        .surfaces
        .iter()
        .find_map(|surface| match surface {
            InteractionPresentationSurface::Amount { max, total, .. } => Some((*max, *total)),
            _ => None,
        })
        .expect("every sideboard candidate carries an Amount surface")
}

#[test]
fn sideboard_prompt_keeps_only_its_own_pool() {
    let state = sideboarding_state();

    let for_owner = filter_state_for_viewer(&state, P1);
    let own = pool_of(&for_owner.deck_pools, P1);
    assert!(!own.registered_main.is_empty());
    assert!(!own.registered_sideboard.is_empty());
    assert!(!own.current_main.is_empty());
    for seat in [0u8, 2, 3] {
        let other = pool_of(&for_owner.deck_pools, PlayerId(seat));
        assert!(other.registered_main.is_empty());
        assert!(other.registered_sideboard.is_empty());
        assert!(other.current_main.is_empty());
    }

    // A non-owner's projection of the same state blanks all four pools, including the
    // sideboarding player's — the leak a `waiting_for.is_sideboard()` gate would ship.
    let for_other = filter_state_for_viewer(&state, P0);
    for seat in 0..SEATS {
        let pool = pool_of(&for_other.deck_pools, PlayerId(seat));
        assert!(pool.registered_main.is_empty());
        assert!(pool.registered_sideboard.is_empty());
        assert!(pool.current_main.is_empty());
    }

    let owner_view = derive_viewer_interaction(&state, &for_owner, P1);
    assert!(owner_view.can_submit);
    let [opportunity] = owner_view.opportunities.as_slice() else {
        panic!("the sideboarding player has exactly one opportunity");
    };
    let InteractionOpportunityResponse::Schema { spec, candidates } = &opportunity.response else {
        panic!("the sideboard prompt publishes a schema response");
    };
    let InteractionResponseSpec::DeckPartition { max_main_total, .. } = spec else {
        panic!("the sideboard prompt publishes a deck partition");
    };
    assert!(*max_main_total > 0);
    // The three lists, one assertion each: `registered_main` and `registered_sideboard`
    // reach `max`, `current_main` reaches `total`.
    assert_eq!(candidate_amount(candidates, MAIN_ONLY).0, 4);
    assert_eq!(candidate_amount(candidates, SIDE_ONLY).0, 3);
    assert_eq!(candidate_amount(candidates, IN_BOTH).1, Some(2));

    let other_view = derive_viewer_interaction(&state, &for_other, P0);
    assert!(!other_view.can_submit);
    assert!(other_view.opportunities.is_empty());
}
