//! Issue #6857 — event-less producers publish the population they froze.
//!
//! `Effect::PumpAll`, `Effect::GoadAll` and `Effect::GiveControl` affect objects
//! without moving them and without emitting any per-object event, so before this
//! change the chain publish site fell through to the `ZoneChanged` harvest and
//! published an EMPTY tracked set. CR 611.2c makes that the WRONG set, not just
//! an unhelpful one: the set of objects a resolution-generated continuous effect
//! modifies is fixed when the effect begins. A following "Untap those creatures"
//! (CR 701.26b) therefore bound nothing — Jeskai Ascendancy's loot-and-untap did
//! not untap.
//!
//! Every row here is measured on the shipped tree. The suite carries its own
//! anti-vacuity instruments:
//!
//!   * `known_changed_control_*` — the row that MUST differ from the old
//!     behaviour. If it ever passes trivially the whole file is meaningless.
//!   * `negative_control_*` — a chain with no consumer at all: the arms must
//!     invent no publish.
//!   * `leg1_witness_*` / `leg2_witness_*` — each pins one leg of
//!     `is_sole_chain_producer`. Deleting that leg from the engine must turn the
//!     named test RED; a leg whose deletion changes nothing is vacuous.
//!   * the PRESERVED rows assert the publish did NOT widen a filter or reach a
//!     grant that declares its own target.
//!
//! Oracle text is verbatim at the branch base unless a deviation is called out
//! in the test's doc comment.

use engine::game::combat::AttackTarget;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::card_type::CoreType;
use engine::types::game_state::{BattlefieldEntryRecord, CastPaymentMode, GameState, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;

/// Every tracked set, id-ordered, each as a sorted list of raw object ids.
fn tracked_sets(state: &GameState) -> Vec<Vec<u64>> {
    let mut sets: Vec<(u64, Vec<u64>)> = state
        .tracked_object_sets
        .iter()
        .map(|(id, members)| {
            let mut ids: Vec<u64> = members.iter().map(|o| o.0).collect();
            ids.sort_unstable();
            (id.0, ids)
        })
        .collect();
    sets.sort();
    sets.into_iter().map(|(_, ids)| ids).collect()
}

/// The single chain tracked set's contents. Panics if the resolution published
/// more than one set — every row in this file is a one-producer chain, and a
/// second set would mean the guard let two producers through.
fn published_set(state: &GameState) -> Vec<u64> {
    let sets = tracked_sets(state);
    assert!(
        sets.len() <= 1,
        "expected at most one tracked set in a single-producer chain, got {sets:?}"
    );
    sets.into_iter().next().unwrap_or_default()
}

fn ids(objects: &[ObjectId]) -> Vec<u64> {
    let mut raw: Vec<u64> = objects.iter().map(|o| o.0).collect();
    raw.sort_unstable();
    raw
}

fn tapped(state: &GameState, id: ObjectId) -> bool {
    state.objects[&id].tapped
}

/// Debug rendering of every transient continuous effect that applies to `id`.
/// The continuous-effect list is the observable for the `GenericEffect` grants
/// (MustAttack / CantBlock / keyword grants) a mass head feeds; a
/// tracked-set-only projection previously scored a non-fix as a fix.
fn effects_on(state: &GameState, id: ObjectId) -> Vec<String> {
    state
        .transient_continuous_effects
        .iter()
        .filter(|tce| tce.affected == engine::types::ability::TargetFilter::SpecificObject { id })
        .map(|tce| format!("{:?}", tce.modifications))
        .collect()
}

fn grant_lands_on(state: &GameState, id: ObjectId, needle: &str) -> bool {
    effects_on(state, id).iter().any(|m| m.contains(needle))
}

// ===========================================================================
// CONTROLS
// ===========================================================================

/// KNOWN-CHANGED CONTROL — issue #6857's own card, cast as the printed card.
/// Jeskai Ascendancy's first trigger is `PumpAll -> SetTapState
/// { target: TrackedSet, Untap }`. Before the fix the published set was empty
/// and the creature stayed TAPPED; it must now be published and untapped.
///
/// The real enchantment is used deliberately rather than a synthesized trigger
/// body: this is the control for #6857, so it should exercise #6857's card, on
/// its real trigger path, with the second (loot) trigger present. If this row
/// ever reads the same as the pre-fix engine, every "identical" reading
/// elsewhere in this file is meaningless.
#[test]
fn known_changed_control_jeskai_ascendancy_untaps_the_creatures_it_pumped() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let mine = scenario.add_creature(P0, "Mine", 3, 3).id();
    scenario.add_enchantment_from_oracle(
        P0,
        "Jeskai Ascendancy",
        "Whenever you cast a noncreature spell, creatures you control get +1/+1 until end of turn. Untap those creatures.\nWhenever you cast a noncreature spell, you may draw a card. If you do, discard a card.",
    );
    let bolt = scenario.add_bolt_to_hand(P0);
    let mut runner: GameRunner = scenario.build();
    runner.state_mut().objects.get_mut(&mine).unwrap().tapped = true;
    // Both of the printed card's triggers fire on the same cast, so the engine
    // parks an APNAP ordering prompt (CR 603.3b) that the one-shot cast driver
    // does not handle. Drive the cast by hand rather than trimming the card to
    // dodge the prompt: the point of this control is that it uses #6857's card.
    let card_id = runner.state().objects[&bolt].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: bolt,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("casting the bolt should be legal");
    // Drain the prompts the printed card creates: the bolt's own target, the
    // CR 603.3b ordering prompt for the two simultaneous cast triggers, and the
    // second trigger's "you may draw a card" (declined — this row is about the
    // first trigger).
    for _ in 0..32 {
        match runner.state().waiting_for.clone() {
            WaitingFor::TargetSelection { .. } => {
                runner
                    .act(GameAction::ChooseTarget {
                        target: Some(TargetRef::Player(P1)),
                    })
                    .expect("the bolt targets a player");
            }
            WaitingFor::OrderTriggers { triggers, .. } => {
                runner
                    .act(GameAction::OrderTriggers {
                        order: (0..triggers.len()).collect(),
                    })
                    .expect("CR 603.3b: order the two cast triggers");
            }
            WaitingFor::OptionalEffectChoice { .. } => {
                runner
                    .act(GameAction::DecideOptionalEffect { accept: false })
                    .expect("CR 608.2d: decline the loot trigger");
            }
            WaitingFor::Priority { .. } if !runner.state().stack.is_empty() => {
                runner
                    .act(GameAction::PassPriority)
                    .expect("passing priority resolves the top of the stack");
            }
            _ => break,
        }
    }
    runner.advance_until_stack_empty();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[mine]));
    assert!(
        !tapped(runner.state(), mine),
        "CR 701.26b: 'those creatures' names the pumped population, so it untaps"
    );
    assert_eq!(runner.state().objects[&mine].power, Some(4), "pump applied");
}

/// NEGATIVE CONTROL — a mass pump with no anaphor at all. The publish gate never
/// fires, so the new arms must invent no set.
#[test]
fn negative_control_mass_pump_without_a_consumer_publishes_nothing() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let mine = scenario.add_creature(P0, "Mine", 3, 3).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Bare Pump",
            true,
            "Creatures you control get +1/+1 until end of turn.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    assert!(tracked_sets(runner.state()).is_empty());
    assert!(runner.state().chain_tracked_set_id.is_none());
    assert_eq!(runner.state().objects[&mine].power, Some(4), "pump applied");
}

// ===========================================================================
// `PumpAll` — FIX rows
// ===========================================================================

/// War Flare's second sentence pair — the plainest `PumpAll -> SetTapState
/// { TrackedSet }` shape in the corpus.
#[test]
fn war_flare_untaps_the_creatures_it_pumped() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 2, 2).id();
    let b = scenario.add_creature(P0, "Mine B", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "War Flare",
            true,
            "Creatures you control get +2/+1 until end of turn. Untap those creatures.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    for id in [a, b] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[a, b]));
    assert!(!tapped(runner.state(), a) && !tapped(runner.state(), b));
}

/// Gleam of Resistance — the REAL card, including its basic landcycling line, so
/// the fixture cannot be a simplified proxy of the shape under test (the
/// `Typecycling` keyword on the built object is the discriminator).
///
/// The opponent's creature staying tapped is the load-bearing half: it proves
/// the published population kept the head filter's `controller: You` rather than
/// being widened to the whole battlefield.
#[test]
fn gleam_of_resistance_untaps_only_the_creatures_its_controller_filter_named() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 2, 2).id();
    let b = scenario.add_creature(P0, "Mine B", 2, 2).id();
    let theirs = scenario.add_creature(P1, "Theirs", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Gleam of Resistance",
            true,
            "Creatures you control get +1/+2 until end of turn. Untap those creatures.\nBasic landcycling {1}{W} ({1}{W}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    for id in [a, b, theirs] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    assert!(
        format!("{:?}", runner.state().objects[&spell].keywords).contains("Typecycling"),
        "fixture guard: the full printed card was built, not a pump-only proxy"
    );
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[a, b]));
    assert!(!tapped(runner.state(), a) && !tapped(runner.state(), b));
    assert!(
        tapped(runner.state(), theirs),
        "CR 611.2c: the frozen population is the head filter's, and it says 'you control'"
    );
}

/// Zealous Display's untap carries `condition: Not(IsYourTurn)`, so it is cast on
/// the OPPONENT's turn. Cast on your own turn the sub never executes and the row
/// is vacuously identical to the pre-fix engine.
#[test]
fn zealous_display_untaps_on_the_opponents_turn() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 2, 2).id();
    let b = scenario.add_creature(P0, "Mine B", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Zealous Display",
            true,
            "Creatures you control get +2/+0 until end of turn. If it's not your turn, untap those creatures.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    // Fixture setup: hand the turn to the opponent so `Not(IsYourTurn)` holds.
    runner.state_mut().active_player = P1;
    runner.state_mut().priority_player = P0;
    runner.state_mut().waiting_for = engine::types::game_state::WaitingFor::Priority { player: P0 };
    for id in [a, b] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    assert_ne!(
        runner.state().active_player,
        P0,
        "fixture guard: on your own turn the untap sub never runs and this row is vacuous"
    );
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[a, b]));
    assert!(!tapped(runner.state(), a) && !tapped(runner.state(), b));
}

/// Motivated Pony's attack trigger. Its untap is gated on
/// `BattlefieldEntriesThisTurn { Food } >= 1`, so a Food entry is stamped into
/// the ledger — without it the branch never executes and the row is vacuous.
/// Only ATTACKING creatures may enter the published set, which is what
/// keeps the `Attacking` property in the head filter honest.
#[test]
fn motivated_pony_untaps_only_the_attacking_creatures_it_pumped() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let pony = scenario
        .add_creature_from_oracle(
            P0,
            "Motivated Pony",
            3,
            3,
            "Trample, haste\nWhenever this creature attacks, attacking creatures get +1/+1 until end of turn. If a Food entered the battlefield under your control this turn, untap those creatures and they get an additional +2/+2 until end of turn.",
        )
        .id();
    let buddy = scenario.add_creature(P0, "Buddy", 2, 2).id();
    let home = scenario.add_creature(P0, "Stays Home", 2, 2).id();
    let mut runner: GameRunner = scenario.build();
    runner.state_mut().objects.get_mut(&buddy).unwrap().keywords =
        vec![engine::types::keywords::Keyword::Haste];
    // Fixture setup: a Food entered the battlefield this turn, so the
    // intervening-if holds and the untap branch actually runs.
    runner
        .state_mut()
        .battlefield_entries_this_turn
        .push(BattlefieldEntryRecord {
            object_id: ObjectId(9_999),
            name: "Food".to_string(),
            core_types: vec![CoreType::Artifact],
            subtypes: vec!["Food".to_string()],
            supertypes: vec![],
            colors: vec![],
            keywords: vec![],
            controller: P0,
        });
    runner.advance_to_combat();
    runner
        .declare_attackers(&[
            (pony, AttackTarget::Player(P1)),
            (buddy, AttackTarget::Player(P1)),
        ])
        .expect("fixture guard: the attack trigger must actually fire");
    runner.advance_until_stack_empty();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[pony, buddy]));
    assert!(!tapped(runner.state(), pony) && !tapped(runner.state(), buddy));
    assert!(
        !published_set(runner.state()).contains(&home.0),
        "CR 611.2c: the non-attacker was never in the frozen population"
    );
}

/// Suicidal Charge — the mass head feeds a `GenericEffect { affected:
/// ParentTarget, MustAttack }` coercion instead of an untap. Before the fix the
/// opponent's creatures were shrunk but not coerced: half the card did nothing.
#[test]
fn suicidal_charge_coerces_the_creatures_it_shrank() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P1, "Theirs A", 3, 3).id();
    let b = scenario.add_creature(P1, "Theirs B", 2, 2).id();
    let src = scenario
        .add_enchantment_from_oracle(
            P0,
            "Suicidal Charge",
            "Sacrifice this enchantment: Creatures your opponents control get -1/-1 until end of turn. Those creatures attack this turn if able.",
        )
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.activate(src, 0).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[a, b]));
    for id in [a, b] {
        assert!(
            grant_lands_on(runner.state(), id, "MustAttack"),
            "CR 608.2c: 'those creatures' names the shrunk population"
        );
    }
    assert_eq!(runner.state().objects[&a].power, Some(2), "shrink applied");
}

// ===========================================================================
// `PumpAll` — PRESERVED rows
// ===========================================================================

/// Elvish Elegy: `Mill -> PumpAll -> ChangeZoneAll { TrackedSetFiltered }`.
///
/// LEG-1 ROW. The `Mill` already published the milled cards, so the mass pump is
/// not the antecedent of "from among the milled cards" and must not join the
/// set. (`leg1_witness_surge_to_victory_*` is the sharper revert probe for the
/// same leg; this row covers the same leg on a `PumpAll` whose own enumeration
/// happens to be empty.)
#[test]
fn elvish_elegy_keeps_the_milled_set_free_of_the_graveyard_pump() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_library_top(P0, &["Lib Elf", "Lib Land", "Lib Bear"]);
    scenario.with_graveyard(P0, &["Yard Creature"]);
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Elvish Elegy",
            false,
            "Mill three cards, then each creature card in your graveyard perpetually gets +1/+1. You may put an Elf or land card from among the milled cards into your hand.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.cast(spell).resolve();

    let set = published_set(runner.state());
    assert_eq!(
        set.len(),
        3,
        "the milled cards, and only those: got {set:?}"
    );
}

/// Heroic Charge, cast UNKICKED. Its trample grant sits behind the kicked
/// condition, so publishing the pumped population must not make it execute.
#[test]
fn heroic_charge_unkicked_publishes_without_granting_trample() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 3, 3).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Heroic Charge",
            false,
            "Kicker {1}{R} (You may pay an additional {1}{R} as you cast this spell.)\nCreatures you control get +2/+1 until end of turn. If this spell was kicked, those creatures also gain trample until end of turn.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(
        runner.state().objects[&a].power,
        Some(5),
        "non-vacuity: the pump ran, so the chain really resolved"
    );
    assert!(
        !grant_lands_on(runner.state(), a, "Trample"),
        "the kicked-only grant must not fire on an unkicked cast"
    );
}

/// Valley Rally with its condition removed, so the targeted grant actually
/// executes. The head is a population and the grant DECLARES its own target: the
/// grant node's own targets must win over the published set.
///
/// DISCLOSED DEVIATION: the printed card gates the grant on `AdditionalCostPaid`
/// (the gift). That branch never runs in this harness, which would make the row
/// vacuous, so the condition is dropped and everything else kept.
#[test]
fn valley_rally_grant_binds_its_own_target_not_the_published_population() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 3, 3).id();
    let b = scenario.add_creature(P0, "Mine B", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Valley Rally Grant Path",
            true,
            "Creatures you control get +2/+0 until end of turn. Target creature you control gains first strike until end of turn.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.cast(spell).target_objects(&[a]).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(
        runner.state().objects[&b].power,
        Some(4),
        "non-vacuity: the mass pump reached the non-targeted creature"
    );
    assert!(grant_lands_on(runner.state(), a, "FirstStrike"));
    assert!(
        !grant_lands_on(runner.state(), b, "FirstStrike"),
        "CR 608.2c: a grant with its own declared target does not read the frozen population"
    );
}

// ===========================================================================
// `GoadAll`
// ===========================================================================

/// Kaima, the Fractured Calm — the consumer is a COUNT
/// (`FilteredTrackedSetSize`), not an anaphor, so the observable is Kaima's
/// counter total. Only the ENCHANTED opponent creature may be counted, which is
/// what proves the head filter's `HasAttachment { Aura }` property survived into
/// the published population.
///
/// DISCLOSED DEVIATION: given as an activated ability so `SelfRef` denotes the
/// permanent and the chain runs without waiting for the printed trigger.
#[test]
fn kaima_counts_only_the_enchanted_creature_it_goaded() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let victim = scenario.add_creature(P1, "Enchanted Victim", 3, 3).id();
    let plain = scenario.add_creature(P1, "Plain Victim", 2, 2).id();
    let aura = scenario
        .add_enchantment_from_oracle(P0, "Kaima Aura", "Enchant creature")
        .id();
    let kaima = scenario
        .add_creature_from_oracle(
            P0,
            "Kaima Body",
            3,
            3,
            "{T}: Goad each creature your opponents control that's enchanted by an Aura you control. Put a +1/+1 counter on Kaima Body for each creature goaded this way.",
        )
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.attach_as_bestowed_aura(aura, victim);
    runner.activate(kaima, 0).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[victim]));
    assert!(
        !published_set(runner.state()).contains(&plain.0),
        "CR 611.2c: the unenchanted creature was never in the frozen population"
    );
    assert_eq!(
        runner.state().objects[&kaima]
            .counters
            .get(&engine::types::counter::CounterType::Plus1Plus1)
            .copied(),
        Some(1),
        "one creature goaded this way"
    );
}

/// Taunt from the Rampart — `GoadAll` feeding a `GenericEffect { affected:
/// ParentTarget, CantBlock }`.
#[test]
fn taunt_from_the_rampart_stops_the_creatures_it_goaded_from_blocking() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let theirs = scenario.add_creature(P1, "Theirs A", 3, 3).id();
    let mine = scenario.add_creature(P0, "Mine", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Taunt from the Rampart",
            true,
            "Goad all creatures your opponents control. Until your next turn, those creatures can't block. (Until your next turn, those creatures attack each combat if able and attack a player other than you if able.)",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[theirs]));
    assert!(grant_lands_on(runner.state(), theirs, "CantBlock"));
    assert!(
        !grant_lands_on(runner.state(), mine, "CantBlock"),
        "CR 701.15a: only the goaded creatures are named"
    );
}

// ===========================================================================
// `GiveControl`
// ===========================================================================

/// Domineering Will — the authority test for `GiveControl`. "Those creatures"
/// names the DECLARED TARGETS (CR 608.2c), and a target the recipient already
/// controls emits no `ControllerChanged`, so an event-harvest authority would
/// leave it tapped. Here the recipient is P0 and one target is already P0's, so
/// the two candidate authorities disagree and the event one fails.
#[test]
fn domineering_will_untaps_a_target_the_recipient_already_controlled() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let theirs = scenario.add_creature(P1, "Theirs", 2, 2).id();
    let already_mine = scenario.add_creature(P0, "Already Mine", 1, 1).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Domineering Will",
            true,
            "Target player gains control of up to three target nonattacking creatures until end of turn. Untap those creatures. They block this turn if able.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    for id in [theirs, already_mine] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    runner
        .cast(spell)
        .target_player(P0)
        .target_objects(&[theirs, already_mine])
        .resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[theirs, already_mine]));
    assert!(!tapped(runner.state(), theirs));
    assert!(
        !tapped(runner.state(), already_mine),
        "CR 608.2c: a declared target that changed no controller is still one of 'those creatures'"
    );
}

/// Coveted Falcon's turn-face-up trigger body: `GiveControl -> Draw
/// { TrackedSetSize }`. The observable is cards drawn, which was 0 before the
/// fix.
#[test]
fn coveted_falcon_draws_for_each_permanent_handed_over() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Give A", 1, 1).id();
    scenario.add_card_to_library_top(P0, "Library Card A");
    scenario.add_card_to_library_top(P0, "Library Card B");
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Falcon Trigger Body",
            true,
            "Target opponent gains control of any number of target permanents you control. Draw a card for each one they gained control of this way.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    let before = runner.state().players[0].hand.len();
    let outcome = runner.cast(spell).target_objects(&[a]).resolve();

    assert_eq!(published_set(outcome.state()), ids(&[a]));
    assert_eq!(
        outcome.state().players[0].hand.len(),
        before,
        "one card drawn, and the spell itself left the hand"
    );
    assert_eq!(
        outcome.state().objects[&a].controller,
        P1,
        "non-vacuity: control actually changed"
    );
}

// ===========================================================================
// LEG WITNESSES — each pins one leg of `is_sole_chain_producer`
// ===========================================================================

/// LEG-1 WITNESS (the sharper of the two: it flips a set's CONTENTS, not just a
/// boolean). Surge to Victory exiles a card and then mass-pumps; "the exiled
/// card" names the exile, not the creatures. The `ChangeZone` ancestor already
/// published, so the mass pump must decline.
///
/// Deleting `no_earlier_producer` makes the pumped creature join the set.
#[test]
fn leg1_witness_surge_to_victory_binds_the_exiled_card_not_the_pumped_creatures() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    scenario.add_creature(P0, "Alpha", 2, 2);
    let graveyard_card = scenario.add_spell_to_graveyard(P0, "Shock", true).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Surge to Victory",
            false,
            "Exile target instant or sorcery card from your graveyard. Creatures you control get +X/+0 until end of turn, where X is that card's mana value. Whenever a creature you control deals combat damage to a player this turn, copy the exiled card. You may cast the copy without paying its mana cost.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    runner
        .cast(spell)
        .target_objects(&[graveyard_card])
        .resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(
        published_set(runner.state()),
        ids(&[graveyard_card]),
        "CR 608.2c: the anaphor names the exile, so the pumped creature must stay out"
    );
    assert_eq!(
        runner.state().objects[&graveyard_card].zone,
        engine::types::zones::Zone::Exile,
        "non-vacuity: the exile really happened"
    );
}

/// LEG-2 WITNESS. Outlaws' Fury pumps FIRST and exiles afterwards, so the later
/// exile is the antecedent of "you may play that card" and the mass pump must
/// decline even though nothing published before it.
///
/// Deleting `!later_node_is_publisher_position` makes the pumped creatures join
/// the exiled card's set, and the play permission would then cover creatures.
#[test]
fn leg2_witness_outlaws_fury_binds_the_later_exile_not_the_pumped_creatures() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let alpha = scenario.add_creature(P0, "Alpha", 2, 2).id();
    scenario
        .add_creature(P0, "Rogue Pal", 1, 1)
        .with_subtypes(vec!["Rogue"]);
    scenario.with_library_top(P0, &["Lib A", "Lib B"]);
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Outlaws' Fury",
            false,
            "Creatures you control get +2/+0 until end of turn. If you control an outlaw, exile the top card of your library. Until the end of your next turn, you may play that card. (Assassins, Mercenaries, Pirates, Rogues, and Warlocks are outlaws.)",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    let set = published_set(runner.state());
    assert_eq!(set.len(), 1, "exactly the exiled card: got {set:?}");
    assert!(
        !set.contains(&alpha.0),
        "CR 608.2c: a head followed by another producer is not the antecedent"
    );
    assert_eq!(
        runner.state().objects[&alpha].power,
        Some(4),
        "non-vacuity: the mass pump ran, it simply did not publish"
    );
}

// ===========================================================================
// PARSER HALF — the implicit-pronoun anaphor ("Untap them.")
// ===========================================================================

/// PARSER KNOWN-CHANGED CONTROL — Rallying Roar. Verbatim. Its untap is an
/// implicit pronoun, which the spell-body default lowers to `ParentTarget`; only
/// the parser rewrite turns it into `TrackedSet(0)`. If this passes without the
/// rewrite, every other parser row here is meaningless.
#[test]
fn parser_control_rallying_roar_untaps_the_creatures_it_pumped() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 2, 2).id();
    let b = scenario.add_creature(P0, "Mine B", 2, 2).id();
    let theirs = scenario.add_creature(P1, "Theirs", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Rallying Roar",
            true,
            "Creatures you control get +1/+1 until end of turn. Untap them.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    for id in [a, b, theirs] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[a, b]));
    assert!(!tapped(runner.state(), a) && !tapped(runner.state(), b));
    assert!(tapped(runner.state(), theirs), "controller filter survives");
}

/// Rally to Battle — same shape, different numbers; kept as its own row because
/// the roster is per-card.
#[test]
fn rally_to_battle_untaps_the_creatures_it_pumped() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 2, 2).id();
    let b = scenario.add_creature(P0, "Mine B", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Rally to Battle",
            true,
            "Creatures you control get +1/+3 until end of turn. Untap them.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    for id in [a, b] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    runner.cast(spell).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[a, b]));
    assert!(!tapped(runner.state(), a) && !tapped(runner.state(), b));
    assert_eq!(runner.state().objects[&a].toughness, Some(5));
}

/// Great Oak Guardian's ETB trigger — the population is `target player`'s
/// creatures, so targeting the OPPONENT makes the anaphor's scope observable:
/// their creatures untap, mine do not.
/// Great Oak Guardian's ETB trigger — the population is `target player`'s
/// creatures, so targeting the OPPONENT makes the anaphor's scope observable:
/// their creatures untap and mine do not. A rewrite that bound "them" to the
/// source or to the parent target could not produce this split.
#[test]
fn great_oak_guardian_untaps_the_targeted_players_creatures_only() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let theirs = scenario.add_creature(P1, "Theirs", 2, 2).id();
    let mine = scenario.add_creature(P0, "Mine", 2, 2).id();
    let spell = scenario
        .add_creature_to_hand_from_oracle(
            P0,
            "Great Oak Guardian",
            4,
            5,
            "Flash (You may cast this spell any time you could cast an instant.)\nReach\nWhen this creature enters, creatures target player controls get +2/+2 until end of turn. Untap them.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    for id in [theirs, mine] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    runner.cast(spell).target_player(P1).resolve();
    runner.advance_until_stack_empty();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[theirs]));
    assert!(!tapped(runner.state(), theirs));
    assert_eq!(runner.state().objects[&theirs].power, Some(4), "pumped");
    assert!(
        tapped(runner.state(), mine),
        "CR 611.2c: the frozen population is the TARGETED player's creatures"
    );
}

/// The General — the same anaphor under an activated ability with a
/// self-exile cost, i.e. the population head is not the ability source.
#[test]
fn the_general_untaps_the_creatures_it_pumped() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 2, 2).id();
    let src = scenario
        .add_enchantment_from_oracle(
            P0,
            "The General",
            "Exile The General: Creatures you control get +1/+1 until end of turn. Untap them.",
        )
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.state_mut().objects.get_mut(&a).unwrap().tapped = true;
    runner.activate(src, 0).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[a]));
    assert!(!tapped(runner.state(), a));
    assert_eq!(runner.state().objects[&a].power, Some(3), "pumped");
}

/// Essence of Antiquity — a `GenericEffect` head (a keyword grant, not a pump)
/// feeding the same implicit-pronoun untap. This is the third publisher class in
/// the parser predicate, and the one with no `PumpAll` involved at all.
///
/// DISCLOSED DEVIATION: the printed card fires this off a Disguise
/// turn-face-up trigger, which this harness cannot drive. The body is given as a
/// `{T}` activated ability on a creature, which keeps every element the row
/// turns on — the same broadcast `affected` filter, the same implicit-pronoun
/// untap, and a real permanent source. `{T}` also taps the source, so the source
/// joining the untapped population ("creatures you control" includes it) is
/// directly observable.
#[test]
fn essence_of_antiquity_untaps_the_creatures_it_granted_hexproof() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let a = scenario.add_creature(P0, "Mine A", 2, 2).id();
    let src = scenario
        .add_creature_from_oracle(
            P0,
            "Essence Body",
            1,
            10,
            "{T}: Creatures you control gain hexproof until end of turn. Untap them.",
        )
        .id();
    let mut runner: GameRunner = scenario.build();
    runner.state_mut().objects.get_mut(&a).unwrap().tapped = true;
    runner.activate(src, 0).resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(published_set(runner.state()), ids(&[a, src]));
    assert!(!tapped(runner.state(), a));
    assert!(
        !tapped(runner.state(), src),
        "the source is one of 'creatures you control', so its own {{T}} tap is undone"
    );
    assert!(grant_lands_on(runner.state(), a, "Hexproof"));
}

/// Valley Floodcaller's cast trigger.
///
/// KNOWN, BOUNDED GAP (issue #7451): the grant's four-subtype filter
/// ("Birds, Frogs, Otters, and Rats") is misparsed upstream of this change —
/// only the last subtype survives into the pumped population. This row therefore
/// asserts the INVARIANT this PR owns, which holds regardless of that bug:
/// **the untapped set is exactly the pumped set is exactly the published set.**
/// Before the fix nothing untapped at all, so the row is strictly closer to
/// correct; when #7451 is fixed the pumped set widens and this test follows it
/// without needing to change, because it asserts the identity and not a
/// hard-coded population.
#[test]
fn valley_floodcaller_untaps_exactly_the_creatures_it_pumped() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let bird = scenario
        .add_creature(P0, "Birdy", 1, 1)
        .with_subtypes(vec!["Bird"])
        .id();
    let frog = scenario
        .add_creature(P0, "Froggy", 1, 1)
        .with_subtypes(vec!["Frog"])
        .id();
    let otter = scenario
        .add_creature(P0, "Ottery", 1, 1)
        .with_subtypes(vec!["Otter"])
        .id();
    let rat = scenario
        .add_creature(P0, "Ratty", 1, 1)
        .with_subtypes(vec!["Rat"])
        .id();
    let bear = scenario.add_creature(P0, "Beary", 2, 2).id();
    scenario.add_creature_from_oracle(
        P0,
        "Valley Floodcaller",
        2,
        2,
        "Flash\nYou may cast noncreature spells as though they had flash.\nWhenever you cast a noncreature spell, Birds, Frogs, Otters, and Rats you control get +1/+1 until end of turn. Untap them.",
    );
    let bolt = scenario.add_bolt_to_hand(P0);
    let subjects = [bird, frog, otter, rat, bear];
    let mut runner: GameRunner = scenario.build();
    for id in subjects {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    let base_power: Vec<Option<i32>> = subjects
        .iter()
        .map(|id| runner.state().objects[id].power)
        .collect();
    runner.cast(bolt).target_player(P1).resolve();
    runner.advance_until_stack_empty();
    evaluate_layers(runner.state_mut());

    let pumped: Vec<u64> = subjects
        .iter()
        .zip(&base_power)
        .filter(|(id, before)| runner.state().objects[*id].power != **before)
        .map(|(id, _)| id.0)
        .collect();
    let untapped: Vec<u64> = subjects
        .iter()
        .filter(|id| !tapped(runner.state(), **id))
        .map(|id| id.0)
        .collect();

    assert!(
        !pumped.is_empty(),
        "non-vacuity: the trigger must have pumped something, or the identity below is trivial"
    );
    assert_eq!(pumped, untapped, "untapped set == pumped set");
    assert_eq!(published_set(runner.state()), pumped, "== published set");
    assert!(
        !untapped.contains(&bear.0),
        "the plain creature is outside the grant's population under any reading of it"
    );
}

/// Trystan's Command — PRESERVED, and a regression sentinel for the two-regime
/// law rather than a fix.
///
/// The card is MODAL (choose two of four sibling abilities), not a chain — the
/// engine resolves the chosen modes in sequence, so the publish gate sees the
/// later mode's consumer. With the destroy mode chosen alongside the pump mode,
/// the destroy publishes first, `is_sole_chain_producer`'s leg 1 declines the
/// mass pump, and the anaphor resolves against the destroyed creature — which
/// untaps nothing. That is a KNOWN, BOUNDED gap that predates this PR: before
/// the parser rewrite the implicit pronoun bound elsewhere and also untapped
/// nothing. The row exists to prove the behaviour did not get WORSE, so do not
/// "simplify" it away on the grounds that it asserts a non-untap.
///
/// STRUCTURAL CONSEQUENCE — this is not "one unmeasured mode pair". MEASURED:
/// the card is `min_choices: 2, max_choices: 2` over `mode_count: 4`, so a
/// companion mode is ALWAYS chosen; and two of the three possible companions
/// publish before mode 4 resolves — destroy (this row) and token copy (the row
/// below, `[0, 3]`, published set = the created token). INFERRED, not measured:
/// the graveyard-return companion publishes too, because it moves cards to hand
/// and the `_ =>` arm of the publish switch harvests `ZoneChanged`. If that
/// inference is wrong, mode 4 is fixable for exactly one of three pairs.
/// **On the two measured pairs, Trystan's Command mode 4 cannot be fixed while
/// the publish gate is chain-wide rather than mode-scoped.**
/// CR 700.2 is the lever: modes are separate instructions, so a sibling mode's
/// `Destroy` arguably should not count as an "earlier producer" for mode 4's
/// anaphor at all. Fixing that means scoping the gate to the mode, not weakening
/// this test.
///
/// Two measured side-facts, recorded because they are easy to misread:
///  * the TRACKED SET does change (empty -> `[victim]`). The parser rewrite
///    creates a `TrackedSet` consumer where there was none, so the pre-existing
///    `Destroy` publish arm now fires. The BOARD is unaffected, because the only
///    consumer is an untap aimed at a creature that is already in the graveyard.
///  * this row is a SECOND leg-1 witness: with `no_earlier_producer` deleted the
///    set becomes `[victim, mine]`, i.e. the mass pump joins the destroy's set.
///
/// The `tapped` assertion below therefore pins a rules-INCORRECT outcome on
/// purpose, as a no-regression sentinel. When the mode-scoping fix lands it must
/// be flipped to `!tapped`, not deleted.
#[test]
fn trystans_command_pump_mode_is_unchanged_when_an_earlier_mode_publishes() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let victim = scenario.add_creature(P1, "Victim", 2, 2).id();
    let mine = scenario.add_creature(P0, "Mine", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Trystan's Command",
            false,
            "Choose two —\n• Create a token that's a copy of target Elf you control.\n• Return one or two target permanent cards from your graveyard to your hand.\n• Destroy target creature or enchantment.\n• Creatures target player controls get +3/+3 until end of turn. Untap them.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    for id in [victim, mine] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    runner
        .cast(spell)
        .modes(&[2, 3])
        .target_objects(&[victim])
        .target_player(P0)
        .resolve();
    evaluate_layers(runner.state_mut());

    assert_eq!(
        published_set(runner.state()),
        ids(&[victim]),
        "CR 608.2c: the earlier mode's destroy is the live antecedent"
    );
    assert_eq!(
        runner.state().objects[&victim].zone,
        engine::types::zones::Zone::Graveyard,
        "non-vacuity: the destroy mode really executed"
    );
    assert_eq!(
        runner.state().objects[&mine].power,
        Some(5),
        "non-vacuity: the pump mode really executed too"
    );
    assert!(
        tapped(runner.state(), mine),
        "unchanged from before this PR — see the doc comment"
    );
}

/// The token-copy companion mode, measured: the second half of the "any pair
/// preempts mode 4" claim in the row above.
///
/// Modes 1 and 4 (`[0, 3]`). The copy token's creation publishes first, leg 1
/// declines the mass pump, and the anaphor binds the TOKEN — so the pumped
/// creatures stay tapped even though the pump itself ran. Same bounded gap as
/// the destroy pair, reached through a different publishing arm, which is the
/// point: the gate is chain-wide, so WHICH earlier mode published is irrelevant.
#[test]
fn trystans_command_token_copy_mode_also_preempts_the_pump_anaphor() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let elf = scenario
        .add_creature(P0, "Elf Pal", 1, 1)
        .with_subtypes(vec!["Elf"])
        .id();
    let mine = scenario.add_creature(P0, "Mine", 2, 2).id();
    let spell = scenario
        .add_spell_to_hand_from_oracle(
            P0,
            "Trystan's Command",
            false,
            "Choose two —\n• Create a token that's a copy of target Elf you control.\n• Return one or two target permanent cards from your graveyard to your hand.\n• Destroy target creature or enchantment.\n• Creatures target player controls get +3/+3 until end of turn. Untap them.",
        )
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner: GameRunner = scenario.build();
    for id in [elf, mine] {
        runner.state_mut().objects.get_mut(&id).unwrap().tapped = true;
    }
    runner
        .cast(spell)
        .modes(&[0, 3])
        .target_objects(&[elf])
        .target_player(P0)
        .resolve();
    evaluate_layers(runner.state_mut());

    let token: Vec<u64> = runner
        .state()
        .battlefield
        .iter()
        .filter(|id| **id != elf && **id != mine)
        .map(|id| id.0)
        .collect();
    assert_eq!(token.len(), 1, "non-vacuity: the copy mode really ran");
    assert_eq!(
        published_set(runner.state()),
        token,
        "CR 608.2c: the earlier mode's token is the live antecedent"
    );
    assert_eq!(
        runner.state().objects[&mine].power,
        Some(5),
        "non-vacuity: the pump mode really executed too"
    );
    assert!(
        tapped(runner.state(), mine),
        "unchanged from before this PR — see the doc comment above"
    );
}
