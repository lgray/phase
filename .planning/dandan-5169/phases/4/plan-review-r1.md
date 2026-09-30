# Plan review r1 — Phase 4 (PREREQ-0), phase-plan mode (phase-fit: Sizing consistency blocking)
MODEL: claude-sonnet-5-5

Verdict: ACCEPT (no blocking findings). Small-change lane: CONFIRMED (1 scope path, ~2 LOC prod + ~15 LOC test, no variant/serialized/pipeline change, T1/T2 fail).

## Key question: is the coverage flip honest? (commands run: read counter.rs, ability.rs, card-data jq, integration test)
- Resolver `game/effects/counter.rs` reads `Effect::Counter.countered_spell_zone` and maps `Hand` -> Zone::Hand, `Library{position}` -> Zone::Library via `ZoneMoveRequest::at_library_position`, `None` -> Graveyard; exile riders take precedence. So the replacement IS executed at runtime, routed through `zone_pipeline::move_object`.
- Memory Lapse (Library Top), Remand (Hand + Draw sub), Spell Crumple (Library Bottom): each has a passing-by-design runtime test in `tests/integration/counter_spell_zone_redirect.rs` (top/bottom/hand + draw). Honest.
- Lapse of Certainty: AST identical to Memory Lapse (jq: Counter, Library Top); same code path; no dedicated runtime test, but identical text/AST, so honest.
- Spell Crumple second sentence ("Put Spell Crumple on the bottom of its owner's library") parses to `PutAtLibraryPosition{SelfRef,Bottom}`; `put_on_top.rs` has a SelfRef-source resolution path and a SelfRef unit test. The existing Crumple integration test uses only the first sentence, so the full-text card is not end-to-end tested. Not a detector concern and not regressed by this phase; LOW, no action required.
- Hinder correctly left unsupported (countered_spell_zone null, Graveyard ChangeZone); `Some(_)` is not over-accepting.

## Findings
- F1 [LOW, text, non-blocking] Plan Verification Matrix row 3 cites existing test `condition_if_accepts_countered_spell_redirect`; the real name in swallow_check.rs is `condition_if_accepts_countered_spell_zone_redirect`. Replace the old string with the new name. Seam/layer/type change: no.
- F2 [LOW, behavior, non-blocking] Spell Crumple end-to-end (both sentences) has no runtime test; flip is still honest (each clause has its own runtime coverage). Optional: none required; out of scope (1-path lane). Seam/layer/type change: no.

Other checks: `effect_is_replacement_carrier` at HEAD matches plan (or-pattern ending `ExileResolvingSpellInsteadOfGraveyard { .. } => true`, `_ => false`); arm placement valid (or-pattern arm before `=> true` terminator needs the comment/arm inserted before the final alternative's `=> true`, which the plan's "terminator stays last" wording covers). Helpers `parse_named`/`only_swallow`/`has_swallowed_detector` exist with the signatures the plan assumes (parse_named takes text, name, types). CRs were grepped by planner; arm is typed, no nom/string concern.
