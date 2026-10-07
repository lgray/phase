# Final review, delta b0ca259fbf..10b758a281 (ordinary)
MODEL: claude-sonnet-5-5
Verdict: no blocking findings. 0 behavior, 0 text, 0 machinery. 1 LOW text ride-along. W clean, HEAD 10b758a281 at end.
Skill read from the main checkout (branch tip differs; the only delta is a "Scout fact pack" section, no pack was supplied).

## Closure (A/B/C blocking items, as classes)
- GamePage reach-guard: test now renders OPEN_ROUND, awaits "Keep Hand", swaps to VIEWER_DECLARED, `waitFor` gone. Present-then-absent by read; vitest not run (brief). Executor's two red probes not re-run.
- Crystal Spray row: baseline green; red under `draw::resolve` early-return (assert "Crystal Spray drew a card after the change", hand 0 vs 1) and red under the latch dropping the named pair (assert "Bad Moon now reads \"Red creatures\"", left (2,2) right (3,3)). `sleight` removed.
- loop_fingerprint test: baseline green; red under a hash reading `library_of(P0)`/`graveyard_of(P0)` for every seat, failing at the NEW assertion ("the non-canonical seat's own container is hashed too"), so the earlier assertions pass under that mutation (new assertion is the discriminator).
- Clash (executor item 4): `v15_clash_reveals_the_pile_top_for_either_seat` stages the pile under P0 and casts as P1 then P0; with clash.rs `top_card_of_library` reverted to the raw `players[..].library.front()` it is red (`PlayerId(1) reveals the pile top: [(PlayerId(0), ObjectId(1))]`) while `v15_clash_reveals_the_top_card_of_a_standard_library` stays green. Claim verified; no new test needed.
- candidates.rs guard: `free_reveal_actors(dandan_mulligan(Declare, 0)).len() == 1` sits at the start of the negative test, same hand pre-mulligan; positive by construction (not probed, only three probes allowed).
- CR citations: grepped 400.1 400.2 400.3 401.2 401.3 608.2c 608.2d 612.1 in docs/MagicCompRules.txt. 601.2b->612.1 (text-changing effects), 724.1->608.2c (instruction order), deck_knowledge `CR 400.3` (owner's corresponding zone), determinize `CR 400.2 + CR 401.2` (library hidden / single face-down pile), mill_targeting `401.3 + 400.1` (library size public; shared-by-all zones): subjects match.
- Jargon sweep: PR-added lines (`git diff 4ab8245808 10b758a281 -- crates client`, `+` lines) matching the brief's pattern: 0; same filter at b0ca259fbf: 5 (control live). The literal tree-wide `git grep` still returns hits, all in lines not authored by this PR (F-/S-coordinates in resource.rs, ws-adapter.test.ts, etc.; none among PR-added lines). Wider B pattern (Phase N, before the sweep, S6, V-r): 0 at candidate, 7 at b0ca259fbf.
- SERUM_POWDER_NAME: `git grep '"Serum Powder"' -- crates/engine` non-test comparison sites: none left (remaining hits are messages, doc comments, test fixtures); used at mulligan.rs lines 340 and 490.

## Delta introduces no defect
Non-comment +/- lines outside tests (`git diff -U0 b0ca259fbf 10b758a281 -- 'crates/*/src/**'`, comment lines filtered; control: same filter over 4ab8245808..b0ca259fbf mulligan.rs yields 814 lines): the SERUM_POWDER_NAME swap, plus lines inside `mod tests` (candidates.rs) and `mod shared_zone_storage_tests` (game_state.rs). No behavior change. Unrequested executor edits (candidates guard, `stray` assertion, test-name prefix drop, graveyard_types reflow) all trace to A/B/C findings.

## LOW (ride-along)
- [LOW][text] dandan_read_sweep.rs, doc on `OPPONENT_CASES` was joined into one ~120-char `///` line. Old: `/// An opponent-targeting card is cast by `P0` against the pile, by `P1` (the canonical seat: the control) and in Standard.` Replacement: wrap after "by `P1`" onto a second `///` line.

## Pre-existing (untagged)
- crates/phase-ai/src/search.rs `first_serum_powder_in_hand` still compares the literal "Serum Powder" (present at base 4ab8245808; outside crates/engine; already raised by C as LOW).

## Probe protocol
Three builds, warm target-dandan, each from empty porcelain, restored with `git checkout --` + touch via EXIT trap: P1 game_state.rs hash; P2 clash.rs + draw.rs (different tests, attributed by test name); P3 effect.rs latch. Baseline of all four rows green in the same session first. End: porcelain empty, HEAD == 10b758a281.
