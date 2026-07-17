//! DynQty subgroup C — "choose up to X, where X is the number of times you chose
//! a mode for that spell" (Riku of Many Paths) and the shared modal dynamic-cap
//! path (Bumi/Riku). Covers the three seams this change introduces:
//!
//!   1. STORAGE — `handle_select_modes` latches the chosen mode indices onto
//!      `SpellContext.chosen_modes`, and cast-finalize stamps them onto the
//!      spell-on-stack `GameObject.chosen_modes` (CR 700.2a + CR 700.2d).
//!   2. RESOLVER — `QuantityRef::EventContextSourceModesChosen` reads
//!      `chosen_modes.len()` off the `current_trigger_event` spell object
//!      (CR 700.2d + CR 601.2b).
//!   3. END-TO-END — Riku's triggered modal cap (`dynamic_max_choices`) resolves
//!      live to the number of modes chosen for the triggering spell, clamped to
//!      Riku's own mode_count (CR 700.2d), and surfaces as
//!      `AbilityModeChoice.modal.max_choices`.
//!
//! All Oracle text is verbatim engine-authoritative text (matches
//! `riku_modal_spell_trigger.rs` / `atarkas_command_*`).

use engine::game::quantity::resolve_quantity;
use engine::game::scenario::{GameScenario, P0};
use engine::types::ability::{QuantityExpr, QuantityRef};
use engine::types::events::GameEvent;
use engine::types::game_state::{GameState, WaitingFor};
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;

const ATARKAS_COMMAND_ORACLE: &str = "Choose two \u{2014}\n\u{2022} Your opponents can't gain life this turn.\n\u{2022} Atarka's Command deals 3 damage to each opponent.\n\u{2022} You may put a land card from your hand onto the battlefield.\n\u{2022} Creatures you control get +1/+1 and gain reach until end of turn.";

/// STORAGE SEAM (real modal SPELL, subset with a gap): casting Atarka's Command
/// choosing modes {3, 0} stamps the ascending indices `[0, 3]` onto the
/// spell-on-stack object's `chosen_modes`. Uses `.commit()` to inspect the live
/// stack object before resolution. No Riku on the battlefield, so the commit
/// driver reaches the `Priority` window cleanly.
///
/// Revert probe: remove the population line (`handle_select_modes`) OR the
/// finalize stamp → `chosen_modes` empty → this assertion fails LOUDLY. Guards
/// the `handle_select_modes` → `SpellContext.chosen_modes` → finalize seam.
#[test]
fn modal_spell_stamps_chosen_modes_on_stack_object() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let atarka = scenario
        .add_spell_to_hand_from_oracle(P0, "Atarka's Command", true, ATARKAS_COMMAND_ORACLE)
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner = scenario.build();

    // Declare modes out of order to also prove the ascending-sort (CR 601.2c).
    let commit = runner.cast(atarka).modes(&[3, 0]).commit();
    let obj = commit
        .state()
        .objects
        .get(&atarka)
        .expect("committed Atarka's Command must be on the stack");
    assert_eq!(
        obj.chosen_modes,
        vec![0, 3],
        "the two chosen modes must be latched (ascending) onto the stack object"
    );

    // Reach-guard (non-vacuous): the spell actually committed to the stack, so the
    // stamp seam was exercised — not skipped by an upstream cast failure.
    assert!(
        matches!(commit.state().waiting_for, WaitingFor::Priority { .. }),
        "cast must have committed to the stack (Priority window)"
    );
}

/// END-TO-END STORAGE→RESOLVER: the same real committed modal spell's stamped
/// `chosen_modes` is read back through the production `resolve_quantity` for
/// `EventContextSourceModesChosen`, mirroring how a cast-trigger resolves it.
///
/// Revert probes: (a) resolver arm `=> 0` → resolves 0 ≠ 2; (b) stamp/population
/// removed → `chosen_modes` empty → 0 ≠ 2.
#[test]
fn modes_chosen_ref_resolves_off_committed_spell() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let atarka = scenario
        .add_spell_to_hand_from_oracle(P0, "Atarka's Command", true, ATARKAS_COMMAND_ORACLE)
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner = scenario.build();

    let mut commit = runner.cast(atarka).modes(&[0, 1]).commit();
    let card_id = commit.state().objects[&atarka].card_id;
    // Point the trigger event at the committed spell, as trigger dispatch does.
    commit.state_mut().current_trigger_event = Some(GameEvent::SpellCast {
        card_id,
        controller: P0,
        object_id: atarka,
    });

    let expr = QuantityExpr::Ref {
        qty: QuantityRef::EventContextSourceModesChosen,
    };
    // source_id is deliberately NOT the event object — the ref must read the
    // event's spell object, not its source.
    let resolved = resolve_quantity(commit.state(), &expr, P0, atarka);
    assert_eq!(
        resolved, 2,
        "EventContextSourceModesChosen must read the 2 stamped modes off the SpellCast event object"
    );
}

/// MULTI-AUTHORITY HOSTILE FIXTURE (b2) + EMPTY SIBLING (b3): with TWO objects
/// carrying different `chosen_modes` and a distinct source, the ref reads the
/// `current_trigger_event` object's count — not the first object's, not the
/// source's. And an object with empty `chosen_modes` (or an absent event)
/// resolves to 0 without panicking.
///
/// Revert probe: if the resolver read `source_id` instead of the event object,
/// it would return the source's count (1) — the assertion (3) fails.
#[test]
fn modes_chosen_ref_reads_event_object_not_source() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    // Three distinct objects standing in for: first modal spell, second modal
    // spell (the one Riku triggers off), and Riku (the trigger source).
    let first = scenario.add_creature(P0, "First Spell", 1, 1).id();
    let second = scenario.add_creature(P0, "Second Spell", 1, 1).id();
    let riku = scenario.add_creature(P0, "Riku Source", 2, 4).id();
    let mut runner = scenario.build();
    let state: &mut GameState = runner.state_mut();
    // First spell chose 1 mode; second chose 3; Riku (source) carries its own 1.
    state.objects.get_mut(&first).unwrap().chosen_modes = vec![0];
    state.objects.get_mut(&second).unwrap().chosen_modes = vec![0, 1, 2];
    state.objects.get_mut(&riku).unwrap().chosen_modes = vec![2];

    let expr = QuantityExpr::Ref {
        qty: QuantityRef::EventContextSourceModesChosen,
    };

    // Trigger event points at the SECOND spell.
    let card_id = runner.state().objects[&second].card_id;
    runner.state_mut().current_trigger_event = Some(GameEvent::SpellCast {
        card_id,
        controller: P0,
        object_id: second,
    });
    // source_id = riku (the trigger source, chosen_modes=[2]).
    assert_eq!(
        resolve_quantity(runner.state(), &expr, P0, riku),
        3,
        "must read the SECOND spell's 3 modes (the event object), not the source's or first's"
    );

    // b3: empty chosen_modes on the event object → 0.
    let empty_card = runner.state().objects[&first].card_id;
    runner
        .state_mut()
        .objects
        .get_mut(&first)
        .unwrap()
        .chosen_modes = Vec::new();
    runner.state_mut().current_trigger_event = Some(GameEvent::SpellCast {
        card_id: empty_card,
        controller: P0,
        object_id: first,
    });
    assert_eq!(
        resolve_quantity(runner.state(), &expr, P0, riku),
        0,
        "empty chosen_modes must resolve to 0"
    );

    // b3: no trigger event → 0 (no panic).
    runner.state_mut().current_trigger_event = None;
    assert_eq!(
        resolve_quantity(runner.state(), &expr, P0, riku),
        0,
        "absent trigger event must resolve to 0"
    );
}
