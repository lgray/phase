//! LIVE-CAPTURE PROBES — env-gated arms that drive a USER'S OWN dump through the production
//! `apply()` beat policy and report where the CR 732.2a bounded offer does or does not appear.
//!
//! **These are DIAGNOSTIC arms, not acceptance rows.** Each is inert (returns immediately, and
//! says so) unless its env var names a dump on disk, because the captures they read are not
//! tracked in this repo — they are user bug-report attachments. What they buy is that the capture
//! which FOUND a defect stays re-runnable against the tree that fixed it, instead of decaying into
//! a screenshot in a journal.
//!
//! Landed so far: the Dina / Bloodthirsty Conqueror arms (`DINA_DUMP`). The F4 lane's arms live on
//! the `probe/f4-no-offer` branch and are not tracked here.
//!
//! # The Dina capture (2026-08-03T19-29-36-888Z) — what it measured
//!
//! A MANDATORY gain/drain trigger chain with no per-iteration choices. The board carries a
//! length-1 `last_loop_action_sequence`:
//!
//! ```text
//! [{ action: Activate { source_id: 268, ability_index: 1 }, controller: 2, pins: [] }]
//! ```
//!
//! 268 is Currency Converter, controlled by an OPPONENT — an activation unrelated to the drain.
//! Before the seat-relative fix, conjunct (1b) refused the bounded offer on ANY non-empty sequence
//! (it was named `DrivingSequenceNotEmpty` then, `ProposerHasDrivingPeriod` now), so that one
//! foreign step suppressed the proposer's offer for the rest of the game.
//!
//! `GameState::migrate_transient_loop_sequence` CLEARS the field at every load that is not a
//! shortcut window, so an in-process replay can never reproduce the live state unless the record
//! is put back. **That asymmetry is the whole point of the pair below**: ARM D1 is what every
//! in-process test sees, ARM D2 is what the running game actually held, and they differ in
//! exactly one field.

use engine::game::engine::apply;
use engine::types::actions::GameAction;
use engine::types::game_state::{GameState, PersistedGameState, WaitingFor};

fn wf_label(w: &WaitingFor) -> String {
    match w {
        WaitingFor::Priority { player } => format!("Priority({})", player.0),
        WaitingFor::LoopShortcut { proposer, .. } => format!("LoopShortcut({})", proposer.0),
        other => format!("{other:?}")
            .split_whitespace()
            .next()
            .unwrap_or("?")
            .trim_end_matches('{')
            .to_string(),
    }
}

/// The TYPED refusal of the bounded-offer mint at this exact frame — the discriminating
/// instrument: it names WHICH conjunct refused instead of collapsing to "no offer". That is what
/// makes a per-beat census non-vacuous; a bare "did not fire" could not distinguish this defect
/// from a board that simply never certified.
fn mint_verdict(state: &GameState) -> String {
    match engine::game::engine::try_offer_bounded_cycle_shortcut(state, false) {
        Ok(_) => "OFFER".to_string(),
        Err(e) => format!("{e:?}"),
    }
}

fn load_dina_raw() -> Option<(GameState, serde_json::Value)> {
    let path = std::env::var("DINA_DUMP").ok()?;
    let json = std::fs::read_to_string(&path).expect("dina dump readable");
    let envelope: serde_json::Value = serde_json::from_str(&json).expect("dina dump parses");
    let mut state = serde_json::from_value::<PersistedGameState>(envelope["gameState"].clone())
        .expect("dina gameState decodes through the production decoder")
        .into_game_state();
    state.loop_detection = engine::types::game_state::LoopDetectionMode::Interactive;
    let raw_seq = envelope["gameState"]["last_loop_action_sequence"].clone();
    Some((state, raw_seq))
}

/// Generic mandatory-chain beat: pass at `Priority`, otherwise take the first legal action.
/// The Dina chain opens no player choices, so no preference ordering is needed.
fn dina_drive_one_beat(state: &mut GameState) -> Result<String, String> {
    let who = state
        .waiting_for
        .acting_player()
        .or_else(|| state.waiting_for.acting_players().first().copied())
        .ok_or_else(|| format!("no acting player at {:?}", state.waiting_for))?;
    let (actions, _costs, _grouped) = engine::ai_support::legal_actions_for_viewer(state, who);
    let chosen = if matches!(state.waiting_for, WaitingFor::Priority { .. }) {
        actions
            .iter()
            .find(|a| matches!(a, GameAction::PassPriority))
            .cloned()
    } else {
        actions
            .iter()
            .find(|a| !matches!(a, GameAction::PassPriority))
            .or_else(|| actions.first())
            .cloned()
    };
    let action = chosen.ok_or_else(|| {
        format!(
            "no action at {:?}; legal = {:?}",
            state.waiting_for,
            actions.iter().take(8).collect::<Vec<_>>()
        )
    })?;
    let label = format!("{action:?}");
    apply(state, who, action.clone())
        .map(|_| label)
        .map_err(|e| format!("apply err ({action:?}): {e:?}"))
}

/// Drives IN PLACE (`&mut`), so the caller can inspect the board the drive stopped on — the
/// offer's proposer, the ring depth, the surviving driving sequence — instead of only the beat.
fn dina_drive_and_report(state: &mut GameState, label: &str, beats: u32) -> Option<u32> {
    eprintln!(
        "[{label}] START turn={} active={} wf={} stack={} ring={} seq={} life={:?}",
        state.turn_number,
        state.active_player.0,
        wf_label(&state.waiting_for),
        state.stack.len(),
        state.loop_detect_ring.len(),
        state.last_loop_action_sequence.len(),
        state.players.iter().map(|p| p.life).collect::<Vec<_>>(),
    );
    let mut fired = None;
    for beat in 0..beats {
        if matches!(
            state.waiting_for,
            WaitingFor::LoopShortcut { .. } | WaitingFor::GameOver { .. }
        ) {
            fired = Some(beat);
            break;
        }
        let at_priority = matches!(state.waiting_for, WaitingFor::Priority { .. });
        let before = wf_label(&state.waiting_for);
        match dina_drive_one_beat(state) {
            Ok(act) => {
                let after_priority = matches!(state.waiting_for, WaitingFor::Priority { .. });
                eprintln!(
                    "[{label}] beat {beat:3} {before:>26} -> {:<26} stack={} ring={} seq={} life={:?} mint={} act={}",
                    wf_label(&state.waiting_for),
                    state.stack.len(),
                    state.loop_detect_ring.len(),
                    state.last_loop_action_sequence.len(),
                    state.players.iter().map(|p| p.life).collect::<Vec<_>>(),
                    if after_priority || at_priority {
                        mint_verdict(state)
                    } else {
                        "-".to_string()
                    },
                    &act.chars().take(40).collect::<String>()
                );
            }
            Err(e) => {
                eprintln!("[{label}] beat {beat:3} ABORT at {before}: {e}");
                break;
            }
        }
    }
    eprintln!(
        "[{label}] END fired={fired:?} wf={} ring={} seq={} life={:?}",
        wf_label(&state.waiting_for),
        state.loop_detect_ring.len(),
        state.last_loop_action_sequence.len(),
        state.players.iter().map(|p| p.life).collect::<Vec<_>>(),
    );
    fired
}

/// (beat the offer fired at, ring depth, life vector) — the axes the two arms are compared on.
fn offer_signature(state: &GameState, fired: Option<u32>) -> (Option<u32>, usize, Vec<i32>) {
    (
        fired,
        state.loop_detect_ring.len(),
        state.players.iter().map(|p| p.life).collect(),
    )
}

/// ARM D1 — the dump as PRODUCTION loads it: `migrate_transient_loop_sequence` has already
/// dropped the driving sequence, so this is the state every in-process test would see. This is
/// the CONTROL: the offer this board raises with the field cleared is the one ARM D2 must match.
#[test]
fn probe_dina_as_loaded() {
    let Some((state, raw_seq)) = load_dina_raw() else {
        eprintln!("DINA_DUMP unset — probe skipped");
        return;
    };
    eprintln!("[DINA-LOADED] raw serialized sequence = {raw_seq}");
    assert!(
        state.last_loop_action_sequence.is_empty(),
        "reach-guard: the production restore hook must have DROPPED the sequence at load"
    );
    assert!(
        state.may_trigger_auto_choices.is_empty(),
        "reach-guard: the Dina dump carries no may-trigger auto choice (the F4 mechanism \
         cannot apply here)"
    );
    let mut state = state;
    let fired = dina_drive_and_report(&mut state, "DINA-LOADED", 140);
    eprintln!("[DINA-LOADED] fired={fired:?}");
    assert!(
        fired.is_some(),
        "REACH-GUARD: the field-cleared control must reach the offer, else ARM D2 has nothing \
         to be identical TO and the pair proves nothing"
    );
}

/// ARM D2 — the same board with the LIVE sequence put back, i.e. what the running game actually
/// held. Only that one field differs from ARM D1.
///
/// **THE FIX BAR, on the user's own capture.** The recorded step is
/// `Activate { source_id: 268 }` controlled by PlayerId(2) — an OPPONENT'S activation, unrelated
/// to the drain. Before the seat-relative (1b) this board answered `Priority(2)` for all 140
/// beats with a step-(1b) refusal census; after it, the drive must reach the SAME offer
/// the field-cleared control reaches, at the same beat, with the same ring depth and the same
/// life vector — while the foreign step is still sitting in state (`seq` stays at 1). That is
/// "a foreign period is inert", measured end-to-end rather than argued.
///
/// The comparison is against ARM D1 re-driven HERE rather than against transcribed numbers: a
/// hardcoded beat/life tuple would decay into a fixture the next drive-policy change reds for a
/// reason that has nothing to do with this defect.
#[test]
fn probe_dina_with_live_sequence() {
    let Some((mut state, raw_seq)) = load_dina_raw() else {
        eprintln!("DINA_DUMP unset — probe skipped");
        return;
    };
    state.last_loop_action_sequence =
        serde_json::from_value(raw_seq.clone()).expect("the dump's own sequence re-parses");
    assert_eq!(
        state.last_loop_action_sequence.len(),
        1,
        "reach-guard: the re-injected live sequence must be the dump's own single step"
    );
    let foreign = state.last_loop_action_sequence[0].controller;
    eprintln!("[DINA-LIVE] re-injected {raw_seq}");

    let fired = dina_drive_and_report(&mut state, "DINA-LIVE", 140);
    eprintln!("[DINA-LIVE] fired={fired:?}");

    // The CONTROL, re-driven in this process: same board, the one field cleared.
    let (mut control, _) = load_dina_raw().expect("the dump is still readable");
    control.last_loop_action_sequence.clear();
    let control_fired = dina_drive_and_report(&mut control, "DINA-CONTROL", 140);

    // IN-ROW REACH-GUARD, not inherited from ARM D1's: the equality below compares this arm to a
    // control re-driven HERE, so on a board where NEITHER side reaches an offer both sides are
    // `(None, (None, ring, life))` and the assertion passes having measured nothing. ARM D1's own
    // guard cannot cover that — a `#[test]` that is skipped, filtered, or reds independently
    // leaves this row still "green". The control must PROVABLY reach the offer in this process.
    assert!(
        control_fired.is_some(),
        "REACH-GUARD: the field-cleared control re-driven in THIS row must reach the offer, else \
         the fix bar below compares two absences and passes vacuously"
    );

    let (proposer, control_proposer) = (offer_proposer(&state), offer_proposer(&control));
    assert_ne!(
        Some(foreign),
        control_proposer,
        "REACH-GUARD: the recorded step must belong to a seat OTHER than the proposer, else \
         this arm is measuring the legitimate own-period case"
    );
    assert_eq!(
        (proposer, offer_signature(&state, fired)),
        (control_proposer, offer_signature(&control, control_fired)),
        "CR 732.2a THE FIX BAR: with an opponent's recorded activation sitting in state, the \
         proposer's own bounded offer must be reached at the same beat, with the same ring \
         depth and the same life vector, as the field-cleared control. A `None` on the left is \
         the original defect: one foreign step suppressing the offer for the whole drive"
    );
    assert_eq!(
        state.last_loop_action_sequence.len(),
        1,
        "and the foreign step is still THERE — the offer was reached with it in state, not by \
         the drive quietly clearing it"
    );
}

fn offer_proposer(state: &GameState) -> Option<engine::types::player::PlayerId> {
    match &state.waiting_for {
        WaitingFor::LoopShortcut { proposer, .. } => Some(*proposer),
        _ => None,
    }
}
