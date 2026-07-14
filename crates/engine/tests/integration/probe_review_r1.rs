//! REVIEW PROBE — NOT FOR MERGE. Adversarial review of COMBO-DETECTOR-LIVE-PLAN.rev3.
//!
//! R1: Does Rev3 P1-b's capture gate (`state.battlefield.len() > battlefield_len_before`,
//!     evaluated inside the `GameAction::ActivateAbility` handler) EVER become true for the
//!     CR 732.2a canary? An activated ability goes on the STACK (CR 602.2a); the token is
//!     created on RESOLUTION, a later beat. Measure the battlefield delta AT the activation beat.
//!
//! R2: Rev3 P1-c re-finds the drive source by
//!     `filter(card_id == ctx.card_id && zone == Battlefield && controller == ctx.controller)
//!      .map(id).min_by_key(id.0)`.
//!     With TWO copies of the same card on the battlefield, does it pick the permanent the
//!     player actually activated?

use engine::game::game_object::AttachTarget;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario};
use engine::types::ability::AbilityKind;
use engine::types::actions::GameAction;
use engine::types::game_state::LoopDetectionMode;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const P0: PlayerId = PlayerId(0);

const PRESENCE_OF_GOND: &str = "Enchant creature\nEnchanted creature has \"{T}: Create a 1/1 green Elf Warrior creature token.\"";
const INTRUDER_ALARM: &str = "Creatures don't untap during their controllers' untap steps.\nWhenever a creature enters, untap all creatures.";

fn granted_activated_index(runner: &GameRunner, obj_id: ObjectId) -> Option<usize> {
    runner
        .state()
        .objects
        .get(&obj_id)?
        .abilities
        .iter()
        .position(|a| a.kind == AbilityKind::Activated)
}

/// ⭐ R1 — the P1-b capture gate, measured at the exact instant the plan evaluates it.
#[test]
fn r1_battlefield_does_not_grow_at_the_activateability_beat() {
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
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    let obj = runner.state_mut().objects.get_mut(&gond).unwrap();
    obj.attached_to = Some(AttachTarget::Object(bear));
    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());

    let ability_index = granted_activated_index(&runner, bear).expect("granted {T} ability");

    let bf_before = runner.state().battlefield.len();
    runner
        .act(GameAction::ActivateAbility {
            source_id: bear,
            ability_index,
        })
        .expect("activation legal");
    let bf_at_activation = runner.state().battlefield.len();
    let stack_at_activation = runner.state().stack.len();

    println!("\n===== R1: P1-b capture gate, measured at the ActivateAbility beat =====");
    println!("  battlefield BEFORE ActivateAbility  = {bf_before}");
    println!("  battlefield AFTER  ActivateAbility  = {bf_at_activation}");
    println!("  stack       AFTER  ActivateAbility  = {stack_at_activation}");
    println!(
        "  P1-b GATE (`state.battlefield.len() > battlefield_len_before`) = {}",
        bf_at_activation > bf_before
    );

    // Now settle and show WHERE the growth actually happens.
    for _ in 0..40 {
        if runner.state().stack.is_empty()
            && matches!(
                runner.state().waiting_for,
                engine::types::game_state::WaitingFor::Priority { .. }
            )
        {
            break;
        }
        match runner.state().waiting_for.clone() {
            engine::types::game_state::WaitingFor::Priority { .. } => {
                if runner.act(GameAction::PassPriority).is_err() {
                    break;
                }
            }
            engine::types::game_state::WaitingFor::OrderTriggers { triggers, .. } => {
                let order: Vec<usize> = (0..triggers.len()).collect();
                if runner.act(GameAction::OrderTriggers { order }).is_err() {
                    break;
                }
            }
            _ => break,
        }
    }
    let bf_after_settle = runner.state().battlefield.len();
    println!("  battlefield AFTER settle (PassPriority beats) = {bf_after_settle}");
    println!(
        "  growth happened at the ActivateAbility beat? {}   at a LATER beat? {}",
        bf_at_activation > bf_before,
        bf_after_settle > bf_at_activation
    );

    assert_eq!(
        bf_at_activation, bf_before,
        "R1: battlefield DID grow at the ActivateAbility beat (P1-b's gate would arm)"
    );
    assert!(
        bf_after_settle > bf_at_activation,
        "R1: the token is created on RESOLUTION, not at the activation beat"
    );
}

/// ⭐ R2 — P1-c's `min_by_key(id.0)` re-find with TWO copies of the same card.
#[test]
fn r2_min_by_key_refind_picks_the_wrong_permanent() {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);

    // TWO identical bears. The DECOY is created first ⇒ it gets the LOWER ObjectId.
    let decoy_bear = scenario.add_creature(P0, "Test Bear", 2, 2).id();
    let real_bear = scenario.add_creature(P0, "Test Bear", 2, 2).id();

    let gond = scenario
        .add_creature_from_oracle(P0, "Presence of Gond", 0, 0, PRESENCE_OF_GOND)
        .as_enchantment()
        .id();
    scenario
        .add_creature_from_oracle(P0, "Intruder Alarm", 0, 0, INTRUDER_ALARM)
        .as_enchantment();

    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;

    // The player enchants — and therefore activates — the HIGHER-id bear.
    let obj = runner.state_mut().objects.get_mut(&gond).unwrap();
    obj.attached_to = Some(AttachTarget::Object(real_bear));
    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());

    let s = runner.state();
    let real_card_id = s.objects.get(&real_bear).unwrap().card_id.clone();
    let decoy_card_id = s.objects.get(&decoy_bear).unwrap().card_id.clone();
    let controller = s.objects.get(&real_bear).unwrap().controller;

    println!("\n===== R2: P1-c drive re-find with two same-card permanents =====");
    println!(
        "  decoy_bear id={decoy_bear:?} card_id={decoy_card_id:?} abilities={}",
        s.objects.get(&decoy_bear).unwrap().abilities.len()
    );
    println!(
        "  real_bear  id={real_bear:?} card_id={real_card_id:?} abilities={}",
        s.objects.get(&real_bear).unwrap().abilities.len()
    );
    println!("  same card_id? {}", real_card_id == decoy_card_id);

    // Rev3 P1-c's re-find, verbatim.
    let refound: ObjectId = s
        .objects
        .values()
        .filter(|o| {
            o.card_id == real_card_id && o.zone == Zone::Battlefield && o.controller == controller
        })
        .map(|o| o.id)
        .min_by_key(|id| id.0)
        .expect("a bear exists");

    println!("  the player ACTIVATED       : {real_bear:?}");
    println!("  Rev3's re-find SELECTS     : {refound:?}");
    println!(
        "  ⇒ re-find picked the permanent the player activated? {}",
        refound == real_bear
    );

    let granted_on_real = granted_activated_index(&runner, real_bear);
    let granted_on_decoy = granted_activated_index(&runner, decoy_bear);
    println!("  granted activated index on real_bear  = {granted_on_real:?}");
    println!("  granted activated index on decoy_bear = {granted_on_decoy:?}");

    // What does the drive's `apply_action(ActivateAbility{ source_id: refound, ability_index })` do?
    if let Some(idx) = granted_on_real {
        let res = runner.act(GameAction::ActivateAbility {
            source_id: refound,
            ability_index: idx,
        });
        println!("  drive would apply ActivateAbility{{source_id:{refound:?}, ability_index:{idx}}} => {:?}",
            res.as_ref().map(|_| "Ok").map_err(|e| format!("{e:?}")));
    }

    assert_ne!(
        refound, real_bear,
        "R2: min_by_key re-find selected the permanent the player actually activated"
    );
}
