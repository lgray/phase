//! PROBE ONLY — NOT FOR MERGE. Rev 4 measurements.
//!
//! B1: the STATIC capture predicate at the `ActivateAbility` beat (mirroring the recast
//! arm's `is_token_creating`, casting_costs.rs:6789) arms, survives to the empty-stack
//! bridge (B), and the canary OFFERS.
//!
//! NON-VACUITY: the negative twin (Presence of Gond, NO Intruder Alarm) must ARM THE
//! CAPTURE (the static predicate is identical) yet NOT OFFER — the discrimination happens
//! at the DRIVE (2nd activation illegal ⇒ RecastAbort), not at an upstream conjunct.

use engine::game::game_object::AttachTarget;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario};
use engine::types::actions::GameAction;
use engine::types::game_state::{LoopDetectionMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const P0: PlayerId = PlayerId(0);

const PRESENCE_OF_GOND: &str = "Enchant creature\nEnchanted creature has \"{T}: Create a 1/1 green Elf Warrior creature token.\"";
const INTRUDER_ALARM: &str = "Creatures don't untap during their controllers' untap steps.\nWhenever a creature enters, untap all creatures.";

/// `with_untapper = false` builds the NEGATIVE TWIN.
fn setup(with_untapper: bool) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    let bear = scenario.add_creature(P0, "Test Bear", 2, 2).id();
    let gond = scenario
        .add_creature_from_oracle(P0, "Presence of Gond", 0, 0, PRESENCE_OF_GOND)
        .as_enchantment()
        .id();
    if with_untapper {
        scenario
            .add_creature_from_oracle(P0, "Intruder Alarm", 0, 0, INTRUDER_ALARM)
            .as_enchantment();
    }
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
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
            _ => break, // LoopShortcut lands here — stop driving.
        }
    }
}

/// Drive one activation + settle. Returns (capture_armed, offered).
fn activate_and_settle(with_untapper: bool, tag: &str) -> (bool, bool) {
    let (mut runner, bear) = setup(with_untapper);
    let idx = granted_ability_index(&runner, bear);

    let bf_before = runner.state().battlefield.len();
    runner
        .act(GameAction::ActivateAbility {
            source_id: bear,
            ability_index: idx,
        })
        .expect("first activation is legal");
    let bf_after = runner.state().battlefield.len();
    let armed = runner.state().last_recast_context.is_some();

    println!("\n===== {tag} =====");
    println!("  battlefield BEFORE ActivateAbility = {bf_before}");
    println!(
        "  battlefield AFTER  ActivateAbility = {bf_after}   (stack={})",
        runner.state().stack.len()
    );
    println!(
        "  ⛔ Rev3's DEAD gate `bf.len() > before` = {}",
        bf_after > bf_before
    );
    println!("  ✅ Rev4 STATIC capture armed          = {armed}");

    settle(&mut runner);

    let offered = matches!(runner.state().waiting_for, WaitingFor::LoopShortcut { .. });
    println!("  ⇒ OFFER (WaitingFor::LoopShortcut)    = {offered}");
    if let WaitingFor::LoopShortcut { certificate, .. } = &runner.state().waiting_for {
        println!("     certificate.unbounded = {:?}", certificate.unbounded);
        println!("     certificate.win_kind  = {:?}", certificate.win_kind);
        println!("     certificate.mandatory = {:?}", certificate.mandatory);
    }
    (armed, offered)
}

/// ⭐ B1 DISCHARGE: the canary OFFERS.
#[test]
fn rev4_b1_canary_offers() {
    let (armed, offered) = activate_and_settle(true, "B1+: canary (Gond + Intruder Alarm)");

    // The B1 measurement, asserted: the capture arms at a beat where the battlefield has
    // NOT grown. Rev 3's dynamic gate was false here ⇒ structurally dead.
    assert!(
        armed,
        "the STATIC capture must arm at the ActivateAbility beat"
    );
    assert!(offered, "⭐ the canary must reach WaitingFor::LoopShortcut");
}

/// ⭐⭐ THE DISCRIMINATOR (acceptance #2), and the proof it is NOT VACUOUS.
///
/// The negative twin ARMS the capture identically (same static predicate — the ability IS
/// token-creating) but does NOT offer, because the DRIVE's 2nd activation is illegal (the
/// bear stays tapped with no untapper) ⇒ `Err(RecastAbort)`. If `armed` were false here,
/// the negative would be passing for an UPSTREAM reason and would be vacuous.
#[test]
fn rev4_negative_twin_arms_capture_but_does_not_offer() {
    let (armed, offered) = activate_and_settle(false, "B1-: NEGATIVE TWIN (Gond, NO untapper)");

    assert!(
        armed,
        "NON-VACUITY: the capture MUST still arm (identical static predicate) — otherwise \
         the negative passes upstream of the drive and proves nothing"
    );
    assert!(
        !offered,
        "⭐ the negative twin must NOT offer (2nd activation illegal ⇒ RecastAbort)"
    );
}
