# Phase 1 plan review, round 1 (phase-plan mode)

Reviewed: plan.md vs charter Phase 1 entry, code at HEAD b9ba9360. Probes run: `bun test scripts/card-bot` (152 pass, 0 fail, 9 files); read register.ts, lfgInteractions.ts, formats.ts, formats.test.ts, index.ts dispatch (routes by command name, confirmed), discord.ts OptionType, ci.yml card-bot-test (same command).

## Sizing consistency (blocking check): CONFIRMED
Small-change lane holds: 1 unit; 4 counted paths equal the charter scope rule literally; index.ts is verify-only and the dispatch is by command name, so the charter's edit-condition is not triggered; no enum variant, serialized shape, WaitingFor/GameAction. T1 and T2 both fail as stated. The 25-entry FORMATS and the only FORMATS-derived choices list (register.ts) match the plan's measurements.

## Verdict: REVISE (2 blocking `text` findings, 1 non-blocking note). No seam, layer or type change is required by any finding.

### F1 [text, blocking] Verification row 1: "references LFG_FORMAT_OPTION once" is wrong
After step 2 register.ts contains the identifier twice (the import line and the use in `lfgCommand.options`), so a `toHaveLength(1)` on a `matchAll(/LFG_FORMAT_OPTION/g)` fails on correct code.
- Old (Step 3 and matrix row 1): "references `LFG_FORMAT_OPTION` once and never `FORMATS`" / "register source references `LFG_FORMAT_OPTION` exactly once".
- Replace with: "the `register.ts` source matches `/options:\s*\[\s*LFG_FORMAT_OPTION,/` (the option is placed first in `lfgCommand.options`, which also pins Discord's required-first rule) and does not match `/\bFORMATS\b/`".
Supplies a concrete fix; no seam change.

### F2 [text, blocking] Cap/slice assertions are not revert-failing at HEAD
Rows 3 and 5 rely on "once Phase 2 makes 26 formats, the assertion fails". With 25 registry entries, dropping `.slice(0, MAX_AUTOCOMPLETE_CHOICES)` (or slicing before filtering) leaves every listed assertion green today, so the discriminating claim is unproven in this phase (roles.md: every new test must be shown red on revert).
- Old (step 1c): `suggestFormats(query)` reads the module-level `FORMATS` only.
- Replace with: `suggestFormats(query: string, registry: readonly LfgFormat[] = FORMATS)`; `lfgAutocomplete` calls the one-argument form. Add to interactions.test.ts a case with a synthetic 30-entry registry (keys `F00`..`F29`, labels `Fmt 00`..`Fmt 29`): empty query returns exactly 25 and the first 25 in order; typing `Fmt 29` returns `F29` (reachable past the cap, fails if slice precedes filter); a query matching 30 returns exactly 25. Keep the real-registry cases. Update the Deferrals text (interim proof is now this synthetic case, not "the loop holding at 25").
Supplies a concrete fix; adds one defaulted parameter to a helper this plan already creates (no seam, layer or type change).

### N1 [non-blocking note] Implementation traps to state in step 1c
- Prefix/contains matching must lowercase both label and key (`HistoricBrawl` typed as key is lowercased in the query); say so explicitly.
- The hostile "tiny" case is a prefix hit on the label, not a contains hit; label it as such (the contains tier is covered by `"com"` finding Duel Commander / Pauper Commander).
- Round-trip negative should also assert the store holds no row after the refused submit (the plan says "no row written"; make it an assertion, e.g. `deps.store` list empty).

## Other checks passed
Class scope (every future format, 0 cards by construction, stated); building blocks reused, no new module; trace of `/lfg server` autocomplete is accurate; refusal-text change is inside a scoped file; CR/Oracle N/A; mirror-equality test left unchanged for Phase 2; deferral matches the charter list.
