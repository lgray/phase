# executor-merge5 (upstream/main b55c6faa02 into HEAD 1c2bd62946)

Mode: implementation/fix (merge resolution). Nothing staged or committed by me. Conflict markers: 0 in all 8 files.

## Protocol restack (upstream +38 full / +37 wire / lobby 15; ours stacks 7 full + 7 wire on top)
Re-measured: HEAD script +44/+43/lobby 16; upstream/main +38/+37/15. Upstream entry (#9423 granter binding) is v109 / wire 91, untouched; ours renumbered by exactly +1.
- Result: PROTOCOL_VERSION 116, WIRE_PROTOCOL_VERSION 98, MIN_SUPPORTED_PROTOCOL = PROTOCOL_VERSION-1 (115 in the broker test), LOBBY 16 (unchanged), script EXPECTED_PROTOCOL_VERSION = UPSTREAM_MAIN_FULL(71)+45, EXPECTED_WIRE = PHASE_TWO_BASE(54)+44, EXPECTED_LOBBY 16.
- scripts/check-protocol-version.mjs: v109 granter, v110..v116 ours (comment block); `// +38` granter then `+39..+45`; wire `+37` granter then `+38..+44`.
- crates/lobby-broker/src/protocol.rs: changelog 116..110 ours (heading, vNNN peers, wire +1; CR 103.5/108.3/612.1 and RESOLUTION_STATE_WIRE_VERSION 4->5 left), 109 upstream; PROTOCOL_VERSION 116; test asserts 116 / 115; lobby-16 entry now says `PROTOCOL_VERSION` 110 and `(110)`.
- crates/server-core/src/protocol.rs: doc paragraphs for ours shifted (peer vN -> vN+1, state vN -> vN+1), upstream's granter paragraph below ours; test `protocol_version_is_116_for_shared_piles_view`, assert 116, revert-probe doc name updated.
- client/src/network/protocol.ts: WIRE 98; wire history 98..92 ours (heading, vNN peer, "full-game protocol" +1), 91 upstream.
- client/src/adapter/ws-adapter.ts: PROTOCOL_VERSION 116; entries 116..110 ours (heading, vNNN, "Wire NN"), 109 upstream; lobby-16 entry "(see PROTOCOL_VERSION 110)" (found by the sweep; was a stale 109).
- client/src/network/__tests__/protocol.test.ts: title v98, toBe(98).
- client/src/adapter/__tests__/p2p-adapter-multiplayer.test.ts: comment (98 -> 97), title (v97)/(v98), setupFrameAt(97)/(98).

### Numeral sweep (instrument: three greps over crates/{server-core,lobby-broker,phase-server,engine-wasm}, client/src/{network,adapter}, scripts; plus .github, deploy, docs)
- Tokens `v109..v117`/`_1NN_` names, `protocol_version_is_N`, "pins ... to vN": hits only in the edited files above (all accounted) plus the two vitest titles; no other file.
- Asserted numerals near PROTOCOL/WIRE constants (`PROTOCOL_VERSION|WIRE_PROTOCOL_VERSION|MIN_SUPPORTED_PROTOCOL` with 95-99/109-117, `toBe(`, `wireProtocolVersion:`, `setupFrameAt(`): all hits in edited files; `setupFrameAt(WIRE_PROTOCOL_VERSION)` sites are symbolic. Remaining 109/91/90 hits are upstream's own entries.
- `\b109\b` sweep over client/src, crates, scripts, docs: remaining = upstream entries (ws-adapter 242, protocol.ts 142, lobby-broker 70) and unrelated (draft drag test, wasm bytes, card_db.rs docs).
- `\b110..115\b` / `\b92..97\b` near protocol|wire|version: only CR 111.2/115.1d (unrelated) and the p2p comment.
- deploy.yml / helm values / manabrew-compat CLAUDE.md read the constant by regex or mention it generically: no numerals to move.
- Control: Rust PROTOCOL_VERSION temporarily 115 -> `Protocol version mismatch: Rust=115, client=116`; restored (cmp from backup, touched), rc 0.

## filter.rs (3 conflict hunks, all one shape)
Upstream #9423 threads `granting_object` through `filter_inner_for_object` (new param before `controller_lookup`); ours had replaced the trailing `ControllerLookup::LiveOnly/LiveOrLki` with `OwnerZone` (the Dandan owner-zone read) / a passed-in `controller_lookup` (`filter_inner_with_lookup`).
- `matches_target_filter_in_owner_zone`, both calls (shared-library/owner-axis read): `ctx.granting_object, ControllerLookup::OwnerZone` (union: upstream arg + our lookup).
- `filter_inner_with_lookup`: `ctx.granting_object, controller_lookup` (union).
`git diff HEAD -- filter.rs` equals upstream's own diff (168+/31-), so no hunk of ours was lost. Exercised by `game::filter::` (210 tests), `dandan_*` (229, incl. owner-axis/scoped-count/read-sweep rows) and upstream `granted_ability_self_binding` (152) in one run.

## Semantic conflicts found by clippy (outside the conflicted files; test code of ours vs upstream signature changes)
- cost_payability.rs (2 calls) `eligible_exile_cost_objects(.., source, None /*granting_object*/, zone, ..)`; costs.rs `find_eligible_exile_targets(.., source, None, zone, ..)`; layers.rs test helper `entry()` gains `granter: None`. Rustfmt'd (skip_children). These files merged textually clean, so they are unstaged (`MM`) on top of the staged merge: stage them.

## Fixture
`gen-test-fixture.py --check` failed (24 upstream cards, e.g. duskana, the rage mother). Ran regen-carddata.sh (gen rc 0), then `gen-test-fixture.py` (wrote 5092 cards), `--check` rc 0. Tracked delta: crates/engine/tests/fixtures/integration_cards.json.gz only (unstaged `M`; stage it). Nothing else tracked drifted.

## Verification (PREPARATORY, merge tip, uncommitted)
- `node scripts/check-protocol-version.mjs` rc 0 (+ control above); `cargo fmt --all -- --check` rc 0.
- `cargo nextest run -p lobby-broker -p server-core`: 805 passed.
- `cargo clippy -p phase-engine --all-targets -- -D warnings`: rc 0 ("Checking ... Finished", after the 4 fixes above; the first run showed the 4 errors, so the instrument is live).
- Focused nextest, phase-engine, features phase-engine/test-support, filter `test(/game::filter::/) | test(granted_ability_self_binding) | test(announced_counter_recipient_set) | test(base_pt_designation_filter) | test(goaded_creature_under_pacifism) | test(dandan) | test(census) | test(no_top_level_test_binaries)`: 703 passed, 0 failed (per-group PASS counts nonzero: filter 210, dandan 229, census 64, granted_ability_self_binding 152, announced_counter 38, base_pt 11, goaded 8).
- vitest (coverage disabled): protocol.test.ts 57 passed; p2p-adapter-multiplayer "wire-protocol version gate" 6 passed; ws-adapter.test.ts 88 passed.
- Not run: full suites, generated-bindings check (no type changes by me).

## Judgement calls / risks
- Server-core doc comment retains a delve (#9400, v103->v104) paragraph that exists at HEAD but not upstream/main; untouched.
- Full-game constant stays defined as upstream + delta per brief; MIN_SUPPORTED_PROTOCOL not touched (code-defined).
