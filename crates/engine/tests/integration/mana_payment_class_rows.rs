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
