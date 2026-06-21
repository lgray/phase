//! Per-opponent iterated exile — Kaya, Spirits' Justice's −2 ability.
//!
//! Printed −2: "Exile target creature you control. For each other player, exile
//! up to one target creature that player controls."
//!
//! This exercises the per-player iterated target/choice primitive
//! (`ChooseFromZone { zone_owner: EachOpponent }`): for EACH OTHER player in
//! APNAP order the controller chooses up to one creature THAT player controls
//! and exiles every chosen permanent (`ChangeZoneAll { TrackedSet }`). The
//! controller is never offered as an iterated chooser (that is the `EachOpponent`
//! vs `EachPlayer` discriminator).
//!
//! The runtime tests drive the REAL `apply` pipeline in a 3-player game: a
//! sorcery carrying the per-other-player clause is cast, the choose loop parks
//! interactive `ChooseFromZoneChoice` prompts, each pick is answered via a real
//! `GameAction::SelectCards`, and every observable zone is engine-produced. The
//! first sentence ("Exile target creature you control") is a pre-existing
//! single-target `ChangeZone` and is asserted at the parser level on the real
//! printed card (the runtime tests isolate the NEW per-other-player seam).
//!
//! CR 101.4: per-player choices are made in APNAP order.
//! CR 102.2: "each other player" excludes the controller.
//! CR 400.7 + CR 608.2c: the implicit "those" mass move acts on exactly the
//! chosen permanents.

use engine::game::scenario::{GameRunner, GameScenario};
use engine::parser::oracle::parse_oracle_text;
use engine::types::ability::{Effect, ZoneOwner};
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

/// The per-other-player sentence of Kaya's −2 (the clause this change adds).
const PER_OTHER_PLAYER: &str =
    "For each other player, exile up to one target creature that player controls.";

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn zone_of(runner: &GameRunner, id: ObjectId) -> Zone {
    runner
        .state()
        .objects
        .get(&id)
        .expect("object present")
        .zone
}

/// Drive the runner until it pauses on a per-player `ChooseFromZoneChoice` or the
/// stack empties.
fn advance_to_choice_or_empty(runner: &mut GameRunner) {
    for _ in 0..200 {
        match &runner.state().waiting_for {
            WaitingFor::ChooseFromZoneChoice { .. } => return,
            WaitingFor::Priority { .. } => {
                if runner.state().stack.is_empty() {
                    return;
                }
                if runner.act(GameAction::PassPriority).is_err() {
                    return;
                }
            }
            _ => return,
        }
    }
}

/// Answer the current per-player `ChooseFromZoneChoice` with `pick`, asserting it
/// is scoped to the spell's controller and offers exactly the iterated player's
/// creatures (never the controller's).
fn answer_pick(runner: &mut GameRunner, expected_chooser: PlayerId, pick: ObjectId) {
    match &runner.state().waiting_for {
        WaitingFor::ChooseFromZoneChoice {
            player,
            cards,
            count,
            ..
        } => {
            assert_eq!(
                *player, expected_chooser,
                "the spell's controller makes every per-other-player pick"
            );
            assert!(*count <= 1, "up to one creature per other player");
            assert!(
                cards.contains(&pick),
                "intended pick {pick:?} must be a legal candidate; offered {cards:?}"
            );
        }
        other => panic!("expected ChooseFromZoneChoice, got {other:?}"),
    }
    runner
        .act(GameAction::SelectCards { cards: vec![pick] })
        .expect("selecting one legal creature must succeed");
}

/// CR 101.4 + CR 102.2 + CR 400.7: For each OTHER player exactly the chosen
/// creature is exiled, while the controller's creatures and each other player's
/// unchosen creatures stay on the battlefield. The controller is NEVER a chooser
/// prompt (the `EachOpponent` discriminator: revert it to `EachPlayer` and the
/// loop would prompt a third time for P0's creatures).
#[test]
fn kaya_minus_two_exiles_one_creature_per_other_player() {
    let mut scenario = GameScenario::new_n_player(3, 4242);
    scenario.at_phase(Phase::PreCombatMain);

    // Controller (P0): a bystander that must remain (proving the loop never
    // touches the controller).
    let p0_bystander = scenario.add_creature(P0, "P0 Bystander", 1, 1).id();
    // P1: a creature to choose-and-exile + one to leave alone.
    let p1_chosen = scenario.add_creature(P1, "P1 Chosen", 3, 3).id();
    let p1_spared = scenario.add_creature(P1, "P1 Spared", 1, 1).id();
    // P2: a single creature to choose-and-exile.
    let p2_chosen = scenario.add_creature(P2, "P2 Chosen", 4, 4).id();

    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Kaya Per-Other-Player Probe", false, PER_OTHER_PLAYER)
        .id();
    let mut runner = scenario.build();
    let card_id = runner.state().objects[&spell].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: spell,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("casting must be accepted");

    // APNAP from P0 means P1 is prompted first, then P2 — the controller (P0) is
    // NEVER an iterated chooser.
    advance_to_choice_or_empty(&mut runner);
    answer_pick(&mut runner, P0, p1_chosen);
    advance_to_choice_or_empty(&mut runner);
    answer_pick(&mut runner, P0, p2_chosen);
    runner.advance_until_stack_empty();

    assert_eq!(
        zone_of(&runner, p1_chosen),
        Zone::Exile,
        "P1's chosen creature must be exiled"
    );
    assert_eq!(
        zone_of(&runner, p2_chosen),
        Zone::Exile,
        "P2's chosen creature must be exiled"
    );
    assert_eq!(
        zone_of(&runner, p1_spared),
        Zone::Battlefield,
        "P1's unchosen creature must remain on the battlefield"
    );
    // CR 102.2: the controller's creature is never an each-other-player candidate.
    assert_eq!(
        zone_of(&runner, p0_bystander),
        Zone::Battlefield,
        "the controller's creature must NOT be exiled by the per-other-player loop"
    );

    assert!(
        runner.state().stack.is_empty(),
        "the per-other-player chain must fully resolve"
    );
    assert!(
        !matches!(
            runner.state().waiting_for,
            WaitingFor::ChooseFromZoneChoice { .. }
        ),
        "no per-player choice should remain pending"
    );
}

/// Negative / discriminator: with only the controller's own creatures, the
/// per-other-player loop offers nothing and exiles nothing — the controller is
/// never prompted (the `EachOpponent`-excludes-controller guarantee). Revert
/// `EachOpponent` → `EachPlayer` and this fails: P0 would be prompted to exile
/// its own creature.
#[test]
fn kaya_minus_two_skips_controller_in_per_other_player_loop() {
    let mut scenario = GameScenario::new_n_player(3, 4243);
    scenario.at_phase(Phase::PreCombatMain);

    let p0_a = scenario.add_creature(P0, "P0 A", 2, 2).id();
    let p0_b = scenario.add_creature(P0, "P0 B", 3, 3).id();
    // P1/P2 have NO creatures.

    let spell = scenario
        .add_spell_to_hand_from_oracle(P0, "Kaya Per-Other-Player Probe B", false, PER_OTHER_PLAYER)
        .id();
    let mut runner = scenario.build();
    let card_id = runner.state().objects[&spell].card_id;
    runner
        .act(GameAction::CastSpell {
            object_id: spell,
            card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Auto,
        })
        .expect("cast accepted");
    runner.advance_until_stack_empty();

    // No choose prompt was ever raised for the controller, so both P0 creatures
    // remain. With `EachPlayer` the loop would have offered P0's creatures.
    assert_eq!(zone_of(&runner, p0_a), Zone::Battlefield);
    assert_eq!(zone_of(&runner, p0_b), Zone::Battlefield);
    assert!(
        !matches!(
            runner.state().waiting_for,
            WaitingFor::ChooseFromZoneChoice { .. }
        ),
        "the controller must never be prompted in the each-OTHER-player loop"
    );
}

/// Parser structural guard for the full printed −2: the first sentence is a
/// single-target self-exile `ChangeZone`, and the second sentence is the
/// `ChooseFromZone { EachOpponent }` → `ChangeZoneAll { TrackedSet }` chain this
/// change adds. Asserts the −2 carries zero `Unimplemented` nodes.
#[test]
fn kaya_minus_two_parses_to_each_opponent_exile_chain() {
    let oracle = "\u{2212}2: Exile target creature you control. For each other player, \
         exile up to one target creature that player controls.";
    let parsed = parse_oracle_text(
        oracle,
        "Kaya, Spirits' Justice",
        &[],
        &["Planeswalker".to_string()],
        &["Kaya".to_string()],
    );
    let dbg = format!("{parsed:#?}");
    assert!(
        !dbg.contains("Unimplemented"),
        "the −2 must parse with zero Unimplemented nodes; got:\n{dbg}"
    );
    // The −2 activated ability's first effect is the self-exile; its sub_ability
    // chain reaches a `ChooseFromZone { EachOpponent }`.
    let minus_two = parsed
        .abilities
        .iter()
        .find(|a| {
            matches!(
                &*a.effect,
                Effect::ChangeZone {
                    destination: Zone::Exile,
                    ..
                }
            )
        })
        .expect("−2 self-exile ChangeZone present");
    // Walk the sub_ability chain to find the per-other-player choose.
    let mut node = minus_two.sub_ability.as_deref();
    let mut found_each_opponent = false;
    while let Some(def) = node {
        if let Effect::ChooseFromZone {
            zone_owner: ZoneOwner::EachOpponent,
            up_to: true,
            ..
        } = &*def.effect
        {
            found_each_opponent = true;
        }
        node = def.sub_ability.as_deref();
    }
    assert!(
        found_each_opponent,
        "the −2 chain must include a ChooseFromZone {{ EachOpponent, up_to }};\n{dbg}"
    );
}
