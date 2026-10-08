# Phase 20 executor report r1 (implementation/fix, phase mode)

Mode: implementation/fix. BASE_SHA = START_SHA = 8253e7aa2a273774351b773de29379752accb9c8. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Model: claude-sonnet-5-5. All evidence below is PREPARATORY, not completion evidence.

## 1. Diff summary (19 files, all in scope.nul; none outside)

- `types/ability.rs`: new `SpentColor { ColorWord, ManaSymbol }` + `color()`; both `ManaColorSpent` variants retyped; three doc blocks rewritten by symbol.
- `parser/oracle_effect/conditions.rs`: symbolic emitter (single + `And`) -> `ManaSymbol`; shadowed word parser -> `ColorWord`; docs; tests re-pinned + `mana_color_spent_emitters_record_word_versus_symbol` (R7b).
- `parser/oracle_trigger.rs`: `try_extract_adamant_condition` -> `ColorWord`; `SymbolicManaSpentIntro::condition` (single + `And`) -> `ManaSymbol`; false "semantically identical" sentence replaced.
- `parser/oracle_trigger_tests.rs`: 10 constructions re-pinned (Adamant `ColorWord`, nine symbolic `ManaSymbol`).
- `game/text_substitution.rs`: `("ManaColorSpent","color")` row deleted; `("ColorWord","color") W` and `("ManaSymbol","color") SYMBOL` rows; unit row R7a.
- `game/effects/mod.rs`, `game/triggers.rs`, `game/coverage.rs`: evaluators/labels read `color.color()`; inline tests re-pinned (Adamant tests `ColorWord`; `{W}{W}` tally test and the effects test `ManaSymbol`).
- `game/ability_scan.rs`, `game/ability_rw.rs`: legacy-shape pins -> `ColorWord` (forced by the lib test profile).
- `tests/integration/text_substitution_cr612.rs`: R1-R6, R8 (no new binary, no mod line).
- Protocol: `scripts/check-protocol-version.mjs`, lobby-broker + server-core `protocol.rs`, `ws-adapter.ts`, `network/protocol.ts`, `protocol.test.ts`, `p2p-adapter-multiplayer.test.ts`.
- `tests/fixtures/integration_cards.json.gz`: regenerated.

## 2. Worktree record

Start: `git rev-parse HEAD` = START_SHA, `git status --porcelain` empty, no staged entries. End: HEAD still START_SHA, `git diff --cached` empty (0), unstaged delta = exactly the 19 scope paths (`comm -23 delta scope` empty, `comm -13` empty). Scoped `rustfmt --edition 2021 --config skip_children=true` over the 13 `.rs` scope paths (rc 0; no path outside scope touched). No commit/stage/push.

## 3. PREPARATORY verification

- `cargo check -p phase-engine --lib --tests`: rc 0 (7 production files + the compiler-forced inline-test sites in `triggers.rs`, `effects/mod.rs`, `ability_scan.rs`, `ability_rw.rs`, `conditions.rs`, `oracle_trigger_tests.rs`; same set as the plan).
- Focused nextest (package `phase-engine`, feature `phase-engine/test-support`), FOCUSED_FILTER = `test(/text_substitution/) | test(/adamant/) | test(/mana_spent/) | test(/mana_color_spent/) | test(/oracle_trigger/) | test(/oracle_effect::conditions/) | test(/census/) | test(/ordering_parity_sweep/) | test(/spent_color/) | test(/firespout|seed_spark|batwing|gruul_scrapper|slaying_fire|trigger_side_word|emitters_record/)`: 1750 run, 1750 passed (includes `text_substitution_cr612` whole module, `ordering_parity_sweep`, every `*census*`, adamant and oracle_trigger tests). No compiler warnings in the log. `no_top_level_test_binaries`: 2/2.
- Protocol: `node scripts/check-protocol-version.mjs` rc 0; control (copy with `+ 47`) rc 1 "Protocol version must remain 118: Rust=117, client=117". `cargo nextest run -p lobby-broker -p server-core protocol`: 122/122. vitest `protocol.test.ts` 57/57 and the p2p "wire-protocol version gate" block 6/6; control (WIRE constant 99 -> 98, restored sha-identical): 2 red (`pins the P2P wire protocol to v99`, `refuses the previous wire protocol (v98) and admits its own (v99)`).
- `./scripts/check-interaction-bindings.sh --check` rc 0; control (appended line to committed `generated/interaction/index.ts`, restored sha-identical): "stale interaction bindings", rc 1.
- `python3 scripts/gen-test-fixture.py --check` rc 0 ("fixture covers all 5105 referenced cards with canonical gzip bytes").
- Not run (per brief: orchestrator owns): `cargo coverage`, `cargo semantic-audit`, clippy, ai-gate, full suites, workspace `cargo check`. Population for the last: `git grep ManaColorSpent` outside `crates/engine` hits only the protocol prose entries I wrote (lobby-broker, server-core, ws-adapter.ts, protocol.ts, the script); no phase-ai/wasm/server/client code reads the tag.

## 4. Parser preparatory gate

Inline grep over added parser lines (114 scanned, control = nonzero projection): zero hits. `scripts/check-parser-combinators.sh` (default base): rc 1 with one flagged line, `crates/engine/src/parser/oracle_effect/imperative.rs` `rest.strip_suffix(" counters")` — a file I did not touch (it comes from the base's upstream merge vs the script's fork-relative default base d6d28370c0); my files were in the 123 scanned and produce nothing. Passing START_SHA as base-ref is `CANNOT ANSWER` (empty range), so the default-base run is the only usable one. Gate G PASS.

## 5. Discriminating-test gate: production-path coverage map

All integration rows drive `GameScenario` + `runner.cast(..).commit()` + P1 `cast(changer).target_objects(&[spell]).choose_option(label).commit().resolve()` (real `apply()` pipeline, verbatim `add_real_card` text from the regenerated fixture) except R6 (oracle-built creature, verbatim ETB text) and the unit rows.

| Claim | Changed seam | Production entry | Test | Fails if reverted (M3 = base classification) | Siblings / negatives |
|---|---|---|---|---|---|
| R1 symbol rider survives word change (spell) | `WORD_CARRIERS` ManaSymbol row | `restamp_resolving_spell_text` via real cast + response | `firespout_symbol_riders_survive_color_word_changes` | `{R}`: Red -> Blue/Green leave ground dead; M3 red: assertion `{R} paid under (Sleight, "Red -> Blue")` | unchanged `{R}` and `{G}` baselines; `{R}` + Green -> Blue; `{G}` + Green -> Blue/Red; Crystal Spray |
| R2 word leaf + symbol rider on one card | same | same | `seed_spark_rewrites_the_token_word_but_keeps_the_symbol_rider` | M3: tokens `[]` vs `[[Blue],[Blue]]` (rider lost, as at base) | unchanged two green; W+R none; W+R Red -> Green none |
| R3 two riders, combat | same | cast in declare-attackers step, combat damage | `batwing_brume_symbol_riders_survive_color_word_changes` | M3 red | `{B}` (19,17), `{W}` (20,20) baselines; Black -> Blue; White -> Black; reach-guards: P0 priority after attack, combat damage phase reached |
| R4 trigger side | `TriggerCondition::ManaColorSpent` carrier (permanent carry) | creature spell resolve -> ETB trigger | `gruul_scrapper_symbol_rider_survives_color_word_changes` | M3 red | unchanged G+R haste, G-only none; Red -> Green on G-only none; reach-guard: creature on battlefield |
| R5 word-form spell rider (preservation) | `OfColor` Word carrier (untouched) | real cast | `slaying_fire_word_rider_is_rewritten_by_a_color_word_change` | green at base and fix by design | RRR 4; Red -> Blue/Green 3; Green -> Blue 4; proves the change reaches the stack object |
| R6 trigger word emitter vs symbol sibling | `try_extract_adamant_condition` -> `ColorWord`; `SymbolicManaSpentIntro` -> `ManaSymbol` | oracle-built creature ETB | `trigger_side_word_form_is_rewritten_and_symbol_form_is_not` | M1/M2 red on the parse-form reach-guard; M3 red on `{R} is a symbol` | word RRR 1, RGG 0, Red -> Blue 0, Green -> Blue 1; symbol RRR 1, Red -> Blue 1; reach-guard asserts the parsed `SpentColor` variant first |
| R7a rewrite over typed conditions | carrier rows | `TextSubstitution::rewrite` | `spent_color_rewrites_the_word_form_and_spares_the_symbol_form` | M3 red; does not compile at base | `ColorWord` Red -> Blue, `ManaSymbol` None, `And` changes only the word leaf, trigger word rewrites, `Not(ManaSymbol)` None |
| R7b emitters record provenance | the four emitters | `parse_symbolic_..`, `parse_word_..`, `parse_condition_text` | `mana_color_spent_emitters_record_word_versus_symbol` + the re-pinned `extract_*` / `suffix_*` tests | M1/M1x red (12 / 17 failures) | each sibling asserts the other form; Adamant `extract_adamant_three_red` -> `ColorWord` |
| R8 export census | emitter output vs Oracle text; all 20 color pairs | `db.face_iter()` | `spent_color_provenance_matches_oracle_text_and_symbols_never_rewrite` | M1x and M3 red | reach-guards `tagged_faces > 0`, `symbol_leaves > 0`; rewrite leg compares only `ManaSymbol` condition objects before/after |

Red-at-base: R1-R4, R6 (symbol row) and R8 are red under M3, which is the base classification (symbol leaf classed Word) on the new shape; R7a/R7b do not compile at base (no `SpentColor`); R5 and R6's word rows are honest preservation rows. No test is shape-only except R7b/R7a, which are unit rows paired with the runtime rows above. No production-reachable arm is covered only by a degenerate fixture: R3's two riders, R2's word-plus-symbol card and R4's G-only row are multi-authority; Firespout's hybrid `{R/G}` is paid by exactly the named color (pool holds one Red or Green).

Mutations (one build per mutation, restored by `cp -p`, `sha256sum -c` OK after every run; control M0 = unmutated, same filter: 25/25 green). The filter selects the new rows plus the symbolic/adamant extractor tests.
- M0: 25 run, 25 passed.
- M1 (both symbolic emitters emit `ColorWord`, committed fixture): 12 failed — R7b `mana_color_spent_emitters_record_word_versus_symbol`, `suffix_symbolic_*` (2), 8 `extract_symbolic_*`/`extract_symbolic_unless_*` in `oracle_trigger`, and R6 (`trigger_side_word_form..`, reach-guard `left ColorWord / right ManaSymbol`). R1-R5 and R8 green because the fixture still carries `ManaSymbol`.
- M1x (M1 plus the fixture rewritten so its 61-style `ManaSymbol` leaves read `ColorWord`, what a regeneration from the mutated emitter writes): 17 failed — M1's 12 plus R1, R2, R3, R4, R8. R5 green. The plan defines M1 only; I read the brief's "M1x" as this fixture-carrying variant.
- M2 (`try_extract_adamant_condition` emits `ManaSymbol`): 2 failed — `extract_adamant_three_red`, R6 (reach-guard `left ManaSymbol / right ColorWord`).
- M3 (`ManaSymbol` row classed `W`): 7 failed — R7a, R1, R2, R3, R4, R6 (`{R} is a symbol`), R8.

## 6. Maintainer-simulation matrix

| Row | Entry / first branch | Authority | Bound value / when | Mode | Storage | Consumers | Invalidation | Hostile fixtures | Serde |
|---|---|---|---|---|---|---|---|---|---|
| Spell-side `ManaColorSpent` provenance | `parse_symbolic_mana_color_spent_condition` (symbolic alt arm of `parse_mana_color_spent_condition`) | the `SpentColor` variant | `SpentColor::ManaSymbol{color}` at parse time | latched at parse (a symbol never becomes a word); payment tally live | `AbilityCondition::ManaColorSpent.color` inside the ability definition / resolving ability copy | `carrier_class` via `walk_strings` in `rewrite_counted`; evaluator `effects/mod.rs` reads `color.color()` vs `colors_spent_to_cast` | none (CR 612.2 symbol stays symbol) | R1, R2, R3 (reach `restamp_resolving_spell_text`) | ability serde shape changes: bare string -> `{type, color}`; export and fixture regenerated; protocol 117/99 |
| Shadowed word fallback | `parse_word_mana_color_spent_condition` (second arm; dead on production path) | variant | `ColorWord` | parse time | same field | same | none | direct unit call in R7b (no production reach; stays shadowed by `try_nom_condition_as_ability_condition`) | same |
| Trigger-side Adamant | `try_extract_adamant_condition` (first branch `tp.find("if at least ")`) | variant | `ColorWord` | parse time | `TriggerCondition::ManaColorSpent.color` in the trigger definition | `carrier_class`; `triggers.rs` evaluator `color.color()` | none | R6 word rows, `extract_adamant_three_red` | same |
| Trigger-side symbol | `SymbolicManaSpentIntro::condition` (`[(color,n)]` arm and `_` fan-out) | variant | `ManaSymbol` | parse time | same | same; the `Not` wrapper carries the leaves | none | R4 (Gruul), R6 symbol sibling, `unless {U}` / mixed rows | same |
| Carrier rows | `carrier_class(Some("ColorWord"\|"ManaSymbol"), "color", ColorWord)` | table | n/a | nearest tag of the leaf's own object | `WORD_CARRIERS` | the three `walk_strings` visitors unchanged | n/a | R7a, R8, `carrier_census_classifies_every_word_position_and_round_trips` (green) | none |

No row is incomplete or `UNREACHABLE`; the old `("ManaColorSpent","color")` row is gone (no object at that key carries a bare string any more; census green).

## 7. New-field threading sweep (`ManaColorSpent.color` retyped to `SpentColor`)

Constructions (all set provenance, required field, no serde default): `conditions.rs` symbolic (single `ManaSymbol`, `And` leaves `ManaSymbol`), word fallback (`ColorWord`); `oracle_trigger.rs` Adamant (`ColorWord`), `SymbolicManaSpentIntro::condition` single + `And` (`ManaSymbol`); tests as listed in section 1. Consumers: `effects/mod.rs` and `triggers.rs` evaluators `threads the field` (`color.color()`); `coverage.rs` two label arms `threads the field`; `ability_scan.rs` `color: _` and `ability_rw.rs`, `assembly.rs`, `oracle_effect/mod.rs`, `oracle_trigger.rs`/`conditions.rs` classification lists use `{ .. }`: `defaults intentionally because` they read no color. Serialization/payload constructors: none outside the engine (population grep above). Resume/continuation/batch seams: the condition is stored in the definition and cloned with the resolving ability; the only reader is the evaluator at resolution. No unlisted site.

## 8. CR-annotation diff gate

Gate command run: `UNVERIFIED` count 0; cited numbers 107.4, 400.7d (pre-existing in a rewritten doc block), 612.2 all present. Subject check: 612.2 "text-changing effect changes only those words used in the correct way (a color word as a color word)" and 107.4 "The mana symbols are {W}, {U}, {B}, {R}, {G}, {C}" describe the annotated `SpentColor` doc and test assertions. r2's T1 (608.2b) is not cited anywhere.

## 9. Regeneration (`gen-card-data.sh`, then `gen-test-fixture.py`)

- Export: 39 cards serialize `ManaColorSpent`, 61 leaves, all `ManaSymbol` (matches the plan's 61 / 39).
- Beyond the new shape, the regeneration changed 53 export entries (205 trigger conditions) and, in the fixture, 11 existing cards plus 9 added cards. All of it is base drift, not this change: the prior export and the committed fixture predate upstream `6c9a99b029` (#9683, Mazemind Tome) which wraps state-trigger conditions in `EventTime { condition }` (e.g. Afiya Grove `HasCounters` -> `EventTime{HasCounters}`; changed fixture cards: afiya grove, covetous dragon, dandân, dark depths, darksteel reactor, emperor crocodile, endangered armodon, mazemind tome, phylactery lich, rune-tail kitsune ascendant, transcendence). The 9 added fixture cards (boomerang, force bubble, nine lives, olivia crimson bride, plague boiler, scrollshift, steady progress, vampire hexmage, whitesun's passage) are referenced by `state_trigger_source_anaphor_recheck_and_self_suppression.rs` and other already-merged tests that the committed fixture lacked. Plus my four new cards (firespout, batwing brume, gruul scrapper, seed spark) and the four fixture cards that change shape (azorius herald, dawnglow infusion, emptiness, mythos of snapdax). Removed: none. Fixture now carries 5105 cards.
- Tracked-file drift check: `git status` after regeneration shows no tracked change beyond the 19 scope paths.

## 10. Protocol sweep accounting

Re-derived at step 0 (before edit): full `UPSTREAM_MAIN_FULL_GAME_PROTOCOL_VERSION (71) + 45` = 116, wire `PHASE_TWO_BASE_WIRE_PROTOCOL_VERSION (54) + 44` = 98, lobby 16, draft 30 — equal to the plan. Stacked to full `+ 46` = 117 (`// +46:` line + `v117` paragraph) and wire `+ 45` = 99 (`// +45:` line). Edited: script constants; lobby-broker `PROTOCOL_VERSION`, a `/// 117 —` doc entry, `assert_eq!(PROTOCOL_VERSION, 117)` and `MIN_SUPPORTED_PROTOCOL` pin 116; server-core `protocol_version_is_117_for_spent_color_provenance` (name, assert, revert-probe doc, preceding paragraph); `ws-adapter.ts` `PROTOCOL_VERSION = 117` + `117 —` entry; `network/protocol.ts` `WIRE_PROTOCOL_VERSION = 99` + `99 —` entry; `protocol.test.ts` title + `toBe(99)`; `p2p-adapter-multiplayer.test.ts` comment, title, `setupFrameAt(98)` refusal / `setupFrameAt(99)` admission. Sweep: `git grep -nE "\b(v116|protocol 116|wire 98|protocol 98|v98)\b"` and the numeral greps (`PROTOCOL_VERSION, N`, `toBe(N)`, `setupFrameAt(N)`) leave only historical prose entries (v98/v116 history lines) and the intended 98 refusal frame; lobby 16 and draft 30 unchanged; `RESOLUTION_STATE_WIRE_VERSION` unchanged (resolution-frame structure not touched).

## 11. Judgement calls

- R6's reach-guard asserts the parsed `SpentColor` variant before the behavior rows; under M1/M2 the guard fires first, so the word-row behavior assertions are not separately exercised in those mutations (they stay in the test and ran green at M0; M3 shows the symbol row discriminating at runtime).
- `Definitions<T>` exposes no iterator (`first()` only), so the guard reads `trigger_definitions.first()`; the oracle-built creature has exactly one trigger.
- The effects/mod.rs evaluator test and the triggers.rs `{W}{W}` tally test use `ManaSymbol`; the five Adamant `triggers.rs` tests use `ColorWord` (each pins the form its production emitter produces).
- R8 runs over the fixture database (5105 faces, 8 tagged faces: the four shape-changed fixture cards plus the four new ones), not the 39-card full export; it is a fixture-population claim, and the full-export population (39 cards / 61 leaves, all `ManaSymbol`) was measured separately from the regenerated export above.

## 12. Stop-and-return items

None. Deviations from the plan: none in design; M1x is not defined in the plan (interpreted above).

## 13. CR annotations added/changed

`CR 612.2 + CR 107.4` on `SpentColor` and the R7a/R7b/test docs; `CR 107.4` in the R7a symbol assertion message; `CR 612.2` in the word assertion and emitter docs. Verified with `grep -nE "^(612\.2|107\.4)" docs/MagicCompRules.txt` (both present, subjects as in section 8).

## 14. Risks for the checkpoint / review

- The regenerated fixture carries the 11 + 9 base-drift items from section 9; a reviewer comparing the fixture diff will see them.
- Protocol 117 / wire 99 are stacked on the lineage's upstream base; if upstream main moves its constants before this lands, re-derive and restack (script rc 0 is the check).
- `cargo coverage` / `semantic-audit` parity (plan step 9) and the parse-diff bot's parse-scope sentence are not run here.
- Scratch logs under `/home/lgray/vibe-coding/dandan-run/scratch/p20-*` (mine: `p20-focused1.log`, `p20-mut-M*.log`, `p20-mut-summary.log`, `p20-post.log`, `p20-gen.log`, `p20-check*.log`, `p20-pc.log`).
