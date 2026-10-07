# Phase 18 plan — card-name candidate domain constant across determinized worlds

Base `PHASE_BASE_SHA` = `303d76fa69`. Task: maintainer MED on PR #9669 (Predict card-name finding in `run-level/review-9669.json`). Charter entry: `phase-charter` RE-CHARTER r6, Phase 18 (deferral list: none).

## Shape of the code (stated by `crates/phase-ai/src/determinize.rs` module header; confirmed by one trace)

`score_candidates_with_session` (`phase-ai/src/search.rs`) runs K samples; each sample is `determinize_opponents` (resamples the opponents' unknown hand slots and `library_of(opponent)` slots from ONE shuffled pool built by `deck_knowledge::unknown_hidden_pool` from `deck_pool_of`) followed by `score_candidates_core`, which rebuilds the candidate list through `engine::ai_support::candidate_actions` on the sampled state. `finalize_mean` requires every action observed K times; the actual-state `AiDecisionContract` filters the final pick. The pin invariant in the header (candidate support constant across samples) is the contract; the generators own keeping it. The trace was driven (probe below): at base the K=2 Predict prompt reaches `finalize_mean`'s support assertion.

## Step 0 — premise

- Predict oracle text, verbatim, from both sources: `curl -s 'https://api.scryfall.com/cards/named?exact=Predict' | jq -r .oracle_text` and `jq -r '.["predict"].oracle_text' client/public/card-data.json` both return: "Choose a card name, then target player mills a card. If a card with the chosen name was milled this way, you draw two cards. Otherwise, you draw a card." The finding's quoted clause is real; the choice is made on resolution with `ChoiceType::CardName`, `options` empty, so candidates come from `card_name_choice_candidates`.
- CR: `grep -n "^400.2\|^401.2" docs/MagicCompRules.txt` — CR 400.2 (library and hand are hidden zones), CR 401.2 (library is a single face-down pile). The annotation uses CR 400.2 only: it is the rule whose subject (hidden-zone identities are unknown to the chooser) is the claim.

## Claims established at the base (bought by command)

1. Library read is the only candidate-domain read of a resampled zone in the generators. `git grep -n 'library_of\|\.library\b' -- crates/engine/src/ai_support/candidates.rs crates/engine/src/ai_support/mod.rs` (non-test) has one hit: `card_name_choice_candidates`. The other zone reads (`.hand`, `graveyard_of`, `battlefield`, `exile`) were read: hand reads are of the deciding seat (`player`), which `determinize_opponents` never resamples when `player == ai_player`; graveyard/battlefield/exile are public. Population: the grep walked `ai_support/` production lines; semantic-owner prompts where `player != ai_player` were not walked and are not part of this finding.
2. Drift is real, shows as predicted: probe row V2 at base fails in `finalize_mean` on its support check (see Verification).
3. `current_main` writers: `git grep -n 'current_main *=' -- crates` (non-test hits: `match_flow.rs` between-games sideboard submit, `visibility.rs` viewer-redaction on a filtered clone) plus the struct literals in `deck_loading.rs` at load; no writer runs inside a game. Sibling reader: `deck_knowledge::unknown_hidden_pool`/`known_remaining_deck_counts`. The AI entry points receive the authoritative state (`engine-wasm/src/lib.rs` `choose_action_with_session(state, ..)` calls), not a `filter_state_for_viewer` clone, so the redaction does not reach the pool the generator reads (derived by reading the call sites; not driven).
4. The unconditional pool domain breaks exactly one existing row: `dandan_analysis_ai_support_reads.rs::v8_card_name_candidates_come_from_the_pile` pins the live pile identities with no pool registered. Probed: with the fix it fails; the repair below makes it green. `git grep -n 'all_card_names' -- crates/*/tests` hits were read; the others set the list for a direct `choose_option`, not for the candidate generator.

## Fix (at the cause)

`card_name_choice_candidates` stops reading the live shared pile. Domain = source display name, battlefield, the deciding seat's hand, `graveyard_of`, then the library term, then exile (positions unchanged), deduplicated case-insensitively, capped as today. The library term is:

- `state.shared_zone_holder(Zone::Library)` is `Some(_)`: the names of `state.deck_pool_of(controller.id)`'s `current_main` entries in registered order (public decklist, written only at load/between games, the same pool `unknown_hidden_pool` samples from).
- `None`: the controller's own `library_of` identities exactly as today (never resampled), so per-seat lists are byte-identical.

The shared test is the existing engine authority `shared_zone_holder`, never a format literal. One capped pusher over `&str` serves both object-name and pool-name sources (the object loops map ids to names). No new type, field, variant, protocol or binding change.

CR annotation at the shared arm: `// CR 400.2: the library is a hidden zone, so the chooser's domain is the pile's registered pool names, not its live identities.`

## Pattern Coverage

Class: every `Choose a card name` prompt the AI answers through the candidate generator (`ChoiceType::CardName` with empty `options`) in a shared-library format; today that is Dandan, the engine's only shared-library format. Population: `jq -r 'to_entries[] | select(.value.oracle_text? // "" | test("choose a card name"; "i")) | .value.name' client/public/card-data.json | sort -u | wc -l` (exported cards whose Oracle text contains the phrase; Predict is in the Dandan list). The charter attributes the class; no single-card special case is built.

## Sizing

- Units: one — the card-name candidate-domain rule (single skill-checklist pass; no registration surfaces). No inter-unit edges.
- Scope paths counted: `crates/engine/src/ai_support/candidates.rs` (generator + inline rows V3/V4); `crates/phase-ai/src/search.rs` (inline rows V1/V2); `crates/engine/tests/integration/dandan_analysis_ai_support_reads.rs` (v8 repair, extension below); `crates/phase-ai/src/determinize.rs` (one doc-comment sentence, comment-only standing class). Expected count 4; no generated artifact, fixture or pipeline data. The charter's two paths plus two named extensions.
- Small-change-lane eligibility: one unit, at most four counted paths, no enum variant, no serialized surface, no `WaitingFor`/`GameAction` change — eligible. T1 fails, so the T1∧T2 conjunction cannot fire.
- Primary-code estimate: about 25 changed lines in `candidates.rs`, one comment sentence in `determinize.rs`.

## Scope extensions for the orchestrator's addendum (every charter decision stands)

1. `crates/engine/tests/integration/dandan_analysis_ai_support_reads.rs`: row v8 asserts the replaced behavior (live pile identities, no pool); it is repaired in-phase.
2. `crates/phase-ai/src/determinize.rs`: the module-header paragraph "Why the AI's own zones are never resampled" holds for per-seat libraries only; a shared library is resampled whole. One sentence is added there (a trap for the next generator author): a shared library is keyed by its registered pool, never by live identities.

## Building Blocks

`GameState::shared_zone_holder` (`types/game_state.rs`, `pub(crate)`, reachable from `ai_support`), `GameState::deck_pool_of` (the pile holder's pool; same resolver `deck_knowledge::pool_holder` uses), `PlayerDeckPool::current_main` (`DeckEntry.card.name`), `graveyard_of`/`library_of` accessors, the existing `push_name` legality filter against `all_card_names`. No new helper beyond the `&str` pusher.

## Logic Placement

Engine (`ai_support/candidates.rs`): it is candidate-domain logic, and the finding is a contract of the generator, not of the resampler. `determinize.rs` is not edited except the comment: the determinizer's pin invariant stays the contract; making it pin the pile would contradict the finding (charter decision, rejected alternatives). Nothing in phase-ai production changes.

## Rust Idioms

Exhaustive `match` on the `Option<PlayerId>` from `shared_zone_holder` (`Some(_)`/`None`), no format comparison, no wildcard, no `bool` axis. Iterators over `&str`, no cloned pool.

## Nom Compliance

Not applicable: no file under `crates/engine/src/parser/` changes.

## Extension vs Creation

Extends the existing generator and the Phase 17 pool-resolution pattern (`deck_pool_of`); no new pattern.

## Analogous Trace

`deck_knowledge::unknown_hidden_pool` (the sanctioned sampler of the same pool): `types/game_state.rs::deck_pool_of`/`shared_zone_holder` → `phase-ai/src/deck_knowledge.rs` (`pool_holder`, `unknown_hidden_pool`) → `phase-ai/src/determinize.rs` (`unknown_slots`, `determinize_opponents`) → `phase-ai/src/search.rs` (`score_candidates_with_session`, `merge_into`, `finalize_mean`, `AiDecisionContract` filter) → `engine/src/ai_support/candidates.rs` (`card_name_choice_candidates`). Trace driven by the base probe below.

## Variant Discoverability

No enum variant is added; `/add-engine-variant` not applicable.

## Identity / Provenance Contract

The pool is the pile holder's registered `PlayerDeckPool`, resolved per call through `deck_pool_of` (live, not snapshotted); it is written only at deck load and between games, so it is constant within a Bo1 game and identical in every determinized sample and in the actual state. Hostile fixture: V1 places the two copies of one unobserved name in two different containers and requires one list.

## Reference Readings

- V3 (per-seat list "equals base"): the card does not determine the candidate domain. CR 201.4 admits any card name in the Oracle card reference, so the generator's bounded domain is a heuristic, not a rules reading; the row preserves the base heuristic, derived-not-derivable, labelled so. The reading is measured, not copied: the literal expected list is written in the row and passes at base and after (probe: green in both).
- V1/V2 compare sampled worlds with the actual state, and the actual state's domain is stated (pool order, capped), not taken from a sibling route.
- The v8 repair states its expected list from the new rule (graveyard name, then pool names in registered order, legal only, deduplicated).

## Verification Matrix

Seam: `card_name_choice_candidates`. Production entry: `candidate_actions` on a `WaitingFor::NamedChoice { choice_type: CardName, options: [] }` reached by a real `Predict` cast. Fixture for V1/V2 (needed for the rows to discriminate): Dandan (`FormatConfig::dandan()`), P0 casts Predict through `GameRunner::cast(..).target_player(P0).resolve()` to the prompt (`/card-test` recipe); registered pool of six cards whose total count equals the number of unknown slots (so every sample is a permutation of the pool); the opponent hand holds at least two slots (a two-copy name can leave the pile only if both copies fit in the hand — with one hand slot the pile's distinct-name set never changes and the row is vacuous); `all_card_names` holds the pool names plus "Predict".

| Row | Claim | Assertion | Reach guard / sibling |
|---|---|---|---|
| V1 (`search.rs`, K>1 world) | The issued name list is identical for the actual state, a state with the unobserved two-copy name X in both pile slots, and the `determinize_opponents` sample of every seed in a range | list equality; X present | reach: list has more than one name and contains X; at least one seed changes the pile's name set (determinization moved names); red at base |
| V2 (`search.rs`, K=2) | `score_candidates_with_session` at K=2 returns a non-empty ensemble whose every action is contract-admitted, and `choose_action_with_session` returns `Some` admitted by `AiDecisionContract` | no `finalize_mean` support assertion; `Some(ChooseOption)` | reach: contract domain size above one, ensemble non-empty; red at base by the support assertion. Do not assert ensemble size equals contract size: the contract carries the source's own name and the scorer drops it at base too, a pre-existing scoring filter that every sample shares |
| V3 (`candidates.rs`, Standard twin) | Per-seat format list equals the literal base list `[battlefield, hand, graveyard, library, exile]` names | exact literal | reach: a library-only name is in the list and the opponent's library name is not; green at base and after |
| V4 (`candidates.rs`, Dandan, pool larger than the cap) | Two states with the same pool and different pile/opponent-hand placements issue identical lists, equal to the registered-pool prefix of the cap length | equality with the literal prefix | reach: pool longer than the cap, the two states' pile name sets differ, the pile is shared (`players[1].library` empty); red at base |
| v8 repair (`dandan_analysis_ai_support_reads.rs`) | Shared rows expect graveyard name then pool names in registered order; the Standard row keeps the library read | literal lists per row | pool contains a name that is in no zone (`Control Magic`), so a library read cannot satisfy it |

Hostile fixtures: the multi-authority case is the two-copy name across hand and pile (V1) and the two placements (V4); the first production branch reached is the `shared_zone_holder` arm (V1/V4 take `Some`, V3 takes `None`); empty/decline paths are not new (`all_card_names` empty returns early, unchanged and covered by `named_choice_card_name_fallback_none_when_all_card_names_empty`). A pool-absent shared state is unreachable in production (Phase 6 registers a pool at load) and gets no row.

Regenerate: `cargo nextest run -p phase-ai --lib -E 'test(predict_card_name)'` and `cargo nextest run -p phase-engine --lib -E 'test(card_name)'` and `cargo nextest run -p phase-engine --test integration -E 'test(v8_card_name)'` (per `common.md` env line). The executor shows each red leg by reverting only the shared arm. The V3 mutation (apply the pool domain without the `shared_zone_holder` gate) is derived red (the Standard fixture registers no pool), unmeasured; the executor measures it.

Coverage status impact: none (no parser or card-data change). `cargo ai-gate` is not owed by this phase: per-seat lists are byte-identical (V3) and no shipped tier samples (`determinization_samples = 0`); the run-level gate noted in `summaries.md` stands.

## Steps

1. `crates/engine/src/ai_support/candidates.rs`, `card_name_choice_candidates`: replace the `push_object_name` id closure with one capped `&str` pusher; map `state.battlefield`, `controller.hand`, `graveyard_of`, `exile` ids to object names (positions unchanged); at the library position `match state.shared_zone_holder(Zone::Library)`: `Some(_)` pushes `deck_pool_of(controller.id)` `current_main` entry names, `None` pushes `library_of(controller.id)` object names. CR 400.2 annotation as above. Fallback block and truncation unchanged.
2. `crates/phase-ai/src/determinize.rs`: one sentence in the "Why the AI's own zones are never resampled" paragraph stating the shared-library exception.
3. Tests per the matrix: V3 and V4 in the `candidates.rs` inline `mod tests` (beside `named_card_choice_uses_bounded_in_game_names`); V1 and V2 in the `search.rs` inline `mod tests` (beside `determinization_candidate_set_stable_over_resampled_opponent_hand`); repair v8 in `dandan_analysis_ai_support_reads.rs` (register a four-name pool for the shared rows; Standard row unchanged). Probed v8 repair: shared expected `[Memory Lapse, Control Magic, Island, Brainstorm]`, Standard `[Memory Lapse, Island, Brainstorm]`.
4. `cargo fmt --all`; the targeted nextest commands above; clippy on `phase-engine` and `phase-ai`.

Pre-existing, outside the findings: the scorer drops the source's own name from the K-sample ensemble while the contract issues it (observed in the V2 probe); no case the change claims depends on it.
