# Phase 6 plan: canonical-seat storage authority and pool resolver (S2a)

Mode: engine-planner, phase-plan mode, phase k=6, revision 1. Planned against HEAD `67cd221e` (Phase 4 candidate). PHASE_BASE will be Phase 5's accepted candidate; Phases 4b and 5 touch none of this phase's files except the shared registration file named in the seams. No cargo was run (another agent owns the build, per the spawn instructions), so every dynamic assertion is labelled UNESTABLISHED and is handed to the executor as a named probe. Static claims carry the command that measured them. The claims ledger is at the end.

## 0. Premise verification (Step 0)

Oracle text fetched verbatim from Scryfall (`cards/named?exact=`) and identical to `client/public/card-data.json` (jq) for each card a test below uses:

- Dandân {U}{U}, Creature, Fish 4/1: "This creature can't attack unless defending player controls an Island.\nWhen you control no Islands, sacrifice this creature."
- Island: "({T}: Add {U}.)"
- Lonely Sandbar: "This land enters tapped.\n{T}: Add {U}.\nCycling {U} ({U}, Discard this card: Draw a card.)"
- Halimar Depths: "This land enters tapped.\nWhen this land enters, look at the top three cards of your library, then put them back in any order.\n{T}: Add {U}."
- Calim, Djinn Emperor {3}{U}{U}{U}: "Flying, ward {2}\nCalim's Breath — {1}{U}, Discard Calim: Tap up to one target nonland permanent. Draw a card. Then you may exile two other cards named Calim, Djinn Emperor from your graveyard. When you do, return Calim from your graveyard to the battlefield tapped.\nWhen you discard Calim, conjure a card named Calim, Djinn Emperor into your library seventh from the top." (coverage-data: `supported: true`, measured with jq over `client/public/coverage-data.json`.)

The 80-card list: the brief's table sums to 10 + 20 + 8 + 4 + 19 x 2 = 80 over 23 distinct names. All 23 names resolve in `client/public/card-data.json` (jq over the keys, lowercased; `dandân` is keyed with the circumflex). Memory Lapse is the brief's Oracle text and its coverage flag was handled by Phase 4.

Clauses the settled decisions bind here: one shared library and one shared graveyard, canonical seat = lowest `PlayerId`, fixed at setup; CR 103.5 shuffles act on the shared pile. Nothing in this phase decides which hand holds a card (Phase 11), the interleaved deal (Phase 12), FreeReveal (Phase 13) or any read sweep (Phases 8, 9, 17).

## 1. Skills and checklists applied

`engine-planner` phase-plan mode; `add-engine-variant` gate: no enum variant is added (measured: `ZoneScope`, `SharedZones`, `DealOrder`, `HandEntryOwnership` exist from Phase 2 and are consumed unchanged; `data/engine-inventory.json` predates Phase 2, it contains `ZoneScope` but not `SharedZones`, and was not regenerated because that needs cargo; `grep -c` over it for `canonical_seat|library_of|deck_pool_of|storage_seat` is 0, no sibling exists). `card-test` for every runtime row (GameScenario plus GameRunner, real faces via `add_real_card`, `support::shared_card_db()`). `add-interactive-effect`, `add-frontend-component`, `add-keyword`, `add-trigger`, `add-static-ability`, `add-replacement-effect`, `oracle-parser`: not applicable (no effect, WaitingFor, parser, variant or UI). `add-card-data-pipeline`: not applicable (no export shape change). CLAUDE.md checklist items for a serialized-shape change (protocol bump, interaction bindings) are answered by measurement in Verification rows V9 and V10: no serialized shape changes.

## 2. Pattern Coverage

Class: every format whose `GameFormat::shared_zones()` declares a shared library or graveyard (today only Dandan; Custom answers `SharedZones::NONE`). The storage authority, the accessors and the pool resolver are the building blocks Phases 8 to 17 consume; deck provisioning serves the 80-card list (23 distinct cards). Assessed against the charter's class attribution (infrastructure phase: zero cards by itself).

## 3. Design

### 3.1 The canonical seat and the storage resolver (types/game_state.rs)

One `impl GameState` block beside the library-knowledge functions (they are in the same file):

- `canonical_seat(&self) -> PlayerId`: the lowest `PlayerId` in `self.players`. It is a pure function of the seat set, which never shrinks (elimination sets flags; measured: `git grep -n 'players\.\(remove\|retain\|truncate\|pop\|drain\)' -- crates/engine/src` has no production hit, and every `players.push` hit is in a test module). So it is fixed for the life of the game without a stored field: the brief says no new `GameState` field, a field would be a serialized shape, and `seat_order` cannot serve (it is rotated by `start_game_with_starting_player`: measured by reading `seat_order.rotate_left`). Not recomputed from liveness, turn or seat order.
- `zone_storage_seat(&self, zone: Zone, seat: PlayerId) -> PlayerId`: exhaustive `match zone` with no wildcard. `Library` reads `format.shared_zones().library`, `Graveyard` reads `.graveyard`, every other zone is `ZoneScope::PerPlayer`; `Shared` returns `canonical_seat()`, `PerPlayer` returns `seat`. It reads `self.format_config.format`, never a payload field. Annotation: "CR 400.1 as modified by the format's shared-zone axis (Dandân announcement); CR 400.3 owner's corresponding zone".
- `library_of`, `graveyard_of` (`&im::Vector<ObjectId>`), `library_of_mut`, `graveyard_of_mut`: `pub`; each resolves through `zone_storage_seat`, then finds the `Player` with `expect("seat exists")` (the same message class the existing sites use). An unknown seat in a non-shared format panics exactly as the `.find(..).expect(..)` sites do today.
- `deck_pool_of(&self, seat) -> Option<&PlayerDeckPool>`: the pool resolver. It resolves `zone_storage_seat(Zone::Library, seat)` and finds the pool with that `player`. Identity when libraries are not shared. Read-only: no `_mut` form is added (no production mutator needs it in this phase). Its production readers are Phase 17's; this phase classifies the engine's readers in 3.6.
- `seats_with_empty_library(&self) -> Vec<PlayerId>`: seats whose `library_of(seat)` is empty. The engine owns the predicate so the wasm boot guard is a thin call and the guard is testable from the integration crate.

### 3.2 Write-side canonicalization inside the journaled appliers (game/zones.rs)

All through the accessors so replay (`apply_resolved_zone_change` calls `zone_container_len`, `remove_from_zone`, `add_to_zone`) resolves the same way on both sides:

- `zone_container_len`: `Library`/`Graveyard` arms read `library_of(owner).len()` / `graveyard_of(owner).len()` (this also validates the replayed `destination_position`); `Hand` arm unchanged.
- `remove_from_zone`, `add_to_zone`: separate `Library`, `Graveyard` and `Hand` arms using `library_of_mut` / `graveyard_of_mut`; `Hand` keeps the owner-keyed player lookup.
- `reorder_within_library` and the insertion in `move_to_library_at_index`: through `library_of_mut`. `advance_library_knowledge_epoch(player)` already receives the seat; it resolves inside (3.4).
- `GameObject.owner` is never rewritten here: a P1-owned card can sit in the canonical seat's container, which is the state Phase 11 produces. `apply_resolved_zone_change`'s `OwnerMismatch` check still compares owner to `command.owner` and needs no change.

### 3.3 Raw writers outside the authority files

- `game/library.rs`: `resolve_and_apply_library_shuffle` keeps its `UnknownPlayer` refusal (an explicit `players.iter().any` check on the acting player), reads `precondition_order` through `library_of(player)`; `apply_resolved_library_shuffle` compares `current_order` through `library_of(command.player)` and installs `resulting_order` through `library_of_mut(command.player)`. `ResolvedLibraryShuffleCommand.player` stays the acting player (it drives `PlayerActionKind::ShuffledLibrary` and "whenever you shuffle"). Annotated `// allow-raw-zone:` (permutation install, CR 701.24a, not a zone event).
- `game/engine_resolution_choices.rs`: the two raw writers (`ScryChoice` and the `DigChoice` library-reorder arm) keep their existing annotated container statements; only the player lookup changes to `let holder = state.zone_storage_seat(Zone::Library, player_or_library_owner)` then `find(|c| c.id == holder)`. The Dig arm uses `state.rng` while `player_state` is live, which a `&mut self` accessor would forbid, so the holder-seat form is the borrow-correct one and leaves the census rows unchanged. `advance_library_knowledge_epoch(..)` calls there are unchanged (they resolve inside).
- `game/effects/conjure.rs::place_conjured_in_library`: `pidx` is found by `zone_storage_seat(Zone::Library, owner)`; the read of the existing library and the whole-container assignment both use that index. Without this the assignment writes the owner's own (empty) container and leaves the conjured objects in two containers.
- `game/mulligan.rs::shuffle_hand_into_library`: the hand read stays owner-keyed; the library shuffle becomes `let mut library = std::mem::take(state.library_of_mut(player)); shuffle_vector(&mut library, &mut state.rng); *state.library_of_mut(player) = library;` annotated `// allow-raw-zone:` (membership-preserving pregame shuffle). It deliberately does not route through `resolve_and_apply_library_shuffle`: the existing test `mulligan_shuffle_back_emits_no_shuffled_library_events` pins zero `ShuffledLibrary` events and the un-journaled RNG consumption (measured by reading the function's doc comment and the test names in mulligan.rs). `draw_n` reads the top through `library_of(player_id).front()`. The all-seat `start_mulligan` shuffle loop and the final shuffle loop in `deck_loading.rs` stay raw: under canonical storage they shuffle the one pile once and shuffle the empty seat as a no-op (`util/im_ext.rs::shuffle_vector` was read; that `SliceRandom::shuffle` on an empty slice consumes no RNG is UNESTABLE here without cargo and is executor probe P-mull, a shuffle of the empty seat must leave `state.rng.get_word_pos()` unchanged).
- `game/effects/draw.rs::select_cards_to_draw`: keep the unknown-player guard (`players.iter().any`), then read `library_of(player_id)` for both the `DrawFromBottom` and top branches. `can_draw_at_least_one` and `apply_draw_after_replacement` both call it, so the empty-library flag (`attempted_empty_library`, written only by the ledger edit) follows.

### 3.4 Library-knowledge keys resolve inside `game_state.rs` (no caller edits)

- `library_knowledge_epoch(owner)` and `library_knowledge_boundary_generation(owner)` index by `library_storage_seat(owner)` (the `Zone::Library` storage seat).
- `advance_library_knowledge_epoch(owner)` resolves `owner` first. The `facts.retain` drops every library-zone fact whose own storage seat equals the resolved seat (`fact.owner` is the card's owner, so the resolved comparison is what removes a P1-owned card's fact when P0's pile is reordered). The closure must not borrow `self` while `facts` is borrowed mutably: hoist a `Copy` resolver (the `ZoneScope` and canonical seat read once into locals).
- `canonicalize_library_knowledge_epoch(owner)`: its `fact.owner == owner` becomes the same resolved comparison.
- `record_zone_change_library_knowledge_stamp`: `LibraryKnowledgeStamp.library_owner` is the resolved seat (the generation it reads is already resolved).
- `remember_card_identities` and `viewer_knows_card_identity` call `library_knowledge_epoch(card.owner)`, so they are covered by the first bullet; `fact.owner == card.owner` in `viewer_knows_card_identity` compares one card with itself and stays.
- `game/visibility.rs` is not edited: measured with `git grep -n 'library_knowledge_epoch\|advance_library_knowledge_epoch\|library_knowledge_boundary_generation\|library_owner' -- crates` and reading every hit outside `game_state.rs`: visibility.rs only clears the tables and compares stamps read from `game_state.rs`; every `advance_library_knowledge_epoch` caller (`zones.rs` x2, `library.rs`, `engine_resolution_choices.rs` x2) passes a seat, none derives the key from a card's owner. The charter's authorization to edit visibility.rs is not used.

### 3.5 Deck provisioning (game/deck_loading.rs)

- `DANDAN_DECKLIST: [(&str, usize); 23]` (printed names, including `Dandân`) and `pub fn dandan_fixed_deck_names() -> Vec<String>` beside `momir_fixed_deck_names`, expanding to 80 names.
- Extract the per-seat fixed payload builder now duplicated inside `momir_fixed_deck_payload`'s closure into `fn fixed_seat_payload(db, names) -> PlayerDeckPayload`; Momir calls it with its names (no behavior change), `dandan_fixed_deck_payload(db, submitted, pile_seat)` calls it for `pile_seat` only and uses `PlayerDeckPayload::default()` for every other seat (P0, P1, and one per `submitted.ai_decks`).
- `load_and_hydrate_decks`: a Dandan branch beside the Momir branch (`format == GameFormat::Dandan`, `Some(db)`: `pile_seat = state.canonical_seat()`; `None`: fall through to the submitted payload as Momir does).
- `load_deck_into_state`: a seat that does not hold the library pile (`state.zone_storage_seat(Zone::Library, seat) != seat`) gets no `PlayerDeckPool` and no `load_player_library` call, so the non-canonical seat's payload is ignored entirely (pool and library) and `state.deck_pools` holds one pool. Computed into booleans before the mutable `deck_pools.push` blocks. For every non-shared format the predicate is identity-true, so nothing changes. The attraction, contraption, sticker, commander and signature paths are not gated (Dandan's payload for them is empty, `place_commanders` is false for Dandan).
- `resolve_names` skips an unresolvable name silently (measured by reading it), so a database missing a list card would yield fewer than 80; V2 pins resolution of all 23 names against the real database.
- Between-games readers (`match_flow.rs::deck_payload_from_current_pools` requires pools for P0 and P1; `bo3_sideboard_players`; `interaction.rs` sideboard validation; `candidates.rs::sideboard_actions`) are reachable only after a Bo3 game 1 ends. Phase 7 caps Dandan at Bo1 and owns the red row for them; they are not edited here.

### 3.6 Seat-keyed `deck_pools` readers in `crates/engine/src` (charter claim 4, classified)

Measured by `git grep -n 'deck_pools' -- crates/engine/src` and reading every production hit:

| Site | Shape | Dandan reach |
|---|---|---|
| `game/boosters.rs`, `game/card_subset.rs` | loop over every pool | fine with one pool |
| `game/deck_loading.rs` (clear, pushes) | writer | this phase |
| `game/commander.rs` (`commander_color_identity`, `commander_creature_types`) | `find(pool.player == seat)`, reads `current_commander` | reached only by commander-referring cards; Dandan's pool has no commander, so absent pool and resolved pool both answer "none" |
| `game/companion.rs::check_companion_reveal` | `find(..)?` | reached at game start for both seats; `companion_offers` returns empty for `SideboardPolicy::Forbidden` with `uses_commander == false` (Dandan), so `None` either way. `handle_declare_companion` is unreachable without an offer |
| `game/effects/search_outside_game.rs` (two) | `find(..)` on `current_sideboard` | sideboard is empty either way (`Forbidden`) |
| `game/mulligan.rs::tiny_leaders_forced_mulligan_pending` | gated on `GameFormat::TinyLeaders` | unreachable |
| `match_flow.rs`, `interaction.rs` (sideboard submission), `candidates.rs::sideboard_actions` | between-games | Phase 7 |

So no engine reader Dandan reaches answers differently under the resolver; none is converted (a conversion would change no answer). `deck_pool_of` is the single authority for the readers that do differ, Phase 17's `phase-ai` `deck_knowledge.rs`.

### 3.7 Boot guard (engine-wasm/src/lib.rs)

The inline `filter(|p| p.library.is_empty())` becomes `state.seats_with_empty_library()` mapped to `.0`. That is the whole wasm edit. `engine-wasm` is the only transport that loads decks and starts a game (measured: `git grep -n 'load_and_hydrate_decks\|start_game(' -- crates/server-core/src crates/phase-server/src crates/engine-wasm/src ':!*test*'`).

### 3.8 Census gate (scripts/zone_authority_census.py), the one path outside the charter

The census classifies `.library.push_back(..)`-style container calls; `library_of_mut(p).push_back(..)` is invisible to it, so the new accessor would be an unguarded bypass of Gate B. Add a fourth family `accessor`: `re.compile(r"(?<!fn )\b(?:library|graveyard)_of_mut\s*\(")`, entered in `FAMILIES`. `zones.rs` is an authority file and is skipped; the definition lines in `game_state.rs` are excluded by the lookbehind; `library.rs` and `mulligan.rs` carry `// allow-raw-zone:` reasons. Class: verification gate (not engine logic). If the orchestrator declines this path, the plan still holds: the `_mut` accessors stay and the gap is recorded as a verdict, not a TODO.

## 4. Building blocks, logic placement, idioms

- Building blocks reused: `GameFormat::shared_zones()`/`ZoneScope`/`SharedZones` (Phase 2), `zones::{add_to_zone, remove_from_zone, zone_container_len}`, `library::resolve_and_apply_library_shuffle`, `momir_fixed_deck_names`/`resolve_names`/`momir_fixed_deck_payload`, `PlayerDeckPayload::default()`, `support::shared_card_db`, `GameScenario::new_with_format`, `add_real_card`.
- New helpers earn their keep: `zone_storage_seat` is the single resolver every accessor, the epoch keys and deck loading call; `seats_with_empty_library` keeps game logic out of the transport.
- Logic placement: all in `engine`; the wasm edit is a call. Frontend untouched.
- Rust: typed `ZoneScope` (no bool), exhaustive `match zone`, no wildcard, `Option` for the resolver.
- Nom compliance: no file under `crates/engine/src/parser/` changes.
- Extension vs creation: extends the owner-keyed container pattern with one resolution layer; does not create a second store.
- Analogous trace: `momir_fixed_deck_names` -> `momir_fixed_deck_payload` -> `load_and_hydrate_decks` -> `load_deck_into_state` (provisioning); `add_to_zone` -> `apply_resolved_zone_change` -> journal replay (writes); `resolve_and_apply_library_shuffle` -> `apply_resolved_library_shuffle` (shuffle); `advance_library_knowledge_epoch` -> `visibility.rs` stamp consumers (epoch keys). The 2HG `topology.rs::shared_resource_dedup_key` precedent is Phase 10's.

## 5. Identity / provenance contract

Canonical seat: source phrase "shared library and graveyard ... canonical seat, lowest PlayerId, fixed at setup"; authority type `PlayerId`; binding time: derived from the fixed seat set, no stored copy; live (not latched), which is equivalent because the set is immutable; consumer `zone_storage_seat`; no invalidation path. Multi-authority hostile fixture: start the booted game with P1 as starting player (so `seat_order == [P1, P0]`) and show the pile is in `players[0].library`, `canonical_seat() == P0`, and P1's mulligan still shuffles that pile. Owner versus storage: a P1-owned real card moved into the pile keeps `owner == P1` while `players[1].library` stays empty.

## 6. Verification matrix

All tests: new file `crates/engine/tests/integration/dandan_shared_pile_storage.rs`, `mod` line added to `tests/integration/main.rs`; real faces from `support::shared_card_db()` (early return when the export is absent, the suite's convention); one inline seam test in `game_state.rs` (a `pub(crate)` stamp field is unreachable from the integration crate). Assertions are pile-side: which hand holds a dealt or drawn card is `DEFERRED(phase 11)`; no hand-holder is asserted.

| Row | Seam and entry point | Test (revert-failing assertion) | Sibling / hostile / paired positive guard |
|---|---|---|---|
| V1 deck load | `load_and_hydrate_decks` with `DeckPayload::default()` on `GameState::new(FormatConfig::dandan(), 2, seed)`, real DB | `players[0].library.len() == 80`, `players[1].library.is_empty()`, `library_of(P0) == library_of(P1)`, card-name multiset equals `DANDAN_DECKLIST`, `deck_pools.len() == 1` with `player == P0`, `seats_with_empty_library()` empty. Revert the raw guard predicate: it returns `[P1]`. Revert the holder gate in `load_deck_into_state`: two pools | Hostile: `start_game_with_starting_player(P1)` rotates `seat_order`; pile still in P0. Paired positive guard: a Momir state (real snow basics) loads 60 to each seat, two pools, and a Standard state with P1's library emptied reports `[P1]` |
| V2 fixed list | `dandan_fixed_deck_names()` | length 80, 23 distinct, equals `FormatConfig::dandan().deck_size == DeckSizeRule::Exactly(80)`, every name resolves via `db.get_face_by_name` (including `Dandân`) | Guard: Momir's 60-name list cross-check keeps passing |
| V3 shuffle (charter row) | real `apply(state, P1, GameAction::MulliganDecision { choice: MulliganChoice::Mulligan })` after `start_game_with_starting_player` | before = `library_of(P1)` pile; after P1's mulligan the pile has the same multiset, length minus 7 and order `!= before.skip(7)` (the un-shuffled expectation). Revert the shuffle routing: order equals `before.skip(7)` (the empty seat is shuffled) | Paired guard: the same seeded start with the canonical seat mulliganing changes the pile relative to its own un-shuffled expectation (`before` with the returned hand appended, first 7 dropped). Hostile: seat rotated |
| V4 opening deal | same boot | after `start_mulligan`, pile length is 80 minus 14: P1's `draw_n` read the pile. Revert `draw_n`: pile is 73 | Guard: both seats' `CardsDrawn` events present |
| V5 draw (charter row) | P1 (owner of a real Lonely Sandbar in hand, an Island in play, pile of real cards) activates Cycling `{U}`: `select_cards_to_draw` -> `apply_draw_after_replacement` | pile length minus 1 and the old top card left the pile; `!players[1].drew_from_empty_library`. Revert `select_cards_to_draw`: P1 draws nothing and the flag is set | Paired: the same flow with P0 cycling draws one (canonical path). Positive reach: cycling cost actually paid (Sandbar in `graveyard_of(P0)`) |
| V6 graveyard write | the same flow | the cycled, P1-owned Sandbar is in `graveyard_of(P1)`, which equals `graveyard_of(P0)`, and `players[1].graveyard` is empty. Revert the `add_to_zone` graveyard arm: it lands in `players[1].graveyard` | Guard: a Standard-format run of V5 puts it in its owner's own graveyard |
| V7 conjure | P1 activates Calim's Breath (real Calim in hand, Islands in play), no target, then the "When you discard Calim" trigger conjures into "your library seventh from the top" | Derived: the trigger goes on the stack after the ability, resolves first (CR 603.3 order), so the conjured Calim sits at index 6 when the trigger resolves and at index 5 after the ability's draw removes the top. Assert exactly one conjured object, in the pile only, `players[1].library` empty, no `ObjectId` in two containers. Revert `place_conjured_in_library`: id appears in two containers | Guard: same flow in a Standard state places it in P1's own library. The index is UNESTABLISHED until probed; if the stack order differs the assertion is re-derived from the measured order, not relaxed to presence |
| V8 journal replay | real move of a P1-owned Brainstorm (hand to library top through `zone_pipeline::move_object`), and `resolve_and_apply_library_shuffle(state, P1)` | clone the pre-state; `apply_resolved_zone_change` / `apply_resolved_library_shuffle` of the recorded commands reproduce `library_of(P0)` exactly. For the shuffle, the command's `player == P1` and the `ShuffledLibrary` event's `player_id == P1` while the pile order changed. Revert `zone_container_len` to the raw owner read: replay returns `DestinationPositionMismatch` | Guard: a non-shared replay of the same move is unchanged. If the journal is not reachable from the integration crate, the same row runs as an inline test in `zones.rs` |
| V9 library knowledge (charter row) | P0 plays real Halimar Depths; its ETB look at the top three records viewer knowledge (`dig.rs`); with the flow paused at the choice, a library boundary occurs: `library::resolve_and_apply_library_shuffle` by the acting seat | Two legs, each with the pile top three containing a P0-owned and a P1-owned real card (`add_real_card(P1, "Island", Zone::Library)` on top): acting seat P1 and acting seat P0. After the shuffle `viewer_knows_card_identity(P0, id)` is false for every looked-at card. Revert the retain to owner-only: the P1 leg keeps the P0-owned cards' facts, the P0 leg keeps the P1-owned card's fact | Positive reach-guard before the shuffle: `viewer_knows_card_identity(P0, top)` is true (the instrument fires). Paired: a Standard state, shuffle by P1, leaves P0's knowledge of its own library intact. Seam test in `game_state.rs`: `record_zone_change_library_knowledge_stamp` for a P1-owned record yields `library_owner == P0`; revert: `P1` |
| V10 pool resolver (charter row) | after V1's load | `deck_pool_of(P1).map(\|p\| p.player) == Some(P0)` and `deck_pool_of(P0)` is the same pool. Revert to seat identity: `deck_pool_of(P1)` is `None` | Paired: Momir state, each seat resolves to its own pool |
| V11 census | `python3 scripts/zone_authority_census.py --check` | passes; a hostile one-line `re` check shows the new family flags `x.library_of_mut(p).push_back(i)` and not `pub fn library_of_mut(` | `--list` row count: rows for the files this phase edits are unchanged except the two `library.rs`/`mulligan.rs` accessor rows, now exempt with reasons |
| V12 protocol | `node scripts/check-protocol-version.mjs`; `scripts/check-interaction-bindings.sh --check` | exit 0 both, no pin file edited | (run at HEAD before editing: exit 0, measured) |
| V13 regression | `cargo nextest` on the engine crate for `zones`, `library`, `mulligan`, `draw`, `deck_loading`, `visibility`, `game_state` modules and the `integration` binary; engine-wasm native tests | green; non-shared formats are identity | executor runs under `cargo-env.sh` |

Deferred rows (structural, named): hand-holder of dealt or drawn cards `DEFERRED(phase 11)`; FreeReveal shuffle leg `DEFERRED(phase 13)`; card-level Brainstorm pile-side legs `DEFERRED(phase 8)`, its hand leg `DEFERRED(phase 11)`; count dedup of scoped quantities `DEFERRED(phase 10)`; the raw reads (`dig.rs`, `casting.rs`, `replacement.rs`, `engine.rs` and the rest of the census list) `DEFERRED(phase 8)`/`DEFERRED(phase 9)`; `phase-ai` pool readers `DEFERRED(phase 17)`; Bo3 between-games failure `DEFERRED(phase 7)`.

Parser: no Oracle text is accepted with deferred semantics; coverage is unchanged (no parser change).

## 7. Reference readings

- V3 paired guard ("the canonical seat's mulligan also changes the pile") copies the existing canonical-seat behavior. Derived from CR 103.5 ("shuffles their hand back into their library") and CR 103.3: a mulligan shuffles the library, and in this format that library is the shared pile. Reference reading measured by reading `shuffle_hand_into_library` (raw-shuffles the seat's own container, which for the canonical seat is the pile): agrees with the derivation. Not dynamically measured.
- V1/V10 Momir guards ("each seat loads 60 and has its own pool") preserve base behavior: the existing `momir_auto_supplies_fixed_deck_for_every_seat` asserts it; derived from the Momir definition recorded at `MOMIR_SNOW_BASICS` (a format definition, not a CR rule). Not re-derived from an external source.
- Standard-format guards ("each seat draws from and discards to its own zones") preserve CR 400.1 ("Each player has their own library, hand, and graveyard"), grepped in `docs/MagicCompRules.txt`; the preserved code is the identity branch of `zone_storage_seat`. No defective reference found.

## 8. CR annotations (each grepped in `docs/MagicCompRules.txt`)

400.1 (each player's own library and graveyard, modified here), 400.3 (owner's corresponding zone), 401.1 (deck becomes library), 103.3 and 103.5 (pregame shuffle and mulligan), 121.1 (draw takes the top of the library), 701.24a (shuffle), 701.20d (reorder invalidates disclosed library occurrences), 704.5b (empty-library draw). No other number is written.

## 9. Sizing

One unit: the canonical-seat storage authority with its lockstep layers (accessors, write canonicalization, epoch keys, provisioning, boot guard, resolver). Dependency edges: accessors (game_state.rs) -> every consumer below; none of the consumers is an independent skill pass. Discriminating test: V1 through V10.

Scope paths (literal; generated artifacts none; fixtures none):

1. `crates/engine/src/types/game_state.rs`
2. `crates/engine/src/game/zones.rs`
3. `crates/engine/src/game/library.rs`
4. `crates/engine/src/game/engine_resolution_choices.rs`
5. `crates/engine/src/game/effects/conjure.rs`
6. `crates/engine/src/game/deck_loading.rs`
7. `crates/engine/src/game/effects/draw.rs`
8. `crates/engine/src/game/mulligan.rs`
9. `crates/engine-wasm/src/lib.rs`
10. `crates/engine/tests/integration/dandan_shared_pile_storage.rs` (new)
11. `crates/engine/tests/integration/main.rs` (one `mod` line)
12. `scripts/zone_authority_census.py` (outside the charter scope rule; class: verification gate)

Authorized, not edited: `crates/engine/src/game/visibility.rs` (3.4). Count: 11 charter paths edited plus 1 out-of-charter = 12 (< 13). T1 fails (1 unit), T2 fails. Estimated diff: about 950 to 1,000 LOC (game_state.rs ~150 with its seam test, deck_loading.rs ~170 with its list, test file ~450, rest ~200), in line with the charter's ~960.

## 10. Seams

- `crates/engine/tests/integration/main.rs` already carries an uncommitted `mod` line from Phase 4b (`loop_only_dependency_fallback`); add exactly one line and re-read before editing.
- `crates/engine/src/types/game_state.rs`: edits are one new `impl` block, the epoch functions, the stamp recorder and one test; re-read at edit time (Phase 5 may touch serialized shapes in this file).
- `crates/engine-wasm/src/lib.rs`: only the boot-guard lines here; Phase 7 edits the ceiling export in another function.
- `mulligan.rs`: Phases 11, 12, 13 build on the routed shuffle and `draw_n`. `draw.rs`: Phases 11 and 14 build on `select_cards_to_draw`. `visibility.rs`: read again by Phase 8 (verify-only there).
- Phase 5's protocol pins (93 to 94, wire 75 to 76) are not touched; this phase adds no serialized shape (no new field, variant or type).

## 11. Claims ledger

MEASURED (command, static): no shared-pile concept before this phase (grep for `canonical_seat|shared_pile|fn library_of` in zones.rs and game_state.rs); write population (`zone_authority_census.py --list` 48 hits/29 rows, gate passes at HEAD; container-method regex 18 hits in five files, whole-container assignments in `conjure.rs` and `library.rs` only, the `&mut` shuffles in `deck_loading.rs` and `mulligan.rs`); `shuffle_hand_into_library` shuffles the seat's own container (read); `seat_order` is rotated at start (read); no production shrink of `players` (grep); the `deck_pools` reader table (grep plus reads); epoch functions' callers pass seats and visibility.rs only clears or compares stamps (grep); `engine-wasm` is the only deck-loading transport (grep); Dandan's `SideboardPolicy::Forbidden`; `check-protocol-version.mjs` exit 0 at HEAD; `zone_authority_census.py --check` passes at HEAD; card Oracle texts and the 23-name resolution (Scryfall, jq).

UNESTABLISHED (no cargo this session; each is a named executor probe): P-deck (V1's 80 cards, one pool, guard admits); P-mull (V3 order inequality, and the pre-Phase-11 interim where P1's mulligan returns an empty hand); P-draw (the Lonely Sandbar cycling flow completes for P1 with the drawn card landing in the owner's hand without a panic); P-conj (the V7 index and stack order; Calim's "up to one target" prompt and the "may exile two" offer with one Calim); P-dig (Halimar Depths pauses at the choice with facts recorded, and `library::resolve_and_apply_library_shuffle` is callable from the integration crate); P-replay (journal accessors visible from the integration crate); P-closure (the retain closure compiles with a hoisted `Copy` resolver); that every existing non-shared test is unchanged (identity branch).
