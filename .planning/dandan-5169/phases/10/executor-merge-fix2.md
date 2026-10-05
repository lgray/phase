Mode: implementation/fix. BASE_SHA/START_SHA: ac7d00e6320b275f5a228456a5a6eff6802f37b9. IMPLEMENTATION_WORKTREE: /home/lgray/vibe-coding/dandan-run/wt-dandan (clean, HEAD==START_SHA, nothing staged at start and end).

Diff: crates/lobby-broker/src/protocol.rs, test `protocol_version_tracks_full_game_wire_additions`: `MIN_SUPPORTED_PROTOCOL` literal 105 -> 107 (= PROTOCOL_VERSION 108 - 1). Only delta; authorized path only.

PREPARATORY (not completion evidence): red at START_SHA (left 107 right 105, 1 failed); after fix `nextest -p lobby-broker` 291/291 pass; `cargo fmt` on the path rc 0; `clippy -p lobby-broker --all-targets -D warnings` rc 0; `node scripts/check-protocol-version.mjs` rc 0. Logs: .planning/dandan-5169/mf2-red.log, mf2-green.log.

Class walk: grep over crates/*.rs and client/src tests for assert/literal pairs with PROTOCOL_VERSION|MIN_SUPPORTED_*|WIRE_PROTOCOL_VERSION|LOBBY_PROTOCOL_VERSION. Members: lobby-broker protocol.rs LOBBY 16 / MIN_LOBBY 2 / PROTOCOL 108 (match constants), server-core protocol.rs PROTOCOL 108 and LOBBY_MIN == PROTOCOL-1 (derived), others compare to the constant. Only MIN_SUPPORTED_PROTOCOL 105 was stale; single occurrence. Client tests use the constants, no literals.

Gates: no parser change; no behavioral change, so no coverage map/matrix rows; no new CR citations; no new fields.
Stop-and-return: none. Risks: none.
