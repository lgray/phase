//! GitHub issue #7234 — Cumulative upkeep must pay typed source-counter
//! effect costs after card-data/save-state deserialization.

use engine::game::scenario::{GameScenario, P0};
use engine::types::ability::{AbilityCost, Effect, EffectKind, QuantityExpr, TargetFilter};
use engine::types::actions::GameAction;
use engine::types::counter::CounterType;
use engine::types::events::GameEvent;
use engine::types::game_state::WaitingFor;
use engine::types::keywords::Keyword;
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const ABOROTH_ORACLE: &str =
    "Cumulative upkeep—Put a -1/-1 counter on this creature. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)";

const VORINCLEX_ORACLE: &str = "Trample, haste\n\
If you would put one or more counters on a permanent or player, put twice that many of each of those kinds of counters on that permanent or player instead.\n\
If an opponent would put one or more counters on a permanent or player, they put half that many of each of those kinds of counters on that permanent or player instead, rounded down.";

const DOC_SAMSON_ORACLE: &str = "If you would put one or more counters on a permanent you control, put that many plus one of each of those kinds of counters on that permanent instead.\n\
{T}: Add X mana of any one color, where X is Doc Samson's power.";

/// CR 702.24a: Card-data and saved games use the externally tagged keyword
/// form. A typed Aboroth effect cost must not be replaced by a zero-mana cost.
#[test]
fn cumulative_upkeep_typed_effect_cost_survives_deserialization() {
    let keyword: Keyword = serde_json::from_str(
        r#"{"CumulativeUpkeep":{"type":"EffectCost","effect":{"type":"PutCounter","counter_type":"M1M1","count":{"type":"Fixed","value":1},"target":{"type":"SelfRef"}}}}"#,
    )
    .expect("typed CumulativeUpkeep payload deserializes");

    assert!(matches!(
        keyword,
        Keyword::CumulativeUpkeep(AbilityCost::EffectCost { effect })
            if matches!(
                effect.as_ref(),
                Effect::PutCounter {
                    counter_type: CounterType::Minus1Minus1,
                    count: QuantityExpr::Fixed { value: 1 },
                    target: TargetFilter::SelfRef,
                }
            )
    ));
}

/// CR 702.24a: Aboroth's effect-as-cost is paid once per age counter. With one
/// pre-existing age counter, the upkeep tick makes two and paying the prompt
/// must place two -1/-1 counters while keeping Aboroth on the battlefield.
#[test]
fn aboroth_cumulative_upkeep_scales_and_pays_source_counter_effect_cost() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::Untap);
    let aboroth = scenario
        .add_creature_from_oracle(P0, "Aboroth", 9, 9, ABOROTH_ORACLE)
        .id();
    let mut runner = scenario.build();
    runner
        .state_mut()
        .objects
        .get_mut(&aboroth)
        .expect("Aboroth exists")
        .counters
        .insert(CounterType::Age, 1);

    runner.auto_advance_to_main_phase();
    runner.advance_until_stack_empty();

    match &runner.state().waiting_for {
        WaitingFor::UnlessPayment { cost, .. } => assert!(matches!(
            cost,
            AbilityCost::EffectCost {
                effect,
            } if matches!(
                effect.as_ref(),
                Effect::PutCounter {
                    counter_type: CounterType::Minus1Minus1,
                    count: QuantityExpr::Fixed { value: 2 },
                    target: TargetFilter::SelfRef,
                }
            )
        )),
        other => panic!("expected Aboroth's cumulative-upkeep payment prompt, got {other:?}"),
    }

    runner
        .act(GameAction::PayUnlessCost { pay: true })
        .expect("Aboroth's counter cost is payable");

    let aboroth_object = runner
        .state()
        .objects
        .get(&aboroth)
        .expect("Aboroth remains");
    assert_eq!(aboroth_object.zone, Zone::Battlefield);
    assert_eq!(aboroth_object.counters.get(&CounterType::Age), Some(&2));
    assert_eq!(
        aboroth_object.counters.get(&CounterType::Minus1Minus1),
        Some(&2),
        "paying the cumulative cost must place one -1/-1 counter for each age counter"
    );
}

/// CR 702.24a + CR 616.1 + CR 118.12: Aboroth's cumulative upkeep is an
/// `AbilityCost::EffectCost` — the *second* unless-payment park site, the
/// sibling of Ward's `AbilityCost::GetPlayerCounters` site. When two printed
/// replacement effects both modify the counter placement the payer is paying
/// WITH, CR 616.1 makes the payer order them, and the payment PARKS mid-cost on
/// `PendingCostMoveResume::CounterAdditionUnlessPayment`.
///
/// CR 616.1 genuinely applies here because the two modifications do not commute
/// on the placement's count: Vorinclex is multiplicative (`Times{2}`) and Doc
/// Samson is additive (`Plus{1}`), so a 3-counter cost settles at 3 → 6 → 7 with
/// Vorinclex first and 3 → 4 → 8 with Doc Samson first.
///
/// CR 118.12: whichever order is chosen, the cost is PAID — the "if they don't"
/// clause "checks whether the player chose to pay an optional cost … regardless
/// of what events actually occurred", and the choice was latched at
/// `PayUnlessCost { pay: true }`, before the replacement pipeline was consulted.
/// CR 118.11 corroborates: a cost whose payment actions were modified is still
/// paid. So the guarded "sacrifice it" never happens.
///
/// `GameEvent::EffectResolved { kind: EffectKind::Sacrifice, .. }` here means
/// "the cumulative-upkeep ability finished resolving", NOT that Aboroth was
/// sacrificed. The paired `zone == Zone::Battlefield` assertion is what proves
/// the permanent survived.
#[test]
fn aboroth_cumulative_upkeep_payment_ordered_by_two_replacements_is_still_paid() {
    // Both CR 616.1 orderings are driven inline in one test function rather than
    // through a parameterised helper: the ordering IS the axis under test, and a
    // single function keeps the ordering assertions beside the counter totals
    // they explain.
    for (index, expected_minus_counters) in [(0usize, 7u32), (1usize, 8u32)] {
        let mut scenario = GameScenario::new();
        scenario.at_phase(Phase::Untap);
        scenario.add_creature_from_oracle(
            P0,
            "Vorinclex, Monstrous Raider",
            6,
            6,
            VORINCLEX_ORACLE,
        );
        scenario.add_creature_from_oracle(
            P0,
            "Doc Samson, Super Psychiatrist",
            3,
            6,
            DOC_SAMSON_ORACLE,
        );
        let aboroth = scenario
            .add_creature_from_oracle(P0, "Aboroth", 9, 9, ABOROTH_ORACLE)
            .id();
        let mut runner = scenario.build();

        runner.auto_advance_to_main_phase();
        runner.advance_until_stack_empty();

        // CR 702.24a + CR 616.1: the AGE counter placement is itself modified by
        // both permanents, so the very first prompt is the ordering choice.
        // Answering it with Vorinclex first makes the age total 1 → 2 → 3.
        assert_replacement_choice_between_vorinclex_and_doc_samson(
            &runner,
            "the age-counter placement must raise the CR 616.1 ordering prompt",
        );
        runner
            .act(GameAction::ChooseReplacement { index: 0 })
            .expect("ordering the age-counter replacements must be legal");
        runner.advance_until_stack_empty();

        // Reach guard: the cost really is the `EffectCost` shape (the second park
        // site), and it scaled with the modified age total.
        match &runner.state().waiting_for {
            WaitingFor::UnlessPayment { player, cost, .. } => {
                assert_eq!(
                    *player, P0,
                    "the cumulative-upkeep payer is Aboroth's controller"
                );
                assert!(
                    matches!(
                        cost,
                        AbilityCost::EffectCost { effect } if matches!(
                            effect.as_ref(),
                            Effect::PutCounter {
                                counter_type: CounterType::Minus1Minus1,
                                count: QuantityExpr::Fixed { value: 3 },
                                target: TargetFilter::SelfRef,
                            }
                        )
                    ),
                    "the modified age total must scale the effect cost to three -1/-1 counters, got {cost:?}"
                );
            }
            other => panic!("expected Aboroth's cumulative-upkeep payment prompt, got {other:?}"),
        }

        runner
            .act(GameAction::PayUnlessCost { pay: true })
            .expect("choosing to pay Aboroth's counter cost must be legal");

        // CR 616.1: the payment itself parks mid-cost on a replacement-ordering
        // choice. This is the assertion that proves the `EffectCost` park site is
        // reached at all.
        assert!(
            runner.state().pending_cost_move_resume.is_some(),
            "paying the effect cost must park the unless-payment continuation"
        );
        assert_replacement_choice_between_vorinclex_and_doc_samson(
            &runner,
            "the payment's own counter placement must raise the CR 616.1 ordering prompt",
        );

        let result = runner
            .act(GameAction::ChooseReplacement { index })
            .expect("ordering the payment's replacements must be legal");

        // CR 118.12: the resume settles through the PAID epilogue, so the
        // cumulative-upkeep ability finishes resolving and the whole reducer
        // step's event buffer survives. Membership, not order: at index 1 the two
        // `ReplacementApplied` events arrive Doc Samson first.
        assert!(
            result.events.iter().any(|event| matches!(
                event,
                GameEvent::EffectResolved {
                    kind: EffectKind::Sacrifice,
                    source_id,
                    ..
                } if *source_id == aboroth
            )),
            "a paid cumulative upkeep must emit the guarded ability's EffectResolved, got {:?}",
            result.events
        );
        assert!(
            result.events.iter().any(|event| matches!(
                event,
                GameEvent::CounterAdded {
                    object_id,
                    counter_type: CounterType::Minus1Minus1,
                    count,
                    ..
                } if *object_id == aboroth && *count == expected_minus_counters
            )),
            "the counters the payer actually paid with must reach the event log, got {:?}",
            result.events
        );

        runner.advance_until_stack_empty();

        let aboroth_object = runner
            .state()
            .objects
            .get(&aboroth)
            .expect("Aboroth remains a known object");
        assert_eq!(
            aboroth_object.zone,
            Zone::Battlefield,
            "CR 118.12: a paid cumulative upkeep must not sacrifice the permanent"
        );
        assert_eq!(
            aboroth_object.counters.get(&CounterType::Minus1Minus1),
            Some(&expected_minus_counters),
            "ordering index {index} must settle the modified counter total"
        );
        assert_eq!(
            aboroth_object.counters.get(&CounterType::Age),
            Some(&3),
            "the age placement was modified by both replacements (1 → 2 → 3)"
        );
    }
}

/// Reach guard shared by both CR 616.1 prompts in the row above: the prompt is
/// the payer's, and it names both printed replacement sources. Asserted by
/// MEMBERSHIP rather than by index, because the candidate order differs between
/// the two prompts — an ordering drift must fail loudly, not silently re-index.
fn assert_replacement_choice_between_vorinclex_and_doc_samson(
    runner: &engine::game::scenario::GameRunner,
    context: &str,
) {
    let WaitingFor::ReplacementChoice {
        player,
        candidate_count,
        ref candidates,
    } = runner.state().waiting_for
    else {
        panic!("{context}, got {:?}", runner.state().waiting_for);
    };
    assert_eq!(
        player, P0,
        "{context}: the affected permanent's controller chooses"
    );
    assert_eq!(
        candidate_count, 2,
        "{context}: both replacements are candidates"
    );
    let names: Vec<&str> = candidates
        .iter()
        .map(|candidate| candidate.source_name.as_str())
        .collect();
    assert!(
        names.contains(&"Vorinclex, Monstrous Raider"),
        "{context}: Vorinclex must be a candidate, got {names:?}"
    );
    assert!(
        names.contains(&"Doc Samson, Super Psychiatrist"),
        "{context}: Doc Samson must be a candidate, got {names:?}"
    );
}
