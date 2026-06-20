//! Kid Loki — "Each creature you control that you've put one or more +1/+1
//! counters on this turn has hexproof."
//!
//! Oracle text:
//!   "Each creature you control that you've put one or more +1/+1 counters on
//!    this turn has hexproof.
//!    Whenever you draw your second card each turn, put a +1/+1 counter on Kid
//!    Loki."
//!
//! This drives the REAL parse → layer pipeline: Kid Loki is built from Oracle
//! text via the scenario harness (same synthesis path as production). The
//! conditional static lowers to a `StaticDefinition` whose `affected` filter
//! carries `FilterProp::CountersPutOnThisTurn { actor: Controller, counters:
//! OfType(+1/+1), comparator: GE, count: 1 }` and modifications
//! `[AddKeyword { Hexproof }]`. The counter-placement history (CR 122.6) is
//! populated through `apply_counter_addition`, the single authority the engine
//! uses whenever a +1/+1 counter is put on a permanent.
//!
//! THE BUG this discriminates: the static is a *historical-action* predicate
//! (CR 122.6 "counters being put on an object"), NOT a current-counter query.
//! Assertion (a) — the creature that received a +1/+1 counter this turn HAS
//! hexproof — fails if the static line is left `Effect::Unimplemented` (the
//! pre-fix state) or if the new filter never matches. Assertion (b) — a creature
//! you control that received NO counter this turn does NOT have hexproof —
//! discriminates a degenerate "all creatures you control" misparse. Assertion
//! (c) — an opponent's creature with a +1/+1 counter does NOT get hexproof from
//! Kid Loki — discriminates the `actor: Controller` scope.
//!
//! Counter placement is driven through the public `resolve_ability_chain`
//! production entry with an `Effect::PutCounter` resolved ability — the same
//! seam Kid Loki's own draw trigger uses — so the CR 122.6 placement history is
//! recorded exactly as it is in production.

use engine::game::effects::resolve_ability_chain;
use engine::game::keywords::has_keyword;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario};
use engine::types::ability::{Effect, QuantityExpr, ResolvedAbility, TargetFilter, TargetRef};
use engine::types::counter::CounterType;
use engine::types::identifiers::ObjectId;
use engine::types::keywords::Keyword;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

const KID_LOKI: &str = "Each creature you control that you've put one or more +1/+1 counters on this turn has hexproof.\n\
Whenever you draw your second card each turn, put a +1/+1 counter on Kid Loki.";

/// True iff `id` currently has `keyword` after a fresh layer evaluation.
fn has_kw(runner: &mut GameRunner, id: ObjectId, keyword: &Keyword) -> bool {
    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());
    has_keyword(&runner.state().objects[&id], keyword)
}

/// CR 122.6: Put one +1/+1 counter on `recipient` as `actor`, driven through the
/// public `resolve_ability_chain` production seam (the same path Kid Loki's draw
/// trigger uses) so the placement is recorded in `counter_added_this_turn`.
fn put_counter(runner: &mut GameRunner, actor: PlayerId, recipient: ObjectId) {
    let ability = ResolvedAbility::new(
        Effect::PutCounter {
            counter_type: CounterType::Plus1Plus1,
            count: QuantityExpr::Fixed { value: 1 },
            target: TargetFilter::ParentTarget,
        },
        vec![TargetRef::Object(recipient)],
        recipient,
        actor,
    );
    let mut events = Vec::new();
    resolve_ability_chain(runner.state_mut(), &ability, &mut events, 0)
        .expect("PutCounter must resolve");
}

#[test]
fn kid_loki_grants_hexproof_only_to_creatures_you_put_counters_on_this_turn() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    // Kid Loki (P0), built from Oracle text through the real parse + synthesis
    // pipeline — carries the conditional hexproof static.
    let _kid_loki = scenario
        .add_creature_from_oracle(P0, "Kid Loki", 1, 4, KID_LOKI)
        .id();

    // P0 creatures: `buffed` will receive a +1/+1 counter this turn; `plain`
    // never does — the negative discriminator.
    let buffed = scenario.add_creature(P0, "Buffed Bear", 2, 2).id();
    let plain = scenario.add_creature(P0, "Plain Bear", 2, 2).id();

    // P1 creature that receives a +1/+1 counter from P1 — the `actor` scope
    // discriminator. Kid Loki's static is scoped to counters YOU (P0) put.
    let opp = scenario.add_creature(P1, "Opponent Bear", 2, 2).id();

    let mut runner = scenario.build();

    // ----- Baseline: no counters placed yet → nobody has hexproof -----
    assert!(
        !has_kw(&mut runner, buffed, &Keyword::Hexproof),
        "before any counter is placed, no creature has Kid Loki's hexproof"
    );

    // P0 puts a +1/+1 counter on `buffed` this turn (CR 122.6 history record).
    put_counter(&mut runner, P0, buffed);
    // P1 puts a +1/+1 counter on their own creature this turn.
    put_counter(&mut runner, P1, opp);

    // (a) The creature P0 put a +1/+1 counter on this turn HAS hexproof. This
    // assertion flips to failure if the Kid Loki static line is reverted to
    // `Effect::Unimplemented` (no static installed) or the new filter never
    // matches the placement record.
    assert!(
        has_kw(&mut runner, buffed, &Keyword::Hexproof),
        "CR 122.6 + CR 702.11: a creature you put a +1/+1 counter on this turn gains hexproof"
    );

    // (b) A creature you control with NO counter this turn does NOT have
    // hexproof — discriminates a degenerate "all creatures you control" misparse.
    assert!(
        !has_kw(&mut runner, plain, &Keyword::Hexproof),
        "a creature with no +1/+1 counter placed this turn must NOT have hexproof"
    );

    // (c) The opponent's creature — buffed by the OPPONENT, not by you — does NOT
    // gain hexproof from Kid Loki. Discriminates the `actor: Controller` scope.
    assert!(
        !has_kw(&mut runner, opp, &Keyword::Hexproof),
        "CR 109.5: Kid Loki's static is scoped to counters YOU put — an opponent's counter does not qualify"
    );
}
