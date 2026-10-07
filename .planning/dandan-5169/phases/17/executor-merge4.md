# Executor report — upstream merge #4 (mode implementation/fix, tree mid-merge, HEAD 2796efca82, upstream/main 4ab8245808)

## Protocol restack (upstream entry untouched at 108 / wire 90 / script +37,+36; ours stacked above)
Final: PROTOCOL_VERSION 115, lobby MIN_SUPPORTED_PROTOCOL = PROTOCOL-1 (114, lobby-broker; server-core MIN stays `PROTOCOL_VERSION` as in HEAD, the brief's PROTOCOL-1 applies to the lobby-broker const and its pin), WIRE 97, LOBBY 16. Script EXPECTED = UPSTREAM(71)+44 / 54+43 with seven renumbered `// +N:` lines each below upstream's +37 / +36 lines.
Hunks (all: ours shifted +1 for entry numbers, `vNNN`/`vNN` peer tokens, `wire NN`/`full-game protocol NNN`, then upstream's entry appended in place):
- lobby-broker/protocol.rs: history block, PROTOCOL_VERSION, two pin asserts (115 / 114).
- server-core/protocol.rs: test doc block, `protocol_version_is_115_for_shared_piles_view` + assert + REVERT-PROBE reference.
- client network/protocol.ts: wire history (97..91 above upstream's 90), WIRE_PROTOCOL_VERSION 97; protocol.test.ts title/literal v97.
- adapter/ws-adapter.ts: history, PROTOCOL_VERSION 115. p2p-adapter-multiplayer.test.ts: comment, title (v96 / v97), `setupFrameAt(96/97)`.
- scripts/check-protocol-version.mjs: both conflict blocks, plus non-conflict header lines `// v108..v114` -> v109..v115 (describing our entries).
Numeral sweep (regex over lobby-broker, server-core, client/src/network, client/src/adapter, phase-server, engine-wasm, scripts for `\bv(9x|10x|11x)\b`, `wire|protocol|full-game N`): every hit accounted. Misses found and fixed after the first pass: p2p test `v95/v96` tokens (the `\b` trap, caught by the script rc=1), wrapped `protocol 110.`/`protocol 108.` in protocol.ts (-> 111 / 109), and three pre-existing stale pointers to the Dandan entry as 105 (lobby-broker lobby-16 entry x2, ws-adapter lobby-16 entry) -> 109. Remaining hits are upstream/earlier entries and CR numbers (103.5, 108.3, 115.1) and are correct.

## layers.rs (1 hunk, order_with_dependencies)
HEAD = our effect-node graph + `dependency_application_order` (CR 613.8a/b loop-only timestamp fallback); upstream = comment rewrite on the old Kahn loop plus new `apply_ability_effects_with_referenced_grants` (auto-merged below, unchanged). Resolution: keep HEAD's code; comment replaced by one that merges both intents (graph is state-blind so compute-once is exact; live-provider buckets go through the one-at-a-time selector). Dropped upstream's "older path falls back for the whole bucket" sentence: false on our code.

## scoped_library_search.rs (1 hunk)
Upstream added `illegal_targets_disposition.is_does_not_resolve()` to the field list of `is_plain_parent_target_delivery`; that list is now `has_no_resolution_riders` on our side (already holds upstream's other listed fields). Resolution: no field list restored; the disposition conjunct added to `has_no_resolution_riders` (shared by scoped search, draw.rs, effects/mod.rs standalone_view callers; StillResolves is a resolution rider a shortcut would drop), doc updated, and an `illegal_targets_disposition` row added to `has_no_resolution_riders_accepts_a_bare_effect_and_refuses_each_rider` (array 7 -> 8).

## Semantic merge breakage outside the conflicted list
text_substitution.rs `rewrite_resolved_ability` is field-exhaustive; upstream's new `ResolvedAbility.illegal_targets_disposition` broke the build. Added `illegal_targets_disposition: _` (rules-state, never text). No other site failed `cargo check -p phase-engine --all-targets`.

## Fixture
First regen attempt ran against the build error above (gen rc=101, stale card-data, fixture 10 cards short). After the fix: regen-carddata.sh gen rc=0; `python3 scripts/gen-test-fixture.py` wrote 5068 cards; `--check` rc 0. Unstaged diff shows only the expected files (no drift in feeds/decks).

## Verification (PREPARATORY, not completion)
- `node scripts/check-protocol-version.mjs` rc 0; control (scratch copy of the files it reads, lobby PROTOCOL_VERSION 114): `Protocol version mismatch: Rust=114, client=115`, rc 1. Scratch removed.
- `cargo fmt --all -- --check` rc 0. `cargo clippy -p phase-engine --all-targets -- -D warnings` rc 0.
- `cargo nextest run -p lobby-broker -p server-core`: 805 passed.
- `cargo nextest run -p phase-engine --features test-support -E 'test(scoped_library_search) | test(layers) | test(dandan_wheel_split) | test(dandan_simultaneous_draw) | test(census)'`: 800 passed (includes the new riders row and dandan_simultaneous_draw v6a/v6b).
- vitest protocol.test.ts 57 passed; p2p-adapter-multiplayer `-t "wire protocol"` 2 passed.
- conflict markers: 0 in all ten text files; the binary fixture is regenerated, not merged.
- Riders-row mutation probe: see final line below.

## Judgement calls / risks
- Disposition conjunct placed in the shared predicate, not re-added to the delivery list: widens refusal for the other two callers only for StillResolves abilities.
- The staged index still marks the ten files UU; the orchestrator must `git add` them (fixture, script, and ws-adapter files included). Nothing was staged or committed by me.
- p2p wire-gate: only the `-t "wire protocol"` slice was run, not the whole 156-test file.
- Riders-row mutation probe: deleting the new conjunct in `has_no_resolution_riders` reds `has_no_resolution_riders_accepts_a_bare_effect_and_refuses_each_rider` (FAIL, rc 100); restored and touched, file byte-identical to the green run.
