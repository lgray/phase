# Phase 1 plan — `scripts/card-bot`: `/lfg` format option off the static choice list

Mode: phase-plan (engine-planner). Base `b9ba9360`. Charter entry: Phase 1 (r3). Skills applicable: none of the engine checklists (TypeScript bot tooling; no engine, parser, wire, WaitingFor or GameAction surface). `add-engine-variant` does not apply (no enum variant).

## Step 0 — premise
No card is involved. The premise is the Discord limit (25 static `choices` per option). Repo evidence: `scripts/card-bot/register.ts` builds `choices: FORMATS.map(...)` for `/lfg format`; `__tests__/formats.test.ts` pins `FORMATS.length <= 25`. At base `FORMATS` has exactly 25 entries (Standard ... Momir), so Phase 2's 26th would exceed the cap. Measured: `git grep -n 'choices' -- scripts/card-bot` shows `register.ts` `FORMATS.map` (line ~45) is the only static list derived from `FORMATS`; the other `choices` (card `build`, lfg `mode`, lfg `build`) are 2-3 element lists not derived from `FORMATS`.

## Measured baseline
`bun test scripts/card-bot` at base: 152 pass, 0 fail, 9 files, ~0.8 s (`ci.yml` job `card-bot-test` runs exactly this command).
Measured dispatch fact: `index.ts::handleInteraction` routes `APPLICATION_COMMAND_AUTOCOMPLETE` by command name (`interaction.data.name === "lfg" ? lfgAutocomplete(...) : autocomplete(...)`), not by option name. `lfgAutocomplete` already branches on `focused?.name === "server"`, so a `format` branch needs no change in `index.ts` (verify-only, not edited).
`register.ts` runs `registerGuildCommands` at top level (side effect on import), so a test cannot import its command object; hence the option definition moves to an importable module (below).

## Sizing
- Units: 1 — "the `/lfg` `format` option is served by autocomplete instead of static choices". Registration surfaces: the option shape (`register.ts`), the autocomplete branch (`lfgInteractions.ts`). Discriminating tests: option declares no `choices` (formats.test.ts); autocomplete `format` behavior (interactions.test.ts).
- Dependency edges: none between units (one unit). Downstream: Phase 2 appends the Dandan `FORMATS` mirror entry; it consumes this phase unchanged.
- Scope-path count (phase-fit counting rule): 4 — `scripts/card-bot/register.ts`, `scripts/card-bot/lfgInteractions.ts`, `scripts/card-bot/__tests__/formats.test.ts`, `scripts/card-bot/__tests__/interactions.test.ts`. No generated artifacts, no mirrors, no fixtures. `scripts/card-bot/index.ts` verify-only (not counted, not edited).
- T1 fails (1 unit), T2 fails (4 paths). Small-change lane: qualifies (1 unit, 4 counted paths, no enum variant, no serialized surface, no WaitingFor/GameAction change). Estimated ~110 LOC changed including tests.

## Pattern Coverage
Class: every format the client registry will ever hold, not the 25 today. After this phase the registry can grow past Discord's 25-choice cap without touching the bot's command shape (Phase 2's Dandan is the first consumer; any later format likewise). Charter-level class attribution: phase 1 is infrastructure for the 26th+ entries; it covers 0 cards by itself by construction. Bot behavior for the existing 25 formats is unchanged.

## Building Blocks
- `FORMATS` / `findFormat` (`formats.ts`), unchanged: the autocomplete source and the submit-side resolver.
- `stringOption`, `jsonResponse`, `ResponseType.APPLICATION_COMMAND_AUTOCOMPLETE_RESULT`, `OptionType` (`discord.ts`), already used by `lfgAutocomplete`.
- `MAX_AUTOCOMPLETE_CHOICES` (25) and `MAX_CHOICE_NAME_LENGTH` (100) constants already in `lfgInteractions.ts`.
- The existing `server` branch's query normalization (`trim().toLowerCase()` of the focused value) is factored into one local helper shared by both branches (no duplicate).
- The prefix-first-then-contains ranking mirrors `coverageData.ts::suggestNames`; it is re-expressed over the small in-memory list, not imported (`coverageData` is the card-name cache).
No new module and no new dependency (the bot's Docker context is `scripts/card-bot` only).

## Logic Placement
- Option shape: exported constant `LFG_FORMAT_OPTION` in `lfgInteractions.ts` (beside the handler that serves it, importable without `register.ts`'s side effect); `register.ts` places it first in `lfgCommand.options` (Discord requires required options first, preserved).
- Ranking/filter: a pure function `suggestFormats(query: string): Array<{ name: string; value: string }>` in `lfgInteractions.ts`, called from the `format` branch of `lfgAutocomplete`.
- The engine/client own no part of this; there is no game logic. The client registry remains the source of truth via `formats.ts` mirror equality (`formats.test.ts`, unchanged).

## Rust Idioms
N/A (TypeScript). TS analogues: `as const` on the option object so `autocomplete: true` is a literal type; `switch`/branch on the focused option name with the empty-choices default preserved; no new booleans in data modeling.

## Nom Compliance
N/A — no file under `crates/engine/src/parser/` changes.

## Extension vs Creation
Extends the existing `server` autocomplete mechanism (same function, same response shape, sibling branch). Creates one exported constant and one pure helper; both justified by testability (register.ts is not importable) and by keeping the 25-cap and ranking in one place.

## Analogous Trace
Traced `/lfg server` autocomplete: `register.ts` (`server` option with `autocomplete: true`) → `index.ts::handleInteraction` (`APPLICATION_COMMAND_AUTOCOMPLETE` routes by command name to `lfgAutocomplete`) → `lfgInteractions.ts::lfgAutocomplete` (`focused?.name === "server"` branch, `.slice(0, MAX_AUTOCOMPLETE_CHOICES)`, `jsonResponse` of `APPLICATION_COMMAND_AUTOCOMPLETE_RESULT`) → `__tests__/interactions.test.ts` "/lfg server autocomplete" describe (uses `command`, `opt(..., focused)`, `body`). Submit side: `lfgCommand` → `findFormat(stringOption(options, "format") ?? "")` → `"Unknown format."` refusal.

## Variant Discoverability
No enum variant added. Not applicable.

## Implementation steps
1. `scripts/card-bot/lfgInteractions.ts`
   a. Import `OptionType` from `./discord` (already imports from there) and `FORMATS` from `./formats`.
   b. Export `LFG_FORMAT_OPTION = { type: OptionType.STRING, name: "format", description: "Game format", required: true, autocomplete: true } as const` with a one-sentence comment: Discord caps static `choices` at 25, so the registry-sized format list is served by autocomplete.
   c. Add `suggestFormats(query)`: normalize `trim().toLowerCase()`; empty query returns the first `MAX_AUTOCOMPLETE_CHOICES` registry entries; otherwise entries whose label or key starts with the query first, then entries whose label or key contains it, each tier in registry order, sliced to `MAX_AUTOCOMPLETE_CHOICES`; map to `{ name: f.label.slice(0, MAX_CHOICE_NAME_LENGTH), value: f.format }`.
   d. In `lfgAutocomplete`, add a `focused?.name === "format"` branch setting `choices = suggestFormats(<focused text>)`; keep the `server` branch behavior byte-identical; share the focused-text normalization helper.
   e. `lfgCommand`'s unknown-format refusal text becomes `"Unknown format. Pick one from the suggestions."` (a typed-but-unpicked value can now reach it; static choices prevented that before).
2. `scripts/card-bot/register.ts`: replace the inline `format` option with `LFG_FORMAT_OPTION` (import from `./lfgInteractions`); drop `FORMATS` from the `./formats` import (keep `MAX_SEATS`).
3. `scripts/card-bot/__tests__/formats.test.ts`: replace "fits Discord's 25-choice cap" with a test that `LFG_FORMAT_OPTION` has `autocomplete === true`, `name === "format"` and no `choices` key, and that `register.ts` source (read with `Bun.file`, precedent: the `P2P_MAX_PEERS` test in the same file) references `LFG_FORMAT_OPTION` once and never `FORMATS` (binds the registration to the option). Mirror-equality test unchanged.
4. `scripts/card-bot/__tests__/interactions.test.ts`: add a `"/lfg format autocomplete"` describe (below): cases in the Verification Matrix.

## Verification Matrix
Command for all rows: `cd /home/user/phase && bun test scripts/card-bot` (expected: green, 152 base tests plus the added ones, none removed except the replaced cap test).

| Claim | Changed seam | Production entry | Test | Revert-failing assertion | Sibling / negative / hostile | Coverage impact |
|---|---|---|---|---|---|---|
| `format` option declares no static `choices` | `LFG_FORMAT_OPTION`, `register.ts` | `bun scripts/card-bot/register.ts` builds `lfgCommand.options` | formats.test.ts "format option is served by autocomplete" | restore `choices: FORMATS.map(...)` on the option (or in `register.ts`) and `"choices" in option` / the `\bFORMATS\b` source check fails | positive reach-guard: `autocomplete === true`, `name === "format"`, register source references `LFG_FORMAT_OPTION` exactly once (the check reads the file it names, so it is not vacuous) | none |
| mirror equality with client registry stays exact | none (`formats.ts` untouched) | n/a | existing "deep-equals the registry's projection" | n/a | unchanged | none |
| autocomplete for `format`, empty query, returns at most 25 rows, names <= 100 chars, values are `FORMATS` keys | `lfgAutocomplete` `format` branch, `suggestFormats` | `handleInteraction` type 4 with focused `format` (signed e2e also routed) | interactions.test.ts | drop `.slice(0, MAX_AUTOCOMPLETE_CHOICES)` and, once Phase 2 makes 26 formats, the length assertion fails; today it asserts `length === Math.min(FORMATS.length, 25)` and equality with the first entries of `FORMATS` | paired positive reach-guard: the existing `server` autocomplete tests stay green and return their rows (branch dispatch not shadowed); a cold/absent `focused` returns `[]` | none |
| partial query filters by prefix then contains, case-insensitive | `suggestFormats` | same | interactions.test.ts: `"com"` returns Commander before formats that merely contain it (e.g. Duel Commander) with a real registry order; `"DUEL"` uppercase returns Duel Commander; a query matching nothing returns `[]` | change the ranking to registry order only and the prefix-before-contains assertion fails | hostile: query with surrounding whitespace is trimmed; key-only match (`"HistoricBrawl"` typed as the key) resolves; label-with-space (`"tiny"` hits label `Tiny Leaders: Reborn` whose key is `TinyLeaders`) | none |
| every registry format is reachable by typing its label (so Phase 2's 26th is reachable though it falls off the empty-query 25) | `suggestFormats` | same | interactions.test.ts loops over all `FORMATS`: typing `f.label` returns a choice with `value === f.format` | slice before filter and the loop fails for any entry past position 25 once the registry has 26; today it holds at 25 and the loop is the standing guard | positive reach-guard: the loop asserts `FORMATS.length > 0` and that each iteration's response is nonempty | none |
| an autocomplete-chosen value round-trips through `findFormat` and posts | `lfgCommand` (unchanged resolution) | `/lfg` command with `format` = a choice's `value` | interactions.test.ts: take the first choice's value from `lfgAutocomplete("duel")`, submit `lfgCommand` with it, response is the public post (buttons Join/Leave/Start) | n/a (guards the value/key contract: if `suggestFormats` emitted labels as values this fails) | negative: submit the raw typed label text with a space that is not a key (`"Duel Commander"`) and get the ephemeral `"Unknown format. Pick one from the suggestions."`, no row written (paired with the positive post above proving the store write happens when valid) | none |

Hostile fixtures considered: multi-authority (two matching labels in different tiers) covered by prefix-vs-contains ordering; empty/no-match path covered; first production branch reached (`focused?.name === "format"`) is reached by every row via `focused: true` on the `format` option; no controller/owner axis exists here (unreachable, no game state).
No Oracle text is accepted with deferred semantics (no parser change).

## Reference Readings
One reference row: "mirror equality unchanged" preserves the existing `FORMATS`-vs-registry projection. Derived and measured: `bun test scripts/card-bot` at base passes that test (152 pass); the reference is a repo-internal equality, not a card reading, and there is no Oracle-text derivation to make. No parity or copied-sibling row otherwise.

## Identity / Provenance Contract
The autocomplete choice `value` is the `GameFormat` key (`LfgFormat.format`), snapshotted by Discord into the submitted `format` option string at submit time and resolved once by `findFormat`. Nothing is latched or stored beyond the existing `Lfg` row (`lfg.ts` already stores the key). A key not in `FORMATS` after a registry upgrade is the pre-existing "ended" path noted in `lfg.ts`, unchanged.

## Deferrals (allowlist)
The 26th (Dandan) mirror entry in `formats.ts` -> Phase 2 (`DEFERRED(phase 2)`; interim proof is the loop test above holding at 25, and the empty-query cap assertion written as `Math.min(FORMATS.length, 25)` so Phase 2 needs no edit here).
Operational note (not a code change): the live guild command set is re-registered by `bun scripts/card-bot/register.ts` at deploy (`deploy/card-bot-push.sh` references it); the PR does not run it.

## Probes
Probed: `bun test scripts/card-bot` baseline (152 pass). Not probed (labelled): the exact Discord server-side behavior for a non-strict autocomplete value (typed text submitted verbatim) — taken from Discord's documented behavior, not measured; the plan's refusal text handles that case.
