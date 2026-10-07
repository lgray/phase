**Changes requested at `10b758a281b9b29c32c818beef8062dd44df40ec`: one MED structural finding from the completed cumulative source review.**

## 🔴 Blocker findings

**[MED — structural] Preserve printed mana-symbol versus color-word provenance.** Evidence: [`text_substitution.rs:87`](https://github.com/phase-rs/phase/blob/10b758a281b9b29c32c818beef8062dd44df40ec/crates/engine/src/game/text_substitution.rs#L87) classifies every `ManaColorSpent.color` as a replaceable word. The live symbolic parser at [`conditions.rs:5097–5123`](https://github.com/phase-rs/phase/blob/10b758a281b9b29c32c818beef8062dd44df40ec/crates/engine/src/parser/oracle_effect/conditions.rs#L5097) emits that same condition for printed mana symbols. The rewrite/restamp path (`text_substitution.rs:219,473`; `stack.rs:1659`) changes the condition color, while [`effects/mod.rs:19841`](https://github.com/phase-rs/phase/blob/10b758a281b9b29c32c818beef8062dd44df40ec/crates/engine/src/game/effects/mod.rs#L19841) compares it with the spell's unchanged recorded payment.

Actual Oracle provides a concrete witness: cast Firespout using red mana and no blue, then respond with Sleight of Mind or Crystal Spray targeting it and choose Red → Blue. Firespout's rider is printed with `{R}`, not the color word “red”; the color-word change must leave that rider intact. This implementation instead changes its spend condition to Blue, so the non-flying damage branch is skipped. Batwing Brume's printed `{W}`/`{B}` spend conditions share the defect. The text-change authority is format-independent, so this affects ordinary-format interactions outside the fixed Dandan list. This is a high-confidence deterministic source trace, not an executed reproduction.

Physically checked `docs/MagicCompRules.txt`: CR 612.1 says text changes “can apply to any words or symbols printed on that object”; CR 612.2 says “A text-changing effect changes only those words that are used in the correct way”. The actual text-changing spells select color **words**; semantic color identity alone does not establish that lexical category.

Preserve word/symbol provenance through the existing typed condition/parser/rewriter authority. Do not simply mark every `ManaColorSpent` tag as SYMBOL: `oracle_trigger.rs:9918` also uses it for genuine word-form Adamant text. Add an Oracle-backed production cast/payment/response/choice/resolution regression using Firespout and/or Batwing Brume, paying the printed-symbol color and choosing a different, unpaid color; assert the original effect occurs. Pair this with a genuine word-form spend condition that must change. The assertions must distinguish this head from the correction; the carrier census and existing mana-production-symbol controls do not cover that distinction. This contract correction needs the full reviewed implementation workflow, not an unreviewed local patch.

## ✅ Clean

The fresh independent cumulative review completed all 198 changed paths from actual merge-base `4ab824580838640e817fac7190280d9433851401`, including production consumers, test registration/census and an explicit adversarial pass. The broad format work reuses engine-owned shared pile, continuation, zone and viewer authorities across engine/transport/UI/AI. No additional material finding was confirmed by that completed pass.

The exact-head immutable artifact `11493462092` from run `37642872708` and the [current sticky](https://github.com/phase-rs/phase/pull/9669#issuecomment-6041496799) reconcile to all nine claimed cards and six signatures, with no Oracle changes or added/removed cards. This is parse-scope evidence, not proof of correct carrier semantics. No direct build, test or runtime reproduction was run by this review.

## 🟡 Other review evidence

The newly published [separate current-head formal review](https://github.com/phase-rs/phase/pull/9669#pullrequestreview-5444876712) raises layer dependency ordering and sampled card-name-domain findings. It is preserved and requires independent reconciliation against this head; this review does not clear, refute or duplicate those findings. Its later publication occurred after the completed critic's feedback snapshot. CI success and CodeRabbit's file-limit skip are not substitutes for that reconciliation or the distinguishing regression above.

**Recommendation: correct lexical provenance at the shared typed authority, add the discriminating production controls, reconcile the separate current-head findings and obtain fresh committed-head review. Keep this PR open with changes requested and its feature label; do not approve or enqueue.**

