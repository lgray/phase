# Phase 20 implementation review, round 1 (review-engine-impl, phase mode)

MODEL: claude-sonnet-5-5. Range PHASE_BASE 8253e7aa2a..CANDIDATE 41c9518db6 (single commit, 19 paths = scope.nul, none outside). Deferral list: none. Addenda phase-20: empty.

## Verdict

ACCEPT. No `behavior` or `machinery` findings. Two `text` findings (T1, T2, both LOW, exit-round material).

## Derivation before code (cards via jq on client/public/card-data.json; CR 612.1/612.2/107.4 grepped)

- Firespout "if {R} was spent ... if {G} was spent", Batwing Brume "{W}"/"{B}", Gruul Scrapper "{R}", Seed Spark "{G}" are printed symbols (CR 107.4); CR 612.2 changes only a color word used as a color word, so Sleight of Mind / Crystal Spray never change these riders. Seed Spark's "green 1/1 Saproling" is a word and does change.
- Slaying Fire "at least three red mana" is a word: Red -> Blue turns RRR from 4 to 3.
- Sleight/Crystal Spray text confirm "one color word" only.

## Checks

1. Design. `SpentColor { ColorWord, ManaSymbol }` on both `ManaColorSpent` variants is the required typed discriminator at the existing parser/carrier-table authority; carrier rows keyed by the nearest tag (`ColorWord` W, `ManaSymbol` SYMBOL); old `ManaColorSpent` row deleted; no emitter deleted, no bool, no sibling variant on a parent enum. Tag collision: `git grep -nE "^\s+(ManaSymbol|ColorWord|SpentColor)\b" -- crates/*/src` finds only `TextWordDomain::ColorWord`, a unit variant that serializes as a bare string (positive control: the grep matched it).
2. Class sweep. Population = every `ManaColorSpent` construction/destructure/consumer: `git grep -n ManaColorSpent -- crates client/src` plus every tracked non-source file (`git ls-files`, `zcat` for .gz, 5282 files; control: `integration_cards.json.gz` is the one hit). Emitters: conditions.rs symbolic (single, `And`) ManaSymbol, shadowed word parser ColorWord, oracle_trigger.rs Adamant ColorWord, `SymbolicManaSpentIntro::condition` (single, `And`) ManaSymbol; consumers: evaluators in effects/mod.rs and triggers.rs, two coverage.rs arms read `color.color()`; ability_scan.rs `color: _`, ability_rw.rs, assembly.rs, oracle_effect/mod.rs, conditions.rs/oracle_trigger.rs classification lists use `{ .. }`. Other conditions through the carrier table: census over the regenerated export (python walk, nearest-tag, 290 cards whose text contains "spent to cast"): symbolic text maps to `(ManaSymbol,color)` for 39 cards and to no other color carrier (no `(OfColor,color)` or `(Token,colors)` leaf on a card whose spend clause is a symbol); `(OfColor,color)` appears only on 16 word-text cards. Tried to construct a member the class should catch: a symbol-written clause that lands in a Word carrier. None exists in the export (only Vigor Mortis has symbol spend text and no `ManaSymbol` leaf; its export abilities are empty, see Pre-existing).
3. Production controls. Present and run: Firespout `{R}`/`{G}` under Red->Blue, Red->Green, Green->Blue (Sleight) and Red->Blue (Crystal Spray) with an unpaid color, Batwing Brume (`{B}`, `{W}`), Gruul Scrapper, Seed Spark (word + symbol on one card), Slaying Fire word-form rewritten, trigger word-form (Adamant text) runtime row plus symbol sibling, unit rows over typed conditions, export census R8 with reach-guards (`tagged_faces`, `symbol_leaves`). Baseline rows in every driver are the positive reach-guard; Slaying Fire and Seed Spark prove the change reaches the stack object.
   Mutation runs (W at candidate, warm `target-dandan`, filter = new rows + symbolic/adamant parser tests; control M0 = 1609 run, 1609 passed):
   - M3 `ManaSymbol` row classed `W` (the base classification on the new shape): 7 red: `spent_color_rewrites_the_word_form_and_spares_the_symbol_form`, `firespout_...`, `seed_spark_...`, `batwing_brume_...`, `gruul_scrapper_...`, `trigger_side_word_form_...`, `spent_color_provenance_...`.
   - M4 `ColorWord` row deleted: 2 red (the unit row, `trigger_side_word_form_...`).
   - M1 both symbolic emitters emit `ColorWord`: 12 red (R7b emitter row, 2 `suffix_symbolic_*`, 8 `extract_symbolic_*`, `trigger_side_word_form_...`).
   - M1x M1 plus the committed fixture's 13 `ManaSymbol` objects rewritten to `ColorWord` (what a regeneration writes): 17 red (adds firespout, seed_spark, batwing_brume, gruul_scrapper, spent_color_provenance_...).
   - M2 Adamant emitter emits `ManaSymbol`: 2 red (`extract_adamant_three_red`, `trigger_side_word_form_...`).
   Limit, not a finding: with the fixture unchanged, M1's spell-side effect is caught by the unit rows only; the runtime rows read the fixture AST, which `gen-test-fixture.py --check` validates for bytes, not currency.
   Restore trap measured: the first M0 run was red (stale binary) because the executor's `cp -p` restore left source mtimes older than the build; after touching all 13 scope `.rs` files M0 was green. The candidate tests are green only on a build of the candidate sources.
4. Protocol. Re-derived: `UPSTREAM_MAIN_FULL_GAME_PROTOCOL_VERSION (71) + 46` = 117, wire `54 + 45` = 99, lobby 16, draft 30 unchanged. `node scripts/check-protocol-version.mjs` rc 0; control (`+ 47` copy) rc 1 "Protocol version must remain 118: Rust=117, client=117". History entries present in ws-adapter.ts, protocol.ts, lobby-broker, server-core, script; pin tests renamed/updated (`protocol_version_is_117_for_spent_color_provenance`, lobby `MIN_SUPPORTED_PROTOCOL` 116 = `PROTOCOL_VERSION.saturating_sub(1)`, vitest `toBe(99)`, `setupFrameAt(98)` refusal / `(99)` admission). Numeral sweep (`git grep` for 116/115/98 asserted forms) leaves only the intended 98 refusal frame. Bindings: no generated file (`engine_wasm.d.ts`, interaction bindings, schema) mentions `ManaColorSpent`, `SpentColor` or `AbilityCondition`; none changes.
   Fixture accounting (`integration_cards.json.gz` decompressed, base vs candidate, key diff): 15 changed + 13 added, 0 removed. 11 changed (afiya grove, covetous dragon, dandan, dark depths, darksteel reactor, emperor crocodile, endangered armodon, mazemind tome, phylactery lich, rune-tail, transcendence) differ only by the state-trigger condition wrapped in `EventTime { condition }` with the inner condition unchanged, the shape produced since upstream `6c9a99b029` (an ancestor of the base; `git merge-base --is-ancestor` rc 0; that commit did not touch the fixture); 4 changed (azorius herald, dawnglow infusion, emptiness, mythos of snapdax) differ only by `"Color"` -> `{type: ManaSymbol, color}`. Added: firespout, batwing brume, gruul scrapper, seed spark (the new rows) and 9 cards (boomerang, force bubble, nine lives, olivia, plague boiler, scrollshift, steady progress, vampire hexmage, whitesun's passage) that the generator picks up from quoted names in already-merged tests; those tests build the cards from oracle text, so none depends on the added entries. Only `dandan_hand_entry_ownership.rs` reads one of the 11 drifted cards from the fixture (Dandan); it and `carrier_census*` pass on the candidate (14 passed). `gen-test-fixture.py --check` rc 0 (5105 cards, canonical bytes).
5. Comments, CR, parser gate. CR 612.2 (color word used as a color word) and 107.4 (mana symbols) grepped; subjects match the `SpentColor` doc and test messages; CR 400.7d, 106.3, 207.2c, 601.2h retained from existing docs resolve. No new parser dispatch: all parser edits are constructor payloads; the added-line scan for `find/split_once/contains/starts_with/strip_prefix` is 0 (control: 67 hits in the same file at the candidate). `scripts/check-parser-combinators.sh` flags only `imperative.rs` (untouched, fork-relative default base).

## Findings

**[LOW][text]** Over-long doc line from an in-place line edit. Evidence: `crates/engine/src/parser/oracle_trigger.rs` doc of `try_extract_symbolic_mana_spent_condition` (136 columns). Why it matters: engine comments are paragraph-wrapped; the paragraph now has a single over-long line. Suggested fix: replace
`/// words. Evaluates like Adamant (`ManaColorSpent`) but carries\n/// `SpentColor::ManaSymbol` where Adamant carries `ColorWord` (CR 612.2). Per CR 400.7d, a permanent's ability can reference "what mana\n/// was spent to pay [its casting] costs."`
with
`/// words. Evaluates like Adamant (`ManaColorSpent`) but carries\n/// `SpentColor::ManaSymbol` where Adamant carries `ColorWord` (CR 612.2). Per\n/// CR 400.7d, a permanent's ability can reference "what mana was spent to pay\n/// [its casting] costs."`

**[LOW][text]** Plan row labels in durable test docs. Evidence: `crates/engine/tests/integration/text_substitution_cr612.rs` doc comments `/// R1. `, `/// R2. `, `/// R3. `, `/// R4. `, `/// R5. `, `/// R6. `, `/// R8. ` (the R-numbers name rows of a plan that is not in the repository). Why it matters: a reader cannot resolve them. Suggested fix: delete the `Rn. ` prefix from each of those seven doc lines (e.g. `/// R1. CR 612.2 + CR 107.4: a color-word change ...` -> `/// CR 612.2 + CR 107.4: a color-word change ...`).

## Pre-existing (untagged, non-blocking)

- Export `abilities` for Vigor Mortis ("If {G} was spent to cast this spell, ...") is `[]`, so its symbol rider has no `ManaSymbol` leaf. Observed in the regenerated export only; not compared at the base, and no row of this change depends on it.
- Parser gate flags `crates/engine/src/parser/oracle_effect/imperative.rs` `rest.strip_suffix(" counters")` (untouched by the change).

## Executor r2-notes (plan-review-r2) closed

T1 (608.2b) not cited anywhere in the diff (grep: 0). R8 `ColorWord` leg has no export population; its reach-guards are asserted and the rewrite leg compares only `ManaSymbol` condition objects. New rows run after the fixture/export regeneration (fixture carries the new shape). Scratch builds touched all sources.

## Worktree record

Start: porcelain empty, HEAD == 41c9518db6. Probes copied originals to `dandan-run/scratch/p20rv/orig`, restored with `cp` (fresh mtime), `sha256sum -c` OK for all four mutated paths after every run. End: `git status --porcelain` empty (0 lines), HEAD 41c9518db6025b372c47e1665d0078b7fef6ad56.
