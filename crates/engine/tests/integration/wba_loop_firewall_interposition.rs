//! CR 732.2a INTERPOSITION acceptance — the loop-shortcut firewall must not veto an offer on
//! account of a replacement effect that is SPENT for the proposed window.
//!
//! **The posture this file serves.** CR 732.2b gives every other player a mechanism for
//! deviating from a proposed shortcut: *"[each other player] may either accept the proposed
//! sequence, or shorten it by naming a place where they will make a game choice that's
//! different than what's been proposed."* A firewall that pre-emptively vetoes on a permanent
//! that merely *observes* is guessing at a declaration the rules assign to a player. A veto
//! belongs only where something on the board would actually falsify the proposed ending state.
//!
//! **What is spent, and why (CR 614.1d + CR 614.12 + CR 400.7).** An "enters tapped unless you
//! control …" land carries a replacement definition whose only subject is its OWN entrance
//! (CR 614.1d templates "[This permanent] enters . . ." separately from "[Objects] enter . . .";
//! CR 614.12 makes the first apply only to that permanent). Once that land is on the
//! battlefield and stays the same object across the window (CR 400.7), the event it watches
//! cannot recur inside the window, so none of its surfaces runs — however loudly its condition
//! would census the board if it ever did. That is INAPPLICABILITY, not disjointness, which is
//! why the relief reaches lands whose census genuinely counts the growing class.
//!
//! **BASE BOARD.** The REAL 4-player `sprout_witherbloom_realistic_lands_4p` dump, loaded
//! through the production restore chokepoint and driven to its CR 732.2a offer by one live
//! Sprout Swarm buyback+convoke recast, using `sprout_inalla_realistic_offer`'s own helpers.
//! Every arm below is ONE OBJECT away from the shipped-green board, which is what makes the
//! pairs discriminating rather than merely green.
//!
//! **ORACLE TEXT IS VERBATIM**, taken from the pinned card-data export, never a paraphrase: a
//! reworded "enters tapped unless" line can take a different parser branch and go green while
//! the real card still vetoes.

use std::sync::Arc;

use engine::game::zones::create_object;
use engine::types::ability::{ReplacementDefinition, TargetFilter, TypedFilter};
use engine::types::card_type::CoreType;
use engine::types::game_state::{GameState, WaitingFor};
use engine::types::identifiers::{CardId, ObjectId};
use engine::types::player::PlayerId;
use engine::types::replacements::ReplacementEvent;
use engine::types::zones::Zone;

use super::sprout_inalla_realistic_offer::{drive_sprout_cast, load_realistic_dump};

const P0: PlayerId = PlayerId(0);

/// The three census lands this row drives, each with its VERBATIM Oracle text and its printed
/// subtypes. All three parse to one `UnlessControlsMatching` entry replacement — a live
/// battlefield census that NO per-condition relief arm in `analysis/resource.rs` matches, so a
/// green arm below cannot be a sibling arm's verdict wearing this row's name.
///
/// They are deliberately three DIFFERENT censuses — a supertype+type census, a colour census
/// and a subtype census — so the row is about the class of card and not about one filter shape.
const CENSUS_LANDS: [(&str, &str, &[&str]); 3] = [
    (
        "Barad-dûr",
        "Barad-dûr enters tapped unless you control a legendary creature.\n{T}: Add {B}.\n\
         {X}{X}{B}, {T}: Amass Orcs X. Activate only if a creature died this turn.",
        &[],
    ),
    (
        "Taiga Stadium",
        "Taiga Stadium enters tapped unless you control a white, blue, or black permanent.\n\
         {T}: Add {R} or {G}.",
        &[],
    ),
    (
        "Country Roads",
        "This land enters tapped unless you control a Mount or Vehicle.\n{T}: Add {W}.\n\
         {1}{W}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with \
         \"This token saddles Mounts and crews Vehicles as though its power were 2 greater.\" \
         Activate only as a sorcery.",
        &[],
    ),
];

/// Parse a census land's real Oracle text and hand back its single replacement definition.
fn census_land_def(name: &str, oracle: &str, subtypes: &[&str]) -> ReplacementDefinition {
    let subs: Vec<String> = subtypes.iter().map(|s| (*s).to_string()).collect();
    let parsed = engine::parser::parse_oracle_text(oracle, name, &[], &["Land".to_string()], &subs);
    assert_eq!(
        parsed.replacements.len(),
        1,
        "fixture pin: {name} parses to exactly ONE replacement definition; a parser change that \
         splits or merges it re-points every arm of this row"
    );
    let def = parsed.replacements[0].clone();
    // The exact triple `replacement_is_spent_self_entry` matches, asserted on the REAL parse so
    // the row cannot drift into testing a shape the corpus does not carry.
    assert_eq!(
        (
            def.event.clone(),
            def.valid_card.clone(),
            def.destination_zone
        ),
        (
            ReplacementEvent::Moved,
            Some(TargetFilter::SelfRef),
            Some(Zone::Battlefield)
        ),
        "fixture pin: {name} carries the CR 614.1d self-entry triple (Moved / SelfRef / \
         Battlefield)"
    );
    def
}

/// Put ONE census land on P0's battlefield, carrying `def` and NOTHING else — no abilities, no
/// triggers, no statics. That is the attributability control: the only new speaker on the board
/// is block (3)'s replacement walk, so every verdict below is block (3)'s.
///
/// BOTH `base_replacement_definitions` AND `replacement_definitions` are written, or
/// `game/layers.rs`'s per-pass reset drops the definition and every arm silently reads an empty
/// store (shipped fixture precedent: `wba_fodder_multiset::graft_doubler`).
fn graft_census_land(state: &mut GameState, name: &str, def: ReplacementDefinition) -> ObjectId {
    let card_id = CardId(state.next_object_id);
    let host = create_object(state, card_id, P0, name.to_string(), Zone::Battlefield);
    let obj = state
        .objects
        .get_mut(&host)
        .expect("the just-created census land is in `objects`");
    obj.card_types.core_types = vec![CoreType::Land];
    obj.base_replacement_definitions = Arc::new(vec![def.clone()]);
    obj.replacement_definitions = vec![def].into();
    host
}

/// Rewrite the grafted definition's `valid_card` from `SelfRef` to `Typed{Land}` — CR 614.1d's
/// OTHER half, "[Objects] enter [the battlefield] . . .", which watches a population the loop's
/// own arrivals join and is therefore not spent by anything.
///
/// Written through `Arc::make_mut` on `base_replacement_definitions` and mirrored into the live
/// store, because `game/layers.rs` re-seeds the live store from the base store on every pass: a
/// mutation applied to the live vector alone is erased before the firewall ever sees it, and the
/// arm would go green for the wrong reason.
fn make_it_watch_every_land(state: &mut GameState, host: ObjectId) {
    let obj = state
        .objects
        .get_mut(&host)
        .expect("the census land is live");
    let base = Arc::make_mut(&mut obj.base_replacement_definitions);
    assert_eq!(
        base.len(),
        1,
        "reach-guard: exactly one grafted definition to rewrite"
    );
    base[0].valid_card = Some(TargetFilter::Typed(TypedFilter::land()));
    obj.replacement_definitions = base.clone().into();
}

/// Count the battlefield Saprolings `who` controls — the cast-resolved reach-guard oracle.
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

/// Drive one live Sprout Swarm recast on `state` and report `(offered, final_waiting_for_label)`,
/// asserting the cast-resolved reach-guards that hold in BOTH directions first — without them a
/// "no offer" could mean "the harness never drove anything".
fn drive_and_report(state: GameState, why: &str) -> bool {
    let before = count_saprolings(&state, P0);
    let outcome = drive_sprout_cast(state);
    assert_eq!(
        outcome.zone_of(ObjectId(405)),
        Zone::Hand,
        "{why} reach-guard: Buyback must return Sprout Swarm to P0's hand, i.e. the cast really \
         resolved"
    );
    assert_eq!(
        count_saprolings(outcome.state(), P0),
        before + 1,
        "{why} reach-guard: the iteration created exactly one more Saproling"
    );
    match outcome.final_waiting_for() {
        WaitingFor::LoopShortcut { proposer, .. } if *proposer == P0 => true,
        // A POSITIVE pin, not merely `!LoopShortcut`: the drive must land on ordinary priority
        // for P0 — the no-offer state — so a refusing arm cannot be satisfied by a wedge.
        WaitingFor::Priority { player } if *player == P0 => false,
        other => panic!("{why}: unexpected terminal prompt {other:?}"),
    }
}

/// **Row 14 — three REAL entry-census lands, each ALONE on the combo board, stop vetoing the
/// CR 732.2a offer; and each returns to REFUSING the moment its definition stops being
/// self-scoped.**
///
/// This is the end-to-end discharge of the spent-self-entry relief. The three lands are the
/// engine's own measured census carriers, they run three DIFFERENT board censuses, and every one
/// of those censuses is live and un-relievable by disjointness — Taiga Stadium in particular is
/// one of the cards `arrival_can_move_a_nonmember_match` refuses, which is why the relief is
/// guard-free and why no per-condition arm could ever have reached it.
///
/// STRUCTURE, so no arm can pass for the wrong reason:
///  * BASELINE (positive control): the untouched dump OFFERS. Without it, a green arm below
///    cannot be told apart from a harness that offers on everything.
///  * ARM A (the claim): dump + the real land ⇒ OFFERS.
///  * ARM B (the live discriminating mutation): the SAME board with `valid_card` rewritten
///    `SelfRef` → `Typed{Land}` through `base_replacement_definitions` ⇒ REFUSES, pinned
///    positively at `Priority{P0}`. One field is the only variable between A and B, so A's offer
///    is attributable to CR 614.1d's self-entry scope and to nothing else — and B is the
///    reach-guard proving block (3) genuinely SEES this definition.
///
/// REVERT / MUTATION PROBE: delete the `continue` at the head of block (3)'s walk in
/// `analysis::resource::fire_time_conditions_read_growing_class_scoped` ⇒ all three ARM A
/// assertions return to REFUSES ⇒ **FAILS**.
#[test]
fn spent_self_entry_relief_offers_on_three_real_entry_census_lands() {
    assert!(
        drive_and_report(load_realistic_dump(), "baseline"),
        "BASELINE positive control: the untouched combo board OFFERS the CR 732.2a shortcut. If \
         this fails, every arm below is vacuous and the finding is about the harness, not the \
         firewall"
    );

    for (name, oracle, subtypes) in CENSUS_LANDS {
        let def = census_land_def(name, oracle, subtypes);

        // ── ARM A: the real card, alone on the board ──
        let mut with_land = load_realistic_dump();
        graft_census_land(&mut with_land, name, def.clone());
        assert!(
            drive_and_report(with_land, name),
            "ARM A ({name}): CR 614.1d + CR 614.12 + CR 400.7 — this land is already on the \
             battlefield and stays the same object across the window, so its own entry \
             replacement can never apply inside the proposed sequence and observes nothing. \
             CR 732.2b already gives every other player the mechanism for deviating; a \
             pre-emptive veto here is the engine guessing at a declaration the rules assign to a \
             player. Deleting block (3)'s spent-self-entry `continue` restores the veto"
        );

        // ── ARM B: one field changed — the definition now watches EVERY land ──
        let mut watching = load_realistic_dump();
        let host = graft_census_land(&mut watching, name, def);
        make_it_watch_every_land(&mut watching, host);
        assert!(
            !drive_and_report(watching, name),
            "ARM B ({name}): CR 614.1d's other half — '[Objects] enter [the battlefield] . . .' \
             — watches a POPULATION, and a loop that puts permanents onto the battlefield can \
             make it apply again inside the window. The veto is correct here, and this arm is \
             also ARM A's reach-guard: block (3) demonstrably sees this definition, so ARM A's \
             offer is the self-entry scope and not a blind walk"
        );
    }
}
