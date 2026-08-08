//! PR-7 — live preserved-`Generic` counter-growth loop detection (Path C).
//!
//! Companion to `loop_shortcut.rs`'s B5 revocable-∞ tests. Covers the live
//! `interactive_loop_bridge` Path-C arm for a self-refilling OPTIONAL cascade that
//! grows a `Generic` charge counter each cycle (CR 122.1) — the axis
//! `loop_states_cover_modulo_counter_growth` was built for. Because the growing charge
//! is a PRESERVED counter, the constant-depth `loop_states_equal_modulo_resources`
//! disjunct FAILS on this fixture, so the Path-C mark can only land via the new
//! counter-growth disjunct: reverting that disjunct makes `drive_until_marked` time out
//! (the revert-failing assertion).
//!
//! The live proliferate loop (Pentad Prism cast + Kilo/Freed/Relic) is NOT sampled by
//! construction — a `ProliferateChoice` beat every cycle hits the sampler's ring-CLEAR
//! arm (see `loop_shortcut.rs` docs). That acceptance path is covered OFFLINE by
//! `drive_offline_pentad_prism` in `corpus_tests.rs`. This file uses the sampler-visible
//! shape: a self-refilling trigger cascade whose per-cycle charge-put resolves with no
//! prompt.

use engine::analysis::resource::{CounterClass, ResourceAxis};
use engine::game::scenario::{GameRunner, GameScenario};
use engine::types::ability::AbilityKind;
use engine::types::actions::GameAction;
use engine::types::counter::CounterType;
use engine::types::events::GameEvent;
use engine::types::game_state::{GameState, LoopDetectionMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaColor;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const P0: PlayerId = PlayerId(0);

/// A SINGLE self-refilling trigger that both grows a `Generic` charge counter and
/// re-gains life in ONE resolution. The trailing "You gain 1 life." re-triggers the
/// same ability (like `SELF_LIFE_ENGINE`), so the stack stays NON-SHRINKING across the
/// resolution — the shape the live loop-detect sampler records. A separate leaf
/// charge-put trigger would shrink the stack on resolution and hit the sampler's
/// ring-CLEAR arm, so the counter-put must ride the self-refilling resolution itself.
const CHARGE_LIFE_ENGINE: &str =
    "Whenever you gain life, put a charge counter on this creature. You gain 1 life.";
const KICKOFF: &str = "You gain 1 life.";

fn charge_of(runner: &GameRunner, id: ObjectId) -> u32 {
    runner
        .state()
        .objects
        .get(&id)
        .and_then(|o| o.counters.get(&CounterType::Generic("charge".to_string())))
        .copied()
        .unwrap_or(0)
}

/// 2-player OPTIONAL beneficial cascade controlled by P0 that grows a `Generic` charge
/// counter each cycle. One creature carries `CHARGE_LIFE_ENGINE` (a single self-refilling
/// trigger that puts a charge counter AND re-gains life in one resolution — the
/// sampler-visible non-shrinking shape). P1 holds a castable Bolt off an untapped Mountain
/// (a meaningful priority action) so the loop is OPTIONAL (`mandatory == false`) ⇒ Path C,
/// not the Path-B draw. Nobody loses life ⇒ Path A finds no faller. Returns runner +
/// (kickoff sorcery id, engine creature id — the charge-counter bearer).
fn setup_2p_optional_charge_growth(mode: LoopDetectionMode) -> (GameRunner, ObjectId, ObjectId) {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(PlayerId(1), 20);
    let engine_creature = scenario
        .add_creature_from_oracle(P0, "Test Charge Life Engine", 2, 2, CHARGE_LIFE_ENGINE)
        .id();
    scenario.add_basic_land(PlayerId(1), ManaColor::Red);
    scenario.add_bolt_to_hand(PlayerId(1));
    let kickoff = scenario
        .add_spell_to_hand_from_oracle(P0, "Test Lifegain Kickoff", false, KICKOFF)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = mode;
    (runner, kickoff, engine_creature)
}

/// Drive `PassPriority`/`OrderTriggers` beats, collecting every emitted event, until
/// `controller`'s revocable-∞ capability is marked (Path C is a SILENT mark — it never
/// changes `waiting_for`, so callers poll `unbounded_resources` directly). Returns the
/// accumulated events and whether the mark landed.
fn drive_until_marked_collecting(
    runner: &mut GameRunner,
    controller: PlayerId,
    cap: usize,
) -> (Vec<GameEvent>, bool) {
    let mut events = Vec::new();
    let marked = |s: &GameState| s.unbounded_resources.contains_key(&controller);
    for _ in 0..cap {
        if marked(runner.state()) {
            return (events, true);
        }
        let action = match runner.state().waiting_for.clone() {
            WaitingFor::Priority { .. } => GameAction::PassPriority,
            WaitingFor::OrderTriggers { triggers, .. } => GameAction::OrderTriggers {
                order: (0..triggers.len()).collect(),
            },
            _ => return (events, marked(runner.state())),
        };
        match runner.act(action) {
            Ok(r) => events.extend(r.events),
            Err(_) => return (events, marked(runner.state())),
        }
    }
    (events, marked(runner.state()))
}

/// PR-7 #6 (live Path-C, revert-failing): an OPTIONAL self-refilling cascade that grows a
/// `Generic` charge counter each cycle is marked as a revocable-∞ capability naming the
/// charge counter axis — and NEVER produces a `GameOver` (CR 104.4b: an optional loop is
/// not a draw; Path C is a silent mark).
///
/// REVERT-FAILING assertion (`marked`): the growing charge is a PRESERVED counter, so the
/// constant-depth `loop_states_equal_modulo_resources` Path-C disjunct FAILS on this
/// fixture (contrast `b5_optional_beneficial_marks_revocable_unbounded`, whose pure-life
/// loop marks via that equality disjunct). The mark can land ONLY via the new
/// `loop_states_cover_modulo_counter_growth` disjunct; reverting it makes the recurrence
/// gate fail and `drive_until_marked_collecting` returns `false`.
#[test]
fn live_optional_charge_growth_marks_counter_advantage_no_gameover() {
    let (mut runner, kickoff, rider) =
        setup_2p_optional_charge_growth(LoopDetectionMode::Interactive);
    let _ = runner.cast(kickoff).resolve();

    let (events, marked) = drive_until_marked_collecting(&mut runner, P0, 500);
    assert!(
        marked,
        "the optional charge-growth cascade must reach the Path-C revocable-∞ mark \
         (only reachable via loop_states_cover_modulo_counter_growth — the growing charge \
         breaks the constant-depth equality disjunct)"
    );

    // Non-vacuity reach-guard: the charge counter genuinely grew (≥2 ⇒ the CHARGE_RIDER
    // trigger parsed AND the loop ran multiple cycles), so the mark is not a degenerate
    // empty capability.
    let charge = charge_of(&runner, rider);
    assert!(
        charge >= 2,
        "reach-guard: the rider must have accrued ≥2 charge counters (loop actually ran); got {charge}"
    );

    // The marked capability names the charge counter axis (CounterClass::Other = a Generic
    // charge counter). This axis appears ONLY because the counter-growth disjunct fired.
    let axes = runner
        .state()
        .unbounded_resources
        .get(&P0)
        .cloned()
        .unwrap_or_default();
    assert!(
        axes.iter()
            .any(|a| matches!(a, ResourceAxis::Counter(CounterClass::Other, _))),
        "P0's revocable-∞ capability must include the Generic charge counter axis; got {axes:?}"
    );

    // Revocability bound: Path C is a silent mark — the game continues at live priority,
    // never a GameOver (neither waiting_for nor an emitted event).
    assert!(
        matches!(runner.state().waiting_for, WaitingFor::Priority { .. }),
        "an optional beneficial loop must fall through to live priority, not GameOver; got {:?}",
        runner.state().waiting_for
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, GameEvent::GameOver { .. })),
        "no GameOver event may be emitted for a revocable optional beneficial loop"
    );
    assert!(
        runner.state().players.iter().all(|p| !p.is_eliminated),
        "a no-loss beneficial loop eliminates no player"
    );
}

/// PR-7 #7 (#4603 OFF gate): under `LoopDetectionMode::Off` the SAME charge-growth
/// fixture never marks a revocable capability — the detector is fully dormant (the
/// sampler never records under Off), restoring exact pre-feature behavior. Paired with
/// #6 (Interactive marks), this proves the user-controllable toggle gates the feature.
#[test]
fn live_charge_growth_off_never_marks() {
    let (mut runner, kickoff, rider) = setup_2p_optional_charge_growth(LoopDetectionMode::Off);
    let _ = runner.cast(kickoff).resolve();

    // Drive a bounded number of beats; Off must never mark, and (being a beneficial
    // no-loss loop) must never reach a GameOver.
    let (events, marked) = drive_until_marked_collecting(&mut runner, P0, 500);
    assert!(
        !marked,
        "Off must never mark a revocable-∞ capability (Interactive-only, #4603)"
    );
    assert!(
        runner.state().unbounded_resources.is_empty(),
        "Off must leave unbounded_resources empty; got {:?}",
        runner.state().unbounded_resources
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, GameEvent::GameOver { .. })),
        "Off must not synthesize a GameOver for this beneficial loop"
    );

    // Reach-guard: the loop still physically ran under Off (charge grew) — so "never
    // marks" is attributable to the OFF gate, not to the loop failing to execute.
    let charge = charge_of(&runner, rider);
    assert!(
        charge >= 2,
        "reach-guard: the cascade must still run under Off (charge grew); got {charge}"
    );
}

/// A FREE, voluntarily-repeatable activation that creates a token AND grows a `+1/+1` counter.
///
/// BOTH CLAUSES ARE LOAD-BEARING, and the token one is not decoration. `apply_action`'s
/// `ActivateAbility` arm bootstraps `last_loop_action_sequence` ONLY when the activated ability
/// `creates_token` (or when a period for the same controller is already open); any other
/// activation CLEARS it. Mana activations arm it through the separate
/// `record_mana_loop_action_step` path. So a counter-only activation can never open a period, and
/// the CR 732.2a offer — which requires a non-empty sequence — is unreachable without a carrier.
/// The `+1/+1` growth therefore rides a token-creating activation, which is also a realistic
/// shape: a token engine whose creature grows as it works.
const PLUS1_TOKEN_ENGINE: &str =
    "{0}: Create a 1/1 colorless Servo artifact creature token. Put a +1/+1 counter on this creature.";

fn plus1_of(runner: &GameRunner, id: ObjectId) -> u32 {
    runner
        .state()
        .objects
        .get(&id)
        .and_then(|o| o.counters.get(&CounterType::Plus1Plus1))
        .copied()
        .unwrap_or(0)
}

/// THE `∞` DISPLAY CHANNEL REGISTERS A `+1/+1` GROWTH (CR 122.1 + CR 732.2a).
///
/// WHY THIS FIXTURE HAD TO BE BUILT rather than reused: a census of every test touching
/// `unbounded_counter_targets` found that all of them grow `charge` — a `Generic` counter, which
/// registers IDENTICALLY under the old cover partition and the current beneficial one. So the
/// whole suite passed byte-for-byte with or without the display/collapse consolidation, and no
/// existing test could distinguish the change from its absence.
///
/// REACHABILITY, measured rather than assumed — a `+1/+1` loop is detected by a DIFFERENT
/// disjunct than a charge loop, and it matters: `CounterType::Plus1Plus1
/// ::is_monotone_loop_resource()` is `true`, so `project_out_resources` strips it and the frames
/// read EQUAL under `loop_states_equal_modulo_resources`. (A charge loop cannot do that —
/// `Generic` is preserved, which is exactly why `loop_states_cover_modulo_counter_growth` exists.)
/// So this loop arrives through the base equality disjunct, is offered, and its `+1/+1` growth is
/// materialized at the boundary by `counter_is_beneficial_materializable` — while the DISPLAY
/// registration, when it was partitioned by the `Generic`-only ω-cover rule, saw nothing. That
/// gap is the defect: a real loop whose collapse lands and whose pills never render `∞`.
///
/// THE REVERT-PROBE (the evidence, run and recorded): restore the display registration to the
/// `Generic`-only derivation — i.e. re-point it at a `grown_generic_counter_targets`-shaped
/// filter instead of projecting `growths` — and assertion (3) flips to an EMPTY target set. Every
/// other assertion here holds under that revert, which is what makes (3) the discriminator rather
/// than a bystander.
///
/// DIVISION OF LABOUR, stated so neither half is overread: this fixture is DERIVED state (a
/// scenario-built loop), so it proves the registration covers the `+1/+1` class end-to-end
/// through a real offer and accept. It does NOT carry production-dump provenance; that burden is
/// `kilo_live_offer_from_real_dump`'s, on a real 4p dump.
#[test]
fn plus_one_counter_growth_registers_its_infinity_display_target() {
    use engine::analysis::decision_template::IterationCount;
    use engine::analysis::loop_check::ShortcutResponse;
    use engine::game::derived_views::derive_views;

    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(PlayerId(1), 20);
    let rider = scenario
        .add_creature_from_oracle(P0, "Test Plus One Token Engine", 2, 2, PLUS1_TOKEN_ENGINE)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;

    // THE DRIVING SHAPE — two constraints, both MEASURED by building the fixture that violates
    // them and watching it fail, not inferred from the code:
    //
    // 1. It must be an ACTIVATION, not a trigger cascade. `try_offer_object_growth_shortcut`
    //    requires a non-empty `last_loop_action_sequence` whose every step is
    //    `is_voluntarily_repeatable()` (CR 601.2a / CR 602.2 / CR 605.3a — casting, activating, and
    //    mana abilities are each a voluntary choice at priority; the helper's own annotation names
    //    all three). A trigger cascade drives itself and records no
    //    action sequence, so it reaches only the Path-C silent mark — which registers no backing
    //    set at all. The cascade version of this fixture grew its counters and then sat at
    //    `Priority` with no offer.
    // 2. The activation must CREATE A TOKEN. `apply_action`'s `ActivateAbility` arm opens a period
    //    only for a token-creating ability (or continues one already open for this controller);
    //    every other activation CLEARS the sequence. A `{0}: Put a +1/+1 counter on this creature.`
    //    version therefore also sat at `Priority` — each activation wiped the very sequence the
    //    offer needs. Mana activations arm it by a different path entirely
    //    (`record_mana_loop_action_step`).
    //
    // So the reachable production shape for a `+1/+1` ∞ display registration is a counter growth
    // riding a token-creating or mana-producing carrier. That is a real constraint on the class,
    // worth stating: it is why no such fixture existed to reuse.
    let ability_index = runner
        .state()
        .objects
        .get(&rider)
        .and_then(|o| {
            o.abilities
                .iter()
                .position(|def| def.kind == AbilityKind::Activated)
        })
        .expect("the {0} activated ability parsed onto the rider");

    let mut offered = false;
    let mut activations = 0usize;
    let mut halt = String::from("ran to the iteration cap");
    for _ in 0..40 {
        if matches!(runner.state().waiting_for, WaitingFor::LoopShortcut { .. }) {
            offered = true;
            break;
        }
        match runner.act(GameAction::ActivateAbility {
            source_id: rider,
            ability_index,
        }) {
            Ok(_) => activations += 1,
            Err(e) => {
                halt = format!(
                    "activation #{} refused: {e:?} (waiting_for {:?})",
                    activations + 1,
                    runner.state().waiting_for
                );
                break;
            }
        }
        // Settle the activation off the stack; stop early if the offer surfaces mid-settle.
        for _ in 0..60 {
            match &runner.state().waiting_for {
                WaitingFor::LoopShortcut { .. } => break,
                WaitingFor::Priority { .. } if runner.state().stack.is_empty() => break,
                _ => {}
            }
            if let Err(e) = runner.act(GameAction::PassPriority) {
                halt = format!(
                    "settle after activation #{activations} stalled: {e:?} (waiting_for {:?})",
                    runner.state().waiting_for
                );
                break;
            }
        }
    }
    offered |= matches!(runner.state().waiting_for, WaitingFor::LoopShortcut { .. });

    // (1) REACH-GUARD: the engine really executed and really grew a `+1/+1` counter, so an empty
    // target set below means "the registration missed the class" and not "no loop happened".
    //
    // THRESHOLD IS ONE, deliberately, and not the `>= 2` the charge cascades above use. Those
    // fixtures need the BOARD to iterate because they are witnessing a Path-C mark that only
    // recurrence can produce. This one witnesses an OFFER, and the offer fires as soon as a single
    // period is recorded and the clone-drive confirms it recurs — the real board never iterates
    // twice. Measured: with `>= 2` this guard failed at `got 1 counters after 1 activation(s)`
    // while the offer had already surfaced, i.e. the guard was rejecting a working fixture.
    // The halt reason rides along because a stalled driver and a broken registration otherwise
    // fail identically.
    let grown = plus1_of(&runner, rider);
    assert!(
        grown >= 1,
        "reach-guard: the +1/+1 engine must actually run; got {grown} counters after \
         {activations} activation(s) — {halt}"
    );

    // (2) REACH-GUARD: a real offer surfaced, so the accept below drives production's
    // `materialize_object_growth_shortcut` rather than a grafted stash.
    assert!(
        offered,
        "reach-guard: the +1/+1 growth loop must raise a natural CR 732.2a offer, got {:?}",
        runner.state().waiting_for
    );

    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(1),
            template: None,
        })
        .expect("P0 (proposer) declares the +1/+1 growth shortcut");
    while matches!(
        runner.state().waiting_for,
        WaitingFor::RespondToShortcut { .. }
    ) {
        runner
            .act(GameAction::RespondToShortcut {
                response: ShortcutResponse::Accept,
            })
            .expect("the opponent accepts");
    }

    // (3) THE ASSERTION — the discriminator. The accept registered the `+1/+1` pair as an `∞`
    // DISPLAY target. Under the `Generic`-only registration this set is EMPTY.
    let targets = runner
        .state()
        .unbounded_counter_targets
        .get(&P0)
        .cloned()
        .unwrap_or_default();
    assert!(
        targets.contains(&(rider, CounterType::Plus1Plus1)),
        "(3) the accept must register the +1/+1 pair as an ∞ display target — this is the \
         assertion the Generic-only display partition failed; got {targets:?}"
    );

    // (4) …and it reaches the WIRE as a pill, which is the user-visible half of (3). Asserted
    // separately because (3) could hold while the projection filtered it back out.
    let views = derive_views(runner.state(), None);
    assert!(
        views
            .unbounded_counters
            .get(&rider)
            .is_some_and(|cts| cts.contains(&CounterType::Plus1Plus1)),
        "(4) the +1/+1 ∞ pill must reach the wire, got {:?}",
        views.unbounded_counters
    );
}
