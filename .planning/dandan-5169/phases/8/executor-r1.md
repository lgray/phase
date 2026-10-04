# Phase 8 executor report r1 (S2b-1 read sweep of `crates/engine/src/game/`)

MODEL: claude-sonnet-5-5. Not committed. Implementation worktree W = `/home/lgray/vibe-coding/dandan-run/wt-dandan`.

## Verdict
Production sweep done for all 45 paths; clippy clean; all new rows green in default fixture mode; red at base shown per row. Seven plan premises were measured false and replaced (section 8). No stop-and-return item.

## 1. Diff summary
- 47 modified tracked files (45 production, `tests/integration/main.rs` mod line, `tests/fixtures/integration_cards.json.gz`) plus new `crates/engine/tests/integration/dandan_read_sweep.rs` (1783 lines, 46 integration rows).
- `git diff --shortstat -- crates`: +998 / -162 (about 600 of the insertions are inline `#[cfg(test)] mod dandan_read_sweep_tests` rows in 14 production files).
- Classes applied: S/S-guard (effect handlers, enumerators, statics), L (`casting.rs` `non_owner_graveyard_ids`, shared by `non_owner_graveyard_play_from_exile_grants` and `non_owner_graveyard_permission_objects`), Q (`GraveyardSize` moved into `candidate_player_scalar_with_state`), W (`normalize_recast_frame` via `_mut`), R (`choose_from_zone`, `restrictions::player_zone_ids`).
- Corollaries: `morph.rs::top_library_object` owner gate dropped (plan 4.3); `clash.rs::top_card_of_library` `.last()` to `.front()` via `library_of` (CR 701.30a).

## 2. Worktree record
- HEAD == START_SHA `e0d9cb1208d24dc375909e8b62d52d5fcd379299`; nothing staged.
- Changed set (tracked diff plus the untracked test file) equals `phases/8/scope.nul` exactly (48 paths; sha256 `49a84162d795aedc89561aadf191c1217a899cc775c697ddc54cec48efe80ec2`, matches the corrected value). `diff` of the two sorted lists: identical.
- Fixture: `python3 scripts/gen-test-fixture.py` then `--check`: `ok: fixture covers all 4964 referenced cards with canonical gzip bytes`.

## 3. Preparatory results (all in W with the isolated env, no Tilt)
- `cargo clippy -p phase-engine --all-targets -- -D warnings`: clean (final run after the last code edit, `Finished`).
- Default fixture mode (no `FORGE_TEST_FULL_DB`): `nextest --test integration dandan_read_sweep`: 46/46 pass. Inline modules `--lib dandan_read_sweep_tests`: 18/18 pass.
- V14: swept-module existing tests, `--lib -E 'test(/^game::(casting|casting_costs|cost_payability|costs|derived|derived_views|engine|engine_debug|engine_payment_choices|layers|morph|quantity|replacement|restrictions|visibility|augment)::/) | test(/^game::effects::/)'`: 5512/5512 pass. Integration filter `dandan|stack_entry_node_reach|clash|scry|surveil|cascade|ripple|discover|library|graveyard|top_of_library|reveal`: 1042/1042 pass. Not the full suite (brief).
- Base-red (tree `base-p8` = `git archive` of START_SHA plus the new test files and inline modules, own target dir). Integration (47 rows as run at base, 41 red): the six green at base: V3 x4 (Brainstorm, Surgical Bay, Metamorphose, Accumulated Knowledge: positive controls, Phase 6 already correct), `v4_explore_over_an_empty_pile` (empty-pile control), and the Aphemia probe row, which was then deleted because it is green at base (its exile path is already pile-aware); the final file has 46 rows, 41 of them red at base. Inline: 15 of 17 red; the two green are the Standard-format controls (`non_owner_grants_in_a_per_seat_format...`, `zone_count_conditions_in_a_per_seat_format...`). The `non_owner_graveyard_ids` dedup unit cannot compile at base (new helper) and is excluded there.
- Reproduce: `base-p8/t5.log` (lib), `base-p8/t6.log` (integration), W logs `phases/8/t17.log`, `t18.log`, `clippy2.log`.

## 4. Parser gate
No parser or Oracle-text change; no `parser/` path in scope. Coverage unchanged; gate not applicable.

## 5. Production-path coverage map (row that goes red when the file is reverted)
Single-file revert = file alone restored to START_SHA in tree `rev-p8` (all other changes kept); batch = several files reverted together, row attributed by mapping (batch results in `phases/8/revs2.log`, `revs3.log`; single in `revs.log`).

| File | Row (revert evidence) |
|---|---|
| casting.rs | single: organ_grinder, think_twice, crucible, fblthp, future_sight; units V7c (4 cases, dedup), Lurrus slot demand |
| casting_costs.rs | single: organ_grinder |
| cost_payability.rs | single: altar, baron, organ_grinder, manakin; unit exile_cost_enumerators |
| costs.rs | unit payer_scoped_graveyard_exile (red at base); NO integration row reaches it (single revert: all green) |
| derived.rs | single: future_sight |
| derived_views.rs, engine_debug.rs | batch: v10_debug_views_and_mill |
| visibility.rs | batch: sphinx_of_jwar_isle |
| layers.rs | batch: mul_daya |
| morph.rs | batch: soul_summons |
| quantity.rs | batch: eldritch_pact |
| replacement.rs | batch: golgari_thug |
| engine_payment_choices.rs | batch: deep_spawn, manakin; units top_library_exile_cost, unless_mill |
| restrictions.rs | units zone_count_conditions (+ Standard control) |
| engine.rs | unit recast_frame_prunes |
| effects/mod.rs | unit candidate_graveyard_size |
| effects/scry.rs | single: opt |
| effects/reveal_top.rs | single: sifter_wurm, parker_luck |
| effects/ripple.rs | single: thrumming_stone |
| effects/clash.rs | single: v15 Standard and pile rows |
| effects/put_on_top.rs | single: crown_of_convergence |
| effects/rad_counters.rs | single: mirelurk_queen (asserts life 18 and rad 0, not only the mill) |
| effects/cascade, dig, discover, exile_from_top_until, exile_top, explore, forage, heist, manifest_dread, mill | batch B1: sweet_gum, telling_time, carnosaur, tasha, reckless_impulse, branchwalker, corpseberry, grave_expectations, ethrimik, mental_note/predict/manakin |
| effects/reveal_until, search_library, seek, separate_piles, surveil, collect_evidence, choose_from_zone | batch B2: treasure_hunt, doomsday, nesting_instinct, fact_or_fiction, consider, kylox, gruesome_menagerie/crown |
| effects/stack_reach.rs | unit surveil_moves_come_from_the_shared_pile (red at base); see 8.3 |
| effects/bounce.rs | unit chosen_non_canonical_player_returns... (red at base); see 8.1 |
| augment.rs, effects/choose_card.rs, exile_face_down_pile.rs, free_cast_from_zones.rs | inline unit rows (red at base) |

Batch attribution is by mapping, not isolation, for B1/B2/B3 files; a file whose mapped row stayed green in its batch would have been an orphan: costs.rs, rad_counters.rs and put_on_top.rs were (B2/single) and were resolved above; no other orphan.

## 6. Maintainer-simulation matrix (final-form rows only)
- V15a: Standard clash, two library orderings swapped end-for-end: `Clash` event `(Some(4), Some(0), Won)` and `(Some(0), Some(4), Lost)`; red at base (bottom read). V15b Dandan, caster either seat: reveal is the pile top, `controller_mana_value == Some(4)`; red at base for P1.
- Every pile row runs the cases (shared, P1), (shared, P0), (Standard, P1): P0 and Standard legs are green at base (positive control), P1-shared red.
- Hostile fixtures: mixed-owner pile read by P1 (V6/V7), `morph` top card not owned by the reading seat (Soul Summons), duplicate-visit L site with P1 as caller (dedup unit), unknown seat not constructible.

## 7. CR gate
Every CR number in added lines (code and tests) grepped in `docs/MagicCompRules.txt`: 118.12, 118.3, 305.1, 400.1, 400.3, 404.1, 406.3, 406.6, 601.2a, 608.2d, 701.17b, 701.25a, 701.30a, 701.30d, 701.40a, 701.58a, 728.1 all present. Subject check: two annotations of mine named the wrong rule (110.4 is permanent types here, 400.7 not the claim) and were corrected to 601.2a and 400.1; the pre-existing 110.4 annotations on neighbouring casting lines were left alone.

## 8. Plan premises found false (all measured, replaced inside scope)
1. V6g Skullwinder: real card parses `ChangeZone` (not `Bounce`); the row is green at base for every seat (jq: no card in `card-data.json` carries a `Bounce` effect with a graveyard `InZone` target). The `bounce.rs` graveyard branch has no real-card reach; covered by the inline unit.
2. V6e/V6f Krosan Avenger, Kindly Stranger: their conditions parse to the Phase 10 deferred quantity arms; census of `ParsedCondition::Zone*` over real cards found 0. Replaced by unit rows in `restrictions.rs` with a Standard control.
3. V16 Swallowed by Leviathan: no integration row can discriminate. `moved_matcher` matches every `ZoneChange`, so any active replacement empties the Counter answer via `replacement_may_come_to_apply` both at base and after; without one, the answer is named at both. Also its unless cost reads the deferred raw `GraveyardSize` arm (P1-shared cost is {0}). Replaced by a unit row calling `instruction_writes` directly (Rest-in-Peace-style replacement on the battlefield, non-canonical surveil; red at base).
4. V6b Treetop Sentries: its optional forage resolves via the already pile-aware resolution payment path (green at base). Replaced by Corpseberry Cultivator (the only real card with `Effect::Forage`).
5. V6c Conspiracy Unraveler: the alternative-cost flow auto-pays and never surfaced `CollectEvidenceChoice` through the harness cast. Replaced by Kylox's Voltstrider activation (same `collect_evidence` handler).
6. V5h Parker Luck and Mirelurk Queen: first versions passed at base (assertion did not touch the swapped read); now assert per-seat `CardsRevealed` events and life/rad outcomes.
7. V8c Gruesome Menagerie parses only the MV1 choice; row asserts the MV1 return. Crown of Convergence parses "choose any 1 card from the library to bottom"; row asserts the offered set equals the pile.

## 9. Judgement calls
- `casting.rs::non_owner_graveyard_permission_objects` is an L-class site not in the plan; it shares the new `non_owner_graveyard_ids` helper (flagged in the diff).
- `derived.rs` `reveal_top_all` left raw (F per plan, `revealed_cards` is a set so duplicates would be harmless but the plan classes it F).
- Census (V13) at working tree: residual `.library`/`.graveyard` lines in `game/` are exactly the plan's F set (`derived` reveal_top_all, `costs` unrestricted, `engine` has_libraries, `targeting` x4, `triggers` x1, `visibility` x2) plus the D set (`quantity` x5) plus Phase 6 files' own kept-raw writes (`engine_resolution_choices`, `mulligan`, `deck_loading`). Command: working-tree variant of `count_reads_lines.py` (reads files, not git revisions) with `'\.(library|graveyard)\b(?!\s*\()'`.

## 10. Stop-and-return items
None. No out-of-list compiler-forced, registration or comment-only site.

## 11. Deviations from the brief
- Brief mentioned scope.nul sha `e300...`; corrected list used per coordinator message.
- Rows added beyond plan list: `v10` mill, V9 (Deep Spawn, Manakin and Millikin, Eldritch Pact, Golgari Thug), V8 Fblthp/Soul Summons/Mul Daya, units for `costs.rs`, `bounce.rs`, `stack_reach.rs` (to cover orphan files).

## 12. Risks
- Batch revert attribution (B1/B2/B3) is mapping-based; each file in a batch has a distinct mapped row and all mapped rows flipped.
- V6 enumerators for `costs.rs` and `bounce.rs` and the `stack_reach.rs` surveil arm are covered only by unit rows, no real-card integration path exists (section 8).
- `quantity.rs` deferred arms and owner/controller-axis collapses stay raw (Phase 10); Skullwinder-class and Swallowed-by-Leviathan P1-shared behaviors depend on them.
