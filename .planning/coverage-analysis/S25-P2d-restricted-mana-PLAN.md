# S25 B8 / P2d — Restricted-Produced-Mana Spend Condition (Tin Street Gossip & Overgrown Zealot)

## STEP 0 — MEASURED RESIDUAL (measure-twice; tranche-doc premise PARTLY FALSE)

Oracle text (live Scryfall):
- **Tin Street Gossip** {2}{R}{G} — Vigilance / `{T}: Add {R}{G}. Spend this mana only to cast face-down spells or to turn creatures face up.`
- **Overgrown Zealot** {1}{G} — `{T}: Add one mana of any color.` / `{T}: Add two mana of any one color. Spend this mana only to turn permanents face up.`

ALREADY EXISTS (verified true):
1. Parser COMPLETE, no gap. `parse_turn_face_up_clause` (parser/oracle_effect/mana.rs:1750) + `parse_face_down_spell_clause` (:1780) nom combinators. Tin Street → `Any([FaceDownSpell, TurnPermanentFaceUp])`; Overgrown → `TurnPermanentFaceUp`. Pinned by passing test `mana_spend_restriction_noncast_action_class_leaves_parse_completely` (mana.rs:3796-3827). NO parser change.
2. All type variants exist: `ManaSpendRestriction::{FaceDownSpell,TurnPermanentFaceUp,Any}` (types/ability.rs:1788/1799/1802); `ManaRestriction::OnlyForSpecialAction(SpecialAction)` (types/mana.rs:586); `SpecialAction::TurnFaceUp` (types/mana.rs:358); `PaymentContext::SpecialAction` (types/mana.rs:317).
3. Runtime GATE complete + tested: `ManaRestriction::allows` OnlyForSpecialAction arm (mana.rs:827), unit-tested mana.rs:2557-2561 + tests/restricted_mana_face_down_and_face_up.rs:278-281.
4. Special-action payment plumbing exists: `pay_special_action_mana_cost(...action)` (casting.rs:11658); analogous **UnlockDoor** emission live at engine.rs:146 (GameAction::UnlockRoomDoor), proven by unlock_door_restricted_mana_pays_room_unlock_cost (engine_tests.rs:1552).

TRUE RESIDUAL (contradicts plan-doc "just emit a PaymentContext"):
- `morph::turn_face_up` (morph.rs:210-312) flips the permanent but **charges NO cost** — nothing to attach a PaymentContext to. Its only production entry `GameAction::TurnFaceUp` handler (engine.rs:4083-4092) calls it directly, no payment. **Latent rules bug** (CR 702.37e/702.168d/701.40b: pay the cost, THEN turn face up). Residual = implement the PAYMENT, not merely route it.
- Consequently `has_payable_branch` (types/ability.rs:1828) hardcodes `TurnPermanentFaceUp => false` (:1842) + `FaceDownSpell => false` (:1836). Sequence-absorption seam (sequence.rs:5244-5251) only absorbs when has_payable_branch true → both cards' Effect::Mana left unabsorbed → `Effect::Unimplemented` (honest red). Pinned by passing tests: overgrown_zealot_turn_face_up_only_is_unsupported_gap (oracle_tests.rs:6421), tin_street_gossip_face_down_or_turn_face_up_is_unsupported_gap (:6455), has_payable_branch_distinguishes_live_and_dead_leaves (ability.rs:18241). card-data.json: both `supported: null`.

VERDICT: (a) implement morph/disguise/manifest turn-face-up cost payment routed via `PaymentContext::SpecialAction(TurnFaceUp)` at the GameAction::TurnFaceUp handler; (b) flip has_payable_branch(TurnPermanentFaceUp)→true. FaceDownSpell stays honest-dead (no engine face-down-cast-via-spell-payment path; CR 702.37c {3} morph cast unimplemented) — Tin Street Gossip supported via the `Any` short-circuit on its live turn-up branch (rules-honest).

## add-engine-variant gate: NO EXTENSION
All slots EXIST_SAME_NAME (ManaSpendRestriction::TurnPermanentFaceUp, ManaRestriction::OnlyForSpecialAction, SpecialAction::TurnFaceUp, PaymentContext::SpecialAction). Pure runtime wiring + a bool-classifier flip. **Zero cross-crate exhaustive-match churn** — mtgish-import (convert/action.rs:6686 maps only XCostOnly/ActivateOnly/SpellType) + phase-ai need NO changes. The "thread a new field through consumers" trap (hit twice this session) does NOT apply.

## Analogous Trace: UnlockDoor special-action restricted-mana (exact sibling)
types/mana.rs (SpecialAction::UnlockDoor:345, PaymentContext::SpecialAction:317, allows:827) → parser parse_unlock_door_clause (mana.rs:1801) → ManaSpendRestriction::UnlockDoor (ability.rs:1763) → has_payable_branch=>true (:1856) → seam absorb (sequence.rs:5245) → **runtime emit** in GameAction::UnlockRoomDoor handler (engine.rs:135-153: apply_special_action_cost_reduction → pay_special_action_mana_cost(...UnlockDoor)) → proven engine_tests.rs:1552. My change replicates the runtime-emit leg for TurnFaceUp at the GameAction::TurnFaceUp handler.

## Pattern coverage (build-for-the-class)
(a) Cost-payment fix corrects turn-face-up for EVERY Morph/Megamorph/Disguise/Manifest card (dozens) — general CR-702.37e/702.168d/701.40b correctness fix. (b) Classifier flip makes every "produce mana spendable only on turn-face-up" card supported: Tin Street Gossip, Overgrown Zealot, turn-up branch of mixed cards (Creeping Peeper), future printings. Primitive = CR-106.6 special-action spend class.

## Building blocks (reuse)
- casting::pay_special_action_mana_cost (casting.rs:11658) — reuse verbatim with SpecialAction::TurnFaceUp.
- casting::apply_special_action_cost_reduction (casting.rs:14346) — include for symmetry (no-op unless ReduceActionCost{TurnFaceUp} static).
- morph::turn_face_up (morph.rs:210) — keep signature UNCHANGED so its ~8 direct-caller unit tests + the free Effect::TurnFaceUp grant path (effects/turn_face_up.rs) stay free.
- BackFaceData (game_object.rs:177: keywords: Vec<Keyword>, mana_cost: ManaCost) — cost source.
- Keyword::{Morph,Megamorph,Disguise}(ManaCost) (keywords.rs:617/618/653).

## Logic placement
- Cost extraction+validation → new pure `turn_face_up_prepare` split out of turn_face_up's front half (game/morph.rs).
- Payment emission → GameAction::TurnFaceUp handler (game/engine.rs), NOT inside turn_face_up (would break free callers + free grant path).
- Liveness classification → has_payable_branch (types/ability.rs) flip one arm.
- Doc-comment truth updates → types/ability.rs:1789-1799, types/mana.rs:320-340/353-358.
  - **[REVIEW C1 — a6a561156f]** ALSO update `game/effects/mana.rs:355-361` (the `TurnPermanentFaceUp => OnlyForSpecialAction(TurnFaceUp)` lowering arm): its "No payment site emits PaymentContext::SpecialAction(TurnFaceUp) yet, so the gate is conservatively unsatisfiable — honest-deferred" comment becomes FALSE after Step 2. File is NOT barred.
  - **[REVIEW C2 — a6a561156f]** ALSO correct stale doc in `tests/restricted_mana_face_down_and_face_up.rs`: module doc L33-38 ("NEITHER is reachable ... nor emits PaymentContext::SpecialAction(TurnFaceUp)") + `overgrown_zealot_turn_face_up_mana_rejects_every_live_context` doc L223-235 ("charges no mana in this engine yet ... no payment site emits ... yet"). Assertions survive (operate at pool.spend_for/allows level) — doc-truth only, no test break.
  - **[REVIEW N2 — a6a561156f]** DO NOT touch `creeping_peeper_full_mana_line_no_unimplemented` (oracle_tests.rs:6376): its Any is already live via SpellType/UnlockDoor → stays green, not a missed pin.

## Steps
**Step 1 — game/morph.rs: split validation+cost out of turn_face_up (no sig change).**
Extract front-half guards (:216-277) + back-face cost into `pub(crate) fn turn_face_up_prepare(state, object_id, player) -> Result<ManaCost, EngineError>`. Cost = `back_face.keywords.iter().find_map(|k| match k { Keyword::Morph(c)|Keyword::Megamorph(c)|Keyword::Disguise(c) => Some(c.clone()), _ => None })`.or_else(|| back_face.card_types.core_types.contains(&CoreType::Creature).then(|| back_face.mana_cost.clone())).ok_or(...). turn_face_up calls turn_face_up_prepare for guards (discard cost — free at primitive level), then runs existing commit half unchanged. CR 702.168d verified docs:5235.

**Step 2 — game/engine.rs GameAction::TurnFaceUp handler (:4083-4092): charge cost (mirror UnlockDoor :135-153).**
```
let p = *player;
// CR 116.2b + 702.37e/702.168d + 701.40b + 106.6: turn-face-up special action paid via PaymentContext::SpecialAction(TurnFaceUp).
let cost = super::morph::turn_face_up_prepare(state, object_id, p)?;
let cost = casting::apply_special_action_cost_reduction(state, p, SpecialAction::TurnFaceUp, cost);
casting::pay_special_action_mana_cost(state, p, Some(object_id), &cost, SpecialAction::TurnFaceUp, &mut events)?;
super::morph::turn_face_up(state, p, object_id, &mut events)?;
```
Order prepare→pay→commit (prepare fully validates so commit can't fail post-payment).

**Step 3 — types/ability.rs has_payable_branch: flip TurnPermanentFaceUp DEAD(:1842)→LIVE(:1844-1856).** `// CR 116.2b + 702.37e: now live`. FaceDownSpell=>false unchanged. Update type doc :1789-1799 + types/mana.rs:320-340/353-358.

**Step 4 — flip honest-red pins → supported-green.**
- oracle_tests.rs:6421 (overgrown_zealot): assert NO Unimplemented; Effect::Mana w/ OnlyForSpecialAction(TurnFaceUp); is_mana_ability. Rename off `_is_unsupported_gap`.
- oracle_tests.rs:6455 (tin_street_gossip): NO Unimplemented; Effect::Mana R+G w/ OnlyForAny([OnlyForFaceDownSpell, OnlyForSpecialAction(TurnFaceUp)]). Rename.
- ability.rs:18241 (has_payable_branch test): flip :18251 to assert TurnPermanentFaceUp.has_payable_branch(); change all-dead example :18254 Any([FaceDownSpell,TurnPermanentFaceUp])→Any([FaceDownSpell]); add now-live Any([FaceDownSpell,TurnPermanentFaceUp]) assertion. Update revert-doc :18227-18239.

## Verification matrix (revert-failing runtime tests, drive apply(GameAction::TurnFaceUp))
Add to game/engine_tests.rs. Fund pool with ONLY the restricted unit (sole payment source); no other untapped mana.
- R1 (positive reach-guard): morph creature morph {3}, pool=3× ManaUnit(Green, OnlyForSpecialAction(TurnFaceUp)) → Ok; !face_down; TurnedFaceUp event; pool total()==0.
- **R2 (LOAD-BEARING charge proof): same creature, EMPTY pool, no sources → Err ("Cannot pay mana cost"); still face_down.** Reverting Step 2 (free) → R2 flips to Ok → fails.
- R3 (context precision, hostile): pool=3× ManaUnit(Green, OnlyForSpecialAction(**UnlockDoor**)) → Err; still face down.
- R4 (restriction negatives): keep existing overgrown_zealot_turn_face_up_mana_rejects_every_live_context (tests/restricted_mana_face_down_and_face_up.rs:237); ADD positive production-payment arm per :233-235.
- R5 (parse-level, Step 4): both cards → Effect::Mana w/ turn-up restriction, no Unimplemented.
- R6 (classifier unit, Step 4): has_payable_branch both directions.
Every negative (R2/R3) paired w/ positive reach-guard R1 through same pay_special_action_mana_cost — no vacuous pass via upstream Unimplemented (effect now real Effect::Mana).

## ⚠️ DRIVER/REVIEW SCRUTINY (hot-path regression risk)
GameAction::TurnFaceUp now CHARGES mana where it was FREE. EXISTING tests that drive GameAction::TurnFaceUp expecting free turn-up will now FAIL (need mana funding). The review-plan + executor MUST enumerate every existing test/caller of GameAction::TurnFaceUp (morph/disguise/manifest suites) and fund their pools OR confirm they don't hit the paid handler. This is the primary regression surface. (Free Effect::TurnFaceUp grant path stays free — only the special action gains a cost.)

## Constraints
- Barred s07-frozen (effects/mod.rs, delayed_trigger.rs, filter.rs): NOT touched. Free grant Effect::TurnFaceUp (effects/mod.rs:3178) untouched. No STOP.
- Nom: no parser change.
- CRs grep-verified: 106.6 (docs:425), 116.2b (:900), 702.37e (:4292), 702.168d (:5235), 701.40b (:3634).
- Per-commit gate: check --workspace + clippy --workspace --exclude phase-tauri --all-targets --features engine/proptest -D warnings + test -p engine. No new field/variant → no cross-crate threading.
- Coverage: post-change both cards flip unsupported→supported (net gain). CI coverage-regression passes only after Step 4 flips the 3 pins. FaceDownSpell stays honest-red (don't claim Tin Street face-down-cast works).

## Out of scope (flag, don't build)
AI legal-action ENUMERATION of GameAction::TurnFaceUp doesn't exist (no generator constructs it; ai_support/mod.rs:133 is only cheap_reject). Making AI CHOOSE to spend this mana on turn-up = separate AI feature (can_pay_special_action_mana_cost_after_auto_tap-gated enumerator, planechase.rs:375 precedent). NOT part of B8/P2d — proven via direct GameAction::TurnFaceUp dispatch R1-R3.
