//! Real-board acceptance tests for the CR 732.2a combo detector.
//!
//! These replay an ACTUAL exported 4-player Commander board (debug panel → "Export Game State")
//! rather than a synthetic `GameScenario`. That distinction is the entire point: the shipped
//! acceptance fixture (`loop_shortcut::object_growth_51st_sprout_swarm_covers_and_offers`, via
//! `sprout_swarm_scenario`) builds a board that CANNOT exist in a real game — no lands, an empty
//! library, no auras, and a stub Witherbloom oracle — and every live defect is invisible to it.
//!
//! `#[ignore]`d because they FAIL TODAY. They are the red acceptance gate for the remediation in
//! `.planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md`; they turn green when it lands.
//!
//!     cargo test -p engine --test integration -- --ignored real_board
//!
//! Root cause summary (measured, see the plan): the engine ARMS correctly
//! (`last_recast_context` is captured with the right card/zone/buyback/convoke) and then declines
//! inside `loop_states_cover_modulo_fodder_growth` → `fire_time_conditions_read_growing_class`,
//! a fail-closed firewall that scans EVERY object in EVERY zone. On this board it is tripped by,
//! in order: `Solemn Simulacrum` (sitting in the LIBRARY — also a CR 400.2 hidden-zone violation),
//! a basic `Forest` (`Effect::Mana => Axes::CONSERVATIVE`), and `Freed from the Real` (an
//! ACTIVATED ability body). Each is independently fatal on any real board.

use engine::game::scenario::GameRunner;
use engine::types::game_state::{GameState, LoopDetectionMode, WaitingFor};
use engine::types::identifiers::ObjectId;

/// The exported board. Override with `PHASE_COMBO_REPRO_STATE` to point at another export.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/combo-repro/witherbloom-sprout-swarm-kilo-4p.json"
);

// Ids read out of the export.
const SPROUT_SWARM: ObjectId = ObjectId(415); // in hand, Convoke + Buyback {3}
const SAPROLING: ObjectId = ObjectId(413); // untapped green Saproling TOKEN (convoke fodder)
const WITHERBLOOM: ObjectId = ObjectId(402); // affinity-for-creatures engine; green (B/G)
const KILO: ObjectId = ObjectId(403); // "whenever Kilo becomes tapped, proliferate"
const PENTAD_PRISM: ObjectId = ObjectId(419); // 1 charge counter; the counter-growth sink

/// Load the debug-panel export (`{gameState, waitingFor, legalActions, turnCheckpoints}`).
fn load_board() -> GameState {
    let path = std::env::var("PHASE_COMBO_REPRO_STATE").unwrap_or_else(|_| FIXTURE.to_string());
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read combo repro fixture {path}: {e}"));
    let value: serde_json::Value = serde_json::from_str(&raw).expect("export is valid json");
    let mut state: GameState = serde_json::from_value(
        value
            .get("gameState")
            .expect("debug export wraps the state in `gameState`")
            .clone(),
    )
    .expect("export deserializes into the engine GameState");
    // A deserialized state carries `layers_dirty = Full`; flush as the WASM bridge does.
    engine::game::layers::flush_layers(&mut state);
    state
}

fn name_of(state: &GameState, id: ObjectId) -> &str {
    state.objects.get(&id).map_or("<absent>", |o| &o.name)
}

/// The board is what we think it is. Guards every other test in this file against a stale fixture,
/// and is itself the proof that the detector's inputs are sound (NOT ignored — must always pass).
#[test]
fn real_board_fixture_is_intact() {
    let state = load_board();
    assert_eq!(state.loop_detection, LoopDetectionMode::Interactive);
    assert_eq!(state.players.len(), 4, "4-player Commander board");
    assert!(state.stack.is_empty());
    assert!(matches!(state.waiting_for, WaitingFor::Priority { .. }));

    assert_eq!(name_of(&state, SPROUT_SWARM), "Sprout Swarm");
    assert_eq!(name_of(&state, WITHERBLOOM), "Witherbloom, the Balancer");
    assert_eq!(name_of(&state, KILO), "Kilo, Apogee Mind");
    assert_eq!(name_of(&state, PENTAD_PRISM), "Pentad Prism");

    // The convoke fodder is a TOKEN and green — and Witherbloom is ALSO green with a LOWER
    // ObjectId, which is why `select_convoke_taps` (lowest-id-per-color) taps the engine piece
    // instead of the fodder. See plan §4/A7 (transient intolerance).
    let fodder = state.objects.get(&SAPROLING).expect("saproling");
    assert!(fodder.is_token, "the fodder must be a token");
    assert!(!fodder.tapped);
    assert!(WITHERBLOOM.0 < SAPROLING.0, "engine piece sorts first");

    // The CR 400.2 hidden-zone tripwire: a card in the LIBRARY currently vetoes detection.
    let library_simulacrum = state
        .objects
        .values()
        .any(|o| o.name == "Solemn Simulacrum" && o.zone == engine::types::zones::Zone::Library);
    assert!(
        library_simulacrum,
        "fixture must retain the library card that trips the all-zones observer scan"
    );
}

/// ⭐ ACCEPTANCE (FAILS TODAY): Witherbloom, the Balancer + Sprout Swarm is a genuine infinite
/// (tap 1 creature to convoke, create 1 token, buyback returns the card, affinity zeroes the
/// generic ⇒ +1 creature, zero mana, forever). Casting it with buyback paid MUST offer the
/// CR 732.2a interactive shortcut.
///
/// Today: arming succeeds, the offer never comes — `waiting_for` stays `Priority`.
#[test]
#[ignore = "known failing — real-board combo-detector bug; see .planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md"]
fn real_board_sprout_swarm_offers_loop_shortcut() {
    let mut runner = GameRunner::from_state(load_board());
    let outcome = runner
        .cast(SPROUT_SWARM)
        .accept_optional() // pay buyback {3}
        .convoke_with(&[SAPROLING]) // tap one green Saproling for the {G} pip
        .commit()
        .resolve();

    // Arming is CORRECT today — this half already passes, and pins that the failure is downstream
    // in the cover, not in the capture (so a "fix" that only touches arming is not a fix).
    let ctx = outcome
        .state()
        .last_recast_context
        .as_ref()
        .expect("a buyback-paid, token-creating cast must capture a recast context");
    assert_eq!(ctx.controller, engine::types::player::PlayerId(0));
    assert_eq!(
        ctx.uses_buyback,
        engine::types::game_state::BuybackUsage::Used
    );

    // THE BUG: the offer never arrives.
    assert!(
        matches!(
            outcome.final_waiting_for(),
            WaitingFor::LoopShortcut { proposer, predicted_winner, .. }
                if *proposer == engine::types::player::PlayerId(0) && predicted_winner.is_none()
        ),
        "expected a CR 732.2a LoopShortcut offer to P0 on a REAL board, got {:?}",
        outcome.final_waiting_for()
    );
}

/// ⭐ CR 400.2 INVARIANCE (FAILS TODAY): the detector's verdict must not depend on the contents of
/// a HIDDEN zone. Library and hand are hidden zones — a verdict that changes when a library card
/// changes is a rules violation by construction, independent of the false-negative problem.
///
/// Discriminating: removing every `Solemn Simulacrum` from the library is the ONLY delta. If the
/// verdict moves, the detector is reading hidden information. (Today it declines both ways for
/// other reasons too — a basic Forest also vetoes — so this asserts EQUALITY of verdict, which is
/// the invariant that must hold before AND after the fix.)
#[test]
#[ignore = "known failing — real-board combo-detector bug; see .planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md"]
fn real_board_verdict_is_invariant_under_hidden_zone_contents() {
    let offered = |mut state: GameState| -> bool {
        engine::game::layers::flush_layers(&mut state);
        let mut runner = GameRunner::from_state(state);
        let outcome = runner
            .cast(SPROUT_SWARM)
            .accept_optional()
            .convoke_with(&[SAPROLING])
            .commit()
            .resolve();
        matches!(outcome.final_waiting_for(), WaitingFor::LoopShortcut { .. })
    };

    let baseline = load_board();

    // Same board, minus the library card that currently trips the all-zones observer scan.
    let mut scrubbed = load_board();
    let doomed: Vec<ObjectId> = scrubbed
        .objects
        .values()
        .filter(|o| o.name == "Solemn Simulacrum" && o.zone == engine::types::zones::Zone::Library)
        .map(|o| o.id)
        .collect();
    assert!(!doomed.is_empty(), "fixture must have the library card");
    for id in &doomed {
        scrubbed.objects.remove(id);
        for p in &mut scrubbed.players {
            p.library.retain(|l| l != id);
        }
    }

    let (with_card, without_card) = (offered(baseline), offered(scrubbed));

    // NON-VACUITY: assert the verdict is OFFER, not merely "the same both ways". Without this the
    // test passes trivially today (`false == false` — a Forest vetoes both boards too), which
    // would make it a vacuous guard. With it, the test is only satisfiable once the detector
    // actually certifies this loop, and it then pins that a hidden-zone edit cannot move it.
    assert!(
        with_card,
        "the loop must be certified on the REAL board (with the library intact), got no offer"
    );
    assert_eq!(
        with_card, without_card,
        "CR 400.2: library is a HIDDEN zone — the loop-shortcut verdict must not depend on its \
         contents. It currently does (a Solemn Simulacrum in the library vetoes detection)."
    );
}
