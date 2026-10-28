//! A static gated on the first combat phase follows every step boundary.

use super::rules::{GameAction, GameRunner, GameScenario, ObjectId, Phase, WaitingFor, P0, P1};
use engine::game::perf_counters;
use engine::types::ability::EffectKind;
use engine::types::events::GameEvent;
use engine::types::mana::{ManaCost, ManaCostShard, ManaType, ManaUnit};

const GATED: &str = "As long as it's the first combat phase of the turn, Gated Bear gets +2/+0.";
const MANDATE: &str = "Cast this spell only during combat.\nYour opponents can't cast spells this turn.\nEnd the combat phase. (Remove all attackers and blockers from combat. Exile all spells and abilities from the stack, including this spell.)";
const TIME_STOP: &str = "End the turn. (Exile all spells and abilities, including this spell. The player whose turn it is discards down to their maximum hand size. Damage heals and \"this turn\" and \"until end of turn\" effects end.)";
const SAVAGE_BEATING: &str = "Cast this spell only during combat on your turn.\nChoose one —\n• Creatures you control gain double strike until end of turn.\n• Untap all creatures you control. After this phase, there is an additional combat phase.\nEntwine {1}{R} (Choose both if you pay the entwine cost.)";

fn scenario() -> GameScenario {
    let mut s = GameScenario::new();
    s.at_phase(Phase::PreCombatMain);
    s.with_library_top(P0, &["Filler A", "Filler B"]);
    s.with_library_top(P1, &["Filler C", "Filler D"]);
    s
}

fn power(r: &GameRunner, id: ObjectId) -> i32 {
    r.state().objects[&id].power.unwrap()
}

fn fund(r: &mut GameRunner, color: ManaType, count: usize) {
    for _ in 0..count {
        r.state_mut().players[0]
            .mana_pool
            .add(ManaUnit::new(color, ObjectId(0), false, vec![]));
    }
}

/// Enters the first combat, declares no attackers, and checks the gate is on.
fn empty_first_combat(r: &mut GameRunner, id: ObjectId) {
    r.advance_to_combat();
    assert!(matches!(
        r.state().waiting_for,
        WaitingFor::DeclareAttackers { .. }
    ));
    assert_eq!(power(r, id), 4, "CR 611.3a: on as the first combat begins");
    r.declare_attackers(&[]).unwrap();
    assert_eq!(r.state().phase, Phase::EndCombat);
    assert_eq!(
        r.state().combat.as_ref(),
        Some(&Default::default()),
        "the combat stays untouched"
    );
    assert_eq!(power(r, id), 4);
}

fn assert_priority(r: &GameRunner) {
    assert!(r.state().stack.is_empty());
    assert!(matches!(
        r.state().waiting_for,
        WaitingFor::Priority { player: P0 }
    ));
}

// CR 511.3 + CR 611.3a: the natural End Combat exit.
#[test]
fn first_combat_static_ends_with_an_untouched_natural_combat() {
    let mut s = scenario();
    let id = s
        .add_creature_from_oracle(P0, "Gated Bear", 2, 2, GATED)
        .id();
    let mut r = s.build();
    empty_first_combat(&mut r, id);
    for _ in 0..2 {
        r.act(GameAction::PassPriority).unwrap();
    }
    assert_eq!(r.state().phase, Phase::PostCombatMain);
    assert!(r.state().combat.is_none());
    assert_priority(&r);
    assert_eq!(power(&r, id), 2);
}

// CR 724.2d + CR 611.3a: Mandate of Peace ends an untouched combat.
#[test]
fn first_combat_static_ends_when_an_untouched_combat_phase_is_ended() {
    let mut s = scenario();
    let id = s
        .add_creature_from_oracle(P0, "Gated Bear", 2, 2, GATED)
        .id();
    let mandate = s
        .add_spell_to_hand_from_oracle(P0, "Mandate of Peace", true, MANDATE)
        .with_mana_cost(ManaCost::Cost {
            generic: 1,
            shards: vec![ManaCostShard::White],
        })
        .id();
    let mut r = s.build();
    empty_first_combat(&mut r, id);
    fund(&mut r, ManaType::White, 2);
    let outcome = r.cast(mandate).resolve();
    assert!(outcome.events().iter().any(|event| matches!(event,
        GameEvent::EffectResolved { kind: EffectKind::EndCombatPhase, source_id, .. }
            if *source_id == mandate)));
    assert_eq!(r.state().phase, Phase::PostCombatMain);
    assert!(r.state().combat.is_none());
    assert_priority(&r);
    assert_eq!(power(&r, id), 2);
}

// CR 724.1d + CR 611.3a: ending the turn during an untouched combat.
#[test]
fn first_combat_static_ends_when_the_turn_ends_during_an_untouched_combat() {
    let mut s = scenario();
    let id = s
        .add_creature_from_oracle(P0, "Gated Bear", 2, 2, GATED)
        .id();
    let time_stop = s
        .add_spell_to_hand_from_oracle(P0, "Time Stop", true, TIME_STOP)
        .with_mana_cost(ManaCost::Cost {
            generic: 4,
            shards: vec![ManaCostShard::Blue; 2],
        })
        .id();
    let mut r = s.build();
    let turn = r.state().turn_number;
    empty_first_combat(&mut r, id);
    fund(&mut r, ManaType::Blue, 6);
    let outcome = r.cast(time_stop).resolve();
    assert!(outcome.events().iter().any(|event| matches!(event,
        GameEvent::EffectResolved { kind: EffectKind::EndTheTurn, source_id, .. }
            if *source_id == time_stop)));
    assert_eq!(r.state().phase, Phase::Cleanup);
    assert_eq!(r.state().turn_number, turn);
    assert!(r.state().combat.is_none());
    assert_priority(&r);
    assert_eq!(power(&r, id), 2);
}

// CR 500.8 + CR 611.3a: an added combat phase directly after an untouched one is not the first.
#[test]
fn first_combat_static_ends_when_an_added_combat_follows_an_untouched_one() {
    let mut s = scenario();
    let id = s
        .add_creature_from_oracle(P0, "Gated Bear", 2, 2, GATED)
        .id();
    let beating = s
        .add_spell_to_hand_from_oracle(P0, "Savage Beating", true, SAVAGE_BEATING)
        .with_mana_cost(ManaCost::Cost {
            generic: 3,
            shards: vec![ManaCostShard::Red; 2],
        })
        .id();
    let mut r = s.build();
    empty_first_combat(&mut r, id);
    fund(&mut r, ManaType::Red, 5);
    r.cast(beating).modes(&[1]).resolve();
    assert_priority(&r);
    assert_eq!(r.state().phase, Phase::EndCombat);
    for _ in 0..2 {
        r.act(GameAction::PassPriority).unwrap();
    }
    assert_eq!(r.state().phase, Phase::BeginCombat);
    assert_eq!(
        r.state().steps_started_this_turn.count(Phase::BeginCombat),
        2
    );
    assert_priority(&r);
    assert_eq!(power(&r, id), 2);
}

/// Full layer passes run while passing from End Combat to the postcombat main phase.
fn untouched_combat_exit_full_passes(oracle: Option<&str>) -> u64 {
    let mut s = scenario();
    let id = match oracle {
        Some(text) => s
            .add_creature_from_oracle(P0, "Gated Bear", 2, 2, text)
            .id(),
        None => s.add_creature(P0, "Plain Bear", 2, 2).id(),
    };
    let mut r = s.build();
    r.advance_to_combat();
    r.declare_attackers(&[]).unwrap();
    r.act(GameAction::PassPriority).unwrap();
    assert_eq!(r.state().phase, Phase::EndCombat);
    perf_counters::reset();
    r.act(GameAction::PassPriority).unwrap();
    let passes = perf_counters::snapshot().layers_full_eval;
    assert_eq!(r.state().phase, Phase::PostCombatMain);
    assert_eq!(power(&r, id), 2);
    passes
}

// CR 611.3a: only a live step-gated static makes a step begin re-run the layers.
#[test]
fn untouched_combat_exit_reruns_layers_only_for_a_step_gated_static() {
    assert_eq!(untouched_combat_exit_full_passes(None), 0);
    assert_eq!(untouched_combat_exit_full_passes(Some(GATED)), 1);
}
