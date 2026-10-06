//! CR 732.2a/b/c: a trigger-driven recorded period owned by the priority holder is offered at the
//! window where its recurrence stands on top of the stack, and a confirmed proposal replays it.
//!
//! Board A is `abdel_adrian_animate_dead_altar_board`'s and Board B is
//! `loop_period_trigger_driven_arming::build_board_b`'s, both driven through `apply()` and never
//! rebuilt. Both answer the CR 603.3b ordering prompt with `altar_resolves_first`, except where a
//! row names `identity_order`.

use std::collections::BTreeMap;
use std::path::Path;

use engine::analysis::decision_template::{ChoicePoint, IterationCount};
use engine::analysis::loop_check::{OfferRoad, ShortcutResponse};
use engine::analysis::resource::{loop_detect_cost, reset_loop_detect_cost, LoopDetectCost};
use engine::game::scenario::{GameRunner, P0};
use engine::game::NamingCause;
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::game_state::{
    CastPaymentMode, GameState, LoopDetectionMode, StackEntryKind, WaitingFor,
};
use engine::types::identifiers::ObjectId;
use engine::types::zones::Zone;

use crate::loop_period_accessor_answers::altar_resolves_first;

/// Beat cap for every drive here; it bounds a runaway drive and is read by no assertion.
const BEAT_CAP: usize = 400;

/// How many of the board's voluntary choices a drive accepts before it declines.
const ACCEPTS: usize = 8;

#[derive(Debug, Clone, Copy)]
enum Board {
    A,
    B,
}

const BOARDS: [Board; 2] = [Board::A, Board::B];

type Policy = Box<dyn FnMut(&GameState) -> Option<GameAction>>;

type Ordering = fn(&GameState) -> Option<GameAction>;

struct Drive {
    runner: GameRunner,
    minting: ObjectId,
    order: Ordering,
    policy: Policy,
}

/// One `apply()`: the detector cost it paid.
struct Apply {
    cost: LoopDetectCost,
}

fn cast_animate_dead(runner: &mut GameRunner, animate_dead: ObjectId, target: ObjectId) {
    let card_id = runner.state().objects[&animate_dead].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: animate_dead,
            card_id,
            targets: vec![target],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("Animate Dead is castable from the built board");
}

/// CR 603.3b: every ordering prompt answered with the identity permutation.
fn identity_order(state: &GameState) -> Option<GameAction> {
    let WaitingFor::OrderTriggers { triggers, .. } = &state.waiting_for else {
        return None;
    };
    Some(GameAction::OrderTriggers {
        order: (0..triggers.len()).collect(),
    })
}

fn start(board: Board) -> Option<Drive> {
    start_ordered(board, altar_resolves_first)
}

/// Board A exiles only Animate Dead at Abdel Adrian's exile while accepts remain; Board B aims
/// each enters trigger at Felidar Guardian and accepts its exile while accepts remain.
fn start_ordered(board: Board, order: Ordering) -> Option<Drive> {
    match board {
        Board::A => {
            let built = crate::abdel_adrian_animate_dead_altar_board::build()?;
            let mut runner = built.runner;
            runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
            let returned = runner.state().objects[&built.animate_dead].card_id;
            cast_animate_dead(&mut runner, built.animate_dead, built.abdel);
            let mut left = ACCEPTS;
            Some(Drive {
                runner,
                minting: built.abdel,
                order,
                policy: Box::new(move |state| {
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
                }),
            })
        }
        Board::B => {
            let built = crate::loop_period_trigger_driven_arming::build_board_b()?;
            let mut runner = built.runner;
            cast_animate_dead(&mut runner, built.animate_dead, built.felidar);
            let felidar = built.felidar;
            let mut left = ACCEPTS;
            Some(Drive {
                runner,
                minting: built.preston,
                order,
                policy: Box::new(move |state| {
                    let legal = engine::ai_support::legal_actions(state);
                    if left > 0 {
                        if let Some(aimed) = legal.iter().find(|action| {
                            matches!(action, GameAction::ChooseTarget {
                                target: Some(TargetRef::Object(id)),
                            } if *id == felidar)
                        }) {
                            return Some(aimed.clone());
                        }
                    }
                    let accept = left > 0;
                    let decided = legal.iter().find(|action| {
                        matches!(action, GameAction::DecideOptionalEffect { accept: answered }
                            if *answered == accept)
                    })?;
                    if accept {
                        left -= 1;
                    }
                    Some(decided.clone())
                }),
            })
        }
    }
}

fn top_trigger_source(state: &GameState) -> Option<ObjectId> {
    match &state.stack.back()?.kind {
        StackEntryKind::TriggeredAbility { source_id, .. } => Some(*source_id),
        _ => None,
    }
}

/// A `Priority{P0}` frame whose top entry is the minting trigger, with a span its return to the
/// top named.
fn recurrence_window(state: &GameState, minting: ObjectId) -> bool {
    matches!(state.waiting_for, WaitingFor::Priority { player } if player == P0)
        && top_trigger_source(state) == Some(minting)
        && engine::game::play_trace_view(state).is_some_and(|view| {
            view.named
                .iter()
                .any(|span| span.cause == NamingCause::TriggerTop)
        })
}

fn is_offer(state: &GameState) -> bool {
    matches!(state.waiting_for, WaitingFor::LoopShortcut { .. })
}

fn token_count(state: &GameState) -> usize {
    state
        .battlefield
        .iter()
        .filter(|id| state.objects.get(id).is_some_and(|object| object.is_token))
        .count()
}

type PlayerZones = (i32, Vec<ObjectId>, Vec<ObjectId>, Vec<ObjectId>);

type BoardFingerprint = (
    BTreeMap<ObjectId, (Zone, bool)>,
    Vec<PlayerZones>,
    Vec<ObjectId>,
    Vec<ObjectId>,
);

/// Every object's zone and tapped state, each player's life, library, hand and graveyard, the
/// stack, and the battlefield.
fn board_fingerprint(state: &GameState) -> BoardFingerprint {
    (
        state
            .objects
            .values()
            .map(|object| (object.id, (object.zone, object.tapped)))
            .collect(),
        state
            .players
            .iter()
            .map(|player| {
                (
                    player.life,
                    player.library.iter().copied().collect(),
                    player.hand.iter().copied().collect(),
                    player.graveyard.iter().copied().collect(),
                )
            })
            .collect(),
        state.stack.iter().map(|entry| entry.id).collect(),
        state.battlefield.iter().copied().collect(),
    )
}

impl Drive {
    fn state(&self) -> &GameState {
        self.runner.state()
    }

    fn next_action(&mut self) -> GameAction {
        let state = self.runner.state();
        if matches!(state.waiting_for, WaitingFor::Priority { .. }) {
            return GameAction::PassPriority;
        }
        (self.order)(state)
            .or_else(|| (self.policy)(state))
            .unwrap_or_else(|| {
                engine::ai_support::legal_actions(state)
                    .into_iter()
                    .find(|action| !matches!(action, GameAction::PassPriority))
                    .expect("a prompt the declared drive does not answer offers a legal action")
            })
    }

    fn apply(&mut self, action: GameAction) -> Apply {
        reset_loop_detect_cost();
        self.runner
            .act(action.clone())
            .unwrap_or_else(|error| panic!("{action:?} was rejected: {error:?}"));
        Apply {
            cost: loop_detect_cost(),
        }
    }

    /// Drive until `apply()` returns an offer, a recurrence window, or the end of the game.
    fn drive_to_window(&mut self) -> Vec<Apply> {
        let mut applies = Vec::new();
        for _ in 0..BEAT_CAP {
            let action = self.next_action();
            applies.push(self.apply(action));
            let state = self.state();
            if is_offer(state)
                || recurrence_window(state, self.minting)
                || matches!(state.waiting_for, WaitingFor::GameOver { .. })
            {
                return applies;
            }
        }
        panic!("the drive reached no window in {BEAT_CAP} beats");
    }

    /// Declare `count` with no template and accept from every responder.
    fn take(&mut self, count: IterationCount) -> Vec<Apply> {
        let mut applies = vec![self.apply(GameAction::DeclareShortcut {
            count,
            template: None,
        })];
        assert!(
            matches!(
                self.state().waiting_for,
                WaitingFor::RespondToShortcut { .. }
            ),
            "the declaration opens the response window; got {}",
            self.state().waiting_for.variant_name()
        );
        while matches!(
            self.state().waiting_for,
            WaitingFor::RespondToShortcut { .. }
        ) {
            applies.push(self.apply(GameAction::RespondToShortcut {
                response: ShortcutResponse::Accept,
            }));
        }
        applies
    }
}

/// Each board driven to the first window `apply()` returns at, which must be its offer.
fn offered(board: Board) -> Option<(Drive, Vec<Apply>)> {
    let mut drive = start(board)?;
    let applies = drive.drive_to_window();
    let state = drive.state();
    assert!(
        matches!(
            state.waiting_for,
            WaitingFor::LoopShortcut { proposer, road: OfferRoad::RecordedPeriod, .. }
                if proposer == P0
        ),
        "{board:?}: the first recurrence window is a recorded-period offer to P0; got {} with \
         top {:?}",
        state.waiting_for.variant_name(),
        top_trigger_source(state)
    );
    match board {
        // CR 117.3b/c: a span a repeat names is offered at the first priority window after the
        // repeated resolution, before the minting trigger is back on top.
        Board::A => {
            assert_eq!(
                engine::game::play_trace_view(state)
                    .and_then(|view| view.offered)
                    .map(|span| span.cause),
                Some(NamingCause::Repeat),
                "A: the offered span is the one a repeat named"
            );
            assert_ne!(
                top_trigger_source(state),
                Some(drive.minting),
                "A: the offer stands before the minting trigger's recurrence window"
            );
        }
        Board::B => assert_eq!(
            top_trigger_source(state),
            Some(drive.minting),
            "B: the offer stands at the window where the minting trigger is on top"
        ),
    }
    Some((drive, applies))
}

fn offer_cost(board: Board, applies: &[Apply]) -> LoopDetectCost {
    let cost = applies.last().expect("the offer apply").cost;
    assert_eq!(
        cost.object_growth_calls, 1,
        "{board:?}: the production producer minted the offer on this apply"
    );
    cost
}

/// CR 732.2a + CR 732.2b + CR 732.2c: each board is offered at its first recurrence window, and
/// taking a fixed count replays the period.
#[test]
fn a_trigger_driven_period_is_offered_at_its_first_recurrence_window_and_taken() {
    for board in BOARDS {
        let Some((mut drive, applies)) = offered(board) else {
            return;
        };
        offer_cost(board, &applies);
        let (tokens, libraries): (usize, Vec<usize>) = (
            token_count(drive.state()),
            drive
                .state()
                .players
                .iter()
                .map(|player| player.library.len())
                .collect(),
        );
        drive.take(IterationCount::Fixed(3));
        let state = drive.state();
        assert!(
            matches!(state.waiting_for, WaitingFor::Priority { .. }),
            "{board:?}: the take ends at priority; got {}",
            state.waiting_for.variant_name()
        );
        assert!(
            token_count(state) > tokens,
            "{board:?}: the take minted tokens ({tokens} -> {})",
            token_count(state)
        );
        for (seat, player) in state.players.iter().enumerate().skip(1) {
            assert!(
                player.library.len() < libraries[seat],
                "{board:?}: seat {seat}'s library was milled by the take"
            );
        }
    }

    let restored = crate::committed_dump_walk::restore_committed(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/combo_infinite_pile_4p_offer.json.gz"),
    )
    .expect("the committed offer restores");
    assert!(
        matches!(
            restored.waiting_for,
            WaitingFor::LoopShortcut {
                road: OfferRoad::RecordedPeriod,
                ..
            }
        ),
        "live control: a priority-driven recorded-period offer restores as one"
    );
}

/// The names of the sources of the stack entries beneath the top one.
fn sources_beneath_top(state: &GameState) -> Vec<String> {
    state
        .stack
        .iter()
        .take(state.stack.len().saturating_sub(1))
        .map(|entry| {
            state
                .objects
                .get(&entry.source_id)
                .map_or_else(String::new, |object| object.name.clone())
        })
        .collect()
}

fn all_altar_triggers(sources: &[String]) -> bool {
    sources.iter().all(|name| name == "Altar of the Brood")
}

/// CR 603.3b + CR 732.2a + CR 732.2c: under identity ordering Board A's Altar of the Brood
/// triggers accumulate beneath the recurrence, and the period is offered and taken.
#[test]
fn an_accumulating_stack_beneath_the_recurrence_is_offered_and_taken() {
    let Some(mut drive) = start_ordered(Board::A, identity_order) else {
        return;
    };
    let mut offer_apply = None;
    for _ in 0..BEAT_CAP {
        let action = drive.next_action();
        let apply = drive.apply(action);
        if is_offer(drive.state()) {
            offer_apply = Some(apply);
            break;
        }
        if matches!(drive.state().waiting_for, WaitingFor::GameOver { .. }) {
            break;
        }
    }
    let offer_apply =
        offer_apply.expect("identity-ordered Board A is offered before the game ends");
    let state = drive.state();
    assert!(
        matches!(
            state.waiting_for,
            WaitingFor::LoopShortcut { proposer, road: OfferRoad::RecordedPeriod, .. }
                if proposer == P0
        ),
        "the offer is a recorded-period offer to P0"
    );
    assert_eq!(offer_apply.cost.object_growth_calls, 1);
    let beneath = sources_beneath_top(state);
    assert!(
        !beneath.is_empty() && all_altar_triggers(&beneath),
        "reach guard: Altar of the Brood triggers accumulate beneath the recurrence; {beneath:?}"
    );
    let tokens = token_count(state);
    let libraries: Vec<usize> = state
        .players
        .iter()
        .map(|player| player.library.len())
        .collect();

    drive.take(IterationCount::Fixed(3));
    let state = drive.state();
    assert!(
        matches!(state.waiting_for, WaitingFor::Priority { .. }),
        "the take ends at priority; got {}",
        state.waiting_for.variant_name()
    );
    assert_eq!(token_count(state), tokens + 3);
    for (seat, player) in state.players.iter().enumerate().skip(1) {
        assert_eq!(
            player.library.len(),
            libraries[seat],
            "seat {seat}'s mills wait beneath the recurrence"
        );
    }
    let after = sources_beneath_top(state);
    assert!(
        after.len() > beneath.len() && all_altar_triggers(&after),
        "the take left more Altar of the Brood triggers beneath the recurrence; {after:?}"
    );
    assert!(
        engine::game::play_trace_view(state).is_none_or(|view| view.offered.is_none()),
        "the take leaves no offer standing in the trace"
    );
}

/// CR 117.3b/c + CR 608.1: Board A is offered before its recurrence window, for the period its base
/// offer performs.
#[test]
fn board_a_is_offered_early_for_its_base_period() {
    let Some((drive, _)) = offered(Board::A) else {
        return;
    };
    let performed = engine::game::period_confirm::performed_for_tests(drive.state())
        .expect("reach: the offer names its span")
        .expect("the offered period replays");
    assert!(
        crate::loop_period_performs::same_up_to_rotation(
            &performed,
            &crate::loop_period_performs::BOARD_A_PERIOD
        ),
        "{performed:?}"
    );
}

/// CR 603.3d + CR 732.2a: Board B is offered at the window where Preston's trigger is back on top,
/// for the cycle it performed, and the offer asks the cycle's announced targets.
#[test]
fn board_b_is_offered_at_its_base_window_with_its_target_answers() {
    let Some((drive, _)) = offered(Board::B) else {
        return;
    };
    let state = drive.state();
    assert_eq!(
        engine::game::play_trace_view(state)
            .and_then(|view| view.offered)
            .map(|span| span.cause),
        Some(NamingCause::TriggerTop)
    );
    assert_eq!(
        engine::game::period_confirm::performed_for_tests(state),
        Some(Ok(vec![
            "Preston, the Vanisher".to_string(),
            "Altar of the Brood".to_string(),
            "Felidar Guardian".to_string(),
            "Altar of the Brood".to_string(),
            "Felidar Guardian".to_string(),
        ])),
        "one replayed cycle resolves Preston's trigger, then each Illusion's"
    );
    let WaitingFor::LoopShortcut { schema, .. } = &state.waiting_for else {
        unreachable!("offered() asserted the offer");
    };
    assert!(
        schema
            .points
            .iter()
            .any(|point| point.slot.point == ChoicePoint::AnnouncedTarget),
        "{:?}",
        schema.points
    );
}

/// CR 732.2a + CR 732.2c: an until-lethal proposal on a trigger-driven offer, whose mint names no
/// winner, ends at priority with the board untouched and the proposer's trace discarded.
#[test]
fn an_until_lethal_take_of_a_trigger_driven_offer_changes_nothing() {
    let Some((mut drive, _)) = offered(Board::A) else {
        return;
    };
    let offer = board_fingerprint(drive.state());
    drive.take(IterationCount::UntilLethal);
    let state = drive.state();
    let WaitingFor::Priority { player } = state.waiting_for else {
        panic!(
            "the fallback ends at priority; got {}",
            state.waiting_for.variant_name()
        );
    };
    assert!(
        state
            .players
            .iter()
            .any(|seat| seat.id == player && !seat.is_eliminated),
        "priority goes to a player still in the game"
    );
    assert_eq!(board_fingerprint(state), offer, "the board is the offer's");
    assert!(
        engine::game::play_trace_view(state).is_none_or(|view| view
            .entries
            .iter()
            .all(|entry| entry.seat != P0
                || matches!(entry.kind, engine::game::EntryKind::Answer { .. }))),
        "the fallback discards the proposer's plays and resolutions"
    );
}

/// Each apply evaluates the loop shortcut at most once, through the take and on to the next window
/// whose cover the producer asks.
#[test]
fn each_apply_evaluates_the_loop_shortcut_at_most_once() {
    for board in BOARDS {
        let Some((mut drive, mut applies)) = offered(board) else {
            return;
        };
        applies.extend(drive.take(IterationCount::Fixed(3)));
        let next = loop {
            let action = drive.next_action();
            let apply = drive.apply(action);
            let asked = apply.cost.object_growth_calls > 0;
            applies.push(apply);
            if asked {
                break applies.last().expect("just pushed").cost;
            }
            assert!(
                applies.len() < BEAT_CAP,
                "{board:?}: no later window asked the producer"
            );
        };
        assert_eq!(
            next.object_growth_calls, 1,
            "{board:?}: the post-take window asks the producer once"
        );
        if let Board::A = board {
            assert!(
                is_offer(drive.state()),
                "Board A: the post-take window is offered again; got {}",
                drive.state().waiting_for.variant_name()
            );
        }
        for (index, apply) in applies.iter().enumerate() {
            assert!(
                apply.cost.object_growth_calls <= 1 && apply.cost.reconcile_calls <= 1,
                "{board:?}: apply {index} evaluated the shortcut more than once; {:?}",
                apply.cost
            );
        }
    }
}

/// The ring road is asked first and the recorded road mints only while the ring leaves priority
/// standing; a ring mint over a trigger-driven record keeps the ring road.
#[test]
fn the_ring_road_is_asked_before_the_recorded_road() {
    let Some((_, applies)) = offered(Board::B) else {
        return;
    };
    assert_eq!(
        applies
            .last()
            .expect("the offer apply")
            .cost
            .reconcile_calls,
        1,
        "Board B: the ring bridge was entered and refused before the recorded road minted"
    );

    let mut state = crate::committed_dump_walk::restore_committed(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fantastic_four_bounded_loop_4p.json.gz"),
    )
    .expect("the committed board restores");
    let mut aimed_at = None;
    for _ in 0..BEAT_CAP {
        reset_loop_detect_cost();
        crate::loop_shortcut_drain_boards::drive_one_beat(&mut state, &mut aimed_at);
        if let WaitingFor::LoopShortcut { road, .. } = state.waiting_for {
            assert_eq!(
                road,
                OfferRoad::Ring,
                "the bounded-cycle mint is the ring road's"
            );
            assert_eq!(
                loop_detect_cost().object_growth_calls,
                0,
                "the recorded road is not asked once the ring road minted"
            );
            return;
        }
    }
    panic!("the committed board reached no offer in {BEAT_CAP} beats");
}

/// Offers `board` and takes it at `n`, metering only the take.
fn metered_take(board: Board, n: u32) -> GameState {
    let (mut drive, _) = offered(board).expect("the board builds from the card fixture");
    engine::game::perf_counters::reset();
    drive.take(IterationCount::Fixed(n));
    drive.state().clone()
}

/// Board A's take: its per-cycle history work does not grow with its count.
#[test]
fn trigger_driven_take_history_work_is_flat_per_cycle_a() {
    use crate::loop_shortcut::{
        assert_take_history_work_is_flat, TakeHistoryMap, TakeHistoryVector,
    };

    assert_take_history_work_is_flat(
        32,
        &[
            TakeHistoryVector::JournalEntries,
            TakeHistoryVector::BattlefieldEntries,
        ],
        &[
            TakeHistoryMap::AbilityResolutions,
            TakeHistoryMap::TrackedObjectSets,
            TakeHistoryMap::TrackedSetMemberCauses,
        ],
        |n| metered_take(Board::A, n),
    );
}

/// Board B's take: its per-cycle history work does not grow with its count.
#[test]
fn trigger_driven_take_history_work_is_flat_per_cycle_b() {
    use crate::loop_shortcut::{
        assert_take_history_work_is_flat, TakeHistoryMap, TakeHistoryVector,
    };

    assert_take_history_work_is_flat(
        32,
        &[
            TakeHistoryVector::JournalEntries,
            TakeHistoryVector::BattlefieldEntries,
        ],
        &[TakeHistoryMap::AbilityResolutions],
        |n| metered_take(Board::B, n),
    );
}
