# MSH-F Executor Report — Cosmic Cube + Hawkeye

Implemented & verified in `wt-msh-f` (branch `feat/msh-f-cosmic-cube-hawkeye`, off main 5eca83b8c). All green. **NOT committed/pushed.** (SendMessage tool unavailable in this context → report delivered via this file + final response.)

## 1. Diff summary (9 files, +603/−72) + commit pathspecs
Two sub-plans live in **disjoint** file sets (two pathspec commits) and are independently compilable (`CastPermissionConstraint.value` was ALREADY `QuantityExpr` pre-change).

**COMMIT A — Cosmic Cube (parser-only):**
- `crates/engine/src/parser/oracle_effect/mod.rs` (+210) — `many_till` import; refactor `parse_cast_permission_constraint` → `parse_beseech_mv_constraint` (existing body, unchanged) `.or_else()` `parse_with_mana_value_constraint` (NEW nom-only); A0/A1 tests.

**COMMIT B — Hawkeye field-lift (8 files):**
- `crates/engine/src/types/ability.rs` (+8) — `DamageModification::Plus { value: u32 }` → `{ value: QuantityExpr }`.
- `crates/engine/src/game/replacement.rs` (+221) — resolver Plus arm resolves the QuantityExpr (sentinel-aware controller); 7 test migrations; B2 runtime test.
- `crates/engine/src/game/effects/add_target_replacement.rs` (+35) — `freeze_damage_modification_x` matches `Fixed{0}`; 2 test migrations.
- `crates/engine/src/parser/oracle_replacement.rs` (+169) — new `map_opt(preceded(tag("plus x, where x is "), rest), parse_cda_quantity)` arm BEFORE freeze arm; `Fixed`-wrap numeric/freeze arms; B1/B3 + 4 migrations.
- `crates/engine/src/parser/oracle_trigger.rs` (+4) — 1 assertion migration.
- `crates/engine/tests/{taii_wakeen,i_call_for_slaughter,rankle_and_torbran}.rs` (+28) — integration migrations + `QuantityExpr` imports.

**No card-data/allowlist "enable" edit needed:** the `DynamicQty` swallow detector marker list (swallow_check.rs:1384/1403) already contains `"type":"Ref"` + `"ManaValue"`, so the unsupported→supported flip auto-occurs in main on card-data regen.

## 2. Verification (cargo run directly — separate target dir)
- `cargo fmt --all` → OK
- `cargo clippy -p engine --all-targets -- -D warnings` → exit 0, zero warnings
- `cargo test -p engine` → **15591 passed, 0 failed** (lib+integration+doc)
- `cargo build -p mtgish-import` → OK (dormant; only references the separate `QuantityModification::Plus`)

## 3. Parser diff gate
- Inline string-dispatch grep on added parser lines → EMPTY → PASS
- `./scripts/check-parser-combinators.sh` → exit 0 → PASS
Pure nom (`many_till`/`alt`/`value`/`tag`/`map`/`map_opt`/`preceded`/`rest`); NEW-3 resolved via `many_till(anychar, alt((tag,tag)))`.

## 4. Discriminating-test / production-path map
| claim | changed seam | prod entry | test | revert-fail |
|---|---|---|---|---|
| A0 | parse_quantity_ref | parse_quantity_ref | cosmic_cube_aggregate_quantity_returns_trailing_suffix | `rest==" without paying its mana cost"` + Max-power Aggregate |
| A1 direct | parse_with_mana_value_constraint | parse_cast_permission_constraint | cosmic_cube_constraint_parses_dynamic_mana_value_ceiling | `Some(ManaValue{LE,Ref(Aggregate{Max,Power})})` (None on revert) |
| A1 beseech | parse_beseech_mv_constraint | same | beseech_suffix_constraint_unchanged_after_refactor | `is 4 or less`⇒`ManaValue{LE,Fixed{4}}` |
| A1 full | parser pipeline | **parse_oracle_text** | cosmic_cube_full_trigger_carries_dynamic_constraint | trigger→CastFromZone.constraint dynamic (card-data shows `null`) |
| B1 | parse_that_much_damage_offset | parse_replacement_line | damage_hawkeye_plus_dynamic_source_power | `Plus{Ref(Power{Source})}` (card-data shows `value:0`) |
| B2 | damage_done_applier Plus arm | **replace_event** | damage_applier_plus_dynamic_source_power_is_live | power2⇒5, power4⇒7; freeze⇒3 |
| B3 | QuantityExpr Deserialize | serde_json::from_str | damage_modification_plus_legacy_int_deserializes_to_fixed | `{"value":2}`⇒`Fixed{2}` |

**Non-vacuous proof:** my first resolver draft (gating on `state.objects.get(&rid.source)`) made i_call_for_slaughter FAIL (+0 not +1) because pending replacements use the `ObjectId(0)` sentinel — proving B2/i_call/rankle exercise the real arm. Fixed by mirroring `damage_modification_for_rid`'s discriminator.

**A2 (runtime finalize MV rejection) = STOP-AND-RETURN to main.** Changed seam = parser (covered A0/A1 incl. production parse-path A1-full). Runtime enforcement is UNCHANGED pre-existing wiring. card-data stale symlink blocks name-load of real card; full combat→trigger→dig→impulse→finalize harness too heavy here. A1-full is a parser shape test, so per the strict gate A2 is returned for main authoring after card-data regen.

## 5. Maintainer-simulation matrix
**Row B (Hawkeye):** authority=replacement source `rid.source`; controller = object-hosted ⇒ `replacement_source_player` (CR 109.4) / pending ⇒ def `source_controller`. Value=`QuantityExpr`, bound at application, LIVE re-resolved (B2: power 2→4 flips 5→7). Storage=`Plus.value` on def. Consumer=`damage_done_applier`. Invalidation: continuous static re-applies; host gone ⇒ `Ref` resolves 0, `Fixed` unaffected. Hostiles: combat (NoncombatOnly), own-permanent (target filter), sentinel-pending Fixed (i_call/rankle). Serde: u32→QuantityExpr, bare-int back-compat (B3), no wire bump.

**Row A (Cosmic Cube):** authority = MV ceiling `Ref(Aggregate{Max,Power,attacking-you-control})`, LIVE, resolved at finalize (UNCHANGED casting path, attackers still attacking CR 508.1). Stored `CastFromZone.constraint` → `ExileWithAltCost{constraint}`. Non-attacker exclusion verified at PARSE (FilterProp::Attacking). **INCOMPLETE row:** runtime multi-authority hostile fixture = A2 main-side item.

## 6. CR-annotation diff gate
107.1b, 107.3a, 109.4, 120, 202.3, 601.2e, 614.1a — all verified vs docs/MagicCompRules.txt, **0 UNVERIFIED**.

## 7. Judgement calls
- Resolver controller derivation extended beyond plan pseudocode to handle `ObjectId(0)` sentinel (pending replacements) — caught by i_call_for_slaughter; behavior-identical for all Fixed cases. CR 109.4.
- `freeze_damage_modification_x`: `if *value==0` → `matches!(value, Fixed{value:0})` so bare-"plus x" still freezes, where-bound `Ref` never clobbered (Taii Wakeen 8 tests).
- A2 deferral (proportionate).

## 8. Stop-and-return items (main-side)
1. A2 runtime finalize-rejection test (after card-data regen).
2. `cargo engine-inventory` regen (Plus field type u32→QuantityExpr) — NOT run (symlink to main).
3. `cargo coverage` / `cargo semantic-audit` — deferred (stale card-data symlink would mislead); run post-regen to confirm the two Swallow:DynamicQty gaps close.

## 9. CR annotations added (grep-verified before writing)
- CR 202.3 + 601.2e — parse_with_mana_value_constraint
- CR 614.1a + 120 + 107.1b — resolver Plus arm; CR 109.4 — controller discriminator
- CR 614.1a + 107.3a — dynamic offset parser arm + freeze fn
- NEW-1 honored: no "counters=CR 121" prose in any comment (no QuantityModification non-goal code comment added).

## 10. Deviations
Resolver sentinel-controller fix (§7); no allowlist edit (auto-flip); A2 deferred; NEW-2/3/4 applied.

## 11. Risks for /review-impl
- HIGH: resolver controller fallback for `ObjectId(0)` sentinel / missing host (PlayerId(0)) — behavior-identical for all Fixed; only Ref-on-pending would be sensitive (no such card today).
- MED: A0/A1 are parser shape tests; runtime MV enforcement verified structurally only (A2 deferred; enforcement code unchanged).
- LOW: `many_till(anychar,…)` O(n·m) per call (short clauses, fine).
- Stale symlinks (card-data, engine-inventory) by design; flip + inventory regen in main.
