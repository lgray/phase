//! CR 732.2a: the confirmer. It replays a candidate period the play trace names, twice, from the
//! priority frame it is asked at, on the proposer's own information set, and certifies the
//! replay's frames through the recurrence cover and the sign check.
//!
//! The trace (`play_trace`) records and names; this module decides whether a named span is a
//! period a shortcut may repeat.

use crate::analysis::resource::{
    CertifiedInstructedDeparture, ObjectGrowthVerdict, ResourceVector,
};
use crate::game::engine::{
    apply, certify_object_growth_frames, clear_frame_bookkeeping, period_sign_check,
    SimulationProbeGuard,
};
use crate::game::mana_payment::{select_convoke_taps, ConvokeTapOrder};
use crate::game::play_trace::{
    self, AnswerOptionality, CostChoices, CostMove, EntryKind, NamedSpan, PromptClass, TraceEntry,
};
use crate::types::ability::TargetRef;
use crate::types::actions::GameAction;
use crate::types::game_state::{GameState, StackEntryKind, WaitingFor};
use crate::types::identifiers::ObjectId;
use crate::types::player::PlayerId;

/// Why the confirmer did not confirm a span, in the order its stages read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfferRefusal {
    /// CR 732.3: optional plays or answers from more than one player.
    Fragmented { seats: Vec<PlayerId> },
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
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PeriodItem {
    seat: PlayerId,
    action: GameAction,
    /// For a play: the stack depth and prompt class it was made at.
    play: Option<(usize, PromptClass)>,
    /// The object-id counter when it was recorded.
    next_object_id: u64,
    /// Ids from here up to `next_object_id` were minted in the period before it was recorded.
    minted_since: u64,
    cost_move: Option<CostMove>,
}

/// A confirmed period: what it replays, the replay's second frame pair, and the sign check's
/// delta.
#[derive(Debug, Clone)]
pub(crate) struct Confirmation {
    pub(crate) period: Vec<PeriodItem>,
    pub(crate) frames: Box<[GameState; 2]>,
    pub(crate) delta: ResourceVector,
    pub(crate) departure: Option<CertifiedInstructedDeparture>,
    /// The source of each triggered ability the first replayed cycle resolved, in order.
    pub(crate) performed: Vec<String>,
}

/// CR 732.3: the seats making optional choices in the span, when there is more than one.
fn fragmented(body: &[TraceEntry]) -> Option<OfferRefusal> {
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
    (seats.len() > 1).then_some(OfferRefusal::Fragmented { seats })
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
            let (action, play, cost_move) = match &entry.kind {
                EntryKind::Play { action, .. } => (action, Some((entry.depth, entry.prompt)), None),
                EntryKind::Answer {
                    action: GameAction::PassPriority,
                    ..
                } if entry.prompt == PromptClass::Priority => return None,
                EntryKind::Answer {
                    action, cost_move, ..
                } => (action, None, cost_move.clone()),
                EntryKind::Resolution { .. } => return None,
            };
            Some(PeriodItem {
                seat: entry.seat,
                action: action.clone(),
                play,
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
struct FrameEnd<'a> {
    frame: &'a GameState,
    holder: PlayerId,
    top: Option<TopOfStack>,
}

impl FrameEnd<'_> {
    fn reached(&self, state: &GameState) -> bool {
        matches!(state.waiting_for, WaitingFor::Priority { player } if player == self.holder)
            && state.stack.len() >= self.frame.stack.len()
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

/// CR 601.2h + CR 702.51a: pays a recorded convoke with the tap set the live board offers, in the
/// detection replay's fodder-first order; `false` when this prompt is not that payment.
fn rebind_convoke(replay: &mut GameState) -> Result<bool, OfferRefusal> {
    let (WaitingFor::ManaPayment { player, .. }, Some(pending)) =
        (&replay.waiting_for, replay.pending_cast.as_ref())
    else {
        return Ok(false);
    };
    let player = *player;
    let Some(taps) = select_convoke_taps(
        replay,
        player,
        &pending.cost,
        ConvokeTapOrder::DetectionFodderFirst,
    ) else {
        return Ok(false);
    };
    let before = replay.clone();
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
                return Ok(false);
            }
            Err(refusal) => return Err(refusal),
        }
    }
    Ok(true)
}

/// One replayed cycle from `replay`'s current state; the source of each triggered ability it
/// resolved, in order.
fn replay_cycle(
    replay: &mut GameState,
    items: &[PeriodItem],
    end: &FrameEnd<'_>,
) -> Result<Vec<String>, OfferRefusal> {
    let mut done = vec![false; items.len()];
    let mut performed = Vec::new();
    for beat in 0..CYCLE_BEATS {
        if matches!(replay.waiting_for, WaitingFor::GameOver { .. })
            || !play_trace::same_window(replay, end.frame)
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
            let Some(at) = next else {
                if beat > 0 && end.reached(replay) {
                    return Ok(performed);
                }
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
        if !answer(replay, items, &mut done, candidates)? {
            return Err(OfferRefusal::UnanswerablePrompt);
        }
    }
    Err(OfferRefusal::NoRecurrence)
}

/// CR 117.4: `player` passes, noting the triggered ability the pass resolves.
fn pass(
    replay: &mut GameState,
    player: PlayerId,
    performed: &mut Vec<String>,
) -> Result<(), OfferRefusal> {
    let top = replay.stack.back().and_then(|entry| match &entry.kind {
        StackEntryKind::TriggeredAbility { source_name, .. } => {
            Some((entry.id, source_name.clone()))
        }
        _ => None,
    });
    step(
        replay,
        player,
        GameAction::PassPriority,
        OfferRefusal::UnanswerablePrompt,
    )?;
    if let Some((id, source_name)) = top {
        if replay.stack.iter().all(|entry| entry.id != id) {
            performed.push(source_name);
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
) -> Result<bool, OfferRefusal> {
    for at in candidates.clone() {
        if done[at] {
            continue;
        }
        let item = &items[at];
        if matches!(item.action, GameAction::TapForConvoke { .. }) {
            if rebind_convoke(replay)? {
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
        match step(replay, item.seat, action, OfferRefusal::UnanswerablePrompt) {
            Ok(()) => {}
            Err(OfferRefusal::UnanswerablePrompt) => continue,
            Err(refusal) => return Err(refusal),
        }
        done[at] = true;
        if let (Some(recorded), Some(offered)) = (&item.cost_move, offered) {
            if offered.moved(replay).arrivals != recorded.arrivals {
                return Err(OfferRefusal::ArrivalDiverged);
            }
        }
        return Ok(true);
    }
    Ok(false)
}

/// The frame with every object the period cast removed, since a cast card returns where it
/// came from (CR 400.7), and its churning bookkeeping cleared.
fn normalize_cast_frame(state: &GameState, casts: &[ObjectId]) -> GameState {
    let mut normalized = state.clone();
    for id in casts {
        if let Some(object) = normalized.objects.get(id).cloned() {
            // allow-raw-zone: prunes a discarded comparison-frame clone, not a gameplay zone event.
            crate::game::zones::remove_from_zone(&mut normalized, *id, object.zone, object.owner);
            normalized.objects.remove(id);
        }
    }
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
    if let Some(refusal) = fragmented(&body) {
        return Err(refusal);
    }
    let items = period_items(entries, span);
    let _probe = SimulationProbeGuard::enter();
    let s_n = crate::game::visibility::proposer_hidden_view(frame, holder);
    let end = FrameEnd {
        frame: &s_n,
        holder,
        top: top_of_stack(&s_n),
    };
    let mut replay = s_n.clone();
    let performed = replay_cycle(&mut replay, &items, &end)?;
    let s_n1 = replay.clone();
    replay_cycle(&mut replay, &items, &end)?;
    let s_n2 = replay;
    let casts: Vec<ObjectId> = items
        .iter()
        .filter_map(|item| match item.action {
            GameAction::CastSpell { object_id, .. } => Some(object_id),
            _ => None,
        })
        .collect();
    let verdict = certify_object_growth_frames(
        [&s_n, &s_n1, &s_n2],
        |state| normalize_cast_frame(state, &casts),
        holder,
    );
    if !verdict.certifies() {
        return Err(OfferRefusal::Cover(verdict));
    }
    let (delta, departure) = period_sign_check(&s_n1, &s_n2, holder)?;
    Ok(Confirmation {
        period: items,
        frames: Box::new([s_n1, s_n2]),
        delta,
        departure,
        performed,
    })
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
