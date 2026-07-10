//! PR-7 Phase 3 — interactive loop-shortcut protocol + APNAP response window.
//!
//! Covers the CR 732.2a/b/c live-detect bridge, `LoopDetectionMode::Interactive`, the
//! `WaitingFor::LoopShortcut`/`RespondToShortcut` states, the `DeclareShortcut`/
//! `RespondToShortcut` actions, the CR 732.4 all-mandatory no-loss draw, and the
//! conservative Shorten → priority window.
//!
//! # Golden discipline (non-circular byte-identity)
//!
//! `GOLDEN_ON` is the exact accumulated `Vec<GameEvent>` Debug string captured from HEAD
//! `dc67bd130` BEFORE the reconcile mode-`match` wrap landed (via a temporary On/Off-only
//! harness run against the UNMODIFIED reconcile body). T-ON replays the same fixture under
//! the wrapped `On` arm and asserts equality — it fails if wrapping the body in the mode
//! `match` perturbed even one event. Because the golden is pre-edit, this is not circular.

use engine::analysis::decision_template::{
    DecisionGroupKey, DecisionKind, DecisionSlot, DecisionTemplate, IterationCount, PinnedDecision,
    ReplayMode, TargetPin, TargetSchedule,
};
use engine::analysis::loop_check::{LoopCertificate, ShortcutProposal, ShortcutResponse, WinKind};
use engine::analysis::resource::BoardDelta;
use engine::game::engine::{apply, EngineError};
use engine::game::scenario::{GameRunner, GameScenario};
use engine::types::actions::GameAction;
use engine::types::events::GameEvent;
use engine::types::game_state::{LoopDetectionMode, WaitingFor, YieldTarget};
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaColor;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

const DRAIN_CLERIC: &str = "Whenever you gain life, each opponent loses 1 life.";
const BLOOD_SIPPER: &str = "Whenever an opponent loses life, you gain 1 life.";
const KICKOFF: &str = "You gain 1 life.";
const SELF_LIFE_ENGINE: &str = "Whenever you gain life, you gain 1 life.";
const LIFE_LOSS_IMMUNE: &str = "Your life total can't change.";

/// The exact accumulated event Debug string of the 2p drain under `On`, captured from
/// HEAD `dc67bd130` on the UNMODIFIED reconcile body. See the module docs.
const GOLDEN_ON: &str = r#"[StackPushed { object_id: ObjectId(3) }, ZoneChanged { object_id: ObjectId(3), from: Some(Hand), to: Stack, record: ZoneChangeRecord { object_id: ObjectId(3), name: "Test Lifegain Kickoff", core_types: [Sorcery], subtypes: [], supertypes: [], keywords: [], trigger_definitions: [], power: None, toughness: None, base_power: None, base_toughness: None, colors: [], mana_value: 0, controller: PlayerId(0), owner: PlayerId(0), from_zone: Some(Hand), cast_from_zone: None, played_from_zone: None, to_zone: Stack, attachments: [], linked_exile_snapshot: [], is_token: false, combat_status: ZoneChangeCombatStatus { attacking: false, blocking: false, blocked: false, attacking_alone: false, blocking_alone: false, defending_player: None }, co_departed: [], entered_incarnation: None, attached_to: None, turn_zone_change_index: 0, is_suspected: false } }, SpellCast { card_id: CardId(3), controller: PlayerId(0), object_id: ObjectId(3) }, PriorityPassed { player_id: PlayerId(1) }, LifeChanged { player_id: PlayerId(0), amount: 1 }, EffectResolved { kind: GainLife, source_id: ObjectId(3) }, ZoneChanged { object_id: ObjectId(3), from: Some(Stack), to: Graveyard, record: ZoneChangeRecord { object_id: ObjectId(3), name: "Test Lifegain Kickoff", core_types: [Sorcery], subtypes: [], supertypes: [], keywords: [], trigger_definitions: [], power: None, toughness: None, base_power: None, base_toughness: None, colors: [], mana_value: 0, controller: PlayerId(0), owner: PlayerId(0), from_zone: Some(Stack), cast_from_zone: None, played_from_zone: None, to_zone: Graveyard, attachments: [], linked_exile_snapshot: [], is_token: false, combat_status: ZoneChangeCombatStatus { attacking: false, blocking: false, blocked: false, attacking_alone: false, blocking_alone: false, defending_player: None }, co_departed: [], entered_incarnation: None, attached_to: None, turn_zone_change_index: 1, is_suspected: false } }, StackResolved { object_id: ObjectId(3) }, PriorityPassed { player_id: PlayerId(1) }, LifeChanged { player_id: PlayerId(1), amount: -1 }, EffectResolved { kind: LoseLife, source_id: ObjectId(1) }, StackResolved { object_id: ObjectId(4) }, PriorityPassed { player_id: PlayerId(1) }, LifeChanged { player_id: PlayerId(0), amount: 1 }, EffectResolved { kind: GainLife, source_id: ObjectId(2) }, StackResolved { object_id: ObjectId(5) }, GameOver { winner: Some(PlayerId(0)) }]"#;

fn life(runner: &GameRunner, p: PlayerId) -> i32 {
    runner
        .state()
        .players
        .iter()
        .find(|pl| pl.id == p)
        .map(|pl| pl.life)
        .unwrap()
}

fn is_eliminated(runner: &GameRunner, p: PlayerId) -> bool {
    runner
        .state()
        .players
        .iter()
        .find(|pl| pl.id == p)
        .map(|pl| pl.is_eliminated)
        .unwrap()
}

/// 2-player self-refilling mutual drain controlled by P0 (constant-depth). P1 starts low so
/// the OFF natural-death stream is short. Returns runner + kick-off sorcery id.
fn setup_2p_drain(mode: LoopDetectionMode) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(P1, 6);
    scenario.add_creature_from_oracle(P0, "Test Drain Cleric", 2, 2, DRAIN_CLERIC);
    scenario.add_creature_from_oracle(P0, "Test Blood Sipper", 2, 2, BLOOD_SIPPER);
    let kickoff = scenario
        .add_spell_to_hand_from_oracle(P0, "Test Lifegain Kickoff", false, KICKOFF)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    (runner, kickoff)
}

/// 2-player drain (as above) but P1 also holds a castable Lightning Bolt off an untapped
/// Mountain — a meaningful priority action that makes the loop OPTIONAL (CR 732.5 probe
/// FALSE). Returns runner + (kickoff, bolt, drain-cleric enabler id).
fn setup_2p_optional_drain(mode: LoopDetectionMode) -> (GameRunner, ObjectId, ObjectId, ObjectId) {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(P1, 20);
    let cleric = scenario
        .add_creature_from_oracle(P0, "Test Drain Cleric", 2, 2, DRAIN_CLERIC)
        .id();
    scenario.add_creature_from_oracle(P0, "Test Blood Sipper", 2, 2, BLOOD_SIPPER);
    scenario.add_basic_land(P1, ManaColor::Red);
    let bolt = scenario.add_bolt_to_hand(P1);
    let kickoff = scenario
        .add_spell_to_hand_from_oracle(P0, "Test Lifegain Kickoff", false, KICKOFF)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    (runner, kickoff, bolt, cleric)
}

/// 3-player growing μ=2 cascade controlled by P0 (both opponents drain), P1 holding a
/// castable Bolt so the loop is OPTIONAL. The ω growing stack means the winner is confirmed
/// via `loop_states_cover_modulo_growth`, not the constant-depth equality.
fn setup_3p_optional_cascade(mode: LoopDetectionMode) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(3, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(P1, 20);
    scenario.with_life(P2, 20);
    scenario.add_creature_from_oracle(P0, "Test Drain Cleric", 2, 2, DRAIN_CLERIC);
    scenario.add_creature_from_oracle(P0, "Test Blood Sipper", 2, 2, BLOOD_SIPPER);
    scenario.add_basic_land(P1, ManaColor::Red);
    scenario.add_bolt_to_hand(P1);
    let kickoff = scenario
        .add_spell_to_hand_from_oracle(P0, "Test Lifegain Kickoff", false, KICKOFF)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    (runner, kickoff)
}

/// 3-player MANDATORY, unstoppable, net-progress, NO-LOSS loop: P0 has a self-refilling
/// "whenever you gain life, you gain 1 life" engine. Nobody drains, nobody can break it
/// (opponents have empty hands / no abilities) ⇒ CR 732.4 draw.
fn setup_3p_draw(mode: LoopDetectionMode) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(3, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(P1, 20);
    scenario.with_life(P2, 20);
    scenario.add_creature_from_oracle(P0, "Test Life Engine", 2, 2, SELF_LIFE_ENGINE);
    let kickoff = scenario
        .add_spell_to_hand_from_oracle(P0, "Test Lifegain Kickoff", false, KICKOFF)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    (runner, kickoff)
}

/// 3-player SUBSET-LETHAL loop: the SAME proven-detected constant-depth mutual drain as
/// `setup_2p_drain` (P0's `DRAIN_CLERIC` + `BLOOD_SIPPER`), embedded in a 3p pod where P2 is
/// IMMUNE to life loss (CR 119.8 "you can't lose life"). So the cycle drains ONLY P1 (sole
/// faller); P2 is a bystander with per-cycle life delta 0 (a second non-faller). Living
/// partition each cycle: fallers = {P1}, non-fallers = {P0, P2} — so `live_mandatory_loop_winner`
/// refuses to name a winner (CR 104.2a). P1 starts very high so it never dies inside the drive
/// window: the test asserts the mid-loop grind (no crown), not a natural CR 704.5a death.
fn setup_3p_subset_lethal(mode: LoopDetectionMode) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(3, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(P1, 1000);
    scenario.with_life(P2, 20);
    scenario.add_creature_from_oracle(P0, "Test Drain Cleric", 2, 2, DRAIN_CLERIC);
    scenario.add_creature_from_oracle(P0, "Test Blood Sipper", 2, 2, BLOOD_SIPPER);
    scenario.add_creature_from_oracle(P2, "Test Bulwark", 2, 2, LIFE_LOSS_IMMUNE);
    let kickoff = scenario
        .add_spell_to_hand_from_oracle(P0, "Test Lifegain Kickoff", false, KICKOFF)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    (runner, kickoff)
}

/// Drive PassPriority/OrderTriggers beats, accumulating events, until a state OTHER than
/// `Priority`/`OrderTriggers` (a `LoopShortcut`/`RespondToShortcut`/`GameOver`/…) or the
/// cap. Returns accumulated events + the terminal `waiting_for`.
fn drive_collect(runner: &mut GameRunner, cap: usize) -> (Vec<GameEvent>, WaitingFor) {
    let mut all: Vec<GameEvent> = Vec::new();
    for _ in 0..cap {
        match runner.state().waiting_for.clone() {
            WaitingFor::Priority { .. } => match runner.act(GameAction::PassPriority) {
                Ok(r) => all.extend(r.events),
                Err(_) => break,
            },
            WaitingFor::OrderTriggers { triggers, .. } => {
                let order: Vec<usize> = (0..triggers.len()).collect();
                match runner
                    .act(GameAction::OrderTriggers { order })
                    .or_else(|_| runner.act(GameAction::OrderTriggers { order: vec![] }))
                {
                    Ok(r) => all.extend(r.events),
                    Err(_) => break,
                }
            }
            _ => break,
        }
    }
    (all, runner.state().waiting_for.clone())
}

// ────────────────────────────── T-OFF ──────────────────────────────

/// T-OFF: the real winning drain under `Off` reaches the natural CR 704.5a SBA death — no
/// ring sampling, no shortcut, no `ResolutionHalted`. Discriminator: the SAME fixture under
/// `Interactive` produces a DIFFERENT outcome (early shortcut, victim positive), proving
/// `Off` runs zero new code.
#[test]
fn off_natural_death_no_shortcut() {
    let (mut runner, kickoff) = setup_2p_drain(LoopDetectionMode::Off);
    let out = runner.cast(kickoff).resolve();
    let mut all: Vec<GameEvent> = out.events().to_vec();
    let (rest, wf) = drive_collect(&mut runner, 2000);
    all.extend(rest);

    assert_eq!(
        wf,
        WaitingFor::GameOver { winner: Some(P0) },
        "OFF: the drain still ends the game for P0, via the NATURAL CR 704.5a death"
    );
    // Natural-death signature: the victim actually crossed 0 and was eliminated.
    assert!(
        life(&runner, P1) <= 0 && is_eliminated(&runner, P1),
        "OFF: P1 must have drained to <= 0 and been eliminated (no early shortcut)"
    );
    // Off runs zero new code: the ring is never populated and no shortcut/halt occurs.
    assert!(
        runner.state().loop_detect_ring.is_empty(),
        "OFF: the loop-detect ring must be empty (sampler gated off)"
    );
    assert!(
        runner.state().unbounded_resources.is_empty(),
        "OFF: no unbounded axes marked (the detector never ran)"
    );
    assert!(
        !all.iter()
            .any(|e| matches!(e, GameEvent::ResolutionHalted { .. })),
        "OFF: no ResolutionHalted — the natural death ends it cleanly"
    );

    // Discriminator: the SAME fixture under Interactive ends DIFFERENTLY (mandatory
    // winning drain → early auto-win with the victim still at positive life).
    let (mut irunner, ikickoff) = setup_2p_drain(LoopDetectionMode::Interactive);
    let _ = irunner.cast(ikickoff).resolve();
    let (_ievents, iwf) = drive_collect(&mut irunner, 500);
    assert_eq!(
        iwf,
        WaitingFor::GameOver { winner: Some(P0) },
        "Interactive: mandatory winning drain auto-wins for P0"
    );
    assert!(
        life(&irunner, P1) > 0,
        "Interactive: the shortcut fired EARLY — P1 still positive ({}), unlike OFF (<=0)",
        life(&irunner, P1)
    );
}

// ────────────────────────────── T-ON ──────────────────────────────

/// T-ON ⭐: the same lethal drain under `On`, byte-identical to the pre-PR-7 event stream
/// (`GOLDEN_ON`, captured from HEAD before the mode-`match` wrap). Fails if wrapping the
/// body perturbed even one event.
#[test]
fn on_shortcut_byte_identical_to_pre_pr7_golden() {
    let (mut runner, kickoff) = setup_2p_drain(LoopDetectionMode::On);
    let out = runner.cast(kickoff).resolve();
    let mut all: Vec<GameEvent> = out.events().to_vec();
    let (rest, wf) = drive_collect(&mut runner, 500);
    all.extend(rest);

    assert_eq!(
        wf,
        WaitingFor::GameOver { winner: Some(P0) },
        "ON: mandatory winning drain auto-wins for P0"
    );
    assert!(
        life(&runner, P1) > 0,
        "ON: the shortcut fired early (P1 positive)"
    );
    assert_eq!(
        format!("{all:?}"),
        GOLDEN_ON,
        "ON: the accumulated event stream must be byte-identical to the pre-PR-7 golden — \
         wrapping the reconcile body in the mode `match` must not perturb any event"
    );
}

// ────────────────────────── T-3p-cascade ──────────────────────────

/// T-3p-cascade: a ≥3p growing-cascade OPTIONAL winning loop under `Interactive`. The bridge
/// OFFERS a `LoopShortcut` (not an auto-win); the proposer declares `UntilLethal`; both
/// opponents are prompted in APNAP order and Accept ⇒ `GameOver{winner: P0}`, winner via the
/// ω-covering path with the opponents still at positive life.
#[test]
fn interactive_3p_optional_cascade_apnap_accept_win() {
    let (mut runner, kickoff) = setup_3p_optional_cascade(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();
    let (_events, wf) = drive_collect(&mut runner, 500);

    // The OFFER fired (NOT an auto-win): waiting on the proposer to declare the shortcut.
    assert_eq!(
        wf,
        runner.state().waiting_for.clone(),
        "drive stopped at a non-priority state"
    );
    let WaitingFor::LoopShortcut { controller, .. } = wf else {
        panic!("Interactive optional cascade must OFFER a LoopShortcut, got {wf:?}");
    };
    assert_eq!(controller, P0, "the proposer is the determinate winner P0");
    // Fired early — both opponents alive at positive life (ω shortcut, not natural death).
    assert!(
        life(&runner, P1) > 0 && life(&runner, P2) > 0 && !is_eliminated(&runner, P1),
        "opponents must be alive at positive life when the offer fires"
    );

    // Proposer declares the shortcut.
    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        })
        .expect("P0 declares the shortcut");

    // APNAP fan-out: first opponent prompted, then the second, both in turn order after P0.
    let WaitingFor::RespondToShortcut {
        player: first,
        remaining_players,
        ..
    } = runner.state().waiting_for.clone()
    else {
        panic!("after Declare, the first opponent must be prompted");
    };
    assert_eq!(
        first, P1,
        "APNAP: first responder is the next player after P0"
    );
    assert_eq!(remaining_players, vec![P2], "APNAP: P2 queued after P1");

    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("P1 accepts");

    let WaitingFor::RespondToShortcut { player: second, .. } = runner.state().waiting_for.clone()
    else {
        panic!("after P1 accepts, P2 must be prompted");
    };
    assert_eq!(second, P2, "APNAP: second responder is P2");

    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("P2 accepts (last) → take the shortcut");

    assert_eq!(
        runner.state().waiting_for,
        WaitingFor::GameOver { winner: Some(P0) },
        "both accepted ⇒ the shortcut resolves to P0's win"
    );
}

// ─────────────────────────── T-3p-draw ────────────────────────────

/// T-3p-draw: a ≥3p MANDATORY, net-progress, no-loss, unstoppable loop draws under
/// `Interactive` (CR 732.4). Discriminator: the SAME fixture under `Off` does NOT draw (it
/// grinds / halts, no §b-B branch), proving the draw is the Interactive path, not a
/// pre-existing outcome.
#[test]
fn interactive_3p_mandatory_no_loss_draw() {
    let (mut runner, kickoff) = setup_3p_draw(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();
    let (_events, wf) = drive_collect(&mut runner, 500);
    assert_eq!(
        wf,
        WaitingFor::GameOver { winner: None },
        "Interactive: an all-mandatory, no-loss, unstoppable net-progress loop is a CR 732.4 draw"
    );

    // Discriminator: under Off the same fixture never draws via §b-B (it grinds to the
    // iteration/growth backstop or keeps going — not GameOver{None} by this branch).
    let (mut orunner, okickoff) = setup_3p_draw(LoopDetectionMode::Off);
    let _ = orunner.cast(okickoff).resolve();
    let (_oevents, owf) = drive_collect(&mut orunner, 500);
    assert_ne!(
        owf,
        WaitingFor::GameOver { winner: None },
        "Off must NOT reach the CR 732.4 net-progress draw (that branch is Interactive-only)"
    );
}

// ────────────────────────── T-Q1-shorten ──────────────────────────

/// T-Q1-shorten ⭐: an OPTIONAL winning drain under `Interactive`. The proposer declares the
/// shortcut; the opponent SHORTENS ⇒ the engine hands THAT opponent a real priority window
/// (CR 732.2c); the opponent casts removal on an enabler ⇒ the loop breaks (no GameOver,
/// re-detection does not re-confirm). Discriminator: replacing Shorten with Accept runs the
/// same fixture to `GameOver{winner: P0}` — proving the WINDOW stopped it, not an unrelated
/// fizzle.
#[test]
fn interactive_shorten_hands_priority_and_breaks_loop() {
    let (mut runner, kickoff, bolt, cleric) =
        setup_2p_optional_drain(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();
    let (_events, wf) = drive_collect(&mut runner, 500);

    let WaitingFor::LoopShortcut { controller, .. } = wf else {
        panic!("optional drain must OFFER a LoopShortcut, got {wf:?}");
    };
    assert_eq!(controller, P0);

    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        })
        .expect("P0 declares");

    // Positive reach-guard: the opponent WAS actually prompted before it responds.
    assert!(
        matches!(
            runner.state().waiting_for,
            WaitingFor::RespondToShortcut { player, .. } if player == P1
        ),
        "P1 must be prompted to respond before shortening"
    );

    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Shorten { at_iteration: 1 },
        })
        .expect("P1 shortens");

    // CR 732.2c: P1 received a real priority window (not the shortcut).
    assert_eq!(
        runner.state().waiting_for,
        WaitingFor::Priority { player: P1 },
        "Shorten hands the shortening opponent a priority window"
    );
    assert!(
        life(&runner, P1) > 0,
        "P1 is alive — the loop was NOT auto-taken"
    );

    // P1 casts removal on an enabler ⇒ the loop breaks.
    let _ = runner.cast(bolt).target_object(cleric).resolve();
    assert!(
        runner.state().objects.get(&cleric).map(|o| o.zone)
            != Some(engine::types::zones::Zone::Battlefield),
        "the drain enabler (Cleric) must have left the battlefield"
    );

    // Re-detection on the next beats does NOT re-confirm the (now-broken) loop.
    let (_r, wf2) = drive_collect(&mut runner, 200);
    assert!(
        !matches!(wf2, WaitingFor::GameOver { winner: Some(_) }),
        "after the enabler is removed, no player is shortcut to a win; got {wf2:?}"
    );
    assert!(
        life(&runner, P1) > 0 && !is_eliminated(&runner, P1),
        "P1 survives — the shorten window genuinely stopped the loop"
    );

    // Discriminator: the SAME fixture with Accept instead of Shorten runs to P0's win.
    let (mut arunner, akickoff, _abolt, _acleric) =
        setup_2p_optional_drain(LoopDetectionMode::Interactive);
    let _ = arunner.cast(akickoff).resolve();
    let (_ae, awf) = drive_collect(&mut arunner, 500);
    assert!(matches!(awf, WaitingFor::LoopShortcut { .. }));
    arunner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        })
        .expect("declare");
    arunner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("accept");
    assert_eq!(
        arunner.state().waiting_for,
        WaitingFor::GameOver { winner: Some(P0) },
        "Accept (not Shorten) ⇒ the loop resolves to P0's win — proves the window stops it"
    );
}

// ───────────────────── T-declare-roundtrip ─────────────────────────

/// T-declare-roundtrip: each protocol action is accepted only from its authorized actor —
/// `DeclareShortcut` from the controller, `RespondToShortcut` from the current responder.
/// A wrong actor is rejected with `WrongPlayer`.
#[test]
fn declare_and_respond_authorization() {
    let (mut runner, kickoff) = setup_3p_optional_cascade(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();
    let (_events, wf) = drive_collect(&mut runner, 500);
    assert!(matches!(wf, WaitingFor::LoopShortcut { controller, .. } if controller == P0));

    // Wrong actor for DeclareShortcut (an opponent) → rejected.
    let wrong = apply(
        runner.state_mut(),
        P1,
        GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        },
    );
    assert!(
        matches!(wrong, Err(EngineError::WrongPlayer)),
        "an opponent may not declare the proposer's shortcut, got {wrong:?}"
    );

    // Correct actor (P0) → accepted; advances to the first responder.
    apply(
        runner.state_mut(),
        P0,
        GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        },
    )
    .expect("P0 declares");
    let WaitingFor::RespondToShortcut { player: first, .. } = runner.state().waiting_for.clone()
    else {
        panic!("expected a RespondToShortcut prompt");
    };

    // Wrong actor for RespondToShortcut (the controller) → rejected.
    let wrong2 = apply(
        runner.state_mut(),
        P0,
        GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        },
    );
    assert!(
        matches!(wrong2, Err(EngineError::WrongPlayer)),
        "the controller may not answer their own shortcut offer, got {wrong2:?}"
    );

    // Correct actor (the prompted opponent) → accepted.
    apply(
        runner.state_mut(),
        first,
        GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        },
    )
    .expect("the prompted opponent accepts");
}

// ─────────────────── T-variant-housekeeping ────────────────────────

/// T-variant-housekeeping: `WaitingFor::LoopShortcut{controller}.acting_player()` reads the
/// `controller` field (routing authorization to the proposer), not a constant.
#[test]
fn loop_shortcut_acting_player_reads_controller() {
    let cert = LoopCertificate {
        unbounded: vec![],
        win_kind: WinKind::LethalDamage,
        mandatory: false,
        residual_board_delta: BoardDelta::default(),
    };
    let wf_a = WaitingFor::LoopShortcut {
        controller: P1,
        certificate: cert.clone(),
    };
    let wf_b = WaitingFor::LoopShortcut {
        controller: P2,
        certificate: cert,
    };
    assert_eq!(wf_a.acting_player(), Some(P1));
    assert_eq!(wf_b.acting_player(), Some(P2));

    // And RespondToShortcut routes to its `player`.
    let proposal = ShortcutProposal {
        controller: P0,
        count: IterationCount::UntilLethal,
        unbounded: vec![],
        win_kind: WinKind::LethalDamage,
        template: None,
    };
    let wf_r = WaitingFor::RespondToShortcut {
        player: P2,
        remaining_players: vec![],
        proposal,
    };
    assert_eq!(wf_r.acting_player(), Some(P2));
}

// ─────────────── T-concede-controller (F1 revert-guard) ────────────────

/// The latched proposer P0 concedes DURING the open APNAP window. `Concede` bypasses the
/// `WaitingFor` dispatch (engine.rs), so `proposal.controller` is never re-validated, and
/// because the acting player (P1) is still alive the elimination self-heal leaves the stale
/// offer standing. When the last opponent accepts, the controller-liveness guard in
/// `apply_confirmed_shortcut` (F1) must REFUSE to crown the departed proposer — CR 104.3a (a
/// player who conceded has lost and cannot be crowned), CR 104.2a (the winner must still be
/// in the game), CR 800.4a (the proposer's loop objects have already left the game) — and
/// hand priority back instead. Reverting F1 makes P2's Accept crown
/// `GameOver{winner: Some(P0)}`, a departed winner, which this test forbids.
#[test]
fn interactive_controller_concede_mid_apnap_does_not_crown_departed() {
    let (mut runner, kickoff) = setup_3p_optional_cascade(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();
    let (_events, wf) = drive_collect(&mut runner, 500);
    let WaitingFor::LoopShortcut { controller, .. } = wf else {
        panic!("optional cascade must OFFER a LoopShortcut, got {wf:?}");
    };
    assert_eq!(controller, P0, "the proposer is the determinate winner P0");

    // P0 declares → APNAP window opens on P1, with P2 queued behind.
    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        })
        .expect("P0 declares");
    let WaitingFor::RespondToShortcut {
        player,
        remaining_players,
        ..
    } = runner.state().waiting_for.clone()
    else {
        panic!(
            "after Declare the APNAP window must open, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(player, P1, "window opens on P1");
    assert_eq!(remaining_players, vec![P2], "P2 queued behind P1");

    // The latched proposer P0 concedes MID-window (CR 104.3a: leaves + loses immediately).
    // The acting player is P1 (alive), so the elimination self-heal does NOT prune the
    // stale proposal — the window survives with a now-departed `proposal.controller`.
    runner
        .act(GameAction::Concede { player_id: P0 })
        .expect("P0 concedes");
    assert!(is_eliminated(&runner, P0), "P0 has left the game");
    assert!(
        matches!(
            runner.state().waiting_for,
            WaitingFor::RespondToShortcut { player, .. } if player == P1
        ),
        "the offer survives the conceder (acting P1 is alive), got {:?}",
        runner.state().waiting_for
    );

    // P1 accepts → advance to P2 (still alive).
    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("P1 accepts");
    assert!(
        matches!(
            runner.state().waiting_for,
            WaitingFor::RespondToShortcut { player, .. } if player == P2
        ),
        "after P1 accepts, P2 (alive) is prompted, got {:?}",
        runner.state().waiting_for
    );

    // P2 accepts (last) → would crown the departed P0 if F1 were reverted.
    let last = runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("P2 accepts (last)");

    // F1: the controller-liveness guard refuses to crown the departed P0 and hands
    // priority back for a later LIVE re-detect. Reverting F1 flips this to
    // GameOver{winner: Some(P0)}.
    assert_ne!(
        runner.state().waiting_for,
        WaitingFor::GameOver { winner: Some(P0) },
        "a departed proposer (P0 conceded) must NOT be crowned (CR 104.2a / 104.3a)"
    );
    match runner.state().waiting_for {
        WaitingFor::Priority { player } => {
            assert!(
                !is_eliminated(&runner, player),
                "F1 must hand priority to a LIVING player (CR 800.4a), not the departed proposer; got Priority {{{player:?}}}"
            );
            assert_ne!(
                player, P0,
                "priority must not return to the conceded proposer P0"
            );
        }
        ref other => panic!("F1 hands priority back (manual fallback), got {other:?}"),
    }
    assert!(
        !last
            .events
            .iter()
            .any(|e| matches!(e, GameEvent::GameOver { winner } if *winner == Some(P0))),
        "no GameOver{{Some(P0)}} event may be emitted for the departed proposer"
    );
}

// ──────────────── T-concede-queued (F2 revert-guard) ────────────────

/// A QUEUED opponent (P2, not yet prompted) concedes AFTER the window opened. `Concede`
/// bypasses the `WaitingFor` dispatch, so `remaining_players` still lists the departed seat.
/// When the prompted opponent (P1) accepts, the liveness filter in
/// `handle_respond_to_shortcut` (F2) must DROP the departed seat and — finding no living
/// remainder — take the shortcut for the still-living proposer P0 instead of advancing onto
/// the departed P2 (CR 800.4a: never wait on a player who has left; F1 then re-validates P0's
/// own liveness before crowning). Reverting F2 makes P1's Accept set
/// `RespondToShortcut{player: P2}` — a permanent wait on a departed player.
#[test]
fn interactive_queued_opponent_concede_no_deadlock() {
    let (mut runner, kickoff) = setup_3p_optional_cascade(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();
    let (_events, wf) = drive_collect(&mut runner, 500);
    let WaitingFor::LoopShortcut { controller, .. } = wf else {
        panic!("optional cascade must OFFER a LoopShortcut, got {wf:?}");
    };
    assert_eq!(controller, P0, "the proposer is the determinate winner P0");

    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        })
        .expect("P0 declares");
    let WaitingFor::RespondToShortcut {
        player,
        remaining_players,
        ..
    } = runner.state().waiting_for.clone()
    else {
        panic!(
            "after Declare the APNAP window must open, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(player, P1, "window opens on P1");
    assert_eq!(remaining_players, vec![P2], "P2 queued behind P1");

    // The QUEUED (not-yet-prompted) opponent P2 concedes. Acting player is P1 (alive), so the
    // self-heal leaves the window on P1 — but `remaining_players` still lists the departed P2.
    runner
        .act(GameAction::Concede { player_id: P2 })
        .expect("P2 concedes");
    assert!(is_eliminated(&runner, P2), "P2 has left the game");
    assert!(
        !is_eliminated(&runner, P0) && !is_eliminated(&runner, P1),
        "P0/P1 remain in the game"
    );

    // P1 accepts. F2 drops departed P2 from the queue; no living remainder ⇒ take the
    // shortcut for the still-living P0 — NOT advance onto departed P2.
    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("P1 accepts (last living opponent)");

    assert!(
        !matches!(
            runner.state().waiting_for,
            WaitingFor::RespondToShortcut { player, .. } if player == P2
        ),
        "must NOT wait on the departed P2 (CR 800.4a), got {:?}",
        runner.state().waiting_for
    );
    assert_eq!(
        runner.state().waiting_for,
        WaitingFor::GameOver { winner: Some(P0) },
        "the last living opponent accepted ⇒ crown the still-living proposer P0"
    );
}

// ───────────── T-subset-lethal (D2 — nonfallers.len()==1 guard) ─────────────

/// D2: a 3p loop that drains ONLY P1 (P2 a bystander, life delta 0) must NOT crown.
/// `live_mandatory_loop_winner` (loop_check.rs) partitions living into fallers/non-fallers and
/// requires `nonfallers.len() == 1` (CR 104.2a — determinate only when EVERY other living
/// player falls); here nonfallers = {P0, P2} (len 2) ⇒ `find_live_loop_winner` returns None,
/// so `interactive_loop_bridge` takes neither Path A (no determinate winner) nor Path B (a
/// life-loss axis is present, so not a CR 732.4 no-loss draw) and falls through to the
/// pre-feature grind.
///
/// REVERT-FAIL: weaken the `nonfallers.len() != 1` gate to an "any-faller wins" rewrite and
/// this MANDATORY loop is wrongly crowned `GameOver{winner: Some(P0)}` — flipping the two
/// no-crown assertions below. (Passes today, proving the gate holds.)
#[test]
fn interactive_3p_subset_lethal_does_not_crown() {
    let (mut runner, kickoff) = setup_3p_subset_lethal(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();
    let (events, wf) = drive_collect(&mut runner, 500);

    // Positive reach-guard: the drain loop genuinely ran on P1 while P2 stayed untouched — we
    // are in the subset-lethal regime the gate must refuse, not an unrelated upstream no-op.
    assert!(
        life(&runner, P1) < 1000 && !is_eliminated(&runner, P1),
        "P1 must have bled (loop ran) but still be alive mid-drive, life = {}",
        life(&runner, P1)
    );
    assert_eq!(
        life(&runner, P2),
        20,
        "P2 is a bystander untouched by the loop (life delta 0 → a second non-faller)"
    );

    // No crown: a subset-lethal loop leaves >1 living non-faller, so no determinate winner.
    assert!(
        !matches!(wf, WaitingFor::GameOver { winner: Some(_) }),
        "subset-lethal loop must NOT crown a winner (CR 104.2a), got {wf:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, GameEvent::GameOver { winner: Some(_) })),
        "no GameOver{{Some}} event may be emitted for a subset-lethal loop"
    );
    // No offer either: the bridge does not OFFER a shortcut for a non-winner loop.
    assert!(
        !matches!(wf, WaitingFor::LoopShortcut { .. }),
        "subset-lethal loop must NOT raise a LoopShortcut offer, got {wf:?}"
    );
}

// ─────────────────── T-B3-materialize (Phase 4b) ───────────────────────

/// Reach `LoopShortcut{P0}` on a fresh `setup_2p_optional_drain(Interactive)` fixture.
/// Returns the runner parked at the offer, `life(P1)` at that instant, and the
/// DRAIN_CLERIC object id (for template pins).
fn reach_2p_optional_drain_offer() -> (GameRunner, i32, ObjectId) {
    let (mut runner, kickoff, _bolt, cleric) =
        setup_2p_optional_drain(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();
    let (_events, wf) = drive_collect(&mut runner, 500);
    let WaitingFor::LoopShortcut { controller, .. } = wf else {
        panic!("optional drain must OFFER a LoopShortcut, got {wf:?}");
    };
    assert_eq!(controller, P0, "the proposer is the determinate winner P0");
    let l0 = life(&runner, P1);
    (runner, l0, cleric)
}

/// Probe the per-cycle P1 drain constant via an independent `Fixed(1)` materialization
/// of the DRAIN_CLERIC/BLOOD_SIPPER pairing (one recurrence = one full cycle).
fn probe_drain_delta() -> i32 {
    let (mut runner, l0, _cleric) = reach_2p_optional_drain_offer();
    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(1),
            template: None,
        })
        .expect("declare Fixed(1)");
    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("accept");
    let delta = l0 - life(&runner, P1);
    assert!(
        delta > 0,
        "Fixed(1) must materialize a nonzero drain cycle, got delta={delta}"
    );
    delta
}

/// A `Fixed(count)` template pinning `object` by `ThisObject{incarnation}` — CR 400.7's
/// per-iteration incarnation re-bind (BLOCKER #4 real teeth).
fn incarnation_pin_template(
    owner: PlayerId,
    object: ObjectId,
    incarnation: u64,
    count: IterationCount,
) -> DecisionTemplate {
    let source = YieldTarget::ThisObject {
        source_id: object,
        incarnation: Some(incarnation),
        trigger_description: None,
    };
    let slot = DecisionSlot {
        source: source.clone(),
        index: 0,
    };
    DecisionTemplate {
        owner,
        decisions: vec![PinnedDecision::Targets {
            slot,
            targets: vec![TargetPin::ByIdentity(source.clone())],
        }],
        replay: ReplayMode::Scheduled { count },
        key: DecisionGroupKey::from_sources(&[source], DecisionKind::LoopChoice),
    }
}

/// A `Fixed(count)` template pinning `cleric` via a PRE-DECLARED (CR 732.2a-predictable)
/// `Piecewise` schedule: iterations `[0, switch)` resolve to `cleric` itself (stable
/// across the drive); at `switch` (if `Some`) the schedule switches to a bogus,
/// never-resolvable `ObjectId` — simulating "the pinned object left the game" at exactly
/// that iteration, entirely from the schedule (no mid-drive test backdoor).
fn piecewise_cleric_template(
    owner: PlayerId,
    cleric: ObjectId,
    switch_to_bogus_at: Option<u32>,
    count: IterationCount,
) -> DecisionTemplate {
    let valid = YieldTarget::ThisObject {
        source_id: cleric,
        incarnation: None,
        trigger_description: None,
    };
    let bogus = YieldTarget::ThisObject {
        source_id: ObjectId(u64::MAX),
        incarnation: None,
        trigger_description: None,
    };
    let slot = DecisionSlot {
        source: valid.clone(),
        index: 0,
    };
    let mut schedule = vec![(0u32, valid.clone())];
    if let Some(at) = switch_to_bogus_at {
        schedule.push((at, bogus));
    }
    DecisionTemplate {
        owner,
        decisions: vec![PinnedDecision::Targets {
            slot,
            targets: vec![TargetPin::Scheduled(TargetSchedule::Piecewise(schedule))],
        }],
        replay: ReplayMode::Scheduled { count },
        key: DecisionGroupKey::from_sources(&[valid], DecisionKind::LoopChoice),
    }
}

/// B3-materialize-stop-short ⭐ (N < cycles-to-lethal): P1's life must drop EXACTLY
/// `N*delta` — a NON-ZERO multiple. This is the empirical BLOCKER #2 gate: if the
/// per-cycle recurrence boundary is unseeded (`waiting_for` never re-matches
/// `Priority{active}`), the drive spins to `cycle_beat_cap` every iteration and aborts at
/// 0 complete cycles, so drop==0 and this assertion FAILS; under the pre-4b decline-stub,
/// drop==0 too — both revert targets are caught by the same assertion.
#[test]
fn b3_materialize_stop_short() {
    let delta = probe_drain_delta();
    let (mut runner, l0, _cleric) = reach_2p_optional_drain_offer();
    let n: u32 = 3;
    assert!(
        (n as i32) * delta < l0,
        "test precondition: N*delta must stay short of lethal (l0={l0}, delta={delta})"
    );

    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(n),
            template: None,
        })
        .expect("declare Fixed(N)");
    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("accept");

    assert_eq!(
        life(&runner, P1),
        l0 - (n as i32) * delta,
        "P1 life must drop EXACTLY N*delta"
    );
    assert!(
        !is_eliminated(&runner, P1),
        "P1 must remain alive (N below cycles-to-lethal)"
    );
    assert!(
        !matches!(runner.state().waiting_for, WaitingFor::GameOver { .. }),
        "must not reach GameOver, got {:?}",
        runner.state().waiting_for
    );
    assert_eq!(
        runner.state().waiting_for,
        WaitingFor::Priority { player: P0 },
        "materialization stops at Priority{{living_priority_seat}} (P0) — manual fallback, \
         not a wrong-crown or a stuck handback"
    );
    assert!(
        runner.state().loop_detect_ring.is_empty(),
        "the ring must be cleared on stop-short (Q3) so the same apply() does not instantly \
         re-offer"
    );
}

/// B3-materialize-cross-lethal ⭐ (N ≥ cycles-to-lethal, un-clamped per Q2): commits and
/// stops at a determinate GameOver mid-drive instead of rolling back to manual play.
/// Revert-failing / discriminating vs stop-short: under a flat "non-Priority ⇒ rollback"
/// reducer (the pre-4b decline-stub, or a naive unconditional-abort materializer), this
/// reverts to manual play — P1 SURVIVES at positive life and `waiting_for == Priority` —
/// flipping every assertion below. The stop-short/cross-lethal PAIR (same fixture, N
/// below vs comfortably above cycles-to-lethal) is the discriminator.
#[test]
fn b3_materialize_cross_lethal() {
    let (mut runner, l0, _cleric) = reach_2p_optional_drain_offer();
    // Un-clamped (Q2): N is comfortably past any plausible per-cycle delta >= 1, so this
    // exercises N far beyond cycles-to-lethal without needing the exact probed delta.
    let n: u32 = (l0 as u32) * 2 + 10;
    let unbounded_before = runner.state().unbounded_resources.clone();

    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(n),
            template: None,
        })
        .expect("declare Fixed(N)");
    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("accept");

    assert_eq!(
        runner.state().waiting_for,
        WaitingFor::GameOver { winner: Some(P0) },
        "N >= cycles-to-lethal must COMMIT + STOP at a determinate GameOver mid-drive"
    );
    assert!(
        life(&runner, P1) <= 0 && is_eliminated(&runner, P1),
        "P1 must be dead (drained to <=0), NOT rolled back to positive life"
    );
    assert_eq!(
        runner.state().unbounded_resources,
        unbounded_before,
        "a finite Fixed(N) drain must NOT mark_unbounded_loop (finite != unbounded, contrast \
         the UntilLethal arm)"
    );
}

/// B3-firewall-abort (BLOCKER #4 real teeth, hostile): `resolve()`'s CR 400.7 incarnation
/// re-bind is the load-bearing per-iteration firewall — `predictability_gate(t, &[])` is a
/// wired FORMAL no-op this phase (empty `required_slots`; its own discriminating coverage
/// is the pre-existing `decision_template.rs` unit tests, not re-claimed here).
/// Positive/negative pair on the SAME template pinning DRAIN_CLERIC by
/// `ThisObject{incarnation}`: incarnation stable ⇒ N cycles materialize; incarnation
/// bumped (simulating a leave+re-entry) BEFORE the drive starts ⇒ `resolve` fails on
/// iteration 0 ⇒ abort at 0 complete cycles, priority handback, loop broken.
#[test]
fn b3_firewall_abort_incarnation_guard() {
    let delta = probe_drain_delta();
    let n: u32 = 3;

    // Positive: incarnation stable across the whole drive.
    let (mut runner, l0, cleric) = reach_2p_optional_drain_offer();
    let inc = runner
        .state()
        .objects
        .get(&cleric)
        .expect("cleric on battlefield")
        .incarnation;
    let template = incarnation_pin_template(P0, cleric, inc, IterationCount::Fixed(n));
    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(n),
            template: Some(template),
        })
        .expect("declare");
    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("accept");
    assert_eq!(
        life(&runner, P1),
        l0 - (n as i32) * delta,
        "stable incarnation ⇒ resolve() succeeds every iteration ⇒ all N cycles materialize"
    );
    assert!(!is_eliminated(&runner, P1));

    // Negative (hostile): bump the pinned object's incarnation AFTER Declare but BEFORE
    // Accept — simulating a leave+re-entry inside the still-open window — while the
    // template still carries the STALE incarnation it was pinned with.
    let (mut runner2, l0b, cleric2) = reach_2p_optional_drain_offer();
    let inc2 = runner2
        .state()
        .objects
        .get(&cleric2)
        .expect("cleric on battlefield")
        .incarnation;
    let template2 = incarnation_pin_template(P0, cleric2, inc2, IterationCount::Fixed(n));
    runner2
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(n),
            template: Some(template2),
        })
        .expect("declare");
    runner2
        .state_mut()
        .objects
        .get_mut(&cleric2)
        .expect("cleric on battlefield")
        .incarnation += 1;
    runner2
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("accept");

    assert_eq!(
        life(&runner2, P1),
        l0b,
        "stale-incarnation resolve() failure must abort at 0 complete cycles (no drain leaked)"
    );
    assert!(!is_eliminated(&runner2, P1));
    assert_eq!(
        runner2.state().waiting_for,
        WaitingFor::Priority { player: P0 },
        "abort hands priority back to living_priority_seat (P0), not a wrong-crown"
    );
    assert!(runner2.state().loop_detect_ring.is_empty());
}

/// B3-abort-rollback-live (CR 608.2b + atomicity): a PRE-DECLARED `Piecewise` schedule
/// pins DRAIN_CLERIC for cycles `[0, k)` then switches to a never-resolvable object at
/// cycle `k` — simulating "the enabler leaves the game" exactly at the k-th iteration,
/// entirely from the schedule (no mid-drive test backdoor). Asserts the drained life is
/// an EXACT multiple `k*delta` — no partial-cycle leak: the aborting iteration k's `ev`
/// must have been dropped, not merged. Negative pair: the SAME schedule shape with the
/// switch point placed past N materializes all N cycles untouched.
#[test]
fn b3_abort_rollback_live_atomicity() {
    let delta = probe_drain_delta();
    let n: u32 = 8;
    let k: u32 = 3;
    assert!(
        k < n,
        "test setup: abort must land strictly before N completes"
    );

    // Negative pair: switch point past N ⇒ no removal ⇒ all N cycles commit.
    let (mut clean_runner, l0_clean, cleric_clean) = reach_2p_optional_drain_offer();
    let clean_template =
        piecewise_cleric_template(P0, cleric_clean, Some(n + 100), IterationCount::Fixed(n));
    clean_runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(n),
            template: Some(clean_template),
        })
        .expect("declare");
    clean_runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("accept");
    assert_eq!(
        life(&clean_runner, P1),
        l0_clean - (n as i32) * delta,
        "no removal ⇒ all N cycles commit"
    );

    // Positive (hostile): switch point AT k ⇒ cycles [0,k) commit, cycle k aborts.
    let (mut runner, l0, cleric) = reach_2p_optional_drain_offer();
    let template = piecewise_cleric_template(P0, cleric, Some(k), IterationCount::Fixed(n));
    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(n),
            template: Some(template),
        })
        .expect("declare");
    runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("accept");

    assert_eq!(
        life(&runner, P1),
        l0 - (k as i32) * delta,
        "rollback must land at EXACTLY k complete cycles — no partial (aborting) cycle leaked"
    );
    assert!(!is_eliminated(&runner, P1));
    assert_eq!(
        runner.state().waiting_for,
        WaitingFor::Priority { player: P0 },
        "abort hands priority back to living_priority_seat (P0)"
    );
    assert!(runner.state().loop_detect_ring.is_empty());
}
