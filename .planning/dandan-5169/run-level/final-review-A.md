# Final review A (ordinary) — crates/engine/src/{game,types}, base 4ab8245808..head b0ca259fbf

MODEL: claude-sonnet-5-5
Verdict: no HIGH. 1 MED [behavior]. 5 LOW (behavior 2, machinery 3, text 0). Read-only slice: no probes or builds run.

## Blocking

**[MED][behavior]** Hand-entry ownership rebind is seeded only on draw/dig/seek/engine_resolution_choices; other Hand-bound deliveries from the shared piles carry `performed_by: None`, so `hand_entry_receiver` returns None and the card keeps its original owner (CR 400.3 owner's hand) under `HandEntryOwnership::ReceiverOwns`.
Evidence: `.hand_taker(` callers (grep over game/): effects/dig.rs, effects/seek.rs, engine_resolution_choices.rs (`...Hand` arm), zone_pipeline.rs draw (`performed_by: Some(drawer)`), effects/explore.rs (`.performed_by(controller)`). Unseeded Hand-bound `ZoneMoveRequest::effect(..)`: effects/reveal_until.rs kept-card arm (`Zone::Hand | Zone::Graveyard | ...` => `ZoneMoveRequest::effect(hit, hit_destination, ..)`), effects/bounce.rs (Graveyard->Hand via `move_dest`, several request sites), effects/scoped_library_search.rs Hand arm. A Raise Dead class effect returning an opponent-owned card from the shared graveyard to hand leaves it owned by the opponent, and the card then goes to the wrong graveyard/library pile later.
Why it matters: the rebind is a class rule (the format's `hand_entry_ownership` axis) but is applied per call site, so each missed site is a silent rules miss. Whether a card in `DANDAN_DECKLIST` reaches a missed site was not measured (no probe).
Suggested fix: derive the taker once where the pipeline already knows the controller of the effect (ability controller for effect-caused Hand moves) instead of per-effect `.hand_taker(..)`, or seed the missed sites with `.hand_taker(ability.controller)`; add one test per site showing owner flips for a shared-graveyard return, red when seeding is removed.

## LOW

**[LOW][behavior]** `effects/clash.rs::top_card_of_library` now reads `library_of(seat).front()`; no runtime clash test exercises the shared-library path (only the accessor swap). Add a clash test with a Dandan state, red when reverted to `players[..].library`.

**[LOW][behavior]** `loop_fingerprint` hashes stored containers; the new test builds both piles through `players[0]`, which is the canonical seat, so it cannot discriminate a hash that reads the non-canonical seat's (empty) container. Add a case where the non-canonical seat's container is non-empty-by-mistake or assert via `library_of`.

**[LOW][machinery]** game/mulligan.rs `serum_powders_in_hand` uses the literal `"Serum Powder"` while `SERUM_POWDER_NAME` (same file) is used at the other site. Replace `.eq_ignore_ascii_case("Serum Powder")` with `.eq_ignore_ascii_case(SERUM_POWDER_NAME)` (duplicate-helper/constant, §5 g).

**[LOW][machinery]** `TextSubstitution::from_label` (types/ability.rs) round-trips a prompt label with `label.split_once(" -> ")`. It parses an engine-generated label, not Oracle text, so the nom mandate does not strictly apply; the clean fix is a typed choice payload (the prompt carries `TextSubstitution` instead of a string). Ride-along.

**[LOW][machinery]** `game/text_substitution.rs` `WORD_CARRIERS` is a hand-maintained (tag,key) table walked over serde JSON; fail-closed on unknown tags, but a new typed-word-carrying variant requires remembering the table. Ride-along: keep the existing fail-closed test as the guard.


## Pre-existing (reproduce at base, never blocking)
- `clash.rs` `top_card_of_library` reading `.library.last()` at base (bottom of a deque whose top is `front`): base defect, the change replaced it.
- `crates/phase-ai/src/search.rs:76` duplicates the "Serum Powder" literal (outside slice A).
- Old `is_plain_parent_target_delivery` had no `optional_player` rider check (base).

## Instruments
- nom-mandate grep over `git diff 4ab8245808 b0ca259fbf -- game types` added lines for `.contains("`, `.find("`, `.split_once`, `.splitn`, string-literal match arms: only match is `split_once(" -> ")` above. Positive control: the same grep run against the diff matches that line (sliceA.diff line `let (from, to) = label.split_once(" -> ")?;`), so the instrument is live.
- CR citations: every CR number in added lines resolved against docs/MagicCompRules.txt (list in scratch/crs.txt); no unresolved numbers; subjects checked for the Dandan-specific ones (103.5b mulligan, 121.1 draw, 400.3 owner's hand, 613.8b dependency loop, 201.2 name).
- Raw-zone read sweep (`\.library\b|\.graveyard\b` on player structs, scratch/raw_reads.txt): remaining raw reads are union reads (correct). Out-of-slice raw reads for B/C: manabrew-compat lib.rs ~3258, phase-ai planner/mod.rs ~318 and ~447, ai_commander.rs ~776.
- Not verified: Dandan decklist card-name resolution against card-data (jq check timed out on the loaded box).
