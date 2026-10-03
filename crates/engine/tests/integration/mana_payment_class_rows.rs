//! Casts payable only through a mana ability the payment must activate, driven
//! through `apply()` on boards of real cards. Each row builds its board twice:
//! once to read the cast's listing and castability and drive the listed cast
//! (the member's ability and each opener activated during payment, fodder
//! chosen), and once as the reach guard (member and openers activated by hand
//! at priority, then castability read with that pool).
//!
//! A row reads `"<listed><castable>/<end>"`: `t`/`f`, then where the spell ends
//! (`BF`, `GY`, `stk`, `H`, `X`), with `!P` when the payment stranded the spell
//! at priority.
use engine::ai_support::legal_actions_full;
use engine::game::apply;
use engine::game::casting::can_cast_object_now;
use engine::game::mana_abilities::is_mana_ability;
use engine::game::scenario::{GameRunner, GameScenario, P0};
use engine::game::scenario_db::GameScenarioDbExt;
use engine::types::ability::AbilityKind;
use engine::types::actions::GameAction;
use engine::types::game_state::{CastPaymentMode, GameState, WaitingFor};
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::zones::Zone;
use engine::types::zones::Zone::{Battlefield as B, Graveyard as G, Hand as H};

use crate::support::shared_card_db;

/// A card beside the member: 'f' fodder a cost may choose, 'T' token fodder,
/// 'a' opener (its longest mana ability is activated), 'e' Aura on the member,
/// 'p' plain.
type Card = (&'static str, Zone, char);

/// (tag, member, member's ability text, cards, counters (card or "member",
/// counter, n), spell, reach guard's castability, expected reading).
///
/// Tag prefixes: `m` casts with manual payment (`m2` activates the openers
/// first); setups after `#`: `speedN`, `drawnN`, `oppturn`, `tapped`, `membergy`,
/// `spellgy`/`spellexile` (the spell starts in the graveyard/exile).
type Row = (
    &'static str,
    &'static str,
    &'static str,
    &'static [Card],
    &'static [(&'static str, &'static str, u32)],
    &'static str,
    bool,
    &'static str,
);

fn longest_mana_index(state: &GameState, id: ObjectId) -> Option<usize> {
    state.objects[&id]
        .abilities
        .iter()
        .enumerate()
        .filter(|(_, a)| a.kind == AbilityKind::Activated && is_mana_ability(a))
        .max_by_key(|(_, a)| format!("{:?}", a.cost).len())
        .map(|(i, _)| i)
}

struct Board {
    r: GameRunner,
    fodder: Vec<ObjectId>,
    /// (source, ability index) to activate during payment, member first.
    acts: Vec<(ObjectId, usize)>,
    spell: ObjectId,
    /// The spell's first colored shard, answered at a mana-color prompt.
    hint: String,
}

fn build(row: &Row) -> Board {
    let (tag, member, text, cards, counters, spell, _, _) = *row;
    let db = shared_card_db().expect("card db");
    let mut s = GameScenario::new();
    s.at_phase(Phase::PreCombatMain);
    let m = s.add_real_card(P0, member, Zone::Hand, db);
    let mut raw = vec![m];
    let (mut fodder, mut openers, mut auras, mut tokens) = (vec![], vec![], vec![], vec![]);
    let mut named = vec![("member", m)];
    for (name, zone, role) in cards {
        let id = s.add_real_card(P0, name, if *zone == B { H } else { *zone }, db);
        if *zone == B {
            raw.push(id);
        }
        match role {
            'f' => fodder.push(id),
            'a' => openers.push(id),
            'e' => auras.push(id),
            'T' => {
                tokens.push(id);
                fodder.push(id);
            }
            _ => {}
        }
        named.push((*name, id));
    }
    let setups: Vec<&str> = tag.split('#').skip(1).collect();
    let spell_zone = if setups.contains(&"spellgy") {
        G
    } else if setups.contains(&"spellexile") {
        Zone::Exile
    } else {
        H
    };
    let sp = s.add_real_card(P0, spell, spell_zone, db);
    for _ in 0..3 {
        s.add_real_card(P0, "Wastes", Zone::Library, db);
    }
    let mut r = s.build();
    {
        let st = r.state_mut();
        let turn = st.turn_number;
        // Placed as permanents that entered on an earlier turn.
        for id in &raw {
            engine::game::zones::remove_from_zone(st, *id, H, P0);
            engine::game::zones::add_to_zone(st, *id, B, P0);
            let o = st.objects.get_mut(id).unwrap();
            o.zone = B;
            o.tapped = false;
            o.summoning_sick = false;
            o.entered_battlefield_turn = Some(turn.saturating_sub(1));
        }
        for a in &auras {
            st.objects.get_mut(a).unwrap().attached_to =
                Some(engine::game::game_object::AttachTarget::Object(m));
            st.objects.get_mut(&m).unwrap().attachments.push(*a);
        }
        for t in &tokens {
            st.objects.get_mut(t).unwrap().is_token = true;
        }
        for item in setups.iter().copied() {
            if let Some(n) = item.strip_prefix("speed") {
                st.players[0].speed = Some(n.parse().unwrap());
            } else if let Some(n) = item.strip_prefix("drawn") {
                st.players[0].cards_drawn_this_turn = n.parse().unwrap();
            } else if item == "oppturn" {
                st.active_player = engine::types::player::PlayerId(1);
                st.priority_player = P0;
                st.waiting_for = WaitingFor::Priority { player: P0 };
            } else if item == "tapped" {
                st.objects.get_mut(&m).unwrap().tapped = true;
            } else if item == "membergy" {
                engine::game::zones::remove_from_zone(st, m, B, P0);
                engine::game::zones::add_to_zone(st, m, G, P0);
                st.objects.get_mut(&m).unwrap().zone = G;
            }
        }
        for (who, counter, n) in counters {
            let id = named
                .iter()
                .find(|(nm, _)| nm == who)
                .expect("counter holder")
                .1;
            let ct = engine::types::counter::parse_counter_type(counter);
            *st.objects
                .get_mut(&id)
                .unwrap()
                .counters
                .entry(ct)
                .or_insert(0) += n;
        }
    }
    engine::game::layers::flush_layers(r.state_mut());
    let member_index = r.state().objects[&m]
        .abilities
        .iter()
        .position(|a| a.kind == AbilityKind::Activated && a.description.as_deref() == Some(text))
        .or_else(|| longest_mana_index(r.state(), m));
    let mut acts: Vec<(ObjectId, usize)> = member_index.map(|i| (m, i)).into_iter().collect();
    acts.extend(
        openers
            .iter()
            .filter_map(|o| longest_mana_index(r.state(), *o).map(|i| (*o, i))),
    );
    if tag.starts_with("m2/") {
        acts.rotate_left(1);
    }
    let hint = match &r.state().objects[&sp].mana_cost {
        engine::types::mana::ManaCost::Cost { shards, .. } => shards
            .iter()
            .map(|x| format!("{x:?}"))
            .find(|x| ["White", "Blue", "Black", "Red", "Green"].contains(&x.as_str()))
            .unwrap_or_default(),
        _ => String::new(),
    };
    Board {
        r,
        fodder,
        acts,
        spell: sp,
        hint,
    }
}

fn listed_cast(state: &GameState, spell: ObjectId) -> Option<GameAction> {
    let (flat, _, _) = legal_actions_full(state);
    flat.into_iter().find(|a| {
        matches!(a, GameAction::CastSpell { .. }) && a.related_object_ids().contains(&spell)
    })
}

fn step(r: &mut GameRunner, action: GameAction) -> bool {
    let actor = r.state().waiting_for.acting_player().unwrap_or(P0);
    apply(r.state_mut(), actor, action).is_ok()
}

/// A land's mana rows are grouped `TapLandForMana` actions, never flat ones.
fn land_row(state: &GameState, act: (ObjectId, usize), hint: &str) -> Option<GameAction> {
    let (_, _, grouped) = legal_actions_full(state);
    let rows: Vec<&GameAction> = grouped
        .values()
        .flatten()
        .filter(|x| {
            matches!(x, GameAction::TapLandForMana { selection }
                if selection.source.object_id == act.0 && selection.ability_index == Some(act.1))
        })
        .collect();
    rows.iter()
        .find(|x| {
            matches!(x, GameAction::TapLandForMana { selection }
                if !hint.is_empty() && format!("{:?}", selection.mana_type) == hint)
        })
        .or(rows.first())
        .map(|x| (*x).clone())
}

/// Answer prompts until priority: each pending activation once, a selection
/// naming only fodder, the largest amount, a mana color, the payment's finish,
/// else the first action that neither cancels nor backs out.
fn answer(b: &mut Board, done: &mut [bool]) {
    for _ in 0..24 {
        if matches!(b.r.state().waiting_for, WaitingFor::Priority { .. }) {
            return;
        }
        if let WaitingFor::ManaSourceSelection { options, .. } = &b.r.state().waiting_for {
            let selection = options
                .iter()
                .find(|o| b.acts.iter().any(|(id, _)| *id == o.source.object_id))
                .or(options.first())
                .cloned();
            if let Some(selection) = selection {
                for (k, (id, _)) in b.acts.iter().enumerate() {
                    done[k] |= *id == selection.source.object_id;
                }
                if !step(&mut b.r, GameAction::ActivateManaSource { selection }) {
                    return;
                }
                continue;
            }
        }
        let state = b.r.state();
        let (flat, _, _) = legal_actions_full(state);
        let lands: Vec<Option<GameAction>> = b
            .acts
            .iter()
            .map(|a| land_row(state, *a, &b.hint))
            .collect();
        let pending = (0..b.acts.len()).find(|&k| {
            let (source, index) = b.acts[k];
            !done[k]
                && (lands[k].is_some()
                    || flat.iter().any(|x| {
                        matches!(x, GameAction::ActivateAbility { source_id, ability_index }
                            if *source_id == source && *ability_index == index)
                    }))
        });
        let pick = if let Some(p) = pending {
            done[p] = true;
            Some(lands[p].clone().unwrap_or(GameAction::ActivateAbility {
                source_id: b.acts[p].0,
                ability_index: b.acts[p].1,
            }))
        } else {
            let fodder_only = |a: &&GameAction| {
                matches!(a, GameAction::SelectCards { cards }
                    if !cards.is_empty() && cards.iter().all(|c| b.fodder.contains(c)))
            };
            flat.iter()
                .filter(fodder_only)
                .max_by_key(|a| match a {
                    GameAction::SelectCards { cards } => cards.len(),
                    _ => 0,
                })
                .or_else(|| {
                    flat.iter()
                        .filter(|a| matches!(a, GameAction::SubmitPayAmount { .. }))
                        .max_by_key(|a| match a {
                            GameAction::SubmitPayAmount { amount } => *amount as i64,
                            _ => -1,
                        })
                })
                .or_else(|| {
                    flat.iter()
                        .find(|a| {
                            matches!(a, GameAction::ChooseManaColor { .. })
                                && !b.hint.is_empty()
                                && format!("{a:?}").contains(b.hint.as_str())
                        })
                        .or_else(|| {
                            flat.iter()
                                .find(|a| matches!(a, GameAction::ChooseManaColor { .. }))
                        })
                })
                .or_else(|| {
                    matches!(state.waiting_for, WaitingFor::ManaPayment { .. })
                        .then(|| flat.iter().find(|a| matches!(a, GameAction::PassPriority)))
                        .flatten()
                })
                .or_else(|| {
                    flat.iter().find(|a| {
                        !matches!(
                            a,
                            GameAction::CancelCast
                                | GameAction::BackToManaPayment
                                | GameAction::PassPriority
                        ) && !matches!(a, GameAction::SelectCards { cards }
                            if cards.iter().any(|c| !b.fodder.contains(c)))
                    })
                })
                .cloned()
        };
        let Some(action) = pick else {
            return;
        };
        if !step(&mut b.r, action) {
            return;
        }
    }
}

/// The row's reach guard (castability after activating member and openers by
/// hand at priority) and its reading of the listed cast.
fn drive(row: &Row) -> (bool, String) {
    let mut b = build(row);
    let castable = can_cast_object_now(b.r.state(), P0, b.spell);
    let offered = listed_cast(b.r.state(), b.spell);
    let mut done = vec![false; b.acts.len()];
    let mut stranded = false;
    let cast = if row.0.starts_with('m') {
        Some(GameAction::CastSpell {
            object_id: b.spell,
            card_id: b.r.state().objects[&b.spell].card_id,
            targets: vec![],
            payment_mode: CastPaymentMode::Manual,
        })
    } else {
        offered.clone()
    };
    if let Some(cast) = cast {
        if step(&mut b.r, cast) {
            answer(&mut b, &mut done);
            for _ in 0..6 {
                if b.r.state().stack.is_empty() {
                    break;
                }
                if matches!(b.r.state().waiting_for, WaitingFor::Priority { .. }) {
                    if !step(&mut b.r, GameAction::PassPriority) {
                        stranded = matches!(b.r.state().waiting_for, WaitingFor::Priority { .. });
                        break;
                    }
                } else {
                    answer(&mut b, &mut done);
                    if !matches!(b.r.state().waiting_for, WaitingFor::Priority { .. }) {
                        break;
                    }
                }
            }
        }
    }
    let end = match b.r.state().objects.get(&b.spell).map(|o| o.zone) {
        Some(B) => "BF",
        Some(G) => "GY",
        Some(Zone::Stack) => "stk",
        Some(H) => "H",
        Some(Zone::Exile) => "X",
        other => panic!("{}: spell ended in {other:?}", row.0),
    };
    let flag = |v: bool| if v { 't' } else { 'f' };
    let reading = format!(
        "{}{}/{end}{}",
        flag(offered.is_some()),
        flag(castable),
        if stranded { "!P" } else { "" }
    );

    let mut c = build(row);
    let mut done = vec![false; c.acts.len()];
    for k in 0..c.acts.len() {
        if done[k] {
            continue;
        }
        done[k] = true;
        let act = c.acts[k];
        let action = land_row(c.r.state(), act, &c.hint).unwrap_or(GameAction::ActivateAbility {
            source_id: act.0,
            ability_index: act.1,
        });
        if step(&mut c.r, action) {
            answer(&mut c, &mut done);
        }
    }
    (can_cast_object_now(c.r.state(), P0, c.spell), reading)
}

/// Every row's reach guard, then its reading; all mismatches reported at once.
fn check(rows: &[Row]) {
    if shared_card_db().is_none() {
        return;
    }
    let failures: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            let (reach, reading) = drive(row);
            if reach != row.6 {
                Some(format!("{}: reach guard castable={reach}", row.0))
            } else if reading != row.7 {
                Some(format!("{}: read {reading}, expected {}", row.0, row.7))
            } else {
                None
            }
        })
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

/// CR 601.2g-h + CR 605.3a: a source with a `{T}` sacrificial row and another
/// row keeps its tap for the sacrificial row when the automatic leg would
/// otherwise strand the payment; a cast payable either way is unchanged.
#[test]
fn dual_row_sacrificial_source_keeps_its_tap() {
    check(DUAL_ROW);
}

#[rustfmt::skip]
const DUAL_ROW: &[Row] = &[
    ("x/Crystal Vein", "Crystal Vein", "{T}, Sacrifice ~: Add {C}{C}.", &[("Phyrexian Altar", B, 'a'), ("Grizzly Bears", B, 'f')], &[], "Glaring Fleshraker", true, "tt/BF"),
    ("x/Dwarven Ruins", "Dwarven Ruins", "{T}, Sacrifice ~: Add {R}{R}.", &[("Ashnod's Altar", B, 'a'), ("Grizzly Bears", B, 'f')], &[], "Barbarian Horde", true, "tt/BF"),
    ("x/Ebon Stronghold", "Ebon Stronghold", "{T}, Sacrifice ~: Add {B}{B}.", &[("Ashnod's Altar", B, 'a'), ("Grizzly Bears", B, 'f')], &[], "Dross Crocodile", true, "tt/BF"),
    ("x/Havenwood Battleground", "Havenwood Battleground", "{T}, Sacrifice ~: Add {G}{G}.", &[("Ashnod's Altar", B, 'a'), ("Grizzly Bears", B, 'f')], &[], "Axebane Beast", true, "tt/BF"),
    ("x/Lake of the Dead", "Lake of the Dead", "{T}, Sacrifice a Swamp: Add {B}{B}{B}{B}.", &[("Swamp", B, 'f'), ("Ashnod's Altar", B, 'a'), ("Grizzly Bears", B, 'f')], &[], "Feral Abomination", true, "tt/BF"),
    ("x/Ruins of Trokair", "Ruins of Trokair", "{T}, Sacrifice ~: Add {W}{W}.", &[("Ashnod's Altar", B, 'a'), ("Grizzly Bears", B, 'f')], &[], "Dutiful Servants", true, "tt/BF"),
    ("x/Svyelunite Temple", "Svyelunite Temple", "{T}, Sacrifice ~: Add {U}{U}.", &[("Ashnod's Altar", B, 'a'), ("Grizzly Bears", B, 'f')], &[], "Amphin Cutthroat", true, "tt/BF"),
    ("uh/crystal-vein+island/coral-merfolk", "Crystal Vein", "{T}: Add {C}.", &[("Island", B, 'p')], &[], "Coral Merfolk", true, "tt/BF"),
    ("uh/tower+bears+ashnod/sliver-construct", "Phyrexian Tower", "{T}, Sacrifice a creature: Add {B}{B}.", &[("Grizzly Bears", B, 'f'), ("Ashnod's Altar", B, 'p')], &[], "Sliver Construct", false, "tt/BF"),
];

/// CR 605.3b + CR 601.2g-h: a mana ability whose mana sub-cost is paid by
/// another mana ability returns to the payment that activated it; the spell is
/// never stranded at priority.
#[test]
fn nested_mana_sub_cost_returns_to_the_payment() {
    check(NESTED_SUB_COST);
}

#[rustfmt::skip]
const NESTED_SUB_COST: &[Row] = &[
    ("m/celebrant+swamp/elves", "Blood Celebrant", "{B}, Pay 1 life: Add one mana of any color.", &[("Swamp", B, 'p')], &[], "Llanowar Elves", true, "tt/BF"),
    ("m2/celebrant+swamp/elves", "Blood Celebrant", "{B}, Pay 1 life: Add one mana of any color.", &[("Swamp", B, 'a')], &[], "Llanowar Elves", true, "tt/BF"),
    ("m/signet+island/first-wing", "Azorius Signet", "{1}, {T}: Add {W}{U}.", &[("Island", B, 'p')], &[], "Azorius First-Wing", true, "tt/BF"),
    ("m/mystic-gate+plains/first-wing", "Mystic Gate", "{W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.", &[("Plains", B, 'p')], &[], "Azorius First-Wing", false, "tt/H"),
    ("mc/sol-ring/bronze-sable", "Sol Ring", "{T}: Add {C}{C}.", &[], &[], "Bronze Sable", true, "tt/BF"),
    ("mc/ancient-tomb/bronze-sable", "Ancient Tomb", "{T}: Add {C}{C}. ~ deals 2 damage to you.", &[], &[], "Bronze Sable", true, "tt/BF"),
    ("u/celebrant+swamp/elves", "Blood Celebrant", "{B}, Pay 1 life: Add one mana of any color.", &[("Swamp", B, 'p')], &[], "Llanowar Elves", true, "tt/BF"),
    ("m/A-Hall of Tagsin", "A-Hall of Tagsin", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Abstergo Entertainment", "Abstergo Entertainment", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Abzan Devotee", "Abzan Devotee", "{1}: Add {W}, {B}, or {G}. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Agent of Stromgald", "Agent of Stromgald", "{R}: Add {B}.", &[("Mountain", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/An-Havva Township", "An-Havva Township", "{1}, {T}: Add {G}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/An-Havva Township (2)", "An-Havva Township", "{2}, {T}: Add {R} or {W}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Apprentice Wizard", "Apprentice Wizard", "{U}, {T}: Add {C}{C}{C}.", &[("Island", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Arcum's Astrolabe", "Arcum's Astrolabe", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Arena of Glory", "Arena of Glory", "{R}, {T}, Exert ~: Add {R}{R}. If that mana is spent on a creature spell, it gains haste until end of turn.", &[("Mountain", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Artifact Unknown Shores", "Artifact Unknown Shores", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Astrolabe", "Astrolabe", "{1}, {T}, Sacrifice ~: Add two mana of any one color. Draw a card at the beginning of the next turn's upkeep.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Aysen Abbey", "Aysen Abbey", "{1}, {T}: Add {W}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Aysen Abbey (2)", "Aysen Abbey", "{2}, {T}: Add {G} or {U}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Azorius Signet", "Azorius Signet", "{1}, {T}: Add {W}{U}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Baldur's Gate", "Baldur's Gate", "{2}, {T}: Add X mana of any one color, where X is the number of other Gates you control.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Barbed Sextant", "Barbed Sextant", "{1}, {T}, Sacrifice ~: Add one mana of any color. Draw a card at the beginning of the next turn's upkeep.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Barrels of Blasting Jelly", "Barrels of Blasting Jelly", "{1}: Add one mana of any color. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Baxter Building", "Baxter Building", "{4}, {T}: Add four mana in any combination of colors.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Blood Celebrant", "Blood Celebrant", "{B}, Pay 1 life: Add one mana of any color.", &[("Swamp", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Bog Initiate", "Bog Initiate", "{1}: Add {B}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Bog Witch", "Bog Witch", "{B}, {T}, Discard a card: Add {B}{B}{B}.", &[("Swamp", B, 'p'), ("Grizzly Bears", H, 'f')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Boros Signet", "Boros Signet", "{1}, {T}: Add {R}{W}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Cabal Coffers", "Cabal Coffers", "{2}, {T}: Add {B} for each Swamp you control.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Cabal Stronghold", "Cabal Stronghold", "{3}, {T}: Add {B} for each basic Swamp you control.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Calciform Pools", "Calciform Pools", "{1}, Remove X storage counters from ~: Add X mana in any combination of {W} and/or {U}.", &[("Wastes", B, 'p')], &[("member", "storage", 2)], "Metallic Sliver", true, "tt/BF"),
    ("m/Capital City", "Capital City", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Captivating Cave", "Captivating Cave", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Cascade Bluffs", "Cascade Bluffs", "{U/R}, {T}: Add {U}{U}, {U}{R}, or {R}{R}.", &[("Island", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Cascading Cataracts", "Cascading Cataracts", "{5}, {T}: Add five mana in any combination of colors.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Castle Garenbrig", "Castle Garenbrig", "{2}{G}{G}, {T}: Add six {G}. Spend this mana only to cast creature spells or activate abilities of creatures.", &[("Forest", B, 'p'), ("Forest", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Castle Sengir", "Castle Sengir", "{1}, {T}: Add {B}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Castle Sengir (2)", "Castle Sengir", "{2}, {T}: Add {U} or {R}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Cave of Temptation", "Cave of Temptation", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Celestial Prism", "Celestial Prism", "{2}, {T}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Ceta Disciple", "Ceta Disciple", "{G}, {T}: Add one mana of any color.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Chromatic Star", "Chromatic Star", "{1}, {T}, Sacrifice ~: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/BF"),
    ("m/Coal Golem", "Coal Golem", "{3}, Sacrifice ~: Add {R}{R}{R}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Conduit Pylons", "Conduit Pylons", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Cormela, Glamour Thief", "Cormela, Glamour Thief", "{1}, {T}: Add {U}{B}{R}. Spend this mana only to cast instant and/or sorcery spells.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Crosis's Attendant", "Crosis's Attendant", "{1}, Sacrifice ~: Add {U}{B}{R}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Crossroads Candleguide", "Crossroads Candleguide", "{2}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Crypt of Agadeem", "Crypt of Agadeem", "{2}, {T}: Add {B} for each black creature card in your graveyard.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Crypt of the Eternals", "Crypt of the Eternals", "{1}, {T}: Add {U}, {B}, or {R}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Crystal Grotto", "Crystal Grotto", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Crystal Quarry", "Crystal Quarry", "{5}, {T}: Add {W}{U}{B}{R}{G}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Daily Bugle Building", "Daily Bugle Building", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Darigaaz's Attendant", "Darigaaz's Attendant", "{1}, Sacrifice ~: Add {B}{R}{G}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Darkwater Catacombs", "Darkwater Catacombs", "{1}, {T}: Add {U}{B}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Desolate Mire", "Desolate Mire", "{1}, {T}: Add {W}{B}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Dimir Signet", "Dimir Signet", "{1}, {T}: Add {U}{B}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Dreadship Reef", "Dreadship Reef", "{1}, Remove X storage counters from ~: Add X mana in any combination of {U} and/or {B}.", &[("Wastes", B, 'p')], &[("member", "storage", 2)], "Metallic Sliver", true, "tt/BF"),
    ("m/Dromar's Attendant", "Dromar's Attendant", "{1}, Sacrifice ~: Add {W}{U}{B}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Energy Refractor", "Energy Refractor", "{2}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Evendo, Waking Haven", "Evendo, Waking Haven", "12+ | {G}, {T}: Add {G} for each creature you control.", &[("Forest", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Farrelite Priest", "Farrelite Priest", "{1}: Add {W}. If this ability has been activated four or more times this turn, sacrifice ~ at the beginning of the next end step.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Ferrous Lake", "Ferrous Lake", "{1}, {T}: Add {U}{R}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Fetid Heath", "Fetid Heath", "{W/B}, {T}: Add {W}{W}, {W}{B}, or {B}{B}.", &[("Plains", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Fire Sprites", "Fire Sprites", "{G}, {T}: Add {R}.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Fire-Lit Thicket", "Fire-Lit Thicket", "{R/G}, {T}: Add {R}{R}, {R}{G}, or {G}{G}.", &[("Mountain", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Flooded Grove", "Flooded Grove", "{G/U}, {T}: Add {G}{G}, {G}{U}, or {U}{U}.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Foraging Wickermaw", "Foraging Wickermaw", "{1}: Add one mana of any color. ~ becomes that color until end of turn. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Fungal Reaches", "Fungal Reaches", "{1}, Remove X storage counters from ~: Add X mana in any combination of {R} and/or {G}.", &[("Wastes", B, 'p')], &[("member", "storage", 2)], "Metallic Sliver", true, "tt/BF"),
    ("m/Giant's Boulder", "Giant's Boulder", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Golden Egg", "Golden Egg", "{1}, {T}, Sacrifice ~: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Golgari Signet", "Golgari Signet", "{1}, {T}: Add {B}{G}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Graven Cairns", "Graven Cairns", "{B/R}, {T}: Add {B}{B}, {B}{R}, or {R}{R}.", &[("Swamp", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Gravestone Strider", "Gravestone Strider", "{1}: Add one mana of any color. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Great Hall of the Citadel", "Great Hall of the Citadel", "{1}, {T}: Add two mana in any combination of colors. Spend this mana only to cast legendary spells.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Gruul Signet", "Gruul Signet", "{1}, {T}: Add {R}{G}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Guild Globe", "Guild Globe", "{2}, {T}, Sacrifice ~: Add two mana of different colors.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Guildmages' Forum", "Guildmages' Forum", "{1}, {T}: Add one mana of any color. If that mana is spent on a multicolored creature spell, that creature enters with an additional +1/+1 counter on it.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/H.E.R.B.I.E., Lovable Robot", "H.E.R.B.I.E., Lovable Robot", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Hall of Oracles", "Hall of Oracles", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Hall of Tagsin", "Hall of Tagsin", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Heap Gate", "Heap Gate", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Helionaut", "Helionaut", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Henge of Ramos", "Henge of Ramos", "{2}, {T}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Hidden Grotto", "Hidden Grotto", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Hydro-Channeler", "Hydro-Channeler", "{1}, {T}: Add one mana of any color. Spend this mana only to cast an instant or sorcery spell.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Implements of Sacrifice", "Implements of Sacrifice", "{1}, {T}, Sacrifice ~: Add two mana of any one color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Initiates of the Ebon Hand", "Initiates of the Ebon Hand", "{1}: Add {B}. If this ability has been activated four or more times this turn, sacrifice ~ at the beginning of the next end step.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Interplanar Beacon", "Interplanar Beacon", "{1}, {T}: Add two mana of different colors. Spend this mana only to cast planeswalker spells.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Izzet Signet", "Izzet Signet", "{1}, {T}: Add {U}{R}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Jenson Carthalion, Druid Exile", "Jenson Carthalion, Druid Exile", "{5}, {T}: Add {W}{U}{B}{R}{G}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Jeskai Devotee", "Jeskai Devotee", "{1}: Add {U}, {R}, or {W}. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Kaleidostone", "Kaleidostone", "{5}, {T}, Sacrifice ~: Add {W}{U}{B}{R}{G}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Knotvine Mystic", "Knotvine Mystic", "{1}, {T}: Add {R}{G}{W}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Koskun Keep", "Koskun Keep", "{1}, {T}: Add {R}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Koskun Keep (2)", "Koskun Keep", "{2}, {T}: Add {B} or {G}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Llanowar Envoy", "Llanowar Envoy", "{1}{G}: Add one mana of any color.", &[("Forest", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Loot, the Pathfinder", "Loot, the Pathfinder", "Exhaust — {G}, {T}: Add three mana of any one color.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Magus of the Coffers", "Magus of the Coffers", "{2}, {T}: Add {B} for each Swamp you control.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Mana Cylix", "Mana Cylix", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Mana Prism", "Mana Prism", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Manaforge Cinder", "Manaforge Cinder", "{1}: Add {B} or {R}. Activate no more than three times each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Mardu Devotee", "Mardu Devotee", "{1}: Add {R}, {W}, or {B}. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Molten Slagheap", "Molten Slagheap", "{1}, Remove X storage counters from ~: Add X mana in any combination of {B} and/or {R}.", &[("Wastes", B, 'p')], &[("member", "storage", 2)], "Metallic Sliver", true, "tt/BF"),
    ("m/Mossfire Valley", "Mossfire Valley", "{1}, {T}: Add {R}{G}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Mox Lotus", "Mox Lotus", "{100}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Mystic Gate", "Mystic Gate", "{W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.", &[("Plains", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Mystic Skull", "Mystic Skull", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Necra Disciple", "Necra Disciple", "{G}, {T}: Add one mana of any color.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Nomadic Elf", "Nomadic Elf", "{1}{G}: Add one mana of any color.", &[("Forest", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Nykthos, Shrine to Nyx", "Nykthos, Shrine to Nyx", "{2}, {T}: Choose a color. Add an amount of mana of that color equal to your devotion to that color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Omni-Cheese Pizza", "Omni-Cheese Pizza", "{1}, {T}, Sacrifice ~: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Opal Palace", "Opal Palace", "{1}, {T}: Add one mana of any color in your commander's color identity. If you spend this mana to cast your commander, it enters with a number of additional +1/+1 counters on it equal to the number of times it's been cast from the command zone this game.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Orb of Dragonkind", "Orb of Dragonkind", "{1}, {T}: Add two mana in any combination of colors. Spend this mana only to cast Dragon spells or activate abilities of Dragons.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Orb of Origin", "Orb of Origin", "{2}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Orochi Leafcaller", "Orochi Leafcaller", "{G}: Add one mana of any color.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Orzhov Signet", "Orzhov Signet", "{1}, {T}: Add {W}{B}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Overflowing Basin", "Overflowing Basin", "{1}, {T}: Add {G}{U}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Painted Bluffs", "Painted Bluffs", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Petalmane Baku", "Petalmane Baku", "{1}, Remove X ki counters from ~: Add X mana of any one color.", &[("Wastes", B, 'p')], &[("member", "ki", 2)], "Metallic Sliver", true, "tt/BF"),
    ("m/Pili-Pala#tapped", "Pili-Pala", "{2}, {Q}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Planar Nexus", "Planar Nexus", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Prismatic Lens", "Prismatic Lens", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Prismite", "Prismite", "{2}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Prophetic Prism", "Prophetic Prism", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Pyramid of the Pantheon", "Pyramid of the Pantheon", "{2}, {T}: Add one mana of any color. Put a brick counter on ~.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Rafi, Retro Racer", "Rafi, Retro Racer", "{R}, Sacrifice ~: Each player adds {R}{R}{R}.", &[("Mountain", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Rakdos Signet", "Rakdos Signet", "{1}, {T}: Add {B}{R}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Rith's Attendant", "Rith's Attendant", "{1}, Sacrifice ~: Add {R}{G}{W}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Rootrider Faun", "Rootrider Faun", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Rugged Prairie", "Rugged Prairie", "{R/W}, {T}: Add {R}{R}, {R}{W}, or {W}{W}.", &[("Mountain", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Rumble Arena", "Rumble Arena", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Saltcrusted Steppe", "Saltcrusted Steppe", "{1}, Remove X storage counters from ~: Add X mana in any combination of {G} and/or {W}.", &[("Wastes", B, 'p')], &[("member", "storage", 2)], "Metallic Sliver", true, "tt/BF"),
    ("m/Salvaged Manaworker", "Salvaged Manaworker", "{1}: Add one mana of any color. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Satyr Hedonist", "Satyr Hedonist", "{R}, Sacrifice ~: Add {R}{R}{R}.", &[("Mountain", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Scarecrow Guide", "Scarecrow Guide", "{1}: Add one mana of any color. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/School of the Unseen", "School of the Unseen", "{2}, {T}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Sea Scryer", "Sea Scryer", "{1}, {T}: Add {U}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Selesnya Signet", "Selesnya Signet", "{1}, {T}: Add {G}{W}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Selvala, Heart of the Wilds", "Selvala, Heart of the Wilds", "{G}, {T}: Add X mana in any combination of colors, where X is the greatest power among creatures you control.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Shadowblood Ridge", "Shadowblood Ridge", "{1}, {T}: Add {B}{R}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Shimmering Grotto", "Shimmering Grotto", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Shire Scarecrow", "Shire Scarecrow", "{1}: Add one mana of any color. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Signpost Scarecrow", "Signpost Scarecrow", "{2}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Simic Signet", "Simic Signet", "{1}, {T}: Add {G}{U}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Skycloud Expanse", "Skycloud Expanse", "{1}, {T}: Add {W}{U}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Skyshroud Elf", "Skyshroud Elf", "{1}: Add {R} or {W}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Standing Stones", "Standing Stones", "{1}, {T}, Pay 1 life: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Stonework Packbeast", "Stonework Packbeast", "{2}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Study Hall", "Study Hall", "{1}, {T}: Add one mana of any color. When you spend this mana to cast your commander, scry X, where X is the number of times it's been cast from the command zone this game.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Sultai Devotee", "Sultai Devotee", "{1}: Add {B}, {G}, or {U}. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Sungrass Prairie", "Sungrass Prairie", "{1}, {T}: Add {G}{W}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Sunken Palace", "Sunken Palace", "{1}{U}, {T}, Exile seven cards from your graveyard: Add {U}. When you spend this mana to cast a spell or activate an ability, copy that spell or ability. You may choose new targets for the copy.", &[("Island", B, 'p'), ("Wastes", B, 'p'), ("Grizzly Bears", G, 'f'), ("Grizzly Bears", G, 'f'), ("Grizzly Bears", G, 'f'), ("Grizzly Bears", G, 'f'), ("Grizzly Bears", G, 'f'), ("Grizzly Bears", G, 'f'), ("Grizzly Bears", G, 'f')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Sunken Ruins", "Sunken Ruins", "{U/B}, {T}: Add {U}{U}, {U}{B}, or {B}{B}.", &[("Island", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Sunscorched Divide", "Sunscorched Divide", "{1}, {T}: Add {R}{W}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Surveillance Room", "Surveillance Room", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Talon Gates of Madara", "Talon Gates of Madara", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Tarkir Omenpath", "Tarkir Omenpath", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Tarnation Vista", "Tarnation Vista", "{1}, {T}: For each color among monocolored permanents you control, add one mana of that color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Temur Devotee", "Temur Devotee", "{1}: Add {G}, {U}, or {R}. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Terrarion", "Terrarion", "{2}, {T}, Sacrifice ~: Add two mana in any combination of colors.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/BF"),
    ("m/The Mycosynth Gardens", "The Mycosynth Gardens", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/The Mystical Archive", "The Mystical Archive", "{1}, {T}: Add two mana in any combination of colors. Spend this mana only to cast spells that aren't from your starting deck.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Three Tree City", "Three Tree City", "{2}, {T}: Choose a color. Add an amount of mana of that color equal to the number of creatures you control of the chosen type.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Three Tree Mascot", "Three Tree Mascot", "{1}: Add one mana of any color. Activate only once each turn.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Treva's Attendant", "Treva's Attendant", "{1}, Sacrifice ~: Add {G}{W}{U}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Twilight Mire", "Twilight Mire", "{B/G}, {T}: Add {B}{B}, {B}{G}, or {G}{G}.", &[("Swamp", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Unknown Event Shores", "Unknown Event Shores", "{1}, {T}: Add 1 mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Unknown Shores", "Unknown Shores", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Urn of Godfire", "Urn of Godfire", "{2}: Add one mana of any color.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Uthros, Titanic Godcore", "Uthros, Titanic Godcore", "12+ | {U}, {T}: Add {U} for each artifact you control.", &[("Island", B, 'p')], &[], "Metallic Sliver", false, "tt/H"),
    ("m/Verdant Eidolon", "Verdant Eidolon", "{G}, Sacrifice ~: Add three mana of any one color.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Vessel of Volatility", "Vessel of Volatility", "{1}{R}, Sacrifice ~: Add {R}{R}{R}{R}.", &[("Mountain", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Viridescent Bog", "Viridescent Bog", "{1}, {T}: Add {B}{G}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Viridian Acolyte", "Viridian Acolyte", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/White Lotus Hideout", "White Lotus Hideout", "{1}, {T}: Add one mana of any color.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Wizards' School", "Wizards' School", "{1}, {T}: Add {U}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Wizards' School (2)", "Wizards' School", "{2}, {T}: Add {W} or {B}.", &[("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Wooded Bastion", "Wooded Bastion", "{G/W}, {T}: Add {G}{G}, {G}{W}, or {W}{W}.", &[("Forest", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Yurlok of Scorch Thrash", "Yurlok of Scorch Thrash", "{1}, {T}: Each player adds {B}{R}{G}.", &[("Wastes", B, 'p')], &[], "Metallic Sliver", true, "tt/BF"),
    ("m/Codie, Vociferous Codex (2)", "Codie, Vociferous Codex", "{4}, {T}: Add {W}{U}{B}{R}{G}. When you next cast a spell this turn, exile cards from the top of your library until you exile an instant or sorcery card with lesser mana value. Until end of turn, you may cast that card without paying its mana cost. Put each other card exiled this way on the bottom of your library in a random order.", &[("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p'), ("Wastes", B, 'p')], &[], "Opt", true, "tt/stk"),
    ("m/Azorius Signet (2)", "Azorius Signet", "{1}, {T}: Add {W}{U}.", &[("Wastes", B, 'p')], &[], "Fugitive Wizard", true, "tt/BF"),
];
