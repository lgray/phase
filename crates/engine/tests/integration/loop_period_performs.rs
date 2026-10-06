//! CR 732.2a + CR 608.1: an offer's confirmed period performs its board's base period, compared on
//! the printed identities of the abilities and spells one replayed cycle resolves.

/// Whether `performed` is `base` begun at some resolution of it.
pub(crate) fn same_up_to_rotation(performed: &[String], base: &[&str]) -> bool {
    performed.len() == base.len()
        && (0..base.len().max(1)).any(|start| {
            performed.iter().map(String::as_str).eq(base
                .iter()
                .cycle()
                .skip(start)
                .take(base.len())
                .copied())
        })
}

/// Board A's base period, in the order the cards resolve it from Abdel Adrian's enters trigger.
pub(crate) const BOARD_A_PERIOD: [&str; 5] = [
    "Abdel Adrian, Gorion's Ward",
    "Animate Dead",
    "Altar of the Brood",
    "Animate Dead",
    "Altar of the Brood",
];

#[test]
fn same_up_to_rotation_accepts_a_rotation_and_rejects_a_swap() {
    let rotated: Vec<String> = [
        "Animate Dead",
        "Altar of the Brood",
        "Abdel Adrian, Gorion's Ward",
        "Animate Dead",
        "Altar of the Brood",
    ]
    .map(String::from)
    .to_vec();
    assert!(same_up_to_rotation(&rotated, &BOARD_A_PERIOD));
    let mut swapped = rotated.clone();
    swapped.swap(0, 1);
    assert!(!same_up_to_rotation(&swapped, &BOARD_A_PERIOD));
    assert!(!same_up_to_rotation(&rotated[1..], &BOARD_A_PERIOD));
}
