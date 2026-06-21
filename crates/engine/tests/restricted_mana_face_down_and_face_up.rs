//! Spend-restriction cluster: face-down casts and turn-face-up / door-unlock
//! special actions on produced mana.
//!
//! Cards in the cluster:
//!   - Creeping Peeper — "{T}: Add {U}. Spend this mana only to cast an
//!     enchantment spell, unlock a door, or turn a permanent face up."
//!   - Overgrown Zealot — "{T}: Add two mana of any one color. Spend this mana
//!     only to turn permanents face up."
//!   - Tin Street Gossip — "{T}: Add {R}{G}. Spend this mana only to cast
//!     face-down spells or to turn creatures face up."
//!
//! CR 106.6 (restricted mana spend) + CR 708.4 (face-down spell) + CR 116.2b /
//! CR 702.37e (turn-face-up special action) + CR 116.2m / CR 709.5e (door
//! unlock).
//!
//! These tests drive the real mana-payment route — `ManaPool::spend_for` with a
//! `PaymentContext` — proving the produced unit is CONSUMED for a legal spend
//! and WITHHELD for an illegal one. The cast pipeline (`can_pay_for_spell` /
//! `pay_cost_*`) and the special-action pipeline
//! (`pay_special_action_mana_cost`) both flow through this exact
//! `ManaRestriction::allows` authority, so a unit test on `spend_for` exercises
//! the same decision a full `apply()` cast/unlock makes.
//!
//! Revert-proof: each assertion flips if the corresponding gate is reverted —
//! see the per-test notes.

use engine::types::identifiers::ObjectId;
use engine::types::mana::{
    ManaPool, ManaRestriction, ManaType, ManaUnit, PaymentContext, SpecialAction, SpellMeta,
};

fn spell(types: &[&str], is_face_down: bool) -> SpellMeta {
    SpellMeta {
        types: types.iter().map(|s| s.to_string()).collect(),
        is_face_down,
        ..SpellMeta::default()
    }
}

/// Tin Street Gossip: "spend this mana only to cast face-down spells" — the
/// `OnlyForFaceDownSpell` half. Drives `spend_for`: a face-down cast consumes
/// the unit; a normal face-up cast withholds it.
///
/// Revert-proof: if `allows_spell` for `OnlyForFaceDownSpell` were changed to
/// ignore `meta.is_face_down` (e.g. return `true`), the face-up assertion below
/// would flip — the unit would be wrongly consumed.
#[test]
fn face_down_spell_mana_consumes_for_face_down_cast_only() {
    let source = ObjectId(1);
    let make_pool = || {
        let mut pool = ManaPool::default();
        pool.add(ManaUnit::new(
            ManaType::Red,
            source,
            false,
            vec![ManaRestriction::OnlyForFaceDownSpell],
        ));
        pool
    };

    // LEGAL: a face-down cast (morph/disguise/cloak) — the unit is consumed.
    let face_down = spell(&["Creature"], true);
    let mut pool = make_pool();
    let spent = pool.spend_for(ManaType::Red, &PaymentContext::Spell(&face_down));
    assert!(
        spent.is_some(),
        "face-down-only mana must pay a face-down cast"
    );
    assert_eq!(pool.total(), 0, "the unit must be consumed");

    // ILLEGAL: a normal face-up cast — the unit is withheld, pool intact.
    let face_up = spell(&["Creature"], false);
    let mut pool = make_pool();
    let spent = pool.spend_for(ManaType::Red, &PaymentContext::Spell(&face_up));
    assert!(
        spent.is_none(),
        "face-down-only mana must not pay a normal face-up cast"
    );
    assert_eq!(pool.total(), 1, "the unit must remain unspent");
}

/// Creeping Peeper: "spend this mana only to cast an enchantment spell, unlock a
/// door, or turn a permanent face up" — the runtime
/// `Any([SpellType("Enchantment"), OnlyForSpecialAction(UnlockDoor),
/// OnlyForSpecialAction(TurnFaceUp)])`. Drives `spend_for`: an enchantment cast
/// consumes the {U}; a non-enchantment cast withholds it.
///
/// Revert-proof: if the `SpellType("Enchantment")` branch were dropped from the
/// disjunction, the enchantment cast would no longer be payable and its
/// assertion would flip.
#[test]
fn creeping_peeper_mana_consumes_for_enchantment_not_creature() {
    let source = ObjectId(2);
    let restriction = ManaRestriction::OnlyForAny(vec![
        ManaRestriction::OnlyForSpellType("Enchantment".to_string()),
        ManaRestriction::OnlyForSpecialAction(SpecialAction::UnlockDoor),
        ManaRestriction::OnlyForSpecialAction(SpecialAction::TurnFaceUp),
    ]);
    let make_pool = || {
        let mut pool = ManaPool::default();
        pool.add(ManaUnit::new(
            ManaType::Blue,
            source,
            false,
            vec![restriction.clone()],
        ));
        pool
    };

    // LEGAL: an enchantment cast — the {U} is consumed.
    let enchantment = spell(&["Enchantment"], false);
    let mut pool = make_pool();
    let spent = pool.spend_for(ManaType::Blue, &PaymentContext::Spell(&enchantment));
    assert!(
        spent.is_some(),
        "Creeping Peeper's {{U}} must pay an enchantment spell"
    );
    assert_eq!(pool.total(), 0, "the {{U}} must be consumed");

    // ILLEGAL: a (non-enchantment) creature cast — the {U} is withheld.
    let creature = spell(&["Creature"], false);
    let mut pool = make_pool();
    let spent = pool.spend_for(ManaType::Blue, &PaymentContext::Spell(&creature));
    assert!(
        spent.is_none(),
        "Creeping Peeper's {{U}} must not pay a non-enchantment spell"
    );
    assert_eq!(pool.total(), 1, "the {{U}} must remain unspent");
}

/// Creeping Peeper's {U} pays the door-unlock special action (CR 116.2m), the
/// branch a Room's unlock cost routes through
/// (`PaymentContext::SpecialAction(UnlockDoor)`).
///
/// Revert-proof: if the `OnlyForSpecialAction(UnlockDoor)` branch were dropped,
/// this assertion would flip — the unit would no longer pay an unlock.
#[test]
fn creeping_peeper_mana_pays_door_unlock_special_action() {
    let source = ObjectId(3);
    let mut pool = ManaPool::default();
    pool.add(ManaUnit::new(
        ManaType::Blue,
        source,
        false,
        vec![ManaRestriction::OnlyForAny(vec![
            ManaRestriction::OnlyForSpellType("Enchantment".to_string()),
            ManaRestriction::OnlyForSpecialAction(SpecialAction::UnlockDoor),
            ManaRestriction::OnlyForSpecialAction(SpecialAction::TurnFaceUp),
        ])],
    ));
    let spent = pool.spend_for(
        ManaType::Blue,
        &PaymentContext::SpecialAction(SpecialAction::UnlockDoor),
    );
    assert!(
        spent.is_some(),
        "Creeping Peeper's {{U}} must pay a door-unlock special action"
    );
    assert_eq!(pool.total(), 0, "the {{U}} must be consumed");
}

/// Overgrown Zealot: "spend this mana only to turn permanents face up" — the
/// `OnlyForSpecialAction(TurnFaceUp)` gate. This special action charges no mana
/// in this engine yet (`game::morph::turn_face_up` flips the permanent for
/// free), so no payment site emits `PaymentContext::SpecialAction(TurnFaceUp)`.
/// The runtime is therefore conservative: the mana is never spendable on any
/// payment context that actually occurs (spell / activation / effect /
/// door-unlock), and is never silently over-permitted.
///
/// This documents the honest-deferred contract: the turn-face-up gate is
/// representable and correctly REJECTS every wrong context, but its positive
/// case awaits routing the morph cost through the special-action payment path.
/// If a future change starts emitting `PaymentContext::SpecialAction(TurnFaceUp)`
/// at a real spend site, the positive assertion below flips from withheld to
/// consumed and this test must gain a positive-payment arm.
#[test]
fn overgrown_zealot_turn_face_up_mana_rejects_every_live_context() {
    let source = ObjectId(4);
    let make_pool = || {
        let mut pool = ManaPool::default();
        // Overgrown Zealot adds two mana of any one color.
        pool.add(ManaUnit::new(
            ManaType::Green,
            source,
            false,
            vec![ManaRestriction::OnlyForSpecialAction(
                SpecialAction::TurnFaceUp,
            )],
        ));
        pool
    };

    // ILLEGAL: a spell cast (even a face-down one) — the unit is withheld.
    let face_down = spell(&["Creature"], true);
    let mut pool = make_pool();
    assert!(
        pool.spend_for(ManaType::Green, &PaymentContext::Spell(&face_down))
            .is_none(),
        "turn-face-up mana must not pay a spell cast"
    );
    assert_eq!(pool.total(), 1);

    // ILLEGAL: an unrelated door-unlock special action — the unit is withheld.
    let mut pool = make_pool();
    assert!(
        pool.spend_for(
            ManaType::Green,
            &PaymentContext::SpecialAction(SpecialAction::UnlockDoor)
        )
        .is_none(),
        "turn-face-up mana must not pay a door unlock"
    );
    assert_eq!(pool.total(), 1);

    // The matching special action would be the only legal context (CR 116.2b);
    // confirm the gate accepts it at the restriction level so the eventual
    // payment wiring is a no-op for this enum.
    assert!(
        ManaRestriction::OnlyForSpecialAction(SpecialAction::TurnFaceUp)
            .allows(&PaymentContext::SpecialAction(SpecialAction::TurnFaceUp))
    );
}
