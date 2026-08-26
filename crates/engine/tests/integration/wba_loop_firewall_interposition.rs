//! CR 732.2a INTERPOSITION acceptance — the loop-shortcut firewall must not veto an offer on
//! account of a replacement effect that is SPENT for the proposed window. CR 732.2b already gives
//! every other player the deviation mechanism: each may accept the proposed sequence or shorten
//! it by naming a place where they will choose differently. Vetoing pre-emptively on a permanent
//! that merely observes guesses at a declaration the rules assign to a player; a veto belongs
//! only where something on the board would falsify the proposed ending state.
//!
//! What makes the relief sound: an "enters tapped unless you control …" land's replacement has
//! its OWN entrance as its only subject — CR 614.1d templates "[This permanent] enters . . ."
//! separately from "[Objects] enter . . .", and CR 614.12 makes the first apply only to that
//! permanent. Once the land is on the battlefield and stays the same object across the window (CR
//! 400.7) the event it watches cannot recur, so none of its surfaces runs, however loudly its
//! condition would census the board. That is INAPPLICABILITY, not disjointness, which is why the
//! relief reaches lands whose census genuinely counts the growing class.

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
///
/// `expected` is the condition arm the CALLER's row reads, and it is required: every row this
/// helper feeds separates on which arm its fixture reaches, so no call site can reach one
/// without declaring which arm it depends on. Only the discriminant is compared — `expected`'s
/// payload is a placeholder.
fn census_land_def(
    name: &str,
    oracle: &str,
    subtypes: &[&str],
    expected: ReplacementCondition,
) -> ReplacementDefinition {
    let subs: Vec<String> = subtypes.iter().map(|s| (*s).to_string()).collect();
    let parsed = engine::parser::parse_oracle_text(oracle, name, &[], &["Land".to_string()], &subs);
    assert_eq!(
        parsed.replacements.len(),
        1,
        "fixture pin: {name} parses to exactly ONE replacement definition; a parser change that \
         splits or merges it re-points every arm of every row this helper feeds"
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
    // Debug's leading identifier is the variant, and the variant is the whole of what this
    // assert compares; `expected`'s payload is a placeholder and must never print as a fixture
    // value.
    let expected_arm: String = format!("{expected:?}")
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    assert_eq!(
        def.condition.as_ref().map(std::mem::discriminant),
        Some(std::mem::discriminant(&expected)),
        "fixture pin: {name} must parse to {expected_arm}, the arm its row reads, not {:?}",
        def.condition
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

/// **Three REAL entry-census lands, each ALONE on the combo board, stop vetoing the CR 732.2a
/// offer, and each REFUSES again the moment its definition stops being self-scoped.** They run
/// three DIFFERENT live censuses no disjointness argument relieves (Taiga Stadium is one
/// `arrival_can_move_a_nonmember_match` refuses), so no per-condition arm reaches them.
///
/// BASELINE (positive control): the untouched dump OFFERS, so a green arm below is not a harness
/// that offers on everything. ARM A: dump + the real land ⇒ OFFERS. ARM B, the live
/// discriminating mutation: `valid_card` rewritten `SelfRef` → `Typed{Land}` ⇒ REFUSES, pinned
/// positively at `Priority{P0}`. One field is the only variable, so A's offer is attributable to
/// CR 614.1d's self-entry scope, and B proves block (3) SEES it.
///
/// REVERT / MUTATION PROBE: delete the `continue` at the head of block (3)'s walk in
/// `analysis::resource::fire_time_conditions_read_growing_class_scoped` ⇒ all three ARM A
/// assertions REFUSE ⇒ **FAILS**.
#[test]
fn spent_self_entry_relief_offers_on_three_real_entry_census_lands() {
    assert!(
        drive_and_report(load_realistic_dump(), "baseline"),
        "BASELINE positive control: the untouched combo board OFFERS the CR 732.2a shortcut. If \
         this fails, every arm below is vacuous and the finding is about the harness, not the \
         firewall"
    );

    for (name, oracle, subtypes) in CENSUS_LANDS {
        let def = census_land_def(
            name,
            oracle,
            subtypes,
            ReplacementCondition::UnlessControlsMatching {
                filter: TargetFilter::None,
            },
        );

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
// CR 732.2a PROPOSAL-ABSENCE acceptance. The relief above is about a replacement effect that
// cannot APPLY inside the window; this one is about an activated ability the proposed sequence
// never ACTIVATES. CR 732.2a defines a shortcut as "a sequence of game choices, for all players",
// and CR 732.2c advances the game "with all game choices contained in the shortcut proposal
// having been taken" — so an ability absent from that sequence is never activated inside the
// window and cannot act on the growing class, HOWEVER LOUDLY IT WOULD READ THE BOARD IF IT EVER
// RAN. That is why it reaches Abandoned Air Temple, whose "+1/+1 counter on each creature you
// control" read is genuine and which no disjointness argument could relieve.
//
// CONTINGENT, not structural: a loop whose proposal DID name one of these abilities restores the
// veto. `loop_driving_activation_is_not_relieved` and `loop_driving_mana_activation_is_not_relieved`
// are the intersection tests; neither is drivable here — this loop's only step is a `Recast`.
// ─────────────────────────────────────────────────────────────────────────────────────────

/// The three census lands [`unactivated_ability_relief_offers_on_three_real_census_lands`] drives,
/// each with its VERBATIM Oracle text from the pinned card-data export. Each carries TWO
/// activated abilities: a mana ability (`{T}: Add ..`, which
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
/// definition and NO abilities, so that its attributability control leaves block (3) as the only
/// speaker on the board. Here block (2) is the subject, so the abilities must be real.
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
/// This is the proposal-absence row's live discriminating mutation AND its reach-guard: it changes
/// ONE enum field on ONE ability, so an offer that survives every other arm but dies here is
/// attributable to the proposal-absence relief and to nothing else on the board.
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

/// **Three REAL census lands whose activated ability the proposed sequence never activates stop
/// vetoing the CR 732.2a offer, and each REFUSES again the moment that ability stops being
/// activated.** Abandoned Air Temple's "+1/+1 counter on each creature you control" really does
/// census the growing Saproling class, so no disjointness arm reaches it and the relief has to be
/// inapplicability-shaped.
///
/// BASELINE (positive control): the untouched dump OFFERS. ARM A: dump + the real land ⇒ OFFERS.
/// ARM B, the live discriminating mutation: the second ability's `kind` rewritten `Activated` →
/// `Spell` ⇒ REFUSES, pinned positively at `Priority{P0}`. That one enum field is the only
/// variable, so A's offer is attributable to CR 732.2a's proposal-absence argument and B proves
/// block (2) SEES the ability.
///
/// REVERT / MUTATION PROBE: delete block (2)'s `&& !not_proposed` conjunct in
/// `analysis::resource::fire_time_conditions_read_growing_class_scoped` ⇒ ARM A REFUSES ⇒ **FAILS**.
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

/// **Chocobo Camp OFFERS the CR 732.2a shortcut, untapped and tapped.** `graft_full_land` ADDS an
/// object and clears nothing, so the loop the shortcut is proposed for is the dump's own. Block
/// (2) is an `any` over `obj.abilities`, so both surfaces have to clear:
///  * `abilities[0]` (`{T}: Add {G}. When you next cast a Bird creature spell this turn, …`) is a
///    CR 605.1a mana ability that CR 605.3a holds out of the proposal-absence relief, so its veto
///    can only be lifted by classifying the delayed trigger's own payload.
///  * `abilities[1]` (the token ability) is relieved by the proposal-absence argument.
///
/// BASELINE (positive control): the untouched dump OFFERS. PAIRED POSITIVE: a land that already
/// offers on the same board still offers, so the question below is about this card and not the
/// board. REACH-GUARDS: two activated abilities, exactly one a mana ability; and ARM B flips
/// `uses_tracked_set` on `abilities[0]`'s delayed payload ⇒ REFUSES, so block (2) reads it.
/// REVERT / MUTATION PROBE: restore `Effect::CreateDelayedTrigger { .. } => Axes::CONSERVATIVE` in
/// `game::ability_scan`'s `scan_effect` ⇒ the OFFER below **FAILS** while BASELINE still passes.
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
    let control_def = census_land_def(
        control_name,
        control_oracle,
        control_subtypes,
        ReplacementCondition::UnlessControlsOtherLeq {
            count: 0,
            filter: TypedFilter::default(),
        },
    );
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
        let def = census_land_def(
            name,
            oracle,
            subtypes,
            ReplacementCondition::UnlessControlsSubtype {
                subtypes: Vec::new(),
            },
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

// ─────────────────────────────────────────────────────────────────────────────────────────
// CR 701.17 INSTRUCTED-MILL acceptance, and the shapes that must still veto.
//
// The arms above are about a loop whose period touches only the battlefield. This half adds
// the mill: `mill_base` is the same combo board plus a real parsed Altar of the Brood, so each
// period also declines every opponent's library. `certify_instructed_opponent_library_departure`
// establishes that decline by exclusion and the cover's four growth obligations consume it;
// C2 is the public-information veto that keeps the certificate honest, and every row below is
// the base plus ONE object that either reaches C2 or provably does not.
//
// CR 400.2 scopes the relief: the detection drive reads the board through the proposer's own
// hidden view, so an observer the proposer may not see cannot carry a veto, while the same
// observer in the proposer's OWN hand still does. That trio is what attributes the hand
// collection, and the interposer/descend rows at the end are the library one.
// ─────────────────────────────────────────────────────────────────────────────────────────

use engine::types::ability::{PlayerFilter, TriggerDefinition};
use engine::types::triggers::TriggerMode;

use super::witherbloom_altar_probe::graft_altar;

const P1: PlayerId = PlayerId(1);

/// Narcomoeba, VERBATIM Oracle text from the pinned `client/public/card-data.json` export.
/// Its library→graveyard trigger is the interposer row's whole subject.
const NARCOMOEBA: (&str, &str, &[&str]) = (
    "Narcomoeba",
    "Flying\nWhen this card is put into your graveyard from your library, you may put it onto \
     the battlefield.",
    &["Illusion"],
);

/// The shared mill base: this module's own combo board plus a real Altar of the Brood, so the
/// loop's period mills every opponent. ONE object away from `load_realistic_dump()`, which is
/// why a hostile row built on it is one object away from an offering board and its silence has
/// exactly one cause.
fn mill_base() -> GameState {
    let mut state = load_realistic_dump();
    graft_altar(&mut state, P0);
    state
}

/// Graft a battlefield permanent under `seat` carrying `def` and nothing else — the
/// attributability control every trigger-shaped row below rests on.
///
/// `push_printed_trigger` is the single authority that keeps `base_trigger_definitions` and the
/// live list in lockstep; a bare `objects.insert` leaves the definition out of the base store,
/// `game/layers.rs`'s per-pass reset drops the live entry, and the arm reads an empty store.
fn graft_observer(
    state: &mut GameState,
    seat: PlayerId,
    zone: Zone,
    def: TriggerDefinition,
) -> ObjectId {
    let card_id = CardId(state.next_object_id);
    let host = create_object(state, card_id, seat, "Departure Observer".to_string(), zone);
    let obj = state
        .objects
        .get_mut(&host)
        .expect("the just-created observer is in `objects`");
    obj.card_types.core_types = vec![CoreType::Artifact];
    obj.push_printed_trigger(def);
    host
}

/// A second Altar of the Brood on P0's battlefield with `mutate` applied to the mill it
/// executes. The rows that use it flip exactly one field, so the pair separates on that field
/// and not on the extra permanent.
fn graft_mutated_altar(
    state: &mut GameState,
    mutate: impl FnOnce(&mut engine::types::ability::AbilityDefinition),
) -> ObjectId {
    let host = graft_altar(state, P0);
    let obj = state
        .objects
        .get_mut(&host)
        .expect("the second Altar is live");
    let base = Arc::make_mut(&mut obj.base_trigger_definitions);
    assert_eq!(
        base.len(),
        1,
        "reach-guard: exactly one grafted trigger to rewrite"
    );
    let execute = base[0]
        .execute
        .as_mut()
        .expect("reach-guard: Altar's trigger carries the mill it executes");
    mutate(execute);
    let rewritten = base[0].clone();
    obj.trigger_definitions.clear();
    obj.base_trigger_definitions = Arc::new(Vec::new());
    obj.push_printed_trigger(rewritten);
    host
}

/// Every non-caster library card rewritten NON-PERMANENT and surface-free.
///
/// On this board a milled card can neither flip `descended_this_turn` (CR 700.11 counts only a
/// permanent card) nor open a window nor carry a definition C2 could read, and it satisfies the
/// cover's own inertness obligation WITHOUT the proposer view having to blank it. That is what
/// makes each row below one live channel with the other provably absent: the pinned control
/// keeps offering when the redaction is taken away, so a row that reddens there reddens on its
/// own object.
fn pinned_mill_base() -> GameState {
    let mut state = mill_base();
    let pinned: Vec<ObjectId> = state
        .players
        .iter()
        .filter(|p| p.id != P0)
        .flat_map(|p| p.library.iter().copied())
        .collect();
    assert!(
        !pinned.is_empty(),
        "reach-guard: the non-caster libraries are non-empty, so the pin below is a rewrite \
         and not an empty walk"
    );
    for id in pinned {
        let obj = state.objects.get_mut(&id).expect("library card is keyed");
        obj.card_types.core_types = vec![CoreType::Instant];
        obj.card_types.supertypes.clear();
        obj.card_types.subtypes.clear();
        obj.trigger_definitions.clear();
        obj.base_trigger_definitions = Arc::new(Vec::new());
        obj.replacement_definitions.clear();
        obj.base_replacement_definitions = Arc::new(Vec::new());
        obj.static_definitions.clear();
        obj.base_static_definitions = Arc::new(Vec::new());
        obj.abilities = Arc::new(Vec::new());
        obj.base_abilities = Arc::new(Vec::new());
        obj.keywords.clear();
        obj.counters.clear();
    }
    state
}

/// The id at `depth` of `victim`'s library AS IT STANDS AFTER the loop-priming cast.
///
/// MEASURED LAW: that cast mills the top of every victim library before the detection drive
/// runs, so a card the drive must mill is placed by the POST-cast order, never the pre-cast
/// one — at pre-cast depth it is consumed by the priming cast and the row asserts against a
/// board that no longer holds the thing it checks. Driving a throwaway copy of the same cast
/// is what says which id that is; its own decline is this helper's reach-guard.
fn post_cast_library_anchor(base: &GameState, victim: PlayerId, depth: usize) -> ObjectId {
    let before = library_ids(base, victim).len();
    let outcome = drive_sprout_cast(base.clone());
    let after = library_ids(outcome.state(), victim);
    assert!(
        after.len() < before,
        "reach-guard: the priming cast must decline {victim:?}'s library, or 'post-cast depth' \
         names the same card as 'pre-cast depth' and the law this helper encodes is untested"
    );
    *after
        .get(depth)
        .expect("the post-cast library is deeper than the requested depth")
}

fn library_ids(state: &GameState, who: PlayerId) -> Vec<ObjectId> {
    state
        .players
        .iter()
        .find(|p| p.id == who)
        .expect("seat exists")
        .library
        .iter()
        .copied()
        .collect()
}

/// Move `card` to sit immediately before `anchor` in `victim`'s library.
fn place_before(state: &mut GameState, victim: PlayerId, anchor: ObjectId, card: ObjectId) {
    let player = state
        .players
        .iter_mut()
        .find(|p| p.id == victim)
        .expect("seat exists");
    let from = player
        .library
        .iter()
        .position(|id| *id == card)
        .expect("the card was created into this library");
    player.library.remove(from);
    let at = player
        .library
        .iter()
        .position(|id| *id == anchor)
        .expect("the anchor is still in this library");
    player.library.insert(at, card);
}

/// Put a real parsed Narcomoeba into `victim`'s library immediately before `anchor`.
fn graft_narcomoeba(state: &mut GameState, victim: PlayerId, anchor: ObjectId) -> ObjectId {
    let (name, oracle, subtypes) = NARCOMOEBA;
    let subs: Vec<String> = subtypes.iter().map(|s| (*s).to_string()).collect();
    let parsed =
        engine::parser::parse_oracle_text(oracle, name, &[], &["Creature".to_string()], &subs);
    assert_eq!(
        parsed.triggers.len(),
        1,
        "fixture pin: Narcomoeba parses to exactly ONE trigger — the library→graveyard window \
         this row is about; a parser change that splits or drops it un-points the row"
    );
    let card_id = CardId(state.next_object_id);
    let host = create_object(state, card_id, victim, name.to_string(), Zone::Library);
    {
        let obj = state
            .objects
            .get_mut(&host)
            .expect("the just-created Narcomoeba is in `objects`");
        obj.card_types.core_types = vec![CoreType::Creature];
        obj.push_printed_trigger(parsed.triggers[0].clone());
    }
    place_before(state, victim, anchor, host);
    host
}

/// One row: what it adds to its base, and whether the base's offer survives it.
struct MillRow {
    what: &'static str,
    board: fn() -> GameState,
    expect_offer: bool,
}

/// Drive EVERY row before asserting, so one disagreeing row does not hide the verdicts of the
/// rows after it — the control and its paired hostile arms are only evidence together.
fn run_rows(rows: &[MillRow]) {
    let observed: Vec<(&str, bool, bool)> = rows
        .iter()
        .map(|row| {
            (
                row.what,
                row.expect_offer,
                drive_and_report((row.board)(), row.what),
            )
        })
        .collect();
    let disagreed: Vec<String> = observed
        .iter()
        .filter(|(_, expected, actual)| expected != actual)
        .map(|(what, expected, actual)| {
            format!("  offer expected {expected}, got {actual} — {what}")
        })
        .collect();
    assert!(
        disagreed.is_empty(),
        "{} of {} rows disagreed with their expectation:\n{}",
        disagreed.len(),
        observed.len(),
        disagreed.join("\n")
    );
}

// ── Board suppliers: each is one row's ONE object ──

fn milled_mode_observer() -> GameState {
    let mut state = mill_base();
    graft_observer(
        &mut state,
        P1,
        Zone::Battlefield,
        TriggerDefinition::new(TriggerMode::Milled),
    );
    state
}

fn from_anywhere_graveyard_observer() -> GameState {
    let mut state = mill_base();
    graft_observer(
        &mut state,
        P1,
        Zone::Battlefield,
        TriggerDefinition::new(TriggerMode::ChangesZone).destination(Zone::Graveyard),
    );
    state
}

fn battlefield_origin_with_library_in_origin_zones() -> GameState {
    let mut state = mill_base();
    let mut def = TriggerDefinition::new(TriggerMode::ChangesZone).origin(Zone::Battlefield);
    def.origin_zones = vec![Zone::Library, Zone::Battlefield];
    graft_observer(&mut state, P1, Zone::Battlefield, def);
    state
}

fn battlefield_origin_alone() -> GameState {
    let mut state = mill_base();
    graft_observer(
        &mut state,
        P1,
        Zone::Battlefield,
        TriggerDefinition::new(TriggerMode::ChangesZone).origin(Zone::Battlefield),
    );
    state
}

fn hand_mill_observer(seat: PlayerId, functions_in: Zone) -> GameState {
    let mut state = mill_base();
    graft_observer(
        &mut state,
        seat,
        Zone::Hand,
        TriggerDefinition::new(TriggerMode::Milled).trigger_zones(vec![functions_in]),
    );
    state
}
fn mill_observer_in_an_opponents_hand() -> GameState {
    hand_mill_observer(P1, Zone::Hand)
}
fn mill_observer_in_the_proposers_own_hand() -> GameState {
    hand_mill_observer(P0, Zone::Hand)
}
fn battlefield_scoped_mill_observer_in_the_proposers_own_hand() -> GameState {
    hand_mill_observer(P0, Zone::Battlefield)
}

fn graveyard_diverting_replacement() -> GameState {
    let mut state = mill_base();
    let def = ReplacementDefinition::new(ReplacementEvent::Moved).destination_zone(Zone::Graveyard);
    graft_census_land(&mut state, "Graveyard Diverter", def);
    state
}

fn real_tapland_self_entry() -> GameState {
    let mut state = mill_base();
    let (name, oracle, subtypes) = CENSUS_LANDS[0];
    let def = census_land_def(
        name,
        oracle,
        subtypes,
        ReplacementCondition::UnlessControlsMatching {
            filter: TargetFilter::None,
        },
    );
    graft_census_land(&mut state, name, def);
    state
}

fn second_altar_opponent_facing() -> GameState {
    let mut state = mill_base();
    graft_mutated_altar(&mut state, |_| {});
    state
}
fn second_altar_each_player() -> GameState {
    let mut state = mill_base();
    graft_mutated_altar(&mut state, |execute| {
        execute.player_scope = Some(PlayerFilter::All);
    });
    state
}
fn second_altar_may_mill() -> GameState {
    let mut state = mill_base();
    graft_mutated_altar(&mut state, |execute| {
        execute.optional = true;
    });
    state
}

fn pinned_with_narcomoeba() -> GameState {
    let mut state = pinned_mill_base();
    let anchor = post_cast_library_anchor(&state, P1, 1);
    graft_narcomoeba(&mut state, P1, anchor);
    state
}

fn pinned_with_vanilla_permanent_at_depth_one() -> GameState {
    let mut state = pinned_mill_base();
    let anchor = post_cast_library_anchor(&state, P1, 1);
    let obj = state
        .objects
        .get_mut(&anchor)
        .expect("the anchored library card is keyed");
    assert!(
        obj.trigger_definitions.is_empty() && obj.base_trigger_definitions.is_empty(),
        "reach-guard: the pin left this card definition-free, so the ONLY variable this row \
         adds is its permanent type"
    );
    obj.card_types.core_types = vec![CoreType::Creature];
    state
}

/// Every seat's library size after one driven cycle — the reading that separates the two
/// silent rows below from the offering control they are each one field away from.
fn library_sizes_after_one_cycle(state: GameState) -> Vec<(PlayerId, usize)> {
    drive_sprout_cast(state)
        .state()
        .players
        .iter()
        .map(|p| (p.id, p.library.len()))
        .collect()
}

/// **The mill base OFFERS.** The positive control every row below rests on: one real Altar of
/// the Brood away from the module's own combo board, the loop's period now declines every
/// opponent's library, and `certify_instructed_opponent_library_departure` establishes that
/// decline so the cover's growth obligations can account it.
///
/// REVERT / MUTATION PROBE: make `certify_instructed_opponent_library_departure` return `None`
/// unconditionally ⇒ this **FAILS** while the module's Altar-free rows above stay green.
#[test]
fn mill_base_offers_the_shortcut() {
    assert!(
        drive_and_report(load_realistic_dump(), "baseline"),
        "BASELINE positive control: the untouched combo board OFFERS, so a failure below is \
         about the Altar and not about the harness"
    );
    assert!(
        drive_and_report(mill_base(), "mill base"),
        "CR 701.17a + CR 701.17b: a period that mills each opponent is still an unbounded \
         object-growth loop — an empty library neither stops the mill nor ends the game — so \
         the certified departure must let the cover account the decline and the offer stand"
    );
}

/// **A functioning observer of the certified departure keeps the veto.** C2 is the certificate's
/// public-information conjunct: the proposal may not promise a library decline that something on
/// the board would act on. Each row is the mill base plus ONE battlefield permanent carrying ONE
/// trigger definition and nothing else, so the verdict is that definition's.
///
/// The last pair is the fail-closed one: `origin: Battlefield` alone PROVES exclusion (a
/// certified id left a LIBRARY), but a non-empty `origin_zones` makes the matcher ignore
/// `origin` entirely and require `from_zone` to be in that set instead — so the same pin stops
/// proving anything and the veto must return. Its paired positive is the same definition with
/// `origin_zones` empty.
///
/// REVERT / MUTATION PROBE: delete the `!def.origin_zones.is_empty()` fail-closed conjunct in
/// `game::triggers::departure_observer_provably_excludes` ⇒ the third row OFFERS ⇒ **FAILS**.
#[test]
fn departure_observing_triggers_keep_the_veto() {
    run_rows(&[
        MillRow {
            what: "PAIRED POSITIVE: the mill base with nothing added OFFERS",
            board: mill_base,
            expect_offer: true,
        },
        MillRow {
            what: "CR 701.17a: a TriggerMode::Milled permanent fires on the certified mill by \
                   definition, so C2 must refuse",
            board: milled_mode_observer,
            expect_offer: false,
        },
        MillRow {
            what: "a 'from anywhere' ChangesZone trigger with destination Graveyard pins no \
                   origin and lands in the zone CR 701.17a puts a milled card in, so neither \
                   half of C2's exclusion holds",
            board: from_anywhere_graveyard_observer,
            expect_offer: false,
        },
        MillRow {
            what: "origin: Battlefield with a non-empty origin_zones including Library — the \
                   matcher ignores `origin`, so the origin pin proves nothing and C2 fails \
                   closed",
            board: battlefield_origin_with_library_in_origin_zones,
            expect_offer: false,
        },
        MillRow {
            what: "PAIRED POSITIVE for the row above: the SAME definition with origin_zones \
                   EMPTY provably cannot match a library departure, so the offer stands",
            board: battlefield_origin_alone,
            expect_offer: true,
        },
    ]);
}

/// **The hidden-zone relief is scoped to what the proposer may not see.** CR 400.2 makes an
/// opponent's hand a hidden zone, so the detection drive reads it through the proposer's own
/// hidden view and a `Milled` observer sitting there carries no veto — the proposer could not
/// have known about it at proposal time (CR 732.2a), and CR 732.2b gives its controller the
/// deviation point where that information re-enters. The SAME definition in the PROPOSER'S own
/// hand is information the proposer does hold, so it still vetoes.
///
/// One field separates each row from the next — the seat that owns the hand, then the zone the
/// definition declares (CR 113.6b) — so the relief is attributed to the hand collection of the
/// proposer-view redaction and to nothing else. The middle row is also the outer two's
/// reach-guard: C2 demonstrably walks hand-functioning definitions, so their offers are the
/// redaction and the zone-of-function gate rather than a scan that never looked.
#[test]
fn a_hand_mill_observer_vetoes_only_in_the_proposers_own_hand() {
    run_rows(&[
        MillRow {
            what: "a hand-functioning Milled observer in an OPPONENT'S hand is outside the \
                   proposer's information and must not veto",
            board: mill_observer_in_an_opponents_hand,
            expect_offer: true,
        },
        MillRow {
            what: "the SAME definition in the PROPOSER'S OWN hand is information the proposer \
                   holds, so C2 must refuse",
            board: mill_observer_in_the_proposers_own_hand,
            expect_offer: false,
        },
        MillRow {
            what: "PAIRED POSITIVE for the row above: the same card in the same hand carrying \
                   the same Milled definition DECLARED to function on the battlefield instead \
                   (CR 113.6b) still OFFERS — so what refuses above is the definition \
                   functioning where the proposer can see it, not an extra card in P0's hand",
            board: battlefield_scoped_mill_observer_in_the_proposers_own_hand,
            expect_offer: true,
        },
    ]);
}

/// **A replacement that could divert the certified departure keeps the veto, and an ordinary
/// tapland does not.** CR 614.6 lets a replacement send a card that would go to a graveyard
/// somewhere else, so a `Moved` definition pinned to Graveyard — or pinned nowhere — observes
/// the very move the certificate promises. The `destination_zone` narrowing is what keeps the
/// measured corpus signature `(Moved, SelfRef, Battlefield)` from vetoing every real board, and
/// the paired row is a REAL card carrying exactly that triple.
#[test]
fn a_graveyard_diverting_replacement_vetoes_and_a_real_tapland_does_not() {
    run_rows(&[
        MillRow {
            what: "CR 614.6: a Moved replacement pinned to the graveyard could divert the \
                   certified departure, so the certificate must refuse",
            board: graveyard_diverting_replacement,
            expect_offer: false,
        },
        MillRow {
            what: "PAIRED POSITIVE: a real entry-census tapland's own CR 614.1d self-entry \
                   replacement is pinned to Battlefield and observes no departure",
            board: real_tapland_self_entry,
            expect_offer: true,
        },
    ]);
}

/// **A caster-side mill is not certified, and neither is a mill the proposer may decline.**
/// Both rows are a SECOND Altar of the Brood with exactly one field of its executed mill
/// rewritten, so each separates from the unmutated second Altar on that field alone.
///
/// `player_scope: All` makes the caster's own library decline too. That movement is not a
/// candidate — the certificate ranges over NON-caster libraries — so C1b's residual finds it
/// unaccounted and refuses. `optional: true` is CR 603.5's "you may", a choice made as the
/// ability resolves and therefore a choice the proposed sequence would have to contain. The
/// library reading in the row below is what tells the two apart, and it is what makes the
/// second one isolated: its period declines exactly the libraries the offering control's does.
#[test]
fn caster_side_and_declinable_mills_are_not_certified() {
    run_rows(&[
        MillRow {
            what: "PAIRED POSITIVE: a second, unmutated opponent-facing Altar still OFFERS, so \
                   the two rows below separate on the rewritten field and not on the permanent",
            board: second_altar_opponent_facing,
            expect_offer: true,
        },
        MillRow {
            what: "an 'each player mills' period leaves the CASTER's own library decline \
                   unaccounted, and C1b's residual refuses it",
            board: second_altar_each_player,
            expect_offer: false,
        },
        MillRow {
            what: "CR 603.5: a 'may' mill is a resolution-time choice, so the proposed \
                   sequence would have to contain an answer the proposer never gave",
            board: second_altar_may_mill,
            expect_offer: false,
        },
    ]);
}

/// The mechanism behind the two silent rows above, read off the boards rather than asserted by
/// name — and the may-mill row's isolation is total: it produces the SAME library outcome as
/// the offering control, so nothing about what the period does to any library can explain why
/// one offers and the other does not.
#[test]
fn the_each_player_mill_declines_the_casters_library_and_the_may_mill_declines_nothing_extra() {
    let offering = library_sizes_after_one_cycle(second_altar_opponent_facing());
    assert_ne!(
        offering,
        library_sizes_after_one_cycle(mill_base()),
        "REACH-GUARD: the second Altar really mills — its board's libraries differ from the \
         one-Altar base, so the two rows below are not comparing a permanent that does nothing"
    );
    assert_eq!(
        library_sizes_after_one_cycle(second_altar_may_mill()),
        offering,
        "the may-mill row's period declines exactly the libraries the OFFERING control's does, \
         so its refusal is the declinability itself (CR 603.5) and not anything the mill did"
    );

    let each_player = library_sizes_after_one_cycle(second_altar_each_player());
    let caster_of = |sizes: &[(PlayerId, usize)]| {
        sizes
            .iter()
            .find(|(id, _)| *id == P0)
            .expect("the caster has a seat")
            .1
    };
    assert!(
        caster_of(&each_player) < caster_of(&offering),
        "the each-player row's refusal IS a caster-side decline: only there does P0's own \
         library shrink, and that movement is what C1b's residual finds unaccounted"
    );
}

/// **The two channels a milled card can reach the drive through, one live per row, the other
/// pinned dead.** Both rows OFFER, and each agrees with the bare pinned control it is measured
/// against, so what they assert is that the proposer-view redaction has taken that channel out
/// of the detection drive before it can act.
///
/// On the pinned base every non-caster library card is non-permanent and definition-free, so a
/// milled card can neither flip `descended_this_turn` (CR 700.11) nor open a window. The
/// interposer row adds ONE real parsed Narcomoeba at post-cast depth 1 of a victim's library —
/// its own library→graveyard window is the live channel. The descend row instead rewrites the
/// card already at that depth to a vanilla PERMANENT carrying no definitions — no window
/// anywhere, so the live channel is the arriving card's permanent type alone.
///
/// REVERT / MUTATION PROBE, RUN rather than reasoned: make
/// `game::visibility::proposer_hidden_view` return its clone unprojected, so the drive reads
/// unredacted library cards ⇒ the pinned control still OFFERS and BOTH rows go silent ⇒ both
/// **FAIL**. The control holding is what makes each failure the row's own object; which
/// conjunct refuses it is not this row's claim.
#[test]
fn a_milled_cards_own_window_and_permanent_type_reach_the_drive_through_neither_channel() {
    run_rows(&[
        MillRow {
            what: "PAIRED CONTROL: the bare pinned mill base OFFERS, so both rows below are \
                   measured against a board whose only difference is their own object",
            board: pinned_mill_base,
            expect_offer: true,
        },
        MillRow {
            what: "INTERPOSER: a real Narcomoeba the drive mills carries its own \
                   library→graveyard window, which the proposer cannot see and the drive \
                   therefore never opens",
            board: pinned_with_narcomoeba,
            expect_offer: true,
        },
        MillRow {
            what: "DESCEND: a vanilla permanent card the drive mills would flip CR 700.11's \
                   descended_this_turn on its owner, a Player field no certified-departure \
                   relief reaches",
            board: pinned_with_vanilla_permanent_at_depth_one,
            expect_offer: true,
        },
    ]);
}

// ─────────────────────────────────────────────────────────────────────────────────────────
// THE ACCEPT PATH
//
// Every row above stops at the OFFER. These two go past it: the route the mill period takes
// once accepted, and what the collapse at the CR 500.5 boundary actually delivers.
//
// The iteration count an accepted mill shortcut DELIVERS is DEFERRED(phase 2) and is asserted
// nowhere below — these rows read a DIRECTION (cards moved, marks gone), never a magnitude.
// ─────────────────────────────────────────────────────────────────────────────────────────

use engine::analysis::decision_template::IterationCount;
use engine::analysis::loop_check::ShortcutResponse;
use engine::analysis::resource::ResourceAxis;
use engine::game::engine::apply;
use engine::types::actions::GameAction;
use engine::types::game_state::{PayableResource, PersistentAxisMaterialization};

/// Everything P0's accept registered. The stash discriminant is the only observable of the
/// route from outside the engine crate — `LoopCollapseRoute` is private to `game::engine`.
fn registered(state: &GameState) -> &[PersistentAxisMaterialization] {
    state
        .pending_unbounded_materialization
        .get(&P0)
        .map_or(&[], Vec::as_slice)
}

/// Whether P0's accept took the concrete replay. Panics rather than answering on an EMPTY
/// stash, so "not the replay" can never be satisfied by an accept that registered nothing.
fn took_the_replay(state: &GameState, why: &str) -> bool {
    let stash = registered(state);
    assert!(
        !stash.is_empty(),
        "{why}: P0's accept registered NOTHING — any route claim about it is vacuous"
    );
    stash
        .iter()
        .all(|m| matches!(m, PersistentAxisMaterialization::DriveSequence { .. }))
}

/// The board driven by one real buyback+convoke recast to its CR 732.2a offer.
fn offer_state(state: GameState) -> GameState {
    let state = drive_sprout_cast(state).state().clone();
    assert!(
        matches!(state.waiting_for, WaitingFor::LoopShortcut { proposer, .. } if proposer == P0),
        "reach-guard: the live recast must surface P0's offer, got {:?}",
        state.waiting_for
    );
    state
}

/// P0 declares `Fixed(n)`; every living opponent accepts in APNAP order.
fn declare_and_accept_all(state: &mut GameState, n: u32) {
    apply(
        state,
        P0,
        GameAction::DeclareShortcut {
            count: IterationCount::Fixed(n),
            template: None,
        },
    )
    .expect("the proposer declares the object-growth shortcut");
    while let WaitingFor::RespondToShortcut { player, .. } = state.waiting_for.clone() {
        apply(
            state,
            player,
            GameAction::RespondToShortcut {
                response: ShortcutResponse::Accept,
            },
        )
        .expect("each living opponent accepts");
    }
}

/// Pass priority through the production path until the CR 500.5 boundary raises the collapse
/// prompt. Bounded so a wedge fails loudly instead of hanging.
fn drive_to_collapse_boundary(state: &mut GameState) {
    for _ in 0..64 {
        if matches!(
            state.waiting_for,
            WaitingFor::PayAmountChoice {
                resource: PayableResource::LoopCollapse { .. },
                ..
            }
        ) {
            return;
        }
        let WaitingFor::Priority { player } = state.waiting_for.clone() else {
            panic!("unexpected non-Priority prompt {:?}", state.waiting_for)
        };
        apply(state, player, GameAction::PassPriority)
            .expect("pass priority toward the CR 500.5 boundary");
    }
    panic!("no collapse prompt within 64 passes");
}

/// Every seat's library size, in seat order.
fn library_sizes(state: &GameState) -> Vec<(PlayerId, usize)> {
    state
        .players
        .iter()
        .map(|p| (p.id, p.library.len()))
        .collect()
}

/// **Adding the mill flips the accepted route to the concrete replay, on a board whose
/// batched payload is NOT empty.** Written as ONE test so the arms are inseparable: the
/// Altar arm alone is satisfiable by a blanket route change, the Altar-free arm alone by
/// never wiring the mill conjunct at all.
///
/// ALTAR-FREE (the discriminator): the untouched combo board grows only Saprolings, so its
/// accept keeps the O(1) batched mint. ALTAR: the same board plus one real Altar of the Brood
/// publishes a `LibraryDelta` axis for which no batched item exists, and the accept switches
/// to the replay — while STILL registering a token axis on the batched arm, which is what
/// makes this the non-empty-`batched` population rather than the pure-mill one.
///
/// REVERT PROBE: delete the `unbatchable_deferred` disjunct at the `game::engine` route seam
/// ⇒ the Altar arm falls back to the batched mint and reds; the Altar-free arm stays green.
#[test]
fn the_mill_flips_a_non_empty_batched_period_onto_the_replay() {
    let mut altar_free = offer_state(load_realistic_dump());
    declare_and_accept_all(&mut altar_free, 2);
    assert!(
        !took_the_replay(&altar_free, "altar-free"),
        "the Altar-free board's only growth is batchable, so its accept keeps the batched mint"
    );
    assert!(
        registered(&altar_free)
            .iter()
            .any(|m| matches!(m, PersistentAxisMaterialization::Tokens(_))),
        "reach-guard: the batched payload carries a token axis, so the Altar arm below \
         differs from this one by the MILL and not by an empty payload"
    );

    let mut milling = offer_state(mill_base());
    declare_and_accept_all(&mut milling, 2);
    assert!(
        took_the_replay(&milling, "altar"),
        "CR 732.2c: the accepted proposal promises a library decline no batched item can \
         deliver, so the same period — token growth included — must route to the replay"
    );
}

/// **An accepted mill collapse MOVES cards and RETIRES its marks.** The first row in this
/// module that goes past the offer, and the one the offer rows cannot substitute for: a
/// Replay arm that failed to deliver the `LibraryDelta` would move zero cards and leave a
/// permanent infinity badge, and every assertion above would still pass.
///
/// Direction only — a NONZERO decline per victim and an absent mark. The delivered iteration
/// COUNT is DEFERRED(phase 2) and is deliberately not asserted.
///
/// REVERT PROBE: delete the `unbatchable_deferred` disjunct at the `game::engine` route seam
/// ⇒ the accept routes to the batched mint, which carries no `LibraryDelta`, so no opponent
/// library declines and P0 keeps its `LibraryDelta` marks ⇒ **FAILS**.
#[test]
fn an_accepted_mill_collapse_moves_cards_and_retires_its_marks() {
    const N: u32 = 3;

    let mut state = offer_state(mill_base());
    let before = library_sizes(&state);
    let victims: Vec<PlayerId> = state
        .players
        .iter()
        .map(|p| p.id)
        .filter(|&id| id != P0)
        .collect();
    assert!(
        !victims.is_empty(),
        "reach-guard: the dump seats opponents for the Altar to mill"
    );

    declare_and_accept_all(&mut state, N);
    let marked_axes = state
        .unbounded_resources
        .get(&P0)
        .cloned()
        .unwrap_or_default();
    for victim in &victims {
        assert!(
            marked_axes.contains(&ResourceAxis::LibraryDelta(*victim)),
            "reach-guard: the accepted proposal carries {victim:?}'s library axis, so its \
             retirement below is a decision rather than an absence that was never there"
        );
    }

    drive_to_collapse_boundary(&mut state);
    apply(&mut state, P0, GameAction::SubmitPayAmount { amount: N })
        .expect("P0 submits the finite loop-collapse count");

    let after = library_sizes(&state);
    for victim in &victims {
        let lookup = |sizes: &[(PlayerId, usize)]| {
            sizes
                .iter()
                .find(|(id, _)| id == victim)
                .map(|(_, n)| *n)
                .expect("every seat is in both readings")
        };
        assert!(
            lookup(&after) < lookup(&before),
            "CR 701.17a: the collapse must actually mill {victim:?} — library went {} -> {}",
            lookup(&before),
            lookup(&after)
        );
    }

    let surviving = state
        .unbounded_resources
        .get(&P0)
        .cloned()
        .unwrap_or_default();
    for victim in &victims {
        assert!(
            !surviving.contains(&ResourceAxis::LibraryDelta(*victim)),
            "CR 732.2c: the collapse DELIVERED {victim:?}'s library decline, so it ends that \
             axis's ∞ mark instead of leaving an infinite-mill badge standing"
        );
    }
}
