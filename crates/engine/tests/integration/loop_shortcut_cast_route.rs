//! CR 732.2a phase 1 (U-route) acceptance — a board carrying a FUNCTIONING cast-mode trigger
//! routes an accepted object-growth collapse to the concrete `DriveSequence` replay instead of the
//! batched `Tokens`/`Counters`/`Life` items.
//!
//! WHY THE ROUTE EXISTS. The batched arm never casts anything — the cast event belongs to the
//! ELIDED period, not to the collapse — so a batched accept re-performs a cast-sourced per-cycle
//! effect ZERO times where live play performs it once per cycle (MEASURED on the composed
//! cast-sourced mill board: `[0,0,0,0]` at N=5 batched, `[0,N,N,N]` forced to the replay). CR
//! 601.2i is the cast event; CR 113.6 / CR 113.6b are the zone gate that keeps a library-resident
//! cast trigger from counting.
//!
//! NO ITERATION BUDGET IN THIS PHASE, AND WHY THERE ARE NO CAP ROWS. Phase 1 ships the ROUTE
//! alone: the published collapse ceiling stays the accepted count on both routes, exactly as it
//! was before this change. An earlier draft clamped the replay route to a lower ceiling and was
//! withdrawn — the producer still published `MAX_SHORTCUT_CYCLES`, so the collapse prompt no
//! longer offered the ceiling the accepted offer had published (CR 732.2c), which the R-seam row
//! `kilo_live_offer_from_real_dump::kilo_reported_capture_offer_states_the_full_ceiling_it_publishes`
//! measured RED on a real playtest capture. Letting a lowered ceiling coexist with `is_bounded()`
//! needs a schema split, which is scheduled as its own later phase.
//!
//! So the rows this file once carried for cap behaviour — "the batched arm is never clamped", "an
//! under-cap accept publishes its own count", "an above-cap declaration lowers the ceiling without
//! falling back to batched" — are deleted rather than ignored. With no cap there is no
//! cap-triggered fallback path and no clamped ceiling at all: `materialize_object_growth_shortcut`
//! does not even take `n`, and its caller folds the accepted count with a plain `.min(n)` on both
//! routes. The hazard those rows guarded is **structurally absent, not merely untested**. It
//! returns as a live hazard only when the schema-split phase reintroduces a lowered ceiling, and
//! those rows belong to that phase's plan.
//!
//! BASE BOARD for every row: the REAL 4-player `sprout_witherbloom_realistic_lands_4p` dump,
//! loaded through the production restore chokepoint and driven through the public
//! `GameRunner`/`apply()` boundary by `sprout_inalla_realistic_offer`'s own two helpers. The
//! grafted and ungrafted arms are **one object apart**, which is what makes the pairs below
//! discriminating rather than merely green.
//!
//! WHAT THIS FILE CANNOT NAME, DELIBERATELY. `LoopCollapseRoute` is private to `game/engine.rs`
//! by charter and MUST STAY private — an integration test is an external crate and cannot name
//! it. The route is observed through the `PersistentAxisMaterialization` discriminant
//! (`ExpectedRoute` below is the test-crate mirror, not a copy of the decision). Widening the
//! production enum's visibility "just for the tests" is forbidden.
//!
//! THE ROUTE IS PAYLOAD-GUARDED. The whole replay disjunction sits under `!batched.is_empty()` —
//! the replay route is only ever the better version of a registration the BATCHED arm would have
//! made, never a registration out of nothing. Without that guard the seam's two arms are
//! asymmetric (the batched arm registers conditionally, the replay arm unconditionally), so a mana
//! engine — which batches NOTHING — gets dragged onto the replay by any cast trigger on the board.
//! HISTORICAL, and true when written: in the first cut that drag ALSO had its own `Mana(_)` ∞ mark
//! collapsed at the CR 500.5 boundary — a MEASURED regression, and the reason this guard exists.
//! That consequence is now closed PER AXIS by `ResourceAxis::unbounded_mark_kind`, not by this
//! guard; what the guard still buys is the uncapped cubic replay cost plus a spurious CR 500.5
//! collapse prompt for a loop with nothing to collapse.
//! `mana_engine_with_cast_trigger_registers_nothing` below is its pin.
//!
//! REVERT PROBES (RUN at the final candidate tip, output captured to
//! `/home/lgray/vibe-coding/item4-run/wba-p1-logs/` — an asserted revert probe is not a revert
//! probe; see the gate log for the tip SHA and the per-probe output).
//! (1) Delete the `cast_sourced` disjunct at the `engine.rs` route seam ⇒ the grafted arms
//! register `Tokens` again ⇒ every row below that asserts `ExpectedRoute::Replay` goes RED.
//! (2) Delete only the `!batched.is_empty()` guard ⇒ the Replay rows stay green and
//! `mana_engine_with_cast_trigger_registers_nothing` goes RED with `[DriveSequence]`.
//! (3) Restore `collapsed_axes: proposal.unbounded.clone()` at the `Replay` arm ⇒ the three
//! mixed-axis rows in `combo_infinite_pile` go RED at all three levels (registration, the direct
//! boundary clear, and the full production round-trip).
//! (4) Classify `LibraryDelta` as `StandingCapability` ⇒ `analysis::resource`'s
//! `unbounded_mark_kind_classifies_every_axis_by_termination_authority` goes RED.
//! (5) Move `ResourceAxis::Mana(_)` out of `derived_views::object_growth_backing`'s `None` arm to
//! `Some(false)` ⇒ the mixed-axis registration row's assertion (d) goes RED.

use engine::analysis::decision_template::IterationCount;
use engine::analysis::loop_check::ShortcutResponse;
use engine::analysis::resource::ResourceAxis;
use engine::game::engine::apply;
use engine::game::functioning_abilities::active_trigger_definitions;
use engine::game::scenario::GameRunner;
use engine::game::zones::create_object;
use engine::types::ability::TriggerDefinition;
use engine::types::actions::GameAction;
use engine::types::game_state::{
    GameState, LoopDetectionMode, PayableResource, PersistentAxisMaterialization, WaitingFor,
};
use engine::types::identifiers::{CardId, ObjectId};
use engine::types::player::PlayerId;
use engine::types::triggers::TriggerMode;
use engine::types::zones::Zone;

use super::loop_shortcut_mana_engine::{
    drive_one_period, mana_ability_index, setup, untap_ability_index,
};
use super::sprout_inalla_realistic_offer::{drive_sprout_cast, load_realistic_dump};
use super::support::shared_card_db;

const P0: PlayerId = PlayerId(0);
/// Sprout Swarm in P0's hand in the realistic 4p dump.
const SPROUT: ObjectId = ObjectId(405);
/// The fodder `drive_sprout_cast` convokes for the {G}. Recorded here because R-mixed's SECOND
/// cast must use a different one — this is a measured fixture fact, not a guess.
const FIRST_CONVOKE_FODDER: ObjectId = ObjectId(406);
/// A second untapped P0 fodder Saproling (406–410, 412 are untapped in the dump).
const SECOND_CONVOKE_FODDER: ObjectId = ObjectId(407);
/// `game::engine::MAX_SHORTCUT_CYCLES`, mirrored because it is `pub(crate)` and this binary is an
/// external crate — the same mirror `fantastic_four_bounded_loop.rs` keeps. It is the LARGEST
/// count `handle_declare_shortcut` accepts (it refuses `Fixed(n)` for `n > MAX_SHORTCUT_CYCLES`
/// and for `n > schema.max_iterations`, and the object-growth mint publishes exactly this), so the
/// large-N arm below runs at the engine's own ceiling rather than at an arbitrary big number.
const MAX_SHORTCUT_CYCLES_MIRROR: u32 = 1_000;

/// Test-crate mirror of the engine's private `LoopCollapseRoute`. It exists because the production
/// enum is private by charter and MUST STAY private — this is the OBSERVABLE, not a copy of the
/// decision. The mapping in [`route_of`] is the only place the proxy is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedRoute {
    /// registers `PersistentAxisMaterialization::DriveSequence { .. }`
    Replay,
    /// registers one of the batched axes (`Tokens` / `Counters` / `Life`)
    Batched,
}

/// Exhaustive, no wildcard: a future persistent axis must be classified here deliberately rather
/// than defaulting into `Batched` — the same obligation the production `match` carries.
fn route_of(m: &PersistentAxisMaterialization) -> ExpectedRoute {
    match m {
        PersistentAxisMaterialization::DriveSequence { .. } => ExpectedRoute::Replay,
        PersistentAxisMaterialization::Tokens(_)
        | PersistentAxisMaterialization::Counters(_)
        | PersistentAxisMaterialization::Life { .. } => ExpectedRoute::Batched,
    }
}

/// The registered discriminant as a short name, so a failure message names what was actually
/// observed instead of dumping a whole `CopiableValues` payload. Exhaustive for the same reason.
fn route_name(m: &PersistentAxisMaterialization) -> &'static str {
    match m {
        PersistentAxisMaterialization::DriveSequence { .. } => "DriveSequence",
        PersistentAxisMaterialization::Tokens(_) => "Tokens",
        PersistentAxisMaterialization::Counters(_) => "Counters",
        PersistentAxisMaterialization::Life { .. } => "Life",
    }
}

/// Everything P0's accepts have registered, in stash order.
fn registered_routes(state: &GameState) -> &[PersistentAxisMaterialization] {
    state
        .pending_unbounded_materialization
        .get(&P0)
        .map_or(&[], Vec::as_slice)
}

/// R-route-assert — the instrument standard. PANICS with the observed discriminants on a silent
/// fall to the batched route, so no row in this file can report a bound (or a fast number) without
/// having asserted its route first. The empty-stash assertion is what stops a vacuous pass on a
/// board that registered nothing at all.
fn assert_route(state: &GameState, expected: ExpectedRoute) {
    let stash = registered_routes(state);
    assert!(
        !stash.is_empty(),
        "R-route-assert: P0's accept registered NOTHING — there is no route to assert, so any \
         bound read after this point would be vacuous"
    );
    let observed: Vec<&'static str> = stash.iter().map(route_name).collect();
    assert!(
        stash.iter().all(|m| route_of(m) == expected),
        "R-route-assert: expected every registered materialization on the {expected:?} route, \
         observed {observed:?}"
    );
}

/// Graft a bare functioning cast-mode trigger onto a NEW P0 battlefield object — the one-object
/// difference between the grafted and ungrafted arms.
///
/// `TriggerMode::SpellCast` with no `valid_card` and no `execute`: the predicate under test keys on
/// `TriggerEventKey::SpellCast(_)` with the payload DISCARDED, so the bare mode is exactly what it
/// must see. Battlefield-resident with empty `trigger_zones`, so CR 113.6's default branch makes it
/// FUNCTION — which is the property the dump's own six library-resident cast triggers lack.
fn graft_cast_trigger(state: &mut GameState, name: &str) -> ObjectId {
    let card_id = CardId(state.next_object_id);
    let host = create_object(state, card_id, P0, name.to_string(), Zone::Battlefield);
    state
        .objects
        .get_mut(&host)
        .expect("the just-created graft host is in `objects`")
        .trigger_definitions
        .push(TriggerDefinition::new(TriggerMode::SpellCast));
    // Positional read-back (`Definitions<T>` exposes no `iter()`): prove the graft actually landed,
    // so a row that later reads `Batched` is a route failure rather than a fixture failure.
    let entries = &state
        .objects
        .get(&host)
        .expect("graft host present")
        .trigger_definitions;
    assert_eq!(
        entries.len(),
        1,
        "the graft host carries exactly one trigger"
    );
    assert_eq!(
        entries
            .get(0)
            .expect("positional read-back of the grafted entry")
            .definition
            .mode,
        TriggerMode::SpellCast,
        "the grafted trigger is cast-mode"
    );
    host
}

/// The realistic board driven to its CR 732.2a offer by one real buyback+convoke recast.
fn offer_state(graft: bool) -> GameState {
    let mut state = load_realistic_dump();
    if graft {
        graft_cast_trigger(&mut state, "Cast Route Probe");
    }
    let outcome = drive_sprout_cast(state);
    let state = outcome.state().clone();
    assert!(
        matches!(state.waiting_for, WaitingFor::LoopShortcut { proposer, .. } if proposer == P0),
        "reach-guard: the live recast must surface P0's CR 732.2a offer{}, got {:?}",
        if graft {
            " EVEN WITH the cast trigger grafted"
        } else {
            ""
        },
        state.waiting_for
    );
    state
}

/// Proposer declares `Fixed(n)`; every living opponent accepts (APNAP).
fn declare_and_accept_all(state: &mut GameState, proposer: PlayerId, n: u32) {
    apply(
        state,
        proposer,
        GameAction::DeclareShortcut {
            count: IterationCount::Fixed(n),
            template: None,
        },
    )
    .expect("the proposer declares the object-growth shortcut");
    while let WaitingFor::RespondToShortcut { player, .. } = state.waiting_for.clone() {
        apply(
            state,
            player,
            GameAction::RespondToShortcut {
                response: ShortcutResponse::Accept,
            },
        )
        .expect("each living opponent accepts");
    }
}

/// Pass priority through the real production path until the CR 500.5 step/phase boundary surfaces
/// a non-`Priority` prompt. Bounded so a wedge fails loudly instead of hanging.
fn drive_to_boundary(state: &mut GameState) {
    let start_phase = state.phase;
    for _ in 0..64 {
        let WaitingFor::Priority { player } = state.waiting_for.clone() else {
            return;
        };
        apply(state, player, GameAction::PassPriority)
            .expect("pass priority toward the next phase boundary");
        if !matches!(state.waiting_for, WaitingFor::Priority { .. }) || state.phase != start_phase {
            return;
        }
    }
    panic!("drive_to_boundary: no CR 500.5 boundary within 64 passes");
}

/// The ceiling the CR 500.5 boundary prompt publishes to the loop's controller.
fn boundary_max(state: &GameState) -> u32 {
    let WaitingFor::PayAmountChoice {
        player,
        resource: PayableResource::LoopCollapse { .. },
        max,
        ..
    } = &state.waiting_for
    else {
        panic!(
            "the CR 500.5 boundary must prompt P0 for the collapse count, got {:?}",
            state.waiting_for
        )
    };
    assert_eq!(*player, P0, "the loop controller is prompted");
    *max
}

// ===========================================================================
// R2 ∧ R2-neg — the route pair. Written as ONE test so the two arms are structurally
// inseparable: R2 alone is satisfiable by a blanket route change, and R2-neg alone by never
// wiring the disjunct.
// ===========================================================================

/// **R2 ∧ R2-neg ∧ R2-typed.** The grafted board routes to the concrete replay; the untouched
/// shipped board, ONE OBJECT AWAY, still routes batched.
///
/// R2-neg is THE discriminator, and the board it runs on is measured NON-TRIVIAL rather than
/// empty: the dump scans 135 active trigger definitions, of which exactly ONE passes the CR 113.6
/// zone gate (an ETB-keyed def) while its SIX `SpellCast`-keyed defs are all library-resident with
/// `trigger_zones` naming only Battlefield or Stack. So this arm tests the ZONE GATE — a real zero
/// with a live same-gate control — not an absence of triggers.
///
/// R2-typed is discharged BY CONSTRUCTION: every assertion here is written in the typed
/// `ExpectedRoute` vocabulary against the typed production route. It cannot be written in the
/// production enum's vocabulary at all, because `LoopCollapseRoute` is private and unnameable from
/// an external test crate.
///
/// **R2-large-N** is the third arm and it is a COUNT-INDEPENDENCE pin, not a performance row: it
/// re-runs the grafted arm at `MAX_SHORTCUT_CYCLES`, the largest count the declare authority
/// accepts. Today this is structural — `materialize_object_growth_shortcut` never receives `n` at
/// all, so no route decision can read it — and the arm exists to keep it that way: any future
/// change that threads a count-dependent fallback into the route seam (the shape the withdrawn
/// iteration budget had) must make this arm choose, rather than silently trading the concrete
/// replay for the batched route at large N. It is also cheap, because the accept only REGISTERS a
/// `DriveSequence`; the cycles are replayed later, at the CR 500.5 boundary this arm never drives
/// to.
#[test]
fn cast_trigger_board_routes_to_replay_untouched_board_stays_batched() {
    // ── R2-neg (the discriminator): untouched shipped board ⇒ batched ──
    let mut ungrafted = offer_state(false);
    declare_and_accept_all(&mut ungrafted, P0, 100);
    assert_route(&ungrafted, ExpectedRoute::Batched);

    // ── R2: one grafted functioning cast trigger ⇒ concrete replay ──
    let mut grafted = offer_state(true);
    declare_and_accept_all(&mut grafted, P0, 100);
    assert_route(&grafted, ExpectedRoute::Replay);

    // ── R2-large-N: same board, the engine's maximum accepted count, same route ──
    let mut grafted_at_ceiling = offer_state(true);
    declare_and_accept_all(&mut grafted_at_ceiling, P0, MAX_SHORTCUT_CYCLES_MIRROR);
    assert_route(&grafted_at_ceiling, ExpectedRoute::Replay);
}

/// **V3 — the NON-BLANKET discriminator.** The per-axis `collapsed_axes` filter at the `Replay`
/// arm keeps every `DeferredAccrual` axis: this already-shipped realistic dump's `DriveSequence`
/// still names the loop's marked axis set EXACTLY, with nothing dropped.
///
/// WHY IT EXISTS AS ITS OWN ROW. The mixed-axis rows in `combo_infinite_pile` assert that a
/// `Mana(_)` axis is EXCLUDED. An over-aggressive implementation that empties every
/// `collapsed_axes` — the trivial "reject `DeferredAccrual`", or "keep only `StandingCapability`"
/// — passes their `Mana` half, and passes the function-level preservation half too. This row is
/// what stops it: it upgrades the sibling above's DISCRIMINANT-only route assertion to an
/// EXACT-SET assertion on the same board.
///
/// REVERT PROBE: invert the filter at `game::engine::materialize_object_growth_shortcut`'s
/// `Replay` arm (keep only `StandingCapability`, or drop `DeferredAccrual`) ⇒ `collapsed_axes`
/// empties ⇒ RED here while the mixed-axis rows stay green.
///
/// REACH-GUARD: the shipped [`assert_route`] instrument on the same arm. Without it an
/// over-aggressive filter could be masked by a ROUTE regression that makes the exact-set assertion
/// unreachable — `assert_route` panics on an empty stash precisely so that cannot happen quietly.
#[test]
fn replay_collapse_names_every_deferred_axis_the_loop_marked() {
    let mut grafted = offer_state(true);
    declare_and_accept_all(&mut grafted, P0, 100);

    // REACH-GUARD: a real replay registration exists to read an exact set off.
    assert_route(&grafted, ExpectedRoute::Replay);

    // The ∞-mark set this accept wrote — the input the filter narrows. Sorted, because it comes
    // from a `BTreeSet`.
    let marked: Vec<ResourceAxis> = grafted
        .unbounded_resources
        .get(&P0)
        .expect("the accept marks P0's ∞ axes")
        .iter()
        .copied()
        .collect();
    assert!(
        !marked.is_empty(),
        "reach-guard: an empty ∞-mark set would make the exact-set assertion below vacuous"
    );

    let stash = registered_routes(&grafted);
    let [PersistentAxisMaterialization::DriveSequence { collapsed_axes, .. }] = stash else {
        panic!("assert_route already pinned the Replay route; got {stash:?}")
    };

    // EXACT SET, not `contains`: every axis this board marks is `DeferredAccrual`
    // (`ResourceAxis::unbounded_mark_kind`), so the accountability filter must drop NOTHING here.
    // `collapsed_axes` preserves `proposal.unbounded`'s order, so it is sorted for the comparison.
    let mut named = collapsed_axes.clone();
    named.sort();
    assert_eq!(
        named, marked,
        "CR 732.2c: this loop marks only DEFERRED axes, so the accountability filter is the \
         IDENTITY on it. A filter that empties or narrows `collapsed_axes` reds here while the \
         mixed-axis rows in `combo_infinite_pile` stay green — which is the whole reason this row \
         is separate from them. marked={marked:?}"
    );
}

/// **R-mixed** — the multi-authority hostile fixture. Two accepts by one controller in ONE phase
/// produce TWO route decisions sharing ONE stash and ONE boundary amount.
///
/// This is what makes the route a per-ACCEPT decision rather than a per-phase one: the cast trigger
/// is grafted BETWEEN the two accepts, so accept #1 is batched and accept #2 is replay on the same
/// board in the same phase.
///
/// The stash-composition assertion is load-bearing and not replaceable by the bound alone: a
/// bound-only row would pass on a board where BOTH accepts took the same route.
///
/// The boundary assertion is the CR 732.2c property at row scale — "the shortcut is taken; the game
/// advances to the last proposed ending point" — so the single prompt the two accepts share must
/// offer the count they were accepted at, on BOTH routes. It is not a cap row: there is no cap in
/// this phase, and `boundary_max` already panics unless exactly one collapse prompt addressed to
/// the loop's controller exists, so a route that published `MAX_SHORTCUT_CYCLES`, zero, or a second
/// prompt fails here.
#[test]
fn two_accepts_one_phase_one_batched_one_replay_share_one_boundary() {
    let mut state = offer_state(false);
    let phase_at_first_accept = state.phase;

    // ── accept #1: no cast trigger on the board yet ⇒ batched ──
    declare_and_accept_all(&mut state, P0, 100);
    assert_route(&state, ExpectedRoute::Batched);
    assert_eq!(
        registered_routes(&state).len(),
        1,
        "reach-guard: the first accept registered exactly one materialization"
    );

    // ── graft BETWEEN the accepts, then cast again in the SAME phase ──
    graft_cast_trigger(&mut state, "Cast Route Probe");
    let second = state
        .objects
        .get(&SECOND_CONVOKE_FODDER)
        .expect("the second convoke fodder is present");
    assert!(
        second.controller == P0 && !second.tapped,
        "fixture fact: the FIRST convoke tapped {FIRST_CONVOKE_FODDER:?}, so accept #2 needs \
         {SECOND_CONVOKE_FODDER:?} — it must still be an untapped P0 permanent"
    );
    let outcome = GameRunner::from_state(state)
        .cast(SPROUT)
        .accept_optional()
        .convoke_with(&[SECOND_CONVOKE_FODDER])
        .commit()
        .resolve();
    let mut state = outcome.state().clone();
    assert_eq!(
        state.phase, phase_at_first_accept,
        "R-mixed precondition: both accepts must land in ONE phase, so they share one CR 500.5 \
         boundary and one bound"
    );
    assert!(
        matches!(state.waiting_for, WaitingFor::LoopShortcut { proposer, .. } if proposer == P0),
        "reach-guard: the second recast must surface a second offer, got {:?}",
        state.waiting_for
    );

    // ── accept #2: the cast trigger is now functioning ⇒ replay ──
    declare_and_accept_all(&mut state, P0, 100);

    // The stash is the multi-authority evidence: two items, ONE per route.
    let observed: Vec<&'static str> = registered_routes(&state).iter().map(route_name).collect();
    assert_eq!(
        observed,
        vec!["Tokens", "DriveSequence"],
        "R-mixed: one stash holding the batched accept #1 and the replay accept #2 — the route is \
         decided PER ACCEPT from the board as it stands at that instant"
    );

    // ── one boundary, one amount: `min(100, 100)`, the count both accepts were taken at ──
    drive_to_boundary(&mut state);
    assert_eq!(
        boundary_max(&state),
        100,
        "R-mixed: ONE boundary applies ONE amount to every stashed item, and CR 732.2c makes that \
         amount the accepted count on both routes — neither route lowers the ceiling its own \
         accept published"
    );
}

// ===========================================================================
// R-mana — the ASYMMETRY row. A board whose BATCHED arm would register NOTHING must not be
// dragged onto the replay by the cast disjunct.
// ===========================================================================

/// **R-mana.** A real Basalt Monolith + Power Artifact mana engine carrying a functioning cast
/// trigger still registers NOTHING — the shipped unscheduled-axis shape — instead of scheduling a
/// `DriveSequence` that would deliver nothing: uncapped cubic replay cost plus a spurious CR 500.5
/// collapse prompt for a loop with nothing to collapse. (Post-`ResourceAxis::unbounded_mark_kind`
/// a `DriveSequence` on this rig would name NO axis at all — `Mana(_)` is the board's only ∞ axis
/// and it classifies `StandingCapability` — which is why the guard's merit is the cost and the
/// prompt, not a mark collapse.)
///
/// MEASURED REGRESSION, not a hypothetical. At the prior candidate `b560049f` this row is RED with
/// `[DriveSequence]`; the `!batched.is_empty()` guard at the route seam is what makes it GREEN. The
/// two arms of that seam are ASYMMETRIC: the batched arm registers CONDITIONALLY (token profile /
/// counter growth / life growth — a mana engine has none of the three), while the replay arm
/// registers a `DriveSequence` UNCONDITIONALLY. `cast_sourced` is the only route disjunct with no
/// axis-shaped conjunct, so before the fix ANY functioning cast trigger anywhere on the board —
/// `functioning_board_trigger_defs` walks `state.objects.values()` with NO controller filter, so an
/// OPPONENT's Monastery Mentor is enough — flipped this board from registering nothing to
/// scheduling a collapse. At the CR 500.5 boundary that stash raises a `PayableResource::LoopCollapse`
/// prompt this shape has never raised — that half is unchanged, because the prompt gate
/// `next_apnap_player_with_pending_materialization` reads STASH PRESENCE only — and, at the prior
/// candidate, `scheduled_collapse_axes` put `Mana(_)` into `axes_to_remove`, ending the ∞ mana
/// mark for the rest of the phase. That second harm is now closed PER AXIS by
/// `ResourceAxis::unbounded_mark_kind`, leaving the prompt and the replay cost as this guard's
/// remaining merit.
///
/// REACHABILITY, proved in-row rather than assumed, because a fixture that never reaches the
/// disjunct would pass vacuously. `functioning_board_trigger_defs` is
/// `objects.values()` × `active_trigger_definitions(state, obj)` × `trigger_definition_functions_in_zone(def, obj.zone)`.
/// Guard (1) runs the first of those two authorities on THIS board and shows the grafted cast-mode
/// def surviving it; the second is a pure `(def, zone)` function with no board input, and the same
/// `(SpellCast-mode def, Battlefield)` pair is measured route-flipping by
/// [`cast_trigger_board_routes_to_replay_untouched_board_stays_batched`] above, which shares this
/// file's [`graft_cast_trigger`]. Guard (2) shows the captured period is non-empty, so the seam's
/// other conjunct `!sequence.is_empty()` holds too. Guard (3) shows the accept genuinely reached
/// `materialize_object_growth_shortcut` (only that path marks the axis), so the emptiness at (4) is
/// a route decision and not "nothing happened". With (1)–(3) satisfied, `cast_sourced` is the ONLY
/// thing between this board and the replay route — which the RED-at-`b560049f` measurement
/// independently confirms, since the fix touches nothing else.
///
/// The paired POSITIVE for this row is not on this rig — a mana engine cannot be given a batched
/// payload without ceasing to be one — it is the grafted arm of
/// [`cast_trigger_board_routes_to_replay_untouched_board_stays_batched`], where the same graft on a
/// board that DOES have a batched payload still routes `Replay`. Together the pair says the
/// disjunct fires on payload and not on the trigger alone.
///
/// The `shared_card_db()` guard is DORMANT in a normal checkout (`integration_cards.json.gz` is
/// tracked); it only fires in a checkout without the card-data pipeline.
#[test]
fn mana_engine_with_cast_trigger_registers_nothing() {
    let Some(db) = shared_card_db() else { return };
    // The mana rig is built on `game::scenario::P0`; this file's `P0` must be the same seat for
    // the graft to land on the loop controller's board at all.
    assert_eq!(
        P0,
        engine::game::scenario::P0,
        "fixture fact: both modules mean the same seat"
    );
    let mut rig = setup(true, LoopDetectionMode::Interactive, db);
    let host = graft_cast_trigger(rig.runner.state_mut(), "Mana Route Probe");

    // ── (1) reach-guard: the scan's per-object authority yields the grafted cast-mode def on THIS
    // board (the graft is not merely present in `objects`, it survives the CR 702.26b / CR 114.4
    // gate that `functioning_board_trigger_defs` applies before the zone gate) ──
    let state = rig.runner.state();
    let active: Vec<&TriggerDefinition> = active_trigger_definitions(
        state,
        state.objects.get(&host).expect("the graft host is present"),
    )
    .map(|entry| entry.definition)
    .collect();
    assert_eq!(
        active.len(),
        1,
        "reach-guard: the graft host carries exactly one ACTIVE trigger def on the mana rig's board"
    );
    assert_eq!(
        active[0].mode,
        TriggerMode::SpellCast,
        "reach-guard: the grafted cast trigger survives the gate `functioning_board_trigger_defs` \
         applies before the zone gate — without this the row never reaches the cast disjunct and \
         passes vacuously"
    );

    let mana_idx = mana_ability_index(rig.runner.state(), rig.basalt)
        .expect("Basalt's {T}: Add {C}{C}{C} mana ability");
    let untap_idx = untap_ability_index(rig.runner.state(), rig.basalt)
        .expect("Basalt's {3}: Untap this artifact ability");
    drive_one_period(&mut rig, mana_idx, untap_idx);
    assert!(
        matches!(
            rig.runner.state().waiting_for,
            WaitingFor::LoopShortcut { .. }
        ),
        "reach-guard: the mana-engine offer must still fire WITH the cast trigger grafted, got {:?}",
        rig.runner.state().waiting_for
    );
    // ── (2) reach-guard: the captured period is the two-activation Basalt+Power cycle, so the
    // route seam's `!sequence.is_empty()` conjunct is satisfied ──
    assert_eq!(
        rig.runner.state().last_loop_action_sequence.len(),
        2,
        "reach-guard: the multi-action mana period is captured, so `!sequence.is_empty()` holds \
         and the cast disjunct is the only conjunct left to decide the route"
    );

    rig.runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(1),
            template: None,
        })
        .expect("declare shortcut");
    rig.runner
        .act(GameAction::RespondToShortcut {
            response: ShortcutResponse::Accept,
        })
        .expect("opponent accepts");

    // ── (3) reach-guard: the accept really ran the materialize path — only it marks the axis ──
    assert!(
        rig.runner
            .state()
            .unbounded_resources
            .get(&P0)
            .is_some_and(|axes| axes.iter().any(|a| matches!(a, ResourceAxis::Mana(_)))),
        "reach-guard: the accept reaches materialize_object_growth_shortcut and marks Mana(_)"
    );

    // ── (4) DISCRIMINATOR: it registered NOTHING, preserving the ∞ mana mark ──
    let observed: Vec<&'static str> = rig
        .runner
        .state()
        .pending_unbounded_materialization
        .values()
        .flatten()
        .map(route_name)
        .collect();
    assert!(
        rig.runner
            .state()
            .pending_unbounded_materialization
            .is_empty(),
        "a mana engine registers NO deferred materialization even with a functioning cast trigger \
         on the board — the batched arm would register nothing, so there is nothing for the \
         replay to be a better version OF, and scheduling one buys uncapped cubic replay cost \
         plus a spurious CR 500.5 collapse prompt for a loop with nothing to collapse; observed \
         {observed:?}"
    );
}
