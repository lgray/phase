# Charter review r6 (charter-mode, whole charter, revision r5)

Reviewer model: claude-sonnet-5-5. Read-only; no cargo. Artifact: `phase-charter` (r5) vs `phase-charter.r4c`.

## Verdict: CLEAN except 1 review-only finding (premise, seam-note sentence). 0 decision revisions, 0 corrections.

## Checks
- **Accepted phases unmoved.** The diff touches only: header (r4 to r5), phase-list index line 4b, the Phase 4b entry (ignored per brief), Phase 6 seam note, Phase 8 seam note, Phase 11 Goal / Scope rule / Deferral list / Seam notes / T1-T2. Phases 1-4 are byte-identical. Phase 6 and 8 are remaining phases and their edits are seam-note sentences only (no goal, acceptance row, seam order, or deferral moved).
- **Phase 11 addition narrow and justified.** Telling Time Oracle text in `client/public/card-data.json` matches the charter quote verbatim; parse is `Dig{count 3, destination Hand, keep_count 1, up_to false, rest_destination Library, rest_split_top_count 1}`. `effects/dig.rs` has exactly one `ZoneMoveRequest` (line 552, put-all, reached only for the `u32::MAX` keep sentinel; Telling Time takes the `DigChoice` path, dig.rs 280). The kept delivery is `engine_resolution_choices.rs` ~4555-4600 (`ZoneMoveRequest::effect(obj_id, kept_zone, ..)` in the `kept_destination` branch); `grep performed_by` in that file is empty. It lies outside the producer grep's directories, so the exception is needed for the Goal's own Telling Time row. It is limited to one map in one branch, an acting `player` binding exists there. Class coverage: it is the Dig-to-Hand delivery for all dig-to-hand cards.
- **Stated limit vs the 23 names (`names.txt`).** Walked all 23 cards' ability/trigger/replacement effect types: none searches, discovers, or reveals-until; hand-bound effects are Draw (incl. cycling, Surgical Bay, Day's Undoing), Dig (Telling Time), ChangeZone Graveyard to Hand (Fengraf), Bounce/ChangeZone of battlefield or stack objects (non-shared origin). Limit holds. It is correctly labelled "dropped, not deferred" and the seam reads the carried receiver. The explore/seek/reveal_until/scoped_library_search producers stay admitted by the grep clause, not part of the limit, consistent with plan-review-r1.
- **Seam notes 6/8/11.** Phase 8 scope excludes `engine_resolution_choices.rs` (Phase 6 "fully covered"), so the Phase 11 note "read-only elsewhere" holds; Phase 8's dig.rs note matches Phase 11's conditional dig.rs entry; Phase 6 and Phase 11 notes name each other.
- **T1 and T2.** One unit; the added file is a producer of the same carrier; count stays 8-13, T1 fails. Conjunction cannot fire. 4b unaffected.
- **Code-state assertions in the added text.** "`ZoneMoveRequest` has no recipient field", "no producer supplies one" are pre-existing claims with measurements; the new sentences about Telling Time's delivery are measured above and no phase boundary rests on them.

## Finding F1 (review-only revision, class premise)
Phase 6 seam note (line 184) says the Phase 11 edit is "a different function than this phase's raw writers", and Phase 11 seam note (line 287) says "a different function". Measured: Phase 6's raw writers at `engine_resolution_choices.rs` 4456 and 4478 (`player_state.library.retain/push_back`, `allow-raw-zone`) sit in the `kept_destination == Some(Zone::Library)` branch of the same `(WaitingFor::DigChoice, GameAction::SelectCards)` arm, inside the same function as the kept map at ~4589. No decision rests on it (phases are sequential, ownership by line), so it is a premise correction with replacement:
- Phase 6 note: replace "(the `DigChoice` `SelectCards` kept-card map only, a different function than this phase's raw writers)" with "(the `DigChoice` `SelectCards` kept-card map only, a different branch of the same arm than this phase's `kept_destination == Some(Zone::Library)` raw writers, sequential)".
- Phase 11 note: replace "a different function, sequential after Phase 6" with "a different branch of the same `SelectCards` arm than Phase 6's raw writers, sequential after Phase 6".
Measurement: `sed -n 4440,4600p crates/engine/src/game/engine_resolution_choices.rs`.

No other findings.
