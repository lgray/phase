//! CR 732.2a: the confirmer. It replays a candidate period the play trace names, twice, from the
//! priority frame it is asked at, on the proposer's own information set, and certifies the
//! replay's frames through the recurrence cover and the sign check. A period that spends
//! something of its proposer's own is not shown to repeat; it is counted by replaying it until a
//! repetition is refused.
//!
//! The trace (`play_trace`) records and names; this module decides whether a named span is a
//! period a shortcut may repeat.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::analysis::decision_template::{
    ChoicePoint, DecisionSlot, DecisionTemplate, MayChoiceOption, PinnedDecision,
    ShortcutDecisionSchema, TargetPin,
};
use crate::analysis::loop_check::{classify_win_kind, LoopCertificate, WinKind};
use crate::analysis::resource::{
    driving_resources_non_decreasing_carried, frame_without, CertifiedInstructedDeparture,
    CleanupPair, CounterClass, CoveredGrowth, ObjectClass, ObjectGrowthVerdict, PredictedDeparture,
    RecurrenceCover, ResourceVector,
};
use crate::game::engine::{
    announced_target_pins, apply, certify_object_growth_frames, clear_frame_bookkeeping,
    counted_offer, has_no_loss_axis, minted_battlefield_ids, object_decision_source,
    period_sign_check, pinnable_mana_color, pinned_decisions_to_points, proliferate_pins,
    replay_bounded_offer, PeriodSign, SimulationProbeGuard,
};
use crate::game::mana_payment::{select_convoke_taps, ConvokeTapOrder};
use crate::game::play_trace::{
    self, AnswerOptionality, CostChoices, CostMove, EntryKind, NamedSpan, PeriodReach, PlayLocus,
    PromptClass, TraceEntry,
};
use crate::types::ability::TargetRef;
use crate::types::actions::GameAction;
use crate::types::events::GameEvent;
use crate::types::game_state::{
    CostResume, GameState, ManaChoiceContext, PayCostKind, StackEntryKind, WaitingFor, YieldTarget,
};
use crate::types::identifiers::{ObjectId, ObjectIncarnationRef};
use crate::types::mana::ManaSourceSelection;
use crate::types::player::PlayerId;

/// Why the confirmer did not confirm a span, in the order its stages read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfferRefusal {
    /// CR 732.3: optional plays or answers from more than one player.
    Fragmented { seats: Vec<PlayerId> },
    /// Engine policy under CR 732.2a: a span is offered to the player whose optional plays and
    /// answers it is made of; the holder is not offered `seat`'s span.
    ForeignSeat { seat: PlayerId },
    /// CR 732.2a: a replayed action drew a random outcome.
    Randomness,
    /// A prompt the replay reached that no recorded answer answers.
    UnanswerablePrompt,
    /// CR 732.2a: the reducer rejected a replayed play.
    IllegalReplayedPlay,
    /// CR 400.7: an object a replayed cost moved arrived somewhere other than where it did.
    ArrivalDiverged,
    /// CR 732.1b: the step ended or the game ended before the period came round again.
    NoRecurrence,
    /// The replay's frames do not cover one another.
    Cover(ObjectGrowthVerdict),
    /// The period makes no progress for its controller.
    NoAxis,
    /// CR 704.5a: the period moves some player toward losing the game, and the threshold
    /// authority refused its replayed frames.
    LossAxis,
}

/// One recorded play or answer of a period, as the replay makes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PeriodItem {
    seat: PlayerId,
    action: GameAction,
    /// For a play: the stack depth and prompt class it was made at.
    play: Option<(usize, PromptClass)>,
    /// An answer the instruction asking it did not let its chooser decline.
    mandatory_answer: bool,
    /// The object-id counter when it was recorded.
    next_object_id: u64,
    /// Ids from here up to `next_object_id` were minted in the period before it was recorded.
    minted_since: u64,
    cost_move: Option<CostMove>,
}

/// CR 732.2a: the plays and answers of a confirmed period, as an offer carries them to its take.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfirmedPeriod {
    items: Vec<PeriodItem>,
    /// What the cover that confirmed the period admitted as its growth.
    #[serde(default)]
    growth: CoveredGrowth,
    /// How far past its step the period runs before it comes round again.
    #[serde(default)]
    reach: PeriodReach,
    /// CR 732.2a: the game choices the first replayed cycle's prompts were answered with, which a
    /// recorded take performs.
    #[serde(default)]
    choices: Vec<PinnedDecision>,
    /// Which authority counted the period's offer.
    #[serde(default, skip_serializing_if = "CountedBy::is_threshold")]
    counted_by: CountedBy,
    /// How one repetition is performed; `None` when no plays were read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    performance: Option<PeriodPerformance>,
}

/// CR 605.3a + CR 605.3b: whether a period's repetition is made only of mana abilities, which its
/// player may activate while paying a cost (CR 601.2g) and which resolve without the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeriodPerformance {
    /// Every play is a mana ability and nothing resolves from the stack.
    ManaAbilitiesOnly,
    /// A play is a spell or a non-mana ability, or a triggered ability resolves, which waits for
    /// priority (CR 603.3), so no cost's payment performs it.
    UsesTheStack,
}

impl PeriodPerformance {
    fn of(body: &[TraceEntry]) -> Self {
        let uses_the_stack = body.iter().any(|entry| match &entry.kind {
            EntryKind::Play { locus, .. } => match locus {
                PlayLocus::Mana(..) => false,
                PlayLocus::Cast(_) | PlayLocus::Activate(..) | PlayLocus::Unread => true,
            },
            EntryKind::Resolution { .. } => true,
            EntryKind::Answer { .. } => false,
        });
        if uses_the_stack {
            Self::UsesTheStack
        } else {
            Self::ManaAbilitiesOnly
        }
    }
}

/// CR 732.2a: which authority states the count of a confirmed period's offer; its take answers to
/// the same one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CountedBy {
    /// The threshold authority, by arithmetic over one certified period; a period that repeats
    /// without bound has no count.
    #[default]
    Threshold,
    /// The counting replay, by performing the period: the seats that left the game in each
    /// repetition it performed, the one that ended the count included.
    Replay { departures: Vec<PredictedDeparture> },
}

impl CountedBy {
    fn is_threshold(&self) -> bool {
        *self == CountedBy::Threshold
    }
}

// `GameAction` derives no `Eq`, and no value an action carries is floating-point.
impl Eq for ConfirmedPeriod {}

impl ConfirmedPeriod {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn growth(&self) -> CoveredGrowth {
        self.growth
    }

    pub fn choices(&self) -> &[PinnedDecision] {
        &self.choices
    }

    pub fn performance(&self) -> Option<PeriodPerformance> {
        self.performance
    }

    /// CR 732.2a: the answers a take of this period performs, which a declaration must agree
    /// with; `None` when no period was confirmed.
    pub fn recorded_answers(&self) -> Option<&[PinnedDecision]> {
        (!self.is_empty()).then_some(self.choices.as_slice())
    }

    /// CR 732.2a: the departures the counting replay saw, for a period it counted; `None` for a
    /// period it did not count.
    pub(crate) fn replayed_departures(&self) -> Option<&[PredictedDeparture]> {
        match &self.counted_by {
            CountedBy::Threshold => None,
            CountedBy::Replay { departures } => Some(departures),
        }
    }
}

/// A confirmed period: what it replays, the replay's last frame pair, and the sign check's
/// delta.
#[derive(Debug, Clone)]
pub(crate) struct Confirmation {
    pub(crate) period: ConfirmedPeriod,
    pub(crate) frames: Box<[GameState; 2]>,
    pub(crate) delta: ResourceVector,
    pub(crate) departure: Option<CertifiedInstructedDeparture>,
    /// The printed identity of each triggered ability the first replayed cycle resolved, in order.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) performed: Vec<String>,
    /// CR 732.2a: the offer at its stated count — a loss period's as the threshold authority
    /// measured it on the replayed frames (CR 704.5a), or a spend period's as the counting replay
    /// performed it; `None` on the unbounded road.
    pub(crate) bounded: Option<BoundedOfferParts>,
}

/// The certificate, schema and declaration a bounded offer publishes.
pub(crate) type BoundedOfferParts = (
    LoopCertificate,
    ShortcutDecisionSchema,
    Option<DecisionTemplate>,
);

/// The refusal of a span whose optional choices are not `holder`'s alone: more than one seat's
/// is a fragmented loop (CR 732.3), and one other seat's is that seat's period.
fn seat_refusal(body: &[TraceEntry], holder: PlayerId) -> Option<OfferRefusal> {
    let mut seats: Vec<PlayerId> = body
        .iter()
        .filter(|entry| match &entry.kind {
            // CR 117.1a + CR 117.1b: casting a spell or activating an ability is optional.
            EntryKind::Play { .. } => true,
            EntryKind::Answer { optional, .. } => *optional == AnswerOptionality::Optional,
            EntryKind::Resolution { .. } => false,
        })
        .map(|entry| entry.seat)
        .collect();
    seats.sort_unstable();
    seats.dedup();
    match seats[..] {
        [] => None,
        [seat] if seat == holder => None,
        [seat] => Some(OfferRefusal::ForeignSeat { seat }),
        _ => Some(OfferRefusal::Fragmented { seats }),
    }
}

/// The span's plays and answers as `frame` replays them, rotated to begin at the trace's current
/// position; priority passes are left to the replay (CR 117.4).
fn period_items(
    entries: &im::Vector<TraceEntry>,
    span: NamedSpan,
    frame: &GameState,
) -> Vec<PeriodItem> {
    let len = span.end - span.start;
    let rotation = if span.end < entries.len() {
        (entries.len() - span.start) % len
    } else {
        0
    };
    (span.start + rotation..span.end)
        .chain(span.start..span.start + rotation)
        .filter_map(|at| {
            let entry = &entries[at];
            // CR 111.7 + CR 111.1: a play whose token source the period made again is the play
            // one period later, made from that token, which the period minted before it.
            if let (
                EntryKind::Play {
                    action,
                    locus: PlayLocus::Mana(spent, _),
                    ..
                },
                Some(source_id),
            ) = (&entry.kind, play_trace::remade_play_source(frame, span, at))
            {
                let minted = frame.next_object_id - entries[entries.len() - len].next_object_id;
                let remade = |id: ObjectId| if id == *spent { source_id } else { id };
                return Some(PeriodItem {
                    seat: entry.seat,
                    action: renamed(action, remade, frame),
                    play: Some((entry.depth, entry.prompt)),
                    mandatory_answer: false,
                    next_object_id: entry.next_object_id + minted,
                    minted_since: entry.next_object_id,
                    cost_move: None,
                });
            }
            let minted_since = entries[at.saturating_sub(len)].next_object_id;
            let (action, play, cost_move, mandatory_answer) = match &entry.kind {
                EntryKind::Play { action, .. } => {
                    (action, Some((entry.depth, entry.prompt)), None, false)
                }
                EntryKind::Answer {
                    action: GameAction::PassPriority,
                    ..
                } if entry.prompt == PromptClass::Priority => return None,
                EntryKind::Answer {
                    action,
                    cost_move,
                    optional,
                    ..
                } => (
                    action,
                    None,
                    cost_move.clone(),
                    *optional == AnswerOptionality::Mandatory,
                ),
                EntryKind::Resolution { .. } => return None,
            };
            Some(PeriodItem {
                seat: entry.seat,
                action: action.clone(),
                play,
                mandatory_answer,
                next_object_id: entry.next_object_id,
                minted_since,
                cost_move,
            })
        })
        .collect()
}

/// CR 400.7: the recorded action with each object the period minted before it replaced by the
/// replay's object minted at the same point; every other object is the one recorded.
fn rebound(item: &PeriodItem, replay: &GameState) -> GameAction {
    let shift = replay.next_object_id.wrapping_sub(item.next_object_id);
    let map = |id: ObjectId| {
        if (item.minted_since..item.next_object_id).contains(&id.0) {
            ObjectId(id.0.wrapping_add(shift))
        } else {
            id
        }
    };
    renamed(&item.action, map, replay)
}

/// `action` with each object it names replaced by the one `map` gives for it. CR 400.7: a mana
/// source selection whose source is replaced names that object as `state` holds it.
fn renamed(
    action: &GameAction,
    map: impl Fn(ObjectId) -> ObjectId,
    state: &GameState,
) -> GameAction {
    let target = |t: &TargetRef| match t {
        TargetRef::Object(id) => TargetRef::Object(map(*id)),
        TargetRef::Player(p) => TargetRef::Player(*p),
    };
    let selected = |selection: &ManaSourceSelection| {
        let id = map(selection.source.object_id);
        ManaSourceSelection {
            source: state
                .objects
                .get(&id)
                .filter(|_| id != selection.source.object_id)
                .map_or(selection.source, ObjectIncarnationRef::from_object),
            ..selection.clone()
        }
    };
    match action {
        GameAction::ChooseTarget { target: chosen } => GameAction::ChooseTarget {
            target: chosen.as_ref().map(target),
        },
        GameAction::SelectTargets { targets } => GameAction::SelectTargets {
            targets: targets.iter().map(target).collect(),
        },
        GameAction::SelectCards { cards } => GameAction::SelectCards {
            cards: cards.iter().copied().map(map).collect(),
        },
        GameAction::ActivateAbility {
            source_id,
            ability_index,
            payment_mode,
        } => GameAction::ActivateAbility {
            source_id: map(*source_id),
            ability_index: *ability_index,
            payment_mode: *payment_mode,
        },
        GameAction::TapLandForMana { selection } => GameAction::TapLandForMana {
            selection: selected(selection),
        },
        GameAction::ActivateManaSource { selection } => GameAction::ActivateManaSource {
            selection: selected(selection),
        },
        GameAction::CastSpell {
            object_id,
            card_id,
            targets,
            payment_mode,
        } => GameAction::CastSpell {
            object_id: map(*object_id),
            card_id: *card_id,
            targets: targets.iter().copied().map(map).collect(),
            payment_mode: *payment_mode,
        },
        // An action naming no object, or one whose objects the period does not mint.
        other => other.clone(),
    }
}

/// What a replayed cycle ends on: the trigger on top, by source and text, or the kind of entry.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TopOfStack {
    Trigger {
        source_name: String,
        description: Option<String>,
    },
    Other(std::mem::Discriminant<StackEntryKind>),
}

fn top_of_stack(state: &GameState) -> Option<TopOfStack> {
    state.stack.back().map(|entry| match &entry.kind {
        StackEntryKind::TriggeredAbility {
            source_name,
            description,
            ..
        } => TopOfStack::Trigger {
            source_name: source_name.clone(),
            description: description.clone(),
        },
        other => TopOfStack::Other(std::mem::discriminant(other)),
    })
}

/// Where a cycle must end: priority back with `holder`, with the frame's top of stack and at least
/// its depth (accumulation beneath is admitted), at a window the period's reach admits.
struct FrameEnd {
    reach: PeriodReach,
    depth: usize,
    holder: PlayerId,
    top: Option<TopOfStack>,
}

impl FrameEnd {
    fn of(frame: &GameState, holder: PlayerId, reach: PeriodReach) -> Self {
        Self {
            reach,
            depth: frame.stack.len(),
            holder,
            top: top_of_stack(frame),
        }
    }

    /// Whether the cycle begun at window `start` may still run at `state`: the same step
    /// in-step, the same turn for a combat period (CR 500.8), and the holder's turns for an
    /// extra-turn period (CR 500.7). Nothing once the holder has left the game, whose turn goes on
    /// without it (CR 800.4a + CR 800.4j).
    fn admits(&self, start: play_trace::WindowKey, state: &GameState) -> bool {
        if !crate::game::players::is_alive(state, self.holder) {
            return false;
        }
        let now = play_trace::WindowKey::of(state);
        match self.reach {
            PeriodReach::InStep => now == start,
            PeriodReach::Combat => now.turn() == start.turn(),
            PeriodReach::ExtraTurn => state.active_player == self.holder,
        }
    }

    /// Whether the cycle begun at window `start` has come round at `state`; a carried cycle only
    /// at a later window of its start's step.
    fn reached(&self, start: play_trace::WindowKey, state: &GameState) -> bool {
        let now = play_trace::WindowKey::of(state);
        let window = match self.reach {
            PeriodReach::InStep => true,
            PeriodReach::Combat | PeriodReach::ExtraTurn => {
                now.phase() == start.phase() && now != start
            }
        };
        window
            && matches!(state.waiting_for, WaitingFor::Priority { player } if player == self.holder)
            && state.stack.len() >= self.depth
            && top_of_stack(state) == self.top
    }
}

/// Upper bound on the actions one replayed cycle may take.
const CYCLE_BEATS: usize = 512;

/// Applies one action, refusing a random outcome drawn by it (CR 732.2a); its events are appended
/// to `events`.
fn step(
    replay: &mut GameState,
    seat: PlayerId,
    action: GameAction,
    rejected: OfferRefusal,
    events: &mut Vec<GameEvent>,
) -> Result<(), OfferRefusal> {
    let outcomes = replay.rng.outcome_draws();
    events.extend(apply(replay, seat, action).map_err(|_| rejected)?.events);
    if replay.rng.outcome_draws() != outcomes {
        return Err(OfferRefusal::Randomness);
    }
    Ok(())
}

/// CR 601.2h + CR 702.51a: pays a cast's mana payment with the convoke tap set the live board
/// offers, in the detection replay's fodder-first order; how many creatures it tapped, or `None`
/// when this prompt is not such a payment.
fn rebind_convoke(
    replay: &mut GameState,
    events: &mut Vec<GameEvent>,
) -> Result<Option<usize>, OfferRefusal> {
    let (WaitingFor::ManaPayment { player, .. }, Some(pending)) =
        (&replay.waiting_for, replay.pending_cast.as_ref())
    else {
        return Ok(None);
    };
    let player = *player;
    let Some(taps) = select_convoke_taps(
        replay,
        player,
        &pending.cost,
        ConvokeTapOrder::DetectionFodderFirst,
    ) else {
        return Ok(None);
    };
    let before = replay.clone();
    let recorded = events.len();
    let tapped = taps.len();
    for (object_id, mana_type) in taps {
        let tapped = step(
            replay,
            player,
            GameAction::TapForConvoke {
                object_id,
                mana_type,
            },
            OfferRefusal::UnanswerablePrompt,
            events,
        );
        match tapped {
            Ok(()) => {}
            Err(OfferRefusal::UnanswerablePrompt) => {
                *replay = before;
                events.truncate(recorded);
                return Ok(None);
            }
            Err(refusal) => return Err(refusal),
        }
    }
    Ok(Some(tapped))
}

/// One replayed cycle from `replay`'s current state; the source of each triggered ability it
/// resolved, in order. `frames`, when given, collects every priority frame the cycle passes, and
/// `events` collects every event its actions emit.
fn replay_cycle(
    replay: &mut GameState,
    items: &[PeriodItem],
    end: &FrameEnd,
    choices: &mut Vec<PinnedDecision>,
    mut frames: Option<&mut Vec<GameState>>,
    events: &mut Vec<GameEvent>,
) -> Result<Vec<String>, OfferRefusal> {
    let mut done = vec![false; items.len()];
    let mut performed = Vec::new();
    let start = play_trace::WindowKey::of(replay);
    for beat in 0..CYCLE_BEATS {
        if replay.game_end.is_some() || !end.admits(start, replay) {
            return Err(OfferRefusal::NoRecurrence);
        }
        let next = done.iter().position(|d| !d);
        let next_play = next.map(|from| {
            (from..items.len())
                .find(|&at| !done[at] && items[at].play.is_some())
                .unwrap_or(items.len())
        });
        if let WaitingFor::Priority { player } = replay.waiting_for {
            if let Some(frames) = frames.as_deref_mut() {
                frames.push(replay.clone());
            }
            if beat > 0 && end.reached(start, replay) && cycle_complete(items, &done) {
                return Ok(performed);
            }
            let Some(at) = next else {
                pass(replay, player, &mut performed, events)?;
                continue;
            };
            let item = &items[at];
            if item.seat == player && item.play == Some((replay.stack.len(), PromptClass::Priority))
            {
                let action = rebound(item, replay);
                step(
                    replay,
                    player,
                    action,
                    OfferRefusal::IllegalReplayedPlay,
                    events,
                )?;
                done[at] = true;
            } else {
                pass(replay, player, &mut performed, events)?;
            }
            continue;
        }
        let Some(at) = next else {
            return Err(OfferRefusal::UnanswerablePrompt);
        };
        let item = &items[at];
        if item.play == Some((replay.stack.len(), PromptClass::Other)) {
            let action = rebound(item, replay);
            step(
                replay,
                item.seat,
                action,
                OfferRefusal::IllegalReplayedPlay,
                events,
            )?;
            done[at] = true;
            continue;
        }
        let candidates = at..next_play.unwrap_or(items.len());
        if !answer(replay, items, &mut done, candidates, choices, events)? {
            return Err(OfferRefusal::UnanswerablePrompt);
        }
    }
    Err(OfferRefusal::NoRecurrence)
}

/// Every item is made, except mandatory answers the replay went past without being asked them.
fn cycle_complete(items: &[PeriodItem], done: &[bool]) -> bool {
    let last_done = done.iter().rposition(|made| *made);
    items
        .iter()
        .zip(done)
        .enumerate()
        .all(|(at, (item, made))| {
            *made || (item.mandatory_answer && last_done.is_some_and(|last| at < last))
        })
}

/// CR 117.4: `player` passes, noting the triggered ability the pass resolves.
fn pass(
    replay: &mut GameState,
    player: PlayerId,
    performed: &mut Vec<String>,
    events: &mut Vec<GameEvent>,
) -> Result<(), OfferRefusal> {
    let top = replay.stack.back().and_then(|entry| {
        play_trace::resolution_identity(replay, entry).map(|identity| (entry.id, identity))
    });
    step(
        replay,
        player,
        GameAction::PassPriority,
        OfferRefusal::UnanswerablePrompt,
        events,
    )?;
    if let Some((id, identity)) = top {
        if replay.stack.iter().all(|entry| entry.id != id) {
            performed.push(identity);
        }
    }
    Ok(())
}

/// Answers the current prompt with the first unconsumed answer among `candidates` the reducer
/// accepts; `false` when none does.
fn answer(
    replay: &mut GameState,
    items: &[PeriodItem],
    done: &mut [bool],
    candidates: std::ops::Range<usize>,
    choices: &mut Vec<PinnedDecision>,
    events: &mut Vec<GameEvent>,
) -> Result<bool, OfferRefusal> {
    for at in candidates.clone() {
        if done[at] {
            continue;
        }
        let item = &items[at];
        if matches!(item.action, GameAction::TapForConvoke { .. }) {
            let convoked = convoke_choice(replay);
            if rebind_convoke(replay, events)?.is_some() {
                record_choice(choices, convoked)?;
                for convoke in candidates.clone() {
                    if matches!(items[convoke].action, GameAction::TapForConvoke { .. }) {
                        done[convoke] = true;
                    }
                }
                return Ok(true);
            }
            continue;
        }
        let offered = item
            .cost_move
            .as_ref()
            .and_then(|_| CostChoices::at(replay));
        let action = rebound(item, replay);
        let choice = answered_choice(replay, &action);
        match step(
            replay,
            item.seat,
            action,
            OfferRefusal::UnanswerablePrompt,
            events,
        ) {
            Ok(()) => {}
            Err(OfferRefusal::UnanswerablePrompt) => continue,
            Err(refusal) => return Err(refusal),
        }
        done[at] = true;
        record_choice(choices, choice)?;
        if let (Some(recorded), Some(offered)) = (&item.cost_move, offered) {
            if offered.moved(replay).arrivals != recorded.arrivals {
                return Err(OfferRefusal::ArrivalDiverged);
            }
        }
        return Ok(true);
    }
    // CR 601.2h + CR 702.51a: a recorded payment the live board cannot repeat is made with the
    // creatures it can tap instead, as a recorded convoke is.
    let convoked = convoke_choice(replay);
    if rebind_convoke(replay, events)?.is_some_and(|tapped| tapped > 0) {
        record_choice(choices, convoked)?;
        return Ok(true);
    }
    Ok(false)
}

/// CR 732.2a: records `choice` as the next occurrence of its source and point, so each time a
/// cycle asks one source the same choice is a slot a declaration can name.
fn record_choice(
    choices: &mut Vec<PinnedDecision>,
    choice: Option<PinnedDecision>,
) -> Result<(), OfferRefusal> {
    if let Some(choice) = choice {
        let choice = choice
            .at_occurrence_among(choices)
            .ok_or(OfferRefusal::UnanswerablePrompt)?;
        choices.push(choice);
    }
    Ok(())
}

/// CR 702.51a: the convoke a replayed cast is about to pay, by the card cast.
fn convoke_choice(replay: &GameState) -> Option<PinnedDecision> {
    let pending = replay.pending_cast.as_ref()?;
    let card_id = replay.objects.get(&pending.object_id)?.card_id;
    Some(PinnedDecision::ConvokeTaps {
        slot: DecisionSlot::first(
            YieldTarget::AllCopies {
                card_id,
                trigger_description: None,
            },
            ChoicePoint::ConvokeTaps,
        ),
    })
}

/// CR 732.2a: the game choice `action` makes at the prompt in hand, as a declaration names it: a
/// target (CR 601.2c, CR 603.3d), a cost's tapped creatures (CR 601.2h), a mana color (CR 605.3a),
/// a "may" (CR 603.5), a proliferate set (CR 701.34a), or a set chosen at resolution (CR 608.2d);
/// `None` for any other answer.
fn answered_choice(replay: &GameState, action: &GameAction) -> Option<PinnedDecision> {
    let source = |id: ObjectId| object_decision_source(replay, id);
    let identities = |ids: &[ObjectId]| {
        ids.iter()
            .map(|id| source(*id).map(TargetPin::ByIdentity))
            .collect::<Option<Vec<_>>>()
            .filter(|pins| !pins.is_empty())
    };
    let set = |slot_source: ObjectId, point: ChoicePoint, targets: Option<Vec<TargetPin>>| {
        Some(PinnedDecision::Targets {
            slot: DecisionSlot::first(source(slot_source)?, point),
            targets: targets?,
        })
    };
    let announced = |slot_source: ObjectId, chosen: &[TargetRef]| {
        let targets = announced_target_pins(replay, chosen).filter(|pins| !pins.is_empty());
        set(slot_source, ChoicePoint::AnnouncedTarget, targets)
    };
    match (&replay.waiting_for, action) {
        (
            WaitingFor::TriggerTargetSelection {
                source_id: Some(slot_source),
                ..
            },
            GameAction::ChooseTarget { target },
        ) => announced(*slot_source, target.as_slice()),
        (
            WaitingFor::TriggerTargetSelection {
                source_id: Some(slot_source),
                ..
            },
            GameAction::SelectTargets { targets },
        ) => announced(*slot_source, targets),
        (WaitingFor::TargetSelection { pending_cast, .. }, GameAction::ChooseTarget { target }) => {
            announced(pending_cast.object_id, target.as_slice())
        }
        (
            WaitingFor::TargetSelection { pending_cast, .. },
            GameAction::SelectTargets { targets },
        ) => announced(pending_cast.object_id, targets),
        (
            WaitingFor::PayCost {
                kind: PayCostKind::TapCreatures { .. },
                resume: CostResume::ManaAbility { mana_ability },
                ..
            },
            GameAction::SelectCards { cards },
        ) => set(
            mana_ability.source_id,
            ChoicePoint::TapCost,
            identities(cards),
        ),
        (
            WaitingFor::EffectZoneChoice {
                source_id,
                is_cost_payment: false,
                ..
            },
            GameAction::SelectCards { cards },
        ) => set(*source_id, ChoicePoint::ResolutionSet, identities(cards)),
        (WaitingFor::ProliferateChoice { .. }, GameAction::SelectTargets { targets }) => set(
            replay.active_proliferate_frame()?.source_id,
            ChoicePoint::ProliferateSet,
            Some(proliferate_pins(replay, targets)).filter(|pins| !pins.is_empty()),
        ),
        (
            WaitingFor::OptionalEffectChoice { source_id, .. },
            GameAction::DecideOptionalEffect { accept },
        ) => Some(PinnedDecision::MayChoice {
            slot: DecisionSlot::first(source(*source_id)?, ChoicePoint::MayGate),
            take: if *accept {
                MayChoiceOption::Take
            } else {
                MayChoiceOption::Decline
            },
        }),
        (
            WaitingFor::ChooseManaColor {
                context: ManaChoiceContext::ManaAbility(pending),
                ..
            },
            GameAction::ChooseManaColor { choice, .. },
        ) => Some(PinnedDecision::ManaColor {
            slot: DecisionSlot::first(source(pending.source_id)?, ChoicePoint::ManaColor),
            color: pinnable_mana_color(choice)?,
        }),
        _ => None,
    }
}

/// The frame with every object the period cast removed, since a cast card returns where it
/// came from (CR 400.7), and its churning bookkeeping cleared.
pub(crate) fn normalize_cast_frame(state: &GameState, casts: &[ObjectId]) -> GameState {
    let mut normalized = frame_without(state, casts);
    clear_frame_bookkeeping(&mut normalized);
    normalized
}

/// A span replayed twice from a priority frame: the stage the confirmer and the
/// mandatory-recurrence witness share.
struct ReplayedPeriod {
    holder: PlayerId,
    items: Vec<PeriodItem>,
    performance: Option<PeriodPerformance>,
    end: FrameEnd,
    choices: Vec<PinnedDecision>,
    casts: Vec<ObjectId>,
    #[cfg(any(test, feature = "test-support"))]
    performed: Vec<String>,
    s_n: GameState,
    s_n1: GameState,
    /// The frame the second cycle reached, or where it was refused.
    replay: GameState,
    second: Result<Vec<String>, OfferRefusal>,
    /// Every frame above is read inside the probe.
    _probe: SimulationProbeGuard,
}

/// CR 117.3d + CR 732.2a: the frame a replay starts from, `holder`'s view with the standing-pass
/// overlays cleared, since the replay takes only the period's own passes.
fn replay_base(frame: &GameState, holder: PlayerId) -> GameState {
    let mut base = crate::game::visibility::proposer_hidden_view(frame, holder);
    base.auto_pass.clear();
    base.stack_resolution_session = None;
    base
}

/// CR 405.5 + CR 608.1 + CR 400.7: the bottom of `frame`'s stack that `next` keeps unchanged,
/// where an entry is its id and the incarnation of the object at it.
fn stack_floor(frame: &GameState, next: &GameState) -> usize {
    let key = |state: &GameState, entry: &crate::types::game_state::StackEntry| {
        let incarnation = state
            .objects
            .get(&entry.id)
            .map(|object| object.incarnation);
        (entry.id, incarnation)
    };
    frame
        .stack
        .iter()
        .zip(next.stack.iter())
        .take_while(|(a, b)| key(frame, a) == key(next, b))
        .count()
}

/// CR 405.5 + CR 608.1: floors are measured on the frames the cover compares, and a refused cycle
/// keeps nothing.
fn period_floors(
    frames: [&GameState; 4],
    third: &Result<Vec<String>, OfferRefusal>,
    casts: &[ObjectId],
) -> [usize; 3] {
    let [a, b, c, d] = frames.map(|frame| normalize_cast_frame(frame, casts));
    let closing = if third.is_ok() {
        stack_floor(&c, &d)
    } else {
        0
    };
    [stack_floor(&a, &b), stack_floor(&b, &c), closing]
}

impl ReplayedPeriod {
    /// Each replayed frame's floor; the closing frame's needs a third cycle.
    fn floors(&self) -> [usize; 3] {
        let mut next = self.replay.clone();
        let third = replay_cycle(
            &mut next,
            &self.items,
            &self.end,
            &mut Vec::new(),
            None,
            &mut Vec::new(),
        );
        period_floors(
            [&self.s_n, &self.s_n1, &self.replay, &next],
            &third,
            &self.casts,
        )
    }
}

/// CR 732.2a: replays the span `span` of the current trace twice from `frame`, a priority frame;
/// the first cycle's refusal refuses, and the second's is kept for the reader.
fn replay_period(frame: &GameState, span: NamedSpan) -> Result<ReplayedPeriod, OfferRefusal> {
    let WaitingFor::Priority { player: holder } = frame.waiting_for else {
        return Err(OfferRefusal::UnanswerablePrompt);
    };
    let Some(entries) = play_trace::span_entries(frame, span) else {
        return Err(OfferRefusal::NoRecurrence);
    };
    let body: Vec<TraceEntry> = entries
        .iter()
        .skip(span.start)
        .take(span.end - span.start)
        .cloned()
        .collect();
    if let Some(refusal) = seat_refusal(&body, holder) {
        return Err(refusal);
    }
    let items = period_items(entries, span, frame);
    let performance = Some(PeriodPerformance::of(&body));
    #[cfg(feature = "test-support")]
    crate::game::perf_counters::record_play_trace(|counters| counters.confirm_drives += 1);
    let probe = SimulationProbeGuard::enter();
    let s_n = replay_base(frame, holder);
    let end = FrameEnd::of(&s_n, holder, span.reach);
    let mut replay = s_n.clone();
    let mut choices = Vec::new();
    #[cfg_attr(not(any(test, feature = "test-support")), allow(unused_variables))]
    let performed = replay_cycle(
        &mut replay,
        &items,
        &end,
        &mut choices,
        None,
        &mut Vec::new(),
    )?;
    let s_n1 = replay.clone();
    let second = replay_cycle(
        &mut replay,
        &items,
        &end,
        &mut Vec::new(),
        None,
        &mut Vec::new(),
    );
    let casts = items
        .iter()
        .filter_map(|item| match item.action {
            GameAction::CastSpell { object_id, .. } => Some(object_id),
            _ => None,
        })
        .collect();
    Ok(ReplayedPeriod {
        holder,
        items,
        performance,
        end,
        choices,
        casts,
        #[cfg(any(test, feature = "test-support"))]
        performed,
        s_n,
        s_n1,
        replay,
        second,
        _probe: probe,
    })
}

/// CR 732.2a: confirms the span `span` of the current trace as a period `frame`'s priority
/// holder may propose to repeat.
pub(crate) fn confirm(frame: &GameState, span: NamedSpan) -> Result<Confirmation, OfferRefusal> {
    let ReplayedPeriod {
        holder,
        items,
        performance,
        end,
        choices,
        casts,
        #[cfg(any(test, feature = "test-support"))]
        performed,
        s_n,
        s_n1,
        mut replay,
        second,
        _probe,
    } = replay_period(frame, span)?;
    // A period whose second cycle is refused has one pair of frames, read as any pair is.
    let (prior, current) = match second {
        Ok(_) => (&s_n1, &replay),
        Err(_) => (&s_n, &s_n1),
    };
    let (verdict, growth) = certify_object_growth_frames(
        [&s_n, &s_n1, current],
        |state| normalize_cast_frame(state, &casts),
        holder,
        None,
    );
    // CR 732.2a: one count per offer. Something of its proposer's own falling makes the period
    // the counting replay's; any other period with a loss axis is the threshold authority's.
    // A spend verdict asks the sign check only for its refusals.
    let sign = match (&verdict, verdict.certifies()) {
        (ObjectGrowthVerdict::Spend, _) => {
            period_sign_check(prior, current, holder).map(|_| PeriodSign::Spend)
        }
        (
            ObjectGrowthVerdict::FodderGrowth(_) | ObjectGrowthVerdict::ResourceRecurrence(_),
            true,
        ) => period_sign_check(prior, current, holder),
        (
            ObjectGrowthVerdict::FodderGrowth(_) | ObjectGrowthVerdict::ResourceRecurrence(_),
            false,
        ) => Err(OfferRefusal::Cover(verdict.clone())),
    };
    let (delta, departure, bounded, growth, counted_by) = match (sign, second) {
        // Only a spend says why a period stopped after one repetition.
        (Ok(PeriodSign::Unbounded(_) | PeriodSign::LossAxis) | Err(_), Err(stopped)) => {
            return Err(stopped);
        }
        (Err(refusal), Ok(_)) => return Err(refusal),
        (Ok(PeriodSign::Unbounded(unbounded)), Ok(_)) => {
            let (delta, departure) = *unbounded;
            (delta, departure, None, growth, CountedBy::Threshold)
        }
        // CR 704.5a + CR 732.2a: a loss period may be repeated only up to the threshold crossings
        // the authority measures on its replayed frames.
        (Ok(PeriodSign::LossAxis), Ok(_)) => {
            let mut frames = Vec::new();
            replay_cycle(
                &mut s_n1.clone(),
                &items,
                &end,
                &mut Vec::new(),
                Some(&mut frames),
                &mut Vec::new(),
            )?;
            // CR 514.1: a turn-cycle period's count is bounded by its cleanup discard.
            let cleanup = (verdict
                == ObjectGrowthVerdict::ResourceRecurrence(Some(RecurrenceCover::TurnCycle)))
            .then(|| CleanupPair::measure(&frames, holder))
            .flatten();
            let parts = replay_bounded_offer(frame, holder, &frames, &choices, cleanup)
                .map_err(|_| OfferRefusal::LossAxis)?;
            let delta = ResourceVector::period(prior, current);
            (delta, None, Some(parts), growth, CountedBy::Threshold)
        }
        // CR 732.2a + CR 118.3: the count is the repetitions the reducer performs, and the offer
        // states no other.
        (Ok(PeriodSign::Spend), second) => {
            let points = pinned_decisions_to_points(&choices, frame, holder)
                .ok_or(OfferRefusal::UnanswerablePrompt)?;
            let mut frames = Vec::new();
            replay_cycle(
                &mut prior.clone(),
                &items,
                &end,
                &mut Vec::new(),
                Some(&mut frames),
                &mut Vec::new(),
            )?;
            let delta = ResourceVector::period(prior, current);
            let pair = Box::new([prior.clone(), current.clone()]);
            let mut count = Count::default();
            count.note(seats(&s_n), &s_n1, true, holder);
            let mut repeats = count.note(seats(&s_n1), &replay, second.is_ok(), holder);
            while repeats && count.repetitions < COUNT_CAP {
                let before = seats(&replay);
                let completed = replay_cycle(
                    &mut replay,
                    &items,
                    &end,
                    &mut Vec::new(),
                    None,
                    &mut Vec::new(),
                )
                .is_ok();
                repeats = count.note(before, &replay, completed, holder);
            }
            let parts = counted_offer(
                &frames,
                points,
                count.survived(holder),
                count.repetitions,
                counted_ending(holder, &s_n, &replay),
            );
            let departures = count.departures;
            return Ok(Confirmation {
                period: ConfirmedPeriod {
                    items,
                    growth: CoveredGrowth::PerformedOnly,
                    reach: span.reach,
                    choices,
                    counted_by: CountedBy::Replay { departures },
                    performance,
                },
                frames: pair,
                delta,
                departure: None,
                #[cfg(any(test, feature = "test-support"))]
                performed,
                bounded: Some(parts),
            });
        }
    };
    Ok(Confirmation {
        period: ConfirmedPeriod {
            items,
            growth,
            reach: span.reach,
            choices,
            counted_by,
            performance,
        },
        frames: Box::new([s_n1, replay]),
        delta,
        departure,
        #[cfg(any(test, feature = "test-support"))]
        performed,
        bounded,
    })
}

/// CR 104.4b + CR 732.4: whether the mandatory span `span`, replayed twice from `frame`, recurs
/// for every living player with net progress and nothing moving toward a state-based action:
/// no loss axis (CR 704.5c), no driving resource falling on a carried permanent (CR 704.5g,
/// CR 704.5i, CR 704.5q), and no -1/-1 counter growth (CR 704.5f).
pub(crate) fn mandatory_period_holds(frame: &GameState, span: NamedSpan) -> bool {
    let Ok(period @ ReplayedPeriod { second: Ok(_), .. }) = replay_period(frame, span) else {
        return false;
    };
    let floors = period.floors();
    let ReplayedPeriod {
        casts,
        s_n,
        s_n1,
        replay: s_n2,
        _probe,
        ..
    } = period;
    let living = seats(frame);
    let mut buried_growth = false;
    let covered = living.iter().all(|&seat| {
        let verdict = certify_object_growth_frames(
            [&s_n, &s_n1, &s_n2],
            |state| normalize_cast_frame(state, &casts),
            seat,
            Some(floors),
        )
        .0;
        buried_growth |=
            verdict == ObjectGrowthVerdict::ResourceRecurrence(Some(RecurrenceCover::StackGrowth));
        verdict.certifies()
    });
    if !covered {
        return false;
    }
    let mut delta = ResourceVector::delta(
        &ResourceVector::snapshot(&s_n1),
        &ResourceVector::snapshot(&s_n2),
    );
    delta.tokens_created += minted_battlefield_ids(&s_n1, &s_n2).len() as i64;
    // CR 104.4b + CR 405.5: entries the period buries are its unbounded axis.
    let buried_rises = buried_growth && floors[2] > floors[1];
    (delta.is_net_progress() || buried_rises)
        && has_no_loss_axis(&delta)
        && living
            .iter()
            .all(|&seat| driving_resources_non_decreasing_carried(&s_n1, &s_n2, seat, Some(&s_n)))
        && delta
            .counters
            .get(&(CounterClass::Minus1Minus1, ObjectClass::Creature))
            .is_none_or(|&grew| grew <= 0)
}

/// [`mandatory_period_holds`] on the mandatory span at `state`, a priority frame; `None` when
/// there is no such span.
#[cfg(any(test, feature = "test-support"))]
pub fn mandatory_period_holds_for_tests(state: &GameState) -> Option<bool> {
    let span = play_trace::mandatory_recurrence_span(state)?;
    Some(mandatory_period_holds(state, span))
}

/// CR 732.2a: the most repetitions the counting replay performs for one offer. An offer at the cap
/// states the cap, which the replay performed; what remains is the next offer's.
pub(crate) const COUNT_CAP: u32 = 25;

/// The players still in the game (CR 800.4a).
fn seats(state: &GameState) -> BTreeSet<PlayerId> {
    state
        .players
        .iter()
        .filter(|player| !player.is_eliminated)
        .map(|player| player.id)
        .collect()
}

/// CR 732.2a: the repetitions the reducer performed, with the seats that left the game in each.
#[derive(Default)]
struct Count {
    repetitions: u32,
    departures: Vec<PredictedDeparture>,
}

impl Count {
    /// Notes one replayed repetition from a frame holding `before` to `after`; whether another
    /// may follow. CR 104.2a + CR 800.4a: a repetition the game ends in, or `holder` leaves in,
    /// was performed although it did not come round, and any other refused repetition was not.
    fn note(
        &mut self,
        before: BTreeSet<PlayerId>,
        after: &GameState,
        completed: bool,
        holder: PlayerId,
    ) -> bool {
        let ended = after.game_end.is_some() || !crate::game::players::is_alive(after, holder);
        if !completed && !ended {
            return false;
        }
        self.repetitions += 1;
        let left = before.difference(&seats(after)).copied().collect();
        self.departures
            .extend(PredictedDeparture::new(self.repetitions, left));
        completed
    }

    /// The repetitions `holder` is still in the game after: every one, or all but the last when
    /// `holder` leaves in it.
    fn survived(&self, holder: PlayerId) -> u32 {
        let leaves = self.departures.last().is_some_and(|last| {
            last.repetition == self.repetitions && last.seats.contains(&holder)
        });
        self.repetitions - u32::from(leaves)
    }
}

/// CR 104.2a + CR 732.2a: how the counted repetitions from `start` to `end` end the game, when
/// they end it with `holder` the winner.
fn counted_ending(holder: PlayerId, start: &GameState, end: &GameState) -> Option<WinKind> {
    let won = end
        .game_end
        .as_ref()
        .is_some_and(|game_end| game_end.winner == Some(holder));
    won.then(
        || match classify_win_kind(holder, &ResourceVector::period(start, end), Some(end)) {
            kind @ (WinKind::LethalDamage
            | WinKind::PoisonLoss
            | WinKind::Decking
            | WinKind::ImmediateWin) => kind,
            // CR 104.2b / CR 104.3e: no opponent's life, poison or library moved, so an effect won it.
            WinKind::ExtraTurns | WinKind::Advantage => WinKind::ImmediateWin,
        },
    )
}

/// CR 732.2c: performs one cycle of `period` from `state`, a frame where `holder` has priority,
/// ending where the cycle comes round again; the events it emits are appended to `events`.
pub(crate) fn perform_cycle(
    state: &mut GameState,
    period: &ConfirmedPeriod,
    holder: PlayerId,
    events: &mut Vec<GameEvent>,
) -> Result<(), OfferRefusal> {
    let end = FrameEnd::of(state, holder, period.reach);
    replay_cycle(state, &period.items, &end, &mut Vec::new(), None, events).map(drop)
}

/// Every span the current trace names, with the confirmer's verdict on each at `state`, which
/// must be a priority frame: the triggered abilities its first cycle resolved, or the refusal.
#[cfg(any(test, feature = "test-support"))]
pub fn confirm_for_tests(state: &GameState) -> Vec<(NamedSpan, Result<Vec<String>, OfferRefusal>)> {
    play_trace::current_named(state)
        .into_iter()
        .map(|span| {
            (
                span,
                confirm(state, span).map(|confirmation| confirmation.performed),
            )
        })
        .collect()
}

/// CR 608.1: one cycle of the span `state`'s trace last offered, replayed from `state` as its
/// acting player's priority frame; the printed identity of each triggered ability it resolved, in
/// order. `None` when nothing was offered.
#[cfg(any(test, feature = "test-support"))]
pub fn performed_for_tests(state: &GameState) -> Option<Result<Vec<String>, OfferRefusal>> {
    let span = play_trace::play_trace_view(state)?.offered?;
    let holder = state.waiting_for.acting_player()?;
    let items = period_items(play_trace::span_entries(state, span)?, span, state);
    let mut frame = state.clone();
    frame.waiting_for = WaitingFor::Priority { player: holder };
    frame.auto_pass.clear();
    let _probe = SimulationProbeGuard::enter();
    let frame = crate::game::visibility::proposer_hidden_view(&frame, holder);
    let end = FrameEnd::of(&frame, holder, span.reach);
    let mut replay = frame.clone();
    Some(replay_cycle(
        &mut replay,
        &items,
        &end,
        &mut Vec::new(),
        None,
        &mut Vec::new(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::game_state::{
        AutoPassMode, StackResolutionAutoPassOverlay, StackResolutionBudget, StackResolutionPolicy,
        StackResolutionSession, TurnBoundary,
    };

    fn with_stack(ids: &[u64]) -> GameState {
        use crate::types::ability::{Effect, QuantityExpr, ResolvedAbility, TargetFilter};
        use crate::types::game_state::{StackEntry, StackEntryKind};
        let mut state = GameState::new_two_player(7);
        state.stack.extend(ids.iter().map(|&id| StackEntry {
            id: ObjectId(id),
            source_id: ObjectId(500),
            controller: PlayerId(0),
            kind: StackEntryKind::TriggeredAbility {
                source_id: ObjectId(500),
                ability: Box::new(ResolvedAbility::new(
                    Effect::GainLife {
                        amount: QuantityExpr::Fixed { value: 1 },
                        player: TargetFilter::Controller,
                    },
                    vec![],
                    ObjectId(500),
                    PlayerId(0),
                )),
                condition: None,
                trigger_event: None,
                description: None,
                source_name: String::new(),
                subject_match_count: None,
                die_result: None,
                provenance: None,
            },
        }));
        state
    }

    /// CR 405.5 + CR 400.7: a floor keeps the entries the next frame holds unchanged, so a
    /// resolved trigger and a spell cast again at its old position are above it.
    #[test]
    fn a_floor_is_the_stack_the_next_frame_keeps_unchanged() {
        let spell_frame = |ids: &[u64], incarnation: u64| {
            let mut state = with_stack(ids);
            let mut spell = crate::game::game_object::GameObject::new(
                ObjectId(2),
                crate::types::identifiers::CardId(2),
                PlayerId(0),
                "Spell".to_string(),
                crate::types::zones::Zone::Stack,
            );
            spell.incarnation = incarnation;
            state.objects.insert(ObjectId(2), spell);
            state
        };
        let frame = spell_frame(&[1, 2, 3], 0);
        assert_eq!(
            (
                stack_floor(&frame, &spell_frame(&[1, 2, 3, 4], 0)),
                stack_floor(&frame, &spell_frame(&[1, 2, 5], 0)),
                stack_floor(&frame, &spell_frame(&[1, 2, 3], 1)),
            ),
            (3, 2, 1)
        );
    }

    /// CR 405.5 + CR 608.1: floors are measured on the frames the cover compares, which hold no
    /// spell the period cast, and a refused cycle keeps nothing for the frame it began at.
    #[test]
    fn floors_are_measured_without_the_periods_casts_and_need_a_completed_cycle() {
        let cast = ObjectId(9);
        let frame = |ids: &[u64]| {
            let mut state = with_stack(ids);
            state.objects.insert(
                cast,
                crate::game::game_object::GameObject::new(
                    cast,
                    crate::types::identifiers::CardId(9),
                    PlayerId(0),
                    "Cast".to_string(),
                    crate::types::zones::Zone::Stack,
                ),
            );
            state
        };
        let frames = [
            frame(&[9, 1]),
            frame(&[9, 1, 2]),
            frame(&[9, 1, 2, 3]),
            frame(&[9, 1, 2, 3, 4]),
        ];
        let [a, b, c, d] = &frames;
        assert_eq!(
            (
                period_floors([a, b, c, d], &Ok(Vec::new()), &[cast]),
                period_floors([a, b, c, d], &Err(OfferRefusal::NoRecurrence), &[cast]),
            ),
            ([1, 2, 3], [1, 2, 0])
        );
    }

    /// CR 117.3d: a replay takes only the period's own passes, so neither standing pass survives
    /// into its base.
    #[test]
    fn the_replay_base_drops_standing_pass_overlays() {
        let mut frame = GameState::new_two_player(7);
        let standing = AutoPassMode::UntilTurnBoundary {
            until: TurnBoundary::MyNextTurnStart,
        };
        frame.auto_pass.insert(PlayerId(1), standing.clone());
        frame.stack_resolution_session = Some(StackResolutionSession {
            entries: Vec::new(),
            cursor: 0,
            representatives: [PlayerId(0)].into_iter().collect(),
            verified_pass_representatives: BTreeSet::new(),
            budget: StackResolutionBudget::Unlimited,
            policy: StackResolutionPolicy::Committed,
            auto_pass_overlay: StackResolutionAutoPassOverlay {
                baseline: [(PlayerId(1), standing)].into_iter().collect(),
            },
        });
        let base = replay_base(&frame, PlayerId(0));
        assert!(base.auto_pass.is_empty());
        assert!(base.stack_resolution_session.is_none());
    }
}
