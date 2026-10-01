# Phase 5 executor report r1 (continuation)

MODE implementation/fix (phase mode). BASE_SHA = START_SHA = 91ad0dff88426ed6d983e8e060d67cd0a030c926. IMPLEMENTATION_WORKTREE=/home/user/phase (not clean at start by design: previous executor's 28 modified + 3 new files; nothing staged; HEAD unchanged at end; delta == the 31 SCOPE_PATHS exactly, no out-of-list path). All evidence below is PREPARATORY, not completion evidence.

## Diff summary
- types: ability.rs (TextWordDomain, TextSubstitution, TextSubstitutionSpec, ContinuousModification::SubstituteTextWord), layers.rs (Layer::Text), ability_visit.rs (arm).
- game: text_substitution.rs (new: WORD_CARRIERS authority, rewrite, active_text_substitutions, Layer-3 pre-pass, restamp_resolving_spell_text with field-exhaustive ResolvedAbility destructure), layers.rs (pre-pass call, bucket filter, incremental escalation, write class ALL, depends_on arm recipient-scoped, shared is_intrinsic_basic_land_mana_ability, unit tests), effects/effect.rs (latch_chosen_text_words with take + unit test added this round), stack.rs (resolve_top call, spells only), mod.rs, forced arms (ability_rw, ability_scan, coverage, become_copy, off_zone_characteristics, quantity).
- parser: oracle_effect/text_change.rs (new, nom recognizer + shape tests), oracle_effect/mod.rs hook, forced arms (oracle.rs, lower.rs, oracle_static/shared.rs).
- phase-ai forced arms (devotion, x_reference); protocol bump files (lobby-broker, server-core, ws-adapter, protocol.ts, 2 client tests, check-protocol-version.mjs).
- tests: integration/text_substitution_cr612.rs (22 tests) + main.rs mod line; fixture integration_cards.json.gz re-sliced.
- This round's edits: re-generated card-data + fixture against the final parser (the last parser edit post-dated the fixture), added effect.rs unit test `text_word_latch_consumes_the_answer` (the plan's C1 `last_named_choice.is_none()` assertion did NOT discriminate take vs clone: measured green under clone), cargo fmt on scope files.

## Red/green (tests cannot compile at base: they use the new types; red shown by neutralizing each seam in one build via a temporary RED env gate, all gates removed afterwards, `grep 'env::var("RED")' crates` empty). Log: phases/5/red-matrix.log, red-latch.log
- pre-pass off: 14 fail (P1,P3,P4 x3,P5,P6,ST1,P8,E1,L1,S3,C1-entwine, P2). restamp off: S1, Acid Rain, S5 fail. escalation off: L1 + S3 fail. depends arm off: 2 layers unit + chain, chain-plus-loop, per-recipient tests fail. latch off: 15 fail. carrier NotAWord->Word: P2 + unit fail. parser off: 4 shape tests fail. take->clone: only the new unit test fails (35 others green) -> unit test is the discriminator.
- Green: 36 new tests pass (35 + latch unit).

## Checks (PREPARATORY)
- cargo fmt on the .rs scope paths; clippy --workspace --all-targets -D warnings: clean (clippy2.log). check-protocol-version.mjs exit 0. check-interaction-bindings.sh --check exit 0, binding unchanged (no regeneration, no admitted-class addition). check-parser-combinators.sh: Gate A/G PASS. Parser inline gate: only #[cfg(test)] lines (`.contains` in text_change tests). CR gate: zero UNVERIFIED.
- client tsc -b --noEmit exit 0; vitest protocol.test.ts + p2p-adapter-multiplayer.test.ts 202 pass (coverage threshold disabled for the 2-file run). phase-ai devotion/x_reference 34 pass.
- CEN1/CEN2 under FORGE_TEST_FULL_DB=1 pass. card-data jq: Magical Hack, Crystal Spray, Sleight, Alter Reality, Glamerdye, Mind Bend, Spectral Shift, Trait Doctoring, Whim of Volrath have no Unimplemented; Balduvian Shaman, Artificial Evolution, New Blood, Magical Hacker, Deceptive Divination stay Unimplemented (Shaman: allowed conditional).
- LAST: full `cargo nextest run -p phase-engine --no-fail-fast`: 31354 passed, 0 failed, 12 skipped (full.log), no file touched after start; end HEAD unchanged, nothing staged.

## Fixture
integration_cards.json.gz: 4649 -> 4734 cards (gen-test-fixture.py --check ok). Added 85 (includes the 21 plan cards; the remainder are names other tests already spell that were absent from the stale fixture), removed 2 (nobody, working stiff: no longer referenced). 162 pre-existing cards changed value; causes measured: `declares_chosen_group` field newly serialized by an earlier phase's parser/type change (dominant; ~130 abilities/triggers), static condition shape (6 cards), Blitz keyword payload (4), MTGJSON refresh noise (legalities, source_printing_ids: e.g. forest, mountain), plus few effect/actor/parse_warnings diffs. None reverted. Base copy: phases/5/integration_cards.base.json.gz. Regenerated data files (card-data.json, oracle-subtypes.json, known-tokens.toml, client/public/*) show no tracked/unstaged entries beyond the 31 paths.

## Maintainer-simulation / coverage map (summary; plan §7/§9 are the full rows)
Claim -> seam -> test: Hack on permanent -> pre-pass -> magical_hack_changes_landwalk_indefinitely; static text -> changed_static_ability_applies_in_the_same_layer_pass; spell on stack -> restamp in resolve_top -> text_change_on_a_spell_changes_what_resolves / acid_rain; permanent spell carries -> incremental escalation -> text_change_on_a_permanent_spell_carries_to_the_permanent; ordering -> depends_on arm -> chain/loop/chain_plus_loop/same_from_different_objects; CR 612.2 carve-out -> WORD_CARRIERS -> mana_symbols_are_not_words...; latch consumption -> effect.rs unit test (helper-level; production reach of a skipped prompt is not constructible, entwined Spectral Shift covers the real two-answer flow); EOT expiry -> crystal_spray_change_ends_at_end_of_turn; non-effect field -> restamp_rewrites_repeat_for (direct call; cast-flow reach is S1). Each has revert-failing assertion per red matrix. Phase-4b loop-only rule present in base (chain_plus_loop green; it is red only if 4b rule absent). No DEFERRED rows.

## Judgement calls / deviations
- `from_label`/`label` etc. kept as plan; red demonstration by seam neutralization instead of base build (impossible: new API). Plan row P4's unit assertions live in layers.rs tests.
- Pre-existing fixture value changes are not from this phase's parser except the new cards.

## Stop-and-return items
None.
