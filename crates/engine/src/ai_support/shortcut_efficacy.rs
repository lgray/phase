// engine-citation-gate: symbol anchors only
//! Controller-relative confinement of a polled player's priority window —
//! stage 2 of [`super::smart_shortcut_response`].
//!
//! CITATION FORM: rule NUMBER only, matching the enrolled test sibling
//! `tests/integration/shorten_efficacy.rs`. The number IS the greppable
//! heading — `grep '^400.1' docs/MagicCompRules.txt` resolves any citation
//! below. `docs/MagicCompRules.txt` is gitignored and re-fetched per checkout,
//! so a line anchor pins a citation to whichever rules revision its author
//! happened to hold.
//!
//! # 1. This is AI POLICY, not a rule
//!
//! CR 732.2b grants an **unconditioned** binary
//! option: "Each other player, in turn order starting after the player who
//! suggested the shortcut, may either accept the proposed sequence, or shorten
//! it by naming a place where they will make a game choice that's different
//! than what's been proposed." There is no criterion, no outcome test, and no
//! efficacy test; the rule explicitly does not even require the shortening
//! player to say what the new choice will be. CR 732.2c adds the only
//! obligation, and it is downstream: "the player who now has priority must make
//! a different game choice than what was originally proposed for that player."
//! Activating a fetchland IS a different game choice, so a fetchland Shorten
//! fully satisfies it.
//!
//! Nothing in the Comprehensive Rules therefore grounds this module's decision.
//! It is an AI policy: a seat whose only available response cannot touch the
//! loop spends its window achieving nothing, and the engine should not burn a
//! real priority window on it. Every CR number below is cited **only for what
//! its text says**, and is used as an *input* the classifier reads (which zones
//! are per-player, who owns what) — never as a licence for the decision.
//!
//! # 2. Fail-closed direction, and why the wildcard is conservative HERE
//!
//! [`WindowReach::MayInterfere`] is the default for every unrecognized shape.
//! That direction is deliberate and is the opposite of the discipline
//! `game::ability_scan` enforces on itself. `ability_scan`'s default is
//! `Axes::NONE` ("this ability reads nothing"), which is its *unsafe*
//! direction — a newly added reader classified inert would ride a false
//! auto-resolution — so that module forbids wildcards outright. Here the
//! default says "this action might interfere", which can produce only a **false
//! `Shorten`** (the polled seat takes a priority window it did not need — it
//! costs beats, never a game) and **never a false `Accept` from a shape it does
//! not recognize** (a seat accepting its own loss while holding a real out).
//! The qualifier is load-bearing; see the residual below. An unknown nesting
//! container is interference regardless of what it nests, so there is no
//! nested-carrier set and no recursion-arm set left to drift, and no
//! hand-maintained allowlist to rot.
//!
//! That argument covers unrecognized *shapes*, and it leaves exactly one
//! residual uncovered: a clause the PARSER silently swallows never becomes a
//! shape here at all, so the ability this module is handed looks strictly MORE
//! confined than the printed card is — and narrowing apparent reach is the
//! dangerous direction, because it produces a false `Accept`, not a false
//! `Shorten`. The residual is stated structurally rather than pinned to a card,
//! because a card witness is only valid until the classifier moves under it:
//! this doc previously named `Invoke Justice[0]` — a lone
//! [`Effect::ChangeZone`] (`origin: Graveyard`, `target: Typed{controller:
//! You}`, no cost, no `sub_ability`) whose printed sentence continues "then
//! distribute four +1/+1 counters among any number of creatures and/or Vehicles
//! target player controls" — and the entry gate added below has since
//! reclassified it `MayInterfere`, because its battlefield entry is not
//! provably tapped. That narrowed the residual; it did not close it. What
//! remains live is the same shape at a destination the entry gate does not
//! constrain: a non-battlefield destination, or a provably tapped entry. No
//! widening of this module can reach any of it — it cannot read a clause that
//! never arrived, so the residual is owned by the parser, not the classifier.
//!
//! The structural guard is the compiler, not a test:
//! [`ability_window_reach`] destructures [`AbilityDefinition`] **without
//! `..`**, and every allowlisted [`Effect`] arm is likewise `..`-free, so a new
//! field on the definition or on an allowlisted variant fails to compile until
//! it is classified. Precedent: `game::ability_scan::ability_definition_axes`.
//!
//! # 3. `analysis::ability_graph::collect_effects` is deliberately NOT used
//!
//! Two independent reasons, one mechanism. Its inner recursion ends `_ => {}`
//! (`analysis/ability_graph.rs`, whose own doc says "a wildcard covers the leaf
//! variants") — silent fail-open. And it walks **4 of `AbilityDefinition`'s 38
//! fields** (`effect`, `sub_ability`, `else_ability`, `mode_abilities`); it
//! never inspects `cost` at all, so `Sacrifice` / `Discard` / `Mill` /
//! `ExileMaterials` costs — every one of which can reach another player's
//! resources — would be structurally invisible.

use crate::game::game_object::GameObject;
use crate::types::ability::{
    AbilityCost, AbilityDefinition, ControllerRef, Effect, FilterProp, PlayerFilter,
    SearchDestinationSplit, TargetFilter,
};
use crate::types::actions::GameAction;
use crate::types::game_state::GameState;
use crate::types::identifiers::ObjectId;
use crate::types::zones::{EtbTapState, Zone};

/// How far a single available action can reach, relative to the player who
/// would take it.
///
/// A fourth axis, orthogonal to the three the engine already carries:
/// `ai_support::FlatPriorityActionClass` classifies *action shape*,
/// `game::ability_rw::WriteScope` classifies *object identity*, and
/// `game::ability_scan::Axes` classifies *AST reads*. This one classifies
/// **controller-relative confinement**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowReach {
    /// Everything this action can touch belongs to the player taking it.
    OwnResourcesOnly,
    /// This action can reach something outside the actor's own resources —
    /// or its shape is not recognized, which is treated identically.
    MayInterfere,
}

impl WindowReach {
    fn of(own_resources_only: bool) -> Self {
        if own_resources_only {
            Self::OwnResourcesOnly
        } else {
            Self::MayInterfere
        }
    }

    /// Absorbing fold: one interfering component makes the whole interfering.
    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::OwnResourcesOnly, Self::OwnResourcesOnly) => Self::OwnResourcesOnly,
            _ => Self::MayInterfere,
        }
    }

    fn may_interfere(self) -> bool {
        matches!(self, Self::MayInterfere)
    }
}

/// True only when every object this filter can match is PROVEN to belong to the
/// acting player.
///
/// CR 400.1: "Each player has their own library, hand, and graveyard.
/// The other zones are shared by all players." A bare `Zone` reference is
/// therefore ambiguous by rule — it can never answer *whose* zone — so the
/// player-qualified authority has to come from the filter. CR 400.3
/// names exactly three zones ("If an object would go to any library, graveyard,
/// or hand other than its owner's, it goes to its owner's corresponding zone")
/// and CR 108.3 defines the owner, which is why owner/controller, not
/// zone, is the axis this predicate reads.
///
/// Unproven is not owned: everything else (`Any`, `None`, `Player`, `Opponent`,
/// `ParentTarget*`, anaphors, specific ids) is `false`.
fn filter_is_actor_owned(filter: &TargetFilter) -> bool {
    match filter {
        TargetFilter::SelfRef | TargetFilter::Controller => true,
        TargetFilter::Typed(typed) => {
            typed.controller == Some(ControllerRef::You)
                || typed.properties.iter().any(|prop| {
                    matches!(
                        prop,
                        FilterProp::Owned {
                            controller: ControllerRef::You
                        }
                    )
                })
        }
        // An `Or` matches an object when ANY leg does, so every leg must be
        // proven; an `And` matches only when ALL legs do, so one proven leg is
        // enough. The emptiness guards keep a degenerate `filters: []` from
        // being proven by a vacuous `all()`.
        TargetFilter::Or { filters } => {
            !filters.is_empty() && filters.iter().all(filter_is_actor_owned)
        }
        TargetFilter::And { filters } => filters.iter().any(filter_is_actor_owned),
        _ => false,
    }
}

/// Is a landing zone confined — i.e. can the seat NOT act on what arrives there,
/// inside the window the Shorten hands back?
///
/// This is the single authority for that question, and it exists because the two
/// doors out of a library drifted apart exactly once: the `ChangeZone` arm gated
/// `Zone::Hand` while the `SearchLibrary` split arm gated only `Zone::Battlefield`,
/// so a `SearchDestinationSplit` routing cards to hand stayed classified confined.
/// MEASURED at that point: all twelve split carriers in `card-data.json` route
/// something to hand — nine via `rest_destination` (the Cultivate class: land to
/// the battlefield tapped, the REST to hand) and three via `primary_destination`
/// (Final Parting, Fork in the Road, Jarad's Orders). Two call sites answering one
/// question is how that happens, so there is now one function and no second answer.
///
/// * `Zone::Battlefield` — CR 110.5b: permanents enter untapped unless something
///   says otherwise, and an untapped permanent taps for mana during the very cast
///   it funds (CR 601.2g). Only a PROVABLY tapped arrival is confined.
/// * `Zone::Hand` — a card in hand is a castable card. After the spell that put it
///   there resolves the active player receives priority (CR 117.3b) and priority
///   then passes in turn order (CR 117.3d), so the responding seat gets it back
///   still inside this window. The AST carries nothing that could prove the card
///   uncastable.
/// * `Zone::Stack` — strictly stronger than the hand case, and the one a `_` arm hid.
///   CR 405.1: a cast spell's card is put on the stack; CR 608.1: the object on top
///   of the stack resolves once all players pass. A card that LANDS there is past
///   casting already and resolves inside this very window, so the seat does not even
///   need priority to get its effect.
/// * `Zone::Command` — CR 903.8: "A player may cast a commander they own from the
///   command zone", and CR 114.1 puts emblems there carrying abilities of their own.
///   It is a zone the seat acts FROM, so it belongs with hand rather than with the
///   need-further-permission group this doc previously filed it under.
/// * `Zone::Library` / `Zone::Graveyard` / `Zone::Exile` — the card lands where the
///   actor needs some FURTHER permission to use it, and that permission would itself
///   be an ability this fold already reads.
///
/// Callers still apply their own extra conjuncts (the battlefield riders on
/// `ChangeZone`); this answers the destination question only.
fn landing_zone_is_confined(destination: Zone, tapped: EtbTapState) -> bool {
    // EXHAUSTIVE ON PURPOSE — no wildcard. `Zone` is a closed seven-variant enum
    // (CR 400.1), and a `_` arm here is FAIL-OPEN in a module whose every other
    // default is fail-closed: it answers "confined" for any variant nobody thought
    // about. That is not hypothetical. This function shipped as
    // `Battlefield => …, Hand => false, _ => true`, and the `_` silently absorbed
    // `Zone::Stack` — a live `ChangeZone` destination (`game::zones`) — so a node
    // landing a card on the stack read as confined, contributing `OwnResourcesOnly` to
    // a fold whose whole purpose is to decide whether an Accept is safe. (Whether any
    // one node's verdict becomes an Accept depends on the rest of the fold; what is
    // certain is that this arm was voting the wrong way.) MEASURED at this candidate's
    // projection: 0 `Stack` destinations and 1 `Command` (Hellkite Courser, whose node
    // lives in `triggers` and is therefore never folded), so the fix is latent today
    // and is rated on what it does when reached, not on how often it is reached.
    // The doc above listed "graveyard, library, exile, command" as the remainder and
    // never mentioned Stack, which is exactly what a wildcard costs you: the arm list
    // stops being a claim the compiler checks. Adding a `Zone` variant must break this
    // build.
    match destination {
        Zone::Battlefield => matches!(tapped, EtbTapState::Tapped),
        Zone::Hand | Zone::Stack | Zone::Command => false,
        Zone::Library | Zone::Graveyard | Zone::Exile => true,
    }
}

/// Reach of one effect node. Three allowlisted shapes; everything else is
/// interference by default (see the module doc, §2).
fn effect_window_reach(effect: &Effect) -> WindowReach {
    match effect {
        // Absent `target_player` is an engine parser convention for "the
        // actor's own library". It is a true description of engine behaviour
        // and it is NOT the safety warrant for this arm, because it is NOT
        // reliable: MEASURED over `data/card-data.json`, five abilities whose
        // Oracle text explicitly names a FOREIGN library still carry a
        // `SearchLibrary` node with `target_player: None` — Head Games[0],
        // Jester's Mask[0], Haunting Echoes[0], Jace, Architect of Thought[2],
        // and the SECOND search of Sadistic Sacrament[0] (whose first search
        // does carry a `Typed` target_player). No aggregate count is claimed
        // here: "how many abilities name a foreign library" has no
        // scope-independent answer, and an unscoped census is worse than none.
        //
        // What makes the arm safe is the ABSORBING ability-level fold in
        // `ability_window_reach`: a foreign-library ability carries some
        // sibling effect or cost that folds to `MayInterfere` regardless of
        // what `target_player` says. `Haunting Echoes[0]` is the in-suite
        // witness for that fold — see
        // `a_foreign_library_search_is_confined_alone_then_absorbed`. Bribery[0]
        // and Praetor's Grasp[0] are also `MayInterfere`, but they witness the
        // OTHER path: both carry `target_player: Typed`, so this arm catches
        // them directly and the fold never has to.
        //
        // CR 400.1 is cited here only for why a bare `Zone::Library`
        // reference is ambiguous in the first place.
        //
        // `split` IS read, and it is the second door into the battlefield. A
        // cultivate-class search moves its own found cards
        // (`SearchDestinationSplit { primary_destination, primary_enter_tapped,
        // rest_destination }`) WITHOUT any `ChangeZone` sub-ability, so the tap
        // gate on that arm cannot see it. Same rule (CR 110.5b), same
        // fail-closed direction: a split may reach the battlefield only through
        // its `primary_destination` and only when `primary_enter_tapped` proves
        // `Tapped`. `rest_destination` carries NO tap state at all, so it can
        // never be proven tapped and a battlefield `rest_destination` is reach
        // by construction.
        Effect::SearchLibrary {
            source_zones: _,
            filter: _,
            count: _,
            reveal: _,
            target_player,
            selection_constraint: _,
            split,
        } => {
            // Destructured `..`-free for the same reason `Effect::ChangeZone` is: a
            // new field on `SearchDestinationSplit` (a `rest_enter_tapped`, say) must
            // be a COMPILE ERROR here rather than a silently ignored arrival modifier.
            // Reading the split by `.field` is exactly how the `Hand` hole survived.
            let split_entry_is_confined = split.as_ref().is_none_or(
                |SearchDestinationSplit {
                     primary_destination,
                     primary_count: _,
                     primary_enter_tapped,
                     rest_destination,
                 }| {
                    // Both destinations go through the SAME authority as the
                    // `ChangeZone` arm. `rest_destination` carries no tap state at
                    // all, so it is asked with `Unspecified` and can never be proven
                    // tapped — a battlefield `rest_destination` is reach by
                    // construction, and a hand one is reach because hand always is.
                    landing_zone_is_confined(*primary_destination, *primary_enter_tapped)
                        && landing_zone_is_confined(*rest_destination, EtbTapState::Unspecified)
                },
            );
            WindowReach::of(target_player.is_none() && split_entry_is_confined)
        }

        // CR 400.1: a library is a per-player zone, so shuffling is
        // confined exactly when the shuffled player is proven to be the actor.
        Effect::Shuffle { target } => WindowReach::of(filter_is_actor_owned(target)),

        // CR 400.1 + CR 400.3 + CR 108.3: a zone change is
        // confined when the moved object is proven actor-owned. The second
        // disjunct is the anaphoric fetch/ramp shape — `Any` is the serde
        // default, i.e. "no filter stated" ("put IT onto the battlefield") —
        // which carries no player information of its own. Its player-qualified
        // authority is the parent `SearchLibrary`, and the ability-level fold
        // is what catches a foreign one: Bribery has the identical
        // `Library -> Battlefield / Any` node and still folds to
        // `MayInterfere` because its parent search carries
        // `target_player: Typed{controller: Opponent}`.
        //
        // OWNERSHIP IS NOT ENOUGH WHEN THE DESTINATION IS THE BATTLEFIELD.
        // CR 110.5b: "Permanents enter the battlefield untapped, unflipped, face
        // up, and phased in unless a spell or ability says otherwise." A land
        // that arrives UNTAPPED taps for mana in the very window the Shorten
        // hands the responder — CR 601.2g runs mana abilities during the cast
        // they fund, and CR 302.6's summoning-sickness bar is a CREATURE rule
        // that never applies to a land. So an untapped fetch is the
        // `Effect::Mana` case below with one extra step, and the reasoning that
        // refuses to allowlist mana production refuses this too. MEASURED at
        // this base on the live parser: `Terramorphic Expanse[0]` and
        // `Evolving Wilds[0]` carry `enter_tapped: Tapped`, while
        // `Crop Rotation[0]` and `Nature's Lore[0]` — same anaphoric
        // `Library -> Battlefield / Any` node, same absent cost — carry
        // `Unspecified` and were allowlisted here until this gate.
        //
        // Fail-closed on the tap axis: ONLY a provably `Tapped` entry stays
        // confined, so `Unspecified` (the serde default, i.e. the AST said
        // nothing) and `Untapped` both fall out. A conditionally-untapped entry
        // (a shock land's pay-life choice, a land-count gate) cannot be proven
        // tapped from the AST and is therefore reach, which is the direction
        // this module's §2 default exists to take.
        //
        // TWO destinations are gated, for the same reason and by different
        // evidence. The question this module asks is never "is the moved card
        // the actor's own?" — the deleted `Effect::Mana` arm is what happens when
        // ownership is mistaken for confinement. It is "can the seat DO something
        // with the result inside the window the Shorten hands back?"
        //
        // * `Battlefield` — an untapped permanent taps for mana during the very
        //   cast it funds (CR 601.2g), so only a PROVABLY tapped arrival is
        //   confined. This applies to BOTH disjuncts, not just the anaphoric one:
        //   proven ownership does not stop an untapped land producing mana, so a
        //   graveyard-recursion shape reaching the battlefield untapped is the
        //   same defect through the other door.
        // * `Hand` — a card put into hand is a CASTABLE card. When the spell that
        //   put it there finishes resolving, the active player receives priority
        //   (CR 117.3b) and priority then passes in turn order (CR 117.3d), so the
        //   responding seat gets it back still inside this window and can cast it.
        //   That is the `Effect::Mana` argument with one extra step, exactly as the
        //   untapped fetch was, and the AST carries nothing that could prove the
        //   returned card uncastable. MEASURED with the hand gate held off, 327
        //   abilities are confined and 176 of them move a card to hand — 166
        //   through this arm and 10 through the `SearchLibrary` split, which is
        //   why both doors now share `landing_zone_is_confined`. The witnesses are
        //   instant-speed: `Auroral Procession` returns ANY graveyard card,
        //   including an instant. Gating both doors leaves 151 confined abilities,
        //   102 of which reach the battlefield, so the flagship tapped-fetch
        //   Accept is untouched: this closes a door, it does not vacate the
        //   feature.
        //
        // No other destination is gated, and that is a claim about what the seat
        // can act with, not an oversight: a move to graveyard, library or exile
        // puts the card somewhere the actor cannot cast or activate it from
        // inside this window without some FURTHER permission, which would itself
        // be an ability this fold already reads.
        //
        // PROVABLY tapped, not NOMINALLY tapped. Three more of this variant's
        // fields decide what actually arrives, and each one is read here rather
        // than ignored, because `enter_tapped` alone is not proof:
        //
        // * `enters_modified_if: Some(filter)` makes the tapped rider
        //   CONDITIONAL on the moved object's characteristics (CR 614.12: check
        //   the characteristics of the permanent as it would exist on the
        //   battlefield; CR 614.12a: any choice it requires is made first), so
        //   an object that fails the filter enters untapped while the field
        //   still reads `Tapped`. Unprovable from the AST ⇒ reach.
        // * `enters_under` (CR 110.2a: "that object enters the battlefield under
        //   that player's control unless the effect states otherwise") routes the
        //   entering permanent to whatever controller it names. Anything but the
        //   actor puts a permanent on ANOTHER player's board, which is reach by
        //   definition — tapped or not.
        // * `enters_attacking` (CR 508.4) puts the creature into combat against
        //   a defending player, planeswalker or battle. That is the clearest
        //   possible touch outside the actor's own resources, and a tapped
        //   attacker is still an attacker.
        Effect::ChangeZone {
            origin,
            destination,
            target,
            owner_library: _,
            enter_transformed: _,
            enters_under,
            enter_tapped,
            enters_attacking,
            up_to: _,
            enter_with_counters: _,
            conditional_enter_with_counters: _,
            face_down_profile: _,
            enters_modified_if,
        } => {
            let object_is_confined = filter_is_actor_owned(target)
                || (matches!(target, TargetFilter::Any)
                    && *origin == Some(Zone::Library)
                    && *destination == Zone::Battlefield);
            let entry_is_confined = landing_zone_is_confined(*destination, *enter_tapped)
                && (*destination != Zone::Battlefield
                    || (enters_modified_if.is_none()
                        && !*enters_attacking
                        && matches!(enters_under, None | Some(ControllerRef::You))));
            WindowReach::of(object_is_confined && entry_is_confined)
        }

        // NOT allowlisted — and `Effect::Mana` is the case that has to be NAMED
        // here rather than left to the reader, because an earlier revision of
        // this module allowlisted it.
        //
        // CR 106.1: "Mana is the primary resource in the game. Players spend
        // mana to pay costs, usually when casting spells and activating
        // abilities." CR 106.4 is the rule that earlier arm cited, and it quoted
        // only the FIRST sentence — "that mana goes into a player's mana pool".
        // The second sentence is the one that refutes the inference drawn from
        // it: "From there, it can be used to pay costs immediately". CR 601.2g
        // then runs mana abilities during the very cast they fund.
        //
        // So mana is FUNGIBLE REACH, and the earlier arm's inference —
        // "producing it removes nothing from the board" — answers where the mana
        // LANDS, which is not what this classifier asks. The question is whether
        // the action widens what the seat can do inside the window
        // `game::engine`'s `RespondToShortcut(Shorten)` arm hands back.
        //
        // Whether it does is NOT a function of the AST: it depends on the rest of
        // the hand, the board, the colors produced, and on what other permanents
        // trigger off mana being added (CR 603.2) or off the source leaving the
        // battlefield (CR 701.21a). This function reads ONE ability and no other
        // object, so `OwnResourcesOnly` here would be a proof it cannot
        // discharge — exactly the unknowable this module's §2 default exists for.
        //
        // It is also the CONSISTENT answer: stage 1 re-admits a sacrifice-for-mana
        // activation precisely BECAUSE the sacrifice is board-changing (see
        // `types::mana::ManaSourcePenalty::is_meaningful_priority_activation`), so
        // classifying that same action as confined here contradicted the stage
        // that handed it over.
        _ => WindowReach::MayInterfere,
    }
}

/// Reach of an activation cost. `Sacrifice` is the one cost shape that can name
/// somebody else's permanent, so it reads the same owner axis as the effects.
fn cost_window_reach(cost: &AbilityCost) -> WindowReach {
    match cost {
        // Tapping the source and spending mana from your own pool (CR 106.4)
        // touch nothing outside the actor.
        //
        // The `Mana` arm here is the SPENDING side and it stays allowlisted;
        // only PRODUCING mana is reach. Paying a mana cost consumes a resource
        // the actor already holds and hands nothing new to the seat, whereas the
        // effect-side `_` arm above explains why adding mana widens what the
        // seat can do inside the window. Flipping this arm would classify every
        // ability with a mana cost as interference and vacate the feature, so
        // its absence from that change is deliberate.
        AbilityCost::Tap => WindowReach::OwnResourcesOnly,
        AbilityCost::Mana { cost: _ } => WindowReach::OwnResourcesOnly,
        AbilityCost::Sacrifice(sacrifice) => {
            WindowReach::of(filter_is_actor_owned(&sacrifice.target))
        }
        AbilityCost::Composite { costs } => costs
            .iter()
            .fold(WindowReach::OwnResourcesOnly, |acc, sub| {
                acc.or(cost_window_reach(sub))
            }),
        _ => WindowReach::MayInterfere,
    }
}

/// Reach of a whole ability, folded over every field that can carry a
/// player-qualified authority.
///
/// The `..`-free destructure is the structural guard (module doc §2): a new
/// `AbilityDefinition` field is a compile error here until it is classified as
/// walked, conservative-when-present, or reasoned read-free.
fn ability_window_reach(def: &AbilityDefinition) -> WindowReach {
    let AbilityDefinition {
        // ---- walked ----
        effect,
        sub_ability,
        else_ability,
        mode_abilities,
        cost,
        player_scope,
        // ---- conservative-when-present: each can name a player, an object, or
        //      a payment outside the actor, and none is walked ----
        activator_filter,
        starting_with,
        target_chooser,
        unless_pay,
        distribute,
        cost_reduction,
        condition,
        duration,
        multi_target,
        target_constraints,
        modal,
        repeat_for,
        announced_x,
        repeat_until,
        optional_for,
        iteration_kind_binding,
        // ---- read-free ----
        // Ability class (activated/triggered/static); no player reference.
        kind: _,
        // Display strings only.
        description: _,
        target_prompt: _,
        // Activation gates: when, from which zone, with which mana, and under
        // which keyword this ability may be activated. NOT player-free, and the
        // earlier "no player reference" claim here was simply false: an
        // `ActivationRestriction::RequiresCondition` carries a `ParsedCondition`
        // that can name a player, and several do ("Activate only if an opponent
        // lost life this turn"). Read-free anyway, which is the load-bearing
        // part: a restriction narrows WHEN an ability may be activated, never
        // WHAT it reaches. It contributes no effect, no target and no payment,
        // so it can only make a window rarer — never wider.
        activation_restrictions: _,
        activation_mana_payment_restriction: _,
        activation_zone: _,
        ability_tag: _,
        // Booleans and scalars that gate shape, never a player or an object.
        optional_targeting: _,
        optional: _,
        target_choice_timing: _,
        min_x_value: _,
        cant_be_copied: _,
        forward_result: _,
        target_selection_mode: _,
        sub_link: _,
        sibling_condition: _,
    } = def;

    let mut acc = effect_window_reach(effect);
    if let Some(sub) = sub_ability {
        acc = acc.or(ability_window_reach(sub));
    }
    if let Some(other) = else_ability {
        acc = acc.or(ability_window_reach(other));
    }
    for mode in mode_abilities {
        acc = acc.or(ability_window_reach(mode));
    }
    if let Some(cost) = cost {
        acc = acc.or(cost_window_reach(cost));
    }
    // CR 400.1: an untargeted mass effect scoped to anyone but the actor
    // reaches another player's resources by construction.
    if let Some(scope) = player_scope {
        acc = acc.or(WindowReach::of(matches!(scope, PlayerFilter::Controller)));
    }

    let conservative_when_present = activator_filter.is_some()
        || starting_with.is_some()
        || target_chooser.is_some()
        || unless_pay.is_some()
        || distribute.is_some()
        || cost_reduction.is_some()
        || condition.is_some()
        || duration.is_some()
        || multi_target.is_some()
        || !target_constraints.is_empty()
        || modal.is_some()
        || repeat_for.is_some()
        || announced_x.is_some()
        || repeat_until.is_some()
        || optional_for.is_some()
        || iteration_kind_binding.is_some();
    acc.or(WindowReach::of(!conservative_when_present))
}

/// Does this object carry rules content this module cannot classify?
///
/// Single authority for that question — both entry points route through it, because
/// the previous shape (each entry point spelling out its own three-field check) is how
/// two answers to one question drift apart, and this module has already paid for that
/// once with `landing_zone_is_confined`.
///
/// The list is deliberately EVERY rules-bearing field `game::printed_cards` writes and
/// `ability_window_reach` cannot read, not a curated subset of the ones known to be
/// dangerous. A curated subset is an allowlist maintained by whoever remembers to
/// update it; MEASURED, the previous three-field version claimed in its own doc comment
/// to cover what `printed_cards` splits a face into, and was wrong by eight fields.
/// `obj.keywords` alone carries printed **Cascade** (`game::triggers`: printed Cascade
/// lives in `obj.keywords` and never reaches `trigger_definitions`), so a Cascade spell
/// whose printed `abilities` all read confined would have been proven
/// `OwnResourcesOnly` while resolving it casts a free spell of arbitrary reach inside
/// the very window the seat just declined to keep.
///
/// MEASURED cost of widening from three fields to all of them, on this candidate's own
/// projection: of the 53 cards that survived the three-field gate, 10 now flip (8 on
/// `keywords`, 1 `modal`, 1 `additional_cost`), leaving 43. The class this classifier
/// exists to protect is untouched — Terramorphic Expanse, Evolving Wilds and Rampant
/// Growth carry none of these fields.
///
/// A presence gate is conservative by construction: it can only move a verdict toward
/// `MayInterfere`, which per §2 is the direction that costs efficacy rather than games.
fn carries_unreadable_rules_content(object: &GameObject) -> bool {
    !object.trigger_definitions.is_empty()
        || !object.replacement_definitions.is_empty()
        || !object.static_definitions.is_empty()
        // Keywords are rules text the fold never sees; Cascade is the sharp case.
        || !object.keywords.is_empty()
        // Casting-time modifiers: each one changes what resolving the object does or
        // what the seat may do with it, and none is an `AbilityDefinition`.
        || object.modal.is_some()
        || object.additional_cost.is_some()
        || object.strive_cost.is_some()
        || object.cleave_variant.is_some()
        || !object.casting_restrictions.is_empty()
        || !object.casting_options.is_empty()
        || !object.spellbook.is_empty()
        // A back face is an entire second face of rules content that this fold never
        // descends into. 0 carriers among today's confined set, so gating it is free.
        || object.back_face.is_some()
        // The four below were found by WRITING the staleness guard below, not by reading
        // this list again — which is the argument for the guard existing. All four are
        // rules-bearing, all four are written by `printed_cards`, and none was gated.
        // MEASURED at this candidate's projection: each is 0 among the cards that survive
        // the gate, and real document-wide (solve conditions 15, Class 38, Case 15,
        // Attraction 35), so they are latent holes rather than live ones — the same shape
        // as the `Zone::Stack` arm, and closed for the same reason.
        || object.case_state.is_some()
        || object.class_level.is_some()
        || object.intensity != 0
        || !object.attraction_lights.is_empty()
        // CR 709.5: a shared type line is two static abilities that remove the name, mana
        // cost and RULES TEXT of each locked half; CR 709.5c names the unlocked
        // designations, and CR 709.5e lets any player unlock a half as a special action at
        // any priority. So which halves are unlocked decides what rules text the permanent
        // has, `obj.abilities` is a flat list that cannot express that, and an opponent can
        // change the answer inside the very window this fold is deciding about.
        // Found by the same review round that caught the `card_type` bucket below, and it
        // is the fourth subtype-derived field of four — the other three were already here,
        // which is what made this one's absence a curated subset rather than a set.
        || object.room_unlocks.is_some()
}

/// Fold every ability an object carries. A missing object, or one carrying no
/// abilities at all, is `MayInterfere` — the fail-closed direction, and the one
/// that keeps an empty fold from being proven confined by its identity element.
///
/// `abilities` is NOT the whole of an object's rules content. `game::printed_cards`
/// spreads a card face across many more rules-bearing fields than this module can
/// classify — `carries_unreadable_rules_content` below is the enumerated authority on
/// which, and the one place to look; a count written here would only drift out of step
/// with it. This module classifies exactly one of them, because `ability_window_reach`
/// destructures an `AbilityDefinition` and the rest carry different types entirely.
/// Folding only `abilities` and returning `OwnResourcesOnly` would therefore prove a
/// card confined from whichever fraction of it happens to be ability-shaped.
///
/// That is not hypothetical. MEASURED on `data/card-data.json`: `Stunning Reversal`
/// projects `abilities` = one `ChangeZone { destination: Exile, target: SelfRef }`
/// — actor-owned, non-battlefield, hence confined on every conjunct this module
/// reads — while its entire function lives in `replacements[0]`, a `GameLoss`
/// replacement ("The next time you would lose the game this turn, instead draw
/// seven cards and your life total becomes 1"). A seat holding it would have read
/// `OwnResourcesOnly` and Accepted the very shortcut the card exists to survive.
/// Per this module's §2 that is the losing direction, and it is the same defect
/// class as the deleted `Effect::Mana` arm: reasoning from the part of the card the
/// classifier can see instead of from what the seat can do.
///
/// So a non-empty unreadable field is `MayInterfere` — not because its contents are
/// known to interfere, but because this module cannot prove they do not, which is the
/// only warrant `OwnResourcesOnly` ever has. Reading them properly (classifying
/// `TriggerDefinition` / `ReplacementDefinition` / `StaticDefinition` the way
/// `ability_window_reach` classifies an `AbilityDefinition`) is the named upgrade
/// path; until then the gate is presence, and presence is conservative.
fn object_window_reach(state: &GameState, object_id: ObjectId) -> WindowReach {
    let Some(object) = state.objects.get(&object_id) else {
        return WindowReach::MayInterfere;
    };
    if object.abilities.is_empty() {
        return WindowReach::MayInterfere;
    }
    if carries_unreadable_rules_content(object) {
        return WindowReach::MayInterfere;
    }
    object
        .abilities
        .iter()
        .fold(WindowReach::OwnResourcesOnly, |acc, ability| {
            acc.or(ability_window_reach(ability))
        })
}

/// Reach of one indexed activated ability. An unresolvable object or an
/// out-of-range index is `MayInterfere`.
///
/// Carries the same unreadable-content gate as `object_window_reach`, and for a
/// reason specific to this path rather than by symmetry: activating an ability is
/// itself a game event, so a trigger on the SAME object can fire off the activation
/// (CR 603.2) or off the cost being paid, and a static ability can change what the
/// activation is allowed to do. This module cannot classify any of that, so an object
/// carrying it is not provably confined no matter how confined the indexed ability
/// reads on its own.
fn indexed_ability_window_reach(
    state: &GameState,
    source_id: ObjectId,
    ability_index: usize,
) -> WindowReach {
    let Some(object) = state.objects.get(&source_id) else {
        return WindowReach::MayInterfere;
    };
    if carries_unreadable_rules_content(object) {
        return WindowReach::MayInterfere;
    }
    object
        .abilities
        .get(ability_index)
        .map_or(WindowReach::MayInterfere, ability_window_reach)
}

/// Stage 2's predicate: does the polled player hold any action that could reach
/// past their own resources?
///
/// Reads the actions stage 1 counted — no clone, no `find_legal_targets`, no
/// simulation, no graph build. The caller MUST hand it
/// `ai_support::stage_two_action_set`, not the bare flat list: stage 1 counts
/// sacrifice-for-mana activations that only ever live in
/// `legal_actions_by_object`, and an action never handed to this fold reaches no
/// arm at all, so the fail-closed `_ => MayInterfere` default cannot protect it.
/// Missing an action here is an Accept by OMISSION — the direction that loses
/// games.
///
/// ponytail: a fetched permanent that itself enables interference is modelled
/// only through its ARRIVAL STATE, never by walking its abilities.
/// `effect_window_reach` keeps a battlefield entry confined only when the AST
/// proves it arrives tapped (CR 110.5b), so the mana axis — the fetched land
/// that taps for mana inside this same window — is closed: an untapped entry
/// reads `MayInterfere`.
///
/// What remains unmodelled, stated precisely rather than as a blanket: a
/// permanent that arrives TAPPED and still enables interference, via an ability
/// whose cost is not tapping (e.g. sacrifice-for-mana). Reaching that needs a
/// further priority window, and this design does NOT claim one is guaranteed —
/// CR 732.1b says only that the shortcut rules *can be used* on a loop, and
/// CR 732.2a makes proposing permissive ("may suggest").
/// Scope of that remainder, on BOTH axes:
///   - across windows: a bounded miss — a seat's fetched answer goes unused for
///     THIS shortcut;
///   - within the window: the worst case is NOT bounded by "one shortcut". On
///     an `UntilLethal` offer the accepted sequence runs to lethal, so the
///     in-window cost of a missed out is elimination.
///
/// Accepted because the miss requires the out to be reachable ONLY through the
/// fetched permanent AND that permanent to arrive tapped; a directly-castable
/// answer is already caught by the top-level fold. Upgrade path: walk the
/// fetched object's own abilities if a real game shows a missed out. Owner:
/// this lane, deferral burndown.
pub(crate) fn any_action_may_interfere(state: &GameState, actions: &[GameAction]) -> bool {
    actions.iter().any(|action| match action {
        GameAction::PassPriority => false,
        GameAction::CastSpell { object_id, .. } => {
            object_window_reach(state, *object_id).may_interfere()
        }
        GameAction::ActivateAbility {
            source_id,
            ability_index,
        } => indexed_ability_window_reach(state, *source_id, *ability_index).may_interfere(),
        // Every other priority action (play a land, declare attackers, special
        // actions, ...) is unclassified and therefore interference.
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::zones::create_object;
    use crate::parser::oracle::parse_oracle_text;
    use crate::types::card::{CardFace, CardMetadata};
    use crate::types::keywords::Keyword;

    /// A card as the pipeline sees it: its REAL printed name plus its verbatim
    /// Oracle text. The name is load-bearing, not decoration — `~`
    /// normalization resolves a card's self-reference by matching its own name,
    /// so parsing "Lightning Bolt deals 3 damage to any target." under a
    /// placeholder name yields `Effect::Unimplemented` and the row would then
    /// be measuring the fail-closed arm instead of the effect's own semantics
    /// (MEASURED: that is exactly what an earlier revision of this table did).
    struct Card {
        name: &'static str,
        oracle: &'static str,
    }

    /// Parse verbatim Oracle text the way the card pipeline does and return the
    /// ability at `index`.
    fn ability(card: &Card, index: usize) -> AbilityDefinition {
        let parsed = parse_oracle_text(card.oracle, card.name, &[], &[], &[]);
        parsed
            .abilities
            .get(index)
            .unwrap_or_else(|| {
                panic!(
                    "{}[{index}] must exist; parsed {} abilities",
                    card.name,
                    parsed.abilities.len()
                )
            })
            .clone()
    }

    fn reach(card: &Card, index: usize) -> WindowReach {
        ability_window_reach(&ability(card, index))
    }

    /// True when this ability's head is the unparsed gap node, i.e. its verdict
    /// rides the fail-closed `_` arm rather than the effect's own semantics.
    fn head_is_unparsed(card: &Card, index: usize) -> bool {
        matches!(
            ability(card, index).effect.as_ref(),
            Effect::Unimplemented { .. }
        )
    }

    const TERRAMORPHIC: Card = Card {
        name: "Terramorphic Expanse",
        oracle: "{T}, Sacrifice this land: Search your library for a basic land \
                                card, put it onto the battlefield tapped, then shuffle.",
    };
    const EVOLVING_WILDS: Card = Card {
        name: "Evolving Wilds",
        oracle: "{T}, Sacrifice this land: Search your library for a basic land \
                                  card, put it onto the battlefield tapped, then shuffle.",
    };
    const RAMPANT_GROWTH: Card = Card {
        name: "Rampant Growth",
        oracle: "Search your library for a basic land card, put that card onto the battlefield \
        tapped, then shuffle.",
    };
    const DEATHRITE_SHAMAN: Card = Card {
        name: "Deathrite Shaman",
        oracle: "{T}: Exile target land card from a graveyard. Add one mana of \
                                    any color.\n{B}, {T}: Exile target instant or sorcery card \
                                    from a graveyard. Each opponent loses 2 life.\n{G}, {T}: \
                                    Exile target creature card from a graveyard. You gain 2 life.",
    };
    const SOUL_GUIDE_LANTERN: Card = Card {
        name: "Soul-Guide Lantern",
        oracle: "When this artifact enters, exile target card from a \
                                      graveyard.\n{T}, Sacrifice this artifact: Exile each \
                                      opponent's graveyard.\n{1}, {T}, Sacrifice this artifact: \
                                      Draw a card.",
    };
    const RELIC_OF_PROGENITUS: Card = Card {
        name: "Relic of Progenitus",
        oracle: "{T}: Target player exiles a card from their \
                                       graveyard.\n{1}, Exile this artifact: Exile all graveyards. \
                                       Draw a card.",
    };
    const SCAVENGING_OOZE: Card = Card {
        name: "Scavenging Ooze",
        oracle: "{G}: Exile target card from a graveyard. If it was a creature \
                                   card, put a +1/+1 counter on this creature and you gain 1 life.",
    };
    const BRIBERY: Card = Card {
        name: "Bribery",
        oracle: "Search target opponent's library for a creature card and put that card onto the \
        battlefield under your control. Then that player shuffles.",
    };
    const PRAETORS_GRASP: Card = Card {
        name: "Praetor's Grasp",
        oracle:
            "Search target opponent's library for a card and exile it face down. Then that player \
        shuffles. You may play that card for as long as it remains exiled.",
    };
    const HAUNTING_ECHOES: Card = Card {
        name: "Haunting Echoes",
        oracle: "Exile all cards from target player's graveyard other than basic land cards. For \
        each card exiled this way, search that player's library for all cards with the same name \
        as that card and exile them. Then that player shuffles.",
    };
    const THOUGHTSEIZE: Card = Card {
        name: "Thoughtseize",
        oracle: "Target player reveals their hand. You choose a nonland card from \
                                it. That player discards that card. You lose 2 life.",
    };
    const DURESS: Card = Card {
        name: "Duress",
        oracle: "Target opponent reveals their hand. You choose a noncreature, nonland \
                          card from it. That player discards that card.",
    };
    const SURVEYORS_SCOPE: Card = Card {
        name: "Surveyor's Scope",
        oracle:
            "{T}, Exile this artifact: Search your library for up to X basic land cards, where X \
        is the number of players who control at least two more lands than you. Put those \
        cards onto the battlefield, then shuffle.",
    };
    const ASSASSINS_TROPHY: Card = Card {
        name: "Assassin's Trophy",
        oracle: "Destroy target permanent an opponent controls. Its controller \
                                    may search their library for a basic land card, put it onto \
                                    the battlefield, then shuffle.",
    };
    const LIGHTNING_BOLT: Card = Card {
        name: "Lightning Bolt",
        oracle: "Lightning Bolt deals 3 damage to any target.",
    };
    const WRATH_OF_GOD: Card = Card {
        name: "Wrath of God",
        oracle: "Destroy all creatures. They can't be regenerated.",
    };
    const NATURALIZE: Card = Card {
        name: "Naturalize",
        oracle: "Destroy target artifact or enchantment.",
    };
    const DIVINATION: Card = Card {
        name: "Divination",
        oracle: "Draw two cards.",
    };
    const PATH_TO_EXILE: Card = Card {
        name: "Path to Exile",
        oracle: "Exile target creature. Its controller may search their library for a basic land \
        card, put that card onto the battlefield tapped, then shuffle.",
    };
    /// Nature's Lore, verbatim (MTGJSON `.data["Nature's Lore"][0].text`). The
    /// MINIMAL PAIR against `RAMPANT_GROWTH` above: same sentence shape, same
    /// absent cost, same anaphoric `Library -> Battlefield / Any` node — and it
    /// differs by the ONE printed word (`tapped`) the gate reads.
    const NATURES_LORE: Card = Card {
        name: "Nature's Lore",
        oracle: "Search your library for a Forest card, put that card onto the battlefield, \
                 then shuffle.",
    };
    /// Crop Rotation, verbatim (MTGJSON, both lines). The reviewer's named
    /// residual. MEASURED on the live parser: the additional-cost line is NOT
    /// carried (`cost: None`, one ability), so its verdict rides the zone-change
    /// node alone — which is exactly why it was allowlisted before this gate.
    const CROP_ROTATION: Card = Card {
        name: "Crop Rotation",
        oracle: "As an additional cost to cast this spell, sacrifice a land.\n\
                 Search your library for a land card, put that card onto the battlefield, \
                 then shuffle.",
    };
    /// Cultivate, verbatim (MTGJSON). The SECOND door onto the battlefield: its
    /// search carries a `SearchDestinationSplit` and moves the found cards
    /// ITSELF, with no `ChangeZone` sub-ability for the tap gate to read.
    /// MEASURED over the corpus: 12 abilities carry a split at all, and every
    /// battlefield-primary one prints "tapped" — so this fixture is the class,
    /// not an example of it.
    const CULTIVATE: Card = Card {
        name: "Cultivate",
        oracle: "Search your library for up to two basic land cards, reveal those cards, put one \
                 onto the battlefield tapped and the other into your hand, then shuffle.",
    };
    const SPREADING_SEAS: Card = Card {
        name: "Spreading Seas",
        oracle: "Enchant land\nWhen this Aura enters, draw a card.\nEnchanted land is an Island.",
    };
    const DARK_RITUAL: Card = Card {
        name: "Dark Ritual",
        oracle: "Add {B}{B}{B}.",
    };
    const LOTUS_PETAL: Card = Card {
        name: "Lotus Petal",
        oracle: "{T}, Sacrifice this artifact: Add one mana of any color.",
    };

    /// Verbatim from the pinned MTGJSON `AtomicCards.json` (`{3}{B}`, Instant),
    /// not from memory. The two lines matter in opposite directions: the SECOND
    /// is all this module can read, and the FIRST is the entire card.
    const STUNNING_REVERSAL: Card = Card {
        name: "Stunning Reversal",
        oracle: "The next time you would lose the game this turn, instead draw seven cards \
                 and your life total becomes 1.\nExile Stunning Reversal.",
    };

    /// V8 — the classifier is correct across the whole class, in BOTH
    /// directions. Acceptance (a) must be confined; every acceptance-(b)
    /// interaction, the graveyard-hate class the origin-keyed predecessor rule
    /// wrongly confined, the foreign-library and foreign-hand analogues, and
    /// the hostile pair must all be interference.
    ///
    /// Revert-probe: deleting any allowlisted arm flips its acceptance-(a) rows
    /// to `MayInterfere`. That clause is TIGHTER than it used to be — there are
    /// now three allowlisted arms and all three are exercised by the three
    /// `confined` rows, whereas the `Effect::Mana` arm this table shipped with
    /// had no acceptance-(a) row witnessing it at all.
    ///
    /// This table no longer witnesses the TRIVIALIZE mutant, and the sentence
    /// that said it did has been removed rather than weakened. It claimed that
    /// trivializing `filter_is_actor_owned` to `true` flips the
    /// `Deathrite Shaman[0]` row to `OwnResourcesOnly`; MEASURED, that stopped
    /// being true when `Effect::Mana` left the allowlist. `Deathrite Shaman[0]`
    /// carries its "Add one mana of any color" clause as a `sub_ability`, which
    /// now folds `MayInterfere` on its own, so the absorbing fold holds the row
    /// no matter what the owner axis says. The owner axis keeps its full
    /// discriminating power; only this table's claim to be its witness went
    /// stale. Its witnesses are the two tests that assert on
    /// `filter_is_actor_owned` DIRECTLY —
    /// `deathrite_and_terramorphic_are_separated_only_by_the_owner_axis` and
    /// `composite_filters_prove_ownership_by_set_logic`, both in this module. Do
    /// NOT weaken any assertion here to restore the old sentence.
    #[test]
    fn window_reach_matches_the_measured_class_table() {
        let confined: &[(&Card, usize)] = &[
            (&TERRAMORPHIC, 0),
            (&EVOLVING_WILDS, 0),
            (&RAMPANT_GROWTH, 0),
            // Surveyor's Scope is deliberately NOT here — it is the fetch
            // class's fail-closed member and gets its own row below, because
            // its verdict rides its `Unimplemented` head and its
            // non-allowlisted `Exile` cost rather than the classifier's
            // hostile-detection logic.
        ];
        let interfering: &[(&Card, usize)] = &[
            // acceptance (b) — real interaction must keep Shortening
            (&LIGHTNING_BOLT, 0),
            (&WRATH_OF_GOD, 0),
            (&NATURALIZE, 0),
            (&DIVINATION, 0),
            (&PATH_TO_EXILE, 0),
            // the B1 defect class: a graveyard is a per-player zone (CR 400.1)
            // and none of these filters proves whose
            (&DEATHRITE_SHAMAN, 0),
            (&DEATHRITE_SHAMAN, 1),
            (&SOUL_GUIDE_LANTERN, 0),
            (&RELIC_OF_PROGENITUS, 1),
            (&SCAVENGING_OOZE, 0),
            // Library analogue — caught by `SearchLibrary.target_player`
            (&BRIBERY, 0),
            (&PRAETORS_GRASP, 0),
            // Library analogue caught by the ABSORBING FOLD instead: its
            // search node carries `target_player: None` and is confined on its
            // own, so neither row above covers this path. The premise that
            // makes it a fold witness is pinned separately by
            // `a_foreign_library_search_is_confined_alone_then_absorbed`.
            (&HAUNTING_ECHOES, 0),
            // Hand analogue — these do not ride `ChangeZone` at all; they fall
            // to the fail-closed default
            (&THOUGHTSEIZE, 0),
            (&DURESS, 0),
            // hostile — a parsed removal spell that names an opponent's
            // permanent explicitly
            (&ASSASSINS_TROPHY, 0),
        ];

        for (card, index) in confined {
            let name = card.name;
            assert!(
                !head_is_unparsed(card, *index),
                "VACUITY GUARD: {name}[{index}] must PARSE — a confined verdict on an \
                 Effect::Unimplemented head would be impossible, so this guard only ever \
                 fires on a broken fixture"
            );
            assert_eq!(
                reach(card, *index),
                WindowReach::OwnResourcesOnly,
                "{name}[{index}] is a self-contained fetch: it must NOT buy a priority window"
            );
        }
        for (card, index) in interfering {
            let name = card.name;
            // The load-bearing anti-vacuity guard. Every row here must reach
            // MayInterfere through the classifier's own logic on a PARSED
            // effect. Without this, a row whose Oracle text merely failed to
            // parse would sit in the table looking like hostile-detection
            // coverage while actually exercising only the fail-closed arm —
            // and the whole parsed-hostile branch could then be deleted with
            // the suite still green. (MEASURED: `Lightning Bolt` did exactly
            // this in an earlier revision, because it was parsed under a
            // placeholder card name and its self-reference never resolved.)
            assert!(
                !head_is_unparsed(card, *index),
                "VACUITY GUARD: {name}[{index}] rides the Effect::Unimplemented fail-closed \
                 arm, so it is NOT a witness for parsed-hostile detection"
            );
            assert_eq!(
                reach(card, *index),
                WindowReach::MayInterfere,
                "{name}[{index}] can reach past its controller's own resources"
            );
        }
    }

    /// The `SearchLibrary` arm's safety warrant is the ABSORBING fold, not the
    /// arm itself — and the two library rows in the table above do NOT witness
    /// it: `Bribery[0]` and `Praetor's Grasp[0]` each carry
    /// `target_player: Typed{controller: Opponent}`, so the arm catches them
    /// directly and the fold is never asked to do anything. This test pins the
    /// path they miss, on one of the five abilities the arm's own comment names
    /// as foreign-library-with-`target_player: None`.
    ///
    /// MEASURED at this base: `Haunting Echoes[0]` searches "that player's
    /// library" — FOREIGN — yet its `sub_ability` heads
    /// `SearchLibrary { target_player: None }`, so that node ALONE classifies
    /// `OwnResourcesOnly`. What makes the ability interference is its sibling
    /// head, `ChangeZoneAll` (graveyard exile, not an allowlisted effect).
    ///
    /// The two assertions name the fail-open node and its absorber; the
    /// composed verdict is the `HAUNTING_ECHOES` row in the interfering table
    /// above, so no assertion here duplicates one there.
    ///
    /// Read the split correctly: MEASURED, that table row's verdict is carried
    /// ENTIRELY by the `ChangeZoneAll` head — the ability has no cost, no
    /// `player_scope` and no conservative-when-present field, so dropping the
    /// fold's `sub_ability` leg leaves it `MayInterfere` unchanged. The reason is
    /// specific to THIS fixture and does NOT generalize: in
    /// `ability_window_reach` the head is the only term that is not optional, so
    /// when the absorber IS the head, there is no leg the fold could lose that
    /// would carry it away. Do NOT read that as "an absorbed verdict can never
    /// flip" — when the absorbers sit on OPTIONAL legs, the verdict flips as soon
    /// as those legs go. MEASURED at this base by fresh `parse_oracle_text` on
    /// `Jace, Architect of Thought[2]`, another of the five abilities the
    /// `SearchLibrary` arm's own comment names: its head is
    /// `SearchLibrary { target_player: None }` — `OwnResourcesOnly` — and all
    /// THREE of its absorbers are optional legs: `cost: Loyalty(-8)` (the
    /// `cost_window_reach` wildcard), a `sub_ability` heading
    /// `ChangeZone { Library -> Exile, target: Any }`, and `player_scope: All`.
    /// Dropping the `sub_ability` and `cost` legs together still measures
    /// `MayInterfere`, because `player_scope` alone still absorbs; dropping all
    /// three measures `OwnResourcesOnly`. That card is NOT a fixture in this
    /// suite, so nothing here pins it — it is cited to bound the claim above, not
    /// as coverage. What remains lockable on Haunting Echoes is therefore the
    /// premise, and the first assertion is what locks it. MEASURED: closing the
    /// `SearchLibrary` arm (`WindowReach::of(target_player.is_none())` to a bare
    /// `MayInterfere`) flips that assertion. It cannot flip `Bribery[0]` or
    /// `Praetor's Grasp[0]` — DERIVED, not measured: closing the arm can only
    /// move a verdict toward `MayInterfere`, and both are already there.
    #[test]
    fn a_foreign_library_search_is_confined_alone_then_absorbed() {
        let echoes = ability(&HAUNTING_ECHOES, 0);
        let search = echoes
            .sub_ability
            .as_ref()
            .expect("PREMISE: Haunting Echoes[0] carries the library search as its sub_ability");
        assert_eq!(
            effect_window_reach(&search.effect),
            WindowReach::OwnResourcesOnly,
            "PREMISE: the foreign-library search must still look CONFINED on its own — that is \
             the fail-open shape the fold exists to absorb. If this flips, the parser now carries \
             the foreign `target_player` and this row has silently become a second Bribery"
        );
        assert_eq!(
            effect_window_reach(&echoes.effect),
            WindowReach::MayInterfere,
            "the absorber: the sibling graveyard-exile head is what the fold ORs in"
        );
    }

    /// The fetch class's ONE fail-open candidate, pinned with its mechanism
    /// visible instead of hidden among the parsed rows.
    ///
    /// Surveyor's Scope is shaped like acceptance (a) — it searches the actor's
    /// own library for the actor's own basics and shuffles the actor's own
    /// library — but at this base the parser does not carry its whole sentence
    /// ("where X is the number of players who control at least two more lands
    /// than you"), so `abilities[0]` heads an `Effect::Unimplemented`.
    ///
    /// The verdict is JOINTLY determined, and the two assertions below are not
    /// equally load-bearing. MEASURED at this base: the head reaches
    /// `effect_window_reach`'s fail-closed `_` arm, AND the cost is
    /// `Composite[Tap, Exile{filter: SelfRef}]` — `AbilityCost::Exile` is not
    /// allowlisted, so `cost_window_reach` returns `MayInterfere` on its own.
    /// Therefore:
    ///
    /// * the head assertion is the one that carries weight. If it ever flips
    ///   (the parser learns the sentence), this row fails LOUDLY rather than
    ///   silently changing which arm it exercises — the failure is the signal
    ///   to re-verify the verdict on the parsed AST and, if it is then
    ///   confined, move it into the confined table above. Do NOT delete it as
    ///   "redundant with the verdict": that leaves a vacuous row;
    /// * the verdict assertion is DOMINATED by the cost leg — it would still
    ///   hold if the head parsed to a fully confined fetch, so it is NOT
    ///   evidence that the `_` effect arm fired. It is kept because it is the
    ///   class-level statement "the fetch class has no fail-open member".
    ///
    /// Kept OUT of the two tables deliberately: an `Unimplemented` row placed
    /// among the interfering rows would look like hostile-detection coverage
    /// while exercising only the fail-closed arm, which is the exact vacuity
    /// the tables' guards exist to forbid.
    #[test]
    fn surveyors_scope_is_the_fetch_classs_fail_closed_member() {
        assert!(
            head_is_unparsed(&SURVEYORS_SCOPE, 0),
            "PREMISE: this row exists because the head is UNPARSED at this base. If the parser \
             has learned the sentence, re-measure the parsed AST and reclassify the row \
             deliberately — do not delete this assertion to make the suite green"
        );
        assert_eq!(
            reach(&SURVEYORS_SCOPE, 0),
            WindowReach::MayInterfere,
            "the fetch class has no fail-open member: an ability the parser cannot carry must \
             never be proven confined"
        );
    }

    /// V1c's premise, at AST granularity: the ONLY thing separating
    /// `Deathrite Shaman[0]` from `Terramorphic Expanse[0]` is
    /// `filter_is_actor_owned`. Trivializing it merges them — which is exactly
    /// the defect the integration row pins on a real board.
    #[test]
    fn deathrite_and_terramorphic_are_separated_only_by_the_owner_axis() {
        let drs = ability(&DEATHRITE_SHAMAN, 0);
        let Effect::ChangeZone { target, origin, .. } = drs.effect.as_ref() else {
            panic!(
                "Deathrite Shaman[0] must head a ChangeZone, got {:?}",
                drs.effect
            );
        };
        assert_eq!(
            *origin,
            Some(Zone::Graveyard),
            "the graveyard origin is what the refuted origin-keyed rule keyed on"
        );
        assert!(
            !filter_is_actor_owned(target),
            "a bare graveyard-card filter names no owner (CR 400.1), so ownership is UNPROVEN — \
             this is the whole of the B1 fix"
        );

        let terramorphic = ability(&TERRAMORPHIC, 0);
        let Some(AbilityCost::Composite { costs }) = terramorphic.cost.as_ref() else {
            panic!("Terramorphic Expanse[0] must carry a composite cost");
        };
        assert!(
            costs.iter().any(
                |c| matches!(c, AbilityCost::Sacrifice(s) if filter_is_actor_owned(&s.target))
            ),
            "Terramorphic sacrifices ITSELF — proven actor-owned, which is why the fix is \
             surgical rather than a blanket flip to MayInterfere"
        );
    }

    /// V8c — the row that discharges B2. An unrecognized nesting container is
    /// interference regardless of what it nests: every branch below is
    /// individually confined, and the container still folds to `MayInterfere`.
    ///
    /// Revert-probe: changing `effect_window_reach`'s `_` arm to
    /// `OwnResourcesOnly` flips this assertion.
    #[test]
    fn an_unrecognized_container_with_confined_branches_is_still_interference() {
        let confined_branch = ability(&RAMPANT_GROWTH, 0);
        assert_eq!(
            ability_window_reach(&confined_branch),
            WindowReach::OwnResourcesOnly,
            "reach-guard: the branch really is confined, so the container's verdict below is \
             attributable to the container and not to its contents"
        );

        let container = Effect::ChooseOneOf {
            chooser: PlayerFilter::Controller,
            branches: vec![confined_branch.clone(), confined_branch],
        };
        assert_eq!(
            effect_window_reach(&container),
            WindowReach::MayInterfere,
            "an unallowlisted container is interference regardless of what it nests"
        );

        assert_eq!(
            effect_window_reach(&Effect::unimplemented("test", "unparsed fragment")),
            WindowReach::MayInterfere,
            "an unparsed effect is the Surveyor's Scope path — never confined"
        );
    }

    /// An object is not the same thing as its ability list, and proving the
    /// ability list confined proves nothing about the object.
    ///
    /// `game::printed_cards` splits one card face across four collections. This
    /// module can classify exactly one of them. Before the gate in
    /// `object_window_reach`, a card whose ability list was the confined fraction
    /// and whose real content sat in another collection read `OwnResourcesOnly`.
    ///
    /// The witness is real, not constructed: `Stunning Reversal` parses to one
    /// ability — `Exile ~`, i.e. `ChangeZone { destination: Exile, target: SelfRef
    /// }`, actor-owned and off the battlefield, so confined on every conjunct this
    /// module reads — plus one `GameLoss` replacement carrying the whole card. A
    /// seat holding it would have Accepted the shortcut the card exists to
    /// survive, which is the direction §2 calls the one that loses games.
    ///
    /// Discrimination is measured, not asserted, and by construction rather than
    /// by a second fixture: the two objects below differ in exactly one field.
    /// The CONTROL clears `replacement_definitions` and nothing else, and it must
    /// return `OwnResourcesOnly` — so the guarded verdict cannot be a constant,
    /// and it is attributable to that field alone.
    ///
    /// Revert-probe: delete the `carries_unreadable_rules_content` call from
    /// `object_window_reach` and this row's first assertion reds while the control
    /// stays green. Cutting a single disjunct out of that function instead reds only
    /// the row for that disjunct, which is how each one is shown to carry its own
    /// weight rather than riding the first.
    #[test]
    fn an_object_whose_rules_content_this_module_cannot_read_is_never_confined() {
        let parsed = parse_oracle_text(
            STUNNING_REVERSAL.oracle,
            STUNNING_REVERSAL.name,
            &[],
            &[],
            &[],
        );
        assert_eq!(
            parsed.abilities.len(),
            1,
            "PREMISE: the card projects exactly one ability; got {:?}",
            parsed.abilities
        );
        assert_eq!(
            parsed.replacements.len(),
            1,
            "PREMISE: and exactly one replacement — the half this module cannot read; got {:?}",
            parsed.replacements
        );
        assert_eq!(
            ability_window_reach(&parsed.abilities[0]),
            WindowReach::OwnResourcesOnly,
            "PREMISE: the ability half really is confined on its own, so the verdict below is \
             produced by the GATE and not by the ability list"
        );

        // `game::zones::create_object` is the shared primitive for this: it allocates the id,
        // builds the object AND registers it in its zone. Hand-rolling the insert here would
        // duplicate it and, worse, would leave the object absent from the hand's own id list.
        let mut state = GameState::default();
        let id = create_object(
            &mut state,
            crate::types::identifiers::CardId(1),
            crate::types::player::PlayerId(0),
            STUNNING_REVERSAL.name.to_string(),
            Zone::Hand,
        );
        let object = state.objects.get_mut(&id).expect("just created");
        object.abilities = std::sync::Arc::new(parsed.abilities.clone());
        object.replacement_definitions = parsed.replacements.clone().into();

        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::MayInterfere,
            "a GameLoss replacement this module cannot classify is not proof of confinement"
        );
        assert_eq!(
            indexed_ability_window_reach(&state, id, 0),
            WindowReach::MayInterfere,
            "the activation path takes the same gate: a trigger or replacement on the SAME object \
             can fire off the activation (CR 603.2)"
        );

        // CONTROL — one field cleared IN PLACE, nothing else touched.
        state
            .objects
            .get_mut(&id)
            .expect("still present")
            .replacement_definitions = Vec::new().into();
        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::OwnResourcesOnly,
            "REACH-GUARD: with the unreadable collection gone the SAME object is confined, so the \
             two verdicts above are attributable to that one field and are not a constant"
        );

        // The TRIGGER disjunct, exercised through the real materialization path rather than
        // asserted from reading it. The three collections are not wired alike: `printed_cards`
        // assigns `replacement_definitions` and `static_definitions` directly, but routes triggers
        // through `base_trigger_definitions` + `materialize_base_trigger_definitions()`. A gate that
        // reads the materialized field while the pipeline only ever fills the printed one would be
        // silently blind to every trigger, so the wiring is the thing under test here, not the
        // `is_empty()` call.
        let triggers = parse_oracle_text(
            SOUL_GUIDE_LANTERN.oracle,
            SOUL_GUIDE_LANTERN.name,
            &[],
            &[],
            &[],
        )
        .triggers;
        assert_eq!(
            triggers.len(),
            1,
            "PREMISE: Soul-Guide Lantern's ETB is a real parsed trigger; got {triggers:?}"
        );
        let object = state.objects.get_mut(&id).expect("still present");
        object.base_trigger_definitions = std::sync::Arc::new(triggers);
        object.materialize_base_trigger_definitions();
        assert!(
            !object.trigger_definitions.is_empty(),
            "PREMISE: materializing the printed triggers must populate the field the gate reads — \
             if this ever fails the gate is blind to triggers no matter what it returns"
        );
        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::MayInterfere,
            "a trigger this module cannot classify is not proof of confinement either"
        );

        // The STATIC disjunct — the third of three, and the one a review round
        // correctly noted had neither a test nor a probe. 7 of the 19 printed cards
        // this gate flips at today's pool are static-only carriers, so an untested
        // disjunct here would leave better than a THIRD of the gate's own measured
        // effect unexercised.
        let object = state.objects.get_mut(&id).expect("still present");
        object.base_trigger_definitions = std::sync::Arc::new(Vec::new());
        object.materialize_base_trigger_definitions();
        assert!(
            object.trigger_definitions.is_empty(),
            "PREMISE: triggers cleared, so the next verdict cannot be the trigger clause again"
        );
        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::OwnResourcesOnly,
            "CONTROL: with every unreadable collection empty the SAME object is confined again"
        );
        let parsed_statics =
            parse_oracle_text(SPREADING_SEAS.oracle, SPREADING_SEAS.name, &[], &[], &[]).statics;
        assert!(
            !parsed_statics.is_empty(),
            "PREMISE: Spreading Seas carries a real parsed static ability; got {parsed_statics:?}"
        );
        state
            .objects
            .get_mut(&id)
            .expect("still present")
            .static_definitions = parsed_statics.into();
        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::MayInterfere,
            "nor is a static ability this module cannot classify proof of confinement"
        );

        // The KEYWORD disjunct. This is the one a review round caught: the gate used to
        // read three collections while `printed_cards` writes far more, and printed
        // Cascade lives in `obj.keywords` and never reaches `trigger_definitions` — so a
        // Cascade spell whose printed abilities all read confined was provably
        // `OwnResourcesOnly` while resolving it casts a free spell of arbitrary reach.
        let object = state.objects.get_mut(&id).expect("still present");
        object.static_definitions = Vec::new().into();
        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::OwnResourcesOnly,
            "CONTROL: statics cleared, so the keyword verdict below is the keyword's alone"
        );
        state
            .objects
            .get_mut(&id)
            .expect("still present")
            .keywords
            .push(Keyword::Cascade);
        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::MayInterfere,
            "a keyword is rules text this fold never reads; Cascade resolves a free spell of \
             arbitrary reach inside the window the seat would have declined to keep"
        );
        assert_eq!(
            indexed_ability_window_reach(&state, id, 0),
            WindowReach::MayInterfere,
            "and the activation path takes the same widened gate, not the old three-field one"
        );

        // A CASTING-TIME MODIFIER, to show the widening is not keyword-specific.
        let object = state.objects.get_mut(&id).expect("still present");
        object.keywords.clear();
        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::OwnResourcesOnly,
            "CONTROL: keyword cleared and the SAME object is confined again, so neither verdict \
             above is a constant"
        );
        state
            .objects
            .get_mut(&id)
            .expect("still present")
            .spellbook
            .push("Lightning Bolt".to_string());
        assert_eq!(
            object_window_reach(&state, id),
            WindowReach::MayInterfere,
            "a spellbook names cards this object can reach for, and it is not an \
             AbilityDefinition either — the gate is over unreadable CONTENT, not over one field"
        );

        // The four fields the STALENESS GUARD surfaced. They are inert at today's pool
        // (0 carriers among the cards that survive the gate), which is exactly why they
        // need a witness here: an inert disjunct with no test is indistinguishable from a
        // disjunct that does nothing, and the next person to tidy this function would have
        // no way to tell. Each is set on the SAME object with every other unreadable field
        // cleared first, so each verdict is attributable to that one field.
        // Named rather than written inline: `clippy::type_complexity` rejects the tuple-of-fn-ptr
        // array form, and the alias is the lint's own suggested remedy.
        type FieldSetter = fn(&mut GameObject);
        let setters: [(&str, FieldSetter); 4] = [
            ("case_state", |o| {
                o.case_state = Some(crate::game::game_object::CaseState {
                    is_solved: false,
                    solve_condition: crate::types::ability::SolveCondition::Text {
                        description: "probe".to_string(),
                    },
                })
            }),
            ("class_level", |o| o.class_level = Some(1)),
            ("intensity", |o| o.intensity = 1),
            ("attraction_lights", |o| o.attraction_lights = vec![1]),
        ];
        for (field, set) in setters {
            let object = state.objects.get_mut(&id).expect("still present");
            object.spellbook.clear();
            object.case_state = None;
            object.class_level = None;
            object.intensity = 0;
            object.attraction_lights.clear();
            assert_eq!(
                object_window_reach(&state, id),
                WindowReach::OwnResourcesOnly,
                "CONTROL before {field}: with every unreadable field cleared the SAME object \
                 is confined again, so the verdict below is attributable to {field} alone"
            );
            set(state.objects.get_mut(&id).expect("still present"));
            assert_eq!(
                object_window_reach(&state, id),
                WindowReach::MayInterfere,
                "{field} is rules content this fold cannot read, so it is not proof of confinement"
            );
        }
    }

    /// STALENESS GUARD. Every printed rules field is either folded or gated.
    ///
    /// The defect this exists to prevent has now happened twice on this one gate: it
    /// shipped reading three collections while `printed_cards` wrote eleven fields, and a
    /// review round found `keywords` (printed Cascade) that way. Writing THIS test then
    /// found four more — `case_state`, `class_level`, `intensity`, `attraction_lights` —
    /// which no amount of re-reading the gate by hand had surfaced. An enumerated list is
    /// only ever correct on the day it is written; a `..`-free destructure is correct
    /// until the compiler says otherwise.
    ///
    /// **Why `CardFace` and not `GameObject`.** `GameObject` has 149 fields, most of them
    /// runtime state (zone, damage, counters, attachments) with no bearing on what a card
    /// can do. Destructuring it here would be a churn magnet that every unrelated field
    /// addition breaks, and it would be blanket-`..`'d back within a round. `CardFace` has
    /// 33 and is the actual source `printed_cards` reads to populate object rules content,
    /// so it guards the defect class that occurred rather than the largest surface
    /// available.
    ///
    /// **Honest scope limit:** this guards fields that reach an object THROUGH
    /// `printed_cards`. A `GameObject` field written by some other path is not covered —
    /// `game::stickers` is the live example, and it writes only the three definition
    /// collections, which are gated.
    #[test]
    fn every_printed_rules_field_is_either_folded_or_gated() {
        // `..`-free ON PURPOSE. A new `CardFace` field is a COMPILE ERROR here until
        // someone sorts it into one of the three buckets. The sort IS the assertion:
        // there is nothing to run, and that is the point — this fires at build time,
        // when it can still be cheap, rather than at review time.
        let CardFace {
            // ---- FOLDED: this module classifies these itself.
            abilities: _,

            // ---- GATED: unreadable rules content. `carries_unreadable_rules_content`
            // returns true on the corresponding `GameObject` field.
            keywords: _,
            triggers: _,
            static_abilities: _,
            replacements: _,
            cleave_variant: _,
            modal: _,
            additional_cost: _,
            casting_restrictions: _,
            casting_options: _,
            strive_cost: _,
            solve_condition: _, // lands as `obj.case_state`
            attraction_lights: _,
            // `metadata` is MIXED, so it is DESTRUCTURED rather than bound whole. Binding
            // it was a hole in the guard that the guard's own comment declared and did not
            // close: `spellbook` is exactly a rules-bearing field that arrived inside this
            // struct, so "mixed" is the reason to open it, not the reason to wave it past.
            metadata:
                CardMetadata {
                    // GATED as `obj.spellbook`.
                    spellbook: _,
                    // Parser-provenance counters: how many abilities came from Forge
                    // scripts rather than the Oracle parser. The abilities themselves are
                    // in the collections above; these are counts of them.
                    forge_abilities: _,
                    forge_triggers: _,
                    forge_statics: _,
                    forge_replacements: _,
                    // Names tokens this card can make; MAKING one runs through
                    // `Effect::Token` in `abilities`, which this fold already reads.
                    related_token_ids: _,
                    // Image/catalog identifiers.
                    source_printing_ids: _,
                    // CR 202.3d + CR 709.4b: a split card's combined off-stack mana value,
                    // read by deck-construction checks. A cost, not what resolving does.
                    off_stack_mana_value_override: _,
                },

            // ---- NOT RULES-BEARING. One reason each, because an unjustified entry here
            // is exactly where the next lazy re-bucket lands.
            name: _,      // identity, not behaviour
            mana_cost: _, // cost to cast, not what resolving does
            // NOT inert, and the earlier reason here ("types/subtypes gate other rules,
            // carry none alone") was measurably WRONG. `printed_cards` derives four
            // object-level rules fields from `subtypes` ALONE, with no CardFace field
            // behind them: `Class` => `class_level` (:206), `Case` => `case_state` (:238),
            // `Room` => `room_unlocks` (:246), `Attraction` => `attraction_lights` (:250).
            // The face's own field is therefore safe to skip only BECAUSE all four
            // derived fields are gated — and `room_unlocks` was not, until the review
            // round that read this line. A wrong reason in this bucket is worse than no
            // reason: it certifies that no gate is needed.
            card_type: _,
            power: _,              // combat statistic
            toughness: _,          // combat statistic
            loyalty: _,            // resource counter, abilities that spend it are in `abilities`
            defense: _,            // battle counter, same argument as loyalty
            oracle_text: _,        // the SOURCE the parser reads; the parse is the rules content
            non_ability_text: _,   // by definition not an ability
            flavor_name: _,        // cosmetic
            color_override: _,     // colour is a characteristic, not an action
            color_identity: _,     // deck construction (CR 903.4), not in-game behaviour
            scryfall_oracle_id: _, // external identifier
            brawl_commander: _,    // format eligibility
            is_commander: _,       // format eligibility
            is_oathbreaker: _,     // format eligibility
            deck_copy_limit: _,    // deck construction
            parse_warnings: _,     // parser diagnostics, never consulted at runtime
            rarities: _,           // printing metadata
        } = CardFace::default();
    }

    /// Every GATED bucket entry above must actually REACH the gate, through the pipeline
    /// that really populates it.
    ///
    /// This exists because a review round measured that seven of the gate's disjuncts had
    /// no test and no revert-probe — `modal`, `additional_cost`, `strive_cost`,
    /// `cleave_variant`, `casting_restrictions`, `casting_options`, `back_face` — each
    /// occurring exactly once in the whole file, in the gate itself. Deleting any of them
    /// left the entire suite green. The sting: `modal` and `additional_cost` are 2 of the
    /// 10 cards the widening actually flips, so the disjuncts with no coverage were the
    /// LIVE ones while the four that had witnesses were the inert ones. The guard above is
    /// a compile-time claim that each field is classified; this is the runtime claim that
    /// the classification is TRUE.
    ///
    /// It also turns the guard's `CardFace` → `GameObject` mapping from a comment into an
    /// assertion. Each case mutates a **`CardFace`** and runs the real
    /// `apply_card_face_to_object`, so a rename or a dropped copy in `printed_cards` reds
    /// here instead of silently un-gating the field — which is the failure the `spellbook`
    /// and `keywords` holes both took to get in.
    #[test]
    fn every_gated_card_face_field_reaches_the_gate_through_printed_cards() {
        use crate::game::printed_cards::apply_card_face_to_object;

        // `clippy::type_complexity` rejects the inline tuple-of-fn-ptr array; the alias is
        // the lint's own suggested remedy.
        type FaceSetter = fn(&mut CardFace);
        let cases: [(&str, FaceSetter); 12] = [
            // --- carried on the face itself.
            ("keywords", |f| f.keywords = vec![Keyword::Cascade]),
            ("cleave_variant", |f| {
                f.cleave_variant = Some(crate::types::card::CleaveVariant::default())
            }),
            ("modal", |f| {
                f.modal = Some(crate::types::ability::ModalChoice::default())
            }),
            ("additional_cost", |f| {
                f.additional_cost = Some(crate::types::ability::AdditionalCost::Kicker {
                    costs: vec![],
                    repeatability: crate::types::ability::AdditionalCostRepeatability::Once,
                })
            }),
            ("casting_restrictions", |f| {
                f.casting_restrictions = vec![crate::types::ability::CastingRestriction::AsSorcery]
            }),
            ("casting_options", |f| {
                f.casting_options = vec![crate::types::ability::SpellCastingOption::free_cast()]
            }),
            ("strive_cost", |f| {
                f.strive_cost = Some(crate::types::mana::ManaCost::default())
            }),
            ("metadata.spellbook", |f| {
                f.metadata.spellbook = vec!["Lightning Bolt".to_string()]
            }),
            // --- DERIVED FROM `card_type.subtypes` ALONE, with no face field behind them.
            // These four are why the guard's `card_type` bucket reason had to be rewritten:
            // the face's own `card_type` is safe to skip only because all four landing
            // fields are gated, and `room_unlocks` was not until this round.
            ("card_type: Class → class_level", |f| {
                f.card_type.subtypes = vec!["Class".to_string()]
            }),
            ("card_type: Case → case_state", |f| {
                f.card_type.subtypes = vec!["Case".to_string()];
                f.solve_condition = Some(crate::types::ability::SolveCondition::Text {
                    description: "probe".to_string(),
                });
            }),
            ("card_type: Room → room_unlocks", |f| {
                f.card_type.subtypes = vec!["Room".to_string()]
            }),
            ("card_type: Attraction → attraction_lights", |f| {
                f.card_type.subtypes = vec!["Attraction".to_string()]
            }),
        ];

        for (field, set) in cases {
            // CONTROL and WITNESS are SEPARATE freshly-created objects rather than one
            // object applied twice. `printed_cards` seeds `class_level` only when
            // `base_characteristics_initialized` is still false (CR 716.2b), so re-applying
            // to the same object would silently skip the very field the Class case tests —
            // and the case would pass for the wrong reason on the three cases beside it.
            let mut face = CardFace {
                name: "Probe".to_string(),
                ..CardFace::default()
            };
            let mut state = GameState::default();

            let control = create_object(
                &mut state,
                crate::types::identifiers::CardId(1),
                crate::types::player::PlayerId(0),
                face.name.clone(),
                Zone::Hand,
            );
            apply_card_face_to_object(
                state.objects.get_mut(&control).expect("just created"),
                &face,
            );
            assert!(
                !carries_unreadable_rules_content(
                    state.objects.get(&control).expect("just created")
                ),
                "CONTROL for {field}: a default face lands no unreadable content, so the \
                 verdict below is produced by {field} and not by the face it rides on"
            );

            set(&mut face);
            let witness = create_object(
                &mut state,
                crate::types::identifiers::CardId(2),
                crate::types::player::PlayerId(0),
                face.name.clone(),
                Zone::Hand,
            );
            apply_card_face_to_object(
                state.objects.get_mut(&witness).expect("just created"),
                &face,
            );
            assert!(
                carries_unreadable_rules_content(
                    state.objects.get(&witness).expect("just created")
                ),
                "CardFace {field} is rules content this fold cannot read, so `printed_cards` \
                 must land it somewhere the gate looks — either the gate lost a disjunct or \
                 the copy into `GameObject` was renamed out from under it"
            );
        }

        // `back_face` is the one gated field with no `apply_card_face_to_object` route:
        // `printed_cards::apply_card_face_to_back_face` fills a `BackFaceData` on the
        // transform path instead. Asserted directly, and reusing `game::specialize`'s
        // existing empty constructor rather than hand-rolling a 22-field literal that would
        // go stale the moment `BackFaceData` gains a field.
        let mut state = GameState::default();
        let id = create_object(
            &mut state,
            crate::types::identifiers::CardId(3),
            crate::types::player::PlayerId(0),
            "Probe".to_string(),
            Zone::Hand,
        );
        assert!(
            !carries_unreadable_rules_content(state.objects.get(&id).expect("just created")),
            "CONTROL for back_face: a bare object carries nothing the gate reads"
        );
        state.objects.get_mut(&id).expect("just created").back_face =
            Some(crate::game::specialize::empty_back_face());
        assert!(
            carries_unreadable_rules_content(state.objects.get(&id).expect("just created")),
            "a back face is an entire second face of rules content this fold never descends \
             into, so its presence is not proof of confinement"
        );
    }

    /// The destination axis, on a REAL parsed node, one field mutated.
    ///
    /// `Stunning Reversal`'s ability is `ChangeZone { destination: Exile, target:
    /// SelfRef }`, so `object_is_confined` holds by ownership alone and does not
    /// depend on `destination` — which makes `destination` the only thing the
    /// verdicts below can be measuring. (The anaphoric fetch node cannot serve
    /// here: its `object_is_confined` disjunct itself requires
    /// `destination == Battlefield`, so mutating the field would move two things
    /// at once and the row would prove nothing.)
    ///
    /// `Hand` is gated for the same reason a `Battlefield`-untapped arrival is: a
    /// card put into hand is a CASTABLE card. The spell that put it there
    /// resolves, the active player receives priority (CR 117.3b) and priority then
    /// passes in turn order (CR 117.3d), so the responding seat gets it back still
    /// inside this window and can cast it. Graveyard, library and exile are not gated
    /// because the actor cannot cast or activate from them without some further
    /// permission — which would itself be an ability this fold already reads.
    ///
    /// Both directions are present, so neither verdict can be a constant.
    #[test]
    fn a_destination_is_confined_only_when_the_seat_cannot_act_on_what_lands_there() {
        let parsed = parse_oracle_text(
            STUNNING_REVERSAL.oracle,
            STUNNING_REVERSAL.name,
            &[],
            &[],
            &[],
        );
        let node = parsed.abilities[0].effect.as_ref().clone();
        let Effect::ChangeZone { target, .. } = &node else {
            panic!("PREMISE: the node must head a ChangeZone; got {node:?}");
        };
        assert!(
            filter_is_actor_owned(target),
            "PREMISE: the target is actor-owned, so `object_is_confined` holds independently of \
             `destination` and every verdict below is attributable to the destination alone"
        );

        // Every one of `Zone`'s seven variants (CR 400.1) appears here. The previous
        // table listed five and the production match closed the gap with `_ => true`,
        // so `Zone::Stack` was classified confined by a wildcard and no row noticed.
        let table = [
            (Zone::Exile, WindowReach::OwnResourcesOnly),
            (Zone::Graveyard, WindowReach::OwnResourcesOnly),
            (Zone::Library, WindowReach::OwnResourcesOnly),
            (Zone::Hand, WindowReach::MayInterfere),
            // Already past casting (CR 405.1) and resolves inside the window
            // (CR 608.1) — strictly stronger reach than the hand row above.
            (Zone::Stack, WindowReach::MayInterfere),
            // CR 903.8: a commander may be cast from here.
            (Zone::Command, WindowReach::MayInterfere),
            // `enter_tapped` is `Unspecified` on this node, so the battlefield
            // arm is reach for the TAP reason, not the destination reason.
            (Zone::Battlefield, WindowReach::MayInterfere),
        ];

        // COMPLETENESS GUARD, compile-time half: this match is exhaustive over `Zone`,
        // so adding a variant breaks THIS test's build and forces a row decision here
        // as well as in the production match. The runtime half below then catches a
        // variant that compiles but was left out of `table`. Deliberately NOT a mirror
        // of the production match — it asserts coverage only, never a verdict, so it
        // cannot pass by agreeing with a wrong implementation.
        for zone in [
            Zone::Library,
            Zone::Hand,
            Zone::Battlefield,
            Zone::Graveyard,
            Zone::Stack,
            Zone::Exile,
            Zone::Command,
        ] {
            match zone {
                Zone::Library
                | Zone::Hand
                | Zone::Battlefield
                | Zone::Graveyard
                | Zone::Stack
                | Zone::Exile
                | Zone::Command => {}
            }
            assert!(
                table.iter().any(|(z, _)| *z == zone),
                "Zone::{zone:?} has no row in the destination table — every zone must be \
                 classified explicitly, because the failure this test exists for is a \
                 destination nobody wrote a row for"
            );
        }

        for (zone, want) in table {
            let mut mutated = node.clone();
            let Effect::ChangeZone { destination, .. } = &mut mutated else {
                unreachable!("just matched above")
            };
            *destination = zone;
            assert_eq!(
                effect_window_reach(&mutated),
                want,
                "destination {zone:?} must classify {want:?}"
            );
        }
    }

    /// V8's hostile edges on the action fold: nothing resolvable, nothing to
    /// resolve, and an index past the end are all interference.
    #[test]
    fn unresolvable_action_subjects_are_interference() {
        let state = GameState::default();
        let missing = ObjectId(9_999_999);
        assert_eq!(
            object_window_reach(&state, missing),
            WindowReach::MayInterfere,
            "an object that is not on the board cannot be proven confined"
        );
        assert_eq!(
            indexed_ability_window_reach(&state, missing, 0),
            WindowReach::MayInterfere,
            "neither can an ability index into an object that is not on the board"
        );
        assert!(
            any_action_may_interfere(
                &state,
                &[GameAction::ActivateAbility {
                    source_id: missing,
                    ability_index: 7,
                }]
            ),
            "an out-of-range ability index is interference, not a confined no-op"
        );
        assert!(
            !any_action_may_interfere(&state, &[GameAction::PassPriority]),
            "reach-guard: the fold CAN return false, so the trues above are attributable"
        );
    }

    /// The `Or`/`And` legs are set logic, not a coin flip, and an empty leg set
    /// is never proven.
    #[test]
    fn composite_filters_prove_ownership_by_set_logic() {
        let owned = TargetFilter::Controller;
        let unowned = TargetFilter::Any;

        assert!(
            !filter_is_actor_owned(&TargetFilter::Or {
                filters: vec![owned.clone(), unowned.clone()],
            }),
            "an Or is proven only when EVERY leg is proven"
        );
        assert!(
            filter_is_actor_owned(&TargetFilter::Or {
                filters: vec![owned.clone(), owned.clone()],
            }),
            "reach-guard: an all-proven Or IS proven, so the negative above is not vacuous"
        );
        assert!(
            filter_is_actor_owned(&TargetFilter::And {
                filters: vec![owned, unowned],
            }),
            "an And narrows, so one proven leg suffices"
        );
        assert!(
            !filter_is_actor_owned(&TargetFilter::Or { filters: vec![] }),
            "a degenerate empty Or must not be proven by a vacuous all()"
        );
    }

    /// Mana is FUNGIBLE REACH (CR 106.1 / CR 106.4 / CR 601.2g), so neither a
    /// cast ritual nor an actor-owned sacrifice-for-mana is confined.
    ///
    /// The Lotus Petal half is the load-bearing one, and its ATTRIBUTION is the
    /// second assertion: the parser emits "Sacrifice this artifact" as a
    /// `TargetFilter::SelfRef`, which `filter_is_actor_owned` returns true for,
    /// so `cost_window_reach` returns `OwnResourcesOnly` and the ONLY thing that
    /// can carry the verdict is the mana effect.
    ///
    /// What that predicate does and does NOT establish — CR 701.21a, quoted from
    /// its FIRST sentence: "To sacrifice a permanent, its controller moves it
    /// from the battlefield directly to its owner's graveyard. A player can't
    /// sacrifice something that isn't a permanent, or something that's a
    /// permanent they don't control." Sentence two bounds the actor to
    /// permanents they CONTROL — that much is grounded. Sentence one is the half
    /// that bounds the conclusion: the card goes to its OWNER's graveyard, so a
    /// controlled-but-not-owned Petal (Control Magic) puts a card into another
    /// player's graveyard while this leg still answers `OwnResourcesOnly`.
    /// `SelfRef` therefore proves control, never ownership, and "confined" is
    /// narrower than the predicate's name reads.
    ///
    /// Not repairable at this seam: `filter_is_actor_owned` is a pure AST
    /// predicate and the AST carries no ownership. Nor does it move this row,
    /// whose verdict the mana head decides on its own. It is named so the limit
    /// is not later widened on the strength of a half-quoted rule — which is the
    /// exact species of error (CR 106.4, above) this whole change repairs.
    ///
    /// A confined cost leg plus a verdict carried by the mana head ALONE is what
    /// separates this row from `v9b`'s Ironworks, which reaches `MayInterfere`
    /// through an UNPROVEN sacrifice filter and therefore never exercises this
    /// arm.
    ///
    /// Revert-probe, EXECUTED: restoring
    /// `Effect::Mana {..} => WindowReach::OwnResourcesOnly` as this match's first
    /// arm reds this row at the Dark Ritual verdict, which is where the run
    /// aborts. The Lotus Petal verdict flips under the same mutation and that is
    /// MEASURED rather than derived — the integration row `v10b` reads exactly
    /// this ability through `indexed_ability_window_reach` and flips to `Accept`
    /// under the same mutation, which it can only do if this fold returned
    /// `OwnResourcesOnly`.
    #[test]
    fn mana_production_is_reach_not_a_confined_own_resource() {
        for (card, index) in [(&DARK_RITUAL, 0usize), (&LOTUS_PETAL, 0usize)] {
            assert!(
                !head_is_unparsed(card, index),
                "VACUITY GUARD: {} must PARSE, or the row measures the fail-closed arm instead \
                 of the mana effect's own semantics",
                card.name
            );
        }

        // PREMISE (card-data is gitignored; re-measured here on the live parser).
        let ritual = ability(&DARK_RITUAL, 0);
        assert!(
            matches!(ritual.effect.as_ref(), Effect::Mana { .. }),
            "PREMISE: Dark Ritual heads Effect::Mana; got {:?}",
            ritual.effect
        );
        assert!(
            ritual.cost.is_none(),
            "PREMISE: Dark Ritual carries no cost, so NOTHING but the head can carry the verdict"
        );
        assert_eq!(
            reach(&DARK_RITUAL, 0),
            WindowReach::MayInterfere,
            "a cast ritual funds an otherwise-unaffordable response"
        );

        let petal = ability(&LOTUS_PETAL, 0);
        let cost = petal
            .cost
            .as_ref()
            .expect("PREMISE: Lotus Petal carries an activation cost");
        assert_eq!(
            cost_window_reach(cost),
            WindowReach::OwnResourcesOnly,
            "ATTRIBUTION: the SelfRef sacrifice is proven CONTROLLED (CR 701.21a bounds the actor \
             to permanents they control; the card still leaves for its OWNER's graveyard — see \
             the doc comment), so the cost leg reads confined and cannot be what produces the \
             verdict below"
        );
        assert_eq!(
            ability_window_reach(&petal),
            WindowReach::MayInterfere,
            "…so the verdict is carried by the mana effect ALONE — this is the row v9b cannot be"
        );

        // Reach-guard: the classifier can still return OwnResourcesOnly, so the
        // two MayInterfere verdicts above are attributable and not a constant.
        assert_eq!(reach(&RAMPANT_GROWTH, 0), WindowReach::OwnResourcesOnly);
    }

    /// The tap-state gate (CR 110.5b), on the MINIMAL PAIR. `Rampant Growth` and
    /// `Nature's Lore` differ by one printed word, and every other axis this
    /// classifier reads is identical: no cost, a `SearchLibrary` head with
    /// `target_player: None`, and one anaphoric
    /// `ChangeZone { Library -> Battlefield, target: Any }` sub-ability. So the
    /// verdicts below are attributable to `enter_tapped` and to nothing else —
    /// the first two assertions MEASURE that premise rather than assert it.
    ///
    /// Why the untapped one is reach: the fetched land arrives ready (CR 302.6's
    /// summoning-sickness bar is a creature rule), taps for mana inside the
    /// window the Shorten hands back, and CR 601.2g runs that mana ability during
    /// the cast it funds. `v10c` in `tests/integration/shorten_efficacy.rs`
    /// measures that whole chain on the real 4p board; this row pins the AST
    /// premise it rests on.
    ///
    /// Revert-probe: in `effect_window_reach`'s `ChangeZone` arm, replace
    /// `object_is_confined && entry_is_confined` with `object_is_confined` alone
    /// (the pre-fix expression) — BOTH untapped rows flip to `OwnResourcesOnly`.
    #[test]
    fn an_untapped_fetch_is_reach_and_a_tapped_one_stays_confined() {
        for card in [&NATURES_LORE, &CROP_ROTATION] {
            let name = card.name;
            assert!(
                !head_is_unparsed(card, 0),
                "VACUITY GUARD: {name}[0] must PARSE, or the row measures the fail-closed arm \
                 instead of the zone-change node's own semantics"
            );
            let def = ability(card, 0);
            assert!(
                def.cost.is_none(),
                "PREMISE: {name}[0] carries no cost, so no cost leg can carry the verdict; got \
                 {:?}",
                def.cost
            );
            assert_eq!(
                effect_window_reach(&def.effect),
                WindowReach::OwnResourcesOnly,
                "PREMISE: {name}[0]'s SearchLibrary head is confined ON ITS OWN, so the verdict \
                 below is produced by the zone-change sub-ability and by nothing else"
            );
            assert_eq!(
                reach(card, 0),
                WindowReach::MayInterfere,
                "{name}[0] puts a land onto the battlefield UNTAPPED (CR 110.5b), which taps for \
                 mana inside the window the Shorten hands back — the Effect::Mana case with one \
                 extra step"
            );
        }

        // The tapped half of the pair, and the reach-guard that keeps the two
        // MayInterfere verdicts above from being a constant.
        assert_eq!(
            reach(&RAMPANT_GROWTH, 0),
            WindowReach::OwnResourcesOnly,
            "the gate is on the TAP STATE, not on the fetch shape: the same sentence with \
             'tapped' printed in it stays confined"
        );

        // Fail-closed direction, on all THREE `EtbTapState` variants, measured on
        // the REAL parsed node rather than a hand-built one. Only a provably
        // `Tapped` entry is allowlisted; `Unspecified` (the AST said nothing) and
        // `Untapped` are both reach — which is what makes a conditionally-untapped
        // entry (a shock land's pay-life choice, a land-count gate) safe by
        // default instead of a shape this predicate would have to model.
        let mut node = ability(&NATURES_LORE, 0)
            .sub_ability
            .as_ref()
            .expect("PREMISE: Nature's Lore[0] carries the zone change as its sub_ability")
            .effect
            .as_ref()
            .clone();
        for (state, expected) in [
            (EtbTapState::Tapped, WindowReach::OwnResourcesOnly),
            (EtbTapState::Unspecified, WindowReach::MayInterfere),
            (EtbTapState::Untapped, WindowReach::MayInterfere),
        ] {
            let Effect::ChangeZone { enter_tapped, .. } = &mut node else {
                panic!("PREMISE: the sub-ability must head a ChangeZone; got {node:?}");
            };
            *enter_tapped = state;
            assert_eq!(
                effect_window_reach(&node),
                expected,
                "fail-closed on the tap axis: {state:?} must classify {expected:?}"
            );
        }
    }

    /// PROVABLY tapped, not NOMINALLY tapped — the three riders that decide what
    /// actually arrives, plus the split that reaches the battlefield without a
    /// `ChangeZone` node at all.
    ///
    /// Every row mutates ONE field of a REAL parsed node and re-measures, so the
    /// unmutated verdict is each row's own positive control: the base node is
    /// asserted `OwnResourcesOnly` first, which is what makes each flip
    /// attributable to the field and not to the fixture.
    ///
    /// The corpus is why these are fixtures rather than hypotheticals: MEASURED
    /// over every card in the pinned MTGJSON projection, 12 abilities carry a
    /// `SearchDestinationSplit`, and NO ability that classifies
    /// `OwnResourcesOnly` carries `enters_under`, `enters_attacking` or
    /// `enters_modified_if` today. So these guards change no card's verdict at
    /// this base — they close the door before a parser improvement or a new card
    /// walks through it, in the one direction (toward `MayInterfere`) that can
    /// never produce a false Accept.
    #[test]
    fn a_battlefield_entry_must_be_provably_tapped_actor_controlled_and_not_attacking() {
        // ── the three ChangeZone riders, on Terramorphic's real fetch node ──
        let base_node = ability(&TERRAMORPHIC, 0)
            .sub_ability
            .as_ref()
            .expect("PREMISE: Terramorphic Expanse[0] carries the zone change as its sub_ability")
            .effect
            .as_ref()
            .clone();
        assert_eq!(
            effect_window_reach(&base_node),
            WindowReach::OwnResourcesOnly,
            "POSITIVE CONTROL: the unmutated tapped fetch node IS confined, so every flip below is \
             attributable to the single field it mutates"
        );

        let mut attacking = base_node.clone();
        let mut foreign = base_node.clone();
        let mut conditional = base_node.clone();
        let Effect::ChangeZone {
            enters_attacking, ..
        } = &mut attacking
        else {
            panic!("PREMISE: the fetch node must head a ChangeZone");
        };
        *enters_attacking = true;
        let Effect::ChangeZone { enters_under, .. } = &mut foreign else {
            panic!("PREMISE: the fetch node must head a ChangeZone");
        };
        *enters_under = Some(ControllerRef::Opponent);
        let Effect::ChangeZone {
            enters_modified_if, ..
        } = &mut conditional
        else {
            panic!("PREMISE: the fetch node must head a ChangeZone");
        };
        *enters_modified_if = Some(TargetFilter::Any);

        for (label, node) in [
            (
                "enters_attacking (CR 508.4): a tapped attacker is still an attacker",
                &attacking,
            ),
            (
                "enters_under (CR 110.2a): the permanent lands on ANOTHER player's board",
                &foreign,
            ),
            (
                "enters_modified_if (CR 614.12 + CR 614.12a): the tapped rider is CONDITIONAL, so \
                 enter_tapped == Tapped is not proof of a tapped entry",
                &conditional,
            ),
        ] {
            assert_eq!(
                effect_window_reach(node),
                WindowReach::MayInterfere,
                "{label}"
            );
        }

        // ── the split door, on Cultivate's real search node ──
        assert!(
            !head_is_unparsed(&CULTIVATE, 0),
            "VACUITY GUARD: Cultivate[0] must PARSE, or this half measures the fail-closed arm"
        );
        let mut search = ability(&CULTIVATE, 0).effect.as_ref().clone();
        let Effect::SearchLibrary { split, .. } = &search else {
            panic!("PREMISE: Cultivate[0] must head a SearchLibrary; got {search:?}");
        };
        let split = split.as_ref().expect(
            "PREMISE: Cultivate[0] carries a SearchDestinationSplit — this is the shape \
                     that reaches the battlefield with no ChangeZone node for the tap gate to read",
        );
        assert_eq!(
            split.primary_destination,
            Zone::Battlefield,
            "PREMISE: the split's PRIMARY destination is the battlefield"
        );
        assert_eq!(
            split.primary_enter_tapped,
            EtbTapState::Tapped,
            "PREMISE: and the real card prints 'tapped', which is the only reason it stays confined"
        );
        assert_eq!(
            split.rest_destination,
            Zone::Hand,
            "PREMISE: and the REST of the search goes to hand — the door this arm used to leave open"
        );
        assert_eq!(
            effect_window_reach(&search),
            WindowReach::MayInterfere,
            "the split arm takes the SAME landing-zone authority as the ChangeZone arm: a rest \
             destination of hand is a castable card inside the window, so Cultivate is NOT confined \
             however tapped its primary arrival is"
        );

        // From here the rest destination is moved OFF hand, which is what makes a
        // confined split expressible at all: MEASURED on this candidate's own
        // projection, all TWELVE `SearchDestinationSplit` carriers route something to
        // hand (nine via `rest_destination`, three via `primary_destination`), so no
        // real card can serve as the positive control for the tap axis. Single-field
        // mutation off a real parsed node is the honest way to get one, and saying so
        // is the point — an inert branch presented as covered is the error this row
        // exists to avoid.
        let Effect::SearchLibrary {
            split: Some(split), ..
        } = &mut search
        else {
            panic!("PREMISE: the mutated node must keep its split");
        };
        split.rest_destination = Zone::Graveyard;
        assert_eq!(
            effect_window_reach(&search),
            WindowReach::OwnResourcesOnly,
            "CONTROL: one field moved and nothing else, so the verdict above is attributable to \
             `rest_destination` alone and is not a constant"
        );

        for (state, expected) in [
            (EtbTapState::Tapped, WindowReach::OwnResourcesOnly),
            (EtbTapState::Unspecified, WindowReach::MayInterfere),
            (EtbTapState::Untapped, WindowReach::MayInterfere),
        ] {
            let Effect::SearchLibrary {
                split: Some(split), ..
            } = &mut search
            else {
                panic!("PREMISE: the mutated node must keep its split");
            };
            split.primary_enter_tapped = state;
            assert_eq!(
                effect_window_reach(&search),
                expected,
                "the split reaches the battlefield WITHOUT a ChangeZone node, so it takes the same \
                 fail-closed tap gate: {state:?} must classify {expected:?}"
            );
        }

        let Effect::SearchLibrary {
            split: Some(split), ..
        } = &mut search
        else {
            panic!("PREMISE: the mutated node must keep its split");
        };
        split.primary_enter_tapped = EtbTapState::Tapped;
        split.rest_destination = Zone::Battlefield;
        assert_eq!(
            effect_window_reach(&search),
            WindowReach::MayInterfere,
            "`rest_destination` carries NO tap state of its own, so a battlefield rest can never be \
             proven tapped and is reach by construction"
        );
    }
}
