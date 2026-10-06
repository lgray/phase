# executor-merge3 (upstream merge #3 protocol restack)
Mode: implementation/fix (merge-conflict resolution). Worktree wt-dandan, HEAD 630000f3c5, mid-merge; no git state changed.

## Result
7 files resolved, 0 conflict markers. PROTOCOL_VERSION 113 (lobby-broker const; server-core derives it), MIN_SUPPORTED_PROTOCOL 112 (lobby: PROTOCOL-1; server-core: == PROTOCOL), WIRE 95, LOBBY 16.
Script: EXPECTED_PROTOCOL = 71 + 42, EXPECTED_WIRE = 54 + 41; upstream +36/+35 lines kept, ours appended as +37..+42 / +36..+41.

## Numeral/title hits
- Live literals changed: lobby-broker `PROTOCOL_VERSION = 113`, test asserts 113 / MIN 112; server-core test fn `protocol_version_is_113_for_simultaneous_draw_dealer` (+ its doc reference, check script requires numeral in the name); client protocol.ts WIRE 95; ws-adapter PROTOCOL_VERSION 113; protocol.test.ts title+toBe v95; p2p test title (v94)/(v95), `setupFrameAt(94)` refusing / `(95)` admitting, comment `(95 -> 94)`.
- History text: our entries renumbered +1 in lobby-broker, ws-adapter, client protocol.ts, server-core test doc, script header (v105/106/109..112 -> v108/109/110..113), incl. in-entry "vN peer", "wire N", "full-game protocol N". Upstream's 107 / wire 89 entries untouched, placed below ours (server-core: between ours and the delve v104 entry).
- Remaining hits for 106..113: CR numbers (CR 106.x/107.x/108.3/111.x/112.2/113.x), `card-bot` ids "111", unrelated line counts (phase-server), `network/peer.ts:69-106`, upstream history entries (v106 replacement-choice, v9x). None are live.
- Sweep of `(v|_)(9x|10[5-9]|11[0-3])` tokens over server-core/lobby-broker/phase-server/engine-wasm/client network+adapter: only upstream history entries remain. phase-server/engine-wasm/scripts: no asserted protocol numerals.

## Judgement calls
- Pre-existing stale numerals in our entries (script header and server-core doc: Dandan v105/v104-peer, Substitute v106/v105-peer, `+36 "the v105"`, p2p comment "refuses v92") were set to the correct restacked values (Dandan v108/v107-peer, Substitute v109/v108-peer, "refuses v94") instead of mechanical +1.

## Verification
- `node scripts/check-protocol-version.mjs` rc 0. Control: lobby-broker PROTOCOL_VERSION temporarily 112 -> `Protocol version mismatch: Rust=112, client=113`; restored (cmp) + touched; rc 0 again. (server-core const is `= lobby_broker::PROTOCOL_VERSION`, so the control goes through lobby-broker.)
- vitest protocol.test.ts 57 pass; p2p-adapter-multiplayer `-t "wire protocol"` 2 pass (run with --coverage.enabled=false).
- `cargo nextest run -p lobby-broker -p server-core`: 805/805 pass (log: .planning/dandan-5169/merge3-nextest.log) ONLY with the stop item below applied temporarily; as the tree stands the engine does not compile.

## Stop item (outside the 7 files; reverted by me)
Semantic merge break, not a text conflict: upstream added an exhaustive `match` in crates/engine/src/game/ability_utils.rs `immediate_modification_target_slot_filter` (E0004: `ContinuousModification::SubstituteTextWord { .. }` not covered). Smallest fix: append `| ContinuousModification::SubstituteTextWord { .. }` to the final `| ContinuousModification::RemoveManaCost => None` arm group (this is the only E0004 in lobby-broker/server-core build; other crates not compiled). I applied it to run nextest, then restored the file (cmp with backup).

## Addendum: approved merge-induced exhaustive-match arms
Class walk = `cargo check --workspace --all-targets` (compiler-found). Two members, rc 0 after:
- crates/engine/src/game/ability_utils.rs `immediate_modification_target_slot_filter`: added `| ContinuousModification::SubstituteTextWord { .. }` to the `=> None` arm group (re-applied, kept).
- crates/phase-llm/src/format_guidance.rs `format_strategy` (upstream-new match over GameFormat, E0004 `Dandan`): `GameFormat::Dandan` added to the "no format-specific approach" `return None` group (generic strategy; Dandan has no format-specific guidance text). Follow-on literals so the existing registry-iterating test stays valid: `GameFormat::Dandan` added to `expects_generic` in `only_freeform_formats_take_the_generic_strategy`, and to the module doc's generic-strategy list. Judgement call: if Dandan should get its own strategy text, that is a separate content decision.
Verification: `cargo check --workspace --all-targets` rc 0; `cargo clippy --workspace --all-targets -- -D warnings` rc 0; `cargo fmt --all -- --check` rc 0; `cargo nextest run -p phase-llm` rc 0 (only_freeform test included). Earlier nextest lobby-broker+server-core 805/805 stands.
