# Phase 10 executor report r2 (implementation/fix, phase mode)

Mode implementation/fix. BASE_SHA = START_SHA = 45a9229a766e98d04782422b0d7aa07810a4fcf2. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan. Clean at start; end: HEAD == START_SHA, nothing staged, delta = `crates/engine/src/game/effects/bounce.rs` only. Results are PREPARATORY, not completion evidence.

## Change
Test `game::effects::bounce::dandan_read_sweep_tests::chosen_non_canonical_player_returns_a_card_from_the_shared_pile_graveyard` (test body + doc line only).
Card: Skullwinder (client/public/card-data.json): "...then choose an opponent. That player returns a card from their graveyard to their hand." In Dandan the chosen player's graveyard is the one shared pile (Phase 10 axis collapse), so both pile cards (P0-owned `own`, P1-owned `theirs`) are legal.
New assertions: (1) resolution surfaces `EffectZoneChoice` for P1 offering exactly {own, theirs} (positive reach-guard); (2) `apply(P1, SelectCards{theirs})` through the real handler; (3) theirs in players[1].hand, pile left = [own].

## Discrimination
Mutation on filter.rs `zone_axis_admits` (`licensed` forced to None = single-seat comparison, i.e. collapse reverted): test red at the new guard, `expected EffectZoneChoice over the pile, got Priority { player: PlayerId(0) }` (p10r2-mut.log). Restored from copy, sha256 9194bf77...f2b8 matches pre-mutation, `touch`ed, `git status` shows only bounce.rs.

## Verification (PREPARATORY)
- `cargo fmt --all -- crates/engine/src/game/effects/bounce.rs`.
- `cargo nextest run -p phase-engine -E 'effects::bounce | game::filter | game::quantity | dandan_filter_owner_axis | dandan_scoped_counts'`: 518 run, 518 pass (bounce 22, filter 205, quantity 269, owner_axis 8, scoped_counts 14). Log p10r2-nextest.log.
- `cargo clippy --workspace --all-targets -- -D warnings`: rc 0 (p10r2-clippy.log).
- Parser gate: n/a. CR gate: no CR number added (608.2d, 400.1 retained, both present in docs/MagicCompRules.txt).
- Whole-engine suite not rerun; r1 had this as its sole failure.

## Matrix row
Seam: bounce non-targeted graveyard branch, `Typed{controller ChosenPlayer, Owned{ChosenPlayer}, InZone Graveyard}`; entry `resolve()` -> `matching.len()` match arm `_` (2 candidates, not the degenerate 1-arm); authority = chosen player P1, bound at evaluation, live; consumer `EffectZoneChoice` intake. Hostile fixture: cross-owner pile cards. DEFERRED(phase 11): which hand receives a pile card owned by the other seat (test selects the chooser-owned card).
