# Phase 17 executor report r1

Mode: implementation/fix (phase mode). BASE_SHA = START_SHA = 1370f485471706db195f5a33b8cdf6dcbf670216. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Start: clean, HEAD == START_SHA, nothing staged. End: HEAD unchanged, nothing staged, delta = 11 paths, all in scope.nul (`comm` against scope.nul: empty). All checks below are PREPARATORY, not completion evidence.

## 1. Diff summary
- phase-ai production: `zone_eval.rs` (graveyard_value via `graveyard_of`), `policies/payoff.rs` (mill_scale via `library_of`), `policies/mill_targeting.rs` (empty test via `library_of`), `search.rs` (large-board predicate via `graveyard_of`; `deck_color_demand` via `deck_pool_of`), `deck_knowledge.rs` (`pool_holder`/`pool_seats`, three lookups via `deck_pool_of`, owner tests via `pool_holder`, graveyard via `graveyard_of`, co-seat hand subtraction), `session.rs` (`from_game` registers every pool seat; fingerprint hashes seat->pool).
- engine: `game/effects/cast_from_zone.rs::looked_at_controller_library_cards` compares library containers (`zone_storage_seat`) instead of `owner == controller`.
- tests: inline rows in the five phase-ai files above + `policies/tests/mill_payoff.rs` + `tests/scenarios.rs` (V9); inline row in `cast_from_zone.rs`; new `tests/integration/dandan_look_top_of_library.rs` + `main.rs` mod line.
- Unedited by verdict: `threat_profile.rs` (V7 green after `deck_knowledge`), `determinize.rs`, `engine_resolution_choices.rs` (its call site passes only the controller; no owner read).

## 2. Step 0 / premises re-measured at base
- Accessors present, `pub`: `library_of`, `graveyard_of`, `deck_pool_of`, `zone_storage_seat`, `canonical_seat`; `shared_zone_holder` is `pub(crate)` (unused).
- Census (`count_reads_lines.py HEAD '\.(library|graveyard)\b(?!\s*\()'`): phase-ai = 9 production lines = the five owned + four `planner/mod.rs` hash reads (plan's 12 predates Phase 9's swaps). Held.
- Claim 4 held: `best_of_three_ceiling.rs::r1_dandan_bo3_plays_as_bo1_and_never_enters_between_games`. `deck_pools_fingerprint` appears only in `lib.rs` (re-export) and `session.rs`. `ai_duel` has no format flag.
- P-provision held: `load_deck_into_state` on a Dandan state gives one pool at P0, `players[1].library` empty, `library_of(P1) == library_of(P0)`, no card database.
- Replaced premise: V2 uses real Predict (present in `integration_cards.json.gz`) added via `add_real_card`, not a hand shape.

## 3. PREPARATORY verification
- `cargo fmt --all -- --check`: rc 0 (after `cargo fmt --all`; delta stayed inside scope).
- `cargo nextest run -p phase-ai -E 'binary(scenarios) or test(/zone_eval|mill_payoff|mill_targeting|deck_knowledge|^session::|search::|determinize|census/)'`: 310 passed (e17-green1.log; re-run after fmt 309 passed + 1 self-SIGTERM by my timeout, that test re-run alone: pass).
- `cargo nextest run -p phase-engine --features test-support -E 'dandan_look_top_of_library | looked_at_cards_follow | kiora_ | svella | cast_from_zone:: | /census/ | dandan_filter_owner_axis | dandan_shared_pile_storage'`: 212 passed (e17-p2-green.log).
- `node scripts/check-protocol-version.mjs`: rc 0 (silent by design; no `types/` path in the diff).
- Not run per brief: clippy, full suites, `cargo ai-gate` (run-level), fixture regen (no new card needed; every real card used is already in the fixture).

## 4. Parser gate
N/A: no `crates/engine/src/parser/` path.

## 5. Discriminating-test gate / production-path coverage map
Red-at-base logs: e17-red1.log, e17-red2.log, e17-p2-red.log (same tests, production edits absent); mutation logs e17-mutV6.log, e17-p2-mut.log.

| Claim / seam | Entry | Test | Assertion that fails on revert | Sibling / negative |
|---|---|---|---|---|
| V1 `zone_eval::graveyard_value` | `zone_bonus` | `shared_graveyard_cancels_for_both_seats`, `shared_graveyard_leaves_hand_difference_intact` | base: 0.25 vs 0.0 for P0 | `per_seat_graveyard_counts_for_its_owner_only` (Standard, green at base and after); hand-difference row; reach: `graveyard_of(P1).len()==3`, raw P1 empty |
| V2 `payoff::mill_scale` | `PayoffPolicy(MILL_PAYOFF).verdict` with real Predict | `shared_pile_size_is_seen_from_both_seats`, `small_shared_pile_is_urgent_from_both_seats` | base: `library_remaining` 0 vs pile size | both seats as AI; small pile -> HIGH; existing Standard x1/x2/x3 rows green |
| V3 `mill_targeting::verdict` | `MillTargetingPolicy.verdict` | `shared_pile_is_not_an_empty_library_for_the_non_holder_seat` | base: -0.7 vs 0.3 | `empty_shared_pile_keeps_the_empty_library_penalty` (green at base and after; reach) |
| V4 `search::large_board_..._sources` | predicate | `large_board_shared_graveyard_is_a_development_source_for_either_seat` | base: predicate true with shared graveyard card | same state without the card returns true (control inside the test); `large_board_own_graveyard_..._in_standard` |
| V5 `deck_knowledge` pool + owner test + graveyard | `known_remaining_deck_counts`, `unknown_hidden_pool` | `shared_pool_accounts_for_every_seats_cards_and_the_shared_graveyard`, `shared_graveyard_card_also_pinned_known_subtracts_once` | base: empty vs `[(Beta,1)]` / `[]` vs `[Alpha]` | `separate_pools_subtract_only_their_own_seats_cards` (green at base) |
| V6 co-seat hand | `unknown_hidden_pool`, `determinize_opponents` | `co_seat_hand_leaves_the_unknown_pool` (base red), `determinized_opponent_slots_never_receive_the_observers_hand_names` (vacuous at base: empty pool; discriminates only M-V6) | M-V6 (drop `.chain(co_seat_hands)`): both fail (e17-mutV6.log) | |
| V7 threat profile | `build_threat_profile_multiplayer` | `threat_profile_exists_for_the_non_holder_seat` | base: `None` for ai=P0 | ai=P1 `Some`; Standard single-pool `None` |
| V8 session + fingerprint + demand | `AiSession::from_game`, `deck_pools_fingerprint`, `deck_color_demand` | `shared_pile_session_analyses_the_pile_for_both_seats`, `fingerprint_covers_which_pool_backs_each_seat`, `deck_color_demand_reads_the_shared_pool_for_both_seats` | base: P1 keys absent; equal fingerprints; demand `[0;5]` vs `[0,0,0,4,0]` | `separate_libraries_keep_each_seats_own_analysis`, `deck_color_demand_keeps_each_seats_own_pool_in_standard` (green at base) |
| V9 liveness | `run_ai_actions_bounded` | `dandan_shared_pile_ai_pair_both_act_and_draw` | base: reach assertion `session.features` has P1 | per-seat actors and per-seat `CardDrawn` counts asserted; pile shrink equals total draws; Standard `two_ai_long_stream_runner` tests green |
| Open item `looked_at_controller_library_cards` | Svella (real card) activation -> `EffectZoneChoice` -> `SelectCards` | `dandan_every_looked_at_spell_is_offered_whoever_owns_it`, `dandan_casting_an_opponent_owned_hit_bottoms_every_other_looked_at_card`, `dandan_declining_bottoms_all_four_looked_at_cards`, inline `looked_at_cards_follow_the_library_the_controller_reads` | M-a (base predicate): 4 of 5 fail; base run: offered `[5]` vs 3 spells, decline bottoms only `[5,9,10,11]` of 4 | Standard twin `standard_offers_the_controllers_own_spells_and_leaves_the_opponent_library_alone` (passes at base and after, as intended); M-b (owner clause removed entirely): only the inline Standard leg fails, so the owner discrimination is pinned by the inline row, not the Standard integration twin |

No shape-only test. No degenerate fixture: Dandan fixtures have P1-owned cards in the pile with the raw P1 container empty (asserted as reach), Svella's window mixes owners.

## 6. Maintainer-simulation matrix
| Seam | Entry / first branch | Authority | Bound value, when | Mode | Storage | Consumers | Invalidation | Hostile rows |
|---|---|---|---|---|---|---|---|---|
| Pool resolution | `deck_pool_of(seat)` -> `zone_storage_seat(Library, seat)` match arm | holder `PlayerId` | per call | live | none (derived) | `known_remaining_deck_counts`, `remaining_deck_view`, `unknown_hidden_pool`, `deck_color_demand`, `from_game` | n/a (seat set never shrinks) | V5, V6, V8 (P1-owned cards, one pool) |
| Owner test | `pool_holder(owner) == Some(pool.player)` | pool holder | per object per call | live | none | the three deck_knowledge loops | n/a | V5 Standard two-pool identity |
| Session registration | `from_game` over `state.deck_pools` | pool seats | build | latched per session; cache key now includes seat->pool | `AiSession` maps | policies via `session.features` etc. | fingerprint change | V8 fingerprint row |
| Looked-at window | `last_revealed_ids` filtered by `Zone::Library` + container | controller's library container | resolution, per call | live predicate | `last_revealed_ids` | cast-selection pool, decline bottoming, miss bottoming | window cleared by effect | Svella Dandan (3 rows), Standard twin, inline window |
Serde / protocol / fixture impact: none (no type change; no card needed beyond the fixture).

## 7. CR gate
Zero `UNVERIFIED` (7 numbers: 104.3c, 400.1, 400.2, 400.3, 401.2, 401.3, 404.1; grep of `docs/MagicCompRules.txt`; each cited for its subject: 400.1 each player's own zones as a format modifies them, 401.2 library hidden, 401.3 library count public, 404.1 graveyard, 400.2/400.3 zone publicity and owner's corresponding zone, 104.3c existing).

## 8. Judgement calls
- Chose `zone_storage_seat(Library, owner) == zone_storage_seat(Library, controller)` for the open item (the idiom `engine_resolution_choices.rs` already uses for library holders) over `library_of(controller).contains`, which is O(n) and ignores the zone field.
- `mill_targeting` keeps the unknown-seat guard as `players.get(idx).is_some() && library_of(..)` so an unknown seat cannot reach `library_of`'s `expect`.
- `from_game` registers `once(pool.player).chain(pool_seats(..))` so a pool whose seat is not its own holder still keeps its own key.
- Added the inline `cast_from_zone.rs` row because the integration Standard twin cannot make the owner clause bite (M-b).
- Card reading (Svella, jq verbatim): "Look at the top four cards of your library. You may cast a spell from among them without paying its mana cost. Put the rest on the bottom of your library in a random order." Under the Dandan announcement "your library" is the one pile, so all looked-at spells are offered whoever owns them and every unchosen looked-at card (any owner) goes to the bottom. No Dandan-list card reaches this path; Svella/Kiora/Aetherworks Marvel/Perception Bobblehead class does.

## 9. Stop-and-return items
Same-class `owner == controller` library reads outside scope.nul, not edited (class sweep of `Zone::Library` + owner reads in `crates/engine/src`):
- `game/casting.rs` (`top_of_library_permission_src`: `obj.zone == Zone::Library && obj.owner == player`) and `game/casting_costs.rs` (`top_of_library_alt_ability_cost_for_object` branch, same predicate): "cast from the top of your library" statics (Future Sight, Realmwalker, Bolas's Citadel) skip an opponent-owned top card of the Dandan pile. Same fix shape (container comparison); needs scope.
- `analysis/resource.rs` C1b `prior_obj.owner == caster`: Phase 9's recorded residual, not a gap of this phase.
Within `cast_from_zone.rs`: single occurrence (the fixed one); other owner mentions are tests or exile.

## 10. CR annotations
Added/changed: doc of `looked_at_controller_library_cards` (CR 400.1), `graveyard_value` (CR 404.1 + CR 400.1), `mill_scale` (CR 104.3c + CR 401.3), `mill_targeting` (CR 401.3), `search.rs` predicate (CR 404.1), `deck_knowledge` (CR 400.2 + CR 400.3, CR 401.2 + CR 400.2). Verified by `grep -an "^<n>" docs/MagicCompRules.txt` (gate above).

## 11. Deviations
V2 real-card shape (premise replaced, see 2). Plan step 5/6 (clippy, full crate, ai-gate) not run per brief. No scope additions.

## 12. Risks
- ai-gate owed at run level (non-shared formats only expected identity; the edits are identity there by construction: `library_of`/`graveyard_of`/`deck_pool_of`/`pool_seats` reduce to the seat itself).
- `unknown_hidden_pool`'s co-seat subtraction assumes a shared pile has two seats (stated in the code comment).
- Top-of-library cast reads (section 9) remain wrong in Dandan until scoped.
