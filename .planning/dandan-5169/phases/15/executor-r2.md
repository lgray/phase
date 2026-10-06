# Phase 15 executor report r2

Mode: implementation/fix (phase mode). BASE_SHA = PHASE_BASE_SHA = 236a35f0b5189161a7513d38261751b0238f5069. START_SHA = ad382d02f3e084f3f04f7dbfe6846662b09ba246. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Start: clean, HEAD == START_SHA, nothing staged. End: HEAD == START_SHA, nothing staged, unstaged delta = effects/mod.rs, effects/change_zone.rs, effects/scoped_library_search.rs, filter.rs (all in the 8-path scope.nul). PREPARATORY evidence only.

## Finding 1 (fixed)
`shared_library_wheel_split`'s `plain` is now exactly `scoped_library_search::has_no_resolution_riders` on a copy of the node with `sub_ability` cleared, for every head move, the shuffle and the draw node. Per the approval, `optional_player.is_none()` moved into `has_no_resolution_riders` itself (the local conjunct is gone); its unit test gained an `optional_player` row, red before the predicate edit (`FAIL ... optional_player`), green after.
Callers of `has_no_resolution_riders` (grep): (1) `draw.rs::plan_simultaneous_draw` (Phase 14 dealer seat gate); (2) `scoped_library_search.rs` delivery shape gate (line ~134); (3) the wheel gate; (4) two unit tests. The predicate only refuses more, and only a node with `optional_player` set; no accepted shape flipped: full engine nextest 33039/33039 pass, including scoped_library_search and the dealer rows.
Draw node: probed earlier (temporary test, removed): a bare scoped draw bound per seat is planned by the dealer, the same draw with `repeat_for` is refused.

Hostile rows added to `shared_library_wheel_split_refuses_every_other_shape` (10 -> 17): repeat_for shuffle, repeat_for move, repeat_for draw, else_ability shuffle, optional_player move, duration shuffle, forward_result move (the extra member). The 10 shipped rows and the accept row are unchanged and green.
Revert leg (`plain` restored to `!optional && condition.is_none()`): `p15-r2-revert.log`: "not the wheel class: [repeat_for shuffle, repeat_for move, repeat_for draw, else_ability shuffle, optional_player move, duration shuffle, forward_result move]" (all 7 new rows red; accept row and `retags_only_the_shuffle` green = live instrument). File restored byte-identical from backup afterwards.

## Finding 2 (done)
`zone_axis_admits` is now `pub(crate)` and `change_zone.rs::scope_player_for_member` is deleted; `resolve_all` calls `zone_axis_admits(state, (zone != Battlefield).then_some(zone), player, |seat| change_zone_all_player_scope_member_matches(obj, seat, &origin_zones))`. Equivalence: non-battlefield, owner == player admits at the first probe; owner sharing the container admits via the seat scan (old code tested owner == owner); otherwise both refuse. Battlefield: licensed `None`, so controller == player alone, as before. Measured: Dandan Cane row, Standard twin (`separate_graveyards_move_only_the_activators_cards`) and the rest of the full suite pass.

## Verification (PREPARATORY)
- `cargo fmt --all -- --check` rc 0; `cargo clippy --workspace --all-targets -- -D warnings` rc 0.
- Targeted nextest (wheel split, scoped_library_search, dandan_wheel_split, dandan_scoped_pile_mass_move, all_player_library_wheel, has_no_resolution_riders, dandan_simultaneous_draw filter): 48/48 pass. Full `cargo nextest run -p phase-engine`: 33039/33039 pass (`p15-r2-full.log`).
- `node scripts/check-protocol-version.mjs` rc 0. No types/ change.

## Coverage / matrix / CR
Production path unchanged for the accepted class (W1 etc. still green); the change only refuses more shapes, so no new production seam; rows are the gate's own unit shape tests, paired with the accepted base chain. No CR number added or changed. No parser file touched. No DEFERRED rows, no stop-and-return items.

## Risks
Draw node now also refuses riders at the gate (previously only optional/condition), which keeps the sequential wheel for such shapes; none exist on printed cards.
