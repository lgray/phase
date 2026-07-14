//! PROBE ONLY — NOT FOR MERGE.
//!
//! **The question rev4 never measured: is the detector reachable in `LoopDetectionMode::On`,
//! and if so, what does it DO there?**
//!
//! The offer bridge gates on `state.loop_detection.samples()` (`game/engine.rs:448`), and
//! `samples()` is `matches!(self, On | Interactive)` (`types/game_state.rs:5975-5977`).
//! ⇒ **`On` reaches the object-growth offer bridge too.** Every rev4 measurement set
//! `Interactive`. This file measures `On`.
//!
//! Why it matters: `On`'s only real consumer is the **`combo-verify` corpus classifier**
//! (`analysis/corpus.rs:2039`), which is an OFFLINE auto-resolving harness — and
//! `corpus.rs` contains **zero** references to `WaitingFor::LoopShortcut`. Before rev4 the
//! object-growth bridge armed only on a buyback-paid token-creating SPELL, a shape no
//! corpus row exercises, so `combo-verify` plausibly never saw a `LoopShortcut`. Rev 4
//! widens the arming class to ANY token-creating activated ability.

use engine::game::game_object::AttachTarget;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario};
use engine::types::actions::GameAction;
use engine::types::game_state::{LoopDetectionMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const P0: PlayerId = PlayerId(0);

const PRESENCE_OF_GOND: &str =
    "Enchant creature\nEnchanted creature has \"{T}: Create a 1/1 green Elf Warrior creature token.\"";
const INTRUDER_ALARM: &str =
    "Creatures don't untap during their controllers' untap steps.\nWhenever a creature enters, untap all creatures.";

fn setup(mode: LoopDetectionMode) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    let bear = scenario.add_creature(P0, "Test Bear", 2, 2).id();
    let gond = scenario
        .add_creature_from_oracle(P0, "Presence of Gond", 0, 0, PRESENCE_OF_GOND)
        .as_enchantment()
        .id();
    scenario
        .add_creature_from_oracle(P0, "Intruder Alarm", 0, 0, INTRUDER_ALARM)
        .as_enchantment();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    let obj = runner.state_mut().objects.get_mut(&gond).unwrap();
    obj.attached_to = Some(AttachTarget::Object(bear));
    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());
    (runner, bear)
}

fn granted_ability_index(runner: &GameRunner, bear: ObjectId) -> usize {
    runner
        .state()
        .objects
        .get(&bear)
        .unwrap()
        .abilities
        .iter()
        .position(|a| a.kind == engine::types::ability::AbilityKind::Activated)
        .expect("aura-granted {T} ability")
}

fn settle(runner: &mut GameRunner) {
    for _ in 0..40 {
        match runner.state().waiting_for.clone() {
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => break,
            WaitingFor::Priority { .. } => {
                if runner.act(GameAction::PassPriority).is_err() {
                    break;
                }
            }
            WaitingFor::OrderTriggers { triggers, .. } => {
                let order: Vec<usize> = (0..triggers.len()).collect();
                if runner.act(GameAction::OrderTriggers { order }).is_err() {
                    break;
                }
            }
            _ => break, // LoopShortcut lands here.
        }
    }
}

/// Returns the terminal `waiting_for` discriminant name after one activation + settle.
fn drive(mode: LoopDetectionMode) -> String {
    let (mut runner, bear) = setup(mode);
    let idx = granted_ability_index(&runner, bear);
    runner
        .act(GameAction::ActivateAbility {
            source_id: bear,
            ability_index: idx,
        })
        .expect("first activation is legal");
    settle(&mut runner);

    let w = &runner.state().waiting_for;
    println!("\n===== canary under LoopDetectionMode::{mode:?} =====");
    println!(
        "  samples() (does this mode reach the bridge?) = {}",
        mode.samples()
    );
    match w {
        WaitingFor::LoopShortcut { certificate, .. } => {
            println!("  ⇒ WaitingFor::LoopShortcut  — AN OFFER. Needs a player decision.");
            println!(
                "     unbounded = {:?}  win_kind = {:?}",
                certificate.unbounded, certificate.win_kind
            );
            "LoopShortcut".to_string()
        }
        WaitingFor::GameOver { winner } => {
            println!(
                "  ⇒ WaitingFor::GameOver {{ winner: {winner:?} }} — AUTO-RESOLVED, no offer."
            );
            "GameOver".to_string()
        }
        other => {
            println!("  ⇒ {other:?} — the detector did NOT fire.");
            format!("{other:?}")
                .split_whitespace()
                .next()
                .unwrap_or("?")
                .to_string()
        }
    }
}

/// ⭐ THE QUESTION: does `On` reach the detector, and does it OFFER or AUTO-RESOLVE?
#[test]
fn on_mode_reaches_the_object_growth_detector() {
    let on = drive(LoopDetectionMode::On);
    let interactive = drive(LoopDetectionMode::Interactive);
    let off = drive(LoopDetectionMode::Off);

    println!("\n===== SUMMARY =====");
    println!("  On          => {on}");
    println!("  Interactive => {interactive}");
    println!("  Off         => {off}");

    // #4603: Off must restore pre-feature behavior — never a detector state.
    assert_ne!(off, "LoopShortcut", "Off must never offer");
    assert_ne!(off, "GameOver", "Off must never auto-resolve a loop");

    // Interactive is rev4's measured result.
    assert_eq!(interactive, "LoopShortcut", "Interactive must offer");

    // The open question, asserted so the answer is recorded rather than assumed:
    // the bridge gates on samples() == On|Interactive, so On should ALSO offer.
    assert_eq!(
        on, "LoopShortcut",
        "On gates on samples() too ⇒ it reaches the SAME object-growth offer bridge. \
         If this fails, On and Interactive diverge on the object-growth path."
    );
}
