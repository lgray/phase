//! CR 732.2a: the confirmer. It replays a candidate period the play trace names, twice, from the
//! priority frame it is asked at, on the proposer's own information set, and certifies the
//! replay's frames through the recurrence cover and the sign check.
//!
//! The trace (`play_trace`) records and names; this module decides whether a named span is a
//! period a shortcut may repeat.

use serde::{Deserialize, Serialize};

use crate::analysis::decision_template::{
    ChoicePoint, DecisionSlot, MayChoiceOption, PinnedDecision, TargetPin,
};
use crate::analysis::resource::{
    frame_without, CertifiedInstructedDeparture, CoveredGrowth, ObjectGrowthVerdict, ResourceVector,
};
use crate::game::engine::{
    announced_target_pins, apply, certify_object_growth_frames, clear_frame_bookkeeping,
    object_decision_source, period_sign_check, pinnable_mana_color, proliferate_pins,
    SimulationProbeGuard,
};
use crate::game::mana_payment::{select_convoke_taps, ConvokeTapOrder};
use crate::game::play_trace::{
    self, AnswerOptionality, CostChoices, CostMove, EntryKind, NamedSpan, PromptClass, TraceEntry,
};
use crate::types::ability::TargetRef;
use crate::types::actions::GameAction;
use crate::types::game_state::{
    CostResume, GameState, ManaChoiceContext, PayCostKind, StackEntryKind, WaitingFor, YieldTarget,
};
use crate::types::identifiers::ObjectId;
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
    /// CR 704.5a: the period moves some player toward losing the game.
    LossAxis,
    /// The period consumes a resource that drives it.
    DrivingResourcesDecrease,
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
}

/// A confirmed period: what it replays, the replay's second frame pair, and the sign check's
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
    /// CR 732.2a: the game choices the first replayed cycle's prompts were answered with.
    pub(crate) choices: Vec<PinnedDecision>,
}

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

/// The span's plays and answers, rotated to begin at the trace's current position; priority
/// passes are left to the replay (CR 117.4).
fn period_items(entries: &im::Vector<TraceEntry>, span: NamedSpan) -> Vec<PeriodItem> {
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
    let target = |t: &TargetRef| match t {
        TargetRef::Object(id) => TargetRef::Object(map(*id)),
        TargetRef::Player(p) => TargetRef::Player(*p),
    };
    match &item.action {
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
        } => GameAction::ActivateAbility {
            source_id: map(*source_id),
            ability_index: *ability_index,
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

/// Where a cycle must end: priority back with `holder`, at the frame's step, with the frame's
/// top of stack and at least its depth (accumulation beneath is admitted).
struct FrameEnd {
    window: play_trace::WindowKey,
    depth: usize,
    holder: PlayerId,
    top: Option<TopOfStack>,
}

impl FrameEnd {
    fn of(frame: &GameState, holder: PlayerId) -> Self {
        Self {
            window: play_trace::WindowKey::of(frame),
            depth: frame.stack.len(),
            holder,
            top: top_of_stack(frame),
        }
    }

    fn reached(&self, state: &GameState) -> bool {
        matches!(state.waiting_for, WaitingFor::Priority { player } if player == self.holder)
            && state.stack.len() >= self.depth
            && top_of_stack(state) == self.top
    }
}

/// Upper bound on the actions one replayed cycle may take.
const CYCLE_BEATS: usize = 512;

/// Applies one action, refusing a random outcome drawn by it (CR 732.2a).
fn step(
    replay: &mut GameState,
    seat: PlayerId,
    action: GameAction,
    rejected: OfferRefusal,
) -> Result<(), OfferRefusal> {
    let outcomes = replay.rng.outcome_draws();
    apply(replay, seat, action).map_err(|_| rejected)?;
    if replay.rng.outcome_draws() != outcomes {
        return Err(OfferRefusal::Randomness);
    }
    Ok(())
}

/// CR 601.2h + CR 702.51a: pays a cast's mana payment with the convoke tap set the live board
/// offers, in the detection replay's fodder-first order; how many creatures it tapped, or `None`
/// when this prompt is not such a payment.
fn rebind_convoke(replay: &mut GameState) -> Result<Option<usize>, OfferRefusal> {
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
        );
        match tapped {
            Ok(()) => {}
            Err(OfferRefusal::UnanswerablePrompt) => {
                *replay = before;
                return Ok(None);
            }
            Err(refusal) => return Err(refusal),
        }
    }
    Ok(Some(tapped))
}

/// One replayed cycle from `replay`'s current state; the source of each triggered ability it
/// resolved, in order.
fn replay_cycle(
    replay: &mut GameState,
    items: &[PeriodItem],
    end: &FrameEnd,
    choices: &mut Vec<PinnedDecision>,
) -> Result<Vec<String>, OfferRefusal> {
    let mut done = vec![false; items.len()];
    let mut performed = Vec::new();
    for beat in 0..CYCLE_BEATS {
        if matches!(replay.waiting_for, WaitingFor::GameOver { .. })
            || play_trace::WindowKey::of(replay) != end.window
        {
            return Err(OfferRefusal::NoRecurrence);
        }
        let next = done.iter().position(|d| !d);
        let next_play = next.map(|from| {
            (from..items.len())
                .find(|&at| !done[at] && items[at].play.is_some())
                .unwrap_or(items.len())
        });
        if let WaitingFor::Priority { player } = replay.waiting_for {
            if beat > 0 && end.reached(replay) && cycle_complete(items, &done) {
                return Ok(performed);
            }
            let Some(at) = next else {
                pass(replay, player, &mut performed)?;
                continue;
            };
            let item = &items[at];
            if item.seat == player && item.play == Some((replay.stack.len(), PromptClass::Priority))
            {
                let action = rebound(item, replay);
                step(replay, player, action, OfferRefusal::IllegalReplayedPlay)?;
                done[at] = true;
            } else {
                pass(replay, player, &mut performed)?;
            }
            continue;
        }
        let Some(at) = next else {
            return Err(OfferRefusal::UnanswerablePrompt);
        };
        let item = &items[at];
        if item.play == Some((replay.stack.len(), PromptClass::Other)) {
            let action = rebound(item, replay);
            step(replay, item.seat, action, OfferRefusal::IllegalReplayedPlay)?;
            done[at] = true;
            continue;
        }
        let candidates = at..next_play.unwrap_or(items.len());
        if !answer(replay, items, &mut done, candidates, choices)? {
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
) -> Result<(), OfferRefusal> {
    let top = replay.stack.back().and_then(|entry| {
        play_trace::resolution_identity(replay, entry).map(|identity| (entry.id, identity))
    });
    step(
        replay,
        player,
        GameAction::PassPriority,
        OfferRefusal::UnanswerablePrompt,
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
) -> Result<bool, OfferRefusal> {
    for at in candidates.clone() {
        if done[at] {
            continue;
        }
        let item = &items[at];
        if matches!(item.action, GameAction::TapForConvoke { .. }) {
            let convoked = convoke_choice(replay);
            if rebind_convoke(replay)?.is_some() {
                choices.extend(convoked);
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
        match step(replay, item.seat, action, OfferRefusal::UnanswerablePrompt) {
            Ok(()) => {}
            Err(OfferRefusal::UnanswerablePrompt) => continue,
            Err(refusal) => return Err(refusal),
        }
        done[at] = true;
        choices.extend(choice);
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
    if rebind_convoke(replay)?.is_some_and(|tapped| tapped > 0) {
        choices.extend(convoked);
        return Ok(true);
    }
    Ok(false)
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

/// CR 732.2a: confirms the span `span` of the current trace as a period `frame`'s priority
/// holder may propose to repeat.
pub(crate) fn confirm(frame: &GameState, span: NamedSpan) -> Result<Confirmation, OfferRefusal> {
    let WaitingFor::Priority { player: holder } = frame.waiting_for else {
        return Err(OfferRefusal::UnanswerablePrompt);
    };
    let Some(entries) = play_trace::current_entries(frame) else {
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
    let items = period_items(entries, span);
    #[cfg(feature = "test-support")]
    crate::game::perf_counters::record_play_trace(|counters| counters.confirm_drives += 1);
    let _probe = SimulationProbeGuard::enter();
    let s_n = crate::game::visibility::proposer_hidden_view(frame, holder);
    let end = FrameEnd::of(&s_n, holder);
    let mut replay = s_n.clone();
    let mut choices = Vec::new();
    #[cfg_attr(not(any(test, feature = "test-support")), allow(unused_variables))]
    let performed = replay_cycle(&mut replay, &items, &end, &mut choices)?;
    let s_n1 = replay.clone();
    replay_cycle(&mut replay, &items, &end, &mut Vec::new())?;
    let s_n2 = replay;
    let casts: Vec<ObjectId> = items
        .iter()
        .filter_map(|item| match item.action {
            GameAction::CastSpell { object_id, .. } => Some(object_id),
            _ => None,
        })
        .collect();
    let (verdict, growth) = certify_object_growth_frames(
        [&s_n, &s_n1, &s_n2],
        |state| normalize_cast_frame(state, &casts),
        holder,
    );
    if !verdict.certifies() {
        return Err(OfferRefusal::Cover(verdict));
    }
    let (delta, departure) = period_sign_check(&s_n1, &s_n2, holder)?;
    Ok(Confirmation {
        period: ConfirmedPeriod { items, growth },
        frames: Box::new([s_n1, s_n2]),
        delta,
        departure,
        #[cfg(any(test, feature = "test-support"))]
        performed,
        choices,
    })
}

/// CR 732.2c: performs one cycle of `period` from `state`, a frame where `holder` has priority,
/// ending where the cycle comes round again.
pub(crate) fn perform_cycle(
    state: &mut GameState,
    period: &ConfirmedPeriod,
    holder: PlayerId,
) -> Result<(), OfferRefusal> {
    let end = FrameEnd::of(state, holder);
    replay_cycle(state, &period.items, &end, &mut Vec::new()).map(drop)
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
    let items = period_items(play_trace::current_entries(state)?, span);
    let mut frame = state.clone();
    frame.waiting_for = WaitingFor::Priority { player: holder };
    let _probe = SimulationProbeGuard::enter();
    let frame = crate::game::visibility::proposer_hidden_view(&frame, holder);
    let end = FrameEnd::of(&frame, holder);
    let mut replay = frame.clone();
    Some(replay_cycle(&mut replay, &items, &end, &mut Vec::new()))
}
