//! CR 603.3 + CR 732.2a — what a recorded trigger-driven loop step carries from the cycle that
//! performed it.
//!
//! Two boards, because their prompt sets are disjoint apart from the ordering prompt: Board A
//! (Abdel Adrian + Animate Dead + Altar of the Brood, built by
//! `abdel_adrian_animate_dead_altar_board` and driven here, never rebuilt) raises `OrderTriggers`
//! and `EffectZoneChoice`; Board B (Preston, the Vanisher + Felidar Guardian + Animate Dead +
//! Altar, built by `loop_period_trigger_driven_arming` and driven here) raises `OrderTriggers`,
//! `TriggerTargetSelection` and `OptionalEffectChoice`.
//!
//! Every board is driven only through `game::engine::apply()`.
//!
//! **Each board's declaration of the step at which the drive declines the loop's voluntary
//! choice.** Board A's cycle is voluntary at Abdel Adrian's exile ("any number", and zero is a
//! number): the row answers that choice in full while accepts remain and with an empty selection
//! after, and the step whose record carries the empty selection is the one whose replay reaches no
//! recurrence. Board B's cycle is voluntary at Felidar Guardian's "you may exile": the row accepts
//! while accepts remain and declines at the first choice of that kind after them, and the step
//! whose record carries the decline is that board's.

use engine::analysis::decision_template::ChoicePoint;
use engine::game::scenario::{GameRunner, P0};
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::game_state::{
    GameState, LoopAction, LoopActionContext, StackEntryKind, WaitingFor,
};
use engine::types::identifiers::ObjectId;
use engine::types::player::PlayerId;

use crate::loop_period_trigger_driven_arming::{build_board_b, PrestonBoard};

/// Beat cap for every live drive here. Read by no assertion; it bounds a runaway drive.
const BEAT_CAP: usize = 320;

/// How many of the board's voluntary choices each row accepts before it declines. A recorded step
/// needs TWO performed resolutions of its trigger: one whose interval writes the record, and the
/// next, whose arming completes it.
const ACCEPTS: usize = 6;

/// What one performed cycle of a board recorded.
struct Performed {
    /// The recorded step, read at the settle frame by the index it occupied at the entry frame,
    /// with the pins the performed cycle wrote onto it.
    step: LoopActionContext,
}

/// Stack entries of one source's triggered ability, by entry id. Each board's minting source
/// carries exactly one functioning trigger entry (asserted by the arming module's
/// `expected_step`), so the source alone names the step's trigger without ambiguity.
fn trigger_entry_ids(state: &GameState, source: ObjectId) -> Vec<ObjectId> {
    state
        .stack
        .iter()
        .filter(|entry| {
            matches!(&entry.kind, StackEntryKind::TriggeredAbility { source_id, .. } if *source_id == source)
        })
        .map(|entry| entry.id)
        .collect()
}

/// The member at `index` of the period's record, when it is a resolving trigger of `source`
/// recorded under `seat`. The identity re-check: `accumulate_loop_action_step` clears the whole
/// vector and re-pushes on a controller change or at 16 steps, so an index can survive into a
/// period that is not this step's. The seat is part of the check because the clear reads the
/// sequence's FIRST step and so fires at length 1 — the re-push lands at index 0 again, and a
/// same-source step there differs from the captured one only in its controller. `seat` is `None`
/// at the beat the capture OPENS, which is the beat that reads the seat rather than re-checking it.
fn recorded_trigger_step_at(
    state: &GameState,
    index: usize,
    source: ObjectId,
    seat: Option<PlayerId>,
) -> Option<&LoopActionContext> {
    let step = state.last_loop_action_sequence.get(index)?;
    match &step.action {
        LoopAction::ResolveTrigger { source_id, .. }
            if *source_id == source && seat.is_none_or(|seat| step.controller == seat) =>
        {
            Some(step)
        }
        _ => None,
    }
}

/// Drive one board through `apply()` under its own declared answering policy, capturing the two
/// beats the step's own answer interval has: the frame the drive is handed, and the frame at which
/// that step's record is COMPLETE.
///
/// `answer` is the board's policy: it sees the live state and returns the action this row's
/// declaration says to take, or `None` to fall through to the first non-pass legal action.
fn perform(
    runner: &mut GameRunner,
    minting_source: ObjectId,
    mut answer: impl FnMut(&GameState) -> Option<GameAction>,
) -> Performed {
    let mut entry: Option<(GameState, usize, PlayerId)> = None;
    for _ in 0..BEAT_CAP {
        let state = runner.state();
        match &entry {
            None => {
                // Beat (a), the frame the drive is handed: `arm_trigger_driven_loop_period` has
                // just appended the step, so it is the sequence's last member and its `pins` are
                // empty. Its own trigger has already left the stack — that is production's own
                // opener precondition, asserted rather than searched for, so a co-resolving
                // sibling fails this row loudly instead of being walked past.
                let index = state.last_loop_action_sequence.len().saturating_sub(1);
                if let Some(seat) = recorded_trigger_step_at(state, index, minting_source, None)
                    .filter(|step| step.pins.is_empty())
                    .map(|step| step.controller)
                {
                    assert!(
                        trigger_entry_ids(state, minting_source).is_empty(),
                        "entry frame: no instance of the step's own trigger may stand on the \
                         stack at the beat the drive is handed"
                    );
                    entry = Some((state.clone(), index, seat));
                }
            }
            Some((entry_frame, index, seat)) => {
                if recorded_trigger_step_at(state, *index, minting_source, Some(*seat)).is_none() {
                    // The vector was cleared and the index now holds another period's step: drop
                    // the capture and re-open a fresh entry frame. Never read the neighbour.
                    entry = None;
                } else if state.last_loop_action_sequence.len() > *index + 1 {
                    // Beat (b): the vector has grown PAST the step, so nothing can attach to it
                    // any more (`record_trigger_step_pin` attaches to `last_mut`) and its record
                    // is complete.
                    assert!(
                        state
                            .battlefield
                            .iter()
                            .any(|id| !entry_frame.battlefield.contains(id)),
                        "reach guard: the performed cycle must MINT an object between the entry \
                         and settle frames, else the frames this row reads are two frames nothing \
                         changed"
                    );
                    return Performed {
                        step: state.last_loop_action_sequence[*index].clone(),
                    };
                }
            }
        }

        let action = match answer(state) {
            Some(action) => action,
            None => engine::ai_support::legal_actions(state)
                .into_iter()
                .find(|action| !matches!(action, GameAction::PassPriority))
                .unwrap_or(GameAction::PassPriority),
        };
        if runner.act(action).is_err() {
            break;
        }
    }
    panic!(
        "the drive never completed a recorded trigger-driven step's answer interval: entry frame \
         {} captured. A row that returned here would pass vacuously.",
        if entry.is_some() { "was" } else { "was NOT" }
    );
}

// ---------------------------------------------------------------------------
// Board A
// ---------------------------------------------------------------------------

fn cast_animate_dead_on(runner: &mut GameRunner, animate_dead: ObjectId, target: ObjectId) {
    let card_id = runner.state().objects[&animate_dead].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: animate_dead,
            card_id,
            targets: vec![target],
            payment_mode: engine::types::game_state::CastPaymentMode::Auto,
        })
        .expect("Animate Dead is castable with the seeded mana");
}

/// Board A's policy: answer Abdel Adrian's voluntary exile in full while accepts remain, with an
/// empty selection after. `accepts` is a cell so the closure can spend them.
fn board_a_policy(accepts: &mut usize) -> impl FnMut(&GameState) -> Option<GameAction> + '_ {
    move |state: &GameState| match &state.waiting_for {
        WaitingFor::EffectZoneChoice { cards, .. } => {
            let answer = if *accepts > 0 {
                *accepts -= 1;
                cards.clone()
            } else {
                vec![]
            };
            Some(GameAction::SelectCards { cards: answer })
        }
        _ => None,
    }
}

fn board_a_performed(accepts: usize) -> Option<Performed> {
    let mut board = crate::abdel_adrian_animate_dead_altar_board::build()?;
    // The arming beat carries the sampler's gate, so a board built with detection OFF records
    // nothing at all. Board B's builder sets this itself; Board A's is shared with rows that do
    // not need it, so this row sets it here rather than changing that board.
    board.runner.state_mut().loop_detection =
        engine::types::game_state::LoopDetectionMode::Interactive;
    cast_animate_dead_on(&mut board.runner, board.animate_dead, board.abdel);
    let abdel = board.abdel;
    let mut left = accepts;
    let mut policy = board_a_policy(&mut left);
    Some(perform(&mut board.runner, abdel, |state| policy(state)))
}

// ---------------------------------------------------------------------------
// Board B
// ---------------------------------------------------------------------------

/// Board B's policy: aim each enters trigger at Felidar Guardian and accept its exile while
/// accepts remain, and decline after. `aim_otherwise` is where a trigger that cannot target Felidar
/// Guardian is aimed; `None` takes the first legal action.
fn board_b_policy(
    felidar: ObjectId,
    aim_otherwise: Option<ObjectId>,
    mut left: usize,
) -> impl FnMut(&GameState) -> Option<GameAction> {
    move |state: &GameState| {
        let legal = engine::ai_support::legal_actions(state);
        let aimed_at = |target: ObjectId| {
            legal.iter().find(|action| {
                matches!(
                    action,
                    GameAction::ChooseTarget { target: Some(TargetRef::Object(id)) } if *id == target
                )
            })
        };
        if left > 0 {
            if let Some(action) = aimed_at(felidar) {
                return Some(action.clone());
            }
        }
        if let Some(action) = aim_otherwise.and_then(aimed_at) {
            return Some(action.clone());
        }
        let accept = left > 0;
        if let Some(action) = legal.iter().find(|action| {
            matches!(
                action,
                GameAction::DecideOptionalEffect { accept: answered } if *answered == accept
            )
        }) {
            if accept {
                left -= 1;
            }
            return Some(action.clone());
        }
        None
    }
}

/// Board B built, with Animate Dead cast onto Felidar Guardian.
fn board_b_cast() -> Option<PrestonBoard> {
    let mut board = build_board_b()?;
    cast_animate_dead_on(&mut board.runner, board.animate_dead, board.felidar);
    Some(board)
}

fn board_b_performed(accepts: usize) -> Option<Performed> {
    let mut board = board_b_cast()?;
    Some(perform(
        &mut board.runner,
        board.preston,
        board_b_policy(board.felidar, None, accepts),
    ))
}

/// CR 732.2a — **the two boards' recorded classes are disjoint apart from the ordering prompt**,
/// which is why neither board alone satisfies R1. Asserted on the records themselves rather than
/// inherited from the measurement that chose the boards.
#[test]
fn the_two_boards_record_disjoint_classes_apart_from_the_ordering_prompt() {
    let (Some(a), Some(b)) = (board_a_performed(ACCEPTS), board_b_performed(ACCEPTS)) else {
        return;
    };
    let points = |step: &LoopActionContext| {
        let mut p: Vec<ChoicePoint> = step.pins.iter().map(|pin| pin.slot().point).collect();
        p.sort_by_key(|point| format!("{point:?}"));
        p.dedup();
        p
    };
    let (pa, pb) = (points(&a.step), points(&b.step));
    assert!(
        !pa.is_empty() && !pb.is_empty(),
        "positive control: both boards must record at least one class, got {pa:?} / {pb:?}"
    );
    let shared: Vec<ChoicePoint> = pa
        .iter()
        .filter(|point| pb.contains(point))
        .copied()
        .collect();
    assert!(
        shared
            .iter()
            .all(|point| *point == ChoicePoint::TriggerOrder),
        "the boards' recorded classes are disjoint apart from the ordering prompt; A {pa:?}, \
         B {pb:?}, shared {shared:?}"
    );
}

/// The seat every board here drives for. Named so a reader does not have to infer it from the
/// builders.
#[allow(dead_code)]
const DRIVER: PlayerId = P0;
