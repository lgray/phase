MODEL: claude-sonnet-5-5

Review Head: 995d99af2d923480f141f9fbc680cf64775907a4 (delta 418fb9a95e..995d99af2d, mulligan.rs only; PHASE_BASE 4bc7c2a358)

Verdict: clean. 0 blocking findings (behavior 0, text 0, machinery 0). F1 and the three LOW text constraints closed. Charter decisions stand.

## Evidence

1. F1 class closure. Probe (scratch git-archive copies of base 4bc7c2a358 and candidate, own target dirs, one in-tree test appended identically to both): formats std2, std3, ffa3, ffa4, 2hg4 x seeds {1,2,3,7,42} x EVERY starting seat, real entry `start_game_with_starting_player`, driven mulligans (Mulligan / Keep / bottoms, 432 Mulligan actions, 124 bottom actions, 160 scenarios, 0 runaways), full per-action event Debug + final hands/libraries + final WaitingFor. Normal turn direction: base and candidate outputs identical in every format including 2hg4 start=P1/P3 (also team-turn and free-for-all). Instrument live: variant A (helper body = `apnap_order_from(state, None, active)`) differs from base in exactly the 2hg4 legs (10 sections, the other formats identical) and the new test goes red at "starting player P1" (left [0,1,2,3], right [1,2,3,0]); variant B (helper offset by one seat) differs in all 5 formats and also reddens `standard_opening_deal_event_order_is_seat_by_seat`. Reach guard `has_shared_team_turns()` present.
   Reversed `turn_direction` legs (3+ seats) differ from base (candidate walks backward from the active seat, base walked forward); not a finding: every game starts on a fresh `GameState::new` (Normal; `restart_between_games_with_starting_player` rebuilds), so a Reversed pregame is unreachable.
   Helper vs `players::apnap_order_from` non-shared branch: the seat-walk loop is duplicated, but (a) the same `turn_order_index` + `is_alive` walk already exists inline at topology.rs (3 sites), vote.rs and turns.rs, so it is the codebase's idiom, not a missed block; (b) `players.rs` is not in scope.nul, so extracting it needs a scope widening. Not a finding.
2. Interleaved arm for a shared-team format: `ordered` is computed from the helper before the `deal_order()` match, so ordering is format-independent by construction; `deal_order()` is derived from `GameFormat` (Dandan alone is Interleaved, 2 seats), so no config can combine the two and the existing Dandan interleave test covers the arm. No dedicated test needed.
3. Redrawers order: `close_declare_round` redrawers and the first deal both use the same active-first walk; entries follow `seat_order` (rotated active-first at game start). Matches CR 103.5 (starting player declares first; Dandan text: dealt one at a time starting with the active player). Dandan is 2 seats so apnap and the seat walk coincide.
4. New test discriminating: variant A red at start P1 (61 sibling mulligan tests unaffected per executor; here the 2 sibling tests I ran stayed as expected).
5. Text constraints: Mulligan bullet and closing paragraph of `handle_mulligan_decision` doc, deal comment, and "CR 103.5 final sentence + CR 103.5c" all applied. Greps: CR 121.2c = "If more than one player is instructed to draw cards, the active player performs all of their draws first" (comment now disclaims it); CR 103.5 = the deal/mulligan procedure; CR 103.5c = multiplayer first mulligan not counted toward the mulligan number (cap path). Subjects match.
6. Whole-artifact pass (delta has zero design findings): the r1 walks over the declare round (held declaration, return-all/shuffle-once-per-holder, implicit Keep at cap, owed bottom keeps the round open), serialized surface, consumers and CR numbers were done on 418fb9 and the delta touches only mulligan.rs ordering/doc, so none moved; re-read close_declare_round / handle_mulligan_decision / advance_after_decision on the head and found no new defect. `node scripts/check-protocol-version.mjs` rc 0 on the candidate. Not run: full nextest, vitest (p12a/b).

## Pre-existing
None relevant.

## Cleanup
Scratch copies, reflinked target dirs and background processes removed; W untouched (HEAD 995d99af2d, clean).
