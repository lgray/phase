//! Windfall's cross-player MAX aggregate — "the greatest number of cards a
//! player discarded this way".
//!
//! Oracle (Scryfall, verified verbatim 2026-08-15):
//!   "Each player discards their hand, then draws cards equal to the greatest
//!    number of cards a player discarded this way."
//!
//! Class: Windfall, Jace's Archivist, Whispering Madness — identical text.
//!
//! CR 608.2e: the discard action is processed simultaneously for every player,
//!   then the draw action reads that completed action's result.
//! CR 608.2h: the draw count is determined ONCE, when the draw action is
//!   applied — not re-derived per player as the fan-out proceeds.
//! CR 608.2i: that determination is a look-back at the already-completed
//!   discard action, the exception to CR 608.2h this clause relies on.
//! CR 701.9a: to discard a card is to move it from hand to graveyard.
//! CR 121.2: drawing N cards is N individual card draws.
//!
//! The regression this pins: the engine reduces the per-player discard counts to
//! ONE untyped scalar whose aggregate lived on the PRODUCER. With the producer
//! set to a cross-player SUM, Windfall drew 8+7+3+3 = 21 for every player
//! instead of the greatest single player's 8.

use engine::game::scenario::{GameScenario, Outcome, P0, P1};
use engine::types::mana::ManaCost;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const WINDFALL: &str = "Each player discards their hand, then draws cards equal to the greatest number of cards a player discarded this way.";

/// Syphon Mind's shape — the NON-superlative "discarded this way" neighbour.
///
/// This does NOT guard the aggregate axis, and an earlier revision of this file
/// claimed that it did. Syphon Mind parses to `FilteredTrackedSetSize` and
/// carries no `PreviousEffectAmount` node at all, so it is structurally
/// incapable of detecting a change to `QuantityRef::PreviousEffectAmount`'s
/// aggregate — measured: it stays green under BOTH the aggregate revert and the
/// clause-freeze revert. What it does guard is real and worth keeping: that the
/// superlative combinator did not STEAL the non-superlative phrasing, i.e. this
/// card still reaches `FilteredTrackedSetSize` and still sums.
///
/// The aggregate axis is guarded at unit level instead — see
/// `game/quantity.rs`'s `previous_effect_amount_live_when_no_snapshot` and
/// `previous_effect_amount_aggregates_are_mutually_distinct`. Measured: no
/// printed card yields a clean integration-level Sum-vs-Max discriminator.
const SYPHON_MIND: &str =
    "Each other player discards a card. You draw a card for each card discarded this way.";

/// Blood Tithe — the drain shape, and the class the corpus actually populates:
/// 40 of the 44 cards carrying both a `player_scope` and a
/// `PreviousEffectAmount` are this `LoseLife` → `GainLife { PreviousEffectAmount }`
/// form.
///
/// Unlike Syphon Mind this DOES build `PreviousEffectAmount`, with `aggregate`
/// absent and therefore `Sum`. CR 119.3: an effect causing a player to gain or
/// lose life adjusts that life total accordingly — one rule covers both
/// directions here. "The life lost this way" is the cross-player TOTAL, 9.
///
/// It is a REACH guard, not an aggregate discriminator: `Effect::LoseLife`
/// publishes no per-player table, so `Max`/`Min` fall back to the total and all
/// three reductions coincide at 9. Measured, not reasoned — see the degeneracy
/// note on the test itself.
const BLOOD_TITHE: &str =
    "Each opponent loses 3 life. You gain life equal to the life lost this way.";

const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);
const SEATS: [PlayerId; 4] = [P0, P1, P2, P3];

/// Deep enough that no draw in these tests is library-limited.
const LIBRARY_DEPTH: usize = 60;

fn seed_library(scenario: &mut GameScenario, player: PlayerId, n: usize) {
    for i in 0..n {
        scenario.add_card_to_library_top(player, &format!("Filler {i}"));
    }
}

fn seed_hand(scenario: &mut GameScenario, player: PlayerId, n: usize) {
    for i in 0..n {
        scenario.add_card_to_hand(player, &format!("Hand Filler {i}"));
    }
}

fn zone_len(outcome: &Outcome, player: PlayerId, zone: Zone) -> usize {
    let p = outcome
        .state()
        .players
        .iter()
        .find(|p| p.id == player)
        .expect("player exists");
    match zone {
        Zone::Hand => p.hand.len(),
        Zone::Library => p.library.len(),
        Zone::Graveyard => p.graveyard.len(),
        other => panic!("zone_len does not cover {other:?}"),
    }
}

/// CR 608.2e + CR 121.2: four seats, hands 8/7/3/3 (the USER-reported board).
/// CR 608.2h: the greatest number of cards any one player discarded is 8 and is
/// determined once when the draw action is applied, so EVERY player draws
/// exactly 8.
///
/// P0's eight are the cards held BESIDE Windfall: CR 601.2a removes the spell
/// from hand when the cast commits to the stack, so it is not itself discarded.
///
/// Non-vacuous and discriminating: the four hand sizes make MAX (8), SUM (21),
/// MIN (3), and per-player (8/7/3/3) four mutually distinguishable outcomes, so
/// the assertion fails under every wrong aggregate, not merely the one that
/// shipped. The graveyard assertion is the reach guard — it proves the discard
/// step actually ran, so a spell that failed to parse or resolve cannot pass a
/// bare hand-size check for the wrong reason.
#[test]
fn windfall_draws_the_greatest_single_players_discard_not_the_cross_player_sum() {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    for (seat, hand) in SEATS.iter().zip([8usize, 7, 3, 3]) {
        seed_hand(&mut scenario, *seat, hand);
        seed_library(&mut scenario, *seat, LIBRARY_DEPTH);
    }
    let windfall = scenario
        .add_spell_to_hand_from_oracle(P0, "Windfall", false, WINDFALL)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();

    let outcome = runner.cast(windfall).resolve();

    let drawn: Vec<usize> = SEATS
        .iter()
        .map(|p| LIBRARY_DEPTH - zone_len(&outcome, *p, Zone::Library))
        .collect();
    let hands: Vec<usize> = SEATS
        .iter()
        .map(|p| zone_len(&outcome, *p, Zone::Hand))
        .collect();
    let graveyards: Vec<usize> = SEATS
        .iter()
        .map(|p| zone_len(&outcome, *p, Zone::Graveyard))
        .collect();
    eprintln!("PROBE windfall/cast: drawn={drawn:?} hands={hands:?} graveyards={graveyards:?}");

    // CR 701.9a reach guard: every player really did discard their whole hand.
    assert!(
        graveyards[0] >= 8 && graveyards[1] >= 7 && graveyards[2] >= 3 && graveyards[3] >= 3,
        "reach guard: each player's hand must have reached the graveyard, got {graveyards:?}"
    );
    assert_eq!(
        drawn,
        vec![8, 8, 8, 8],
        "each player draws the GREATEST single-player discard (8), not the cross-player sum (21)"
    );
    assert_eq!(
        hands,
        vec![8, 8, 8, 8],
        "each hand holds exactly the freshly drawn cards"
    );
}

/// NON-INTERFERENCE, not an aggregate guard. Syphon Mind in a four-player game:
/// the three other players each discard one card and the controller draws 3.
///
/// What this discriminates: that the superlative combinator did not swallow the
/// non-superlative "discarded this way" phrasing — this card must still reach
/// `FilteredTrackedSetSize` and still sum. What it does NOT discriminate: the
/// aggregate axis. Syphon Mind builds no `PreviousEffectAmount` node, so it
/// cannot see a change to that ref's `aggregate` and stays green under both
/// revert arms. The cross-aggregate guard lives at unit level, in
/// `game/quantity.rs`'s `previous_effect_amount_aggregates_are_mutually_distinct`
/// and `previous_effect_amount_live_when_no_snapshot` — no printed card gives a
/// clean integration-level Sum-vs-Max discriminator.
#[test]
fn syphon_mind_shape_still_draws_the_cross_player_total() {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    for seat in SEATS {
        seed_hand(&mut scenario, seat, 1);
        seed_library(&mut scenario, seat, LIBRARY_DEPTH);
    }
    let syphon = scenario
        .add_spell_to_hand_from_oracle(P0, "Syphon Mind", false, SYPHON_MIND)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();

    let outcome = runner.cast(syphon).resolve();

    let drawn = LIBRARY_DEPTH - zone_len(&outcome, P0, Zone::Library);
    let opponents_discarded: usize = [P1, P2, P3]
        .iter()
        .map(|p| zone_len(&outcome, *p, Zone::Graveyard))
        .sum();
    eprintln!("PROBE syphon/cast: drawn={drawn} opponents_discarded={opponents_discarded}");

    // Reach guard: the discard step ran for all three opponents (CR 701.9a).
    assert_eq!(
        opponents_discarded, 3,
        "reach guard: each of the three other players must discard one card"
    );
    assert_eq!(
        drawn, 3,
        "controller draws one per card discarded across all opponents (sum), not the max (1)"
    );
}

/// CR 608.2c: a zero-contributor board must not disturb the Max class.
///
/// Board 8/7/3/**0** — P3 has an empty hand, so "each player discards their
/// hand" emits no discard event for them and the event-built table arrives as
/// `{8,7,3}` with P3 absent. The producer fills that gap with a 0 so an
/// aggregate reduces over every subject.
///
/// SCOPE — this asserts the NON-REGRESSION half only: the greatest discard is
/// still 8, so every player including the empty-handed one still draws 8. It
/// does NOT assert the table's contents, and deliberately so:
/// `last_effect_counts_by_player` is cleared at the player-action boundary, so
/// it reads `[]` from `outcome.state()` regardless of the fix. An earlier
/// revision asserted on it and failed with `left: []` — an INSTRUMENT failure,
/// not a fix failure. The table's contents are asserted where they survive, at
/// unit level: `game/effects/mod.rs`'s `fill_zero_contributors_*` tests, which
/// pin `Min` at 0 filled versus 3 unfilled.
#[test]
fn windfall_zero_contributor_board_still_draws_the_greatest() {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    seed_hand(&mut scenario, P0, 8);
    seed_hand(&mut scenario, P1, 7);
    seed_hand(&mut scenario, P2, 3);
    // P3: no hand at all — the zero contributor.
    for seat in SEATS {
        seed_library(&mut scenario, seat, LIBRARY_DEPTH);
    }
    let windfall = scenario
        .add_spell_to_hand_from_oracle(P0, "Windfall", false, WINDFALL)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();

    let outcome = runner.cast(windfall).resolve();

    let drawn: Vec<usize> = SEATS
        .iter()
        .map(|p| LIBRARY_DEPTH - zone_len(&outcome, *p, Zone::Library))
        .collect();
    let graveyards: Vec<usize> = SEATS
        .iter()
        .map(|p| zone_len(&outcome, *p, Zone::Graveyard))
        .collect();
    eprintln!("PROBE windfall/zero-contributor: drawn={drawn:?} graveyards={graveyards:?}");

    // CR 701.9a reach guard: the discard step ran, and P3 really contributed
    // nothing — without this, an all-8 draw could pass on a board that never
    // had a zero contributor at all.
    assert_eq!(
        graveyards[3], 0,
        "reach guard: P3 must be the zero contributor, got {graveyards:?}"
    );
    assert!(
        graveyards[0] >= 8,
        "reach guard: the discard step must have run, got {graveyards:?}"
    );
    assert_eq!(
        drawn,
        vec![8, 8, 8, 8],
        "non-regression: the greatest discard is still 8, so every player draws 8"
    );
}

/// REACH + non-regression guard for the drain class — NOT an aggregate
/// discriminator. Read the measured degeneracy below before trusting it as one.
///
/// Blood Tithe in a four-player game: each of the three opponents loses 3 life,
/// so "the life lost this way" is 3 + 3 + 3 = 9 (CR 119.3) and the controller
/// gains 9. This is the shape 40 of the 44 corpus cards carrying both a
/// `player_scope` and a `PreviousEffectAmount` take, so it is the widest
/// non-regression this file has.
///
/// WHAT IT DISCRIMINATES, measured by sentinel probe: the ref is genuinely
/// reached — forcing an early `return 999` at the top of the
/// `QuantityRef::PreviousEffectAmount` arm moves this card to 1019 life. So a
/// change that stopped routing the drain class through that arm fails here.
///
/// WHAT IT DOES **NOT** DISCRIMINATE: the aggregate axis. `Effect::LoseLife`
/// publishes no per-player breakdown — only `Discard` / `DiscardCard` /
/// `ChangeZoneAll` populate `last_effect_counts_by_player` — so the table is
/// EMPTY here and `Max`/`Min` both fall back to `unwrap_or(total)`. All three
/// reductions coincide:
///
///   Sum -> 9      Max -> 9      Min -> 9      (degenerate)
///
/// Measured, not reasoned: forcing `AggregateFunction::Sum => per_player.max()
/// .unwrap_or(total)` leaves this test green at 29. An earlier revision of this
/// comment claimed `Max -> 3` and that a global flip would fail here. That was
/// wrong, and it is the same error as the Syphon Mind control above — a
/// discriminating claim derived from the parse tree and never revert-probed.
///
/// The aggregate axis IS discriminated, at unit level where a populated table
/// can be constructed directly: `game/quantity.rs`'s
/// `previous_effect_amount_live_when_no_snapshot` asserts `Max` = 8 over
/// `{P0:8, P1:3}` with `last_effect_amount` = 11, so `Sum` fails it.
#[test]
fn blood_tithe_drain_still_gains_the_cross_player_total() {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    for seat in SEATS {
        seed_library(&mut scenario, seat, LIBRARY_DEPTH);
    }
    let tithe = scenario
        .add_spell_to_hand_from_oracle(P0, "Blood Tithe", false, BLOOD_TITHE)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();

    let outcome = runner.cast(tithe).resolve();

    let life: Vec<i32> = SEATS
        .iter()
        .map(|p| {
            outcome
                .state()
                .players
                .iter()
                .find(|pl| pl.id == *p)
                .expect("player exists")
                .life
        })
        .collect();
    eprintln!("PROBE blood-tithe/cast: life={life:?}");

    // Reach guard: the loss step actually ran for all three opponents, so the
    // per-player table really does hold three entries. Without this, a gain of 9
    // could be read off a table that never fanned out.
    assert_eq!(
        &life[1..],
        &[17, 17, 17],
        "reach guard: each of the three opponents loses exactly 3 (CR 119.3)"
    );
    assert_eq!(
        life[0], 29,
        "controller gains the cross-player TOTAL life lost (9) via \
         PreviousEffectAmount — a reach guard for the 40-card drain class, not an \
         aggregate discriminator (see the degeneracy note above)"
    );
}

/// PROBE for the second, independent defect the code map surfaced: the draw
/// tail keeps `player_scope: All` and re-fans-out, and each player's completed
/// draw re-stamps the shared scalar with that player's DELIVERED count. So a
/// player whose library ran short does not just draw fewer cards — they
/// redefine how many every LATER player draws.
///
/// CR 608.2h: the draw action's count is determined only once, when the
/// effect is applied — one player's short library cannot change another
/// player's count. CR 608.2e: the whole fan-out is one action processed
/// simultaneously. CR 121.2c: the SERIALIZATION (the active player performs
/// all of their draws first, then each other player in turn order) is itself
/// rules-correct — only the leaked count is not.
///
/// Discriminating: P0's library holds 5, everyone else 60. Correct = [5,8,8,8].
/// Leaked-delivered-count = [5,5,5,5]. The two differ on three seats.
#[test]
fn windfall_short_library_does_not_shrink_later_players_draws() {
    let mut scenario = GameScenario::new_n_player(4, 42);
    scenario.at_phase(Phase::PreCombatMain);
    for (seat, hand) in SEATS.iter().zip([8usize, 7, 3, 3]) {
        seed_hand(&mut scenario, *seat, hand);
        seed_library(
            &mut scenario,
            *seat,
            if *seat == P0 { 5 } else { LIBRARY_DEPTH },
        );
    }
    let windfall = scenario
        .add_spell_to_hand_from_oracle(P0, "Windfall", false, WINDFALL)
        .with_mana_cost(ManaCost::zero())
        .id();
    let mut runner = scenario.build();
    let outcome = runner.cast(windfall).resolve();

    let drawn: Vec<usize> = SEATS
        .iter()
        .map(|p| {
            let depth = if *p == P0 { 5 } else { LIBRARY_DEPTH };
            depth - zone_len(&outcome, *p, Zone::Library)
        })
        .collect();
    eprintln!(
        "PROBE windfall/short-library: drawn={drawn:?} waiting={:?}",
        outcome.final_waiting_for()
    );
    assert_eq!(
        drawn,
        vec![5, 8, 8, 8],
        "P0's short library caps only P0; every later player still draws the greatest discard (8)"
    );
}
