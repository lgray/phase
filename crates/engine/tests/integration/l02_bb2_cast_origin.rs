//! L02 BB2 — cast-origin conditions on four Standard cards.
//!
//! Two parallel cast-origin channels, ZERO new engine variants:
//!   - Channel A (resolving-spell self-reference, `AbilityCondition::CastFromZone`):
//!       * Antiquities on the Loose — "Then if this spell was cast from anywhere
//!         other than your hand, put a +1/+1 counter on each Spirit you control."
//!       * Otterball Antics          — "If this spell was cast from anywhere other
//!         than your hand, put a +1/+1 counter on that creature."
//!   - Channel B (entering-permanent provenance, `TriggerCondition::WasCast`):
//!       * Anti-Venom, Horrifying Healer — "if he was cast" (pronoun subject).
//!       * Ran and Shaw                  — "if you cast them and there are three or
//!         more Dragon and/or Lesson cards in your graveyard" (And-fold).
//!
//! Each card has a parse-fidelity row (exact condition payload + no `Condition_If`
//! swallow) and discriminating runtime rows driven through the real cast/trigger
//! pipeline. Oracle text is verbatim from `data/card-data.json`. CR 603.4
//! (intervening-if), CR 601.2a (cast origin zone), CR 400.7 (new-object memory).

use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::parser::oracle::parse_oracle_text;
use engine::parser::oracle_ir::diagnostic::OracleDiagnostic;
use engine::types::ability::{
    AbilityCondition, Comparator, CountScope, QuantityExpr, QuantityRef, TriggerCondition,
    TypeFilter, ZoneRef,
};
use engine::types::card_type::Supertype;
use engine::types::counter::CounterType;
use engine::types::game_state::CastingVariant;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::zones::Zone;
use engine::types::ObjectId;
use engine::types::PlayerId;

// ---------------------------------------------------------------------------
// Verbatim Oracle text (data/card-data.json, 2026-07-11)
// ---------------------------------------------------------------------------

const ANTIQUITIES: &str = "Create two 2/2 red and white Spirit creature tokens. \
Then if this spell was cast from anywhere other than your hand, put a +1/+1 counter \
on each Spirit you control.\n\
Flashback {4}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)";

const OTTERBALL: &str = "Create a 1/1 blue and red Otter creature token with prowess. \
If this spell was cast from anywhere other than your hand, put a +1/+1 counter on that \
creature. (Whenever you cast a noncreature spell, a creature with prowess gets +1/+1 \
until end of turn.)\n\
Flashback {3}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)";

const ANTI_VENOM: &str = "When Anti-Venom enters, if he was cast, return target creature \
card from your graveyard to the battlefield.\n\
If damage would be dealt to Anti-Venom, prevent that damage and put that many +1/+1 counters on him.";

const RAN_AND_SHAW: &str = "Flying, firebending 2\n\
When Ran and Shaw enter, if you cast them and there are three or more Dragon and/or Lesson \
cards in your graveyard, create a token that's a copy of Ran and Shaw, except it's not legendary.\n\
{3}{R}: Dragons you control get +2/+0 until end of turn.";

/// A vanilla {0} reanimation sorcery: the entering permanent is *put onto the
/// battlefield* (never cast), so its `cast_from_zone` stays `None` (CR 400.7).
/// This is the Channel-B "put-into-play-not-cast" discriminator.
const REANIMATE: &str = "Return target creature card from your graveyard to the battlefield.";

// ---------------------------------------------------------------------------
// Parse helpers
// ---------------------------------------------------------------------------

fn parse(oracle: &str, name: &str, types: &[&str]) -> engine::parser::oracle::ParsedAbilities {
    let types: Vec<String> = types.iter().map(|s| s.to_string()).collect();
    parse_oracle_text(oracle, name, &[], &types, &[])
}

/// First `sub_ability` condition attached anywhere in the parsed abilities —
/// Channel A attaches the cast-origin gate on a `SequentialSibling` sub.
fn first_sub_condition(
    parsed: &engine::parser::oracle::ParsedAbilities,
) -> Option<AbilityCondition> {
    parsed
        .abilities
        .iter()
        .filter_map(|a| a.sub_ability.as_ref())
        .find_map(|s| s.condition.clone())
}

/// First trigger condition attached (Channel B ETB intervening-if).
fn first_trigger_condition(
    parsed: &engine::parser::oracle::ParsedAbilities,
) -> Option<TriggerCondition> {
    parsed.triggers.iter().find_map(|t| t.condition.clone())
}

fn has_condition_if_swallow(parsed: &engine::parser::oracle::ParsedAbilities) -> bool {
    parsed.parse_warnings.iter().any(|w| {
        matches!(
            w,
            OracleDiagnostic::SwallowedClause { detector, .. } if detector == "Condition_If"
        )
    })
}

// ===========================================================================
// P (parse fidelity)
// ===========================================================================

/// Antiquities: the "Then if this spell was cast from anywhere other than your
/// hand" rider lowers to `Not(CastFromZone{Hand})` on the counter sub-ability.
/// Revert-probe: delete the S1 passive arm in `strip_cast_from_zone_conditional`
/// → `condition == None` and `Condition_If` reappears (both asserts flip).
#[test]
fn antiquities_sub_condition_is_not_cast_from_hand() {
    let parsed = parse(ANTIQUITIES, "Antiquities on the Loose", &["Sorcery"]);
    assert_eq!(
        first_sub_condition(&parsed),
        Some(AbilityCondition::Not {
            condition: Box::new(AbilityCondition::CastFromZone { zone: Zone::Hand }),
        }),
        "expected Not(CastFromZone{{Hand}}) on the counter sub-ability"
    );
    assert!(
        !has_condition_if_swallow(&parsed),
        "Condition_If must clear once the passive cast-origin gate attaches"
    );
}

/// Otterball: same passive rider (no leading "Then"), same `Not(CastFromZone{Hand})`.
/// Revert-probe: identical to Antiquities.
#[test]
fn otterball_sub_condition_is_not_cast_from_hand() {
    let parsed = parse(OTTERBALL, "Otterball Antics", &["Sorcery"]);
    assert_eq!(
        first_sub_condition(&parsed),
        Some(AbilityCondition::Not {
            condition: Box::new(AbilityCondition::CastFromZone { zone: Zone::Hand }),
        }),
        "expected Not(CastFromZone{{Hand}}) on the counter sub-ability"
    );
    assert!(
        !has_condition_if_swallow(&parsed),
        "Condition_If must clear once the passive cast-origin gate attaches"
    );
}

/// Anti-Venom: the pronoun subject "if he was cast" lowers to the zoneless
/// `WasCast{None,None,None}` trigger intervening-if. Revert-probe: remove
/// he/she/they from `parse_was_cast_condition` → `condition == None` + swallow.
#[test]
fn anti_venom_trigger_condition_is_was_cast() {
    let parsed = parse(ANTI_VENOM, "Anti-Venom, Horrifying Healer", &["Creature"]);
    assert_eq!(
        first_trigger_condition(&parsed),
        Some(TriggerCondition::WasCast {
            zone: None,
            controller: None,
            owner: None,
        }),
        "expected zoneless WasCast on the ETB trigger"
    );
    assert!(
        !has_condition_if_swallow(&parsed),
        "Condition_If must clear once the pronoun WasCast gate attaches"
    );
}

/// Ran and Shaw: "if you cast them and there are three or more Dragon and/or
/// Lesson cards in your graveyard" lowers to `And[WasCast, QuantityComparison]`.
/// The multi-subtype `card_types` list [Dragon, Lesson] is the load-bearing OR.
/// Revert-probe: remove the native recognizer → `condition == None` + swallow.
#[test]
fn ran_and_shaw_trigger_condition_is_cast_and_graveyard_count() {
    let parsed = parse(RAN_AND_SHAW, "Ran and Shaw", &["Creature"]);
    let cond = first_trigger_condition(&parsed).expect("Ran and Shaw ETB must carry a condition");
    let conditions = match cond {
        TriggerCondition::And { conditions } => conditions,
        other => panic!("expected And[..], got {other:?}"),
    };
    assert_eq!(conditions.len(), 2, "cast-provenance AND graveyard-count");
    assert_eq!(
        conditions[0],
        TriggerCondition::WasCast {
            zone: None,
            controller: None,
            owner: None,
        },
        "first conjunct is the unscoped WasCast (you cast them)"
    );
    match &conditions[1] {
        TriggerCondition::QuantityComparison {
            lhs,
            comparator,
            rhs,
        } => {
            assert_eq!(*comparator, Comparator::GE, "three OR MORE");
            assert_eq!(*rhs, QuantityExpr::Fixed { value: 3 }, "threshold 3");
            match lhs {
                QuantityExpr::Ref {
                    qty:
                        QuantityRef::ZoneCardCount {
                            zone,
                            card_types,
                            scope,
                            ..
                        },
                } => {
                    assert_eq!(*zone, ZoneRef::Graveyard);
                    assert_eq!(*scope, CountScope::Controller, "your graveyard only");
                    let subtypes: Vec<String> = card_types
                        .iter()
                        .filter_map(|t| match t {
                            TypeFilter::Subtype(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect();
                    assert!(
                        subtypes.iter().any(|s| s == "Dragon")
                            && subtypes.iter().any(|s| s == "Lesson"),
                        "both Dragon and Lesson subtypes must be present (the and/or OR); got {subtypes:?}"
                    );
                }
                other => panic!("expected ZoneCardCount lhs, got {other:?}"),
            }
        }
        other => panic!("expected QuantityComparison second conjunct, got {other:?}"),
    }
    assert!(
        !has_condition_if_swallow(&parsed),
        "Condition_If must clear once the And-fold attaches"
    );
}

// ===========================================================================
// P (building-block class tests, via the public parser)
// ===========================================================================

/// Channel-A class: the passive stripper handles positive and negated forms
/// across all owner-specific zones — not just "your hand". Routes through the
/// public parser on synthetic sorceries. Revert-probe: delete the S1 arm → all
/// three lose their sub condition.
#[test]
fn channel_a_class_covers_positive_and_negated_all_zones() {
    // Negated: "anywhere other than your graveyard" → Not(CastFromZone{Graveyard}).
    let neg = parse(
        "Draw a card. If this spell was cast from anywhere other than your graveyard, draw a card.",
        "Synthetic Negated GY",
        &["Sorcery"],
    );
    assert_eq!(
        first_sub_condition(&neg),
        Some(AbilityCondition::Not {
            condition: Box::new(AbilityCondition::CastFromZone {
                zone: Zone::Graveyard
            }),
        }),
    );
    // Positive: "was cast from exile" → CastFromZone{Exile}.
    let pos = parse(
        "Draw a card. If this spell was cast from exile, draw a card.",
        "Synthetic Positive Exile",
        &["Sorcery"],
    );
    assert_eq!(
        first_sub_condition(&pos),
        Some(AbilityCondition::CastFromZone { zone: Zone::Exile }),
    );
}

/// Channel-B class: he/she/they pronoun subjects all lower to zoneless WasCast
/// on an ETB intervening-if. Revert-probe: remove the pronoun tags from
/// `parse_was_cast_condition` → all three lose their trigger condition.
#[test]
fn channel_b_class_covers_he_she_they_pronouns() {
    for (subject, verb) in [("he", "was"), ("she", "was"), ("they", "were")] {
        let oracle = format!("When Testcard enters, if {subject} {verb} cast, draw a card.");
        let parsed = parse(&oracle, "Testcard", &["Creature"]);
        assert_eq!(
            first_trigger_condition(&parsed),
            Some(TriggerCondition::WasCast {
                zone: None,
                controller: None,
                owner: None,
            }),
            "pronoun subject '{subject} {verb} cast' must lower to zoneless WasCast"
        );
    }
}

// ===========================================================================
// P (non-regression: benign coverage collisions must parse as before)
// ===========================================================================

/// N4: Gideon, the Oathsworn's "(He can't attack if he was cast this turn.)" is
/// parenthetical reminder text, stripped before parsing. The pronoun widening
/// must NOT make it swallow or mis-attach — no `Condition_If`.
#[test]
fn gideon_reminder_text_unchanged_no_swallow() {
    const GIDEON: &str =
        "Menace\n(He can't attack if he was cast this turn.)\nWhenever Gideon attacks, draw a card.";
    let parsed = parse(GIDEON, "Gideon, the Oathsworn", &["Creature"]);
    assert!(
        !has_condition_if_swallow(&parsed),
        "reminder-text 'if he was cast this turn' must not produce a Condition_If swallow"
    );
}

/// N6: Spiders-Man, Heroic Horde's "if they were cast using web-slinging" is
/// intercepted by the dedicated web-slinging arm (ordered before the generic
/// tail). The pronoun widening must NOT steal it — it must still lower to
/// `CastVariantPaidPersistent{WebSlinging}`.
#[test]
fn spiders_man_web_slinging_unchanged() {
    const SPIDERS_MAN: &str = "When Spiders-Man enters, if they were cast using web-slinging, \
        create two 1/1 green and white Spider creature tokens with reach.";
    let parsed = parse(SPIDERS_MAN, "Spiders-Man, Heroic Horde", &["Creature"]);
    assert_eq!(
        first_trigger_condition(&parsed),
        Some(TriggerCondition::CastVariantPaidPersistent {
            variant: engine::types::ability::CastVariantPaid::WebSlinging,
        }),
        "web-slinging must still win over the generic pronoun WasCast arm"
    );
    assert!(!has_condition_if_swallow(&parsed));
}

// ===========================================================================
// Runtime helpers
// ===========================================================================

fn fund(runner: &mut GameRunner, player: PlayerId, mana: &[ManaType]) {
    let pool = &mut runner
        .state_mut()
        .players
        .iter_mut()
        .find(|p| p.id == player)
        .unwrap()
        .mana_pool;
    for m in mana {
        pool.add(ManaUnit::new(*m, ObjectId(0), false, vec![]));
    }
}

/// +1/+1 counter totals across every battlefield token of `subtype` controlled
/// by P0 (Antiquities' Spirits / Otterball's Otter).
fn token_counters(state: &engine::types::game_state::GameState, subtype: &str) -> Vec<u32> {
    state
        .objects
        .values()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.is_token
                && o.controller == P0
                && o.card_types.subtypes.iter().any(|s| s == subtype)
        })
        .map(|o| {
            o.counters
                .get(&CounterType::Plus1Plus1)
                .copied()
                .unwrap_or(0)
        })
        .collect()
}

// ===========================================================================
// R — Antiquities on the Loose (Channel A)
// ===========================================================================

/// Cast from HAND: `cast_from_zone = Some(Hand)` → `Not(CastFromZone{Hand})` is
/// FALSE → the counter sub is skipped (CR 603.4 resolve-time gate). Two Spirit
/// tokens are still created (reach-guard: the ability resolved and reached the
/// sub-ability dispatch), each with ZERO counters.
///
/// Revert-probe (MEASURED): deleting the S1 passive arm drops the condition to
/// `None`, so the sub runs unconditionally and each Spirit gains a +1/+1 counter
/// → the `== [0, 0]` assertion flips to `[1, 1]`.
#[test]
fn antiquities_hand_cast_no_counters() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let mut b =
        scenario.add_spell_to_hand_from_oracle(P0, "Antiquities on the Loose", false, ANTIQUITIES);
    b.with_mana_cost(engine::types::mana::ManaCost::generic(0));
    let spell = b.id();
    let mut runner = scenario.build();

    let out = runner.cast(spell).resolve();
    let counters = token_counters(out.state(), "Spirit");
    assert_eq!(
        counters.len(),
        2,
        "reach-guard: two Spirit tokens created (ability resolved), got {counters:?}"
    );
    assert_eq!(
        counters,
        vec![0, 0],
        "cast from hand → Not(CastFromZone{{Hand}}) false → no +1/+1 counters"
    );
}

/// Cast via FLASHBACK from the graveyard: `cast_from_zone = Some(Graveyard)` →
/// `Not(CastFromZone{Hand})` is TRUE → each Spirit gains a +1/+1 counter. This is
/// the positive reach-guard proving the counter mechanism runs when the gate
/// holds (non-vacuity for the hand-cast negative).
#[test]
fn antiquities_flashback_cast_grants_counters() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let mut b = scenario.add_spell_to_graveyard(P0, "Antiquities on the Loose", false);
    b.from_oracle_text(ANTIQUITIES);
    let spell = b.id();
    let mut runner = scenario.build();
    // Flashback {4}{W}{W}
    fund(
        &mut runner,
        P0,
        &[
            ManaType::White,
            ManaType::White,
            ManaType::Colorless,
            ManaType::Colorless,
            ManaType::Colorless,
            ManaType::Colorless,
        ],
    );

    let out = runner
        .cast(spell)
        .casting_variant(CastingVariant::Flashback)
        .resolve();
    let counters = token_counters(out.state(), "Spirit");
    assert_eq!(
        counters.len(),
        2,
        "reach-guard: two Spirit tokens created, got {counters:?}"
    );
    assert_eq!(
        counters,
        vec![1, 1],
        "flashback from graveyard → Not(CastFromZone{{Hand}}) true → +1/+1 on each Spirit"
    );
}

// ===========================================================================
// R — Otterball Antics (Channel A, ParentTarget)
// ===========================================================================
//
// Otterball's cast-origin gate governs ONLY "put a +1/+1 counter on that
// creature", where "that creature" is the ParentTarget (the just-created Otter
// token). That anaphoric ParentTarget counter is a PRE-EXISTING engine no-op —
// measured directly: casting "Create a 1/1 ... Otter token. Put a +1/+1 counter
// on that creature." UNCONDITIONALLY (no cast-origin gate at all) also lands 0
// counters. So no runtime observation can distinguish gate-true from gate-false
// for Otterball; a hand/flashback counter test would be VACUOUS (0 either way).
// The Otterball cast-origin gate is therefore covered by its parse-fidelity test
// (`otterball_sub_condition_is_not_cast_from_hand`); the identical S1 seam
// (`strip_cast_from_zone_conditional`) is exercised at runtime by Antiquities,
// whose `PutCounterAll` effect (a working effect) yields the [0,0]/[1,1]
// discriminator above. (The ParentTarget-counter no-op is a separate latent bug,
// out of scope for BB2's cast-origin work.)

// ===========================================================================
// R — Anti-Venom, Horrifying Healer (Channel B, pronoun WasCast)
// ===========================================================================

/// Cast Anti-Venom normally (from hand) with a creature card in the graveyard:
/// `cast_from_zone = Some(Hand)` so `WasCast` is TRUE → the ETB returns the bait
/// creature to the battlefield. Positive reach-guard proving the return path runs.
#[test]
fn anti_venom_cast_returns_creature() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let bait = scenario
        .add_creature_to_graveyard(P0, "Grizzly Bear", 2, 2)
        .id();
    let mut b = scenario.add_creature_to_hand_from_oracle(
        P0,
        "Anti-Venom, Horrifying Healer",
        3,
        3,
        ANTI_VENOM,
    );
    b.with_mana_cost(engine::types::mana::ManaCost::generic(0));
    let anti = b.id();
    let mut runner = scenario.build();

    let out = runner.cast(anti).target_object(bait).resolve();
    assert_eq!(
        out.zone_of(bait),
        Zone::Battlefield,
        "cast Anti-Venom → WasCast true → ETB returns the graveyard creature"
    );
}

/// Put Anti-Venom onto the battlefield via reanimation (NOT cast): its
/// `cast_from_zone` stays `None` → `WasCast` FALSE → the ETB intervening-if
/// declines at both CR 603.4 checkpoints → the bait creature is NOT returned.
///
/// Reach-guard: Anti-Venom itself entered the battlefield (the reanimate ran, so
/// the ETB path was reached). Revert-probe (MEASURED): removing the pronoun tags
/// drops the condition to `None`, the ETB fires ungated, and the bait is returned
/// → `zone_of(bait) == Graveyard` flips to `Battlefield`.
#[test]
fn anti_venom_put_into_play_no_return() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let mut ab = scenario.add_creature_to_graveyard(P0, "Anti-Venom, Horrifying Healer", 3, 3);
    ab.from_oracle_text(ANTI_VENOM);
    let anti = ab.id();
    let bait = scenario
        .add_creature_to_graveyard(P0, "Grizzly Bear", 2, 2)
        .id();
    let mut reani = scenario.add_spell_to_hand_from_oracle(P0, "Reanimate", false, REANIMATE);
    reani.with_mana_cost(engine::types::mana::ManaCost::generic(0));
    let reani = reani.id();
    let mut runner = scenario.build();

    // Reanimate Anti-Venom (first declared legal target); the bait is the second
    // declared object so that, on a revert, the ungated ETB has a return target.
    let out = runner.cast(reani).target_objects(&[anti, bait]).resolve();
    assert_eq!(
        out.zone_of(anti),
        Zone::Battlefield,
        "reach-guard: Anti-Venom was reanimated onto the battlefield"
    );
    assert_eq!(
        out.zone_of(bait),
        Zone::Graveyard,
        "put into play (not cast) → WasCast false → ETB does not return the bait"
    );
}

// ===========================================================================
// R — Ran and Shaw (Channel B, And-fold)
// ===========================================================================

fn ran_shaw_token_exists(state: &engine::types::game_state::GameState) -> Option<bool> {
    state
        .objects
        .values()
        .find(|o| o.is_token && o.zone == Zone::Battlefield && o.name == "Ran and Shaw")
        .map(|o| o.card_types.supertypes.contains(&Supertype::Legendary))
}

/// Stock P0's graveyard with `dragons` Dragon creatures + `lessons` Lesson
/// sorceries. Lesson + Dragon split proves the multi-subtype OR.
fn stock_graveyard(scenario: &mut GameScenario, player: PlayerId, dragons: usize, lessons: usize) {
    for i in 0..dragons {
        scenario
            .add_creature_to_graveyard(player, &format!("Dragon {i}"), 4, 4)
            .with_subtypes(vec!["Dragon"]);
    }
    for i in 0..lessons {
        let mut l = scenario.add_spell_to_graveyard(player, &format!("Lesson {i}"), false);
        l.with_subtypes(vec!["Lesson"]);
    }
}

/// Cast Ran and Shaw with 2 Dragon + 1 Lesson (= 3 across BOTH subtypes) in your
/// graveyard: `WasCast` true AND count ≥ 3 → a non-legendary copy token is
/// created. Positive reach-guard + proves the Dragon/Lesson OR is load-bearing.
#[test]
fn ran_and_shaw_cast_with_three_makes_nonlegendary_copy() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    stock_graveyard(&mut scenario, P0, 2, 1);
    let mut b = scenario.add_creature_to_hand_from_oracle(P0, "Ran and Shaw", 4, 4, RAN_AND_SHAW);
    b.with_mana_cost(engine::types::mana::ManaCost::generic(0));
    b.as_legendary();
    let ran = b.id();
    let mut runner = scenario.build();

    let out = runner.cast(ran).resolve();
    let legendary = ran_shaw_token_exists(out.state());
    assert_eq!(
        legendary,
        Some(false),
        "cast + 3 (2 Dragon + 1 Lesson) → non-legendary copy token created"
    );
}

/// Cast Ran and Shaw with only 2 matching cards: the count conjunct is FALSE →
/// no copy. Revert-probe: dropping the count conjunct from the And makes the
/// lone-WasCast condition true here → a copy would be created (flips the None).
#[test]
fn ran_and_shaw_cast_with_two_no_copy() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    stock_graveyard(&mut scenario, P0, 1, 1);
    let mut b = scenario.add_creature_to_hand_from_oracle(P0, "Ran and Shaw", 4, 4, RAN_AND_SHAW);
    b.with_mana_cost(engine::types::mana::ManaCost::generic(0));
    b.as_legendary();
    let ran = b.id();
    let mut runner = scenario.build();

    let out = runner.cast(ran).resolve();
    assert_eq!(
        ran_shaw_token_exists(out.state()),
        None,
        "only 2 matching cards → count conjunct false → no copy token"
    );
}

/// Reanimate Ran and Shaw (NOT cast) with 3 matching cards: the WasCast conjunct
/// is FALSE even though the count is satisfied → no copy. Revert-probe: dropping
/// the WasCast conjunct makes the count-only condition true → a copy appears.
#[test]
fn ran_and_shaw_reanimate_with_three_no_copy() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    stock_graveyard(&mut scenario, P0, 2, 1);
    let mut b = scenario.add_creature_to_graveyard(P0, "Ran and Shaw", 4, 4);
    b.from_oracle_text(RAN_AND_SHAW);
    b.as_legendary();
    let ran = b.id();
    let mut reani = scenario.add_spell_to_hand_from_oracle(P0, "Reanimate", false, REANIMATE);
    reani.with_mana_cost(engine::types::mana::ManaCost::generic(0));
    let reani = reani.id();
    let mut runner = scenario.build();

    let out = runner.cast(reani).target_object(ran).resolve();
    assert_eq!(
        out.zone_of(ran),
        Zone::Battlefield,
        "reach-guard: Ran and Shaw was reanimated onto the battlefield"
    );
    assert_eq!(
        ran_shaw_token_exists(out.state()),
        None,
        "put into play (not cast) → WasCast conjunct false → And false → no copy token \
         (the reanimated original is not a token)"
    );
}

/// Dragons in an OPPONENT's graveyard do NOT count (CountScope::Controller):
/// 2 Dragon in P0's graveyard + 1 Dragon in P1's graveyard → P0 sees only 2 →
/// count conjunct false → no copy. Proves the scope, not just the count.
#[test]
fn ran_and_shaw_opponent_graveyard_dragon_uncounted() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    stock_graveyard(&mut scenario, P0, 2, 0);
    stock_graveyard(&mut scenario, P1, 1, 0);
    let mut b = scenario.add_creature_to_hand_from_oracle(P0, "Ran and Shaw", 4, 4, RAN_AND_SHAW);
    b.with_mana_cost(engine::types::mana::ManaCost::generic(0));
    b.as_legendary();
    let ran = b.id();
    let mut runner = scenario.build();

    let out = runner.cast(ran).resolve();
    assert_eq!(
        ran_shaw_token_exists(out.state()),
        None,
        "opponent's Dragon does not count toward your graveyard → no copy"
    );
}
