//! Regression for issue #6640: The Serpent Society's Ward—Get five poison
//! counters never gave the targeting opponent poison counters, because the
//! Oracle parser had no `WardCost` variant for "give yourself N counters" and
//! silently fell back to `WardCost::Mana(generic: 0)` — a free, always-paid
//! Ward that does nothing.
//!
//! https://github.com/phase-rs/phase/issues/6640
//!
//! CR references:
//!   - CR 702.21a: Ward — counter the targeting spell/ability unless the
//!     targeting player pays the stated cost.
//!   - CR 122.1 + CR 104.3d: giving a player poison counters; a player with
//!     ten or more poison counters loses the game (a separate SBA, not
//!     exercised by this test).

use engine::game::scenario::{GameScenario, P0, P1};
use engine::game::zones::create_object;
use engine::types::ability::{
    EffectKind, QuantityModification, ReplacementDefinition, ReplacementMode,
    ReplacementPlayerScope,
};
use engine::types::actions::GameAction;
use engine::types::events::GameEvent;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::CardId;
use engine::types::phase::Phase;
use engine::types::player::PlayerCounterKind;
use engine::types::replacements::ReplacementEvent;
use engine::types::zones::Zone;
use std::sync::Arc;

const SERPENT_SOCIETY: &str = "Deathtouch\n\
Ward—Get five poison counters. (A player with ten or more poison counters loses the game.)\n\
Whenever another creature you control with deathtouch dies, each opponent sacrifices a nontoken creature of their choice.";

#[test]
fn serpent_society_ward_prompts_the_targeting_opponent_for_poison_counters() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let serpent_society = scenario
        .add_creature_from_oracle(P0, "The Serpent Society", 3, 4, SERPENT_SOCIETY)
        .id();
    let destroy = scenario
        .add_spell_to_hand_from_oracle(P1, "Destroy Spell", true, "Destroy target creature.")
        .id();
    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }

    runner
        .cast(destroy)
        .target_objects(&[serpent_society])
        .commit();
    runner.advance_until_stack_empty();

    let WaitingFor::UnlessPayment { player, cost, .. } = &runner.state().waiting_for else {
        panic!(
            "Ward must prompt the targeting opponent to pay the poison-counter cost, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(*player, P1);
    assert!(matches!(
        cost,
        engine::types::ability::AbilityCost::GetPlayerCounters {
            counter_kind: PlayerCounterKind::Poison,
            count: 5,
        }
    ));
}

#[test]
fn serpent_society_ward_declined_counters_the_spell_and_gives_no_poison() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let serpent_society = scenario
        .add_creature_from_oracle(P0, "The Serpent Society", 3, 4, SERPENT_SOCIETY)
        .id();
    let destroy = scenario
        .add_spell_to_hand_from_oracle(P1, "Destroy Spell", true, "Destroy target creature.")
        .id();
    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }

    runner
        .cast(destroy)
        .target_objects(&[serpent_society])
        .commit();
    runner.advance_until_stack_empty();

    runner
        .act(GameAction::PayUnlessCost { pay: false })
        .expect("declining Ward must be a legal action");
    runner.advance_until_stack_empty();

    assert_eq!(
        runner.state().players[P1.0 as usize].poison_counters,
        0,
        "declining Ward's cost must not give the opponent any poison counters"
    );
    assert!(
        runner
            .state()
            .objects
            .get(&serpent_society)
            .is_some_and(|obj| obj.zone == engine::types::zones::Zone::Battlefield),
        "declining Ward's cost must counter the targeting spell, leaving Serpent Society alive"
    );
    assert!(
        !runner.state().stack.iter().any(|entry| entry.id == destroy),
        "the countered spell must be removed from the stack"
    );
}

#[test]
fn serpent_society_ward_paid_gives_five_poison_and_the_spell_resolves() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let serpent_society = scenario
        .add_creature_from_oracle(P0, "The Serpent Society", 3, 4, SERPENT_SOCIETY)
        .id();
    let destroy = scenario
        .add_spell_to_hand_from_oracle(P1, "Destroy Spell", true, "Destroy target creature.")
        .id();
    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }

    runner
        .cast(destroy)
        .target_objects(&[serpent_society])
        .commit();
    runner.advance_until_stack_empty();

    runner
        .act(GameAction::PayUnlessCost { pay: true })
        .expect("the opponent pays Ward's poison-counter cost");
    runner.advance_until_stack_empty();

    assert_eq!(
        runner.state().players[P1.0 as usize].poison_counters,
        5,
        "paying Ward's cost must give the targeting opponent five poison counters"
    );
    assert!(
        runner
            .state()
            .objects
            .get(&serpent_society)
            .is_none_or(|obj| obj.zone != engine::types::zones::Zone::Battlefield),
        "paying Ward's cost must let the targeted destroy spell resolve, removing Serpent Society from the battlefield"
    );
}

/// CR 104.3d + CR 704.5c: a payment that pushes the payer to ten or more
/// poison counters must trigger the loss state-based action immediately —
/// before the targeted destroy spell gets a chance to continue resolving.
/// Mirrors `crates/engine/src/game/sba.rs`'s own `sba_poison_10_player_loses`
/// unit test's expected shape.
#[test]
fn serpent_society_ward_payment_that_reaches_ten_poison_loses_the_game() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let serpent_society = scenario
        .add_creature_from_oracle(P0, "The Serpent Society", 3, 4, SERPENT_SOCIETY)
        .id();
    let destroy = scenario
        .add_spell_to_hand_from_oracle(P1, "Destroy Spell", true, "Destroy target creature.")
        .id();
    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
        state.players[P1.0 as usize].poison_counters = 5;
    }

    runner
        .cast(destroy)
        .target_objects(&[serpent_society])
        .commit();
    runner.advance_until_stack_empty();

    runner
        .act(GameAction::PayUnlessCost { pay: true })
        .expect("the opponent pays Ward's poison-counter cost");

    assert_eq!(
        runner.state().players[P1.0 as usize].poison_counters,
        10,
        "5 existing + 5 from Ward's cost must reach the ten-poison threshold"
    );
    assert!(
        matches!(
            runner.state().waiting_for,
            WaitingFor::GameOver { winner: Some(p) } if p == P0
        ),
        "reaching ten poison must trigger the CR 104.3d loss SBA immediately, got {:?}",
        runner.state().waiting_for
    );
    assert!(
        runner
            .state()
            .objects
            .get(&serpent_society)
            .is_some_and(|obj| obj.zone == engine::types::zones::Zone::Battlefield),
        "the game must end (P1 loses) before the destroy spell gets a chance to resolve, so Serpent Society must still be on the battlefield"
    );
}

/// CR 122.1 + CR 614.17 + CR 702.21a: Solemnity's "Players can't get
/// counters" is a CR 614.17 can't-effect, not a CR 614.1 replacement, and it
/// makes Ward's poison-counter cost a FAILED payment rather than a free bypass.
/// Before this fix, `add_player_counter_with_replacement` reported `Prevented`
/// as if it were a paid cost, so the targeting opponent's spell would
/// incorrectly continue resolving even though no poison was actually given —
/// nullifying Ward's entire deterrent for free.
///
/// CR 614.17c is why this row never reaches the deferred settle at all: an
/// event that can't happen "can only be replaced by a self-replacement effect …
/// Other replacement and/or prevention effects can't modify or replace it", so
/// `replacement::pipeline_loop` short-circuits a MANDATORY prohibition ahead of
/// any CR 616.1 ordering prompt. The payment is therefore decided synchronously
/// inside `costs::pay_ability_cost_for_resolution` (CR 614.17b: a player can't
/// pay a cost that includes an event that can't happen) and never parks. That
/// is what makes this row the over-reach discriminator: if the payment ever did
/// park, `resume_counter_addition_unless_payment` would settle it PAID under
/// CR 118.12 and this row's premise would be wrong — so the synchronous settle
/// is asserted below, not just the final board.
///
/// Solemnity's real Oracle text is "Players can't get counters." /
/// "Counters can't be put on artifacts, creatures, enchantments, or lands." —
/// only the first (relevant) sentence is used in this fixture.
#[test]
fn serpent_society_ward_payment_prevented_by_solemnity_counters_the_spell() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario
        .add_creature_from_oracle(P0, "Solemnity", 0, 0, "Players can't get counters.")
        .as_enchantment();
    let serpent_society = scenario
        .add_creature_from_oracle(P0, "The Serpent Society", 3, 4, SERPENT_SOCIETY)
        .id();
    let destroy = scenario
        .add_spell_to_hand_from_oracle(P1, "Destroy Spell", true, "Destroy target creature.")
        .id();
    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
    }

    runner
        .cast(destroy)
        .target_objects(&[serpent_society])
        .commit();
    runner.advance_until_stack_empty();

    runner
        .act(GameAction::PayUnlessCost { pay: true })
        .expect("attempting to pay Ward's poison-counter cost must be a legal action even when Solemnity prevents the actual counter gain");

    // CR 614.17c: a mandatory "players can't get counters" effect is
    // short-circuited ahead of any CR 616.1 ordering prompt, so this payment is
    // settled synchronously and never parks.
    assert!(
        runner.state().pending_cost_move_resume.is_none(),
        "a mandatory can't-effect must not park a cost-move resume, got {:?}",
        runner.state().pending_cost_move_resume
    );
    assert!(
        matches!(runner.state().waiting_for, WaitingFor::Priority { player } if player == P1),
        "the Solemnity-prevented payment settles synchronously, got {:?}",
        runner.state().waiting_for
    );

    runner.advance_until_stack_empty();

    assert_eq!(
        runner.state().players[P1.0 as usize].poison_counters,
        0,
        "Solemnity must prevent the poison counters from actually being given"
    );
    assert!(
        runner
            .state()
            .objects
            .get(&serpent_society)
            .is_some_and(|obj| obj.zone == engine::types::zones::Zone::Battlefield),
        "a prevented player-counter payment must be treated as a FAILED cost, countering the targeting spell exactly like a declined payment — Serpent Society must survive"
    );
    assert!(
        !runner.state().stack.iter().any(|entry| entry.id == destroy),
        "the countered spell must be removed from the stack"
    );
}

/// Installs a synthetic OPTIONAL "you may prevent a player from getting
/// counters" replacement on a fresh P0 permanent. No real card has exactly
/// this wording, so — mirroring this file's own Solemnity test (which uses a
/// real, if partial, MANDATORY prevention) and the engine's established
/// pattern for exercising an optional replacement choice with no real-card
/// precedent — the definition is installed directly, after `scenario.build()`,
/// so the real Ward -> `GetPlayerCounters` -> `add_player_counter_with_
/// replacement` -> `replace_event` path discovers it naturally (a production
/// setup, not a hand-constructed `WaitingFor`).
///
/// Why synthetic, stated as a predicate rather than a card list: no printed card
/// produces an OPTIONAL `AddCounter` replacement. Every definition matching
/// `event == "AddCounter"` in `client/public/card-data.json` is `Mandatory`
/// (33 of 33 at time of writing; regenerate with
/// `jq '[.[] | (.replacements // [])[] | select(.event=="AddCounter") | .mode.type] | group_by(.) | map({m:.[0],n:length})' client/public/card-data.json`).
/// Combined with CR 614.17c — which short-circuits every MANDATORY prohibition
/// ahead of the CR 616.1 prompt — this synthetic definition is the only route to
/// `CostMoveDrainBoundary::ReplacementPrevented` at the counter-addition resume
/// root, which is real-card-dead today.
///
/// The field set is load-bearing, and this is the single site that owns the
/// reason. `object_replacement_candidate_applies` (`game/replacement.rs`)
/// consults `repl_def.valid_card` for EVERY event kind, including player
/// placements, whenever it is `Some`; `replacement_valid_card_matches` resolves
/// an `AddCounter` event through `ProposedEvent::affected_object_id`, which is
/// `CounterPlacement::object_id` — `None` for a player placement — and then
/// `.unwrap_or(false)`. So a definition carrying a `valid_card` filter is
/// EXCLUDED from a player counter placement, and the candidate predicate for
/// this fixture's event is `valid_player.is_some() && valid_card.is_none()`
/// (plus the counter-type matcher, the condition, and the mode). `valid_card` is
/// therefore left `None` here deliberately, not by omission.
fn install_optional_player_counter_prevention(state: &mut engine::types::game_state::GameState) {
    let source = create_object(
        state,
        CardId(9101),
        P0,
        "Optional Poison Warden".to_string(),
        Zone::Battlefield,
    );
    let mut def = ReplacementDefinition::new(ReplacementEvent::AddCounter);
    def.mode = ReplacementMode::Optional { decline: None };
    def.quantity_modification = Some(QuantityModification::Prevent);
    def.valid_player = Some(ReplacementPlayerScope::AnyPlayer);
    let reps = vec![def];
    let obj = state.objects.get_mut(&source).unwrap();
    obj.replacement_definitions = reps.clone().into();
    obj.base_replacement_definitions = Arc::new(reps);
}

/// Regression for reviewer matthewevans's finding on PR #6662: a Ward
/// player-counter cost whose `AddCounter` event needs a CR 616.1 replacement
/// choice (as opposed to Solemnity's unconditional, mandatory prevention
/// above) must not orphan the unless-payment continuation. Before this fix,
/// `add_player_counter_with_replacement`'s `NeedsChoice` arm replaced
/// `waiting_for` with the bare `ReplacementChoice` prompt and nothing
/// preserved `pending_effect`/`trigger_event` — once the player answered the
/// prompt, `handle_replacement_choice` applied (or failed to apply) the
/// counters and reset straight to `WaitingFor::Priority`, leaving Ward's
/// guarded "counter the spell" outcome permanently undetermined: the
/// targeting spell was neither countered nor allowed to resolve.
///
/// Accept branch, and the discriminating `ReplacementPrevented` case: the payer
/// ACCEPTS the optional prevention, so the counter placement is completely
/// replaced (CR 614.6 — "if an event is replaced, it never happens") and zero
/// poison counters are given.
///
/// CR 118.12 is why the Ward cost is nevertheless PAID: the "if they don't"
/// clause "checks whether the player chose to pay an optional cost … regardless
/// of what events actually occurred", and that choice was latched at
/// `PayUnlessCost { pay: true }` — before the replacement pipeline was ever
/// consulted. CR 118.11 corroborates: a cost whose payment actions were modified
/// is still paid. So the targeting spell RESOLVES and Serpent Society dies.
///
/// This fixture — not the Solemnity row above — is the only route to
/// `CostMoveDrainBoundary::ReplacementPrevented` at the counter-addition resume
/// root. Solemnity's MANDATORY can't-effect is short-circuited by CR 614.17c
/// before any CR 616.1 prompt exists, so it is settled synchronously in
/// `costs.rs` (CR 614.17b) and never parks. Only an OPTIONAL prevention that the
/// payer accepts can carry a prevented placement into this resume.
#[test]
fn serpent_society_ward_optional_counter_prevention_accepted_still_pays_the_ward_cost_and_resolves_the_spell(
) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let serpent_society = scenario
        .add_creature_from_oracle(P0, "The Serpent Society", 3, 4, SERPENT_SOCIETY)
        .id();
    let destroy = scenario
        .add_spell_to_hand_from_oracle(P1, "Destroy Spell", true, "Destroy target creature.")
        .id();
    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
        install_optional_player_counter_prevention(state);
    }

    runner
        .cast(destroy)
        .target_objects(&[serpent_society])
        .commit();
    runner.advance_until_stack_empty();

    runner
        .act(GameAction::PayUnlessCost { pay: true })
        .expect("attempting to pay Ward's poison-counter cost must be legal even when an optional replacement can prevent it");

    // Reaching a REPLACEMENT CHOICE (not an orphaned bare Priority) is the
    // regression's core assertion.
    let WaitingFor::ReplacementChoice {
        player,
        candidate_count,
        ..
    } = runner.state().waiting_for
    else {
        panic!(
            "optional player-counter prevention must surface a real replacement choice, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(
        player, P1,
        "the payer (Ward's targeting opponent) makes the replacement choice"
    );
    assert_eq!(
        candidate_count, 2,
        "an Optional replacement offers accept (0) and decline (1)"
    );

    let result = runner
        .act(GameAction::ChooseReplacement { index: 0 })
        .expect("accepting the optional prevention must be a legal replacement choice");

    runner.advance_until_stack_empty();

    assert_eq!(
        runner.state().players[P1.0 as usize].poison_counters,
        0,
        "CR 614.6: the accepted prevention must stop the poison counters from being given"
    );
    // The maintainer's required discriminator: the payer chose to pay, so under
    // CR 118.12 the Ward cost is PAID even though the placement was replaced away.
    // Before the fix this arm mapped `ReplacementPrevented` to a failed payment
    // and countered the spell instead.
    assert!(
        runner
            .state()
            .objects
            .get(&serpent_society)
            .is_none_or(|obj| obj.zone != Zone::Battlefield),
        "CR 118.12: a prevented placement on a chosen-to-pay cost is still a PAID cost — the targeting spell must resolve and remove Serpent Society from the battlefield"
    );
    assert!(
        !runner.state().stack.iter().any(|entry| entry.id == destroy),
        "the targeting spell must leave the stack by RESOLVING, not be left stranded"
    );

    // CR 118.12: the resume settles through the PAID epilogue, so Ward's guarded
    // ability finishes resolving and the whole reducer step's event buffer
    // survives instead of being discarded by the decline tail. Asserted on the
    // events captured from the `ChooseReplacement` act above.
    assert!(
        result.events.iter().any(|event| matches!(
            event,
            GameEvent::EffectResolved {
                kind: EffectKind::Counter,
                ..
            }
        )),
        "a paid Ward cost must emit the guarded ability's EffectResolved, got {:?}",
        result.events
    );
}

/// Decline branch: the optional replacement does not apply, so the original
/// `AddCounter` proceeds unmodified (`PlayerCounterAdditionOutcome::Applied`)
/// — a PAID Ward payment, so the targeting spell must resolve normally.
#[test]
fn serpent_society_ward_optional_counter_prevention_declined_pays_the_cost_and_resolves_the_spell()
{
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let serpent_society = scenario
        .add_creature_from_oracle(P0, "The Serpent Society", 3, 4, SERPENT_SOCIETY)
        .id();
    let destroy = scenario
        .add_spell_to_hand_from_oracle(P1, "Destroy Spell", true, "Destroy target creature.")
        .id();
    let mut runner = scenario.build();
    {
        let state = runner.state_mut();
        state.active_player = P1;
        state.priority_player = P1;
        state.waiting_for = WaitingFor::Priority { player: P1 };
        install_optional_player_counter_prevention(state);
    }

    runner
        .cast(destroy)
        .target_objects(&[serpent_society])
        .commit();
    runner.advance_until_stack_empty();

    runner
        .act(GameAction::PayUnlessCost { pay: true })
        .expect("attempting to pay must be legal");
    let WaitingFor::ReplacementChoice { .. } = runner.state().waiting_for else {
        panic!(
            "expected a replacement choice, got {:?}",
            runner.state().waiting_for
        );
    };

    let result = runner
        .act(GameAction::ChooseReplacement { index: 1 })
        .expect("declining the optional prevention must be a legal replacement choice");

    // CR 118.12: the deferred paid settle must emit exactly what the immediate
    // paid leg emits — the counters the payer actually took, and the guarded
    // ability's completion. Before the fix this reducer step returned NO events
    // at all: the resume routed through the decline tail, whose `ActionResult` is
    // discarded while `action_result` has already drained the event buffer.
    assert!(
        result.events.iter().any(|event| matches!(
            event,
            GameEvent::PlayerCounterChanged {
                player,
                counter_kind: PlayerCounterKind::Poison,
                delta: 5,
            } if *player == P1
        )),
        "the poison counters the payer actually took must reach the event log, got {:?}",
        result.events
    );
    assert!(
        result.events.iter().any(|event| matches!(
            event,
            GameEvent::EffectResolved {
                kind: EffectKind::Counter,
                ..
            }
        )),
        "a paid Ward cost must emit the guarded ability's EffectResolved, got {:?}",
        result.events
    );

    runner.advance_until_stack_empty();

    assert_eq!(
        runner.state().players[P1.0 as usize].poison_counters,
        5,
        "declining the optional prevention must let Ward's cost actually give five poison counters"
    );
    assert!(
        runner
            .state()
            .objects
            .get(&serpent_society)
            .is_none_or(|obj| obj.zone != Zone::Battlefield),
        "a successfully paid Ward cost must let the targeted destroy spell resolve"
    );
}
