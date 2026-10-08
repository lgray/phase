# Phase 21 implementation review, round 1 (phase mode, checkpoint)

Reviewer model: claude-opus-5-5. Skill: review-engine-impl at 1ecbc840cf (read from the phase-rs-workdir checkout).

Review Head: d12daf57938f9d9db5762f20ed36ff1d4c76701e
Completion Gate: FAIL (incomplete, not red): the p21c coverage pair had not finished at review time (`cov-base/coverage.json` empty, base generation still running). Every other check passed at the candidate SHA: fmt, protocol gate, clippy, bindings, and a full nextest run with 38206 tests, all passing. The frontend vitest run passed 604 files. The orchestrator closes the gate when cov-base finishes; parser files are unchanged, and the two `card-data.json` files are the same size.
Maintainer-Simulation Gate: PASS. All four seam rows (pile choice, empty rule, wasm seat loop, export) are complete. U1, the UI-chosen pile reaching the engine end to end, is `DEFERRED(phase 22)`.

Range: `git diff 2bfcb784c2..d12daf5793` is one commit, and its 10 paths equal `scope.nul`. The executor started from a10c2316dd. The four production files at the candidate have the same sha256 as the executor's mutation baseline (`logs/mutations.txt`), so the rebase onto the maintainer merge did not change the executor's content.

## Verdict

0 behavior, 0 machinery, 1 text (LOW, non-blocking). Pre-existing: 1.

## Expected behaviour (derived before reading the code)

The format announcement, the charter's Phase 21 entry and the USER addition give the following:
- One pile per game: the pile seat's submitted 80 cards, or the Secret Lair 80 when it submits none.
- Any resolvable 80 is legal, with no ban list and no copy limit.
- The CR 407.3 ante refusal still applies ("When not playing for ante, players can't include these cards in their decks or sideboards").
- Per CR 103.3 ("The players' decks become their libraries"), the pile becomes the one shared library and is shuffled.
- Other seats' submissions are ignored.
- An empty submission is admitted wherever the engine supplies the deck.

The code matches this.

## Design

The typed `DeckSupply { PlayerBuilt, EngineFixed, HostPile }` is one closed axis, not a bool, with no shared name root. `deck_supply` is an exhaustive match with no wildcard, and the census enforces it.

`supplies_fixed_deck()` is derived as `deck_supply() != PlayerBuilt`. The method value and the registry's stored field agree for every format (E5b).

Single authorities:
- The loader `dandan_pile_payload` keeps the existing format-keyed branch, per plan decision 6.
- The empty rule lives in `validate_deck_for_format` and reads `selected.tag().deck_supply()`. There is no format literal at the validator.
- The wasm export is a one-line `map_or` over the axis.

`ReplayHeader`, wire types and protocol constants are untouched. The d.ts diff is the declaration plus the `InitOutput` member: 5 changed lines.

## Class sweep (by command)

`git grep -n -E 'load_and_hydrate_decks\(|validate_deck_for_format\(|validate_name_deck_for_format_full\(|evaluate_deck_format_gate\(|evaluate_deck_compatibility\(' d12daf5793 -- crates ':!crates/engine/tests'`

Each production member is either changed or shown to deliver the pile:
- **wasm `initialize_game_impl`**, which serves both local and multiplayer-host games: `validate_deck_list_seats` now validates every seat. The loader then runs, and the header records the submitted `DeckList`.
- **phase-server create handler and `ai_seat_setups`**: both call `validate_name_deck_for_format_full`, which reaches the new rule. Covered by S1 and S2.
- **`evaluate_deck_format_gate`** (the P2P guest gate): reaches the rule; covered by E2-gate.
- **`server-core::start_game`**: loads `decks[0]` as `payload.player`, and seat 0 is `SeatImmutable`.
- **`replay::reconstruct_initial_state`**: goes through the same loader; covered by E4b.
- **phase-ai bins and `duel_suite`**: build no Dandan state.
- **`evaluate_deck_compatibility`** (the UI hint): does not route through the gate, so it still calls an empty Dandan deck incompatible. Phase 22 reads `deck_supply` for the pile choice and bypasses the deck requirement, so nothing depends on the hint here.

Members I tried to construct, each refuted:
- A guest seat reaching seat 0 through a seat delta: seat 0 is immutable.
- A post-create AI-seat `SetKind` deck: it only fills seat 1, which the loader ignores.
- A restored not-yet-started room: it rebuilds seat 0 from its `DeckChoice`.
- `deck: None` / `Named` AI seats: these stay refused. The plan's no-sender verdict holds.

I could not build a start path that bypasses the loader or the gate.

## Discriminating rows (my probes, in place on target-dandan)

The worktree started at `porcelain=[]` with HEAD equal to the candidate. Originals were copied to `scratch/rv21-orig`, restored with cp and touched, and checked by sha256 (all OK). The tree ended at `porcelain=[]` with HEAD == d12daf5793.

| Probe | Production arm reverted | Red | Green (reach guards / preservation) |
|---|---|---|---|
| P1 | M1 loader always default + M3 wasm skip restored | E1a/E1c, E1d, E4a, E4b (M1); W1, W2 (M3) | E1b (preservation), E5c, E2*, W3, census |
| P2 | M2 empty rule removed + M4 `Dandan => EngineFixed` | E2 validator, E2 gate, E2s, S1, S2, W1, W2 (M2); W3, E5a (M4) | `full_mode_create_rejects_format_invalid_host_deck` (Standard stays `DeckRejected`), E5b, E5c, E1b |
| P3 | M5 whole pile-seat payload cloned when non-empty, default otherwise | E1d only | E1a, E1b, E4a, E4b, E2* |
| Green | all restored | — | 51/51 (filter covering every row plus phase-server create/AI-seat modules) |

Notes on the probes:
- Three production arms were reverted (loader, validator rule, wasm skip), plus M4 and M5.
- In P1/P2 the two mutations touch disjoint rows. The W rows use the synthetic db, so they read only the validator and the wasm loop.
- E4b's pile assertion is red under M1. Its replay == live check is labelled preservation, which closes T3.

## Plan-review constraints

- T1: the d.ts adds only the declaration and the `InitOutput` member. No drift hunk.
- T2: executor report §12 carries the replacement PR-body wording, and `addenda/phase-22` holds both review lines.
- T3: closed as described above.

All three are closed.

## Momir

The behaviour change is intended, per the charter's Phase 21 seam note, and is covered:
- An empty submission is accepted: E2s, S2, W2.
- A non-empty submission is validated at wasm: W2, where 60 Plains are refused.

## Gates

- **Protocol check:** `node scripts/check-protocol-version.mjs` rc 0. Control: a scratch copy of every file the script reads, with lobby-broker `PROTOCOL_VERSION` set to 99999, gives "Protocol version mismatch: Rust=99999, client=118" and rc 1.
- **CR citations:** the added citations are CR 103.3, 103.4, 400.1 and 407.3. Each was grepped in `docs/MagicCompRules.txt`, and its subject matches the annotated text.
- **Parser:** no parser file changed.

## Findings

**[LOW]** The wasm export doc names a consumer that does not exist at this commit. Evidence: `crates/engine-wasm/src/lib.rs` doc on `deck_supply_for_format`, copied into `client/src/wasm/engine_wasm.d.ts`. Why it matters: the CLAUDE.md comment rule keeps caller behaviour out of comments, and the lobby offers no pile choice until Phase 22. Suggested fix: when lib.rs is next regenerated, replace "Who supplies `format`'s deck; the lobby offers a pile choice only for `HostPile`." with "Who supplies `format`'s deck." [text], exit-round material handed forward as a constraint. It needs no round.

## Pre-existing

`crates/engine/src/types/custom_format.rs`, the doc on `from_lobby_config`: "the only built-in that sets it (Momir) is likewise rejected below". This is false since Phase 2: `FormatConfig::dandan()` also sets `supplies_fixed_deck: true`. It reproduces at 2bfcb784c2. Not blocking; for the owner's triage.
