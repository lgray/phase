# Phase 5 implementation review r1

MODEL: claude-sonnet-5-5
Range: 91ad0dff88426ed6d983e8e060d67cd0a030c926..eccc407e7ae8d35a83c71f5f2d0f15bc5691f106 (one commit, 31 files == scope.txt exactly)
Review Head: eccc407e7ae8d35a83c71f5f2d0f15bc5691f106
Completion Gate: PASS (evidence logs on committed bytes; I ran no cargo, per brief)
Maintainer-Simulation Gate: PASS (no DEFERRED rows; deferral list is "None")
Verdict: APPROVE. No HIGH or MED findings. Four LOW items below.

## Measured checks
- `git grep 'env::var("RED")'` at candidate: empty.
- CR numbers in added lines (Rust + TS), all grepped in docs/MagicCompRules.txt: none missing. 612.1/612.2/612.3, 613.1c, 613.7b, 613.8a/b, 608.2b/2d, 611.2a/2c, 400.7a, 305.6, 205.3i, 702.14c, 112.1/113.1c, 601.2b read and match their uses (CR 400.7a is the right cite for the permanent-spell carry).
- CR 613.8b: `depends_on` arm is recipient-scoped and word-class-scoped. Chain = `b.to == a.from`; loop = `b.from == a.from` (symmetric); Chosen/non-SpecificObject inert. `Layer::Text.has_dependency_ordering()` is true, so the arm is live. The chain-plus-loop test is discriminating: timestamp-only gives Plains, the rule gives Forest.
- Single rewrite authority: `WORD_CARRIERS` + `TextSubstitution::rewrite` are the only classifiers. Two readers (Layer 3 pre-pass for phased-in battlefield objects; `restamp_resolving_spell_text` in `resolve_top`, spells only per CR 113.1c) both go through it. No second word table.
- ResolvedAbility restamp is field-exhaustive (destructure, no `..`). Rewritten vs runtime-state split is defensible (announced_x/chosen_x/modal are announcement-time, CR 601.2b). The restamp mutates the local copy of the stack ability, so repeated resolution cannot double-apply. Serde round trip is safe: the only `skip_*` field is `TriggerDefinitionOccurrenceRef::Unmaterialized`, outside the rewritten `entry.definition`.
- CR 400.7: an indefinite effect on a permanent is ended on battlefield exit by the existing `zones::prune_object_bound_effects_on_exit` (which also carries the 400.7a stack-to-battlefield exemption), so Duration::Permanent does not leak across incarnations.
- Latch: `register_transient_effect` is called once per GenericEffect (not per target), so `take()` cannot starve a second recipient. Only fires when a `Chosen` mod is present. The unit test is the sole discriminator of take vs clone (production reach of a skipped prompt is not constructible); the entwine test covers the real two-answer flow. Accepted.
- Incremental flush: escalation guarded by `effect_can_reach_incremental_recipients`, which is `None => true` and `SpecificObject => membership`. Correct, over-escalates only when the effect names an entrant.
- Parser: nom-only (`tag/alt/value/opt/verify/terminated/preceded/eof`, `nom_on_lower`, `parse_target`, `parse_duration`); no `find/split_once/contains` dispatch outside `#[cfg(test)]`. Fail-closed for other word classes verified by the Artificial Evolution / New Blood / Magical Hacker test with a reach guard. The hook runs before the duration-peeling shell, which is required.
- Parser measurement: 13 flips = Phase 4's 4 + this phase's 9 (Magical Hack, Crystal Spray, Sleight of Mind, Mind Bend, Alter Reality, Glamerdye, Spectral Shift, Trait Doctoring, Whim of Volrath); no regressions.
- Frontend: only protocol pin files and their tests; zero game logic. Protocol 93->94 and wire 75->76 consistent across lobby-broker, server-core, protocol.ts, ws-adapter.ts, both client tests and `check-protocol-version.mjs` (+23/+22 offsets with comments). The Rust and TS docs state lobby 15 does not move.
- Fixture attribution (base copy verified byte-identical to `git show 91ad0dff:...`): 87 added / 2 removed (net +85) / 162 changed. None of the 162 contains `SubstituteTextWord` or "change the text"; the dominant diffs are `declares_chosen_group` (+130, from upstream 2b9790c3, before the run), Blitz payload shape (4), static condition shape (6), MTGJSON metadata/legalities, and two effect-type re-parses (Jace, Reality Sculptor ChangeZoneAll->ExileTop; Pemmin's Aura PumpAll->ChooseOneOf) that this phase's text-change hook cannot reach (it is gated on the `change the text of ` tag). Attribution holds. The run-base card-data diff is only the 13 cards, consistent with the fixture drift being pre-existing staleness.

## Findings

### F1 LOW text — comments violate the one-sentence rule (worker-env "Comments")
Multi-sentence or plan-label comments in added code:
- `crates/engine/src/game/text_substitution.rs`: `active_text_substitutions` doc (three sentences: "Only effects whose recipient ... / Effects are grouped ..."), `WORD_CARRIERS` doc (two), `latch_chosen_text_words` doc in `effects/effect.rs` (three), the `Runtime state, never text: ...` block in `rewrite_resolved_ability`, the module doc.
- `crates/engine/src/game/layers.rs` `depends_on` arm comment (three sentences).
- `crates/engine/tests/integration/text_substitution_cr612.rs`: 21 test doc comments lead with plan row ids, e.g. old `/// P1 + P2 + P5: a land-type word in a keyword is text (CR 612.1) and a change with` -> replace with `/// A land-type word in a keyword is text (CR 612.1), and a change with no stated duration lasts indefinitely (CR 611.2a).` Same for `S1`, `S3`, `S4`, `C1`, `ST1`, `PR2` prefixes in the test file and `text_change.rs` (`/// PR2: the keyworded ...`, `/// SHAPE: ...`).
Fix: trim each to the single sentence that carries the reason/CR; drop plan ids.

### F2 LOW text — CR 305.7 cite does not fit a text change
`text_substitution.rs:322`: old `// CR 305.6 + CR 305.7: the replaced type's intrinsic mana ability goes with` -> `// CR 305.6: the replaced type's intrinsic mana ability goes with`. CR 305.7 governs effects that *set* a land's subtype (and strips other abilities); a word substitution is not that effect, and the code only relies on 305.6's type-derived intrinsic ability. Same cite appears in the `magical_hack_changes_basic_land_subtype` test comment.

### F3 LOW machinery (test) — no pinned test for an indefinite change ending when the permanent leaves
Duration::Permanent text changes rely on `zones::prune_object_bound_effects_on_exit` (verified present, correct) but no test casts Magical Hack on a permanent, removes it from the battlefield, returns it, and asserts the printed text (CR 400.7). Add one paired case to the integration file (Bog Wraith: Plains after the change; after bounce/replay, Swamp). Not a defect today.

### F4 LOW text — executor report miscounts the fixture delta
`executor-r1.md` says "Added 85 ... removed 2"; measured 87 added, 2 removed (net +85). Correct the report line only; the fixture bytes are fine.

## Not findings (checked)
- `walk_strings` None-tag carriers match only when no `type` ancestor exists; the census test (corpus + full-DB run in `cen-full.log`) enforces classification, so an unlisted position fails closed.
- Layer 3 pre-pass runs before the single all-layer gather, so layer 2 statics also read changed text; same convention as the existing text layer, no deviation.
- AI/frontend: a 40-label `Choose` is displayed generically; no rules impact.
