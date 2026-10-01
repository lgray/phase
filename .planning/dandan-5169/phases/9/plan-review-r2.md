# Phase 9 plan review, round 2 (phase-plan mode, phase-fit context declared)

Reviewer model: claude-sonnet-5-5. Plan: `phases/9/plan.md` (rev 2). No cargo run; every claim is a read or grep.

Verdict: APPROVE WITH ONE TEXT FIX (F3, text, one-line citation). F1, F2, N1 of round 1 are closed. No behavior or machinery findings. Sizing consistent (1 unit; T2 = 9 edited charter paths + 2 test paths = 11, below 13).

## Round-1 closure checks

- F1 closed. Section 3 counts re-added: S = 1 (snapshot) + 1 (seat_headroom_bound) + 2 (candidates graveyard) + 2 (card_name_choice) + 1 (mod.rs) + 1 (combo) + 1 (fetch) + 1 (determinize) + 3 (llm) = 13; F = 4 + 2 + 4 = 10; P17 5; P6 1; sum 29. Scope matrix lists determinize.rs (9 edited charter paths). V13 text, section 10 and step 0 are consistent with it.
- N1 closed. 4.2 now says the 23 names hold two one-shot mills and reach no pile-depleting cycle, and names the `classify_win_kind` / firewall effect.
- F2 closed, design measured:
  - `unknown_slots` at HEAD is `hand.chain(player.library)` filtered `!known && !is_token && owner == opponent`. The plan's replacement (hand keeps the owner filter; library = `library_of(opponent)` with known and token filters only) matches that shape.
  - `known` is `pinned_known_ids`: revealed, public-revealed, the AI's private looks, and `viewer_knows_card_identity`. Phase 6 plan 3.4 resolves `library_knowledge_epoch` by storage seat, so a P1-owned pile card's fact is invalidated by a pile reorder. `known` is therefore storage-consistent and ownership adds no knowledge information. CR 401.2 and 401.3 grepped (lines 2001, 2003) and cited correctly.
  - Hand keeps the owner filter, which is right (hand storage is per seat).
- V15 discriminates both defects. Base: the non-canonical container is empty, so only the hand card comes back (red). Mutation restoring `owner == opponent` on the library slots drops the P0-owned pile card (red). (b) opponent = P0 is red at base for the P1-owned pile card because the raw read hits the owner filter, and the P0-owned one is the reach-guard. (c) Standard control, token skip and known skip reach-guards are present. Mixed-owner order [hand, P1 pile, P0 pile] is stated, so ordering is also pinned.

## Blocking

### F3 (text) wrong rule cited for zone routing
Section 4.3 (F2 paragraph): "(CR 401.1, CR 108.3 via the zone-routing of `add_to_zone`)". CR 108.3 (grepped, line 564) defines the owner; the routing rule is CR 400.3 (grepped, line 1942: "If an object would go to any library, graveyard, or hand other than its owner's, it goes to its owner's corresponding zone"), which section 1 already lists as grepped. Replace "CR 401.1, CR 108.3 via the zone-routing of `add_to_zone`" with "CR 401.1 and CR 400.3 via the zone-routing of `add_to_zone`".

## Non-blocking

- N1 (text, suggest) 4.3 claims the read swap is unobservable until Phase 17; true for the swap. The F2 filter drop is not covered by that sentence and is observable earlier for opponent = canonical seat once a pile can hold P1-owned cards (after Phase 11's owner rebind): the slots then include P1-owned pile cards while `unknown_hidden_pool` (Phase 17's owner filter) subtracts only P0-owned public cards, so the pool is over-supplied (no truncation leak, a skewed distribution until Phase 17). Pre-Phase 11 the pile is all P0-owned (Phase 6 loads the 80 cards through P0), so nothing changes before then. Add one sentence saying so.
- N2 (text, suggest) the doc comment above `unknown_slots` (lines 157-162: "cards not owned by `opponent` (a borrowed card's identity is already public)") becomes stale for the library half; the plan's annotation sentence mentions only the new CR comment. Update that doc comment in the same edit.
- N3 `card_name_choice_candidates`, V11, V12 and the F pins were re-read and match round 1; no change.
