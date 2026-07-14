//! PROBE ONLY — NOT FOR MERGE.
//!
//! M0: build CR 732.2a's own worked example (MagicCompRules.txt:6373) LIVE and drive it
//! through the real production path. Presence of Gond enchanting a vanilla creature +
//! Intruder Alarm, all on the battlefield, `loop_detection = Interactive`.
//!
//! Oracle text is byte-copied from `data/card-data.json`.

use engine::game::game_object::AttachTarget;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario};
use engine::types::actions::GameAction;
use engine::types::card_type::CoreType;
use engine::types::game_state::{LoopDetectionMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const P0: PlayerId = PlayerId(0);

/// `jq -r '.["presence of gond"].oracle_text' data/card-data.json`
const PRESENCE_OF_GOND: &str = "Enchant creature\nEnchanted creature has \"{T}: Create a 1/1 green Elf Warrior creature token.\"";

/// `jq -r '.["intruder alarm"].oracle_text' data/card-data.json`
const INTRUDER_ALARM: &str = "Creatures don't untap during their controllers' untap steps.\nWhenever a creature enters, untap all creatures.";

fn setup(mode: LoopDetectionMode) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);

    // The enchanted creature — a vanilla 2/2, already on the battlefield (not summoning-sick).
    let bear = scenario.add_creature(P0, "Test Bear", 2, 2).id();

    // Presence of Gond — Aura enchantment on the battlefield.
    let gond = scenario
        .add_creature_from_oracle(P0, "Presence of Gond", 0, 0, PRESENCE_OF_GOND)
        .as_enchantment()
        .id();

    // Intruder Alarm — enchantment on the battlefield.
    scenario
        .add_creature_from_oracle(P0, "Intruder Alarm", 0, 0, INTRUDER_ALARM)
        .as_enchantment();

    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;

    // Attach the Aura (CR 303.4) — the layer pass then grants the {T} ability to the bear.
    let obj = runner.state_mut().objects.get_mut(&gond).unwrap();
    obj.attached_to = Some(AttachTarget::Object(bear));
    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());

    (runner, bear)
}

fn creature_count(runner: &GameRunner) -> usize {
    let s = runner.state();
    s.battlefield
        .iter()
        .filter(|id| {
            s.objects
                .get(id)
                .is_some_and(|o| o.card_types.core_types.contains(&CoreType::Creature))
        })
        .count()
}

fn dump(tag: &str, runner: &GameRunner) {
    let s = runner.state();
    println!(
        "  [{tag:<22}] wf={:<28} stack={} ring={} creatures={} last_recast={} unbounded={:?}",
        format!("{:?}", s.waiting_for)
            .chars()
            .take(28)
            .collect::<String>(),
        s.stack.len(),
        s.loop_detect_ring.len(),
        creature_count(runner),
        s.last_recast_context.is_some(),
        s.unbounded_resources,
    );
}

/// Find the granted `{T}: Create a 1/1 Elf Warrior` activated ability index on the bear.
fn granted_ability_index(runner: &GameRunner, bear: ObjectId) -> Option<usize> {
    let s = runner.state();
    let obj = s.objects.get(&bear)?;
    obj.abilities
        .iter()
        .position(|a| a.kind == engine::types::ability::AbilityKind::Activated)
}

/// Pass priority until the stack drains or we hit a non-Priority/OrderTriggers prompt.
fn settle(runner: &mut GameRunner, tag: &str) {
    for i in 0..40 {
        match runner.state().waiting_for.clone() {
            WaitingFor::Priority { .. } if runner.state().stack.is_empty() => break,
            WaitingFor::Priority { .. } => {
                if runner.act(GameAction::PassPriority).is_err() {
                    println!("  [{tag}] PassPriority ERR at step {i}");
                    break;
                }
            }
            WaitingFor::OrderTriggers { triggers, .. } => {
                let order: Vec<usize> = (0..triggers.len()).collect();
                if runner
                    .act(GameAction::OrderTriggers { order })
                    .or_else(|_| runner.act(GameAction::OrderTriggers { order: vec![] }))
                    .is_err()
                {
                    break;
                }
            }
            other => {
                println!("  [{tag}] HALT on non-priority prompt: {other:?}");
                break;
            }
        }
        dump(&format!("{tag} pass#{i}"), runner);
    }
}

#[test]
fn m0_canary_live_drive() {
    let (mut runner, bear) = setup(LoopDetectionMode::Interactive);

    println!("\n===== M0: CR 732.2a canary (Presence of Gond + Intruder Alarm), Interactive =====");
    dump("initial", &runner);

    let idx = granted_ability_index(&runner, bear);
    println!("  granted activated-ability index on bear: {idx:?}");
    let Some(ability_index) = idx else {
        let s = runner.state();
        let o = s.objects.get(&bear).unwrap();
        panic!(
            "PROBE SETUP FAILED: no granted activated ability on the bear. abilities={:?}",
            o.abilities.iter().map(|a| a.kind).collect::<Vec<_>>()
        );
    };

    for iter in 0..3 {
        println!("\n--- iteration {iter} ---");
        let before = creature_count(&runner);
        match runner.act(GameAction::ActivateAbility {
            source_id: bear,
            ability_index,
        }) {
            Ok(_) => dump(&format!("it{iter} activated"), &runner),
            Err(e) => {
                println!("  it{iter} ActivateAbility ERR: {e:?}");
                let s = runner.state();
                println!("  bear tapped={:?}", s.objects.get(&bear).map(|o| o.tapped));
                break;
            }
        }
        settle(&mut runner, &format!("it{iter}"));
        dump(&format!("it{iter} SETTLED"), &runner);
        println!(
            "  creatures {} -> {} (loop grows board: {})",
            before,
            creature_count(&runner),
            creature_count(&runner) > before
        );

        if let WaitingFor::LoopShortcut { .. } = runner.state().waiting_for {
            println!("  ⭐⭐ OFFER FIRED at iteration {iter}");
            return;
        }
    }

    println!("\n===== M0 VERDICT =====");
    println!("  final wf           = {:?}", runner.state().waiting_for);
    println!(
        "  OFFER fired?       = {}",
        matches!(runner.state().waiting_for, WaitingFor::LoopShortcut { .. })
    );
    println!(
        "  MARK fired?        = {} ({:?})",
        !runner.state().unbounded_resources.is_empty(),
        runner.state().unbounded_resources
    );
    println!(
        "  ring len           = {}",
        runner.state().loop_detect_ring.len()
    );
    println!("  creatures          = {}", creature_count(&runner));
}

// ═══════════════════════════════════════════════════════════════════════════
// M3 / M4 / M5: evaluate the covers + the firewall on REAL consecutive settle
// frames captured from the live drive above (the exact frames bridge (B) at
// engine.rs:445-451 evaluates: empty stack, Priority, post-settle).
// ═══════════════════════════════════════════════════════════════════════════

use engine::analysis::resource::probe;
use engine::types::game_state::GameState;

/// Drive the canary and capture the settle frame after each iteration.
fn capture_settle_frames(n: usize) -> Vec<GameState> {
    let (mut runner, bear) = setup(LoopDetectionMode::Interactive);
    let ability_index = granted_ability_index(&runner, bear).expect("granted ability");
    let mut frames = vec![runner.state().clone()];
    for i in 0..n {
        runner
            .act(GameAction::ActivateAbility {
                source_id: bear,
                ability_index,
            })
            .unwrap_or_else(|e| panic!("activate {i}: {e:?}"));
        settle(&mut runner, &format!("cap{i}"));
        frames.push(runner.state().clone());
    }
    frames
}

/// The single new battlefield object between two frames (the reproduced token class).
fn derived_fodder(before: &GameState, after: &GameState) -> engine::game::game_object::GameObject {
    let new_ids: Vec<_> = after
        .battlefield
        .iter()
        .copied()
        .filter(|id| !before.battlefield.contains(id))
        .collect();
    assert_eq!(
        new_ids.len(),
        1,
        "expected exactly one new battlefield object"
    );
    after.objects.get(&new_ids[0]).cloned().unwrap()
}

#[test]
fn m3_m4_covers_and_firewall_on_live_frames() {
    let frames = capture_settle_frames(3);
    println!("\n===== M3/M4: covers + firewall on LIVE settle frames =====");
    for (i, f) in frames.iter().enumerate() {
        println!(
            "  frame[{i}] bf={} stack={} wf={:?}",
            f.battlefield.len(),
            f.stack.len(),
            f.waiting_for
        );
    }

    // ---- M4: which cover covers the canary? ----
    for pair in [(1usize, 2usize), (2, 3)] {
        let (a, b) = (&frames[pair.0], &frames[pair.1]);
        println!("\n--- pair (frame{}, frame{}) ---", pair.0, pair.1);
        println!(
            "  loop_states_equal_modulo_resources   = {}",
            probe::cover_equal_modulo_resources(a, b)
        );
        println!(
            "  loop_states_cover_modulo_growth      = {}",
            probe::cover_modulo_growth(a, b)
        );
        println!(
            "  loop_states_cover_modulo_counter_gr. = {}",
            probe::cover_modulo_counter_growth(a, b)
        );
        println!(
            "  loop_states_cover_modulo_OBJECT_gr.  = {}   ⭐",
            probe::cover_object_growth(a, b)
        );
        let mut fodder = derived_fodder(a, b);
        println!(
            "    fodder class: name={:?} tapped={} triggers={} statics={} abilities={} keywords={}",
            fodder.name,
            fodder.tapped,
            probe::obj_trigger_count(&fodder),
            probe::obj_static_count(&fodder),
            fodder.abilities.len(),
            fodder.keywords.len(),
        );
        engine::analysis::resource::probe_project_object_for_loop(&mut fodder);
        println!(
            "  loop_states_cover_modulo_FODDER_gr.  = {}   ⭐",
            probe::cover_fodder_growth(a, b, &fodder)
        );
        println!(
            "  object-growth cover LIMBS: {:#?}",
            probe::object_growth_cover_limbs(a, b)
        );

        // ---- M3: the firewall, per-limb, on the frame the cover feeds it ----
        let cf = probe::flushed(b);
        println!("  FIREWALL(short-circuit) = {}", probe::firewall(&cf));
        println!("  FIREWALL LIMBS = {:#?}", probe::firewall_limbs(&cf));
    }
}
