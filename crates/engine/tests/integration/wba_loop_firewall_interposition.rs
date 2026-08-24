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
use engine::types::ability::{
    AbilityKind, Effect, ReplacementCondition, ReplacementDefinition, TargetFilter, TypedFilter,
};
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
/// OTHER half, "[Objects] enter [the battlefield] . . .". `replacement_is_spent_self_entry`
/// tests `valid_card` for `SelfRef` syntactically, so this rewrite alone lapses that relief.
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
            "ARM B ({name}): with `valid_card` rewritten off `SelfRef` the definition is CR \
             614.1d's other half — '[Objects] enter [the battlefield] . . .' — so the relief \
             fails its `Some(SelfRef)` conjunct, block (3) consults the condition, and that \
             live census keeps the veto. This arm is also ARM A's reach-guard: block (3) \
             demonstrably sees this definition, so ARM A's offer is the self-entry scope and \
             not a blind walk"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────
// CR 732.2a PROPOSAL-ABSENCE acceptance — rows 15 and 19.
//
// **What this half adds to the file's posture.** The relief above is about a replacement
// effect that cannot APPLY inside the window. This one is about an activated ability the
// proposed sequence never ACTIVATES. CR 732.2a defines a shortcut as "a sequence of game
// choices, for all players", and CR 732.2c advances the game "with all game choices contained
// in the shortcut proposal having been taken" — so an activated ability absent from that
// sequence is never activated inside the window and cannot act on the growing class, HOWEVER
// LOUDLY IT WOULD READ THE BOARD IF IT EVER RAN. That is why this relief reaches Abandoned Air
// Temple, whose "+1/+1 counter on each creature you control" read is genuine on the merits and
// which no disjointness argument could ever relieve.
//
// **CONTINGENT relief, stated at the file level so nobody re-reads it as structural.** A loop
// whose proposal DID name one of these abilities restores the veto. The unit rows
// `loop_driving_activation_is_not_relieved` and `loop_driving_mana_activation_is_not_relieved`
// are the intersection tests, and they cannot be driven here: the Sprout Swarm loop's only
// recorded step is a `Recast`, which names a card being cast and never an activation.
// ─────────────────────────────────────────────────────────────────────────────────────────

/// The three census lands row 15 drives, each with its VERBATIM Oracle text from the pinned
/// card-data export. Each carries TWO activated abilities: a mana ability (`{T}: Add ..`, which
/// CR 605.3a keeps OUT of this relief) and a second, non-mana ability whose body reads the
/// board. They are deliberately three DIFFERENT reads — a counter sweep over every creature
/// you control, a token mint with a board-scaled cost reduction, and a targeted keyword grant —
/// so the row is about the class of card and not about one effect shape.
const PROPOSAL_LANDS: [(&str, &str, &[&str]); 3] = [
    (
        "Abandoned Air Temple",
        "This land enters tapped unless you control a basic land.\n{T}: Add {W}.\n\
         {3}{W}, {T}: Put a +1/+1 counter on each creature you control.",
        &[],
    ),
    (
        "The Lonely Mountain",
        "({T}: Add {R}.)\nThis land enters tapped unless you control an Equipment.\n\
         {4}{R}, {T}: Create a 2/2 red Dwarf creature token. This ability costs {1} less to \
         activate for each Equipment you control. Activate only as a sorcery.",
        &["Mountain"],
    ),
    (
        "Fire Nation Palace",
        "This land enters tapped unless you control a basic land.\n{T}: Add {R}.\n\
         {1}{R}, {T}: Target creature you control gains firebending 4 until end of turn. \
         (Whenever it attacks, add {R}{R}{R}{R}. This mana lasts until end of combat.)",
        &[],
    ),
];

/// Chocobo Camp's VERBATIM Oracle text, from the same export.
const CHOCOBO_CAMP: (&str, &str, &[&str]) = (
    "Chocobo Camp",
    "This land enters tapped unless you control a legendary creature.\n\
     {T}: Add {G}. When you next cast a Bird creature spell this turn, it enters with an \
     additional +1/+1 counter on it.\n\
     {2}{G}{G}, {T}: Create a 2/2 green Bird creature token with \"Whenever a land you control \
     enters, this token gets +1/+0 until end of turn.\"",
    &[],
);

/// Put ONE land on P0's battlefield carrying its REAL parsed abilities AND its real entry
/// replacement, and nothing else. Both replacement stores are written for the same reason
/// [`graft_census_land`] writes both: `game/layers.rs` re-seeds the live store from the base
/// store on every pass.
///
/// The abilities are the point of this helper — [`graft_census_land`] deliberately installs a
/// definition and NO abilities, because row 14's attributability control needs block (3) to be
/// the only speaker on the board. Here block (2) is the subject, so the abilities must be real.
fn graft_full_land(state: &mut GameState, card: (&str, &str, &[&str])) -> ObjectId {
    let (name, oracle, subtypes) = card;
    let subs: Vec<String> = subtypes.iter().map(|s| (*s).to_string()).collect();
    let parsed = engine::parser::parse_oracle_text(oracle, name, &[], &["Land".to_string()], &subs);
    assert_eq!(
        parsed.replacements.len(),
        1,
        "fixture pin: {name} parses to exactly ONE replacement definition (the CR 614.1d entry \
         condition block (3) relieves); a parser change that splits or merges it re-points every \
         arm of this row"
    );
    assert_eq!(
        nonmana_ability_index(&parsed.abilities).len(),
        1,
        "fixture pin: {name} parses to exactly ONE NON-mana activated ability — the surface this \
         partition's relief acts on. Pinned by PREDICATE, not by index: an intrinsic basic-land \
         mana ability is added by the DATABASE LOADER and not by the parser, so a card's parsed \
         ability count is not its exported one (MEASURED: The Lonely Mountain exports 2 and \
         parses to 1)"
    );
    assert!(
        parsed.triggers.is_empty() && parsed.statics.is_empty(),
        "fixture pin: {name} carries NO triggers and NO static abilities, so blocks (1), (4) \
         and (5) are silent and every verdict below is block (2)'s or block (3)'s"
    );

    let card_id = CardId(state.next_object_id);
    let host = create_object(state, card_id, P0, name.to_string(), Zone::Battlefield);
    let obj = state
        .objects
        .get_mut(&host)
        .expect("the just-created land is in `objects`");
    obj.card_types.core_types = vec![CoreType::Land];
    obj.abilities = Arc::new(parsed.abilities.clone());
    obj.base_replacement_definitions = Arc::new(parsed.replacements.clone());
    obj.replacement_definitions = parsed.replacements.into();
    host
}

/// The indices of the parsed abilities that are NOT CR 605.1a mana abilities — i.e. the ones
/// this partition's relief can act on at all, since CR 605.3a holds mana abilities out of it.
///
/// A PREDICATE rather than a positional pin, because a land's parsed ability list is not its
/// exported one: intrinsic basic-land-type mana abilities are attached by the database loader,
/// so The Lonely Mountain exports two abilities and parses to one, while Chocobo Camp exports
/// and parses two.
fn nonmana_ability_index(abilities: &[engine::types::ability::AbilityDefinition]) -> Vec<usize> {
    abilities
        .iter()
        .enumerate()
        .filter(|(_, a)| !engine::game::mana_abilities::is_mana_ability(a))
        .map(|(i, _)| i)
        .collect()
}

/// Rewrite the grafted land's sole NON-mana ability from `Activated` to `Spell` kind —
/// CR 117.1b's other side. A `Spell`-kind def is not reached through activation at all, so "the
/// proposal never activated it" says nothing about it and the relief must refuse.
///
/// This is row 15's live discriminating mutation AND its reach-guard: it changes ONE enum field
/// on ONE ability, so an offer that survives every other arm but dies here is attributable to
/// the proposal-absence relief and to nothing else on the board.
fn spellify_the_nonmana_ability(state: &mut GameState, host: ObjectId) {
    let obj = state.objects.get_mut(&host).expect("the land is live");
    let abilities = Arc::make_mut(&mut obj.abilities);
    let targets = nonmana_ability_index(abilities);
    assert_eq!(
        targets.len(),
        1,
        "reach-guard: exactly one non-mana ability to rewrite"
    );
    assert_eq!(
        abilities[targets[0]].kind,
        AbilityKind::Activated,
        "reach-guard: the non-mana ability really is the ACTIVATED one this relief acts on"
    );
    abilities[targets[0]].kind = AbilityKind::Spell;
}

/// Flip `uses_tracked_set` on the CR 603.7 delayed triggered ability the grafted land's MANA
/// ability creates. `true` resolves that payload against the parent ability's tracked object
/// set, a referent the definition cannot see, so the firewall must fail closed and refuse.
///
/// One bool on one node is the only variable it changes, so an offer that survives it would
/// mean block (2) never read this node at all.
fn track_the_delayed_payload(state: &mut GameState, host: ObjectId) {
    let obj = state.objects.get_mut(&host).expect("the land is live");
    let abilities = Arc::make_mut(&mut obj.abilities);
    assert!(
        !nonmana_ability_index(abilities).contains(&0),
        "reach-guard: `abilities[0]` really is the CR 605.1a mana ability that carries the \
         delayed trigger"
    );
    let sub = abilities[0]
        .sub_ability
        .as_mut()
        .expect("reach-guard: the mana ability carries the delayed-trigger sub-ability");
    let Effect::CreateDelayedTrigger {
        uses_tracked_set, ..
    } = sub.effect.as_mut()
    else {
        panic!(
            "reach-guard: that sub-ability's effect is the `Effect::CreateDelayedTrigger` \
             this row is about"
        );
    };
    *uses_tracked_set = true;
}

/// **Row 15 — three REAL census lands whose activated ability the proposed sequence never
/// activates stop vetoing the CR 732.2a offer; and each returns to REFUSING the moment that
/// ability stops being an activated one.**
///
/// This is the end-to-end discharge of the proposal-absence relief. Abandoned Air Temple is the
/// one of these whose read is genuine on the merits — "put a +1/+1 counter on each creature you
/// control" really does census the growing Saproling class — which is exactly why no
/// disjointness arm reaches it and why this relief has to be inapplicability-shaped.
///
/// STRUCTURE, so no arm can pass for the wrong reason:
///  * BASELINE (positive control): the untouched dump OFFERS. Without it, a green arm below
///    cannot be told apart from a harness that offers on everything.
///  * ARM A (the claim): dump + the real land ⇒ OFFERS.
///  * ARM B (the live discriminating mutation): the SAME board with the second ability's `kind`
///    rewritten `Activated` → `Spell` ⇒ REFUSES, pinned positively at `Priority{P0}`. One enum
///    field is the only variable between A and B, so A's offer is attributable to CR 732.2a's
///    proposal-absence argument — and B is the reach-guard proving block (2) genuinely SEES
///    this ability.
///
/// REVERT / MUTATION PROBE: delete the `&& !not_proposed` conjunct at block (2) in
/// `analysis::resource::fire_time_conditions_read_growing_class_scoped` ⇒ all three ARM A
/// assertions return to REFUSES ⇒ **FAILS**.
#[test]
fn unactivated_ability_relief_offers_on_three_real_census_lands() {
    assert!(
        drive_and_report(load_realistic_dump(), "baseline"),
        "BASELINE positive control: the untouched combo board OFFERS the CR 732.2a shortcut. \
         If this fails, every arm below is vacuous and the finding is about the harness, not \
         the firewall"
    );

    for card in PROPOSAL_LANDS {
        let name = card.0;

        // ── ARM A: the real card, alone on the board ──
        let mut with_land = load_realistic_dump();
        graft_full_land(&mut with_land, card);
        assert!(
            drive_and_report(with_land, name),
            "ARM A ({name}): CR 732.2a + CR 732.2c — the proposed sequence contains no \
             activation of this land's ability, so it is never activated inside the window and \
             cannot act on the growing class, whatever it would read if it ran. CR 732.2b \
             already gives every other player the mechanism for deviating; a pre-emptive veto \
             here is the engine guessing at a declaration the rules assign to a player. \
             Deleting block (2)'s `&& !not_proposed` conjunct restores the veto"
        );

        // ── ARM B: one enum field changed — the ability is no longer an activated one ──
        let mut spellified = load_realistic_dump();
        let host = graft_full_land(&mut spellified, card);
        spellify_the_nonmana_ability(&mut spellified, host);
        assert!(
            !drive_and_report(spellified, name),
            "ARM B ({name}): CR 117.1b scopes the activation rule — and with it CR 732.2a's \
             'sequence of game choices' — to ACTIVATED abilities. A `Spell`-kind def is not \
             reached through activation at all, so the proposal's silence about it proves \
             nothing and the veto is correct. This arm is also ARM A's reach-guard: block (2) \
             demonstrably sees this ability, so ARM A's offer is the relief and not a blind scan"
        );
    }
}

/// **Chocobo Camp OFFERS the CR 732.2a shortcut, untapped and tapped.**
///
/// The board is the realistic combo dump with Chocobo Camp grafted onto it as the only
/// grafted proposal land — `graft_full_land` ADDS an object and clears nothing, so the loop
/// the shortcut is proposed for is the dump's own.
///
/// Block (2) is an `any` over `obj.abilities`, so both surfaces have to clear:
///  * `abilities[0]` (`{T}: Add {G}. When you next cast a Bird creature spell this turn, …`)
///    is a CR 605.1a mana ability that CR 605.3a holds out of the proposal-absence relief, so
///    its veto can only be lifted by classifying the delayed trigger's own payload.
///  * `abilities[1]` (the token ability) is relieved by the proposal-absence argument.
///
/// STRUCTURE, so no arm can pass for the wrong reason:
///  * BASELINE (positive control): the untouched dump OFFERS, so an OFFER below is the
///    card's and not the harness's.
///  * PAIRED POSITIVE: a land that already offers on the same board still offers, so the
///    question below is about this card rather than about the board.
///  * REACH-GUARDS: two activated abilities, exactly one a mana ability; and ARM B flips
///    `uses_tracked_set` on `abilities[0]`'s delayed payload ⇒ REFUSES, so block (2) reads it.
///
/// REVERT / MUTATION PROBE: restore `Effect::CreateDelayedTrigger { .. } => Axes::CONSERVATIVE`
/// in `game::ability_scan`'s `scan_effect` ⇒ the OFFER assertion below **FAILS** while the
/// BASELINE control above still passes, so that red is this card's and not the harness's.
#[test]
fn chocobo_camp_offers_untapped_and_tapped() {
    assert!(
        drive_and_report(load_realistic_dump(), "bare dump"),
        "BASELINE positive control: the untouched combo board OFFERS, so an OFFER below is \
         the card's and not the harness's"
    );
    assert!(
        {
            let mut with_temple = load_realistic_dump();
            graft_full_land(&mut with_temple, PROPOSAL_LANDS[0]);
            drive_and_report(with_temple, "air temple control")
        },
        "PAIRED POSITIVE: Abandoned Air Temple offers on the same board, so the verdict below \
         is about this card and not about the board"
    );

    for tapped in [false, true] {
        let mut board = load_realistic_dump();
        let host = graft_full_land(&mut board, CHOCOBO_CAMP);
        {
            let obj = board.objects.get_mut(&host).expect("Chocobo Camp is live");
            obj.tapped = tapped;
            assert_eq!(
                obj.abilities.len(),
                2,
                "reach-guard: Chocobo Camp parses to TWO activated abilities, and block (2) \
                 is an `any` over them — so a green verdict means both cleared"
            );
            assert_eq!(
                nonmana_ability_index(&obj.abilities),
                vec![1],
                "reach-guard: exactly ONE of the two is a CR 605.1a mana ability — \
                 `abilities[0]`, which CR 605.3a holds OUT of the proposal-absence relief — \
                 while `abilities[1]` IS reached by it, so both surfaces are reached"
            );
        }
        assert!(
            drive_and_report(board, "chocobo camp"),
            "(tapped = {tapped}): CR 732.2a — with the delayed trigger's payload classified \
             instead of vetoed on its shape, the mana ability's surface reads nothing that \
             the loop's own growth can move, so the shortcut offer is legal on this board"
        );
    }

    // ── ARM B: the SAME board, one bool changed on the node this row is about ──
    let mut tracked = load_realistic_dump();
    let host = graft_full_land(&mut tracked, CHOCOBO_CAMP);
    track_the_delayed_payload(&mut tracked, host);
    assert!(
        !drive_and_report(tracked, "tracked-set chocobo camp"),
        "ARM B: with `uses_tracked_set` set on `abilities[0]`'s delayed payload the firewall \
         fails CLOSED — CR 603.7's delayed ability would resolve against a tracked set this \
         definition cannot see — so this arm is the reach-guard for the arms above: block \
         (2) demonstrably reads that node, and their offers are its classification and not \
         a blind scan"
    );
}

// ─────────────────────────────────────────────────────────────────────────────────────────
// CR 732.2a SUBTYPE-CENSUS acceptance.
//
// The arms above all run on `UnlessControlsMatching` lands. This half runs the corpus shape
// whose scan arm now reports the census its evaluator runs — `UnlessControlsSubtype` — beside
// the cluster sibling whose arm is untouched, so a verdict here is attributable to that arm
// and not to the grafting harness.
// ─────────────────────────────────────────────────────────────────────────────────────────

/// The two `UnlessControlsSubtype` check lands, VERBATIM Oracle text from the pinned export.
/// Both are `core_types: [Land]` with no printed subtypes and one parsed replacement each.
const CHECK_LANDS: [(&str, &str, &[&str]); 2] = [
    (
        "Dragonskull Summit",
        "This land enters tapped unless you control a Swamp or a Mountain.\n{T}: Add {B} or {R}.",
        &[],
    ),
    (
        "Hinterland Harbor",
        "This land enters tapped unless you control a Forest or an Island.\n{T}: Add {G} or {U}.",
        &[],
    ),
];

/// The untouched cluster sibling: `UnlessControlsOtherLeq`, whose scan arm is
/// `Axes::CONSERVATIVE` before and after this change. Same Oracle-text discipline.
const OTHER_LEQ_CONTROL: (&str, &str, &[&str]) = (
    "Blackcleave Cliffs",
    "This land enters tapped unless you control two or fewer other lands.\n{T}: Add {B} or {R}.",
    &[],
);

/// **Two REAL subtype-census lands, each ALONE on the combo board, still offer the CR 732.2a
/// shortcut once their condition reports the census it runs; and each REFUSES the moment its
/// definition stops being self-scoped.**
///
/// CR 614.1d + CR 614.12 + CR 400.7: on ARM A the land is already on the battlefield and stays
/// the same object, so its own entry replacement cannot apply inside the window and the
/// def-scoped relief carries the offer whatever the condition says. ARM B rewrites `valid_card`
/// away from `SelfRef`, failing the relief's `Some(SelfRef)` conjunct — a syntactic test, not a
/// population one — so the condition is consulted and no arm relieves this subtype census.
///
/// REVERT / MUTATION PROBE: restore `=> Axes::NONE` on `scan_replacement_condition`'s
/// `UnlessControlsSubtype` arm ⇒ both ARM B assertions OFFER ⇒ **FAILS**. ARM A is invariant
/// under every mutation of that arm; its own revert is deleting block (3)'s spent-self-entry
/// `continue` in `analysis::resource::fire_time_conditions_read_growing_class_scoped`.
#[test]
fn check_lands_still_offer_with_the_subtype_arm_repaired() {
    assert!(
        drive_and_report(load_realistic_dump(), "baseline"),
        "BASELINE positive control: the untouched combo board OFFERS the CR 732.2a shortcut. \
         Without it, every arm below is vacuous and a green row is about the harness"
    );

    // ── CONTROL, run FIRST so both of its readings survive a red arm below: the untouched
    // cluster sibling through the SAME two shapes. Block (3) carries a disjointness relief
    // for `UnlessControlsOtherLeq` and none for `UnlessControlsSubtype`, so this pair offers
    // through both shapes while the pair below separates at ARM B — the difference is the
    // condition, not the `valid_card` rewrite the two shapes share.
    let (control_name, control_oracle, control_subtypes) = OTHER_LEQ_CONTROL;
    let control_def = census_land_def(control_name, control_oracle, control_subtypes);
    let mut control_a = load_realistic_dump();
    graft_census_land(&mut control_a, control_name, control_def.clone());
    assert!(
        drive_and_report(control_a, control_name),
        "CONTROL ARM A ({control_name}): the sibling condition takes the same def-scoped \
         relief the subtype lands take below"
    );
    let mut control_b = load_realistic_dump();
    let control_host = graft_census_land(&mut control_b, control_name, control_def);
    make_it_watch_every_land(&mut control_b, control_host);
    assert!(
        drive_and_report(control_b, control_name),
        "CONTROL ARM B ({control_name}): an 'other lands you control' census provably cannot \
         count a growing class of creature tokens, so block (3)'s disjointness relief clears \
         it and the `valid_card` rewrite ALONE does not refuse an offer"
    );

    for (name, oracle, subtypes) in CHECK_LANDS {
        let def = census_land_def(name, oracle, subtypes);
        assert!(
            matches!(
                def.condition,
                Some(ReplacementCondition::UnlessControlsSubtype { .. })
            ),
            "fixture pin: {name} must parse to the `UnlessControlsSubtype` arm this row pins — \
             `UnlessControlsMatching` reports the same sibling axis and gets no \
             `condition_disjoint` relief, so a re-route there would leave the arms below green \
             while that arm goes untested"
        );

        // ── ARM A: the real card, alone on the board ──
        let mut with_land = load_realistic_dump();
        graft_census_land(&mut with_land, name, def.clone());
        assert!(
            drive_and_report(with_land, name),
            "ARM A ({name}): CR 614.1d + CR 614.12 + CR 400.7 — the land is already on the \
             battlefield and stays the same object across the window, so its own entry \
             replacement can never apply inside the proposed sequence. The def-scoped relief \
             fires ahead of the condition surface, so repairing the subtype arm does not cost \
             this offer"
        );

        // ── ARM B: one field changed — the definition now watches EVERY land ──
        let mut watching = load_realistic_dump();
        let host = graft_census_land(&mut watching, name, def);
        make_it_watch_every_land(&mut watching, host);
        assert!(
            !drive_and_report(watching, name),
            "ARM B ({name}): with `valid_card` rewritten off `SelfRef` the relief fails its \
             `Some(SelfRef)` conjunct and block (3) reaches the condition. The evaluator \
             censuses the live battlefield for a controlled permanent of a listed subtype, and \
             no disjointness arm can prove that census invariant, so CR 732.2a's predictability \
             requirement is unmet and the offer is refused"
        );
    }
}
