# Phase 14 implementation review r2 (phase mode, delta)

Delta: 99fff2ab44c4992486c1dbe1912f8f04052cde2d..630000f3c5c13b31932e8342c52c9ea92b4f1c77 (PHASE_BASE 2964d6818f). Review Head: 630000f3c5.

VERDICT: no findings. r1 MED closed. Counts: behavior 0, text 0, machinery 0.

- Diff names only crates/engine/tests/integration/dandan_simultaneous_draw.rs (+57/-1); production untouched.
- Cards: Alms Collector "If an opponent would draw two or more cards, instead you and that player each draw a card.", Quantum Riddler "As long as you have one or fewer cards in hand, if you would draw one or more cards, you draw that many cards plus one instead.", Prosperity "Each player draws X cards." (jq over client/public/card-data.json).
- CR 121.2a (docs/MagicCompRules.txt): replacement effects referring to number of cards drawn modify the instruction before any individual draw; subject matches the Settling-stage park.
- Reach-guard: ReplacementChoice for P1, frame stage Settling{next:1}, no CardDrawn before the prompt; post-answer asserts frame retired, hands 3/1, 4 draws.
- Probe (git-archive copy, cold target, deleted after): control 16/16 green incl. v12. mutA (settle_dealer_seat Break ignored at the resume site) red: v12 (fails at its own parked-frame assertion, frame lost) + v11. mutA2 (park swallowed inside settle_dealer_seat) red: v12 + v11. mutB (Settling->Dealing transition replaced by Settling{next+1}) red: v12 + 12 others.
