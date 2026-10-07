# Phase 18 implementation review r1 (phase mode)

MODEL: claude-sonnet-5-5. Range 303d76fa69..f1a649cc5a (single commit). W clean at candidate before and after (porcelain empty, HEAD == f1a649cc5a, candidates.rs sha 88659571b4a69262 == pre-probe).

## Verdict
No blocking findings. behavior 0, text 0, machinery 0. One LOW non-blocking naming note, one pre-existing list entry.

## Checks
1. Card read first: Predict oracle (client/public/card-data.json, plan Step 0) = "Choose a card name, then target player mills a card. ..."; expected shared-library domain = source name, battlefield, controller hand, graveyard, registered pool names (registered order), exile, deduped, cap 24. The diff produces exactly that; V4 and v8 assert the literal lists.
2. Seam: fix is at the generator. Sweep over candidates.rs production (lines before the `mod tests` at 6409), pattern `library_of|\.library\b|Zone::Library|Zone::Hand|\.hand\b|hand_of|zone_object_ids|objects\.values\(\)|top_of_library|players\[.*\]\.hand|\.name\b`: hits are deciding-seat hand reads (4242, 4483, 4572, 4682, 4976, 5050, 5640), top-of-library permission reads (4284, 4708), deck-entry names (5458, not a resampled zone), the search name-collapse (6001, over a WaitingFor list; searched cards are pinned by `remember_card_identities`), and the changed generator (5589-5611, positive control: the pattern matches it). Only the top-of-library reads are a further member; see Pre-existing.
3. V3 and gate: `match state.shared_zone_holder(Zone::Library)` (types/game_state.rs, exhaustive over Zone via `format.shared_zones()`), no format literal. Per-seat arm unchanged.
4. Mutation probes in place on the warm target (`p18-rev-run.sh`, logs `scratch/p18-rev-mutA.log`, `p18-rev-mutB.log`):
   - A = shared arm never taken (`.filter(|_| false)`): V1 FAIL ("actual states agree"), V2 FAIL (`finalize_mean` "observed in 1/2 samples (support drift)"), V4 FAIL (`[Pool 3..]` vs `[Pool 0..]`), v8 FAIL (`Control Magic` missing), V3 PASS.
   - B = pool domain with no gate (`.or(Some(PlayerId(0)))`): V3 FAIL (`Deep` missing), v8 FAIL (Standard row `[Memory Lapse]`), V1/V2/V4 PASS.
   - Reach guards: V1 `redistributed > 0` is evaluated and passes at the candidate (V1 PASS under B reaches the seed loop); V2 contract domain > 1 and non-empty ensemble pass; V4 asserts pile shared, pool 30 > cap 24, pile name sets differ. Note: under A, V1 trips on the actual-state equality before the seed loop; V2 carries the sampled-world red.
5. CR: `grep -naE "^400\.2([^0-9]|$)"` resolves ("Library and hand are hidden zones"); the comment's subject (hidden zone, so identities are not the domain) matches. Comments are one sentence each; no history/evidence; no serialized or protocol surface touched (4 files: candidates.rs, search.rs inline tests, integration row file, one determinize.rs doc sentence; all in scope.nul or the addenda).
6. Plan-review constraints: V3/V4 construct the prompt directly (executor report states so); determinize.rs sentence sits inside the paragraph and copies no `candidates.rs:NNNN` coordinate; probe tests carry one-sentence comments; CR comment is the supplied replacement. All closed.

## Non-blocking
- [LOW, text] `dandan_analysis_ai_support_reads.rs` test name `v8_card_name_candidates_come_from_the_pile`: the shared rows now take the pool, so "come_from_the_pile" is false for them. Renaming is exit-round material; no behavior change.

## Pre-existing (untagged, not blocking)
- Shared-pile top-of-library permission candidates drift across determinized worlds. `casting::top_of_library_land_playable_by_permission` / `top_of_library_plot_source` read `library_of(player).front()` identity; `determinize::pinned_known_ids` does not pin a top card a Future Sight/Bolas's Citadel-class static makes visible to its controller, and `unknown_slots` resamples the whole shared pile. Probe (temporary test in search.rs, removed): Dandan, P0 with `TopOfLibraryCastPermission{Play, Unlimited}`, pile top = land: `PlayLand` candidates actual=1, sampled over seeds 0..16 = [0 x11, 1, 1, 0 x3] (support drift of the same shape as the Predict finding). Reproduces at the base (those functions and `determinize.rs` are untouched by the change); no row the change asserts depends on it. Cause is the pin set, not a generator.
- (From plan review, not repeated) the K-sample scorer drops the source's own name; per-seat branch reads the own face-down library by identity.
