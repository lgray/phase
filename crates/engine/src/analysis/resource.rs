//! `ResourceVector`: the monotone resource axes a net-progress loop can pump,
//! plus the resource-projected loop equality that distinguishes a beneficial
//! (CR 732.2) loop from a mandatory-draw (CR 104.4b / CR 732.4) loop.
//!
//! # Why a *separate* comparison from `loop_states_equal`
//!
//! CR 104.4b: a loop of *mandatory* actions that repeats a sequence "with no way
//! to stop" is a draw. The engine's existing `loop_states_equal` answers exactly
//! that question: it treats two states as the same loop point only when life,
//! damage, counters, and mana also match — because a mandatory loop that keeps
//! changing those values is not truly repeating and is *not* a draw.
//!
//! CR 732.2a: a player may instead take a *shortcut* through a loop "that repeats
//! a specified number of times". This is how a *beneficial* loop terminates: it
//! makes net progress on some resource each cycle (deal 1 more damage, add 1 more
//! mana, mill 1 more card), so the board returns to an identical configuration
//! while a resource counter strictly increases. Detecting that requires the
//! **complement** of `loop_states_equal`: board/zones/tap-state identical, but the
//! monotone resources allowed to differ.
//!
//! [`ResourceVector`] is the typed catalogue of those monotone axes;
//! [`loop_states_equal_modulo_resources`] is the projected comparison.

use std::collections::BTreeMap;

use crate::types::card_type::CoreType;
use crate::types::counter::CounterType;
use crate::types::game_state::{loop_states_equal, GameState};
use crate::types::mana::ManaType;
use crate::types::player::PlayerId;

/// WUBRG + colorless, the canonical index order used by [`ResourceVector::mana`].
///
/// Matches `ManaColor::ALL` (WUBRG) with colorless appended, so index `i` of the
/// mana array is `MANA_INDEX[i]`.
const MANA_INDEX: [ManaType; 6] = [
    ManaType::White,
    ManaType::Blue,
    ManaType::Black,
    ManaType::Red,
    ManaType::Green,
    ManaType::Colorless,
];

/// CR 122.1: classification of the object/player a counter sits on, so a counter
/// axis is keyed by *what kind of thing accumulates it* (a +1/+1 loop on a
/// creature is a different unbounded resource than loyalty on a planeswalker).
///
/// Typed rather than stringly so the win-classifier can `match` exhaustively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectClass {
    /// CR 302: a creature on the battlefield.
    Creature,
    /// CR 306: a planeswalker on the battlefield.
    Planeswalker,
    /// CR 310: a battle on the battlefield.
    Battle,
    /// CR 119 / CR 122: a player (poison, energy, experience, …).
    Player,
    /// Any other counter-bearing object (artifact, enchantment, land, …).
    Other,
}

/// CR 122.1: analysis-layer classification of a counter kind.
///
/// The engine's [`CounterType`] is intentionally **not** reused as a map key
/// here: it derives neither `Ord` (required for `BTreeMap` keys) nor a small
/// closed set — it carries `Generic(String)`, `Keyword(KeywordKind)`, and
/// parameterized `PowerToughness { .. }` variants. Adding `Ord` to that
/// crate-wide enum (and transitively to `KeywordKind`) to satisfy one analysis
/// map would be a far larger, non-additive change. Instead this module owns a
/// small `Ord` classification of the counter dimensions the corpus cares about
/// (CR 122.1: +1/+1, loyalty, poison, …) and folds the long tail into `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CounterClass {
    /// CR 122.1a: a +1/+1 counter.
    Plus1Plus1,
    /// CR 122.1a: a -1/-1 counter.
    Minus1Minus1,
    /// CR 306.5b: a loyalty counter on a planeswalker.
    Loyalty,
    /// CR 310.4c: a defense counter on a battle.
    Defense,
    /// CR 122.1 + CR 704.5c: a poison counter on a player (10 ⇒ that player loses).
    Poison,
    /// CR 122.1: an energy counter ({E}) in a player's energy reserve.
    Energy,
    /// Any other counter kind (charge, lore, time, keyword, generic, …).
    Other,
}

impl CounterClass {
    /// Map an engine [`CounterType`] to its analysis classification.
    fn from_counter_type(ct: &CounterType) -> CounterClass {
        match ct {
            CounterType::Plus1Plus1 => CounterClass::Plus1Plus1,
            CounterType::Minus1Minus1 => CounterClass::Minus1Minus1,
            CounterType::Loyalty => CounterClass::Loyalty,
            CounterType::Defense => CounterClass::Defense,
            _ => CounterClass::Other,
        }
    }
}

/// A non-counter, non-mana trigger/event family whose firings a loop can pump
/// without changing the board (the canonical example is proliferate, but also
/// magecraft, constellation, etc.). Typed rather than stringly.
///
/// CR 701.x keyword-action and CR 603.x triggered-ability families. These counts
/// are **not** directly readable from a `GameState` snapshot — they are events,
/// not stored totals — so [`ResourceVector::snapshot`] always leaves
/// [`ResourceVector::generic_triggers`] empty and the simulation harness (PR-1)
/// feeds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TriggerKind {
    /// CR 701.34: proliferate (the keyword action a loop can pump mana-neutrally).
    Proliferate,
    /// CR 207.2c + CR 603: magecraft — an ability word (no individual CR entry)
    /// for a triggered ability that fires on casting/copying an instant or sorcery.
    Magecraft,
    /// CR 207.2c + CR 603: constellation — an ability word for a triggered
    /// ability that fires when an enchantment enters under your control.
    Constellation,
    /// CR 207.2c + CR 603: landfall — an ability word for a triggered ability
    /// that fires when a land enters under your control.
    Landfall,
    /// Any other tracked trigger/keyword-action family.
    Other,
}

/// A vector of the **monotone** resources an infinite loop can pump.
///
/// "Monotone" = a beneficial loop only ever drives these in one direction within
/// a cycle (it gains mana/life/damage/tokens/triggers; a *consumed* axis like
/// mana or life may also be spent, which is why net-progress is tested as a
/// *delta* over a full cycle, not per step).
///
/// # Two population sources
///
/// 1. **State-readable** (filled by [`ResourceVector::snapshot`]): absolute
///    levels the engine stores directly — floating mana, per-player life,
///    library sizes, and counters on objects/players.
/// 2. **Event-fed** (left zero by `snapshot`, populated externally by the PR-1
///    harness): counts of events the engine does not retain as a running total
///    readable from a single `GameState` — damage dealt, tokens created, cards
///    drawn, casts, and trigger firings. Each such field is documented below.
///
/// Compare two snapshots with [`ResourceVector::delta`] to get the per-cycle
/// change; [`ResourceVector::is_net_progress`] then decides whether the cycle is
/// a beneficial (CR 732.2) loop.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceVector {
    /// CR 106.1: floating mana by color, indexed `[W, U, B, R, G, C]` (see
    /// [`MANA_INDEX`]). Summed across all players' pools. **State-readable.**
    pub mana: [i64; 6],

    /// CR 119.1: per-player life total. **State-readable.**
    pub life: BTreeMap<PlayerId, i64>,

    /// CR 120.1: cumulative damage *dealt to* each player this analysis window.
    /// Damage is an event, not a stored total. **Event-fed** (left empty by
    /// `snapshot`).
    pub damage_dealt: BTreeMap<PlayerId, i64>,

    /// CR 401: per-player library size, as a signed delta-friendly count.
    /// Positive = larger library. Mill loops drive this negative.
    /// **State-readable** (absolute library size at snapshot time).
    pub library_delta: BTreeMap<PlayerId, i64>,

    /// CR 111: tokens created this analysis window. **Event-fed.**
    pub tokens_created: i64,

    /// CR 121: cards drawn this analysis window. **Event-fed.**
    pub cards_drawn: i64,

    /// CR 601: spells cast this analysis window (storm / cast-count loops).
    /// **Event-fed.**
    pub casts_this_step: i64,

    /// CR 207.2c + CR 603: landfall triggers this window (landfall is an ability
    /// word for a land-enters triggered ability). **Event-fed.**
    pub landfall_triggers: i64,

    /// CR 500.8 + CR 506: extra combat phases created this window (CR 500.8
    /// governs adding phases; CR 506 is the combat phase). **Event-fed.**
    pub combat_phases: i64,

    /// CR 500.7: extra turns created this window. **Event-fed.**
    pub extra_turns: i64,

    /// CR 700.4 + CR 603.6c: "dies" (leaves-the-battlefield-to-graveyard)
    /// triggers this window. **Event-fed.**
    pub death_triggers: i64,
    /// CR 603.6a: enters-the-battlefield triggers this window. **Event-fed.**
    pub etb_triggers: i64,
    /// CR 603.6c: leaves-the-battlefield triggers this window. **Event-fed.**
    pub ltb_triggers: i64,
    /// CR 701.21: sacrifice triggers this window. **Event-fed.**
    pub sac_triggers: i64,

    /// CR 122.1: counters by `(kind, object class)`. Includes +1/+1, loyalty,
    /// and poison (poison/energy are keyed under [`ObjectClass::Player`]).
    /// **State-readable.**
    pub counters: BTreeMap<(CounterClass, ObjectClass), i64>,

    /// Generic trigger/keyword-action firings by family (proliferate, magecraft,
    /// …) — the mana-neutral axis a proliferate loop pumps. **Event-fed.**
    pub generic_triggers: BTreeMap<TriggerKind, i64>,
}

impl ResourceVector {
    /// Snapshot the **state-readable** resource levels directly out of a
    /// `GameState`: floating mana, per-player life, per-player library size, and
    /// counters on every object (battlefield) and player.
    ///
    /// Event-fed fields (damage, tokens, draws, casts, all `*_triggers`, and
    /// [`Self::generic_triggers`]) are left at their `Default` (zero/empty); the
    /// PR-1 harness feeds them from the event stream.
    pub fn snapshot(state: &GameState) -> ResourceVector {
        let mut v = ResourceVector::default();

        // CR 106.1: floating mana, summed across every player's pool.
        for player in &state.players {
            for (i, color) in MANA_INDEX.iter().enumerate() {
                v.mana[i] += player.mana_pool.count_color(*color) as i64;
            }
            // CR 119.1: per-player life.
            v.life.insert(player.id, player.life as i64);
            // CR 401: per-player library size.
            v.library_delta
                .insert(player.id, player.library.len() as i64);
            // CR 122.1 + CR 704.5c: poison counters live in a dedicated field.
            if player.poison_counters > 0 {
                v.counters.insert(
                    (CounterClass::Poison, ObjectClass::Player),
                    player.poison_counters as i64,
                );
            }
            // CR 122.1: energy reserve.
            if player.energy > 0 {
                v.counters.insert(
                    (CounterClass::Energy, ObjectClass::Player),
                    player.energy as i64,
                );
            }
        }

        // CR 122.1: counters on battlefield objects, keyed by counter kind and
        // the bearer's object class.
        for id in &state.battlefield {
            let Some(object) = state.objects.get(id) else {
                continue;
            };
            let class = object_class(object.card_types.core_types.as_slice());
            for (ct, count) in &object.counters {
                let key = (CounterClass::from_counter_type(ct), class);
                *v.counters.entry(key).or_insert(0) += *count as i64;
            }
        }

        v
    }

    /// Component-wise `after - before`. For map-backed axes, missing keys are
    /// treated as `0`, and the result keeps any key present on either side.
    ///
    /// The result is the per-cycle change to feed [`Self::is_net_progress`].
    pub fn delta(before: &ResourceVector, after: &ResourceVector) -> ResourceVector {
        let mut mana = [0i64; 6];
        for (i, slot) in mana.iter_mut().enumerate() {
            *slot = after.mana[i] - before.mana[i];
        }
        ResourceVector {
            mana,
            life: map_delta(&before.life, &after.life),
            damage_dealt: map_delta(&before.damage_dealt, &after.damage_dealt),
            library_delta: map_delta(&before.library_delta, &after.library_delta),
            tokens_created: after.tokens_created - before.tokens_created,
            cards_drawn: after.cards_drawn - before.cards_drawn,
            casts_this_step: after.casts_this_step - before.casts_this_step,
            landfall_triggers: after.landfall_triggers - before.landfall_triggers,
            combat_phases: after.combat_phases - before.combat_phases,
            extra_turns: after.extra_turns - before.extra_turns,
            death_triggers: after.death_triggers - before.death_triggers,
            etb_triggers: after.etb_triggers - before.etb_triggers,
            ltb_triggers: after.ltb_triggers - before.ltb_triggers,
            sac_triggers: after.sac_triggers - before.sac_triggers,
            counters: map_delta(&before.counters, &after.counters),
            generic_triggers: map_delta(&before.generic_triggers, &after.generic_triggers),
        }
    }

    /// Iterate every scalar component of this vector as a signed value, paired
    /// with whether that axis is **consumed** (may legitimately be spent inside a
    /// beneficial loop, e.g. mana and life) — see [`Self::is_net_progress`].
    fn components(&self) -> impl Iterator<Item = (Component, i64)> + '_ {
        let mana = self
            .mana
            .iter()
            .map(|&n| (Component::Consumed, n))
            .collect::<Vec<_>>();
        let life = self.life.values().map(|&n| (Component::Consumed, n));
        let library = self.library_delta.values().map(|&n| (Component::Gained, n));
        let damage = self.damage_dealt.values().map(|&n| (Component::Gained, n));
        let counters = self.counters.values().map(|&n| (Component::Gained, n));
        let triggers = self
            .generic_triggers
            .values()
            .map(|&n| (Component::Gained, n));
        let scalars = [
            self.tokens_created,
            self.cards_drawn,
            self.casts_this_step,
            self.landfall_triggers,
            self.combat_phases,
            self.extra_turns,
            self.death_triggers,
            self.etb_triggers,
            self.ltb_triggers,
            self.sac_triggers,
        ]
        .map(|n| (Component::Gained, n));

        mana.into_iter()
            .chain(life)
            .chain(library)
            .chain(damage)
            .chain(counters)
            .chain(triggers)
            .chain(scalars)
    }

    /// CR 732.2a: is this delta a **net-progress** cycle — the signature of a
    /// beneficial loop that should be shortcut rather than drawn?
    ///
    /// True iff:
    /// 1. at least one component strictly increased (the loop makes progress
    ///    each cycle), and
    /// 2. no **consumed** component (mana, life) is net-negative — a loop that
    ///    spends more mana/life than it makes is not sustainable and would stop
    ///    on its own (so it is not an infinite net-progress loop).
    ///
    /// `Gained` axes (damage, tokens, draws, counters, triggers, library) are
    /// allowed to be negative on a *given* axis (e.g. a mill loop drives
    /// `library_delta` negative — that is the win, not a violation); only the
    /// *consumed* axes constrain sustainability. A mill loop still satisfies (1)
    /// via some other axis (or via a negative library being the unbounded
    /// resource — callers read [`Self::unbounded_components`] for that).
    pub fn is_net_progress(&self) -> bool {
        let mut any_increase = false;
        for (component, value) in self.components() {
            match component {
                Component::Consumed if value < 0 => return false,
                _ => {}
            }
            if value > 0 {
                any_increase = true;
            }
        }
        any_increase
    }

    /// The component axes that strictly increased over this delta — the
    /// candidate **unbounded** resources a `WinKind` classifier (PR-2) reads to
    /// name the loop's win condition. A mill axis surfaces here as a negative
    /// `library_delta`, so it is reported separately via its sign.
    ///
    /// Returns each increasing axis as a [`ResourceAxis`] tag with its signed
    /// magnitude.
    pub fn unbounded_components(&self) -> Vec<(ResourceAxis, i64)> {
        let mut out = Vec::new();
        for (i, &n) in self.mana.iter().enumerate() {
            if n > 0 {
                out.push((ResourceAxis::Mana(MANA_INDEX[i]), n));
            }
        }
        for (pid, &n) in &self.life {
            if n > 0 {
                out.push((ResourceAxis::Life(*pid), n));
            }
        }
        for (pid, &n) in &self.damage_dealt {
            if n > 0 {
                out.push((ResourceAxis::DamageDealt(*pid), n));
            }
        }
        // CR 401: a mill loop is unbounded *downward* on library size.
        for (pid, &n) in &self.library_delta {
            if n != 0 {
                out.push((ResourceAxis::LibraryDelta(*pid), n));
            }
        }
        for (&key, &n) in &self.counters {
            if n > 0 {
                out.push((ResourceAxis::Counter(key.0, key.1), n));
            }
        }
        for (&kind, &n) in &self.generic_triggers {
            if n > 0 {
                out.push((ResourceAxis::Trigger(kind), n));
            }
        }
        for (axis, n) in [
            (ResourceAxis::TokensCreated, self.tokens_created),
            (ResourceAxis::CardsDrawn, self.cards_drawn),
            (ResourceAxis::Casts, self.casts_this_step),
            (ResourceAxis::LandfallTriggers, self.landfall_triggers),
            (ResourceAxis::CombatPhases, self.combat_phases),
            (ResourceAxis::ExtraTurns, self.extra_turns),
            (ResourceAxis::DeathTriggers, self.death_triggers),
            (ResourceAxis::EtbTriggers, self.etb_triggers),
            (ResourceAxis::LtbTriggers, self.ltb_triggers),
            (ResourceAxis::SacTriggers, self.sac_triggers),
        ] {
            if n > 0 {
                out.push((axis, n));
            }
        }
        out
    }
}

/// Whether a resource axis is *consumed* (spendable inside a loop) or purely
/// *gained*. Consumed axes constrain loop sustainability; see
/// [`ResourceVector::is_net_progress`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Consumed,
    Gained,
}

/// A tagged, named resource axis — the typed identity of one unbounded resource,
/// used by the (PR-2) `WinKind` classifier to describe a loop certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAxis {
    Mana(ManaType),
    Life(PlayerId),
    DamageDealt(PlayerId),
    LibraryDelta(PlayerId),
    Counter(CounterClass, ObjectClass),
    Trigger(TriggerKind),
    TokensCreated,
    CardsDrawn,
    Casts,
    LandfallTriggers,
    CombatPhases,
    ExtraTurns,
    DeathTriggers,
    EtbTriggers,
    LtbTriggers,
    SacTriggers,
}

/// CR 122.1: classify a counter-bearing object by its core types.
fn object_class(core_types: &[CoreType]) -> ObjectClass {
    if core_types.contains(&CoreType::Creature) {
        ObjectClass::Creature
    } else if core_types.contains(&CoreType::Planeswalker) {
        ObjectClass::Planeswalker
    } else if core_types.contains(&CoreType::Battle) {
        ObjectClass::Battle
    } else {
        ObjectClass::Other
    }
}

/// Component-wise `after - before` for an ordered map, retaining every key on
/// either side and dropping entries that net to zero.
fn map_delta<K: Ord + Copy>(
    before: &BTreeMap<K, i64>,
    after: &BTreeMap<K, i64>,
) -> BTreeMap<K, i64> {
    let mut out = BTreeMap::new();
    for (&k, &a) in after {
        let b = before.get(&k).copied().unwrap_or(0);
        let d = a - b;
        if d != 0 {
            out.insert(k, d);
        }
    }
    for (&k, &b) in before {
        if !after.contains_key(&k) && b != 0 {
            out.insert(k, -b);
        }
    }
    out
}

/// CR 732.2a vs CR 104.4b: the **complement** of the engine's strict loop
/// equality (`types::game_state::loop_states_equal`).
///
/// `loop_states_equal` treats two states as the same loop point only when life,
/// damage, counters, power/toughness, loyalty, and mana also match — correct for
/// a *mandatory* loop, which is a draw (CR 104.4b / CR 732.4) only if it truly
/// repeats with nothing changing.
///
/// This function answers the opposite question for a *beneficial* loop
/// (CR 732.2a, the shortcut): are the two states identical in **board, zones, and
/// tap-state**, allowing the monotone resources to differ? It is built directly
/// on `normalize_for_loop` (so it inherits the exact volatile-field exclusions
/// the strict path uses) and then additionally projects out the monotone
/// resources before delegating to `loop_states_equal`:
///
/// - per-player `life`, `mana_pool`, and the per-turn resource trackers
///   (life gained/lost, cards drawn, tokens, …) the strict `PartialEq` compares;
/// - per-object `damage_marked` and `counters` (and the counter-derived
///   `power`/`toughness`/`loyalty`/`defense`), so a +1/+1 or loyalty pump loop is
///   recognized as the same board.
///
/// Everything else — controller, zone, tapped, attachments, names, object count,
/// stack, phase, priority — must still match exactly, so a genuine board change
/// (an extra permanent, a different tap state, a moved card) returns `false`.
pub fn loop_states_equal_modulo_resources(a: &GameState, b: &GameState) -> bool {
    let pa = project_out_resources(a);
    let pb = project_out_resources(b);
    loop_states_equal(&pa, &pb)
}

/// Clone a state through `normalize_for_loop` and additionally zero every
/// monotone resource the modulo comparison must ignore. The result is only ever
/// fed to `loop_states_equal`; it is never used as a live game state.
fn project_out_resources(state: &GameState) -> GameState {
    let mut s = state.normalize_for_loop();

    for player in &mut s.players {
        // CR 119: life is monotone in a drain/lifegain loop.
        player.life = 0;
        // CR 106.1: floating mana is consumed/produced within the loop.
        player.mana_pool.clear();
        // CR 122.1: player counters that a loop pumps (poison/energy/…).
        player.poison_counters = 0;
        player.energy = 0;
        player.player_counters.clear();
        // Per-turn resource trackers the strict PartialEq compares — these grow
        // with the loop but do not change the board configuration.
        player.life_gained_this_turn = 0;
        player.life_lost_this_turn = 0;
        player.cards_drawn_this_turn = 0;
        player.cards_drawn_this_step = 0;
    }

    for (_, object) in s.objects.iter_mut() {
        // CR 120: marked damage is a monotone resource (lifelink/ping loops).
        object.damage_marked = 0;
        // CR 122.1: counters (and their derived P/T/loyalty/defense) are the
        // pumped resource of a +1/+1 or loyalty loop; project them all out so
        // two cycles of such a loop compare as the same board.
        object.counters.clear();
        object.power = None;
        object.toughness = None;
        object.loyalty = None;
        object.defense = None;
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::game_object::GameObject;
    use crate::types::identifiers::{CardId, ObjectId};
    use crate::types::zones::Zone;

    fn pid(n: u8) -> PlayerId {
        PlayerId(n)
    }

    fn battlefield_creature(state: &mut GameState, id: u64, controller: u8) -> ObjectId {
        let oid = ObjectId(id);
        let mut object = GameObject::new(
            oid,
            CardId(1),
            PlayerId(controller),
            "Walking Ballista".to_string(),
            Zone::Battlefield,
        );
        object.card_types.core_types = vec![CoreType::Artifact, CoreType::Creature];
        state.objects.insert(oid, object);
        state.battlefield.push_back(oid);
        oid
    }

    /// CR 104.4b vs CR 732.2a: two byte-identical states must compare equal under
    /// BOTH the strict equality and the resource-modulo equality.
    #[test]
    fn identical_states_equal_under_both_comparisons() {
        let mut state = GameState::new_two_player(7);
        battlefield_creature(&mut state, 500, 0);
        let copy = state.clone();

        assert!(
            loop_states_equal(&state.normalize_for_loop(), &copy.normalize_for_loop()),
            "identical states must be strictly equal"
        );
        assert!(
            loop_states_equal_modulo_resources(&state, &copy),
            "identical states must be modulo-resources equal"
        );
    }

    /// THE KEY DISCRIMINATOR (CR 732.2a vs CR 104.4b): same board but different
    /// life, mana, and counters must be **modulo-resources equal** (a beneficial
    /// loop point) yet **strictly unequal** (not a mandatory-draw loop). This is
    /// the entire reason the modulo comparison exists; reverting the resource
    /// projection makes the modulo assertion fail.
    #[test]
    fn same_board_different_resources_is_modulo_equal_but_strictly_unequal() {
        let mut a = GameState::new_two_player(7);
        let oid = battlefield_creature(&mut a, 500, 0);

        let mut b = a.clone();
        // Drain a life point, float a red mana, add a +1/+1 counter, mark damage.
        b.players[1].life -= 1;
        b.players[0].life += 1;
        b.players[0]
            .mana_pool
            .add(crate::types::mana::ManaUnit::new(
                ManaType::Red,
                oid,
                false,
                Vec::new(),
            ));
        if let Some(o) = b.objects.get_mut(&oid) {
            o.counters.insert(CounterType::Plus1Plus1, 3);
            o.damage_marked = 2;
        }

        assert!(
            !loop_states_equal(&a.normalize_for_loop(), &b.normalize_for_loop()),
            "differing life/mana/counters must NOT be strictly equal (else a wrongful CR 104.4b draw)"
        );
        assert!(
            loop_states_equal_modulo_resources(&a, &b),
            "same board with only monotone resources differing must be modulo-resources equal (CR 732.2a net-progress loop point)"
        );
    }

    /// A real board difference (an extra permanent) must make even the
    /// resource-modulo comparison return false — the projection must not blur
    /// genuine board changes.
    #[test]
    fn extra_permanent_is_not_modulo_equal() {
        let mut a = GameState::new_two_player(7);
        battlefield_creature(&mut a, 500, 0);
        let mut b = a.clone();
        battlefield_creature(&mut b, 501, 0);

        assert!(
            !loop_states_equal_modulo_resources(&a, &b),
            "an extra permanent is a genuine board change, not a resource difference"
        );
    }

    /// A different tap state is a genuine board difference (tap/untap loop phase)
    /// — modulo-resources must NOT blur it.
    #[test]
    fn different_tap_state_is_not_modulo_equal() {
        let mut a = GameState::new_two_player(7);
        let oid = battlefield_creature(&mut a, 500, 0);
        let mut b = a.clone();
        if let Some(o) = b.objects.get_mut(&oid) {
            o.tapped = true;
        }

        assert!(
            !loop_states_equal_modulo_resources(&a, &b),
            "a tapped-vs-untapped object is a board difference, not a resource difference"
        );
    }

    /// `snapshot` reads life, mana, library size, and counters directly out of a
    /// `GameState`; `delta` then measures a known monotone change exactly.
    #[test]
    fn snapshot_and_delta_measure_known_changes() {
        let mut before_state = GameState::new_two_player(7);
        let oid = battlefield_creature(&mut before_state, 500, 0);
        let before = ResourceVector::snapshot(&before_state);

        let mut after_state = before_state.clone();
        after_state.players[1].life -= 5; // opponent took 5 (drain)
        after_state.players[0]
            .mana_pool
            .add(crate::types::mana::ManaUnit::new(
                ManaType::Green,
                oid,
                false,
                Vec::new(),
            ));
        if let Some(o) = after_state.objects.get_mut(&oid) {
            o.counters.insert(CounterType::Plus1Plus1, 2);
        }
        let after = ResourceVector::snapshot(&after_state);

        let delta = ResourceVector::delta(&before, &after);

        // Green mana index is 4 in WUBRG+C order.
        assert_eq!(delta.mana[4], 1, "one green mana floated");
        assert_eq!(
            delta.life.get(&pid(1)).copied(),
            Some(-5),
            "opponent lost 5 life"
        );
        assert_eq!(
            delta
                .counters
                .get(&(CounterClass::Plus1Plus1, ObjectClass::Creature))
                .copied(),
            Some(2),
            "two +1/+1 counters added to a creature"
        );
        // Library unchanged ⇒ no key for either player.
        assert!(delta.library_delta.is_empty(), "no library change");
    }

    /// `is_net_progress` is true for a +damage / consume-nothing delta and false
    /// for a no-op and for a delta that net-consumes a consumed axis (life).
    #[test]
    fn net_progress_classification() {
        // +damage, nothing consumed ⇒ net progress.
        let mut win = ResourceVector::default();
        win.damage_dealt.insert(pid(1), 1);
        assert!(
            win.is_net_progress(),
            "+1 damage with no cost is net progress"
        );

        // No-op ⇒ not net progress.
        let noop = ResourceVector::default();
        assert!(
            !noop.is_net_progress(),
            "an empty delta is not net progress"
        );

        // Net-negative consumed axis (life) ⇒ not net progress even with a gain.
        let mut bleed = ResourceVector {
            tokens_created: 1,
            ..Default::default()
        };
        bleed.life.insert(pid(0), -1);
        assert!(
            !bleed.is_net_progress(),
            "a loop that net-loses life is not sustainable, so not infinite net progress"
        );
    }

    /// REVERT-PROBE for the modulo-vs-strict discriminator: a fabricated
    /// "strict-only" comparison (the *uncomplemented* equality, i.e. forgetting
    /// to project out resources) must reject the same-board/different-resources
    /// pair that the real modulo comparison accepts. This pins that the resource
    /// projection is load-bearing: remove it (fall back to `loop_states_equal`)
    /// and the discriminator collapses.
    #[test]
    fn revert_probe_projection_is_load_bearing() {
        let mut a = GameState::new_two_player(7);
        let oid = battlefield_creature(&mut a, 500, 0);
        let mut b = a.clone();
        b.players[1].life -= 1;
        if let Some(o) = b.objects.get_mut(&oid) {
            o.counters.insert(CounterType::Plus1Plus1, 1);
        }

        // The real (complemented) comparison accepts it.
        assert!(loop_states_equal_modulo_resources(&a, &b));
        // The un-complemented comparison (what a revert would leave) rejects it.
        assert!(
            !loop_states_equal(&a.normalize_for_loop(), &b.normalize_for_loop()),
            "without the resource projection the comparison would (wrongly) reject this beneficial-loop point"
        );
    }

    /// `unbounded_components` names the axis that grew — the input the PR-2
    /// `WinKind` classifier reads. A mill loop surfaces as a negative library.
    #[test]
    fn unbounded_components_names_growing_axes() {
        let mut drain = ResourceVector::default();
        drain.damage_dealt.insert(pid(1), 3);
        let axes = drain.unbounded_components();
        assert_eq!(axes, vec![(ResourceAxis::DamageDealt(pid(1)), 3)]);

        let mut mill = ResourceVector::default();
        mill.library_delta.insert(pid(1), -4);
        let axes = mill.unbounded_components();
        assert_eq!(
            axes,
            vec![(ResourceAxis::LibraryDelta(pid(1)), -4)],
            "a mill loop is unbounded downward on library size"
        );
    }
}
