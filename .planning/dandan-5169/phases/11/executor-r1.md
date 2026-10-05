MODEL: claude-sonnet-5-5

# Phase 11 executor report r1 (S3 hand-entry ownership rebind)

Mode: implementation/fix (phase mode). BASE_SHA = START_SHA = b9346173849aaeb2e668e2c402a10856758cedf3. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Start check: HEAD == START_SHA, tree clean, nothing staged. End check: HEAD unchanged, nothing staged, delta = 20 modified + 1 new file, all inside scope.nul (`comm` of touched vs scope: none outside). All evidence below is PREPARATORY, not completion evidence. Nothing committed.

## 1. Diff summary
- `game/zone_pipeline.rs`: private `hand_entry_receiver` (axis match + origin = both seats resolve the object's current zone to one container); `ZoneMoveRequest::hand_taker` (Hand-gated `performed_by`); `ZoneMoveRequest::draw(object, drawer, seed)`; call in `deliver_replaced_zone_change`; doc extension; inline module `hand_entry_receiver_tests` (V9).
- `game/zones.rs`: `resolve_and_apply_zone_change_to_receiver` (new real body; `receiver` param; `rebound_from`; record owner/controller retag + `sync_trigger_source_context`); `resolve_and_apply_zone_change` kept as a receiver-free wrapper, `#[cfg(any(test, feature = "test-support"))]`; `apply_resolved_zone_change` precondition `rebound_from.unwrap_or(owner)` + `install_rebound_owner` (owner, base_controller, controller); `move_to_zone_with_entry_flags` gains `hand_receiver`; inline module `hand_entry_rebind_tests` (V10).
- `types/resolved_commands.rs`: `ResolvedZoneChangeCommand.rebound_from` (`serde(default, skip_serializing_if)`); `zone_change_command_is_invalid` clause. `types/proposed_event.rs`: doc only.
- Producers: `effects/draw.rs` (drawer = `pending.player`), `mulligan.rs::draw_n` (`.performed_by(player_id)`), `effects/explore.rs` (`.performed_by(controller)`), `effects/seek.rs` (`.hand_taker(ability.controller)`), `engine_resolution_choices.rs` DigChoice kept map (`.hand_taker(player)`, the only edit there).
- Compiler-forced literals: `game/log.rs`, `tests/integration/unmaterialized_lki_serialization.rs` (`rebound_from: None`).
- Protocol 108->109, wire 90->91, lobby 16 unchanged: lobby-broker + server-core `protocol.rs` (pin test renamed `protocol_version_is_109_for_hand_entry_ownership_rebind`, `MIN_SUPPORTED_PROTOCOL` assert 107->108), `ws-adapter.ts`, `network/protocol.ts`, `protocol.test.ts`, `p2p-adapter-multiplayer.test.ts`, `scripts/check-protocol-version.mjs` (+38 / +37).
- Tests: new `tests/integration/dandan_hand_entry_ownership.rs` (11 rows) + `mod` line in `main.rs`; fixture `integration_cards.json.gz` regenerated (`--check` rc 0).

## 2. Step 0 re-measurements (plan premises replaced inside scope)
- `zone_storage_seat`, `OwnerMismatch` (zones.rs x3), `ZoneMoveRequest` without a recipient field, `performed_by` plumbing, `ChangeZone` supplying `ability.controller` at its call sites: all as planned. Phase 10's zone-door redesign does not touch these anchors; the existing `dandan_filter_owner_axis` Fengraf rows stay green.
- FALSE premise 1: `resolve_and_apply_zone_change(` has a call site the plan missed, `tests/integration/dandan_shared_pile_storage.rs` (not in scope). Adding a parameter would force an out-of-scope edit, so the receiver goes in a new `resolve_and_apply_zone_change_to_receiver` and the old name stays as a receiver-free wrapper. That wrapper has no production caller: un-gated it was a dead-code warning in the non-test-support lib build (seen in the `interaction-bindings` build), so it is `cfg(any(test, feature = "test-support"))`, the same gate as `pub mod zones`. If the driver prefers the plan's single signature, the cost is a one-line edit to `dandan_shared_pile_storage.rs` (out of list; not done).
- FALSE premise 2: `effects/dig.rs::move_mass_put_all_selected` as a Hand producer. Census: `jq` over `client/public/card-data.json` for `Dig` effects with `keep_count == null`, `keep_count_expr == null` and destination Hand or Exile returns zero rows (Library and Battlefield only; Stargaze has an X expr). No real card reaches a Hand put-all, no real-card test is possible, so the supply is dropped (not edited, in scope but unchanged). Reachable-class evidence is the command above.
- FALSE premise 3: the plan's `receiver.filter(|_| to == Zone::Hand)` after the Attraction redirect. The raw Command/Stack branch never reads the receiver, and the seam only returns `Some` for a Hand `to`, so the filter is dead code (its mutant was equivalent); deleted, the invariant is stated in `move_to_zone_with_entry_flags`' doc.
- FALSE premise 4: Adherent's Heirloom as the Seek census row. `filter.rs::most_prevalent_creature_types_in_zone` filters `obj.owner != owner` over the shared library and needs `state.all_creature_types`, so a pile card owned by the other seat never matches (pre-existing Dandan read-sweep defect, reproduces at base; not in the 23-card list, not depended on). Row uses real Hollowhenge Wrangler ("seek a land card", `Seek` destination Hand). Reported once as pre-existing: owner `filter.rs`/Phase 10 or 8.
- Plan edit not made: the doc extension of `PendingBatchZoneMoveRequest.performed_by` (`types/game_state.rs`, not in scope; comment-only).
- Protocol numbers re-derived: upstream+delta; script was `+37` / `+36` => `+38` / `+37`. Pin names/titles grepped (`protocol_version_is_108_*`, "pins the P2P wire protocol to v90", the P2P gate title and literals 89/90 moved to 90/91). Red leg: with the five Rust/TS constants bumped and the script untouched, `node scripts/check-protocol-version.mjs` printed "Protocol version must remain 108: Rust=109, client=109"; green after the script edit (rc 0). vitest `protocol.test.ts` + `p2p-adapter-multiplayer.test.ts`: 211 passed.
- Gonti base value (V4b): `exiled_by == None` at START_SHA (row passed on the unedited tree: `red-base.log`).

## 3. PREPARATORY verification (no Tilt in this checkout; direct, isolated cargo)
- `cargo fmt --all -- <the 17 scope .rs paths>`: applied, delta stays in scope.
- `cargo clippy -p phase-engine --lib -- -D warnings` rc 0; `cargo clippy --workspace --all-targets -- -D warnings` rc 0 (after fixing one unused import of mine; first run `clippy.log` was red on it).
- `./scripts/check-interaction-bindings.sh --check` rc 0.
- Full `nextest -p phase-engine`: 32726 passed, 0 failed (`nextest-engine.log`); lobby-broker + server-core: 805 passed. These ran before two last trivial edits (the `cfg` on the wrapper, one unused import in my new test file); after them: clippy both legs rc 0 and targeted nextest (`hand_entry|dandan_|cr733|unmaterialized_lki`) 265 passed.
- `python3 scripts/gen-test-fixture.py --check` rc 0.

## 4. Parser gate
No file under `crates/engine/src/parser/` is touched: n/a.

## 5. Discriminating-test gate (production-path coverage map)
Red at base (`red-base.log`, new rows on the unedited tree): V1, V2, V3, V4, V5, V6, V7a, V7b, V8 red for the right reason (the card lands in P0's hand / P0 holds 14 cards); V4b and the V1 guard pass at base by design (preservation / positive). Green at the candidate: 11/11 + 11 inline.
Mutations (each applied, run, restored from a byte backup; `mutation-summary.txt`, `mut-*.log`; control = green2.log, 22 passed):

| Claim | Seam | Entry / test | Revert-failing mutant (red tests) | Siblings / negatives |
|---|---|---|---|---|
| Draw owner = drawer | `ZoneMoveRequest::draw` -> `hand_entry_receiver` | `v1_brainstorm_draw...`, `v2_...`, `v6_...`, `v8_...` (real Brainstorm, Lonely Sandbar cycle, Surgical Bay) | M1 (constructor without taker): v1, v2, v6, v8 | `v1_brainstorm_from_the_pile_holder_and_in_standard_are_unchanged` (P0 and Standard) |
| Opening deal / mulligan redraw | `mulligan.rs::draw_n` | `v3_opening_deal_and_mulligan_redraw...` (boot + `MulliganDecision`) | M2: v3 | P0's hand untouched |
| Dig to hand (Telling Time) | DigChoice kept map | `v4_telling_time...` (both seats) | M3: v4 | `v4b_dig_to_exile_does_not_name_a_performer` (real Gonti, `exiled_by == None`, owner unchanged) red under M6 (Hand gate deleted) |
| ChangeZone gy->hand (Haunted Fengraf; source sacrificed as cost) | `change_zone.rs` supplies `ability.controller` | `v5_haunted_fengraf...` (both seats) | M2' (seam returns None): v5 + 12 more | acting seat P0 positive |
| Control Magic / Unsubstantiate chain | owner vs controller | `v6_a_stolen_drawn_creature...` | M1: red at first assertion | asserts controller P0 / owner P1 after steal; never in P0's hand |
| Explore, Seek | explore/seek producers | `v7_explore...`, `v7_seek...` | M11, M10 | |
| Hand invariant | all | `v8_every_card_in_a_hand...` (Brainstorm + Fengraf in one game; reach: P1 holds P0-staged cards) | M1, M2', M4, M5 | |
| Axis / origin / carrier / redirect | `hand_entry_receiver`, batch pause | V9 inline: axis (Standard vs Dandan), per-seat origin keeps owner, taker == owner journals no rebind, Attraction redirect, serialized pause + redirect, declined redirect | Mb (origin gate deleted): origin row; Md (`from_pending` drops taker): both pause rows; Mc: 3 rows; M2': axis, Attraction, pause rows | |
| Journal / replay | `resolve_and_apply_..._to_receiver`, `apply_resolved_zone_change`, validator | V10 inline (replay equality, `OwnerMismatch{P0,P1}`, later move keeps owner as controller, validator refuses same-owner / non-Hand / non-shared-origin forms, serde omit/default) | M4: 18 red; M5: 14; M7 (record not retagged): 8; M8 (base_controller): 2; M9 (validator clause): 1 | |

Killed mutants: M1, M2, M2', M3, M6, Mb, Mc, Md, M4, M5, M7, M8, M9, M10, M11 (15).
NOT killed (equivalent mutants, no discriminating fixture exists):
- M3' axis gate deleted (`OwnerKept => {}`): red = 0. `shared_zones()` and `hand_entry_ownership()` are both Dandan-only (`format.rs`), so every `OwnerKept` format fails the origin gate anyway. The axis match is kept because the charter requires the rebind to be axis-gated and Phase 2's census expects a consumer; its behavior row is the Standard/Dandan pair, which cannot flip until a format shares a zone without `ReceiverOwns`.
- M12 (dig put-all supply) moot: that supply was dropped (premise 2).
No test is shape-only. Every "stays owner" assertion has a rebinding sibling in the same fixture shape.

## 6. New-field threading sweep (`ResolvedZoneChangeCommand.rebound_from`)
- construct: `zones.rs::resolve_and_apply_zone_change_to_receiver` threads the field; `game/log.rs` test literal and `unmaterialized_lki_serialization.rs`: defaults intentionally because the fixtures journal an unrebound move.
- consume: `apply_resolved_zone_change` (precondition + install), `zone_change_command_is_invalid`. No other consumer: `git grep` over `crates client` shows only protocol docs and the above; client wire strips the journal.
- New parameter threading: `hand_receiver` on `move_to_zone_with_entry_flags` (wrapper and two in-file tests pass `None`); `ZoneMoveRequest::draw` drawer (single caller `draw.rs`).

## 7. Maintainer-simulation matrix
| Row | Entry, first branch reached | Authority | Bound value / when | Mode | Storage | Consumer | Invalidation | Hostile fixture | Serde |
|---|---|---|---|---|---|---|---|---|---|
| Hand taker | producer builds request (draw: `pending.player`; `draw_n`: seat; Dig kept map: DigChoice `player`; explore: exploring permanent's controller; seek: `ability.controller`; ChangeZone: `ability.controller`) | the player whose hand receives | `PlayerId`, request construction | snapshot | `EntryMods.performed_by` -> `ProposedEvent::ZoneChange.performed_by` / `PendingBatchZoneMoveRequest.performed_by` | `hand_entry_receiver` in `deliver_replaced_zone_change` | final `to != Hand` (Moved redirect), taker == owner, per-seat origin, `OwnerKept` format, Attraction redirect | V9 pause/redirect, Attraction, per-seat origin; V5 source sacrificed before resolution; V6 owner != controller | no new serialized shape on the request; the existing field already serializes |
| Rebind install | `resolve_and_apply_zone_change_to_receiver` (receiver Some, from Library/Graveyard) | receiver | `rebound_from` = prior owner, at move | snapshot, journaled | `ResolvedZoneChangeCommand.rebound_from` | `apply_resolved_zone_change` (live and replay run one body) | validator refuses same owner, non-Hand, non-Library/Graveyard origin | V10 incl. `OwnerMismatch` and tampered journals | new field, omitted when None, defaulted when absent; bump 109 / wire 91 |
Incomplete rows: none. Redirect residual (plan 3.4 (6)): a Hand-requested move redirected to Exile by a `Moved` replacement records `exiled_by = Some(taker)` where base recorded none; pinned by `the_taker_survives_a_serialized_pause...`. Note this also applies to a draw in non-Dandan formats now that `draw` always names the drawer (CR 406.6 + CR 608.2c performer); no corpus card in the 23 list reaches it.

## 8. CR gate
Diff + untracked scan: every cited number found in `docs/MagicCompRules.txt` (108.3, 108.4a, 109.4, 121.1, 400.3, 406.6, 608.2c, 614.5, 717.6); `UNVERIFIED` count 0. Citations used: 108.3 + 108.4a + 109.4 on the install (owner / controller off battlefield and stack), 400.3 on the seam doc, 121.1 + 608.2c on `hand_taker`/`draw`, 717.6 in the move doc. 108.3 is cited as "modified by the format's hand-entry axis" because the rule itself says the owner is the player who started the game with the card.

## 9. Judgement calls
Wrapper + new function instead of a signature change (premise 1); dropped dig put-all and the dead `filter` (premises 2, 3); Hollowhenge Wrangler for Seek (premise 4); V9/V10 inline in `zone_pipeline.rs`/`zones.rs` because `move_object` and `resolve_and_apply_zone_change_to_receiver` are `pub(crate)`.

## 10. Stop-and-return items
None blocking. Open for the driver: (a) collapse the wrapper into the single signature if a one-line edit to the out-of-scope `dandan_shared_pile_storage.rs` is allowed; (b) the pre-existing Heirloom-class filter owner read above.

## 11. Risks
`ZoneMoveRequest::draw` now names the drawer for every format (see redirect residual). Hand-bound producers not supplied (search partitions, discover, reveal-until, bounce "owner's hand", Dig put-all) keep owner-hand behavior by design; no Dandan list card reaches them. M3' is an equivalent mutant until a format shares a zone without `ReceiverOwns`.
