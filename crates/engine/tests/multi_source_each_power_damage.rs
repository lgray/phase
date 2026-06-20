//! Runtime regression: "multiple source creatures each deal damage equal to
//! their power to a single target" (`DamageSource::EachTarget`).
//!
//! Three Standard-legal cards are genuinely supported by this clause — Allies at
//! Last, Coordinated Clobbering, Terrific Team-Up. (A fourth, Graceful Takedown,
//! has a HETEROGENEOUS compound source set — "<group A> and up to one other
//! target <group B>" — that the single-filter source picker cannot represent; it
//! is deferred to an honest `Effect::Unimplemented` at the parser. See the parser
//! unit test `graceful_takedown_compound_source_is_honest_unimplemented`.) The
//! single-source form ("target creature you control deals damage equal to its
//! power to target creature") was already supported; this exercises the
//! MULTI-source generalization where EACH chosen source deals its OWN power to
//! the shared recipient.
//!
//! CR 120.1: the object that deals damage is the source of that damage.
//! CR 601.2c: a variable number of targets is announced once; each chosen object
//!            becomes a target.
//! CR 208.1 + CR 608.2: a creature's power is a modifiable characteristic, read
//!            at resolution (current value).
//!
//! The recipients are sized so the assertions DISCRIMINATE the multi-source
//! semantics: a 1/5 recipient survives a single power-3 source (3 < 5) but dies
//! to the SUM of two power-3 sources (6 >= 5). Reverting the parser change
//! (clause → `Effect::Unimplemented`, no damage) or the runtime change (only one
//! source resolves, 3 damage) leaves the recipient alive and fails the test.

use engine::game::scenario::{GameScenario, P0, P1};
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const COORDINATED_CLOBBERING: &str = "Tap one or two target untapped creatures you control. \
     They each deal damage equal to their power to target creature an opponent controls.";

const ALLIES_AT_LAST: &str = "Up to two target creatures you control each deal damage equal \
     to their power to target creature an opponent controls.";

const TERRIFIC_TEAM_UP: &str = "One or two target creatures you control each get +1/+0 until \
     end of turn. They each deal damage equal to their power to target creature an opponent \
     controls.";

/// Coordinated Clobbering — back-reference form ("They each deal …" after the
/// tap sentence). Two power-3 creatures each deal 3 to a 1/5 opponent creature:
/// 6 total, lethal. Asserts the recipient is dealt the SUM of both sources'
/// powers (it dies and leaves the battlefield), and both sources are tapped.
#[test]
fn coordinated_clobbering_two_sources_each_deal_own_power() {
    let mut scenario = GameScenario::new_n_player(2, 42);
    scenario.at_phase(Phase::PreCombatMain);

    // Two power-3 sources the controller will tap and have deal damage.
    let source_a = scenario.add_vanilla(P0, 3, 3);
    let source_b = scenario.add_vanilla(P0, 3, 3);
    // A 1/5 recipient: survives 3 damage (one source) but dies to 6 (both).
    let recipient = scenario.add_vanilla(P1, 1, 5);

    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Coordinated Clobbering", false, COORDINATED_CLOBBERING)
        .with_mana_cost(ManaCost::zero())
        .id();

    let mut runner = scenario.build();

    // Sources first (the two tapped creatures), then the shared recipient.
    let outcome = runner
        .cast(spell)
        .target_objects(&[source_a, source_b, recipient])
        .resolve();

    let state = outcome.state();
    // CR 208.1 + CR 608.2 + CR 120.1: 3 (source_a) + 3 (source_b) = 6 damage; the 1/5 dies.
    assert_eq!(
        outcome.zone_of(recipient),
        Zone::Graveyard,
        "recipient must take 6 total damage (both sources) and die; \
         single-source 3 would leave it alive — got recipient in {:?}",
        outcome.zone_of(recipient)
    );
    // The leading "Tap one or two target … creatures" sentence taps both sources.
    assert!(
        state.objects[&source_a].tapped,
        "source_a must be tapped by the tap clause"
    );
    assert!(
        state.objects[&source_b].tapped,
        "source_b must be tapped by the tap clause"
    );
}

/// Coordinated Clobbering — single chosen source (the "one or two" lower bound).
/// One power-3 source deals exactly 3 to a 1/5 recipient: it SURVIVES (3 < 5).
/// This negative case proves the recipient's death in the two-source test comes
/// from the SUM of both sources, not from a single source over-dealing.
#[test]
fn coordinated_clobbering_single_source_deals_only_its_own_power() {
    let mut scenario = GameScenario::new_n_player(2, 42);
    scenario.at_phase(Phase::PreCombatMain);

    let source_a = scenario.add_vanilla(P0, 3, 3);
    let recipient = scenario.add_vanilla(P1, 1, 5);

    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Coordinated Clobbering", false, COORDINATED_CLOBBERING)
        .with_mana_cost(ManaCost::zero())
        .id();

    let mut runner = scenario.build();

    let outcome = runner
        .cast(spell)
        .target_objects(&[source_a, recipient])
        .resolve();

    let state = outcome.state();
    // CR 208.1 + CR 608.2: only source_a's power (3) is dealt; the 1/5 survives.
    assert_eq!(
        outcome.zone_of(recipient),
        Zone::Battlefield,
        "single source deals only its own power (3 < 5); recipient must survive"
    );
    assert_eq!(
        state.objects[&recipient].damage_marked, 3,
        "recipient must be marked exactly 3 (source_a's power), not more"
    );
    assert!(state.objects[&source_a].tapped, "source_a must be tapped");
}

/// Allies at Last — direct subject form ("Up to two target creatures you control
/// each deal damage equal to their power …"). Two power-4 sources each deal 4 to
/// a 2/7 recipient: 8 total, lethal (8 >= 7). Exercises the `TargetOnly` source
/// picker + `EachTarget` sub-ability path (no preceding tap/pump sentence).
#[test]
fn allies_at_last_direct_subject_two_sources_each_deal_own_power() {
    let mut scenario = GameScenario::new_n_player(2, 42);
    scenario.at_phase(Phase::PreCombatMain);

    let source_a = scenario.add_vanilla(P0, 4, 4);
    let source_b = scenario.add_vanilla(P0, 4, 4);
    // 2/7 recipient: survives one power-4 source (4 < 7), dies to both (8 >= 7).
    let recipient = scenario.add_vanilla(P1, 2, 7);

    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Allies at Last", false, ALLIES_AT_LAST)
        .with_mana_cost(ManaCost::zero())
        .id();

    let mut runner = scenario.build();

    let outcome = runner
        .cast(spell)
        .target_objects(&[source_a, source_b, recipient])
        .resolve();

    // CR 120.1 + CR 208.1 + CR 608.2: 4 + 4 = 8 damage from the two sources; the 2/7 dies.
    assert_eq!(
        outcome.zone_of(recipient),
        Zone::Graveyard,
        "recipient must take 8 total (both sources) and die"
    );
}

/// Terrific Team-Up — the "get +1/+0 then they each deal damage" form. The
/// SAME-resolution +1/+0 pump must be applied BEFORE each source's power is read
/// for damage (CR 608.2c: instructions are followed in order; CR 208.1: power is
/// modifiable). Two 3/3 sources become 4/3, so 4 + 4 = 8 damage kills a 2/7
/// recipient. The buff is LOAD-BEARING for lethality: without it the sources deal
/// only 3 + 3 = 6 (< 7) and the recipient survives. Reverting the parser change
/// (clause → `Unimplemented`, no damage) or dropping the pump-then-power ordering
/// leaves the recipient alive and fails this assertion.
#[test]
fn terrific_team_up_same_resolution_pump_is_read_before_damage() {
    let mut scenario = GameScenario::new_n_player(2, 42);
    scenario.at_phase(Phase::PreCombatMain);

    // Two 3/3 sources: base power 3 each (6 total) is NON-lethal vs toughness 7;
    // only the +1/+0 buff (effective power 4 each, 8 total) is lethal.
    let source_a = scenario.add_vanilla(P0, 3, 3);
    let source_b = scenario.add_vanilla(P0, 3, 3);
    let recipient = scenario.add_vanilla(P1, 2, 7);

    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Terrific Team-Up", false, TERRIFIC_TEAM_UP)
        .with_mana_cost(ManaCost::zero())
        .id();

    let mut runner = scenario.build();

    let outcome = runner
        .cast(spell)
        .target_objects(&[source_a, source_b, recipient])
        .resolve();

    // CR 608.2c + CR 208.1: the +1/+0 is applied first, so each source's power is
    // read as 4 at damage resolution: 4 + 4 = 8 >= 7, the 2/7 dies. Base power 6
    // would leave it alive — the pump being read after itself is the discriminator.
    assert_eq!(
        outcome.zone_of(recipient),
        Zone::Graveyard,
        "recipient must die to the BUFFED power (8 total); base power 6 would not be lethal"
    );
}
