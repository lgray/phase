// engine-citation-gate: symbol anchors only
//! CR 732.2b/c stage 2 — EFFICACY of a polled seat's loop-shortcut response.
//!
//! CITATION FORM: rule NUMBER only. The number is itself the greppable heading —
//! `grep '^732.2c' docs/MagicCompRules.txt` resolves any citation below. Line
//! anchors are forbidden here (this file is enrolled in
//! `subsystem_citations_are_symbol_anchored`) because `docs/MagicCompRules.txt`
//! is gitignored and re-fetched per checkout, so a line anchor is pinned to
//! whichever rules revision the author happened to hold — the anchors this file
//! originally shipped already resolved to the wrong lines against the revision
//! fetched into the neighbouring checkout.
//!
//! `ai_support::smart_shortcut_response` shipped with a POSSIBILITY predicate
//! only: any meaningful priority action bought a `Shorten`, i.e. a real priority
//! window. That is right for a seat holding a Bolt and wrong for a seat holding
//! a fetchland — activating Terramorphic Expanse satisfies CR 732.2c's "must
//! make a different game choice" while changing nothing about the loop, so the
//! window is spent achieving nothing.
//!
//! Stage 2 is AI POLICY, not a rule: CR 732.2b grants an
//! unconditioned accept-or-shorten option and states no efficacy criterion. The
//! rows below pin the policy's two arms and, more importantly, pin the ONE
//! thing an over-broad version would destroy — that a seat holding real
//! interaction still gets its window.
//!
//! # Mutant discipline
//!
//! Two mutants are named per row, and every row states which one flips it:
//! * **DROP** — delete stage 2 from `smart_shortcut_response` (both arms), i.e.
//!   restore the shipped one-stage predicate.
//! * **TRIVIALIZE** — make the stage-2 predicate constant. For arm (B) that is
//!   `shortcut_efficacy::filter_is_actor_owned ≡ true` (everything looks
//!   confined); for arm (A) it is deleting the `crowned_winner` guard.
//!
//! A row whose expected value equals the SHIPPED value cannot be flipped by
//! DROP — its discriminating power is entirely in TRIVIALIZE, and that is
//! stated on the row rather than papered over.

use engine::analysis::decision_template::IterationCount;
use engine::analysis::loop_check::ShortcutResponse;
use engine::game::engine::apply;
use engine::game::scenario::{GameRunner, GameScenario};
use engine::types::ability::{AbilityDefinition, AbilityKind, Effect, QuantityExpr, TargetFilter};
use engine::types::actions::{GameAction, PrecastCopyShortcutResponse};
use engine::types::card_type::CoreType;
use engine::types::game_state::{GameState, LayersDirty, LoopDetectionMode, WaitingFor};
use engine::types::identifiers::{CardId, ObjectId};
use engine::types::mana::{ManaColor, ManaCost, ManaCostShard};
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);

// --- Oracle-text constants, and what each one's provenance ACTUALLY is ---
//
// Two provenances ship here and they are deliberately not conflated, because
// "verbatim Oracle text" is a claim about a printing and only some of these
// make it.
//
// (1) SIBLING-FIXTURE PROVENANCE — the four loop-shape constants below are
//     byte-identical copies of the shipped constants in
//     `tests/integration/loop_shortcut.rs`, which does not present them as any
//     card's Oracle text either. They exist to reproduce that file's mutual-drain
//     loop, and copying them verbatim is what keeps the two files' loop shape
//     identical. MEASURED against MTGJSON `AtomicCards.json` (`.data[*][0].text`):
//       * `DRAIN_CLERIC` IS one printing's complete Oracle text (Epicure of Blood,
//         Marauding Blight-Priest — 2 exact matches);
//       * `BLOOD_SIPPER` matches NO card, not even as a substring;
//       * `KICKOFF` / `TARGETED_KICKOFF` match no card's complete text; they are
//         single-clause fragments (substrings of 53 and 3 cards respectively).
//     So do NOT cite this block as card-derived: only `DRAIN_CLERIC` would
//     survive that claim, and it is not why any of the four is here.
//
// (2) CARD PROVENANCE — `TERRAMORPHIC` and `DEATHRITE_SHAMAN` (below) ARE their
//     named printing's complete Oracle text, verified byte-for-byte against
//     MTGJSON. That matters for those two specifically: they are the rows'
//     subject matter, and a paraphrase can take a different parser branch and go
//     green while the real card stays broken.

const DRAIN_CLERIC: &str = "Whenever you gain life, each opponent loses 1 life.";
const BLOOD_SIPPER: &str = "Whenever an opponent loses life, you gain 1 life.";
const KICKOFF: &str = "You gain 1 life.";
const TARGETED_KICKOFF: &str = "Target player gains 1 life.";

/// Terramorphic Expanse, verbatim. Acceptance (a): a fetchland is the canonical
/// action that is legal, meaningful to stage 1, and totally confined.
const TERRAMORPHIC: &str = "{T}, Sacrifice this land: Search your library for a basic land card, \
                            put it onto the battlefield tapped, then shuffle.";

/// Deathrite Shaman, verbatim, all three abilities. Ability `[0]`'s cost is
/// `{T}` ALONE — no mana component — which is what lets the V1c fixture deny
/// `{B}`/`{G}` and still leave `[0]` legal. Its target is a land card in *a*
/// graveyard: the AST names no player, so ownership is UNPROVEN (CR 400.1 —
/// "Each player has their own library, hand, and graveyard"), which is exactly
/// why an `origin`-keyed confinement rule would wrongly call it self-contained.
const DEATHRITE_SHAMAN: &str = "{T}: Exile target land card from a graveyard. Add one mana of any \
                                color.\n{B}, {T}: Exile target instant or sorcery card from a \
                                graveyard. Each opponent loses 2 life.\n{G}, {T}: Exile target \
                                creature card from a graveyard. You gain 2 life.";

// ---------------------------------------------------------------------------
// Shared drive helpers. Deliberately local: `loop_shortcut.rs`'s equivalents
// are private to that module and it is not in this change's scope.
// ---------------------------------------------------------------------------

/// Pass/answer beats until the state leaves `Priority`/`OrderTriggers`.
fn drive_collect(runner: &mut GameRunner, cap: usize) -> WaitingFor {
    for _ in 0..cap {
        match runner.state().waiting_for.clone() {
            WaitingFor::Priority { .. } => {
                if runner.act(GameAction::PassPriority).is_err() {
                    break;
                }
            }
            WaitingFor::OrderTriggers { triggers, .. } => {
                let order: Vec<usize> = (0..triggers.len()).collect();
                if runner
                    .act(GameAction::OrderTriggers { order })
                    .or_else(|_| runner.act(GameAction::OrderTriggers { order: vec![] }))
                    .is_err()
                {
                    break;
                }
            }
            _ => break,
        }
    }
    runner.state().waiting_for.clone()
}

/// The exact action list `smart_shortcut_response` folds over — obtained by
/// CALLING production's recipe (`ai_support::shortcut_probe`), not by copying it.
/// A local copy would drift the moment production's recipe changed, and every
/// reach-guard in this file reads this list, so the guards would then be
/// measuring a different action set than the code under test.
fn probe_actions(state: &GameState, player: PlayerId) -> Vec<GameAction> {
    engine::ai_support::shortcut_probe(state, player).1
}

/// Stage 1's verdict, evaluated on the PROBE state — which is the state
/// production evaluates it on. Evaluating it on the caller's
/// `RespondToShortcut` state instead silently drops
/// `has_meaningful_priority_action`'s sacrifice-for-mana rung, which is gated on
/// `waiting_for` being `Priority`.
fn stage_one_meaningful(state: &GameState, player: PlayerId) -> bool {
    let (probe, actions) = engine::ai_support::shortcut_probe(state, player);
    engine::ai_support::has_meaningful_priority_action(probe.state(), &actions)
}

/// Which ability indices of `source` are actually enumerated at this window.
/// V1c's two reach-guards read this.
fn legal_ability_indices(state: &GameState, player: PlayerId, source: ObjectId) -> Vec<usize> {
    let mut indices: Vec<usize> = probe_actions(state, player)
        .iter()
        .filter_map(|a| match a {
            GameAction::ActivateAbility {
                source_id,
                ability_index,
            } if *source_id == source => Some(*ability_index),
            _ => None,
        })
        .collect();
    indices.sort_unstable();
    indices
}

/// The shipped `setup_3p_optional_cascade` shape (`loop_shortcut.rs`): P0 runs
/// a self-refilling mutual drain, P1's Mountain + Bolt make the loop OPTIONAL
/// so an offer is raised at all. `decorate` stages the seat under test.
fn optional_cascade(decorate: impl FnOnce(&mut GameScenario)) -> (GameRunner, ObjectId) {
    let mut scenario = GameScenario::new_n_player(3, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(P1, 20);
    scenario.with_life(P2, 20);
    scenario.add_creature_from_oracle(P0, "Test Drain Cleric", 2, 2, DRAIN_CLERIC);
    scenario.add_creature_from_oracle(P0, "Test Blood Sipper", 2, 2, BLOOD_SIPPER);
    scenario.add_basic_land(P1, ManaColor::Red);
    scenario.add_bolt_to_hand(P1);
    decorate(&mut scenario);
    let kickoff = scenario
        .add_spell_to_hand_from_oracle(P0, "Test Lifegain Kickoff", false, KICKOFF)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    (runner, kickoff)
}

/// Cast the kick-off, drive to the offer, have P0 declare, then walk the APNAP
/// queue to `seat` by submitting manual Accepts for everyone ahead of it (never
/// the AI's answer — that would stop the queue).
fn respond_window_at(runner: &mut GameRunner, kickoff: ObjectId, seat: PlayerId) {
    let _ = runner.cast(kickoff).resolve();
    let wf = drive_collect(runner, 500);
    assert!(
        matches!(wf, WaitingFor::LoopShortcut { .. }),
        "reach-guard: the optional cascade must OFFER a shortcut, got {wf:?}"
    );
    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        })
        .expect("the proposer declares");
    for _ in 0..8 {
        match runner.state().waiting_for {
            WaitingFor::RespondToShortcut { player, .. } if player == seat => return,
            WaitingFor::RespondToShortcut { .. } => {
                runner
                    .act(GameAction::RespondToShortcut {
                        response: ShortcutResponse::Accept,
                    })
                    .expect("manual Accept advances the APNAP queue");
            }
            _ => break,
        }
    }
    panic!(
        "reach-guard: {seat:?} must be polled; stopped at {:?}",
        runner.state().waiting_for
    );
}

// ===========================================================================
// V1b — ACCEPTANCE (a), Path A class: a fetchland no longer buys a window.
// ===========================================================================

/// The shipped optional-cascade fixture plus exactly ONE card: a Terramorphic
/// Expanse on the polled seat's battlefield. Stage 1 still says "you have a
/// meaningful action" — a non-mana activated ability always does — and stage 2
/// answers the question stage 1 cannot: the action reaches nothing but its own
/// controller's library, so the window would change nothing.
///
/// MUTANTS — both flip the `Accept` assertion:
/// * DROP ⇒ `Shorten { at_iteration: 0 }` (this IS the shipped behaviour, which
///   is the defect).
/// * TRIVIALIZE arm (B) (`any_action_may_interfere ≡ true`) ⇒ `Shorten`.
///
/// REACH-GUARDS (both are assertions): the fetchland really is enumerated, and
/// stage 1 really does return `true`. Without them an `Accept` here would be
/// indistinguishable from the empty-board stage-1 path — the vacuity that a
/// naive version of this row would ship.
#[test]
fn v1b_a_confined_fetchland_accepts_instead_of_buying_a_vacuous_window() {
    let mut terramorphic = ObjectId(0);
    let (mut runner, kickoff) = optional_cascade(|s| {
        terramorphic = s
            .add_land_from_oracle(P2, "Terramorphic Expanse", TERRAMORPHIC)
            .id();
    });
    respond_window_at(&mut runner, kickoff, P2);

    let actions = probe_actions(runner.state(), P2);
    assert!(
        actions.contains(&GameAction::ActivateAbility {
            source_id: terramorphic,
            ability_index: 0,
        }),
        "REACH-GUARD 1: the fetchland's ability must be enumerated, otherwise this row \
         degenerates to the empty-board stage-1 path; got {actions:?}"
    );
    assert!(
        stage_one_meaningful(runner.state(), P2),
        "REACH-GUARD 2: stage 1 (POSSIBILITY, untouched by this change) must still return \
         true — an Accept produced by stage 1 would prove nothing about stage 2"
    );

    assert_eq!(
        engine::ai_support::smart_shortcut_response(runner.state(), P2),
        ShortcutResponse::Accept,
        "a seat whose ONLY action is a self-contained fetch has no efficacious response; \
         spending a real priority window on it changes nothing (CR 732.2c is satisfied by \
         any different choice, which is precisely why it grants no efficacy)"
    );
}

// ===========================================================================
// V1c — B1 REGRESSION LOCK. The graveyard-hate seat still Shortens.
// ===========================================================================

/// The one row whose sole job is pinning the owner axis. A graveyard is a
/// PER-PLAYER zone (CR 400.1) and `Zone` carries no
/// player field, so a rule keyed on `ChangeZone.origin` cannot tell "exile a
/// land card from MY graveyard" from "…from YOURS". Deathrite Shaman `[0]`
/// exiles a land card from P0's graveyard — a real interaction with another
/// player's resources — and must keep its window.
///
/// This row's expected value (`Shorten`) IS the shipped value, so **DROP cannot
/// flip it**. Its whole discriminating power is the TRIVIALIZE arm:
/// `filter_is_actor_owned ≡ true` makes DRS `[0]` fold to `OwnResourcesOnly`
/// and the response becomes `Accept` — the row FLIPS.
///
/// That flip only exists if the fixture leaves ability `[0]` and ONLY `[0]`
/// legal, so both constraints ship as assertions:
/// * REACH-GUARD 1 — without a land card in a graveyard, `[0]` is not
///   enumerated at all and the action list collapses to `["PassPriority"]`;
///   stage 1 returns false and the row would pass through the wrong path.
/// * REACH-GUARD 2 — with `{B}`/`{G}` available and a matching graveyard card,
///   `[1]`/`[2]` become legal. Their `LoseLife`/`GainLife` sub-effects classify
///   `MayInterfere` even under the mutant, so `any_action_may_interfere`'s
///   `.any()` absorbs the mutation and the row passes VACUOUSLY.
///
/// The fixture denies `{B}`/`{G}` by construction (P2 controls no lands) and
/// stages no instant/sorcery/creature card in any graveyard, so `[1]` and `[2]`
/// are each blocked on two independent axes.
#[test]
fn v1c_graveyard_hate_across_a_per_player_zone_keeps_its_window() {
    let mut shaman = ObjectId(0);
    let (mut runner, kickoff) = optional_cascade(|s| {
        // CR 302.6 (the summoning-sickness rule): `add_creature_from_oracle`
        // stages a pre-existing battlefield creature, so the `{T}` cost is
        // payable.
        shaman = s
            .add_creature_from_oracle(P2, "Deathrite Shaman", 1, 2, DEATHRITE_SHAMAN)
            .id();
        // The land card sits in P0's graveyard — the ability reaches ACROSS a
        // per-player zone that the AST does not player-qualify. That crossing
        // is the whole of the defect this row locks.
        s.add_land_to_graveyard(P0, "Test Graveyard Land");
    });
    respond_window_at(&mut runner, kickoff, P2);

    let indices = legal_ability_indices(runner.state(), P2, shaman);
    assert!(
        indices.contains(&0),
        "REACH-GUARD 1: without a land card in a graveyard the Shaman's [0] is not enumerated \
         and this row degenerates to the stage-1 empty-action path; got {indices:?}"
    );
    assert_eq!(
        indices,
        vec![0],
        "REACH-GUARD 2: [1]/[2] must stay illegal. They classify MayInterfere even under the \
         TRIVIALIZE mutant, so leaving one legal lets .any() absorb the mutation and this row \
         passes vacuously; got {indices:?}"
    );

    assert_eq!(
        engine::ai_support::smart_shortcut_response(runner.state(), P2),
        ShortcutResponse::Shorten { at_iteration: 0 },
        "exiling a land card out of ANOTHER player's graveyard is real interaction — the \
         confinement predicate must require PROVEN actor ownership, not merely a zone name"
    );
}

// ===========================================================================
// V2 / V3 — ACCEPTANCE (b) and the matched pair.
// ===========================================================================

/// V3, both arms in one row, because neither arm alone is the discriminator.
/// The two boards are identical but for P2's holdings:
///   * `{}` ⇒ Accept, reached through stage 1 (nothing to do);
///   * `{Mountain, Lightning Bolt}` ⇒ Shorten, reached through stage 2.
///
/// The pass ⇒ grant / respond ⇒ no-grant pair is what proves stage 2 did not
/// over-generalize into "always Accept". Sibling coverage for Wrath of God,
/// Naturalize, Divination and Path to Exile is at classifier granularity in
/// `ai_support::shortcut_efficacy`'s unit table (they are sorceries/instants
/// with no legal target on this board, so a runtime row would assert on
/// castability rather than on efficacy).
///
/// MUTANTS: TRIVIALIZE arm (B) (`any_action_may_interfere ≡ false`, or
/// `filter_is_actor_owned ≡ true` — Bolt's `DealDamage` reaches neither, so it
/// is the whole-predicate constant that bites) flips the Bolt arm to `Accept`.
/// DROP leaves both arms at their shipped values and flips neither; that is
/// stated rather than claimed otherwise.
#[test]
fn v3_matched_pair_empty_seat_accepts_and_bolt_seat_still_shortens() {
    // Arm 1 — nothing at all.
    let (mut bare, bare_kickoff) = optional_cascade(|_| {});
    respond_window_at(&mut bare, bare_kickoff, P2);
    let bare_actions = probe_actions(bare.state(), P2);
    assert!(
        !stage_one_meaningful(bare.state(), P2),
        "reach-guard: this arm must resolve at STAGE 1, so it stays a control for the stage-2 \
         arm below; got {bare_actions:?}"
    );
    assert_eq!(
        engine::ai_support::smart_shortcut_response(bare.state(), P2),
        ShortcutResponse::Accept,
        "no meaningful action ⇒ Accept, unchanged from the shipped predicate"
    );

    // Arm 2 — the SAME board plus a Mountain and a Bolt.
    let (mut armed, armed_kickoff) = optional_cascade(|s| {
        s.add_basic_land(P2, ManaColor::Red);
        s.add_bolt_to_hand(P2);
    });
    respond_window_at(&mut armed, armed_kickoff, P2);
    let armed_actions = probe_actions(armed.state(), P2);
    assert!(
        armed_actions
            .iter()
            .any(|a| matches!(a, GameAction::CastSpell { .. })),
        "reach-guard: the Bolt must actually be castable, otherwise this arm tests the empty \
         board twice; got {armed_actions:?}"
    );
    assert_eq!(
        engine::ai_support::smart_shortcut_response(armed.state(), P2),
        ShortcutResponse::Shorten { at_iteration: 0 },
        "ACCEPTANCE (b): a seat holding real interaction must still get its priority window — \
         this is the assertion any over-broad confinement rule destroys first"
    );
}

// ===========================================================================
// V4 / V6 / V7 — arm (A): the crowned seat, keyed on `predicted_winner`.
// ===========================================================================

/// CR 732.2a lets the player with priority propose a
/// shortcut whose predictable result crowns SOMEONE ELSE. This fixture is the
/// shipped `interactive_offer_separates_priority_proposer_from_predicted_winner`
/// shape: P1 proposes, P0 is the measured winner, and P1 (the proposer) is
/// excluded from the response queue, so P0 is polled.
///
/// P0 also holds a Mountain and a Bolt — the reach-guard the measurement proved
/// load-bearing. WITHOUT them P0 has no meaningful action and Accepts via
/// stage 1, making the row vacuous; WITH them the shipped predicate returns
/// `Shorten`, i.e. the crowned player shortens its own guaranteed win.
///
/// Three claims ride this one board:
/// * **V4** — arm (A) fires: the crowned seat Accepts.
/// * **V6** — it is keyed on `predicted_winner`, never `proposer`. The row
///   asserts `proposer != predicted_winner` and `polled == predicted_winner`,
///   so a `proposer`-keyed implementation (which passes every other row) fails
///   exactly here.
/// * **V7** — read order. Arm (A) reads the proposal off the ORIGINAL state;
///   `smart_shortcut_response` overwrites its probe clone's `waiting_for` with
///   `Priority` before enumerating. Moving that read after the clone makes
///   `crowned_winner` unconditionally `None` and this row FAILS — it is the
///   only row that can detect the mis-ordering.
///
/// MUTANTS — both flip the `Accept` assertion: DROP ⇒ `Shorten`; TRIVIALIZE
/// arm (A) (delete the `crowned_winner` guard) ⇒ `Shorten` via arm (B), because
/// the Bolt is genuine interference.
#[test]
fn v4_the_crowned_seat_accepts_its_own_predicted_win() {
    let mut scenario = GameScenario::new_n_player(2, 7);
    scenario.at_phase(Phase::PreCombatMain);
    scenario.with_life(P0, 20);
    scenario.with_life(P1, 20);
    scenario.add_creature_from_oracle(P0, "Test Drain Cleric", 2, 2, DRAIN_CLERIC);
    scenario.add_creature_from_oracle(P0, "Test Blood Sipper", 2, 2, BLOOD_SIPPER);
    scenario.add_basic_land(P1, ManaColor::Red);
    scenario.add_bolt_to_hand(P1);
    // The reach-guard: P0 must hold a meaningful action or stage 1 answers first.
    scenario.add_basic_land(P0, ManaColor::Red);
    scenario.add_bolt_to_hand(P0);
    let kickoff = scenario
        .add_spell_to_hand_from_oracle(P1, "P0 Lifegain Kickoff", false, TARGETED_KICKOFF)
        .id();
    let mut runner = scenario.build();
    runner.state_mut().loop_detection = LoopDetectionMode::Interactive;
    runner.state_mut().active_player = P1;
    runner.state_mut().priority_player = P1;
    runner.state_mut().waiting_for = WaitingFor::Priority { player: P1 };

    let _ = runner.cast(kickoff).target_player(P0).resolve();
    let wf = drive_collect(&mut runner, 500);
    let WaitingFor::LoopShortcut {
        proposer,
        predicted_winner,
        ..
    } = wf
    else {
        panic!("reach-guard: P1's priority window must receive an offer, got {wf:?}");
    };
    assert_eq!(
        proposer, P1,
        "CR 732.2a routes the offer to the priority holder"
    );
    assert_eq!(
        predicted_winner,
        Some(P0),
        "reach-guard: the two authorities must actually DIFFER, or the winner-keyed and \
         proposer-keyed implementations are indistinguishable here"
    );

    runner
        .act(GameAction::DeclareShortcut {
            count: IterationCount::UntilLethal,
            template: None,
        })
        .expect("P1 declares");
    let WaitingFor::RespondToShortcut {
        player,
        ref proposal,
        ..
    } = runner.state().waiting_for
    else {
        panic!(
            "reach-guard: a response window must open, got {:?}",
            runner.state().waiting_for
        );
    };
    assert_eq!(
        player, P0,
        "the proposer is excluded from its own response queue, so the crowned seat is polled"
    );
    assert_ne!(
        proposal.proposer,
        proposal
            .predicted_winner
            .expect("this offer names a winner"),
        "V6: the multi-authority premise — a proposer-keyed rule would read P1 here"
    );

    let actions = probe_actions(runner.state(), P0);
    assert!(
        stage_one_meaningful(runner.state(), P0),
        "REACH-GUARD: without a meaningful action P0 would Accept via stage 1 and this row \
         would be vacuous; got {actions:?}"
    );

    assert_eq!(
        engine::ai_support::smart_shortcut_response(runner.state(), P0),
        ShortcutResponse::Accept,
        "arm (A): the offer's predicted result already crowns this seat. CR 732.2c grants a \
         shortening player nothing but the obligation to choose differently, so shortening \
         here moves the game away from a win it already holds"
    );
}

// ===========================================================================
// V1 / V5 — REAL 4-player board, loaded through the production restore
// chokepoint and driven through the public `apply()` boundary.
// ===========================================================================

/// Inflate a committed dump fixture.
fn gunzip_dump(gz: &[u8]) -> String {
    use std::io::Read;
    let mut json = String::new();
    flate2::read::GzDecoder::new(gz)
        .read_to_string(&mut json)
        .expect("fixture .json.gz must inflate to UTF-8 JSON");
    json
}

/// Decode AS `PersistedGameState` — the production chokepoint the server's
/// `from_persisted` and WASM's `decode_restored_game_state` both funnel
/// through — rather than decoding a bare `GameState`.
fn restore_dump(json: &str) -> GameState {
    let envelope: serde_json::Value =
        serde_json::from_str(json).expect("dump envelope parses as JSON");
    serde_json::from_value::<engine::types::game_state::PersistedGameState>(
        envelope["gameState"].clone(),
    )
    .expect("gameState deserializes through the production decoder")
    .into_game_state()
}

/// The LIVE-PATH board: the real 4-player Dina / Bloodthirsty Conqueror drain
/// on which the defect actually occurs, because seat P2 controls a Terramorphic
/// Expanse.
///
/// Derived from the read-only pristine archive, and the derivation is the
/// artifact's provenance rather than a claim about it:
/// `unzip -p combofb-dumps-pristine/dina-conqueror-offers-no-ff.zip |
///  jq -c '{gameState}' | gzip -9 -n`
/// → 844846 bytes, sha256
/// `9843d5165cbbf7dd7bca4171c7888c190b7eba7e52a2ed095b44ff76fadd7886`.
/// `gzip -n` is a no-op from a pipe but load-bearing from a file (it strips the
/// stored name and mtime), so KEEP it: a re-derivation that stages the 21 MB
/// dump through an intermediate file — the natural thing to do at that size —
/// otherwise misses the digest and presents as a corrupt artifact rather than
/// as convention drift.
fn live_path_board() -> GameState {
    restore_dump(&gunzip_dump(include_bytes!(
        "../fixtures/dina_noff_turn5_4p.json.gz"
    )))
}

/// The QUIET board: the same matchup captured at a beat where NO seat holds any
/// meaningful priority action. Retained only as a negative control — see
/// `v1_control_*` for why it has no discriminating power for acceptance (a).
fn quiet_board() -> GameState {
    restore_dump(&gunzip_dump(include_bytes!(
        "../fixtures/dina_conqueror_4p.json.gz"
    )))
}

fn dump_driver_forbids(a: &GameAction) -> bool {
    matches!(a, GameAction::Concede { .. } | GameAction::Debug(_))
}

fn dump_beat_actor(state: &GameState) -> Option<(PlayerId, Vec<GameAction>)> {
    if let Some(p) = state.waiting_for.acting_player() {
        let (actions, _costs, _grouped) = engine::ai_support::legal_actions_for_viewer(state, p);
        if !actions.is_empty() {
            return Some((p, actions));
        }
    }
    for p in state.players.iter().map(|p| p.id) {
        let (actions, _costs, _grouped) = engine::ai_support::legal_actions_for_viewer(state, p);
        if !actions.is_empty() {
            return Some((p, actions));
        }
    }
    None
}

/// One beat of the drain-drive policy: at `Priority` ALWAYS pass (the mandatory
/// triggers re-trigger — that IS the loop), answer every other prompt.
fn dump_drive_one_beat(state: &mut GameState) -> Result<(), String> {
    let Some((who, actions)) = dump_beat_actor(state) else {
        return Err(format!("no legal actor at {:?}", state.waiting_for));
    };
    let chosen = if matches!(state.waiting_for, WaitingFor::Priority { .. }) {
        actions
            .iter()
            .find(|a| matches!(a, GameAction::PassPriority))
            .cloned()
    } else {
        actions
            .iter()
            .find(|a| !matches!(a, GameAction::PassPriority) && !dump_driver_forbids(a))
            .or_else(|| actions.iter().find(|a| !dump_driver_forbids(a)))
            .cloned()
    };
    let Some(action) = chosen else {
        return Err(format!("empty action list at {:?}", state.waiting_for));
    };
    apply(state, who, action.clone())
        .map(|_| ())
        .map_err(|e| format!("apply err ({action:?}): {e:?}"))
}

/// Drive real beats until the board mints a bounded offer.
fn drive_to_offer(state: &mut GameState, cap: usize) -> Option<usize> {
    for beat in 0..cap {
        if matches!(state.waiting_for, WaitingFor::LoopShortcut { .. }) {
            return Some(beat);
        }
        if dump_drive_one_beat(state).is_err() {
            return None;
        }
    }
    None
}

// NOTE: no `give_fetchland` staging helper. The live-path test drives the seat's
// OWN Terramorphic Expanse out of the restored dump (`ObjectId(203)`), so there is
// nothing to inject; a staging helper here would have made the live test synthetic
// again. `give_bolt` below survives because the positive control needs an
// interactive card the recorded board does not contain.

/// Stage a castable Lightning Bolt in `player`'s hand, mirroring
/// `GameScenario::add_bolt_to_hand` (same `Effect::DealDamage` ability, same
/// absence of a printed mana cost) so the positive control below is the same
/// interaction the shipped fixtures use.
fn give_bolt(state: &mut GameState, player: PlayerId) -> ObjectId {
    give_bolt_with_cost(state, player, ManaCost::zero())
}

/// `give_bolt` with a PRINTED cost, so a row can stage an interaction the seat
/// cannot yet afford. `GameObject::mana_cost` is the field the castability probe
/// reads, and `ManaCost`'s `Default` is `zero()` (`GameObject::new` seeds both
/// cost fields from it), so the free-Bolt caller above is byte-unchanged.
///
/// BOTH fields are assigned, but the LIVE one is what carries this helper —
/// `base_mana_cost` is NOT load-bearing for the objects staged here, and saying
/// otherwise would be a justification the next reader trusts.
///
/// READ FROM SOURCE (three call sites, not a runtime probe — the evidence grade
/// is stated because overclaiming it is the very habit this comment replaces).
/// `game::layers`' base→live reseed does run
/// `mana_cost = base_mana_cost.clone()` (`seed_live_characteristics_from_base`),
/// and every consumer here does reach the object through
/// `ai_support::shortcut_probe`, which flushes layers — but the full pass
/// applies that reseed (via `reset_recipient_to_base`) only over
/// `battlefield_phased_in_ids()`, and the hand branch of the same pass resets
/// `keywords` alone. `layers::layer_pass_materializes_keywords`' doc is the
/// in-repo authority for that split ("Battlefield — resets the full
/// characteristic set" vs "Hand — keywords-only reset"); the incremental arm
/// resets only battlefield entrants and their hosts, so it cannot reach a hand
/// object either. This object is staged to `Zone::Hand`, so no pass reseeds its
/// `mana_cost`. `GameObject`'s
/// `sync_missing_base_characteristics` — which the hand branch DOES call —
/// would in fact back-fill `base_mana_cost` from the live field, the opposite
/// direction.
///
/// `base_mana_cost` is set for symmetry: it keeps the two fields from
/// disagreeing on a freshly minted object, and it keeps the helper correct if
/// the hardcoded `Zone::Hand` below ever becomes the battlefield, where the
/// reseed WOULD restore the default (free) cost over the printed one and
/// silently leave an "otherwise-unaffordable" premise measuring nothing.
fn give_bolt_with_cost(state: &mut GameState, player: PlayerId, cost: ManaCost) -> ObjectId {
    let card_id = CardId(state.next_object_id);
    let id = engine::game::zones::create_object(
        state,
        card_id,
        player,
        "Lightning Bolt".to_string(),
        Zone::Hand,
    );
    let ability = AbilityDefinition::new(
        AbilityKind::Spell,
        Effect::DealDamage {
            amount: QuantityExpr::Fixed { value: 3 },
            target: TargetFilter::Any,
            damage_source: None,
            excess: None,
        },
    );
    let obj = state.objects.get_mut(&id).expect("just created");
    obj.card_types
        .core_types
        .push(engine::types::card_type::CoreType::Instant);
    obj.base_card_types = obj.card_types.clone();
    obj.base_mana_cost = cost.clone();
    obj.mana_cost = cost;
    obj.abilities = std::sync::Arc::new(vec![ability.clone()]);
    obj.base_abilities = std::sync::Arc::new(vec![ability]);
    state.layers_dirty = LayersDirty::full();
    id
}

/// Declare the offer this board minted, then poll `seat` by walking the APNAP
/// queue with MANUAL Accepts (never the AI's answer, which would stop the
/// queue). Returns the state parked at `seat`'s response window.
fn declare_and_poll(state: &GameState, seat: PlayerId) -> GameState {
    let WaitingFor::LoopShortcut {
        proposer,
        ref schema,
        ..
    } = state.waiting_for
    else {
        panic!(
            "declare_and_poll expects a LoopShortcut window, got {:?}",
            state.waiting_for
        );
    };
    let mut s = state.clone();
    apply(
        &mut s,
        proposer,
        GameAction::DeclareShortcut {
            count: schema.iteration_count.clone(),
            template: None,
        },
    )
    .expect("the proposer declares its own offer");
    for _ in 0..8 {
        match s.waiting_for {
            WaitingFor::RespondToShortcut { player, .. } if player == seat => return s,
            WaitingFor::RespondToShortcut { player, .. } => {
                apply(
                    &mut s,
                    player,
                    GameAction::RespondToShortcut {
                        response: ShortcutResponse::Accept,
                    },
                )
                .expect("manual Accept advances the APNAP queue");
            }
            _ => break,
        }
    }
    panic!(
        "reach-guard: {seat:?} must be polled; stopped at {:?}",
        s.waiting_for
    );
}

/// The non-`PassPriority` actions available to `seat`, rendered with the source
/// object's name / zone / controller so a reach-guard failure names the board
/// rather than an opaque id.
fn non_pass_actions(state: &GameState, seat: PlayerId) -> Vec<String> {
    probe_actions(state, seat)
        .iter()
        .filter(|a| !matches!(a, GameAction::PassPriority))
        .map(|a| match a {
            GameAction::ActivateAbility {
                source_id,
                ability_index,
            } => {
                let o = state.objects.get(source_id);
                format!(
                    "ActivateAbility({source_id:?} {:?} #{ability_index} zone={:?} controller={:?})",
                    o.map(|o| o.name.clone()),
                    o.map(|o| o.zone),
                    o.map(|o| o.controller)
                )
            }
            other => format!("{other:?}"),
        })
        .collect()
}

// ===========================================================================
// V1 / V5 — ACCEPTANCE (a) on the REAL 4-player board, LIVE PATH.
// ===========================================================================

/// **The acceptance row.** A real 4-player Dina / Bloodthirsty Conqueror drain,
/// restored through the production chokepoint
/// (`PersistedGameState::into_game_state()`, the same path the server's
/// `from_persisted` and WASM's `decode_restored_game_state` funnel through) and
/// driven beat by beat through the public `apply()` boundary. No `GameScenario`
/// anywhere on this row: a synthetic board going green while the live 4p case
/// failed is a documented failure mode in this lane.
///
/// The defect, on this exact board: seat P2 controls **`ObjectId(203)`,
/// "Terramorphic Expanse"**, on the battlefield. A non-mana activated ability is
/// unconditionally "meaningful" to stage 1, so the shipped one-stage predicate
/// answered `Shorten { at_iteration: 0 }` — handing P2 a real priority window
/// whose only content is cracking its own fetchland, which cannot touch the
/// drain. Stage 2 answers `Accept`.
///
/// **V5 — bounded offers are NOT exempt.** MEASURED on this board: the offer
/// mints at beat 21 carrying `predicted_winner: None` and
/// `IterationCount::Fixed(25)`. It is the BOUNDED class, not the `UntilLethal`
/// class the synthetic rows use, and `predicted_winner: None` additionally
/// proves arm (A) cannot be what produces the `Accept` below — only arm (B)
/// can. Re-introducing an `UntilLethal`-only gate makes this row return
/// `Shorten` and fail.
///
/// **Why the flip set is exactly {P2}, asserted rather than asserted-about.**
/// P1 and P3 are polled on the same board and hold nothing, so they answer at
/// stage 1 and are unaffected. That is the sibling control: the change is
/// surgical, not a blanket flip to `Accept`.
///
/// MUTANTS — the `Accept` assertion flips under both:
/// * **DROP** (delete stage 2) ⇒ `Shorten { at_iteration: 0 }`, which is the
///   shipped behaviour and therefore the defect itself;
/// * **TRIVIALIZE** (`any_action_may_interfere ≡ true`) ⇒ `Shorten`.
///
/// The opposite direction is `v1_positive_control_*` below, on this same board.
#[test]
fn v1_live_path_fetchland_seat_accepts_on_the_real_4p_board() {
    let mut board = live_path_board();
    assert!(
        !matches!(board.waiting_for, WaitingFor::LoopShortcut { .. }),
        "reach-guard: the dump must not ship AT an offer — the offer is this drive's product, \
         not its input; got {:?}",
        board.waiting_for
    );
    let beat = drive_to_offer(&mut board, 400).expect(
        "CR 732.2a: the offer must FIRE on this real 4p drain. A failure here is the offer \
         never being raised, not a fixture accident",
    );
    let WaitingFor::LoopShortcut {
        predicted_winner,
        ref schema,
        ..
    } = board.waiting_for
    else {
        unreachable!("drive_to_offer only returns at a LoopShortcut window");
    };

    // ── V5's premise, read off the live offer rather than assumed ──
    assert_eq!(
        predicted_winner, None,
        "V5: the BOUNDED class mints no crown — so arm (A) is structurally unable to produce \
         the Accept below, and only arm (B) can (offer beat {beat})"
    );
    assert_eq!(
        schema.iteration_count,
        IterationCount::Fixed(25),
        "V5: a FINITE count is the point — stage 2 takes the identical rule for it and for \
         the UntilLethal class"
    );

    // ── the row: P2, whose only action is its own fetchland ──
    let polled = declare_and_poll(&board, P2);
    let non_pass = non_pass_actions(&polled, P2);

    assert_eq!(
        non_pass.len(),
        1,
        "REACH-GUARD: P2 must hold EXACTLY ONE non-pass action. Two would let the fold's \
         .any() reach Shorten through the other one and this row would pass for the wrong \
         reason; zero would make it the stage-1 path; got {non_pass:?}"
    );
    assert!(
        non_pass[0].contains("Terramorphic Expanse")
            && non_pass[0].contains("zone=Some(Battlefield)")
            && non_pass[0].contains("controller=Some(PlayerId(2))"),
        "REACH-GUARD: that one action must be P2's OWN battlefield fetchland — the object the \
         diagnosis pinned; got {non_pass:?}"
    );

    // NON-VACUITY PIN. The guards above read the FLAT list, which cannot contain
    // a BATTLEFIELD mana activation: `candidates.rs` excludes it at generation
    // (`!is_mana_ability(&ability_def)`), and a land's `TapLandForMana` is
    // additionally dropped by `flat_priority_actions_with_probe`'s
    // `GameAction::is_mana_ability` filter. (It CAN contain a hand- or
    // graveyard-zone mana activation, which has its own candidate loop and is a
    // `GameAction::ActivateAbility` — so the filter never sees it. That class is
    // not on this board, and what would catch it is the `non_pass.len() == 1`
    // REACH-GUARD above, NOT the assertion below: such an activation is already
    // IN the flat list, so it shows up as a second non-pass action, while
    // `stage_two_action_set` only APPENDS `meaningful_sacrifice_mana_actions` —
    // a non-sacrifice one adds nothing and `stage_two == flat` still holds.)
    // The set stage 2 actually folds over is WIDER still — `stage_two_action_set`
    // re-admits sacrifice-for-mana activations — so without this the flagship is
    // blind to exactly the class that would vacuate the feature: a seat that
    // acquired a Lotus-Petal-shaped permanent during the drive would silently
    // start Shortening and this row would flip.
    let (probe, flat) = engine::ai_support::shortcut_probe(&polled, P2);
    let stage_two = engine::ai_support::stage_two_action_set(probe.state(), &flat);
    assert_eq!(
        stage_two, flat,
        "NON-VACUITY: P2 must own NO mana-producing action on this board, so its Accept is \
         produced by the fetchland's confinement and NOT by the absence of a widening. If this \
         fails, the flagship Accept is no longer measuring what it claims — re-derive the row, \
         do NOT relax the assertion"
    );

    assert!(
        stage_one_meaningful(&polled, P2),
        "REACH-GUARD: stage 1 (POSSIBILITY, untouched here) must still return true. An Accept \
         produced by stage 1 would prove nothing about stage 2 — this is the assertion that \
         makes the row non-vacuous"
    );

    assert_eq!(
        engine::ai_support::smart_shortcut_response(&polled, P2),
        ShortcutResponse::Accept,
        "ACCEPTANCE (a), LIVE PATH: cracking its own fetchland cannot touch the drain, so P2 \
         must not buy a priority window with it. CR 732.2c is satisfied by ANY different \
         choice, which is exactly why satisfying it carries no efficacy"
    );

    // ── sibling control: the flip set is exactly {P2} ──
    for seat in [P1, P3] {
        let other = declare_and_poll(&board, seat);
        assert!(
            !stage_one_meaningful(&other, seat),
            "sibling control: {seat:?} holds nothing on this board, so it answers at stage 1 \
             and stage 2 never runs for it; got {:?}",
            non_pass_actions(&other, seat)
        );
        assert_eq!(
            engine::ai_support::smart_shortcut_response(&other, seat),
            ShortcutResponse::Accept,
            "sibling control: {seat:?} is unchanged by this fix — the flip set is exactly {{P2}}"
        );
    }
}

/// The positive control for the row above, on the SAME real board: give P2 a
/// castable Lightning Bolt and it must Shorten.
///
/// This is what makes `v1_live_path_*`'s `Accept` attributable. The same
/// instrument, on the same restored 4p board, at the same offer, returns BOTH
/// values — so the `Accept` is caused by the fetchland's confinement and not by
/// anything about the board, the beat, or the offer class. Without this row a
/// classifier that answered `Accept` unconditionally would pass.
///
/// MUTANT: `any_action_may_interfere ≡ false` ⇒ `Accept` — this row flips.
///
/// This row's expected value (`Shorten`) IS the shipped value, so **DROP cannot
/// flip it**. Its whole discriminating power is the TRIVIALIZE arm named above.
#[test]
fn v1_positive_control_interactive_seat_still_shortens_on_the_real_4p_board() {
    let mut board = live_path_board();
    drive_to_offer(&mut board, 400).expect("the offer must fire");
    let bolt = give_bolt(&mut board, P2);

    let polled = declare_and_poll(&board, P2);
    let actions = probe_actions(&polled, P2);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, GameAction::CastSpell { object_id, .. } if *object_id == bolt)),
        "REACH-GUARD: the Bolt must actually be castable here, or this control cannot fire; \
         got {:?}",
        non_pass_actions(&polled, P2)
    );

    assert_eq!(
        engine::ai_support::smart_shortcut_response(&polled, P2),
        ShortcutResponse::Shorten { at_iteration: 0 },
        "ACCEPTANCE (b), LIVE PATH: a seat holding real interaction must still get its window. \
         This is the assertion any over-broad confinement rule destroys first"
    );
}

/// NEGATIVE CONTROL — and its limits are the point.
///
/// The previously-tracked `dina_conqueror_4p` board is the same matchup at a
/// beat where NO seat holds any meaningful priority action. MEASURED: the offer
/// mints at beat 19 with `predicted_winner: None`, `IterationCount::Fixed(30)`,
/// and all three polled seats (P1, P2, P3) enumerate exactly
/// `["PassPriority"]` — length one, NOT empty; it is
/// `has_meaningful_priority_action` returning false that produces the Accept,
/// not an empty action vector.
///
/// **This board therefore has ZERO discriminating power for acceptance (a), and
/// it is retained only for what it CAN show.** The defect needs a seat holding
/// a meaningful-but-vacuous action; a board with no such seat cannot exhibit it,
/// so this row passes identically with and without stage 2. Do not promote it
/// to an acceptance row, and do not read its green as evidence about the fix:
/// what it pins is the one-way property that stage 2 must not make a quiet
/// board start Shortening.
#[test]
fn v1_control_quiet_board_is_unchanged_and_cannot_discriminate() {
    let mut board = quiet_board();
    let beat = drive_to_offer(&mut board, 400).expect("the quiet board still mints an offer");
    let WaitingFor::LoopShortcut {
        predicted_winner,
        ref schema,
        ..
    } = board.waiting_for
    else {
        unreachable!()
    };
    assert_eq!(predicted_winner, None, "bounded class (offer beat {beat})");
    assert_eq!(schema.iteration_count, IterationCount::Fixed(30));

    for seat in [P1, P2, P3] {
        let polled = declare_and_poll(&board, seat);
        let actions = probe_actions(&polled, seat);
        assert_eq!(
            actions,
            vec![GameAction::PassPriority],
            "the premise of this control: {seat:?} enumerates exactly one action, and it is a \
             pass. If this ever fails the board is no longer quiet and the row's `cannot \
             discriminate` claim needs re-deriving"
        );
        assert!(
            !stage_one_meaningful(&polled, seat),
            "and it is stage 1, not an empty action list, that answers"
        );
        assert_eq!(
            engine::ai_support::smart_shortcut_response(&polled, seat),
            ShortcutResponse::Accept,
            "no-regress: stage 2 must not make a quiet seat start Shortening"
        );
    }
}

// ===========================================================================
// V8 — the SECOND window this authority answers: the pre-cast copy route.
// ===========================================================================

const PRECAST_EPOCH: u64 = 7;
const PRECAST_BREAKPOINT: u64 = 99;

/// Re-park an already-polled state at the PRE-CAST responder window, board
/// untouched.
///
/// Hand-built from the engine's own constructor shape
/// (`game::precast_copy_shortcut::responder_wait`), exactly as the shipped
/// `precast_copy_shortcut.rs` fixture `precast_shortcut_response_state` does.
/// Sound HERE specifically: `smart_shortcut_response` reads `waiting_for` for
/// one thing only (the crown — and this variant carries no crown to read) and
/// then re-parks its own probe clone at `Priority`, so the efficacy answer is a
/// function of the BOARD. Driving a genuine pre-cast copy route would supply a
/// different board, which is the one variable this row must hold fixed against
/// `v1b`/`v3` above.
fn as_precast_window(state: &GameState, seat: PlayerId) -> GameState {
    let mut s = state.clone();
    s.waiting_for = WaitingFor::RespondToPrecastCopyShortcut {
        player: seat,
        epoch: PRECAST_EPOCH,
        breakpoint_ids: vec![PRECAST_BREAKPOINT],
        remaining_players: Vec::new(),
    };
    s
}

/// The pre-cast reply the PRODUCTION candidate builder emits for this state.
/// Goes through `ai_support::candidate_actions`, i.e. the real consumer at
/// `candidates::candidate_actions_broad_with_probe`, so the
/// `ShortcutResponse` → `PrecastCopyShortcutResponse` mapping is measured too.
fn precast_candidate_response(state: &GameState) -> PrecastCopyShortcutResponse {
    let replies: Vec<PrecastCopyShortcutResponse> = engine::ai_support::candidate_actions(state)
        .iter()
        .filter_map(|candidate| match &candidate.action {
            GameAction::PrecastCopyShortcut { epoch, response } if *epoch == PRECAST_EPOCH => {
                Some(response.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        replies.len(),
        1,
        "reach-guard: the pre-cast responder window must offer exactly one reply candidate, \
         otherwise this row is reading something else; got {replies:?}"
    );
    replies[0].clone()
}

/// `smart_shortcut_response` is the single authority for TWO accept-or-shorten
/// windows, not one: `candidates::candidate_actions_broad_with_probe` routes
/// `WaitingFor::RespondToPrecastCopyShortcut` through it as well and maps the
/// answer onto `PrecastCopyShortcutResponse`. Stage 2 therefore changed behaviour
/// at that window too, and this row measures it instead of assuming it.
///
/// Uniform treatment is the deliberate choice: both windows ask the identical
/// question — is a real priority window worth taking here — so a seat whose only
/// action cannot touch the loop should decline both. Arm (A) is separately
/// INAPPLICABLE here rather than merely skipped: `RespondToPrecastCopyShortcut`
/// carries no proposal summary and hence no `predicted_winner` field, so there is
/// no crown to read. Stage 1 and arm (B) both apply and both run.
///
/// NON-VACUITY, and it is arm 2 that supplies it: `candidates.rs` maps a
/// `Shorten` with an EMPTY `breakpoint_ids` back to `Accept`, so on a
/// breakpoint-less prompt both answers would collapse to `Accept` and arm 1
/// would pass for free. Arm 2 returns `Shorten { breakpoint_id }` off the same
/// staged breakpoint list, which proves the mapping is live and arm 1's `Accept`
/// is the efficacy verdict rather than the collapse.
///
/// MUTANTS — both RUN, not reasoned about:
/// * `any_action_may_interfere ≡ true` ⇒ arm 1's production-path assertion fails
///   with `left: Shorten { breakpoint_id: 99 }, right: Accept`. This is also the
///   direct measurement of the non-vacuity claim above: the mapping's `Shorten`
///   branch really is reachable on this prompt.
/// * `any_action_may_interfere ≡ false` ⇒ arm 2 fails with
///   `left: Accept, right: Shorten { breakpoint_id: 99 }`.
///
/// DROP (delete stage 2 entirely) flips arm 1 the same way and leaves arm 2 at
/// its shipped value; the first mutant covers that direction.
#[test]
fn v8_precast_window_takes_the_same_efficacy_answer() {
    // Arm 1 — the confined fetchland seat.
    let (mut runner, kickoff) = optional_cascade(|s| {
        s.add_land_from_oracle(P2, "Terramorphic Expanse", TERRAMORPHIC);
    });
    respond_window_at(&mut runner, kickoff, P2);
    let fetch_precast = as_precast_window(runner.state(), P2);

    assert!(
        stage_one_meaningful(&fetch_precast, P2),
        "REACH-GUARD: stage 1 must still say `meaningful` at the PRE-CAST window, or this arm \
         measures the stage-1 path and says nothing about stage 2; got {:?}",
        non_pass_actions(&fetch_precast, P2)
    );
    // The PRODUCTION-PATH assertion comes first deliberately: it is the one that
    // has to discriminate, and an authority-level assertion ahead of it would
    // absorb every mutant before the candidate builder was ever exercised.
    assert_eq!(
        precast_candidate_response(&fetch_precast),
        PrecastCopyShortcutResponse::Accept,
        "the pre-cast candidate builder must carry stage 2's answer through: a window whose only \
         content is cracking one's own fetchland is worth no more on the pre-cast route than on \
         the generic one"
    );
    assert_eq!(
        engine::ai_support::smart_shortcut_response(&fetch_precast, P2),
        ShortcutResponse::Accept,
        "and the authority itself answers identically at both windows"
    );

    // Arm 2 — the same board plus real interaction. The window is still granted.
    let (mut armed, armed_kickoff) = optional_cascade(|s| {
        s.add_land_from_oracle(P2, "Terramorphic Expanse", TERRAMORPHIC);
        s.add_basic_land(P2, ManaColor::Red);
        s.add_bolt_to_hand(P2);
    });
    respond_window_at(&mut armed, armed_kickoff, P2);
    let armed_precast = as_precast_window(armed.state(), P2);

    assert!(
        probe_actions(&armed_precast, P2)
            .iter()
            .any(|a| matches!(a, GameAction::CastSpell { .. })),
        "reach-guard: the Bolt must be castable at the pre-cast window too, or this arm repeats \
         arm 1; got {:?}",
        non_pass_actions(&armed_precast, P2)
    );
    assert_eq!(
        precast_candidate_response(&armed_precast),
        PrecastCopyShortcutResponse::Shorten {
            breakpoint_id: PRECAST_BREAKPOINT
        },
        "ACCEPTANCE (b) on the pre-cast route: a seat holding real interaction keeps its window, \
         named at the breakpoint the engine issued to it. This is also arm 1's non-vacuity proof \
         — the Shorten branch of the mapping is reachable on this exact prompt"
    );
}

// ===========================================================================
// V9 — COVERAGE INVARIANT: stage 2 classifies everything stage 1 counted.
// ===========================================================================

/// Krark-Clan Ironworks, verbatim, verified byte-for-byte against MTGJSON
/// `AtomicCards.json` (`.data["Krark-Clan Ironworks"][0].text`; `.types` is
/// `["Artifact"]`). CARD PROVENANCE, in the sense the header block above defines
/// — it is the shape under test, so a paraphrase could take a different parser
/// branch and go green while the real card stayed broken.
///
/// Why THIS card: its activation is the issue #544 shape — a sacrifice-for-mana
/// ability that `legal_actions` structurally omits while
/// `has_meaningful_priority_action`'s second rung still counts it off `state`.
/// That gap between the two stages' inputs is the whole subject of this section.
const IRONWORKS: &str = "Sacrifice an artifact: Add {C}{C}.";

/// Stage the Ironworks on `player`'s battlefield, ability taken from the REAL
/// parser (see `give_parsed_card`, which this is now one call into).
///
/// It was an inlined copy of that helper until the two were diffed field by
/// field and found byte-equivalent — same parse call, same assertion text once
/// `name` is substituted, same `create_object` arguments, same core-type push,
/// same `base_card_types`/`abilities`/`base_abilities` assignment, same
/// `layers_dirty`. Delegating is behaviour-identical BY CONSTRUCTION, and the
/// divergence it prevents already fired once inside this same change:
/// `base_mana_cost` reached one construction path and not the other.
fn give_ironworks(state: &mut GameState, player: PlayerId) -> ObjectId {
    give_parsed_card(
        state,
        player,
        "Krark-Clan Ironworks",
        IRONWORKS,
        CoreType::Artifact,
        Zone::Battlefield,
    )
}

/// A vanilla artifact for the Ironworks to eat. No abilities and no card claim:
/// it exists so the sacrifice cost is payable, and it must contribute no action
/// of its own or it would give the fold a second way to reach `Shorten`.
fn give_artifact_fodder(state: &mut GameState, player: PlayerId) -> ObjectId {
    let id = engine::game::zones::create_object(
        state,
        CardId(state.next_object_id),
        player,
        "Test Artifact Fodder".to_string(),
        Zone::Battlefield,
    );
    let obj = state.objects.get_mut(&id).expect("just created");
    obj.card_types
        .core_types
        .push(engine::types::card_type::CoreType::Artifact);
    obj.base_card_types = obj.card_types.clone();
    obj.summoning_sick = false;
    state.layers_dirty = LayersDirty::full();
    id
}

/// The optional-cascade board with an Ironworks + one artifact staged on P2
/// AFTER the response window opens, so the addition cannot perturb the drive to
/// the offer.
fn ironworks_seat_polled() -> (GameRunner, ObjectId) {
    let (mut runner, kickoff) = optional_cascade(|_| {});
    respond_window_at(&mut runner, kickoff, P2);
    let ironworks = give_ironworks(runner.state_mut(), P2);
    let _fodder = give_artifact_fodder(runner.state_mut(), P2);
    (runner, ironworks)
}

/// The invariant, asserted directly on the set rather than inferred from a
/// verdict: **stage 2 folds over every action stage 1 counted as meaningful.**
///
/// This is the row that survives a future reclassification. `v9b` below reads
/// the Ironworks' *verdict*, which a later, more precise `filter_is_actor_owned`
/// could legitimately flip (CR 701.21a: "A player can't sacrifice something that
/// isn't a permanent, or something that's a permanent they don't control" — so
/// "Sacrifice an artifact" IS actor-owned in fact, merely not PROVEN so by this
/// AST). This row does not depend on the verdict at all — it
/// pins that the action is *handed to the classifier*, which is what keeps a
/// newly added stage-1 rung from silently reintroducing Accept-by-omission.
///
/// The three assertions are the defect's three premises, in order:
///  1. the activation is ABSENT from the flat list (issue #544 grouping), so
///  2. stage 1 nonetheless counts it — via the `state`-reading second rung — and
///  3. `stage_two_action_set` therefore has to put it back, or the two stages
///     read different inputs.
///
/// Revert-probe (EXECUTED, see the report): defining `stage_two_action_set` as
/// `flat_actions.to_vec()` fails assertion 3.
#[test]
fn v9a_stage_two_folds_over_every_action_stage_one_counted() {
    let (runner, ironworks) = ironworks_seat_polled();
    let activation = GameAction::ActivateAbility {
        source_id: ironworks,
        ability_index: 0,
    };

    let (probe, flat) = engine::ai_support::shortcut_probe(runner.state(), P2);
    assert!(
        !flat.contains(&activation),
        "PREMISE 1: sacrifice-for-mana stays out of the flat priority list (issue #544) — if it \
         were present, the two stages would already agree and this row would be vacuous; got \
         {flat:?}"
    );
    assert!(
        stage_one_meaningful(runner.state(), P2),
        "PREMISE 2: stage 1 counts it anyway, off `state` rather than off that list — this is the \
         asymmetry the invariant exists to close; got {flat:?}"
    );

    let stage_two = engine::ai_support::stage_two_action_set(probe.state(), &flat);
    assert!(
        stage_two.contains(&activation),
        "THE INVARIANT: every action stage 1 counted as meaningful must be handed to stage 2. An \
         action the classifier never sees reaches no arm, so the fail-closed default cannot save \
         it and the seat Accepts BY OMISSION; got {stage_two:?}"
    );
}

/// The response-level discriminator: with the invariant restored, this seat
/// Shortens; without it, it Accepts.
///
/// It discriminates because the Ironworks activation classifies `MayInterfere`,
/// and since V10 it does so through TWO independent legs. Its `Sacrifice` cost
/// filter (`Typed{Artifact}`) names no controller, so `filter_is_actor_owned`
/// cannot PROVE actor ownership and `cost_window_reach` takes the fail-closed
/// direction; and its `Effect::Mana` head is no longer allowlisted either, so
/// the head alone would carry the verdict.
///
/// The counterfactual this doc used to carry — "a sacrifice ability whose filter
/// *were* proven actor-owned would classify `OwnResourcesOnly` … i.e. would not
/// discriminate" — is FALSE since `Effect::Mana` left the allowlist, and
/// `v10b` is the row that refutes it on exactly that shape (Lotus Petal's
/// `SelfRef` sacrifice IS proven actor-owned, and the seat still Shortens). What
/// survives is the sentence's purpose: the unproven filter is still what makes
/// THIS row's own MUTANT discriminate, because that mutant deletes the widening
/// rather than touching the classifier.
///
/// REACH-GUARD: the flat list is asserted to be EXACTLY `[PassPriority]`. That
/// is what makes the verdict attributable: `PassPriority` is the classifier's
/// one `false` arm, so the flat half cannot reach `Shorten` on its own and the
/// only action that can is the one the widening added.
///
/// MUTANT (EXECUTED, see the report): `stage_two_action_set ≡ flat_actions
/// .to_vec()` — i.e. delete the widening — flips this row to `Accept`.
///
/// This row's verdict is now OVER-DETERMINED, which changes what a future
/// refinement does to it. The doc used to predict that a `filter_is_actor_owned`
/// which learns to prove "Sacrifice an artifact" actor-owned (CR 701.21a bounds
/// the actor to permanents they CONTROL, which is the fact such a refinement
/// would be reading) would red this row; it will not, because the unallowlisted
/// `Effect::Mana` head carries `MayInterfere` unconditionally. The guidance the
/// prediction carried still stands and is the part to keep: if this row ever
/// does red, re-derive the fixture on an ability whose reach is genuinely
/// unproven — NOT delete the row, and NOT weaken the classifier. Its
/// discriminating power comes from its MUTANT rather than from the cost filter's
/// imprecision, and `v9a` above holds the invariant meanwhile.
#[test]
fn v9b_a_sacrifice_for_mana_seat_still_gets_its_window() {
    let (runner, ironworks) = ironworks_seat_polled();
    let activation = GameAction::ActivateAbility {
        source_id: ironworks,
        ability_index: 0,
    };

    let flat = probe_actions(runner.state(), P2);
    assert_eq!(
        flat,
        vec![GameAction::PassPriority],
        "REACH-GUARD: the flat half must be exactly the one action the classifier answers `false` \
         on, or a `Shorten` here is not attributable to the widening; got {flat:?}"
    );
    assert!(
        stage_one_meaningful(runner.state(), P2),
        "reach-guard: stage 1 must return true, or the seat resolves at stage 1 and never reaches \
         the fold under test"
    );

    assert_eq!(
        engine::ai_support::smart_shortcut_response(runner.state(), P2),
        ShortcutResponse::Shorten { at_iteration: 0 },
        "a seat whose only meaningful action is {activation:?} must keep its window: the AST does \
         not prove the sacrificed artifact is this seat's own, and stage 2 must not Accept a \
         reach it cannot rule out"
    );
}

// ===========================================================================
// V10 — MANA IS FUNGIBLE REACH, not a confined own resource.
//
// CR 106.4's first sentence ("that mana goes into a player's mana pool") is the
// half an earlier `Effect::Mana` arm quoted; the rest of the same rule says the
// mana "can be used to pay costs immediately", CR 106.1 says paying costs is
// mana's whole function, and CR 601.2g runs mana abilities during the very cast
// they fund. So producing mana widens what the polled seat can do inside the
// window `game::engine`'s `RespondToShortcut(Shorten)` arm hands back, and the
// classifier — which reads ONE ability's AST and no other object — cannot prove
// otherwise. Both rows below ride the REAL 4p board through the same production
// chokepoint the flagship uses.
// ===========================================================================

/// Dark Ritual, verbatim. CARD PROVENANCE in the sense the header block defines.
/// The `Effect::Mana` head with `cost: None` is the point — nothing but the head
/// can carry this object's verdict.
const DARK_RITUAL: &str = "Add {B}{B}{B}.";

/// Lotus Petal, verbatim. CARD PROVENANCE. This is the maintainer's named
/// "actor-owned sacrifice-for-mana" class: the parser emits "Sacrifice this
/// artifact" as a `TargetFilter::SelfRef`, which `filter_is_actor_owned`
/// returns true for, so the cost leg reads confined and the mana head carries
/// the verdict ALONE. CR 701.21a bounds the actor to permanents they CONTROL,
/// which is the most that predicate can be grounded in — its first sentence
/// sends the sacrificed card to its OWNER's graveyard, so control is not
/// ownership and "confined" is narrower than the predicate's name suggests.
/// `shortcut_efficacy`'s `mana_production_is_reach_not_a_confined_own_resource`
/// quotes the rule in full and names the limit; nothing here rests on it,
/// because the mana head decides this verdict either way. That is also what
/// makes this row not a second `v9b`,
/// whose Ironworks reaches the same verdict through an UNPROVEN cost filter.
const LOTUS_PETAL: &str = "{T}, Sacrifice this artifact: Add one mana of any color.";

/// Sol Ring, verbatim. CARD PROVENANCE. The ORDINARY mana source: no sacrifice
/// leg, so `mana_ability_penalty` is `None` rather than `Sacrifices` and the
/// stage-2 widening must not re-admit it.
const SOL_RING: &str = "{T}: Add {C}{C}.";

/// Stage a real card with its abilities taken from the REAL parser, not
/// hand-built: every verdict below is a function of the AST, so a hand-written
/// `AbilityDefinition` would let this section pass against a shape the pipeline
/// never produces. Parameterized over the three axes the V10 rows vary, and the
/// SINGLE staging path for parsed cards in this file — `give_ironworks` above
/// delegates here rather than keeping the byte-equivalent copy it used to be.
fn give_parsed_card(
    state: &mut GameState,
    player: PlayerId,
    name: &str,
    oracle: &str,
    core_type: CoreType,
    zone: Zone,
) -> ObjectId {
    let parsed = engine::parser::oracle::parse_oracle_text(oracle, name, &[], &[], &[]);
    assert_eq!(
        parsed.abilities.len(),
        1,
        "PREMISE: {name} parses to exactly one ability; got {:?}",
        parsed.abilities
    );
    let id = engine::game::zones::create_object(
        state,
        CardId(state.next_object_id),
        player,
        name.to_string(),
        zone,
    );
    let obj = state.objects.get_mut(&id).expect("just created");
    obj.card_types.core_types.push(core_type);
    obj.base_card_types = obj.card_types.clone();
    // No `summoning_sick = false` here: it would be a no-op that reads as
    // load-bearing. `zones::create_object` documents that it deliberately does
    // NOT set the flag (only the real ETB pipeline's
    // `reset_for_battlefield_entry` does), `add_to_zone` never touches it, and
    // `GameObject::new` already defaults it to `false`.
    obj.abilities = std::sync::Arc::new(parsed.abilities.clone());
    obj.base_abilities = std::sync::Arc::new(parsed.abilities);
    state.layers_dirty = LayersDirty::full();
    id
}

/// The Ritual is staged as an INSTANT and with no printed mana cost, and both
/// are deliberate. Without a core type the sorcery-timing gate refuses the cast
/// at this window and the funder never enters the action set — the row would go
/// vacuous silently. And Dark Ritual's real printed cost is `{B}` while P2
/// controls no mana source, so a printed-cost Ritual would itself be uncastable
/// and the row would measure an empty board twice. What the row measures is the
/// CLASSIFIER, which reads the AST and never the mana cost.
fn give_dark_ritual(state: &mut GameState, player: PlayerId) -> ObjectId {
    give_parsed_card(
        state,
        player,
        "Dark Ritual",
        DARK_RITUAL,
        CoreType::Instant,
        Zone::Hand,
    )
}

/// THE SIZING SITE. This source contributes **exactly 1** to
/// `game::mana_sources`' `feasible_mana_capacity`: its `AnyOneColor` production
/// carries `count: Fixed { value: 1 }`, and that arm of
/// `game::effects::mana`'s `resolve_mana_types_for_ability` returns
/// `vec![mana_type; amount]` — length `amount`, NOT `color_options.len()`.
///
/// MEASURED on this fixture, the only mana-gated action P2 owns at this window
/// is Angel of the Ruins' hand-zone plainscycling (object 210), whose cost is
/// `Composite[Mana{generic 2}, Discard{self_ref}]` — `{2}` generic. **The margin
/// is exactly 1 mana.** ANY staged P2 source contributing 2 or more unlocks that
/// cycling, puts an `ActivateAbility` in P2's flat list, and destroys `v10b`'s
/// attribution — its `non_pass` assertion is what fails, loudly, if that
/// happens. A future edit that raises this source's capacity silently destroys
/// the discrimination, so do NOT "strengthen" it.
fn give_lotus_petal(state: &mut GameState, player: PlayerId) -> ObjectId {
    give_parsed_card(
        state,
        player,
        "Lotus Petal",
        LOTUS_PETAL,
        CoreType::Artifact,
        Zone::Battlefield,
    )
}

/// Capacity **2** (`Colorless { count: Fixed 2 }`), which is why the control it
/// serves asserts re-admission ONLY and never a verdict — see `v10b`.
fn give_sol_ring(state: &mut GameState, player: PlayerId) -> ObjectId {
    give_parsed_card(
        state,
        player,
        "Sol Ring",
        SOL_RING,
        CoreType::Artifact,
        Zone::Battlefield,
    )
}

/// V10a — a CAST mana spell funds an otherwise-unaffordable answer, so the seat
/// must keep its window.
///
/// The pair varies exactly one object: a Dark Ritual in P2's hand. Both arms
/// hold the same `{B}{B}{B}` Bolt, and assertion 1 is the operational definition
/// of "otherwise-unaffordable" — `feasible_mana_capacity` is battlefield-scoped,
/// so a Ritual sitting in HAND contributes 0 and the castability gate
/// structurally cannot see "cast a ritual first, then the Bolt". That two-step
/// is exactly what the priority window buys, and CR 601.2g / CR 117.1d are the
/// rules that make the mana available to pay a cost the moment it is produced.
///
/// The row does NOT assert the post-resolution board: reaching it would require
/// driving the Ritual through the stack, and the row's discriminating power does
/// not depend on it. The arithmetic is carried by the verbatim texts — the
/// Ritual adds `{B}{B}{B}`, the Bolt is printed at `{B}{B}{B}`.
///
/// MUTANT: restoring `Effect::Mana {..} => WindowReach::OwnResourcesOnly` as
/// `effect_window_reach`'s first arm flips the SHORTEN arm to `Accept`. Under it
/// the Ritual's single ability folds `OwnResourcesOnly` (head `Effect::Mana`,
/// `cost: None`), and P2's remaining actions are `PassPriority` — the
/// classifier's one `false` arm — and the fetchland, whose verdict rides
/// untouched arms. The ACCEPT arm is unaffected by the mutation by construction:
/// its action set carries no `Effect::Mana` node at all.
///
/// Every `GameAction::CastSpell` matcher below binds `{ object_id, .. }` and
/// must NOT name `payment_mode`: a Petal- or Ritual-funded cast can be offered
/// as `CastPaymentMode::AutoExceptSacrificialMana`, and a mode-specific matcher
/// would fail with a message claiming the funding does not work.
#[test]
fn v10a_a_cast_mana_spell_that_funds_an_unaffordable_answer_keeps_its_window() {
    let mut board = live_path_board();
    drive_to_offer(&mut board, 400).expect("CR 732.2a: the offer must fire on this real 4p drain");
    // ONE drive, ONE poll, shared by both arms — so staging cannot perturb the
    // drive, the offer schema, or the APNAP walk.
    let polled = declare_and_poll(&board, P2);

    let mut base = polled.clone();
    let bolt = give_bolt_with_cost(
        &mut base,
        P2,
        // `{B}{B}{B}` — sized to exactly what one Dark Ritual adds.
        ManaCost::Cost {
            shards: vec![ManaCostShard::Black; 3],
            generic: 0,
        },
    );

    // ── arm ACCEPT: the interaction alone ──
    let accept_arm = base.clone();
    // ── arm SHORTEN: same board, same Bolt, PLUS the funding piece ──
    let mut shorten_arm = base.clone();
    let ritual = give_dark_ritual(&mut shorten_arm, P2);

    for (label, arm) in [("ACCEPT", &accept_arm), ("SHORTEN", &shorten_arm)] {
        assert!(
            !probe_actions(arm, P2).iter().any(
                |a| matches!(a, GameAction::CastSpell { object_id, .. } if *object_id == bolt)
            ),
            "PREMISE ({label} arm): the Bolt must be OTHERWISE-UNAFFORDABLE at poll time — a \
             hand-zone Ritual is invisible to the battlefield-scoped capacity scan, so the \
             castability gate cannot see the two-step the window buys; got {:?}",
            non_pass_actions(arm, P2)
        );
        assert!(
            stage_one_meaningful(arm, P2),
            "reach-guard ({label} arm): stage 1 must return true, or the seat answers at stage 1 \
             and the fold under test never runs"
        );
    }

    assert!(
        probe_actions(&shorten_arm, P2)
            .iter()
            .any(|a| matches!(a, GameAction::CastSpell { object_id, .. } if *object_id == ritual)),
        "reach-guard: the FUNDER must really be castable, or the SHORTEN arm is the ACCEPT arm \
         with extra steps; got {:?}",
        non_pass_actions(&shorten_arm, P2)
    );

    let accept_non_pass = non_pass_actions(&accept_arm, P2);
    assert_eq!(
        accept_non_pass.len(),
        1,
        "ATTRIBUTION: the ACCEPT arm's action set must be the flagship's exactly, so its Accept \
         is the already-shipped verdict and the pair's ONLY variable is the Ritual; got \
         {accept_non_pass:?}"
    );
    assert!(
        accept_non_pass[0].contains("Terramorphic Expanse")
            && accept_non_pass[0].contains("zone=Some(Battlefield)")
            && accept_non_pass[0].contains("controller=Some(PlayerId(2))"),
        "ATTRIBUTION: that one action must be P2's OWN battlefield fetchland; got \
         {accept_non_pass:?}"
    );

    assert_eq!(
        engine::ai_support::smart_shortcut_response(&accept_arm, P2),
        ShortcutResponse::Accept,
        "the pair's negative arm: an unaffordable answer and a confined fetchland buy nothing"
    );
    assert_eq!(
        engine::ai_support::smart_shortcut_response(&shorten_arm, P2),
        ShortcutResponse::Shorten { at_iteration: 0 },
        "the pair's positive arm: producing mana is REACH (CR 106.1 / CR 106.4 / CR 601.2g). \
         The ritual is the one object that differs, and accepting here surrenders a live out"
    );
}

/// V10b — an ACTOR-OWNED sacrifice-for-mana seat keeps its window. This is the
/// maintainer's named class, and it is the row `v9b` structurally cannot be.
///
/// The pair varies exactly ONE object: a Lotus Petal on P2's battlefield. No
/// Bolt, no Swamp, no second permanent.
///
/// **CAPACITY THRESHOLD — the sizing this row lives or dies on.** The staged
/// source contributes exactly **1** to `feasible_mana_capacity` (see
/// `give_lotus_petal`), and the only mana-gated action P2 owns at this window is
/// Angel of the Ruins' hand-zone plainscycling at `{2}` generic. **The margin is
/// exactly 1 mana.** ANY staged P2 source contributing 2 or more unlocks that
/// cycling, puts an `ActivateAbility` in P2's flat list, and destroys this row's
/// attribution — the `non_pass` assertion below is what fails, loudly, if that
/// happens. A row that reds that way is a fixture-threshold artifact, not a
/// defect in the classifier: check whether `non_pass_actions` names object 210
/// first, shrink the staged source, and do NOT respond by relaxing an
/// `Effect::Mana` classification.
///
/// **P2's full reachable surface at this window, enumerated so the threshold is
/// not a claim about the hand alone.** MEASURED from the committed fixture:
/// battlefield 1 (Terramorphic Expanse, capacity 0); hand 7, of which six are
/// sorcery-speed (`Victimize`, `Plains`, `Arcane Signet`, `Commander's Sphere`,
/// `Compleated Huntmaster`, `Night's Whisper`) and only the Angel offers a
/// mana-gated instant-speed action; command zone 1 — `Brimaz, Blight of
/// Oreskos`, `{2}{W}{B}`, a CREATURE, and `format_config.command_zone` is true,
/// so `casting::spell_objects_available_to_cast`'s `Zone::Command` clause does
/// put it inside the candidate loop. The fixture is `active_player` 0 in
/// `CombatDamage` with priority at P2, so it is neither P2's turn nor a main
/// phase and sorcery-speed timing blocks Brimaz — and no amount of mana lifts a
/// timing gate. Library-zone activations are gated `is_active && stack_empty`
/// and P2 is not active. So `{2}` really is the whole threshold, over the whole
/// surface rather than over the hand.
///
/// MUTANT: restoring `Effect::Mana {..} => WindowReach::OwnResourcesOnly` flips
/// the SHORTEN arm to `Accept`. The Petal's cost leg is ALREADY
/// `OwnResourcesOnly` — `Composite[Tap, Sacrifice{SelfRef}]`, and
/// `filter_is_actor_owned` proves `SelfRef` at its first match arm — so with the
/// mana arm restored the whole ability folds `OwnResourcesOnly`. That is exactly
/// why this row is not a second `v9b`, whose verdict rides its UNPROVEN cost
/// filter and is untouched by the mutation.
#[test]
fn v10b_an_actor_owned_sacrifice_for_mana_seat_keeps_its_window() {
    let mut board = live_path_board();
    drive_to_offer(&mut board, 400).expect("CR 732.2a: the offer must fire on this real 4p drain");
    let polled = declare_and_poll(&board, P2);

    // ── arm ACCEPT: the polled board, untouched ──
    let accept_arm = polled.clone();
    // ── arm SHORTEN: the polled board plus ONE object ──
    let mut shorten_arm = polled.clone();
    let petal = give_lotus_petal(&mut shorten_arm, P2);
    let activation = GameAction::ActivateAbility {
        source_id: petal,
        ability_index: 0,
    };

    assert!(
        !probe_actions(&shorten_arm, P2).contains(&activation),
        "PREMISE: the Petal's ability IS a mana ability (CR 605.1a), so candidate generation \
         excludes it from the flat list outright. This is the issue-#544 asymmetry the whole \
         V9/V10 section exists for — if it were present, stage 2 would already see it and the \
         widening below would be vacuous"
    );

    let non_pass = non_pass_actions(&shorten_arm, P2);
    assert_eq!(
        non_pass.len(),
        1,
        "ATTRIBUTION + THRESHOLD SENTINEL: if the Petal's mana made ANY P2 action affordable — a \
         hand card, or the Angel's {{2}} cycling — it would appear here and could carry \
         MayInterfere independently of the arm under test. See this row's capacity-threshold \
         note before touching the staged source; got {non_pass:?}"
    );
    assert!(
        non_pass[0].contains("Terramorphic Expanse")
            && non_pass[0].contains("zone=Some(Battlefield)")
            && non_pass[0].contains("controller=Some(PlayerId(2))"),
        "ATTRIBUTION: that one action must still be P2's OWN battlefield fetchland; got \
         {non_pass:?}"
    );

    let (probe, flat) = engine::ai_support::shortcut_probe(&shorten_arm, P2);
    let mut expected = flat.clone();
    expected.push(activation);
    assert_eq!(
        engine::ai_support::stage_two_action_set(probe.state(), &flat),
        expected,
        "the widening added EXACTLY the Petal. Order is determinate — `stage_two_action_set` is \
         the flat list chained with the meaningful sacrifice-mana actions — and the penalty is \
         `Sacrifices` because `mana_ability_penalty`'s FIRST clause is `cost_includes_sacrifice`, \
         which inspects `Composite` legs"
    );

    let (accept_probe, accept_flat) = engine::ai_support::shortcut_probe(&accept_arm, P2);
    assert_eq!(
        engine::ai_support::stage_two_action_set(accept_probe.state(), &accept_flat),
        accept_flat,
        "the negative half of the widening, on the same instrument: without the Petal there is \
         nothing to re-admit"
    );

    for (label, arm) in [("ACCEPT", &accept_arm), ("SHORTEN", &shorten_arm)] {
        assert!(
            stage_one_meaningful(arm, P2),
            "reach-guard ({label} arm): stage 1 must return true, or the seat answers at stage 1 \
             and the fold under test never runs"
        );
    }

    assert_eq!(
        engine::ai_support::smart_shortcut_response(&accept_arm, P2),
        ShortcutResponse::Accept,
        "the pair's negative arm — and it is the flagship's own verdict on the flagship's own \
         action set"
    );
    assert_eq!(
        engine::ai_support::smart_shortcut_response(&shorten_arm, P2),
        ShortcutResponse::Shorten { at_iteration: 0 },
        "the pair's positive arm: an actor-owned sacrifice-for-mana activation is two board \
         events, not a confined own resource. Stage 1 re-admits it PRECISELY because the \
         sacrifice is board-changing, so classifying it as confined here contradicted the stage \
         that handed it over"
    );

    // ── FUNDING LEMMA (CR 117.1d + CR 601.2g), deliberately QUARANTINED ──
    //
    // These two clones NEVER reach `smart_shortcut_response`. Staging the probe
    // spell into an arm above would put a `CastSpell` in the flat list whose own
    // `Effect::DealDamage` reaches the fail-closed arm — the pair would then
    // Shorten with the fix reverted and would stop measuring anything. The
    // separation IS the design; do not merge them.
    //
    // `{1}` generic on purpose: a generic residual is decided by comparing the
    // summed capacity against it, so both halves are decided on one read path
    // with zero slack. Without the Petal, P2's battlefield is one Terramorphic
    // Expanse, whose ability heads `SearchLibrary` and contributes 0, so the sum
    // is 0 < 1; with the Petal it is exactly 1 >= 1.
    let mut funded = shorten_arm.clone();
    let probe_spell = give_bolt_with_cost(&mut funded, P2, ManaCost::generic(1));
    assert!(
        probe_actions(&funded, P2).iter().any(
            |a| matches!(a, GameAction::CastSpell { object_id, .. } if *object_id == probe_spell)
        ),
        "FUNDING: with the Petal on the battlefield the engine's own castability gate says a \
         {{1}} interaction IS payable — by activating a mana ability during cost payment, which \
         is exactly what happens inside the window this row buys (CR 117.1d / CR 601.2g); got \
         {:?}",
        non_pass_actions(&funded, P2)
    );

    let mut unfunded = accept_arm.clone();
    let unfunded_spell = give_bolt_with_cost(&mut unfunded, P2, ManaCost::generic(1));
    assert!(
        !probe_actions(&unfunded, P2).iter().any(
            |a| matches!(a, GameAction::CastSpell { object_id, .. } if *object_id == unfunded_spell)
        ),
        "FUNDING, negative half: the SAME interaction on the SAME board minus the Petal is NOT \
         payable. Together with the assertion above, the Petal's mana is what makes it reachable \
         — which is the whole content of 'otherwise-unaffordable'; got {:?}",
        non_pass_actions(&unfunded, P2)
    );

    // ── the ORDINARY-mana-source control: plain mana never leaks into stage 2 ──
    let mut ordinary = polled.clone();
    let sol_ring = give_sol_ring(&mut ordinary, P2);
    let (ordinary_probe, ordinary_flat) = engine::ai_support::shortcut_probe(&ordinary, P2);
    assert!(
        !ordinary_flat.iter().any(
            |a| matches!(a, GameAction::ActivateAbility { source_id, .. } if *source_id == sol_ring)
        ),
        "candidate generation excludes a mana ability outright (CR 605.1a); got {ordinary_flat:?}"
    );
    assert_eq!(
        engine::ai_support::stage_two_action_set(ordinary_probe.state(), &ordinary_flat),
        ordinary_flat,
        "NON-VACUITY: an ordinary mana source has penalty `None`, not `Sacrifices`, so the \
         stage-2 widening does NOT re-admit it. The Lotus Petal DOES enter this same set in this \
         same test, so this emptiness is a measurement and not a stuck instrument"
    );
    // NO verdict assertion here, deliberately. Sol Ring contributes 2 to
    // `feasible_mana_capacity`, which is enough to pay Angel of the Ruins'
    // hand-zone plainscycling already sitting in P2's hand on this fixture. That
    // activation enters the FLAT list and is `MayInterfere` on two independent
    // legs (`AbilityCost::Discard` is not allowlisted; its `sub_ability` moves a
    // card to a HAND, which the anaphoric fetch disjunct does not cover), so
    // this seat answers Shorten — through candidate affordability, a route this
    // control does not model and claims nothing about. Asserting Accept here
    // would assert a false fact about the board.
}
