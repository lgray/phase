//! CR 104.4b + CR 732.4: a loop of mandatory actions is a draw, whether the players pass
//! through it or take the auto-pass shortcut over it (CR 732.1), and even when each cycle
//! only re-mints identities, discloses a card, or asks a choice with one legal option. A
//! loop holding an optional action — a "may" answered or remembered, a remembered optional
//! replacement, a choice among several targets, or an out some player holds — is not.

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::time::Duration;

use engine::game::perf_counters::play_trace_counters;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::actions::{GameAction, PriorityYieldOp, ReplacementAutoChoice};
use engine::types::events::GameEvent;
use engine::types::game_state::{
    AutoMayChoice, AutoPassRequest, LoopDetectionMode, MayTriggerAutoChoiceScope, StackEntryKind,
    WaitingFor, YieldScope,
};
use engine::types::identifiers::ObjectId;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

use crate::support::shared_card_db;

#[derive(Clone, Copy, Debug)]
enum Drive {
    Pass,
    UntilTurnBoundary,
    UntilStackEmpty,
    /// Both players pass until the turn boundary, and P1 yields to each of P0's triggers as
    /// it reaches the top of the stack, so one action drives many cycles.
    FullYields,
}

/// How P0 answers Leonin Snarecaster's "may": at every prompt, or once, remembered.
#[derive(Clone, Copy, Debug)]
enum May {
    Prompted { accept: bool },
    Remembered { accept: bool },
}

const EVERY_DRIVE: [Drive; 3] = [
    Drive::Pass,
    Drive::UntilTurnBoundary,
    Drive::UntilStackEmpty,
];

#[derive(Clone, Copy, Debug, Default)]
struct Board {
    /// Brain Maggot on P0's battlefield and a Lightning Bolt in P1's hand.
    maggot: bool,
    /// Animate Dead targets Grizzly Bears instead of Worldgorger Dragon.
    bears: bool,
    /// P1 has a Mountain on the battlefield and a Lightning Bolt in hand.
    optional: bool,
    /// Leonin Snarecaster on P0's battlefield.
    snarecaster: Option<May>,
    /// Gibbering Fiend on P0's battlefield and Battletide Alchemist on P1's, whose
    /// prevention P1 remembers at its first prompt.
    fiend: bool,
    /// Banisher Priest on P0's battlefield and two Grizzly Bears on P1's.
    banisher: bool,
    /// P0 has a Mountain on the battlefield and a Lightning Bolt in hand.
    p0_out: bool,
}

#[derive(Debug, PartialEq)]
enum Terminal {
    GameOver(Option<PlayerId>),
    SettledOnEmptyStack,
    BeatsExhausted,
    OtherPrompt(String),
}

struct Run {
    terminal: Terminal,
    runner: GameRunner,
    target: ObjectId,
    /// Times the reanimated card entered the battlefield, at the end and at half time.
    entries: usize,
    entries_at_half: usize,
    /// The most entries one action produced.
    max_entries_per_action: usize,
    reveals: usize,
    reveals_at_half: usize,
    max_ring: usize,
    /// Times each prompt other than priority was answered.
    prompts: BTreeMap<&'static str, usize>,
    /// Priority beats at which P0's Mountain stood untapped on the battlefield.
    out_beats: usize,
}

impl Run {
    fn prompted(&self, prompt: &str) -> usize {
        self.prompts.get(prompt).copied().unwrap_or(0)
    }
}

fn run(board: Board, mode: LoopDetectionMode, drive: Drive, beats: usize) -> Option<Run> {
    let db = shared_card_db()?;
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let target_name = if board.bears {
        "Grizzly Bears"
    } else {
        "Worldgorger Dragon"
    };
    let target = scenario.add_real_card(P0, target_name, Zone::Graveyard, db);
    let animate = scenario.add_real_card(P0, "Animate Dead", Zone::Hand, db);
    let maggot = board.maggot.then(|| {
        let maggot = scenario.add_real_card(P0, "Brain Maggot", Zone::Battlefield, db);
        scenario.add_real_card(P1, "Lightning Bolt", Zone::Hand, db);
        maggot
    });
    if board.optional {
        scenario.add_real_card(P1, "Mountain", Zone::Battlefield, db);
        scenario.add_real_card(P1, "Lightning Bolt", Zone::Hand, db);
    }
    if board.snarecaster.is_some() {
        scenario.add_real_card(P0, "Leonin Snarecaster", Zone::Battlefield, db);
    }
    if board.fiend {
        scenario.add_real_card(P0, "Gibbering Fiend", Zone::Battlefield, db);
        scenario.add_real_card(P1, "Battletide Alchemist", Zone::Battlefield, db);
    }
    if board.banisher {
        scenario.add_real_card(P0, "Banisher Priest", Zone::Battlefield, db);
        for _ in 0..2 {
            scenario.add_real_card(P1, "Grizzly Bears", Zone::Battlefield, db);
        }
    }
    let p0_mountain = board.p0_out.then(|| {
        scenario.add_real_card(P0, "Lightning Bolt", Zone::Hand, db);
        scenario.add_real_card(P0, "Mountain", Zone::Battlefield, db)
    });
    for seat in [P0, P1] {
        for _ in 0..10 {
            scenario.add_real_card(seat, "Swamp", Zone::Library, db);
        }
    }
    scenario.with_mana_pool(
        P0,
        (0..2)
            .map(|_| ManaUnit::new(ManaType::Black, ObjectId(0), false, vec![]))
            .collect(),
    );
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    drop(runner.cast(animate).target_object(target).commit());

    let (mut entries, mut entries_at_half, mut reveals, mut reveals_at_half, mut max_ring) =
        (0, 0, 0, 0, 0);
    let (mut max_entries_per_action, mut out_beats, mut remembered) = (0, 0, false);
    let mut prompts = BTreeMap::new();
    let mut terminal = Terminal::BeatsExhausted;
    for beat in 0..beats {
        if beat == beats / 2 {
            (entries_at_half, reveals_at_half) = (entries, reveals);
        }
        let state = runner.state();
        let mut yield_first = None;
        max_ring = max_ring.max(state.loop_detect_ring.len());
        if !matches!(state.waiting_for, WaitingFor::Priority { .. }) {
            *prompts.entry(state.waiting_for.variant_name()).or_insert(0) += 1;
        }
        let action = match &state.waiting_for {
            WaitingFor::GameOver { winner } => {
                terminal = Terminal::GameOver(*winner);
                break;
            }
            WaitingFor::Priority { .. } if state.stack.is_empty() => {
                terminal = Terminal::SettledOnEmptyStack;
                break;
            }
            WaitingFor::Priority { player } => {
                if p0_mountain.is_some_and(|mountain| {
                    let mountain = &state.objects[&mountain];
                    mountain.zone == Zone::Battlefield && !mountain.tapped
                }) {
                    out_beats += 1;
                }
                match drive {
                    Drive::Pass => GameAction::PassPriority,
                    Drive::UntilTurnBoundary => until_turn_boundary(),
                    Drive::UntilStackEmpty => GameAction::SetAutoPass {
                        mode: AutoPassRequest::UntilStackEmpty,
                    },
                    Drive::FullYields => {
                        yield_first = state.stack.back().and_then(|top| match &top.kind {
                            StackEntryKind::TriggeredAbility { source_id, .. }
                                if *player == P1
                                    && top.controller == P0
                                    && !state.is_priority_yielded(P1, top) =>
                            {
                                Some(*source_id)
                            }
                            _ => None,
                        });
                        until_turn_boundary()
                    }
                }
            }
            // Animate Dead's trigger goes on the stack first, so every other trigger of the
            // cycle resolves before the Dragon returns; Brain Maggot's goes last, so it
            // resolves first.
            WaitingFor::OrderTriggers { triggers, .. } => {
                let mut order: Vec<usize> = (0..triggers.len()).collect();
                order.sort_by_key(|&index| {
                    let source = triggers[index].source_id;
                    (source != animate, Some(source) == maggot)
                });
                GameAction::OrderTriggers { order }
            }
            WaitingFor::RevealChoice { cards, .. } => {
                reveals += 1;
                GameAction::SelectCards {
                    cards: cards.clone(),
                }
            }
            WaitingFor::OptionalEffectChoice {
                may_trigger_key,
                same_card_may_trigger_choice_available,
                ..
            } => match board.snarecaster {
                Some(May::Remembered { accept }) if !remembered && may_trigger_key.is_some() => {
                    remembered = true;
                    GameAction::DecideOptionalEffectAndRemember {
                        choice: if accept {
                            AutoMayChoice::Accept
                        } else {
                            AutoMayChoice::Decline
                        },
                        scope: if *same_card_may_trigger_choice_available {
                            MayTriggerAutoChoiceScope::SameCard
                        } else {
                            MayTriggerAutoChoiceScope::ExactInstance
                        },
                    }
                }
                Some(May::Prompted { accept } | May::Remembered { accept }) => {
                    GameAction::DecideOptionalEffect { accept }
                }
                None => GameAction::DecideOptionalEffect { accept: false },
            },
            WaitingFor::ReplacementChoice { .. } if board.fiend && !remembered => {
                remembered = true;
                GameAction::ChooseReplacementAndRemember {
                    choice: ReplacementAutoChoice::Optional { index: 0 },
                }
            }
            WaitingFor::ReplacementChoice { .. } => GameAction::ChooseReplacement { index: 0 },
            WaitingFor::TriggerTargetSelection { .. } => engine::ai_support::legal_actions(state)
                .into_iter()
                .next()
                .expect("Banisher Priest has a creature to target"),
            other => {
                terminal = Terminal::OtherPrompt(format!("{other:?}"));
                break;
            }
        };
        if let Some(source_id) = yield_first {
            runner
                .act(GameAction::SetPriorityYield {
                    op: PriorityYieldOp::Add {
                        source_id,
                        scope: YieldScope::AllCopies,
                    },
                })
                .expect("P1 may yield to a trigger on the stack");
        }
        let result = runner.act(action).expect("every driven action is legal");
        let entered = result
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    GameEvent::ZoneChanged { object_id, to: Zone::Battlefield, .. }
                        if *object_id == target
                )
            })
            .count();
        entries += entered;
        max_entries_per_action = max_entries_per_action.max(entered);
    }
    Some(Run {
        terminal,
        runner,
        target,
        entries,
        entries_at_half,
        max_entries_per_action,
        reveals,
        reveals_at_half,
        max_ring,
        prompts,
        out_beats,
    })
}

fn until_turn_boundary() -> GameAction {
    GameAction::SetAutoPass {
        mode: AutoPassRequest::UntilTurnBoundary {
            until: Default::default(),
        },
    }
}

const PLAIN: Board = Board {
    maggot: false,
    bears: false,
    optional: false,
    snarecaster: None,
    fiend: false,
    banisher: false,
    p0_out: false,
};

const ANCHOR: Board = Board {
    maggot: true,
    ..PLAIN
};

/// Worldgorger Dragon returned by Animate Dead repeats with no player choice, so under
/// Interactive loop detection the game is a draw on every drive.
#[test]
fn worldgorger_animate_dead_is_a_draw_on_every_drive() {
    for drive in EVERY_DRIVE {
        let Some(run) = run(PLAIN, LoopDetectionMode::Interactive, drive, 300) else {
            return;
        };
        assert_eq!(run.terminal, Terminal::GameOver(None), "{drive:?}");
        assert!(
            run.entries >= 2,
            "{drive:?}: reach: the Dragon entered at least twice, got {}",
            run.entries
        );
    }
}

/// Brain Maggot's forced reveal of P1's only nonland card repeats each cycle with one legal
/// option, but the saved order of Brain Maggot's and Animate Dead's triggers applies every
/// cycle and is counted as a decision (CR 603.3b), so the loop is not declared.
#[test]
fn worldgorger_with_a_forced_reveal_under_a_saved_order_is_not_declared() {
    for drive in EVERY_DRIVE {
        let Some(run) = run(ANCHOR, LoopDetectionMode::Interactive, drive, 120) else {
            return;
        };
        assert_eq!(run.terminal, Terminal::BeatsExhausted, "{drive:?}");
        assert!(
            run.reveals >= 2,
            "{drive:?}: reach: reveals were answered across cycles, got {}",
            run.reveals
        );
        assert_eq!(
            run.prompted("OrderTriggers"),
            1,
            "{drive:?}: reach: the trigger order was asked once and then saved"
        );
        assert!(
            run.entries >= 3,
            "{drive:?}: reach: the saved order applied over several cycles, got {}",
            run.entries
        );
    }
}

/// Loop detection Off keeps the game running on every drive.
#[test]
fn worldgorger_runs_on_with_loop_detection_off() {
    for (board, beats) in [(PLAIN, 300), (ANCHOR, 400)] {
        for drive in EVERY_DRIVE {
            let Some(run) = run(board, LoopDetectionMode::Off, drive, beats) else {
                return;
            };
            assert_eq!(run.terminal, Terminal::BeatsExhausted, "{drive:?}");
            assert!(
                run.entries > run.entries_at_half,
                "{drive:?}: reach: the Dragon keeps entering ({} at half, {} at the end)",
                run.entries_at_half,
                run.entries
            );
            if board.maggot {
                assert!(
                    run.reveals > run.reveals_at_half,
                    "{drive:?}: reach: the reveals keep coming ({} at half, {} at the end)",
                    run.reveals_at_half,
                    run.reveals
                );
            }
        }
    }
}

/// A position compared against its own sample trivially recurs; a finite reanimation
/// must settle rather than draw.
#[test]
fn a_finite_reanimation_settles_without_a_draw() {
    let board = Board {
        bears: true,
        ..PLAIN
    };
    let Some(run) = run(board, LoopDetectionMode::Interactive, Drive::Pass, 60) else {
        return;
    };
    assert_eq!(run.terminal, Terminal::SettledOnEmptyStack);
    let bears = run
        .runner
        .state()
        .objects
        .get(&run.target)
        .expect("Bears exist");
    assert_eq!(bears.zone, Zone::Battlefield, "reach: the Bears returned");
}

/// CR 104.4b: "Loops that contain an optional action don't result in a draw." P1 can
/// cast Lightning Bolt at any point.
#[test]
fn worldgorger_with_an_optional_action_is_not_a_draw() {
    let board = Board {
        optional: true,
        ..PLAIN
    };
    for drive in [Drive::Pass, Drive::UntilStackEmpty] {
        let Some(run) = run(board, LoopDetectionMode::Interactive, drive, 300) else {
            return;
        };
        assert!(
            !matches!(run.terminal, Terminal::GameOver(_)),
            "{drive:?}: {:?}",
            run.terminal
        );
        assert!(
            run.max_ring >= 2,
            "{drive:?}: reach: the loop accumulated a ring, max {}",
            run.max_ring
        );
    }
}

const MODES: [LoopDetectionMode; 2] = [LoopDetectionMode::Interactive, LoopDetectionMode::Off];

fn snarecaster(may: May) -> Board {
    Board {
        snarecaster: Some(may),
        ..PLAIN
    }
}

/// Asserts the loop ran its beats without being declared while the Dragon kept entering.
fn assert_runs_on(run: &Run, leg: &str) {
    assert_eq!(run.terminal, Terminal::BeatsExhausted, "{leg}");
    assert!(
        run.entries > run.entries_at_half && run.entries_at_half > 0,
        "{leg}: reach: the Dragon keeps entering ({} at half, {} at the end)",
        run.entries_at_half,
        run.entries
    );
}

/// CR 104.4b + CR 603.5: Leonin Snarecaster's "you may tap" is answered every cycle, so the
/// loop holds an optional action and is no draw, whichever way it is answered.
#[test]
fn a_may_answered_every_cycle_is_not_a_draw() {
    for accept in [true, false] {
        for mode in MODES {
            for drive in EVERY_DRIVE {
                let leg = format!("accept={accept} {mode:?} {drive:?}");
                let Some(run) = run(snarecaster(May::Prompted { accept }), mode, drive, 40) else {
                    return;
                };
                assert_runs_on(&run, &leg);
                assert!(
                    run.prompted("OptionalEffectChoice") >= 2,
                    "{leg}: reach: the may was asked across cycles"
                );
            }
        }
    }
}

/// CR 104.4b + CR 603.5: a "may" answered once and remembered is still answered every cycle,
/// so the loop holds an optional action and is no draw.
#[test]
fn a_remembered_may_answer_is_not_a_draw() {
    let decline = snarecaster(May::Remembered { accept: false });
    let accept = snarecaster(May::Remembered { accept: true });
    remembered_may_runs_on(&[
        (decline, LoopDetectionMode::Off, Drive::Pass),
        (accept, LoopDetectionMode::Interactive, Drive::Pass),
        (accept, LoopDetectionMode::Off, Drive::Pass),
    ]);
}

/// The Interactive remembered-decline legs, one drive per test: each replays its period
/// at every beat.
#[test]
fn a_remembered_may_decline_passing_is_not_a_draw() {
    remembered_decline_runs_on(Drive::Pass);
}

#[test]
fn a_remembered_may_decline_until_stack_empty_is_not_a_draw() {
    remembered_decline_runs_on(Drive::UntilStackEmpty);
}

#[test]
fn a_remembered_may_decline_until_turn_boundary_is_not_a_draw() {
    remembered_decline_runs_on(Drive::UntilTurnBoundary);
}

fn remembered_decline_runs_on(drive: Drive) {
    let decline = snarecaster(May::Remembered { accept: false });
    remembered_may_runs_on(&[(decline, LoopDetectionMode::Interactive, drive)]);
}

fn remembered_may_runs_on(legs: &[(Board, LoopDetectionMode, Drive)]) {
    for &(board, mode, drive) in legs {
        let leg = format!("{:?} {mode:?} {drive:?}", board.snarecaster);
        let Some(run) = run(board, mode, drive, 40) else {
            return;
        };
        assert_runs_on(&run, &leg);
        assert_eq!(
            run.prompted("OptionalEffectChoice"),
            1,
            "{leg}: reach: the may was asked once and then remembered"
        );
    }
}

/// CR 104.4b + CR 616.1e: Battletide Alchemist's prevention of Gibbering Fiend's damage,
/// remembered once, is chosen every cycle, so the loop is no draw.
#[test]
fn a_remembered_optional_prevention_is_not_a_draw() {
    let board = Board {
        fiend: true,
        ..PLAIN
    };
    for mode in MODES {
        for drive in EVERY_DRIVE {
            let leg = format!("{mode:?} {drive:?}");
            let Some(run) = run(board, mode, drive, 30) else {
                return;
            };
            assert_runs_on(&run, &leg);
            assert_eq!(
                run.prompted("ReplacementChoice"),
                1,
                "{leg}: reach: the prevention was asked once and then remembered"
            );
        }
    }
}

/// CR 104.4b + CR 601.2c: Banisher Priest's trigger chooses between two creatures each
/// cycle, so the loop is no draw.
#[test]
fn a_target_chosen_every_cycle_is_not_a_draw() {
    let board = Board {
        banisher: true,
        ..PLAIN
    };
    for mode in MODES {
        for drive in [Drive::Pass, Drive::UntilTurnBoundary] {
            let leg = format!("{mode:?} {drive:?}");
            let Some(run) = run(board, mode, drive, 60) else {
                return;
            };
            assert_runs_on(&run, &leg);
            assert!(
                run.prompted("TriggerTargetSelection") >= 2,
                "{leg}: reach: a target was chosen across cycles"
            );
        }
    }
}

/// CR 104.4b + CR 732.5: P0 can cast Lightning Bolt whenever its Mountain is back on the
/// battlefield, so the loop holds an out and is no draw, even judged from a position where
/// the Mountain is exiled.
#[test]
fn an_out_held_within_the_period_is_not_a_draw() {
    let p0_out = Board {
        p0_out: true,
        ..PLAIN
    };
    let p1_out = Board {
        optional: true,
        ..PLAIN
    };
    for (board, modes) in [
        (p0_out, &MODES[..]),
        (p1_out, &[LoopDetectionMode::Off][..]),
    ] {
        for &mode in modes {
            for drive in EVERY_DRIVE {
                let leg = format!("p0_out={} {mode:?} {drive:?}", board.p0_out);
                let Some(run) = run(board, mode, drive, 40) else {
                    return;
                };
                assert_runs_on(&run, &leg);
                if board.p0_out {
                    assert!(
                        run.out_beats >= 1,
                        "{leg}: reach: P0's Mountain stood untapped at some priority"
                    );
                }
            }
        }
    }
}

/// Off, with full yields one action drives many cycles, and the auto-pass backstop declares
/// none of these boards.
#[test]
fn full_yield_drives_run_on_with_loop_detection_off() {
    let fiend = Board {
        fiend: true,
        ..PLAIN
    };
    for (leg, board, beats, remembered) in [
        ("plain", PLAIN, 7, None),
        (
            "remembered decline",
            snarecaster(May::Remembered { accept: false }),
            11,
            Some("OptionalEffectChoice"),
        ),
        (
            "remembered accept",
            snarecaster(May::Remembered { accept: true }),
            11,
            Some("OptionalEffectChoice"),
        ),
        (
            "remembered prevention",
            fiend,
            11,
            Some("ReplacementChoice"),
        ),
    ] {
        let Some(run) = run(board, LoopDetectionMode::Off, Drive::FullYields, beats) else {
            return;
        };
        assert_eq!(run.terminal, Terminal::BeatsExhausted, "{leg}");
        assert!(
            run.max_entries_per_action >= 16,
            "{leg}: reach: one action returned the Dragon many times, got {}",
            run.max_entries_per_action
        );
        if let Some(prompt) = remembered {
            assert_eq!(
                run.prompted(prompt),
                1,
                "{leg}: reach: one remembered answer"
            );
        }
    }
}

/// Generous for a debug build; an action that never returns runs until killed.
const TIMEOUT_SECS: u64 = 180;

/// An auto-pass drive that reaches its iteration cap hands control back, so under
/// Interactive the action returns even when the loop it ran is not declared; the same drive
/// over a mandatory loop still draws.
#[test]
fn a_capped_auto_pass_action_returns_under_interactive() {
    let Some((terminal, _, _)) = full_yield_drive_returns(PLAIN) else {
        return;
    };
    assert_eq!(
        terminal,
        Terminal::GameOver(None),
        "control: the mandatory loop draws"
    );
    capped_drive_runs_on(snarecaster(May::Remembered { accept: true }));
}

#[test]
fn a_capped_auto_pass_action_returns_after_a_remembered_decline() {
    capped_drive_runs_on(snarecaster(May::Remembered { accept: false }));
}

fn capped_drive_runs_on(board: Board) {
    let leg = format!("{:?}", board.snarecaster);
    let Some((terminal, max_entries_per_action, prompts)) = full_yield_drive_returns(board) else {
        return;
    };
    assert_eq!(terminal, Terminal::BeatsExhausted, "{leg}");
    assert!(
        max_entries_per_action >= 16,
        "{leg}: reach: one action reached the iteration cap, got {max_entries_per_action}"
    );
    assert_eq!(prompts, 1, "{leg}: reach: one remembered answer");
}

/// Runs an Interactive full-yield drive on its own thread so an action that never returns
/// fails the test instead of hanging it.
fn full_yield_drive_returns(board: Board) -> Option<(Terminal, usize, usize)> {
    let (tx, rx) = mpsc::channel();
    let _drive = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let leg =
                run(board, LoopDetectionMode::Interactive, Drive::FullYields, 12).map(|run| {
                    let prompts = run.prompted("OptionalEffectChoice");
                    (run.terminal, run.max_entries_per_action, prompts)
                });
            let _ = tx.send(leg);
        })
        .expect("spawn the drive");
    rx.recv_timeout(Duration::from_secs(TIMEOUT_SECS))
        .unwrap_or_else(|err| panic!("an Interactive full-yield action never returned ({err:?})"))
}

/// CR 117.4 + CR 732.2a: a period confirmed on a frame where its holder stands on an auto-pass
/// is replayed one recorded pass at a time, so each triggered ability the replay resolves takes
/// a pass from both players, each its own action.
#[test]
fn a_standing_auto_pass_does_not_pass_inside_a_confirm_replay() {
    let before = play_trace_counters();
    let Some(run) = run(
        snarecaster(May::Remembered { accept: false }),
        LoopDetectionMode::Interactive,
        Drive::UntilTurnBoundary,
        12,
    ) else {
        return;
    };
    let counted = play_trace_counters().since(before);
    assert_eq!(run.terminal, Terminal::BeatsExhausted);
    assert!(
        counted.confirm_drives > 0 && counted.resolution_hooks_in_probe > 0,
        "reach: a replay resolved triggered abilities ({counted:?})"
    );
    assert!(
        2 * counted.resolution_hooks_in_probe <= counted.probe_entries,
        "a replayed action passed for its holder as well ({counted:?})"
    );
}
