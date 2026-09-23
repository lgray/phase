//! CR 732.2a/b/c: a trigger-driven recorded period owned by the priority holder is offered at the
//! window where its recurrence stands on top of the stack, and a confirmed proposal replays it.
//!
//! Board A is `abdel_adrian_animate_dead_altar_board`'s and Board B is
//! `loop_period_trigger_driven_arming::build_board_b`'s, both driven through `apply()` and never
//! rebuilt. Both answer the CR 603.3b ordering prompt with `altar_resolves_first`.

use std::collections::BTreeMap;
use std::path::Path;

use engine::analysis::decision_template::{ChoicePoint, IterationCount};
use engine::analysis::loop_check::{OfferRoad, ShortcutResponse};
use engine::analysis::resource::{loop_detect_cost, reset_loop_detect_cost, LoopDetectCost};
use engine::game::scenario::{GameRunner, P0};
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::game_state::{
    CastPaymentMode, GameState, LoopAction, LoopDetectionMode, StackEntryKind, WaitingFor,
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

struct Drive {
    runner: GameRunner,
    minting: ObjectId,
    policy: Policy,
}

/// One `apply()`: the frame it was handed, the action, and the detector cost it paid.
struct Apply {
    before: GameState,
    action: GameAction,
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

/// Board A exiles only Animate Dead at Abdel Adrian's exile while accepts remain; Board B aims
/// each enters trigger at Felidar Guardian and accepts its exile while accepts remain.
fn start(board: Board) -> Option<Drive> {
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

/// A `Priority{P0}` frame whose top entry is the minting trigger, with a period recorded.
fn recurrence_window(state: &GameState, minting: ObjectId) -> bool {
    matches!(state.waiting_for, WaitingFor::Priority { player } if player == P0)
        && top_trigger_source(state) == Some(minting)
        && !state.last_loop_action_sequence.is_empty()
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
        altar_resolves_first(state)
            .or_else(|| (self.policy)(state))
            .unwrap_or_else(|| {
                engine::ai_support::legal_actions(state)
                    .into_iter()
                    .find(|action| !matches!(action, GameAction::PassPriority))
                    .expect("a prompt the declared drive does not answer offers a legal action")
            })
    }

    fn apply(&mut self, action: GameAction) -> Apply {
        let before = self.state().clone();
        reset_loop_detect_cost();
        self.runner
            .act(action.clone())
            .unwrap_or_else(|error| panic!("{action:?} was rejected: {error:?}"));
        Apply {
            before,
            action,
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
    assert_eq!(
        top_trigger_source(state),
        Some(drive.minting),
        "{board:?}: the offer stands at the window where the minting trigger is on top"
    );
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

/// CR 732.2a: a period that mixes a land tap into the trigger-driven record has no kind, so no
/// window of the rest of the game offers it.
#[test]
fn a_mixed_period_is_never_offered() {
    let Some(mut drive) = start(Board::A) else {
        return;
    };
    for _ in 0..BEAT_CAP {
        let state = drive.state();
        if matches!(state.waiting_for, WaitingFor::Priority { player } if player == P0)
            && !state.last_loop_action_sequence.is_empty()
        {
            break;
        }
        let action = drive.next_action();
        drive.apply(action);
    }
    let (_, _, grouped) = engine::ai_support::legal_actions_full(drive.state());
    let tap = grouped
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .flatten()
        .find(|action| matches!(action, GameAction::TapLandForMana { .. }))
        .expect("P0 can tap a land at its first window with a record");
    drive.apply(tap);
    let record = &drive.state().last_loop_action_sequence;
    assert!(
        record
            .iter()
            .any(|step| matches!(step.action, LoopAction::ResolveTrigger { .. }))
            && record
                .iter()
                .any(|step| matches!(step.action, LoopAction::TapLandForMana { .. })),
        "reach guard: the record mixes a trigger resolution with a land tap"
    );
    assert_eq!(drive.state().loop_period_controller_for_tests(), Some(P0));

    for _ in 0..BEAT_CAP {
        if matches!(drive.state().waiting_for, WaitingFor::GameOver { .. }) {
            return;
        }
        assert!(
            !is_offer(drive.state()),
            "a mixed period is offered at a {:?}-top window",
            top_trigger_source(drive.state())
        );
        let action = drive.next_action();
        drive.apply(action);
    }
    panic!("the drive did not reach the end of the game in {BEAT_CAP} beats");
}

/// CR 603.3d + CR 732.2a: a record that no longer answers the target prompt is refused by the
/// producer's drive, one apply before the untouched record's offer.
#[test]
fn a_record_missing_a_target_answer_is_not_offered() {
    let Some((_, applies)) = offered(Board::B) else {
        return;
    };
    let offer_apply = applies.last().expect("the offer apply");
    let replay = |strip: bool| {
        let mut runner = GameRunner::from_state(offer_apply.before.clone());
        let removed = runner
            .state_mut()
            .last_loop_action_sequence
            .last_mut()
            .map(|step| {
                let before = step.pins.len();
                if strip {
                    step.pins
                        .retain(|pin| pin.slot().point != ChoicePoint::AnnouncedTarget);
                }
                before - step.pins.len()
            })
            .expect("the record is non-empty one apply before the offer");
        reset_loop_detect_cost();
        runner
            .act(offer_apply.action.clone())
            .expect("the policy's answer is accepted");
        (removed, is_offer(runner.state()), loop_detect_cost())
    };

    let (_, control, _) = replay(false);
    assert!(control, "control: the untouched record is offered");
    let (removed, offered, cost) = replay(true);
    assert!(removed > 0, "the fixture removed the target answer");
    assert!(
        !offered,
        "a record missing its target answer is not offered"
    );
    assert_eq!(
        cost.object_growth_calls, 1,
        "the producer was entered and its drive refused"
    );
}

/// CR 732.2a + CR 732.2c: an until-lethal proposal on a trigger-driven offer, whose mint names no
/// winner, ends at priority with the board untouched and the record cleared.
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
        state.last_loop_action_sequence.is_empty(),
        "the fallback clears the record"
    );
}

/// CR 603.6a: the offer apply's relief consults reach past the shape stage.
#[test]
fn the_offer_apply_consults_relief_past_the_shape_stage() {
    for board in BOARDS {
        let Some((_, applies)) = offered(board) else {
            return;
        };
        let cost = offer_cost(board, &applies);
        assert!(
            cost.etb_relief_consults()
                - cost.etb_relief_shape_declined
                - cost.etb_relief_pre_gate_refused
                > 0,
            "{board:?}: a consult passed the shape stage; {cost:?}"
        );
    }
}

/// CR 400.7 + CR 603.6a: at the offer apply a blinked resident the relieved definitions' matchers
/// match refuses relief at the identity stage, and no consult grants it.
#[test]
fn an_identity_unstable_resident_refuses_relief_at_the_offer_apply() {
    for board in BOARDS {
        let Some((_, applies)) = offered(board) else {
            return;
        };
        let cost = offer_cost(board, &applies);
        assert!(
            cost.etb_relief_identity_refused > 0,
            "{board:?}: a consult was refused at the identity stage; {cost:?}"
        );
        assert_eq!(
            cost.etb_relief_excluded, 0,
            "{board:?}: no consult granted relief; {cost:?}"
        );
    }
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
                matches!(drive.state().waiting_for, WaitingFor::Priority { .. }),
                "Board A: the post-take window's cover refuses; got {}",
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
