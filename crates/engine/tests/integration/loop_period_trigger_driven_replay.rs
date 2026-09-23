//! CR 603.3 + CR 732.2a — a recorded trigger-driven loop step REPLAYS from what the step
//! recorded, and the replay settles at the frame the performed cycle itself reaches.
//!
//! Two boards, because their prompt sets are disjoint apart from the ordering prompt: Board A
//! (Abdel Adrian + Animate Dead + Altar of the Brood, built by
//! `abdel_adrian_animate_dead_altar_board` and driven here, never rebuilt) raises `OrderTriggers`
//! and `EffectZoneChoice`; Board B (Preston, the Vanisher + Felidar Guardian + Animate Dead +
//! Altar, built by `loop_period_trigger_driven_arming` and driven here) raises `OrderTriggers`,
//! `TriggerTargetSelection` and `OptionalEffectChoice`.
//!
//! Every board is driven only through `game::engine::apply()`. The replay is the production
//! drive, reached through the `test-support`-gated hand-off beside it. The replay rows read each
//! board at the windows where P0 holds priority with the minting trigger on top, the frame the
//! recorded-period producer enters, and compare the replay with the next such window.
//!
//! **Each board's declaration of the step at which the drive declines the loop's voluntary
//! choice.** Board A's cycle is voluntary at Abdel Adrian's exile ("any number", and zero is a
//! number): the row answers that choice in full while accepts remain and with an empty selection
//! after, and the step whose record carries the empty selection is the one whose replay reaches no
//! recurrence. Board B's cycle is voluntary at Felidar Guardian's "you may exile": the row accepts
//! while accepts remain and declines at the first choice of that kind after them, and the step
//! whose record carries the decline is that board's.

use engine::analysis::decision_template::{ChoicePoint, PinnedDecision, TargetPin};
use engine::game::engine::{drive_loop_sequence_iteration_for_tests, RecastAbort};
use engine::game::scenario::{GameRunner, P0, P1};
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::game_state::{
    GameState, LoopAction, LoopActionContext, StackEntryKind, WaitingFor, YieldTarget,
};
use engine::types::identifiers::ObjectId;
use engine::types::player::PlayerId;

use crate::loop_period_accessor_answers::altar_resolves_first;
use crate::loop_period_trigger_driven_arming::{build_board_b, PrestonBoard};

/// Beat cap for every live drive here. Read by no assertion; it bounds a runaway drive.
const BEAT_CAP: usize = 320;

/// How many of the board's voluntary choices each row accepts before it declines. A replayed step
/// needs TWO performed resolutions of its trigger: one whose interval writes the record, and the
/// next, whose arming completes it.
const ACCEPTS: usize = 6;

/// What one performed cycle of a board handed the replay.
struct Performed {
    /// Beat (a): the frame the drive was handed, before any of the step's own prompts were
    /// answered.
    entry: GameState,
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
                        entry: entry_frame.clone(),
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

/// **A harness self-check, not a production seam.** Every assertion runs
/// `recorded_trigger_step_at`, the predicate this file defines, so no production line reverts
/// to break the row; what it pins is the discriminating power of the index+source+seat key the
/// replay rows below read a recorded sequence through. Both polarities run: the ordinary
/// same-seat successor is admitted at the captured index, and a same-source re-push under
/// another seat is refused. The fixture is the board's own recorded step with its controller —
/// and nothing else — changed, so the refusal can only be the seat conjunct's.
#[test]
fn recorded_trigger_step_at_separates_a_re_push_that_differs_only_in_its_seat() {
    let Some(performed) = board_b_performed(ACCEPTS) else {
        return;
    };
    let LoopAction::ResolveTrigger { source_id, .. } = performed.step.action else {
        panic!("positive control: the captured step is trigger-driven");
    };
    let seat = performed.step.controller;
    let mut state = performed.entry;
    state.last_loop_action_sequence = vec![performed.step.clone()];
    assert!(
        recorded_trigger_step_at(&state, 0, source_id, Some(seat)).is_some(),
        "positive control: the ordinary same-seat successor is still admitted at the index"
    );
    let mut re_pushed = performed.step;
    re_pushed.controller = if seat == P0 { P1 } else { P0 };
    assert_ne!(
        re_pushed.controller, seat,
        "positive control: the fixture really changes the seat"
    );
    state.last_loop_action_sequence = vec![re_pushed];
    assert!(
        recorded_trigger_step_at(&state, 0, source_id, Some(seat)).is_none(),
        "a same-source re-push at the captured index under another seat is not this step"
    );
}

// ---------------------------------------------------------------------------
// The replay
// ---------------------------------------------------------------------------

/// How many of Abdel Adrian's exiles Board A's window drive accepts; the rows read windows at
/// which accepts remain, so the live cycle answers as the record does.
const BOARD_A_WINDOW_ACCEPTS: usize = 8;

/// How many of Felidar Guardian's exiles Board B's window drive accepts, for the same reason.
const BOARD_B_WINDOW_ACCEPTS: usize = 12;

/// How many windows each board's replay rows read.
const WINDOWS: usize = 2;

/// The frame the recorded-period producer enters with the minting trigger on top and its own step
/// last in the record, that step, and the next window with a new instance of the trigger on top.
struct Window {
    entry: GameState,
    step: LoopActionContext,
    next: GameState,
}

fn top_is_trigger_of(state: &GameState, source: ObjectId) -> bool {
    state
        .stack
        .back()
        .is_some_and(|top| trigger_entry_ids(state, source).contains(&top.id))
}

/// Drive to the first `count` windows, passing at every priority and declining every offer.
///
/// A window's entry is the `Priority{P0}` frame the recorded-period producer reads. Where it mints
/// an offer, the mint writes only `waiting_for` over that frame, so the entry is the offer with P0's
/// priority restored.
fn recurrence_windows(
    runner: &mut GameRunner,
    minting: ObjectId,
    count: usize,
    mut answer: impl FnMut(&GameState) -> Option<GameAction>,
) -> Vec<Window> {
    let mut windows = Vec::new();
    let mut open: Option<(GameState, LoopActionContext)> = None;
    for _ in 0..BEAT_CAP {
        let state = runner.state();
        let offer = matches!(state.waiting_for, WaitingFor::LoopShortcut { .. });
        let p0_priority =
            matches!(state.waiting_for, WaitingFor::Priority { player } if player == P0);
        if let Some((entry, step)) = &open {
            if (p0_priority || offer)
                && top_is_trigger_of(state, minting)
                && state.stack.back().map(|top| top.id) != entry.stack.back().map(|top| top.id)
            {
                windows.push(Window {
                    entry: entry.clone(),
                    step: step.clone(),
                    next: state.clone(),
                });
                open = None;
                if windows.len() == count {
                    return windows;
                }
            }
        }
        let p0_offer = matches!(
            state.waiting_for,
            WaitingFor::LoopShortcut { proposer, .. } if proposer == P0
        );
        if open.is_none() && (p0_priority || p0_offer) && top_is_trigger_of(state, minting) {
            if let Some(step) = state.last_loop_action_sequence.last().filter(|step| {
                matches!(
                    step.action,
                    LoopAction::ResolveTrigger { source_id, .. } if source_id == minting
                )
            }) {
                let mut entry = state.clone();
                entry.waiting_for = WaitingFor::Priority { player: P0 };
                open = Some((entry, step.clone()));
            }
        }
        let action = if offer {
            GameAction::DeclineShortcut
        } else if matches!(state.waiting_for, WaitingFor::Priority { .. }) {
            GameAction::PassPriority
        } else {
            answer(state).unwrap_or_else(|| {
                engine::ai_support::legal_actions(state)
                    .into_iter()
                    .find(|action| !matches!(action, GameAction::PassPriority))
                    .expect("a prompt the declared drive does not answer offers a legal action")
            })
        };
        runner
            .act(action.clone())
            .unwrap_or_else(|error| panic!("{action:?} was rejected: {error:?}"));
    }
    panic!(
        "the drive reached {} of {count} windows in {BEAT_CAP} beats",
        windows.len()
    );
}

/// Board A's windows: Altar of the Brood's triggers resolve first (CR 603.3b), and Abdel Adrian's
/// exile takes only Animate Dead while accepts remain.
fn board_a_windows() -> Option<Vec<Window>> {
    let mut board = crate::abdel_adrian_animate_dead_altar_board::build()?;
    board.runner.state_mut().loop_detection =
        engine::types::game_state::LoopDetectionMode::Interactive;
    let returned = board.runner.state().objects[&board.animate_dead].card_id;
    cast_animate_dead_on(&mut board.runner, board.animate_dead, board.abdel);
    let mut left = BOARD_A_WINDOW_ACCEPTS;
    Some(recurrence_windows(
        &mut board.runner,
        board.abdel,
        WINDOWS,
        move |state| {
            altar_resolves_first(state).or_else(|| {
                let WaitingFor::EffectZoneChoice { cards, .. } = &state.waiting_for else {
                    return None;
                };
                let exiled = if left > 0 {
                    left -= 1;
                    cards
                        .iter()
                        .copied()
                        .filter(|id| {
                            state
                                .objects
                                .get(id)
                                .is_some_and(|object| object.card_id == returned)
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                Some(GameAction::SelectCards { cards: exiled })
            })
        },
    ))
}

/// Board B's windows: a trigger that cannot target Felidar Guardian is aimed at Altar of the Brood,
/// so every occurrence answers as the record does.
fn board_b_windows() -> Option<Vec<Window>> {
    let mut board = board_b_cast()?;
    Some(recurrence_windows(
        &mut board.runner,
        board.preston,
        WINDOWS,
        board_b_policy(board.felidar, Some(board.altar), BOARD_B_WINDOW_ACCEPTS),
    ))
}

/// Both boards' windows, or `None` when either board's cards are absent.
fn both_boards() -> Option<[(&'static str, Vec<Window>); 2]> {
    Some([
        ("Board A", board_a_windows()?),
        ("Board B", board_b_windows()?),
    ])
}

/// Replay one recorded step from `entry`, through the production drive. `[None]` is the
/// definition slice: a `ResolveTrigger` opener validates the frame and reads no definition.
fn replay(entry: &GameState, step: &LoopActionContext) -> (GameState, Result<(), RecastAbort>) {
    let mut clone = entry.clone();
    let verdict =
        drive_loop_sequence_iteration_for_tests(&mut clone, std::slice::from_ref(step), 0, &[None]);
    (clone, verdict)
}

/// Re-stamp every by-identity target of the step's FIRST `Targets` pin to an object id no object
/// carries, and return how many were re-stamped.
fn restamp_first_targets_pin_id(step: &mut LoopActionContext) -> usize {
    for pin in step.pins.iter_mut() {
        let PinnedDecision::Targets { targets, .. } = pin else {
            continue;
        };
        let restamped = targets
            .iter_mut()
            .filter_map(|target| match target {
                TargetPin::ByIdentity(YieldTarget::ThisObject { source_id, .. }) => Some(source_id),
                _ => None,
            })
            .map(|source_id| *source_id = ObjectId(u64::MAX - 1))
            .count();
        if restamped > 0 {
            return restamped;
        }
    }
    0
}

/// CR 405.5 + CR 117.3b + CR 603.3 + CR 732.2a — **charter acceptance row 1.** A recorded
/// trigger-driven step replays from the window its producer enters, and settles at the next window
/// the performed cycle reaches.
///
/// The comparison is this row's own subject — every zone, each player's state, and the stack —
/// never whole-`GameState` equality: the drive runs the reconcile-free internal path by design, so
/// `derive_display_state`'s mirror fields and `apply()`'s per-action counter differ between the
/// frames for reasons that are not this row's.
#[test]
fn a_recorded_trigger_driven_step_replays_to_the_frame_performance_reaches() {
    let Some(boards) = both_boards() else {
        return;
    };
    for (board, windows) in boards {
        for (index, window) in windows.iter().enumerate() {
            assert!(
                window
                    .next
                    .battlefield
                    .iter()
                    .any(|id| !window.entry.battlefield.contains(id)),
                "{board} window {index}: reach guard — the performed cycle minted an object"
            );
            let (settled, verdict) = replay(&window.entry, &window.step);
            assert!(
                verdict.is_ok(),
                "{board} window {index}: the recorded step replays; got {verdict:?}"
            );
            assert_eq!(
                settled.battlefield, window.next.battlefield,
                "{board} window {index}: the battlefield the replay settles at"
            );
            assert_eq!(
                settled.exile, window.next.exile,
                "{board} window {index}: the exile zone the replay settles at"
            );
            assert_eq!(
                settled.command_zone, window.next.command_zone,
                "{board} window {index}: the command zone the replay settles at"
            );
            assert_eq!(
                settled.stack, window.next.stack,
                "{board} window {index}: the stack the replay settles at"
            );
            assert_eq!(
                settled.players, window.next.players,
                "{board} window {index}: each player's state at the frame the replay settles at"
            );
        }
    }
}

/// CR 608.2b — **a pin the drive already took is not re-asserted at a foreign beat.** CR 608.2b's
/// subject is the resolving object: "If the spell or ability specifies targets, **it** checks
/// whether the targets are still legal." So a recorded target's legality belongs to the beat that
/// answers its pin, and a later beat of the same interval is not that pin's moment.
///
/// Reach guard, as a property rather than a beat count: the step's record is non-empty and the
/// drive returns `Ok`, and a `ResolveTrigger` step's settle refuses unless every recorded entry was
/// settled — so `Ok` IS "every prompt the record names was answered".
///
/// The re-check's own end, in the same invocation: re-stamping the first `Targets` pin's target to
/// an object id no object carries aborts the drive at the beat that takes it.
#[test]
fn a_pin_the_drive_already_took_is_not_re_checked_at_a_foreign_beat() {
    let Some(boards) = both_boards() else {
        return;
    };
    for (board, windows) in boards {
        for (index, window) in windows.iter().enumerate() {
            assert!(
                !window.step.pins.is_empty(),
                "{board} window {index}: positive control — the record carries answers"
            );
            let (_, verdict) = replay(&window.entry, &window.step);
            assert!(
                verdict.is_ok(),
                "{board} window {index}: every prompt the record names is answered; got \
                 {verdict:?}"
            );

            let mut missing = window.step.clone();
            assert_eq!(
                restamp_first_targets_pin_id(&mut missing),
                1,
                "{board} window {index}: the fixture changes only the one pinned target"
            );
            let (_, missing_verdict) = replay(&window.entry, &missing);
            assert!(
                missing_verdict.is_err(),
                "{board} window {index}: a pinned target no object carries aborts at the beat \
                 that takes it; got {missing_verdict:?}"
            );
        }
    }
}

/// Re-stamp every `Order` pin's SLOT SOURCE to an incarnation no live object carries, and return
/// how many were re-stamped.
fn restamp_order_slot_sources(step: &mut LoopActionContext) -> usize {
    step.pins
        .iter_mut()
        .filter_map(|pin| match pin {
            PinnedDecision::Order { slot, .. } => match &mut slot.source {
                YieldTarget::ThisObject { incarnation, .. } => Some(incarnation),
                YieldTarget::AllCopies { .. } => None,
            },
            _ => None,
        })
        .map(|incarnation| *incarnation = Some(incarnation.unwrap_or(0) + 1_000))
        .count()
}

/// CR 732.2a — **an entry the interval never asks for is refused at the settle.** The prompt
/// occurrences of one drive and the step's recorded entries are in bijection: an untaken entry
/// means this drive's interval is not the one the recording wrote.
///
/// The fixture appends a byte-copy of the record's last entry that is not an ordering entry, since
/// an ordering entry no prompt asks for is spent (CR 603.3b). Selection walks the pins in index
/// order, so the copy sits behind every original of its class and is left untaken. Paired control
/// in the same invocation: the unmodified record replays.
#[test]
fn an_entry_the_interval_never_asks_for_is_refused_at_the_settle() {
    let Some(boards) = both_boards() else {
        return;
    };
    for (board, windows) in boards {
        for (index, window) in windows.iter().enumerate() {
            let (_, control) = replay(&window.entry, &window.step);
            assert!(
                control.is_ok(),
                "{board} window {index}: control — the unmodified record replays; got {control:?}"
            );

            let mut spare = window.step.clone();
            let duplicate = spare
                .pins
                .iter()
                .rev()
                .find(|pin| !matches!(pin, PinnedDecision::Order { .. }))
                .expect("positive control: the record carries a non-ordering entry to copy")
                .clone();
            let point = duplicate.slot().point;
            spare.pins.push(duplicate);
            let (_, verdict) = replay(&window.entry, &spare);
            assert!(
                verdict.is_err(),
                "{board} window {index}: a spare {point:?} entry no occurrence can take is \
                 refused at the settle; got {verdict:?}"
            );
        }
    }
}

/// CR 400.7 + CR 603.3b — **a trigger-driven step's key does not read the ordering source.** The
/// object that asked is replaced between repetitions, so the place in the sequence is the whole of
/// the class: an ordering pin whose source names a spent incarnation still answers its prompt.
///
/// Board B's window interval meets the ordering prompt, so the same step without its `Order`
/// entries is refused there: the prompt was answered from an entry, not spent. The other end of
/// the key (the same pin refused once the source is part of the key) is
/// `a_stale_ordering_source_is_refused_only_when_the_source_is_part_of_the_key` beside
/// `take_answer`.
#[test]
fn a_stale_ordering_source_still_answers_a_trigger_driven_step() {
    let Some(windows) = board_b_windows() else {
        return;
    };
    let window = &windows[0];
    let (_, control) = replay(&window.entry, &window.step);
    assert!(
        control.is_ok(),
        "control — the unmodified record replays; got {control:?}"
    );

    let mut unordered = window.step.clone();
    unordered
        .pins
        .retain(|pin| !matches!(pin, PinnedDecision::Order { .. }));
    assert_ne!(
        unordered.pins, window.step.pins,
        "positive control — the record carries ordering entries"
    );
    let (_, unordered_verdict) = replay(&window.entry, &unordered);
    assert!(
        unordered_verdict.is_err(),
        "the ordering prompt the interval meets needs its entry; got {unordered_verdict:?}"
    );

    let mut stale = window.step.clone();
    let restamped = restamp_order_slot_sources(&mut stale);
    assert!(
        restamped > 0,
        "positive control — the record carries ordering pins to re-stamp"
    );
    let (_, verdict) = replay(&window.entry, &stale);
    assert!(
        verdict.is_ok(),
        "{restamped} ordering pins naming a spent incarnation still answer their prompts; got \
         {verdict:?}"
    );
}

/// The seat every board here drives for. Named so a reader does not have to infer it from the
/// builders.
#[allow(dead_code)]
const DRIVER: PlayerId = P0;
