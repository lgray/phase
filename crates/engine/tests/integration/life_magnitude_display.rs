//! CR 119.3 + CR 732.2a: `DerivedViews::life_magnitude` — the engine-owned life-readout magnitude.
//! A seat reads `Unbounded` only while an accepted shortcut marks its own Life axis; a drain
//! victim keeps its finite total.

use std::collections::{BTreeMap, BTreeSet};

use engine::analysis::decision_template::IterationCount;
use engine::analysis::loop_check::ShortcutResponse;
use engine::analysis::resource::ResourceAxis;
use engine::game::derived_views::{derive_views, DisplayMagnitude, UnboundedResourceView};
use engine::game::scenario::GameRunner;
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, GameState, WaitingFor};
use engine::types::mana::ManaType;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

use crate::loop_shortcut::{gunzip_dump, restore_dump};
use crate::play_trace::{
    ability, activate, altar_board, cast, chooses_color, is_offer, names, settle,
};
use crate::support::shared_card_db;

const P0: PlayerId = PlayerId(0);
const P3: PlayerId = PlayerId(3);

fn life(state: &GameState, player: PlayerId) -> i32 {
    state
        .players
        .iter()
        .find(|p| p.id == player)
        .expect("seat exists")
        .life
}

fn accept_every_responder(runner: &mut GameRunner) {
    while let WaitingFor::RespondToShortcut { .. } = runner.state().waiting_for {
        runner
            .act(GameAction::RespondToShortcut {
                response: ShortcutResponse::Accept,
            })
            .expect("accept the shortcut");
    }
}

/// Phyrexian Altar ("Sacrifice a creature: Add one mana of any color."), Gravecrawler ("This
/// creature can't block. You may cast this card from your graveyard as long as you control a
/// Zombie."), Walking Corpse (a Zombie with no rules text) and Soul Warden ("Whenever another
/// creature enters, you gain 1 life."): the loop grows P0's own life, so P0 reads ∞ from the take
/// until the collapse lands, and P1 never does.
#[test]
fn own_life_growth_reads_unbounded_until_the_collapse_lands() {
    let db = shared_card_db().expect("card db");
    let (mut runner, altar, gravecrawler) = altar_board(Some("Soul Warden"), db);
    let score = |a: &GameAction| 2 * names(&[gravecrawler])(a) + chooses_color(ManaType::Black)(a);
    for _ in 0..6 {
        cast(&mut runner, gravecrawler, vec![], CastPaymentMode::Auto);
        settle(&mut runner, &score);
        if is_offer(runner.state()) {
            break;
        }
        let index = ability(runner.state(), altar, true);
        activate(&mut runner, altar, index);
        settle(&mut runner, &score);
        if is_offer(runner.state()) {
            break;
        }
    }
    let WaitingFor::LoopShortcut {
        proposer,
        certificate,
        ..
    } = runner.state().waiting_for.clone()
    else {
        panic!(
            "expected the loop offer, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(proposer, P0);
    assert_eq!(certificate.unbounded, vec![ResourceAxis::Life(P0)]);
    let offer_life = life(runner.state(), P0);

    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::Fixed(3),
            template: None,
        })
        .expect("declare the shortcut");
    accept_every_responder(&mut runner);

    let state = runner.state();
    assert!(matches!(state.waiting_for, WaitingFor::Priority { .. }));
    assert_eq!(state.phase, Phase::PreCombatMain);
    assert_eq!(
        state.unbounded_resources,
        BTreeMap::from([(P0, BTreeSet::from([ResourceAxis::Life(P0)]))])
    );
    let views = derive_views(state, None);
    let wire = serde_json::to_value(&views).expect("derived views serialize");
    let golden = serde_json::json!({
        "life_magnitude": wire
            .get("life_magnitude")
            .expect("the take frame emits life_magnitude")
            .clone(),
    });
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../client/src/test/fixtures/unbounded-life-wire.json"
    );
    if std::env::var_os("UPDATE_WIRE_GOLDEN").is_some() {
        std::fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(&golden).unwrap()),
        )
        .expect("write the wire golden");
    }
    let unbounded_p0 = BTreeMap::from([(P0, DisplayMagnitude::Unbounded)]);
    assert_eq!(views.life_magnitude, unbounded_p0);

    let take_phase = runner.state().phase;
    for _ in 0..50 {
        let state = runner.state();
        if state.phase != take_phase || !matches!(state.waiting_for, WaitingFor::Priority { .. }) {
            break;
        }
        runner.act(GameAction::PassPriority).expect("pass priority");
    }
    let state = runner.state();
    assert!(
        matches!(state.waiting_for, WaitingFor::PayAmountChoice { player, .. } if player == P0),
        "the collapse prompts P0 at the boundary, got {:?}",
        state.waiting_for
    );
    assert_eq!(state.phase, Phase::BeginCombat);
    assert_eq!(derive_views(state, None).life_magnitude, unbounded_p0);

    runner
        .act(GameAction::SubmitPayAmount { amount: 3 })
        .expect("submit the collapse count");
    let state = runner.state();
    // Soul Warden gains 1 per entering creature; each of the 3 iterations casts one Gravecrawler.
    assert_eq!(life(state, P0), offer_life + 3);
    assert!(state.unbounded_resources.is_empty());
    assert!(derive_views(state, None).life_magnitude.is_empty());
    assert!(matches!(state.waiting_for, WaitingFor::Priority { .. }));

    let committed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("committed wire golden"))
            .unwrap();
    assert_eq!(
        golden, committed,
        "the client's life wire golden drifted from engine output — re-run with \
         UPDATE_WIRE_GOLDEN=1"
    );
}

/// Enduring Tenacity ("Whenever you gain life, target opponent loses that much life.") and
/// Exquisite Blood ("Whenever an opponent loses life, you gain that much life."): one shortcut
/// under P0 marks both P0's gain and P3's drain, so only P0's readout is ∞.
#[test]
fn drain_victim_keeps_a_finite_readout_under_a_mixed_mark() {
    let state = restore_dump(&gunzip_dump(include_bytes!(
        "../fixtures/tenacity_exquisite_blood_4p.json.gz"
    )));
    let WaitingFor::LoopShortcut {
        proposer,
        ref schema,
        ref certificate,
        ..
    } = state.waiting_for
    else {
        panic!("expected the restored offer, got {:?}", state.waiting_for);
    };
    assert_eq!(proposer, P0);
    assert_eq!(schema.iteration_count, IterationCount::UntilLethal);
    assert!(schema.points.is_empty());
    assert_eq!(
        certificate.unbounded,
        vec![ResourceAxis::Life(P0), ResourceAxis::Life(P3)]
    );

    let mut runner = GameRunner::from_state(state);
    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        })
        .expect("declare the shortcut");
    accept_every_responder(&mut runner);

    let state = runner.state();
    assert!(matches!(
        state.waiting_for,
        WaitingFor::GameOver { winner: Some(P0) }
    ));
    assert_eq!(
        state.unbounded_resources.get(&P0),
        Some(&BTreeSet::from([
            ResourceAxis::Life(P0),
            ResourceAxis::Life(P3)
        ]))
    );
    let views = derive_views(state, None);
    assert!(views.unbounded_resources.contains(&UnboundedResourceView {
        player: P3,
        axis: ResourceAxis::Life(P3),
    }));
    assert_eq!(
        views.life_magnitude,
        BTreeMap::from([(P0, DisplayMagnitude::Unbounded)])
    );
}
