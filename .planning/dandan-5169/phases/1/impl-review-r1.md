# Phase 1 implementation review, round 1 (phase mode)

Review Head: 2cf3d3aae0aeb555eae93c7b230cf35612b41100 (range b9ba9360..2cf3d3aa; reproduced with `git diff`, 4 files, +128/-16, all in SCOPE_PATHS; index.ts untouched)
Completion Gate: PASS (`bun test scripts/card-bot` re-run at the candidate: 158 pass, 0 fail, 9 files; worktree clean, HEAD == candidate)
Maintainer-Simulation Gate: PASS (every plan-matrix row maps to a test through the production handlers `lfgAutocomplete` / `lfgCommand`; no row deferred except the charter's Phase 2 Dandan entry, `DEFERRED(phase 2)`, which the synthetic 30-entry registry test covers in the interim)

## Verdict: ACCEPT. No HIGH or MED findings.

## Constraint closure
- F1 CLOSED: formats.test.ts asserts `/options:\s*\[\s*LFG_FORMAT_OPTION,/` present and `/\bFORMATS\b/` absent in the register.ts source (read via `Bun.file`); the `toHaveLength(1)` form is gone. register.ts diff shows `LFG_FORMAT_OPTION` first in `lfgCommand.options` and the `FORMATS` import dropped.
- F2 CLOSED: `suggestFormats(query, registry = FORMATS)`; `lfgAutocomplete` calls the one-argument form; test "a registry past the cap..." uses a synthetic 30-entry registry (empty query -> first 25 in order; `Fmt 29` -> `F29`; `fmt` -> exactly 25). The slice-after-filter and cap are therefore discriminating today. (Executor reports red-on-revert mutations; I did not re-run them, and the tree is read-only for me.)
- N1 CLOSED: filter lowercases label and key (`haystacks`); the `HistoricBrawl` key-typed test discriminates (label "Historic Brawl" does not contain that string, so only the lowercased key matches); `tiny` is a label-prefix hit; the negative round-trip asserts `created()` still has length 1 after the refused submit.

## Checks
- Only FORMATS-derived static choices list was register.ts; now gone. Dispatch: `handleInteraction` routes autocomplete by command name, so no index.ts change needed (charter's edit condition not triggered).
- Value contract: choice `value` is `f.format` (the key), resolved by unchanged `findFormat`; name truncated to 100, results capped at 25.
- Server branch behavior unchanged apart from sharing `normalizeQuery` (same trim/lowercase, same non-string fallback); existing server autocomplete tests pass.
- Refusal text change is inside scope and asserted by a test.
- No CR/Oracle surface; no engine, wire or serialized change; no protocol bump needed.

## Findings
None HIGH/MED. (LOW, informational, no action required: the 100-char name truncation has no dedicated test; it is a one-line slice on labels all well under 100 chars.)
