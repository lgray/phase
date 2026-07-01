# Hulk Targeting Fix — ROOT-CAUSE plan (v3, after CR 608.2b rejection)

Worktree: `/home/lgray/vibe-coding/wt-msh-hulk` (branch feat/msh-hulk). ISOLATED → cargo DIRECT.
HEAD: db79f6d5c (condition) + 4a7e89452 (BUGGY targeting — to be superseded).

## WHY v2 (commit 4a7e89452) WAS REJECTED — verified BLOCKER
hulk-targeting-reviewer (independent, runtime probes) + my own verification proved the narrow arm
`TargetFilter::ParentTarget | None if ability.targets.is_empty() => source` OVER-FIRES: it conflates
two cases that arrive identically (ParentTarget + truly-empty `ability.targets`):
- (a) SelfRef-head anaphor (Hulk: "put a +1/+1 counter on him ... untap him") → bind source ✓
- (b) **declined optional-target** anaphor → CR 608.2b: anaphor has NO referent → must NO-OP ✗

VERIFIED affected class (card-data.json parses): Tyvar Kell [+1], A-Tyvar Kell [+1], Nissa Who Shakes
the World [+1] all = head `PutCounter{Typed,multi_target{min:0,max:1}}` → sub `SetTapState{ParentTarget,
Untap,Single}`. Decline the optional target → my arm wrongly untaps the SOURCE planeswalker + emits a
spurious PermanentUntapped event (`process_one_untap` has no `obj.tapped` precondition). Runtime-proven by
reviewer's 2 probes through `resolve_ability_chain`.

The cited counters.rs precedent CONTRADICTS the arm: counters.rs:1412-1417 guards with
`!has_object_target && has_choice_bookkeeping_player` (NOT bare is_empty()), with an explicit CR 608.2b
note (:1405-1411) that it must NOT fire for declined "up to one target" slots. Hulk has no bookkeeping
marker either → the discriminator is ERASED by resolve time. **Conclusion: the anaphor MUST be resolved
at PARSE time, where the head's subject filter is still available.**

## ROOT-CAUSE FIX — resolve the anaphor from the antecedent's subject (parse-time)
The HEAD's target filter is the discriminator (visible in parsed AST): SelfRef (Hulk) vs Typed-optional
(Tyvar). Fix in 3 parts:

### Part 1 — REVERT the runtime arm (tap_untap.rs)
- Delete the `TargetFilter::ParentTarget | TargetFilter::None if ability.targets.is_empty() => vec![source_id]`
  arm (lines ~51-64) and its doc bullet (~24-29). Restore the original 4-bullet doc + 3-arm match
  (`SelfRef` → source; two `TrackedSet`; `_` → chosen targets). The pre-existing `SelfRef` arm (line 37)
  is what Hulk's sub will hit AFTER Part 2.

### Part 2 — REVISION (v3.1): patch lives in `lower.rs`, called inside `lower_effect_chain_ir`
The first v3 draft wired the patch into the two `parse_effect_chain*` wrappers (mod.rs:18533/18569).
That MISSED the trigger path: `oracle_trigger.rs:1091` calls `lower_effect_chain_ir` DIRECTLY (as do
`oracle.rs:1871` for activated/loyalty abilities and ~12 other sites), so Hulk's untap stayed
`ParentTarget` and both integration tests failed. Root cause: there are 15 `lower_effect_chain_ir`
callers; per-site wiring is fragile. FIX: put the patch INSIDE `lower_effect_chain_ir` (lower.rs), the
single chokepoint, as one of its post-assembly rewrite passes (next to `rewire_*`,
`retarget_counter_additional_cost_to_target`). Reuse the EXISTING building block
`definition_targets_self_source` (lower.rs:79, identical to my inline check) and mirror the established
sibling helper `rewrite_else_parent_target_to_self_ref` (lower.rs:140, the else-branch analogue for
"Repeat Offender's 'Otherwise, suspect it'"). Removed the 2 mod.rs wiring calls and the sequence.rs
function/tests. Tests colocated in lower.rs `self_ref_tap_anaphor_tests`.

### Part 2 (original draft) — NEW parse-time patch (sequence.rs) — builds for the class
```rust
/// CR 608.2c + CR 608.2b: A chained tap/untap anaphor ("untap him"/"untap it")
/// inherits its referent from the antecedent head's subject. When the head's
/// subject is the source itself (`SelfRef` — The Incredible Hulk: "put a +1/+1
/// counter on him ... untap him"), the anaphor refers to the source, so lower the
/// chained `SetTapState`'s `ParentTarget` to `SelfRef`, which
/// `tap_untap_target_ids` binds to the source. A head with a real or *optional*
/// target (Tyvar Kell: "...up to one target Elf. Untap it.") keeps `ParentTarget`:
/// it binds the chosen target, and a DECLINED optional target leaves
/// `ability.targets` empty so the sub correctly does nothing (CR 608.2b — the
/// anaphor has no referent). Scope = `Single` (the anaphoric singular); `All`
/// ("untap all ...") is a population filter, never an anaphor.
pub(super) fn patch_self_ref_head_tap_anaphor(def: &mut AbilityDefinition) {
    let head_is_self_ref = matches!(def.effect.target_filter(), Some(TargetFilter::SelfRef));
    if let Some(sub) = def.sub_ability.as_mut() {
        patch_self_ref_head_tap_anaphor(sub); // recurse deeper chains first
        if head_is_self_ref {
            if let Effect::SetTapState {
                target: target @ TargetFilter::ParentTarget,
                scope: EffectScope::Single,
                ..
            } = sub.effect.as_mut()
            {
                *target = TargetFilter::SelfRef;
            }
        }
    }
}
```
Building blocks reused: `Effect::target_filter()` (ability.rs:11295, pub; returns Some(&SelfRef) for
`PutCounter{SelfRef}` head). `target_filter_is_self_ref` is private to ability.rs → inline the
`matches!(.., SelfRef)` (do NOT widen its visibility — multi-agent safety, avoid touching ability.rs).

### Part 3 — WIRE into the patch pipeline (mod.rs)
Add `sequence::patch_self_ref_head_tap_anaphor(&mut def);` right after the existing
`sequence::patch_reveal_until_for_library_category_exile(&mut def);` at BOTH chokepoints:
mod.rs:18533 (`parse_effect_chain`) and :18569 (`parse_effect_chain_with_context` — the trigger path
Hulk's Enrage uses).

## Tests — non-vacuous + DISCRIMINATING (each with revert evidence)
1. **Parser POSITIVE (sequence.rs or parser test):** parse The Incredible Hulk → assert the chained
   SetTapState sub's target == `SelfRef`. **Discriminate:** revert Part 2 → target stays `ParentTarget` → fails.
2. **Parser NEGATIVE / CR 608.2b fence (the killer test that would have caught v2):** parse Tyvar Kell →
   assert the chained SetTapState sub's target == `ParentTarget` (NOT rewritten — head is Typed, not
   SelfRef). **Discriminate:** an over-broad patch (matching any head) flips this to SelfRef → fails. This
   is the exact discrimination v2's arm lacked.
3. **Runtime declined-optional NO-OP (tap_untap.rs):** ResolvedAbility SetTapState{ParentTarget,Single,
   Untap}, source pre-tapped, EMPTY targets → assert source STAYS tapped AND no PermanentUntapped event.
   **Discriminate:** reintroduce v2's arm → source untapped + event fires → fails. (Fences the regression.)
4. **Integration `enrage_untaps_hulk_when_attacking` (KEEP existing):** Hulk tapped + attacking + Enrage →
   tapped==false + counter==1. **Discriminate:** revert Part 2 → sub stays ParentTarget → no-op → fails.
   Not-attacking control branch stays tapped (gate, not resolver).
5. **KEEP** `untap_parent_target_with_chosen_object_does_not_untap_source` (narrowness fence) and
   `untap_triggering_source_with_empty_targets_is_noop` (class fence). **REMOVE**
   `untap_parent_target_with_empty_targets_untaps_source` (it asserted v2's now-reverted behavior).
6. Existing `untap_self_ref_with_empty_targets_untaps_source` (tap_untap.rs:388) already covers the
   resolve-time SelfRef→source path Hulk's patched sub hits — no new resolver test needed there.

## CR annotations (grep-verified vs docs/MagicCompRules.txt)
- CR 608.2c (chained-instruction anaphora). CR 608.2b (declined "up to one target" → anaphor no referent
  → does nothing). CR 701.26a/b (tap/untap). Verify each before writing.
- Minor: fix the stale CR cite on `untap_triggering_source_with_empty_targets_is_noop`'s doc (reviewer flagged
  603.7c off-target for a SpellCast trigger).

## Verify (cargo-direct in wt-msh-hulk; Tilt does NOT watch this worktree)
`cargo fmt --all`; `cargo clippy -p engine --lib --tests -- -D warnings`; `cargo test -p engine` (report
counts) + the 4 new/kept tests by name. Then FULL CI-equiv before any push (rebase-before-push lesson).

## Ship — ONE combined Hulk PR (per user "make sure it all ends up in one PR for the hulk")
Supersede 4a7e89452 (amend or follow-on commit; squash collapses). Re-review by hulk-targeting-reviewer.
Then rebase feat/msh-hulk onto current upstream/main + full CI-equiv + single `gh pr create`.

## Out of scope (documented)
- Generalizing the rewrite to non-SetTapState effects (no other verified card; each effect resolves its own
  anaphor — counters.rs has its own mechanism). Patch scoped to the proven class = SetTapState anaphors.
- `else_branch` recursion (matches `patch_reveal_until` precedent: sub_ability only).
- Pre-existing tap_untap SelfRef arm CR 400.7 incarnation guard; stale CR 603.7e doc at line 14.
