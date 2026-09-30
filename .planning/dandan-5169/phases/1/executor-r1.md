# Executor r1 — phase 1
Mode: implementation/fix (phase mode). BASE_SHA=START_SHA=b9ba93609631d6d4bfab7169c00a9e57cec58ae0. IMPLEMENTATION_WORKTREE=/home/user/phase. Start: clean, HEAD==START_SHA, nothing staged. End: HEAD unchanged, nothing staged, delta = the 4 scope paths only.

## Diff
- scripts/card-bot/lfgInteractions.ts: exported LFG_FORMAT_OPTION; suggestFormats(query, registry = FORMATS); normalizeQuery shared by server and format branches; `format` branch in lfgAutocomplete; refusal text "Unknown format. Pick one from the suggestions."
- scripts/card-bot/register.ts: LFG_FORMAT_OPTION replaces the static-choices option; FORMATS import dropped.
- scripts/card-bot/__tests__/formats.test.ts: cap test replaced by "format option is served by autocomplete" (F1 regexes: /options:\s*\[\s*LFG_FORMAT_OPTION,/ present, /\bFORMATS\b/ absent).
- scripts/card-bot/__tests__/interactions.test.ts: "/lfg format autocomplete" describe (6 tests; F2 synthetic 30-entry registry; N1 traps: lowercased key match, "tiny" as prefix hit, store-empty assertion via spyCreates).

## PREPARATORY (not completion evidence)
`bun test scripts/card-bot`: 158 pass, 0 fail (base 152; +7 new, -1 replaced). No card-bot lint/type config exists (no tsconfig; CI runs only bun test). No Rust changed; parser gate, cargo fmt, CR gate N/A (no CR cited).

## Red on revert (temporary mutation, restored byte-identical, then green 158)
- Static choices restored in register.ts -> formats "served by autocomplete" red.
- Drop .slice cap -> "registry past the cap" (synthetic) red.
- Slice before filter -> same test red.
- Ranking registry-order only (prefix tier = contains) -> "prefix matches precede contains" red.
- Remove format branch -> 5 format-autocomplete tests red.
- Old refusal text -> "chosen value posts ... refused" red.

## Coverage map
Each plan-matrix row maps to the test named above through lfgAutocomplete/lfgCommand (production handlers; handleInteraction routes by command name, index.ts untouched). Mirror equality test unchanged. DEFERRED(phase 2): 26th Dandan FORMATS entry; interim proof is the synthetic-registry test. New-field sweep: none (no field added). Matrix: no game state, no authority/binding; value = FORMATS key resolved once by findFormat.

## Judgement calls
- Empty query needs no special case: startsWith("") puts every entry in the prefix tier in registry order.
- lfgAutocomplete calls suggestFormats(stringOption(options,"format") ?? "") (already-normalized inside suggestFormats).
- "com" test asserts Commander, CommanderDraft first (both prefix), DuelCommander later; real registry order.

## Stop-and-return: none. Deviations: none. Risks: live guild command set needs re-running register.ts at deploy (not run).
