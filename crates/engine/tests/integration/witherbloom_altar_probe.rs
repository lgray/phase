//! PROBE (investigation only — not a shipped row). Drives the user's 4p Witherbloom /
//! Sprout Swarm / Altar of the Brood board through the production cast boundary and records what
//! the CR 732.2a bounded-shortcut machinery does with a mill-carrying infinite loop.
//!
//! Board (verified against the dump itself, `gameState.objects[…]`, and against Scryfall):
//!  - obj 55 Sprout Swarm in P0's hand: `Convoke` + `Buyback {3}`, one Spell ability creating a
//!    1/1 green Saproling.
//!  - obj 397 Witherbloom, the Balancer: static `CastWithKeyword{Affinity{Creature}}` over
//!    Or(Instant You, Sorcery You).
//!  - obj 90 Altar of the Brood: `ChangesZone`→Battlefield trigger on `Typed{Permanent, You,
//!    [Another]}` executing `Effect::Mill{Fixed 1, Controller, Graveyard}` with
//!    `player_scope: Opponent`.
//!  - obj 67 Doubling Season: `CreateToken` replacement, `Times{factor: 2}`.
//!  - P0 controls 9 creatures (7 untapped, 6 of them green) → affinity zeroes the {4} generic of
//!    {1}{G} + buyback {3} (CR 702.41 / CR 702.27), convoke pays the {G} by tapping one untapped
//!    green creature (CR 702.51a — not a tap ability, so summoning sickness is irrelevant).
//!
//! Loop invariants: +2 Saprolings/cycle (Doubling Season), each opponent mills 2/cycle
//! (CR 701.17), NO life change, P0's own library UNTOUCHED. The only bound-shaped axis in the
//! whole cycle is the OPPONENTS' library sizes — and per CR 701.17b + CR 121.4 an empty library
//! neither stops the mill nor ends the game, so it must not bound the loop.
//!
//! Prior art this mirrors exactly: `sprout_inalla_realistic_offer.rs` — the SAME Witherbloom +
//! Sprout Swarm loop on a real 4p dump WITHOUT Altar, where ONE live cast surfaces
//! `WaitingFor::LoopShortcut{P0}`. The only board difference here is Altar of the Brood.

use engine::game::scenario::GameRunner;
use engine::types::game_state::{GameState, PersistedGameState, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const P0: PlayerId = PlayerId(0);
const SPROUT: ObjectId = ObjectId(55);
const ALTAR: ObjectId = ObjectId(90);
const DOUBLING_SEASON: ObjectId = ObjectId(67);
/// An untapped P0 green Saproling to convoke for the {G} (417, 422, 432, 436, 437 are untapped).
const FODDER: ObjectId = ObjectId(417);

fn gunzip(gz: &[u8]) -> String {
    use std::io::Read;
    let mut json = String::new();
    flate2::read::GzDecoder::new(gz)
        .read_to_string(&mut json)
        .expect("fixture .json.gz must inflate to UTF-8 JSON");
    json
}

/// Load through the REAL production restore chokepoint, exactly as the sibling Sprout tests do.
fn load_wb() -> GameState {
    let json = gunzip(include_bytes!(
        "../fixtures/witherbloom_altar_sprout_swarm_4p.json.gz"
    ));
    let envelope: serde_json::Value =
        serde_json::from_str(&json).expect("dump envelope parses as JSON");
    serde_json::from_value::<PersistedGameState>(envelope["gameState"].clone())
        .expect("gameState deserializes through the production decoder")
        .into_game_state()
}

fn count_saprolings(state: &GameState, who: PlayerId) -> usize {
    state
        .battlefield
        .iter()
        .filter(|id| {
            state
                .objects
                .get(id)
                .is_some_and(|o| o.controller == who && o.name == "Saproling")
        })
        .count()
}

fn libs(state: &GameState) -> Vec<usize> {
    state.players.iter().map(|p| p.library.len()).collect()
}
fn gys(state: &GameState) -> Vec<usize> {
    state.players.iter().map(|p| p.graveyard.len()).collect()
}
fn lives(state: &GameState) -> Vec<i32> {
    state.players.iter().map(|p| p.life).collect()
}

/// One live Sprout Swarm cycle through the public boundary: accept Buyback, convoke `fodder`
/// for the {G}, commit, resolve.
fn drive_sprout_cast(state: GameState, fodder: ObjectId) -> engine::game::scenario::Outcome {
    GameRunner::from_state(state)
        .cast(SPROUT)
        .accept_optional()
        .convoke_with(&[fodder])
        .commit()
        .resolve()
}

fn report(label: &str, state: &GameState) {
    eprintln!(
        "[{label}] wf={:?}",
        std::format!("{:?}", state.waiting_for)
            .chars()
            .take(220)
            .collect::<String>()
    );
    eprintln!(
        "[{label}] libs={:?} gy={:?} life={:?} saprolings={} seq_len={}",
        libs(state),
        gys(state),
        lives(state),
        count_saprolings(state, P0),
        state.last_loop_action_sequence.len(),
    );
    if let WaitingFor::LoopShortcut {
        proposer,
        certificate,
        schema,
        ..
    } = &state.waiting_for
    {
        eprintln!("[{label}] *** OFFER *** proposer={proposer:?}");
        eprintln!("[{label}] CERT={certificate:#?}");
        eprintln!("[{label}] SCHEMA={schema:#?}");
    }
}

/// ARM A — the board exactly as the user dumped it (Altar of the Brood present).
#[test]
#[ignore = "probe"]
fn probe_a_with_altar() {
    let state = load_wb();
    assert!(
        matches!(state.waiting_for, WaitingFor::Priority { player } if player == P0),
        "fixture precondition: ordinary priority for P0, got {:?}",
        state.waiting_for
    );
    assert_eq!(
        state.objects.get(&SPROUT).map(|o| (o.name.as_str(), o.zone)),
        Some(("Sprout Swarm", Zone::Hand))
    );
    assert_eq!(
        state.objects.get(&ALTAR).map(|o| o.name.as_str()),
        Some("Altar of the Brood")
    );
    let f = state.objects.get(&FODDER).expect("fodder present");
    assert!(f.name == "Saproling" && f.controller == P0 && !f.tapped);
    report("A0", &state);

    let out = drive_sprout_cast(state, FODDER);
    report("A1", out.state());
    eprintln!("[A1] sprout zone = {:?}", out.zone_of(SPROUT));
}

/// ARM B — the SAME board with Altar of the Brood surgically removed before the drive. The A/B
/// differential: if the offer appears here and not in ARM A, the mill trigger is the suppressor.
#[test]
#[ignore = "probe"]
fn probe_b_without_altar() {
    let mut state = load_wb();
    state.battlefield.retain(|id| *id != ALTAR);
    state.objects.remove(&ALTAR);
    report("B0", &state);

    let out = drive_sprout_cast(state, FODDER);
    report("B1", out.state());
    eprintln!("[B1] sprout zone = {:?}", out.zone_of(SPROUT));
}

/// ARM D — multi-cycle WITHOUT Altar. The matched multi-cycle partner of ARM C: if C never
/// offers and D does, Altar is the suppressor at multi-cycle depth too.
#[test]
#[ignore = "probe"]
fn probe_d_multi_cycle_without_altar() {
    let mut state = load_wb();
    state.battlefield.retain(|id| *id != ALTAR);
    state.objects.remove(&ALTAR);
    multi_cycle(state, "D");
}

/// ARM C — the user's board, driven for SEVERAL cycles, in case one cycle is not enough history
/// for the detector on this board. Each cycle convokes a different untapped fodder Saproling.
#[test]
#[ignore = "probe"]
fn probe_c_multi_cycle_with_altar() {
    multi_cycle(load_wb(), "C");
}

/// The 2×2 factorial. `derived_fodder_class` (engine.rs:5175) is fail-closed at EXACTLY ONE new
/// battlefield object per period, so Doubling Season (2 Saprolings/cycle) is a suppressor
/// INDEPENDENT of Altar. Removing DS isolates the Altar/mill axis.
fn remove(state: &mut GameState, id: ObjectId) {
    state.battlefield.retain(|x| *x != id);
    state.objects.remove(&id);
}

/// ARM E — Doubling Season removed, Altar KEPT. One token/cycle ⇒ the fodder class derives; the
/// only remaining deviation from the shipped-green sibling board is the mill.
#[test]
#[ignore = "probe"]
fn probe_e_no_doubling_season_with_altar() {
    let mut state = load_wb();
    remove(&mut state, DOUBLING_SEASON);
    report("E0", &state);
    let out = drive_sprout_cast(state, FODDER);
    report("E1", out.state());
}

/// ARM F — Doubling Season AND Altar removed. The matched control for ARM E: this board should
/// reproduce the shipped `sprout_inalla_realistic_offer_fires` shape and OFFER. If F offers and
/// E does not, the mill is the isolated suppressor.
#[test]
#[ignore = "probe"]
fn probe_f_no_doubling_season_no_altar() {
    let mut state = load_wb();
    remove(&mut state, DOUBLING_SEASON);
    remove(&mut state, ALTAR);
    report("F0", &state);
    let out = drive_sprout_cast(state, FODDER);
    report("F1", out.state());
}

/// P3's Pyreswipe Hawk (obj 298) — its `Attacks` trigger body is
/// `Pump{power: Aggregate{Max, ManaValue, Typed{Artifact, controller You}}}`, a ledger read the
/// growing-class firewall vetoes on (measured: `PROBE-FW: veto @ ... obj="Pyreswipe Hawk"`).
const PYRESWIPE_HAWK: ObjectId = ObjectId(298);

/// ARM G — Doubling Season + Pyreswipe Hawk removed, Altar KEPT. Isolates the mill: with S1 and
/// S3 neutralized the ONLY remaining deviation is Altar's mill.
#[test]
#[ignore = "probe"]
fn probe_g_isolate_mill() {
    let mut state = load_wb();
    remove(&mut state, DOUBLING_SEASON);
    remove(&mut state, PYRESWIPE_HAWK);
    report("G0", &state);
    let out = drive_sprout_cast(state, FODDER);
    report("G1", out.state());
}

/// ARM H — Doubling Season + Pyreswipe Hawk + Altar removed. The matched positive endpoint: if H
/// OFFERS and G does not, the mill is proven to be the isolated suppressor on the user's own board.
#[test]
#[ignore = "probe"]
fn probe_h_isolate_mill_control() {
    let mut state = load_wb();
    remove(&mut state, DOUBLING_SEASON);
    remove(&mut state, PYRESWIPE_HAWK);
    remove(&mut state, ALTAR);
    report("H0", &state);
    let out = drive_sprout_cast(state, FODDER);
    report("H1", out.state());
}

const PIT_OF_OFFERINGS: ObjectId = ObjectId(9);

/// ARM I — peel one more layer: DS + Altar + Pyreswipe Hawk + Pit of Offerings removed. Measures
/// how DEEP the firewall's false-positive stack goes on a realistic 4p board.
#[test]
#[ignore = "probe"]
fn probe_i_peel_two() {
    let mut state = load_wb();
    for id in [DOUBLING_SEASON, ALTAR, PYRESWIPE_HAWK, PIT_OF_OFFERINGS] {
        remove(&mut state, id);
    }
    report("I0", &state);
    let out = drive_sprout_cast(state, FODDER);
    report("I1", out.state());
}

fn multi_cycle(mut state: GameState, prefix: &str) {
    report(&format!("{prefix}0"), &state);
    for (i, fodder) in [422u64, 432, 436, 437, 417].into_iter().enumerate() {
        if matches!(state.waiting_for, WaitingFor::LoopShortcut { .. }) {
            eprintln!("[{prefix}{i}] offer already up; stopping");
            break;
        }
        // Re-aim at any untapped P0 Saproling if the scripted one is already tapped.
        let pick = if state
            .objects
            .get(&ObjectId(fodder))
            .is_some_and(|o| !o.tapped && o.controller == P0)
        {
            ObjectId(fodder)
        } else {
            let found = state
                .battlefield
                .iter()
                .find(|id| {
                    state.objects.get(id).is_some_and(|o| {
                        o.controller == P0 && o.name == "Saproling" && !o.tapped
                    })
                })
                .copied();
            match found {
                Some(id) => id,
                None => {
                    eprintln!("[{prefix}{i}] no untapped fodder left; stopping");
                    break;
                }
            }
        };
        let out = drive_sprout_cast(state, pick);
        state = out.state().clone();
        report(&format!("{prefix}{}", i + 1), &state);
    }
}
