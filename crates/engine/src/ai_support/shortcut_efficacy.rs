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
//! `Shorten`. MEASURED at this base, via `parse_oracle_text` on the verbatim
//! MTGJSON text: `Invoke Justice[0]` parses to a lone [`Effect::ChangeZone`]
//! (`origin: Graveyard`, `target: Typed{controller: You}`) with no cost, no
//! `sub_ability` and every conservative-when-present field absent, so it
//! classifies `OwnResourcesOnly` — while the card's sentence continues "then
//! distribute four +1/+1 counters among any number of creatures and/or Vehicles
//! target player controls". No widening of this module can close that: it
//! cannot read a clause that never reached it, so the residual is owned by the
//! parser, not by the classifier.
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

use crate::types::ability::{
    AbilityCost, AbilityDefinition, ControllerRef, Effect, FilterProp, PlayerFilter, TargetFilter,
};
use crate::types::actions::GameAction;
use crate::types::game_state::GameState;
use crate::types::identifiers::ObjectId;
use crate::types::zones::Zone;

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
        Effect::SearchLibrary {
            source_zones: _,
            filter: _,
            count: _,
            reveal: _,
            target_player,
            selection_constraint: _,
            split: _,
        } => WindowReach::of(target_player.is_none()),

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
        Effect::ChangeZone {
            origin,
            destination,
            target,
            owner_library: _,
            enter_transformed: _,
            enters_under: _,
            enter_tapped: _,
            enters_attacking: _,
            up_to: _,
            enter_with_counters: _,
            conditional_enter_with_counters: _,
            face_down_profile: _,
            enters_modified_if: _,
        } => WindowReach::of(
            filter_is_actor_owned(target)
                || (matches!(target, TargetFilter::Any)
                    && *origin == Some(Zone::Library)
                    && *destination == Zone::Battlefield),
        ),

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

/// Fold every ability an object carries. A missing object, or one carrying no
/// abilities at all, is `MayInterfere` — the fail-closed direction, and the one
/// that keeps an empty fold from being proven confined by its identity element.
fn object_window_reach(state: &GameState, object_id: ObjectId) -> WindowReach {
    let Some(object) = state.objects.get(&object_id) else {
        return WindowReach::MayInterfere;
    };
    if object.abilities.is_empty() {
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
fn indexed_ability_window_reach(
    state: &GameState,
    source_id: ObjectId,
    ability_index: usize,
) -> WindowReach {
    state
        .objects
        .get(&source_id)
        .and_then(|object| object.abilities.get(ability_index))
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
/// ponytail: a fetched permanent that itself enables interference is not
/// modelled. Using it needs a further priority window, and this design does NOT
/// claim one is guaranteed — CR 732.1b says only that the shortcut rules *can
/// be used* on a loop, and CR 732.2a makes proposing
/// permissive ("may suggest").
/// Scope, stated on BOTH axes:
///   - across windows: a bounded miss — a seat's fetched answer goes unused for
///     THIS shortcut;
///   - within the window: the worst case is NOT bounded by "one shortcut". On
///     an `UntilLethal` offer the accepted sequence runs to lethal, so the
///     in-window cost of a missed out is elimination.
///
/// Accepted because the miss requires the out to be reachable ONLY through the
/// fetched permanent; a directly-castable answer is already caught by the
/// top-level fold. Upgrade path: walk the fetched object's own abilities if a
/// real game shows a missed out. Owner: this lane, deferral burndown.
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
    use crate::parser::oracle::parse_oracle_text;

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
    const DARK_RITUAL: Card = Card {
        name: "Dark Ritual",
        oracle: "Add {B}{B}{B}.",
    };
    const LOTUS_PETAL: Card = Card {
        name: "Lotus Petal",
        oracle: "{T}, Sacrifice this artifact: Add one mana of any color.",
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
    /// `TargetFilter::SelfRef`, which `filter_is_actor_owned` PROVES actor-owned
    /// — and CR 701.21a is why proving it is SOUND rather than merely convenient
    /// ("A player can't sacrifice something that isn't a permanent, or something
    /// that's a permanent they don't control"). The rule grounds
    /// actor-OWNERSHIP; it does not dictate the AST encoding.
    /// So `cost_window_reach` returns `OwnResourcesOnly` and the ONLY thing that
    /// can carry the verdict is the mana effect. That is what separates this row
    /// from `v9b`'s Ironworks, which reaches `MayInterfere` through an UNPROVEN
    /// sacrifice filter and therefore never exercises this arm.
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
            "ATTRIBUTION: the SelfRef sacrifice IS proven actor-owned (CR 701.21a: a player can \
             only sacrifice a permanent they control), so the cost leg is confined and cannot be \
             what produces the verdict below"
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
}
