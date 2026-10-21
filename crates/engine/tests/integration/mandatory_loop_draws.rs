//! CR 104.4b + CR 732.4: a loop of mandatory actions is a draw, whether the players pass
//! through it or take the auto-pass shortcut over it (CR 732.1), and even when each cycle
//! only re-mints identities, discloses a card, or asks a choice with one legal option.

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::actions::GameAction;
use engine::types::events::GameEvent;
use engine::types::game_state::{AutoPassRequest, LoopDetectionMode, WaitingFor};
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
}

const EVERY_DRIVE: [Drive; 3] = [
    Drive::Pass,
    Drive::UntilTurnBoundary,
    Drive::UntilStackEmpty,
];

#[derive(Clone, Copy, Default)]
struct Board {
    /// Brain Maggot on P0's battlefield and a Lightning Bolt in P1's hand.
    maggot: bool,
    /// Animate Dead targets Grizzly Bears instead of Worldgorger Dragon.
    bears: bool,
    /// P1 has a Mountain on the battlefield and a Lightning Bolt in hand.
    optional: bool,
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
    reveals: usize,
    reveals_at_half: usize,
    max_ring: usize,
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
    let mut terminal = Terminal::BeatsExhausted;
    for beat in 0..beats {
        if beat == beats / 2 {
            (entries_at_half, reveals_at_half) = (entries, reveals);
        }
        let state = runner.state();
        max_ring = max_ring.max(state.loop_detect_ring.len());
        let action = match &state.waiting_for {
            WaitingFor::GameOver { winner } => {
                terminal = Terminal::GameOver(*winner);
                break;
            }
            WaitingFor::Priority { .. } if state.stack.is_empty() => {
                terminal = Terminal::SettledOnEmptyStack;
                break;
            }
            WaitingFor::Priority { .. } => match drive {
                Drive::Pass => GameAction::PassPriority,
                Drive::UntilTurnBoundary => GameAction::SetAutoPass {
                    mode: AutoPassRequest::UntilTurnBoundary {
                        until: Default::default(),
                    },
                },
                Drive::UntilStackEmpty => GameAction::SetAutoPass {
                    mode: AutoPassRequest::UntilStackEmpty,
                },
            },
            // Brain Maggot's trigger goes on the stack last, so it resolves first.
            WaitingFor::OrderTriggers { triggers, .. } => {
                let mut order: Vec<usize> = (0..triggers.len()).collect();
                order.sort_by_key(|&index| Some(triggers[index].source_id) == maggot);
                GameAction::OrderTriggers { order }
            }
            WaitingFor::RevealChoice { cards, .. } => {
                reveals += 1;
                GameAction::SelectCards {
                    cards: cards.clone(),
                }
            }
            other => {
                terminal = Terminal::OtherPrompt(format!("{other:?}"));
                break;
            }
        };
        let result = runner.act(action).expect("every driven action is legal");
        entries += result
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
    }
    Some(Run {
        terminal,
        runner,
        target,
        entries,
        entries_at_half,
        reveals,
        reveals_at_half,
        max_ring,
    })
}

const PLAIN: Board = Board {
    maggot: false,
    bears: false,
    optional: false,
};

const ANCHOR: Board = Board {
    maggot: true,
    bears: false,
    optional: false,
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

/// Brain Maggot's forced reveal of P1's only nonland card, and that card's return when
/// the Maggot leaves (CR 610.3), repeat each cycle with one legal option, so the loop is
/// still mandatory and still a draw.
#[test]
fn worldgorger_with_a_forced_reveal_is_a_draw_on_every_drive() {
    for drive in EVERY_DRIVE {
        let Some(run) = run(ANCHOR, LoopDetectionMode::Interactive, drive, 400) else {
            return;
        };
        assert_eq!(run.terminal, Terminal::GameOver(None), "{drive:?}");
        assert!(
            run.reveals >= 1,
            "{drive:?}: reach: a reveal was answered before the draw"
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
