# PR #4904 soundness fix — resolution-time choice-freeness gate for `loop_states_cover_modulo_growth`

Status: PLAN (no code changed). Branch `feat/combo-pr6.5-growing-cascade` @ `2e7ad800c`, worktree `/home/lgray/vibe-coding/pr65-wt`.
Fixes maintainer finding (matthewevans, CHANGES_REQUESTED, HIGH): the growing-cascade shortcut can certify a cover whose stack entries can still open **resolution-time player choices** (proliferate/populate/clash/explore/sacrifice-choice/…), i.e. future player input the C2/no-ordering-input gate does not model.

Every file:line below was read in this worktree during planning. Every CR number was grep-verified against `docs/MagicCompRules.txt` (§7).

---

## 0. Executive summary

Add a **fail-closed, compiler-exhaustive, resolver-grounded resolution-choice classifier** and wire it as a new item (6) of `loop_states_cover_modulo_growth` (crates/engine/src/analysis/resource.rs:695), applied to **every** current-stack entry (not only grown ones — §3 H2 shows the non-grown hole is real):

- New ability_scan-local 2-variant verdict enum `ResolutionChoiceFreedom { FreeUnlessLifeReplacements, MayPrompt }` (typed, not a bool — repo rule) + `ability_resolution_choice_freedom(&ResolvedAbility)` / `effect_resolution_choice_freedom(&Effect)` in `crates/engine/src/game/ability_scan.rs`, mirroring the existing walker's no-wildcard / destructure-without-`..` discipline (ability_scan.rs:106-162).
- Allow-list is exactly `{Effect::GainLife, Effect::LoseLife}` (what the shipped N-fixtures need), each grounded by a resolver trace (§5.2): `resolve_gain` (life.rs:19-110) and `resolve_lose` (life.rs:293-365) each run their own **inline** `replace_event` pipeline, and **all four** `waiting_for` raises in life.rs (:98, :157, :251, :353) are `replacement_choice_waiting_for` NeedsChoice raises — the life-event replacement pipeline is the only prompt surface (`NeedsChoice`: replacement.rs:6221-6247 single-optional, :6263-6279 material ordering; plus the mandatory body-continuation drain, §3 H4 route c).
- That residual prompt surface is closed by an analysis-local **environmental guard** `life_event_replacements_may_prompt(state)` (resource.rs). Life-class = the **registry-derived** set `{GainLife, LoseLife, LifeReduced, PayLife}` — every `ReplacementEvent` whose registry matcher matches `ProposedEvent::LifeGain|LifeLoss` (measured, §5.3(b); `PayLife` and `LifeReduced` both match `LifeLoss`). The guard scans object-attached defs (`active_replacements`) **plus** the floating store `state.pending_damage_replacements` (replacement.rs:4838-4862, sentinel `ObjectId(0)`), and rejects when a life-class def is *optional*, carries a **body continuation** (`execute.is_some() || runtime_execute.is_some()`), or when ≥2 life-class defs watch the same proposed-event class. Fail-closed over-approximation of `find_applicable_replacements` (conditions/filters deliberately ignored).
- Everything else — all ~208 other `Effect` variants, `Spell`/`ActivatedAbility`/`KeywordAction` entries, and every choice-bearing ability-level structure (`optional`, `unless_pay`, `target_chooser`, `target_choice_timing == Resolution`, `modal`, `mode_abilities`, `repeat_until: ControllerChoice`) — is `MayPrompt` ⇒ cover rejected.
- Guard test at the 9092a8961 standard pins the classifier output (allow set pinned as exactly `{GainLife, LoseLife}`, with an allow-arm census, §6); four new N1 hostiles (n1_o/q/r/s) with executed revert-fail mutations; all existing N-series tests stay green; `loop_detection` default-OFF byte-behavior untouched (the gate lives inside the already-gated cover fn — engine.rs:323, :671; sole production caller analysis/loop_check.rs:288).

Hard constraint restated for the implementer: **the #4904 branch must contain ZERO `.claude/skills/` changes** (§9). The SKILL.md checklist edit is a separate deliverable for the #4905 branch (§11).

---

## 1. The finding and its CR basis

### 1.1 Verified: the finding is real

- Gate under attack: `stack_entry_has_no_ordering_input` (resource.rs:860-871) checks only: entry is a `TriggeredAbility`, not mid-construction (`pending_trigger_entry`), `targets.is_empty()`, `multi_target.is_none()`, `distribution.is_none()`, `target_constraints.is_empty()`. Sole caller: item (3) grown-entry loop, resource.rs:722-728.
- None of those conjuncts prove the *resolver* cannot enter a non-priority `WaitingFor` later. Measured raise sites for the finding's named kinds:
  - `WaitingFor::ProliferateChoice` — effects/proliferate.rs:109 (raise inside `drive_single_proliferate_action`); auto-resolves **only** when `eligible.is_empty()` (proliferate.rs:90) — eligibility includes **player counters** (`proliferatable_player_counters`, proliferate.rs:27, consulted in `collect_proliferate_eligible` at :50/:65), a projected axis.
  - `WaitingFor::PopulateChoice` — effects/populate.rs:50.
  - `WaitingFor::ClashChooseOpponent` — effects/clash.rs:47.
  - `WaitingFor::ExploreChoice` — effects/explore.rs:191.
  - `WaitingFor::EffectZoneChoice` (sacrifice picks) — effects/sacrifice.rs:306.
- The AST read-walk cannot see these: they are **resolver** behavior. `Effect::Proliferate` is a unit variant (types/ability.rs:8835) — nothing in its AST reads a projected axis, so item (4) passes it (consistent with the maintainer's statement about the scanner).

### 1.2 CR basis — verified numbers (full grep evidence in §7)

- **CR 732.2a** is the correct rule for this fix: a shortcut must have "predictable results of the sequence of choices … It can't include conditional actions, where the outcome of a game event determines the next action a player takes." A resolution-time prompt whose option set depends on evolving (projected) state is exactly such a conditional action. This is also the codebase's canonical detector cite (analysis/loop_check.rs:38, resource.rs module docs).
- **CR 732.5** (the reviewer's cite) exists but says "No player can be forced to perform an action that would end a loop other than actions called for by objects involved in the loop" — the forced-loop-ending rule, not the choice-freeness rule. **PR-reply note:** politely agree with the substance and cite CR 732.2a as the annotation basis; the code annotations in this fix use 732.2a (+ CR 608.2d for resolution-time choices, CR 616.1 for the replacement-ordering choice). The reply must describe THIS fix only — it must NOT mention any future detector feature or roadmap (§9.6).
- Pre-existing misannotation found (do NOT fix in this PR — different file, out of scope; note for a follow-up): life.rs:158, :255 comment "CR 614.7: Multiple competing replacements — player must choose", but CR 614.7 is "…that event never happens, the replacement effect simply doesn't do anything." The correct rule is **CR 616.1** (affected player chooses among multiple replacement effects). The *new* code this plan adds cites CR 616.1.

---

## 2. Verified code map (all read this session)

| Anchor | What it is |
|---|---|
| resource.rs:695-747 | `loop_states_cover_modulo_growth` — items (1)-(5); doc list at :688-694 |
| resource.rs:722-728 | item (3): grown places gated by `stack_entry_has_no_ordering_input` |
| resource.rs:733-739 | item (4): `stack_entry_reads_projected_resource` over **every** current-stack entry |
| resource.rs:741-744, 948-1044 | item (5): `fire_time_conditions_read_projected_resource` (loops i-iv) |
| resource.rs:860-871 | `stack_entry_has_no_ordering_input` (sole caller = item 3) |
| resource.rs:882-909 | `stack_entry_reads_projected_resource` — the per-entry pattern the new gate mirrors |
| resource.rs:1928-2405 | N1 test module: `churn_entry` (:1953-1974), `gain_ability` (:1977-1988), `cover_base` (:1996-2008), `bf_object` (:2010-2022), tests n1_p1/p2 + n1_a…n1_n, n1_kg/kr/ks |
| ability_scan.rs:1-56 | walker header: three axes, no-wildcard rationale, traversal-closure rule (R4-G2) |
| ability_scan.rs:82-104 | `Axes` (NONE / CONSERVATIVE / or) |
| ability_scan.rs:115-225 | `resolved_ability_axes` — the exhaustive no-`..` `ResolvedAbility` destructure to mirror |
| ability_scan.rs:3100-3147 | pub(crate) wrapper fns (naming precedent); test module at :3149 |
| types/game_state.rs:2904 | `WaitingFor` enum (110 variants; `Priority` at :2905) |
| types/ability.rs:8420-8437 | `Effect::GainLife { amount, player }`, `Effect::LoseLife { amount, target }` |
| types/ability.rs:13814-13823 | `TargetChoiceTiming { Stack, Resolution }` (CR 601.2c + CR 603.3d + CR 608.2d doc) |
| types/ability.rs:16818-16819 | `ReplacementDefinition { pub event: ReplacementEvent, … }`; `mode` at :16828 |
| types/replacements.rs:27,29 | `ReplacementEvent::LoseLife`, `ReplacementEvent::GainLife` |
| effects/mod.rs:2914-2926 | `resolve_effect` — canonical exhaustive dispatch (the resolver surface the classifier is pinned to) |
| effects/mod.rs:4294, 4353 | `WaitingFor::OptionalEffectChoice` raise (the `optional: true` resolution prompt) |
| effects/life.rs:19-110 | `resolve_gain` — its OWN inline `replace_event` pipeline (does NOT call `apply_life_gain`); `NeedsChoice` raise :96-101 (`waiting_for` at :98); Execute arm does NOT drain the substitution continuation — only Prevented does (~:94); Execute-arm continuations drain at the stack.rs post-resolution point |
| effects/life.rs:293-365 | `resolve_lose` — same shape; `NeedsChoice` raise :352-355 (`waiting_for` at :353) |
| effects/life.rs (grep `waiting_for`) | exactly FOUR raises — :98 (`resolve_gain`), :157 (`apply_life_gain`), :251 (`apply_damage_life_loss`), :353 (`resolve_lose`) — ALL are `replacement_choice_waiting_for` NeedsChoice raises |
| effects/life.rs:116-163, :220-258 | `apply_life_gain` / `apply_damage_life_loss` — the SIBLING pipelines used by other callers (`resolve_set_life_total` :404+, damage conversion); same single NeedsChoice prompt shape (:155-160, :251-257) |
| effects/life.rs:174-184 | `drain_substitution_continuation` → `apply_pending_post_replacement_effect` |
| replacement.rs:2303-2308 / :3467-3471 | `gain_life_matcher` (matches `ProposedEvent::LifeGain`) / registered under `ReplacementEvent::GainLife` |
| replacement.rs:2453-2455 / :3474-3478 | `life_reduced_matcher` (matches `ProposedEvent::LifeLoss`) / `ReplacementEvent::LifeReduced` — serde-reachable "LifeReduced" (types/replacements.rs:182) |
| replacement.rs:2468-2479 / :3481-3485 | `lose_life_matcher` (matches `ProposedEvent::LifeLoss`; state-dependent Bloodletter logic) / `ReplacementEvent::LoseLife` |
| replacement.rs:3324-3326 / :3553-3558 | `pay_life_matcher` (**matches `ProposedEvent::LifeLoss`**) / `ReplacementEvent::PayLife` — serde-reachable "PayLife" (types/replacements.rs:181) |
| replacement.rs:4838-4862 | `find_applicable_replacements` ALSO scans the floating store `state.pending_damage_replacements` (game_state.rs:7184; sentinel `ObjectId(0)`; `is_consumed` skip :4859-4861) — `active_replacements` alone is NOT the full candidacy authority |
| replacement.rs:5511-5524 | mandatory-Execute stash: `runtime_execute` ⇒ `PostReplacementContinuation::Resolved(runtime)` (prompt-capable body continuation) |
| engine_replacement.rs:1159 | `apply_pending_post_replacement_effect` — drains the continuation via `resolve_ability_chain` and returns/sets a non-priority `WaitingFor` when the body prompts (:1150-1156 helper) |
| replacement.rs:6221-6247 | single **optional** candidate ⇒ `NeedsChoice` prompt |
| replacement.rs:6263-6279 | ≥2 candidates with material ordering ⇒ `NeedsChoice` (CR 616.1); degenerate orderings auto-resolve (:6281-6292) |
| replacement.rs:638 | `pub(crate) fn replacement_mode_is_optional` |
| resource.rs:1058-1060 | item 5's `replacement_body_may_read_projected` rejects `execute.is_some()` (incidental; the guard re-checks it) |
| resource.rs:976-981 | item 5 scans `runtime_execute` ONLY for projected reads — a non-reading prompt-capable body passes item 5 (hole H4 route c) |
| types/ability.rs:17031-17034 | `runtime_execute` builder (public field + builder — hand-built fixtures and runtime shields can carry it) |
| functioning_abilities.rs:322-334 | `active_replacements(state) -> impl Iterator<Item = (usize, &GameObject, &ReplacementDefinition)>` — item 5's replacement-liveness authority |
| triggers.rs:4195-4232 | `TriggerDispatchDisposition`: `Paused` = mid-construction modal/target prompt (firewalled by `pending_trigger_entry`); **`Pushed` can carry unresolved resolution-time filter slots** (Good King Mog note, :4222-4230) — confirms the resource.rs:855-859 modal claim AND exposes hole H3 |
| triggers.rs:22566-22614 | `granted_keyword_trigger_conditions_projected_reads_are_exactly_known_gaps` — the guard-test standard to mirror (representative enumeration × classifier, pinned set, non-vacuity assert, executed revert-fail documented in 9092a8961) |
| analysis/loop_check.rs:285-289 | sole production caller of the cover fn (inside `live_mandatory_loop_winner`) |
| engine.rs:323, :671 | `state.loop_detection.is_on()` gates ring maintenance + reconcile scan (default-OFF firewall) |
| tests/pr65_growing_cascade_win.rs | N3 ON/OFF integration fixture (drain cleric "each opponent loses 1 life" + sipper "you gain 1 life") |
| game/quantity.rs, game/filter.rs | **zero** production `WaitingFor` raises (quantity.rs hits are `#[cfg(test)]`-only, :11644+; filter.rs zero) — grounds "payload evaluation is pure" for the allow-list |

Precedent commits traced end-to-end: `9092a8961` (guard-test standard: compiler-exhaustive representative fn + pinned classifier output + executed revert-fail) and `bceec86e3` (fix shape: single synthesis authority + fail-closed scan, loop (iv)).

---

## 3. Root cause and hole taxonomy

Why the pre-growth equality path was safe: it only certified loops whose stack entries were *observed* resolving without prompting inside the sampled window (a non-priority `WaitingFor` is a non-sampling beat that clears the ring — analysis/loop_check.rs module docs). Cover-modulo-growth breaks that: it extrapolates resolutions that were **never observed**. Choice-freeness is state-dependent, so "same normalized kind as an observed entry" proves nothing. Four concrete holes, each of which the fix must close:

- **H1 — grown, never-observed kinds.** A kind can grow (`pn≥1, cn>pn`) without any instance resolving in-window (growth caused by *another* kind's resolution; deep entries frozen under LIFO, CR 608.1 / CR 405.5). Its first modeled resolution may prompt (e.g. grown `Effect::Proliferate` once any player counter exists). This is the maintainer's stated hole.
- **H2 — non-grown kinds with projected option-surfaces.** The window can resolve proliferate *before* the beat that grants the first poison counter: the observed proliferate auto-resolved (`eligible.is_empty()`, proliferate.rs:90), the covering pair still matches (player counters are projected), and every *future* proliferate resolution prompts. So the gate must cover **all** current-stack entries, not only grown ones — item (4) already has exactly this all-entries scope for AST reads (resource.rs:733-739); the choice gate is its resolver-level analogue. This goes beyond the maintainer's literal ask; delivering grown-only would knowingly leave an adjacent hole of the same class.
- **H3 — resolution-timing target slots.** An entry can legally sit on the stack with `targets` empty and choices deferred to resolution (`TargetChoiceTiming::Resolution`, ability.rs:13814-13823; non-CR-115.1d filter slots reach the stack per CR 603.3 — triggers.rs:4222-4230). Today's `targets.is_empty()` conjunct is *satisfied* by exactly these entries. The classifier must reject `target_choice_timing == Resolution`, `target_chooser.is_some()`, and non-empty `mode_abilities` (reflexive modal, resolution-time `WaitingFor::ModalChoice`-class prompts — the Grist pattern, triggers.rs:22630+ test helpers).
- **H4 — replacement-pipeline prompts on the allow-listed kinds themselves.** Even `GainLife`/`LoseLife` resolution can prompt, via THREE routes:
  - **(a) single optional candidate** ⇒ `NeedsChoice` (replacement.rs:6221-6247), raised by `resolve_gain`/`resolve_lose`'s inline pipelines (life.rs:96-101, :352-355).
  - **(b) ≥2 candidates with material ordering** ⇒ `NeedsChoice` (CR 616.1, replacement.rs:6263-6279).
  - **(c) single MANDATORY def with a body continuation:** on Execute, `runtime_execute` is stashed as `PostReplacementContinuation::Resolved` (replacement.rs:5511-5524; `execute` bodies stash similarly below) and later drained through `apply_pending_post_replacement_effect` (engine_replacement.rs:1159), which runs an arbitrary `ResolvedAbility` via `resolve_ability_chain` and can set a non-priority `waiting_for` (e.g. a `Sacrifice` body ⇒ `EffectZoneChoice`, sacrifice.rs:306). Item (5) rejects `execute.is_some()` incidentally (resource.rs:1058-1060) but scans `runtime_execute` **only for projected reads** (resource.rs:976-981) — a non-reading prompt-capable `runtime_execute` passes items (1)-(5).

  The matching def set is WIDER than `{GainLife, LoseLife}`: `pay_life_matcher` (replacement.rs:3324-3326, `ReplacementEvent::PayLife`) and `life_reduced_matcher` (:2453-2455, `ReplacementEvent::LifeReduced`) **also** match `ProposedEvent::LifeLoss`. The replacement *set* is board-fixed (gate 1) and item (5) already rejects projected-reading replacement conditions/bodies fail-closed (resource.rs:963-986), but a condition-free quantity-mod life replacement (Rhox Faithmender class ×2, one optional one, or one mandatory one with a body continuation) survives items (1)-(5) and prompts on the first *unobserved* life resolution. All three routes are closed by the environmental guard (§5.3(b)).

Modal firewall claim re-verified (driver request): a modal trigger pauses mid-construction (`TriggerDispatchDisposition::Paused`, triggers.rs:4204-4210) with `pending_trigger_entry` set — item 3's firewall (resource.rs:864-866) covers that window; the chosen mode is baked by rewriting the entry's ability (triggers.rs:4180-4190). The classifier still rejects `modal.is_some()` conservatively (fail-closed; no shipped fixture carries modal — over-rejection only).

---

## 4. Design

### 4.1 Chosen shape: hybrid (a)+(b), converged as the driver predicted

Structure of (a) — a classifier over the embedded ability reusing the walker's closure discipline — with the allow-list realism of (b): only 2 of ~210 `Effect` variants are claimed choice-free, each with a resolver trace. Grouped `| … => MayPrompt` arms need **no** per-kind evidence (an ungrounded reject is only a false negative); each allow-list arm needs full grounding (an ungrounded allow is a soundness bug).

### 4.2 Why NOT a 4th axis on `Axes`

`Axes::NONE` means "no *reads* on the three axes" — orthogonal to prompting. `Effect::Scry` reads nothing projected yet always prompts. Folding a `choice` bool into `Axes` would make every existing `NONE` arm silently claim choice-free — ~200 latent soundness bugs — and would require re-auditing every arm of `scan_effect` in one diff. A separate function family keeps the fail-closed default explicit and the audit surface tiny. The ability_scan.rs header (:1-56, "three independent classification questions") gets a short note that the choice classifier is a *separate question family about resolver behavior*, deliberately not an `Axes` axis, for exactly this reason.

### 4.3 The verdict type — typed, 2-variant, ability_scan-local

```rust
/// CR 732.2a + CR 608.2d: resolution-time choice-freeness verdict for the
/// growing-cascade cover gate (analysis::resource item 6). NOT an `Axes` axis:
/// this classifies RESOLVER prompting behavior, not AST reads (see §4.2 note).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ResolutionChoiceFreedom {
    /// Resolving can never enter a non-priority `WaitingFor` in ANY state,
    /// EXCEPT through the life-event replacement pipeline (single optional
    /// candidate, replacement.rs:6221; CR 616.1 material ordering,
    /// replacement.rs:6263; mandatory body-continuation drain,
    /// replacement.rs:5511-5524 → engine_replacement.rs:1159). Callers MUST
    /// pair this verdict with `life_event_replacements_may_prompt`
    /// (resource.rs) — the paired obligation is part of this variant's
    /// contract.
    FreeUnlessLifeReplacements,
    /// May prompt, or unproven — the fail-closed default.
    MayPrompt,
}
```

Why no plain `Free` variant yet: both allow-listed kinds genuinely can prompt via the life-replacement pipeline, so `Free` would be uninhabited today. Adding it later is compiler-guided (new variant ⇒ every exhaustive match flags). Documented on the enum. Why not a `bool`: repo rule (CLAUDE.md "never a raw bool" for design-space distinctions) — the truth here is three-valued-in-principle, two-valued-today, and the paired environmental obligation must be type-visible, not comment-only.

Not a new *engine AST* variant: `ResolutionChoiceFreedom` is a new analysis-internal type, `pub(crate)`, never serialized, never on a wire/state surface. add-engine-variant lens run anyway: Stage 1 existence — no existing verdict enum; nearest prior art is `cost_component_choice_free` (mana_abilities.rs:2553), a bool over `AbilityCost` components for mana-ability activation — different layer (activation cost payment, not resolution prompting), not reusable. Stage 2/3 — single-axis 2-variant type, single CR concept (CR 608.2d resolution choices). `cargo engine-inventory` consulted for "choice"-named surfaces (none conflicting).

### 4.4 Gate placement and scope

New item **(6)** in `loop_states_cover_modulo_growth`, after item (5) (resource.rs:741-744), covering **every** current-stack entry (H2). Items (1)-(5) unchanged; item (6) is a strictly additive conjunct, so every existing hostile still rejects for its original reason. Default-OFF byte-exactness: the cover fn's sole production caller is analysis/loop_check.rs:288 inside `live_mandatory_loop_winner`; both the ring maintenance and the reconcile scan are gated on `state.loop_detection.is_on()` (engine.rs:323, :671). N3's OFF arm pins this.

Deliberate over-rejections (all fail-safe, documented in code):
- Non-`TriggeredAbility` entries (`Spell`/`ActivatedAbility`/`KeywordAction`) ⇒ `MayPrompt`, even bottom-frozen ones that the extrapolation never resolves. Ceiling + upgrade path noted in the fn doc (model frozen positions only if a real fixture needs it).
- `modal.is_some()` ⇒ `MayPrompt` even when the mode is already baked (§3 end).
- `unless_pay` ⇒ `MayPrompt` (belt-and-braces: item 4 already rejects it — ability_scan.rs:200-204 sets `projected`).

### 4.5 Structural requirements (binding — implementer and impl-reviewer enforce these)

Three requirements so a future consumer can add inputs at the gate without rework. **No speculative code, fields, or enum variants beyond these three items** — boolean-granularity `MayPrompt` classification is deliberately sufficient today:

1. **Classifiers are pure fact-producers.** `effect_resolution_choice_freedom`, `ability_resolution_choice_freedom`, `stack_entry_resolution_choice_freedom`, and `life_event_replacements_may_prompt` answer factual questions ("can this resolution prompt?", "can the life-replacement environment prompt?") and return verdicts/bools. None of them decides a cover outcome, returns early out of the cover fn, or takes rejection-policy parameters. (Current design already satisfies this — keep it that way under review pressure.)
2. **One gate seam.** The reject-on-`MayPrompt` decision (including the `FreeUnlessLifeReplacements` ⇒ environmental-guard pairing) lives at exactly ONE site: the item (6) block in `loop_states_cover_modulo_growth` (§5.3(c)). Do not spread verdict-based rejection into item (3)'s loop, the classifiers, or the guard. Item (3) remains the *ordering-input* gate (a different fact) and is not modified.
3. **Extension-point doc comment at the seam.** The item (6) block carries the EXTENSION POINT comment (§5.3(c) verbatim draft) recording the future soundness obligations: (a) pinned choices must be state-independent designations legal at every iteration of the growing state; (b) cover-modulo-growth must hold under the pinned outcomes; (c) only the acting player's own choices are pinnable — opponent-choice entries stay rejectors unless every option preserves the certificate (CR 104.2a-grounded winner predicate). CR 104.2a grep-verified (§7).

---

## 5. Per-file change list

### 5.1 `crates/engine/src/game/ability_scan.rs` — new classifier section (after the pub(crate) wrappers, ~:3147)

**(a) `ResolutionChoiceFreedom`** as in §4.3, plus a two-line `fn join(self, other: Self) -> Self` (worst-of; `MayPrompt` dominates).

**(b) `pub(crate) fn ability_resolution_choice_freedom(a: &ResolvedAbility) -> ResolutionChoiceFreedom`** — exhaustive destructure **without `..`**, mirroring `resolved_ability_axes` (ability_scan.rs:116-162) so a future `ResolvedAbility` field fails to compile until classified for the choice question. Field verdict table (every field of the :116-162 destructure re-decided for *this* question — the read-walk's classifications must NOT be copied; e.g. `optional` is read-free but choice-bearing):

| Field(s) | Contribution | Grounding |
|---|---|---|
| `effect` | `effect_resolution_choice_freedom(effect)` | §5.2 |
| `sub_ability`, `else_ability` | recurse, `join` | chain resolution runs both branches' effects (effects/mod.rs `resolve_ability_chain` contract, :2917-2921 doc) |
| `condition` | none | branch selector, pure eval; both branches recursed anyway |
| `optional`, `optional_for`, `optional_targeting` | `true`/`Some` ⇒ `MayPrompt` | `WaitingFor::OptionalEffectChoice` raise, effects/mod.rs:4294 (CR 608.2d) |
| `unless_pay` | `Some` ⇒ `MayPrompt` | resolution-time pay prompt; also item-4 redundant (ability_scan.rs:200-204) |
| `target_chooser` | `Some` ⇒ `MayPrompt` | resolution-time chooser (H3) |
| `target_choice_timing` | `Resolution` ⇒ `MayPrompt` | ability.rs:13814-13823 (CR 608.2d); H3 |
| `modal` | `Some` ⇒ `MayPrompt` | conservative; §3 end |
| `mode_abilities` | non-empty ⇒ `MayPrompt` | reflexive modal resolution prompt (triggers.rs Grist helpers :22630+) |
| `repeat_until` | `ControllerChoice` ⇒ `MayPrompt`; `WhileCondition`/`UntilStopConditions` ⇒ none | mirror `scan_repeat_continuation` structure (ability_scan.rs:230-242); only the controller-prompted variant is a player choice |
| `player_scope`, `starting_with`, `repeat_for`, `duration`, `multi_target`, `target_constraints`, `distribution` | none (bind, one-line justification each) | iteration/count/duration eval is pure (quantity.rs / filter.rs: zero production `WaitingFor`); `distribution` is concrete pre-assigned portions (CR 601.2d announce-time division; ability_scan.rs:152 comment); `multi_target` is announce-time — the `Resolution`-timing case is caught by the timing row |
| `targets`, ids, snapshots, flags, provenance (`source_id`…`dig_found_nothing_for_parent_target`, same set as :135-161) | none (bind `_` with the same one-line justifications) | concrete data, no prompt |

**(c) `fn effect_resolution_choice_freedom(e: &Effect) -> ResolutionChoiceFreedom`** — one `match`, **no `_ =>` wildcard**, no `match-ergonomics` escape:

- Allow-list arms, destructured **without `..`** (so a new field on these variants fails to compile until re-audited — mirrors the #4905 checklist's NONE-arm rule):
  ```rust
  // CR 119.3 + CR 732.2a: resolver trace effects/life.rs — resolve_gain
  // (life.rs:19-110) runs its OWN inline replace_event pipeline; its only
  // prompt is ReplacementResult::NeedsChoice (life.rs:96-101). Player
  // selection = pure filter eval (game/filter.rs: no WaitingFor); amount =
  // pure quantity eval (game/quantity.rs: no WaitingFor). Verdict is
  // payload-independent. PAIRED OBLIGATION: caller runs
  // life_event_replacements_may_prompt (resource.rs item 6), which also
  // covers the mandatory body-continuation drain (H4 route c).
  Effect::GainLife { amount: _, player: _ } => ResolutionChoiceFreedom::FreeUnlessLifeReplacements,
  // Same shape: resolve_lose (life.rs:293-365), only prompt = NeedsChoice
  // (life.rs:352-355).
  Effect::LoseLife { amount: _, target: _ } => ResolutionChoiceFreedom::FreeUnlessLifeReplacements,
  ```
  Implementer MUST re-read `resolve_gain`/`resolve_lose` end-to-end before writing these arms and confirm the prompt inventory: `grep -n waiting_for crates/engine/src/game/effects/life.rs` returns exactly FOUR raises — :98 (`resolve_gain`), :157 (`apply_life_gain`), :251 (`apply_damage_life_loss`), :353 (`resolve_lose`) — all `replacement_choice_waiting_for` NeedsChoice raises (plus the SetLifeTotal deferral *comment* at :475, whose raises are the `apply_*` ones). Any additional raise found ⇒ that kind drops to `MayPrompt` and STOP/report. Note the Execute-arm asymmetry: `resolve_gain`'s Execute arm does NOT drain the substitution continuation (only Prevented does, ~life.rs:94, unlike `apply_life_gain`:146) — Execute-arm continuations drain at the stack.rs post-resolution point, which is prompt-capable and therefore covered by guard clause (b), not by this trace.
- Everything else: grouped `Effect::Foo { .. } | Effect::Bar { .. } | … => ResolutionChoiceFreedom::MayPrompt` arms (category-commented for readability; compiler exhaustiveness is preserved since every variant is named). No payload scanning needed on the reject side.

**(d) Module-header note** (append to ability_scan.rs:1-56 docs): the choice classifier is a separate question family (§4.2 rationale), its fail-closed default, and the resolver-trace evidence bar for any future allow-list promotion.

**(e) Guard test** in the existing `#[cfg(test)]` module (:3149+), `resolution_choice_verdicts_are_exactly_pinned`, at the 9092a8961 standard:
- Pins allow-list verdicts: `GainLife`/`LoseLife` (representative payloads) ⇒ `FreeUnlessLifeReplacements`.
- Pins reject verdicts for the finding's kinds + adjacent siblings, each with its raise-site citation in a comment: `Proliferate` (proliferate.rs:109), `Populate` (populate.rs:50), `Clash` (clash.rs:47), `Explore` (explore.rs:191), `Scry`, `Sacrifice` (sacrifice.rs:306), `DiscardCard` (costs.rs:605 class) ⇒ `MayPrompt`.
- States the pinned allow set **explicitly** in the test doc + an assertion message: "the `FreeUnlessLifeReplacements` set is exactly `{Effect::GainLife, Effect::LoseLife}`" — paired with the §6 allow-arm census so a future third allow arm cannot land without turning this pin + the census stale (M5 closure: hand-picked-representative pinning alone would let a new allow arm through silently).
- Pins ability-level wrappers: take `gain_ability(1)`-style base (⇒ `FreeUnlessLifeReplacements`), then each single-field mutation (`optional = true`; `unless_pay = Some(..)`; `target_chooser = Some(..)`; `target_choice_timing = Resolution`; non-empty `mode_abilities`; `repeat_until = Some(ControllerChoice)`; `modal = Some(..)`) ⇒ `MayPrompt`. The unmutated base assert is the paired positive reach-guard proving the wrapper flip (not something upstream) causes each `MayPrompt`.
- Non-vacuity assert mirroring 9092a8961: at least one pinned kind on each side.
- **Executed revert-fail (implementer must run and document in the commit message):** temporarily classify `Effect::Scry { .. }` ⇒ `FreeUnlessLifeReplacements` → test goes RED; restore.
- Compiler-exhaustiveness leg: the `match` itself (a new `Effect` variant fails to compile in (c)) — same structural mechanism as `keyword_synthesizes_granted_trigger` in 9092a8961.

### 5.2 Allow-list grounding summary (the evidence the review gate demands, per kind)

| Kind | Resolver entry (effects/mod.rs dispatch :2944-2945) | Full prompt inventory on all paths (REAL call graph) | Residual & closure |
|---|---|---|---|
| `Effect::GainLife` | `life::resolve_gain` (life.rs:19-110) — its OWN inline `replace_event` pipeline; it does **not** call `apply_life_gain` | (1) `NeedsChoice` ⇒ `waiting_for = replacement_choice_waiting_for(..)` (life.rs:96-101). (2) Prevented arm drains `drain_substitution_continuation` (~:94 → :174-184 → `apply_pending_post_replacement_effect`, engine_replacement.rs:1159 — prompt-capable). (3) Execute arm does NOT drain inline; its stashed continuation (replacement.rs:5511-5524) drains at the stack.rs post-resolution point — prompt-capable | ALL prompt routes go through life-class replacement defs (NeedsChoice needs an optional/materially-ordered candidate; continuations need a def with `execute`/`runtime_execute`) ⇒ closed by §5.3(b) guard clauses (a)/(b)/(c) |
| `Effect::LoseLife` | `life::resolve_lose` (life.rs:293-365) — same inline-pipeline shape | `NeedsChoice` (life.rs:352-355); Prevented-arm drain; Execute-arm stack.rs drain. CR 119.7/119.8 can't-gain/can't-lose short-circuits are deterministic | same; NOTE the matching def set for `ProposedEvent::LifeLoss` spans THREE `ReplacementEvent` keys (`LoseLife`, `LifeReduced`, `PayLife` — §5.3(b)) |

Sibling pipelines `apply_life_gain` (life.rs:116-163, `NeedsChoice` :155-160) / `apply_damage_life_loss` (:220-258, :251-257) are used by OTHER callers (`resolve_set_life_total` life.rs:404+, damage-to-life-loss conversion) — not by `resolve_gain`/`resolve_lose`; they have the same single NeedsChoice prompt shape, corroborating the class-level claim. Total prompt inventory for life.rs: exactly four `waiting_for` raises (:98, :157, :251, :353), all NeedsChoice.

Payload purity: `QuantityExpr` eval via game/quantity.rs and `TargetFilter` eval via game/filter.rs contain zero production `WaitingFor` raises (measured; quantity.rs hits are in `#[cfg(test)]` at :11644+).

**Mandatory pre-flight probe (keeps N2/N3 green):** confirm the N3 oracle texts parse to allow-listed kinds — "each opponent loses 1 life" ⇒ `Effect::LoseLife`, "you gain 1 life" ⇒ `Effect::GainLife` — by running the N2/N3 tests after wiring (`tilt` test-engine resource) or a one-off parse probe. If either parses to a different variant (e.g. a scoped sibling), that variant joins the allow-list **only** with the same resolver-trace grounding; otherwise stop and report.

### 5.3 `crates/engine/src/analysis/resource.rs`

**(a) `fn stack_entry_resolution_choice_freedom(entry: &StackEntry) -> ResolutionChoiceFreedom`** (private, next to `stack_entry_reads_projected_resource` :882-909): exhaustive match over all four `StackEntryKind`s (named; no wildcard): `TriggeredAbility { ability, .. }` ⇒ `ability_resolution_choice_freedom(ability)`; `Spell { .. }` / `ActivatedAbility { .. }` / `KeywordAction { .. }` ⇒ `MayPrompt` with the frozen-bottom ceiling comment (§4.4). Trigger-level `condition` (intervening-if re-check, CR 603.4) is pure evaluation — no contribution; noted in the doc.

**(b) `fn life_event_replacements_may_prompt(state: &GameState) -> bool`** (private, next to `fire_time_conditions_read_projected_resource`), plus a small helper `fn replacement_event_matches_life(event: &ReplacementEvent) -> Option<LifeEventClass>`:

- **Life-class set is REGISTRY-DERIVED, not hand-picked.** Coupling rule (goes in the comment verbatim): *life-class ⇔ the `ReplacementEvent`'s registry matcher matches `ProposedEvent::LifeGain` or `ProposedEvent::LifeLoss`.* Measured derivation (every matcher in replacement.rs matching a life `ProposedEvent`):
  - `gain_life_matcher` (:2303-2308) → `ReplacementEvent::GainLife` (:3467-3471) — matches `LifeGain`.
  - `lose_life_matcher` (:2468-2479) → `ReplacementEvent::LoseLife` (:3481-3485) — matches `LifeLoss` (state-dependent Bloodletter logic; irrelevant to the guard, which over-approximates by event key).
  - `life_reduced_matcher` (:2453-2455) → `ReplacementEvent::LifeReduced` (:3474-3478) — matches `LifeLoss`.
  - `pay_life_matcher` (:3324-3326) → `ReplacementEvent::PayLife` (:3553-3558) — matches `LifeLoss`.
  So life-class = `{GainLife (→LifeGain), LoseLife, LifeReduced, PayLife (→LifeLoss)}`. The helper is a **compiler-exhaustive no-wildcard match over ALL `ReplacementEvent` variants** (non-life variants explicitly listed ⇒ `None`), so a NEW `ReplacementEvent` variant fails to compile until classified against the coupling rule. Note the classification criterion is the *matcher*, not the variant name — a hand-picked set had already missed `PayLife` and `LifeReduced`; the arm-writing rule is "grep the matcher" (`rg -n 'ProposedEvent::Life(Gain|Loss)' crates/engine/src/game/replacement.rs` over matcher fns), re-run whenever the set is edited.
- **Def sources scanned (candidacy authorities):** `active_replacements(state)` (functioning_abilities.rs:322 — object-attached defs, item 5's authority) **chained with** `state.pending_damage_replacements.iter()` (the game-state-level floating store, game_state.rs:7184, scanned by `find_applicable_replacements` at replacement.rs:4838-4862 with sentinel `ObjectId(0)`). Skip `is_consumed` defs in the floating store (mirrors :4859-4861). `pending_step_end_mana_handlers` (game_state.rs:7193) is a different type (`StepEndManaScanEntry`) that IS scanned by `find_applicable_replacements`, but only under the `ProposedEvent::EmptyManaPool` event gate (replacement.rs:4977) — structurally unable to produce a life-class candidate, hence excluded from the guard's candidacy authority; document exactly this in the comment. [Wording corrected per plan-review R2 MINOR 1 — the prior "not scanned by `find_applicable_replacements`" claim was false; driver re-verified the EmptyManaPool gate at replacement.rs:4971-4980.]
- **Reject clauses**, per life-class def (annotate: CR 616.1 for (c); raise/stash anchors for (a)/(b)):
  - (a) `replacement_mode_is_optional(&def.mode)` (replacement.rs:638) ⇒ `true` — single optional candidate prompts (replacement.rs:6221-6247).
  - (b) `def.execute.is_some() || def.runtime_execute.is_some()` ⇒ `true` — a MANDATORY body continuation is stashed on Execute (`PostReplacementContinuation::Resolved`, replacement.rs:5511-5524) and drained via `apply_pending_post_replacement_effect` (engine_replacement.rs:1159), which runs an arbitrary `ResolvedAbility` and can set a non-priority `waiting_for`. `execute.is_some()` is ALSO rejected by item 5 (`replacement_body_may_read_projected`, resource.rs:1058-1060) — re-checked here deliberately so the guard does not depend on item ordering or on item 5's incidental behavior; `runtime_execute` is NOT otherwise covered (item 5 scans it only for projected reads, resource.rs:976-981).
  - (c) count per **proposed-event class** (LifeGain-matchers = `{GainLife}`; LifeLoss-matchers = `{LoseLife, LifeReduced, PayLife}`): either class count ≥2 ⇒ `true` — material-ordering prompt (CR 616.1, replacement.rs:6263-6279). Counting per proposed class (not per `ReplacementEvent` key) is required: one `LifeLoss` event draws candidates from all three LifeLoss-matching keys.
- Conditions, `valid_player` scopes, and amounts are deliberately ignored — over-count ⇒ over-reject ⇒ fail-safe. Precision note (real-world false-negative avoided): a single mandatory quantity-mod def with no body (Bloodletter of Aclazotz `lose_life_applier` doubling, replacement.rs:2481-2499; Rhox Faithmender class) does NOT trip any clause — candidates.len()==1 + mandatory + no continuation resolves deterministically (replacement.rs:6250-6261), so a drain cover with one such doubler on board still certifies.
- **Implementer verification sweep:** complete the `find_applicable_replacements` (replacement.rs:4315+) sweep for *virtual* life-event candidates — measured this session: virtual candidates are shield-counter Destroy/Damage, Compleated AddCounter, BeginPhase, and an Abundance-class Draw gate; none for `LifeGain`/`LifeLoss` — re-confirm over the whole function (`rg -n 'ProposedEvent::Life' crates/engine/src/game/replacement.rs` within its body); if any virtual life candidate exists, extend the guard.

**(c) Item (6) wiring** in `loop_states_cover_modulo_growth`, after item (5) (:741-744). This block is **THE single gate seam** for resolution-choice rejection (structural requirement, §4.5): the classifiers produce facts only; every reject-on-verdict decision lives here and nowhere else (item (3) is untouched and gates a different fact — announcement-time ordering input).

```rust
// (6) CR 732.2a + CR 608.2d: resolution-time choice gate, fail-closed, over
// EVERY current-stack entry — the extrapolation models future resolutions the
// window never observed (grown kinds) and re-runs observed kinds in states that
// differ on projected axes, where a resolver's choice surface (e.g. proliferate
// eligibility over player counters, CR 701.34a) can open a prompt that the
// AST-level item-4 scan cannot see. Verdicts from the ability_scan classifier
// (pure fact-producers — rejection is decided ONLY here);
// FreeUnlessLifeReplacements additionally requires the CR 616.1 environmental
// guard below.
//
// EXTENSION POINT — pinned fixed choices (CR 732.2a): a shortcut proposal MAY
// pre-specify choices in advance ("always choose permanent P"); only
// CONDITIONAL actions are forbidden. A future consumer may treat a MayPrompt
// entry as choice-free when a pin covers it, PROVIDED: (a) the pin is a
// STATE-INDEPENDENT designation whose option remains legal at every iteration
// of the growing state (never "the newest copy"); (b) cover-modulo-growth
// still holds under the pinned outcomes; (c) only the acting player's own
// choices are pinnable — opponent-choice entries remain rejectors unless
// EVERY option preserves the certificate (the win stays forced per the
// CR 104.2a-grounded winner predicate). Plug pins in at THIS seam as an
// additional input; do not rewire the classifiers or spread the decision.
let mut needs_life_guard = false;
for entry in &current.stack {
    match stack_entry_resolution_choice_freedom(entry) {
        ResolutionChoiceFreedom::MayPrompt => return false,
        ResolutionChoiceFreedom::FreeUnlessLifeReplacements => needs_life_guard = true,
    }
}
if needs_life_guard && life_event_replacements_may_prompt(current) {
    return false;
}
```

**(d) Doc updates:** cover-fn doc list (:688-694) gains item (6); `stack_entry_has_no_ordering_input` doc (:851-859) gets one line delimiting its contract to announcement-time ordering input, with resolution-time choices owned by item (6). Perf note: O(stack × AST) + O(objects × defs), same order as items (4)/(5).

### 5.4 Tests — `resource.rs` N1 module additions (reuse `churn_entry`/`gain_ability`/`cover_base`/`bf_object`)

Naming continues the N1 letter scheme; each test carries the §6 matrix row as its doc comment.

- **n1_o_grown_choice_opening_proliferate_false** (finding fixtures i + iii in one): `cover_base()`-shaped pair whose grown kind wraps `Effect::Proliferate` (unit variant; empty targets) — prior `[G, P]`, current `[G, P, P]`; **zero counters anywhere** in either state, so in the current state the entry would auto-resolve without any prompt (`eligible.is_empty()`, proliferate.rs:90) — proving the gate is *structural*, not observational (the projected poison axis can inhabit the option surface mid-extrapolation). Assert `!cover(...)`. Inline reach-guard: same pair with P swapped for `gain_ability(2)` churn entries ⇒ `cover == true` (proves the fixture passes gates 1-5 and only item 6 rejects). **Pre-req check for discriminance:** confirm `scan_effect(Effect::Proliferate)` is not `CONSERVATIVE` on the projected axis (the maintainer measured it as non-reading; verify in scan_effect's arm) — otherwise item 4 masks the new gate and a different clean kind (Explore, per the finding) is substituted.
- **n1_q_ungrown_choice_opening_entry_false** (H2 discriminator): prior `[P, G]`, current `[P, G, G]` — P count equal (un-grown), G grown+allow-listed. Assert `!cover(...)`. Inline reach-guard: drop P from both stacks ⇒ `true`. This test is what forces the all-entries scope; the grown-only mutation turns exactly this test red.
- **n1_r_life_replacement_environment_false** (H4, five arms; def-construction precedent: n1_kr, resource.rs:2282+; every def is condition-free with a `QuantityModification`-or-empty body unless stated, so it *survives* item 5 — no-read bodies per resource.rs:1046-1060):
  1. **Optional GainLife def** on an object present in BOTH states ⇒ `!cover(...)` (clause a). Mutation: delete the `needs_life_guard` conjunct ⇒ RED.
  2. **Two MANDATORY quantity-mod GainLife defs** ⇒ `!cover(...)` (clause c, ≥2 per proposed class). Mutation: change clause (c) to count per `ReplacementEvent` key spread across two different life keys — N/A here (same key); primary mutation: drop clause (c) ⇒ RED.
  3. **Optional `PayLife` def + LoseLife churn entries** (B1 fixture; add a `lose_ability(n)` helper mirroring `gain_ability`, wrapping `Effect::LoseLife`): PayLife's matcher matches `ProposedEvent::LifeLoss` (replacement.rs:3324-3326), so this def can prompt a grown LoseLife resolution ⇒ `!cover(...)`. Mutation: narrow the life-class set to `{GainLife, LoseLife}` (the hand-picked set that misses PayLife) ⇒ RED.
  4. **Single MANDATORY GainLife def with `runtime_execute: Some(<prompt-capable, non-projected-reading body>)`** (B2 fixture; builder ability.rs:17031-17034) ⇒ `!cover(...)` (clause b). Inline reach-guard: assert `!ability_reads_projected_resource(&runtime_body)` — proves item 5 passes the def and only clause (b) rejects. Mutation: drop the `runtime_execute.is_some()` half of clause (b) ⇒ RED.
  5. **Floating-store arm** (M3): the arm-1 optional GainLife def placed in `state.pending_damage_replacements` on both states (no object def) ⇒ `!cover(...)`. Mutation: drop the floating-store chain from the guard's def sources ⇒ RED.
  Shared inline reach-guard: the arm-1 def with a non-life `event` (e.g. `Mill`) ⇒ `cover == true`.
- **n1_s_resolution_timing_targets_false** (H3): grown G whose ability has `target_choice_timing = TargetChoiceTiming::Resolution` (targets empty ⇒ passes today's ordering gate). Assert `!cover(...)`. Inline reach-guard: identical ability with `Stack` timing ⇒ `true`.

No changes to `tests/pr65_growing_cascade_win.rs`, analysis/loop_check.rs tests, or any existing N1 test — they are verification targets, not modification targets.

---

## 6. Verification matrix (test → property → mutation that makes it RED)

Executed-revert-fail protocol: implementer applies each mutation, runs the named test, records RED in the commit message, restores (the 9092a8961 protocol).

| Test | Property pinned | Paired positive reach-guard | Mutation ⇒ RED |
|---|---|---|---|
| n1_o (new) | grown choice-opening kind rejects even when it would auto-resolve in the compared state | inline: P→G swap ⇒ cover true | delete item (6) loop, or classify `Proliferate` ⇒ `FreeUnlessLifeReplacements` |
| n1_q (new) | non-grown choice-opening entry rejects (all-entries scope) | inline: remove P ⇒ cover true | scope item (6) to `cn > pn` entries only |
| n1_r (new, 5 arms) | life-replacement environment rejects allow-listed kinds: optional (a), ≥2 per proposed class (c), PayLife class-set completeness (B1), mandatory `runtime_execute` body (B2, clause b), floating store (M3) | inline: non-life event class ⇒ cover true; arm 4 additionally asserts `!ability_reads_projected_resource(&runtime_body)` (item-5 pass proof) | per-arm: delete `needs_life_guard` conjunct; drop clause (c); narrow life-class to `{GainLife, LoseLife}`; drop `runtime_execute` half of clause (b); drop the floating-store chain |
| n1_s (new) | `Resolution` target timing rejects despite empty targets | inline: `Stack` timing ⇒ cover true | remove the `target_choice_timing` row from the ability classifier |
| resolution_choice_verdicts_are_exactly_pinned (new) | classifier output set exact, both directions; ability-level wrapper flips | unmutated base ability ⇒ `FreeUnlessLifeReplacements` | classify `Scry` ⇒ `FreeUnlessLifeReplacements` (executed, restored) |
| n1_p1, n1_p2 (existing, unmodified) | no false rejection of the shipped homogeneous / interleaved covers | — (they ARE the positives) | any over-broad reject (e.g. `GainLife` ⇒ `MayPrompt`) turns these red |
| N2 (analysis/loop_check.rs:1174+), N3 ON arm (pr65_growing_cascade_win.rs:110) | end-to-end drain cascade still certifies through the parser-produced ASTs | — | wrong allow-list (parsed kind not admitted) turns these red — this is the §5.2 pre-flight probe's runnable form |
| N3 OFF arm (:151) | default-OFF byte-behavior (natural death, no detector artifacts) | — | any gating regression |
| N1(a-n, kg, kr, ks), N5, N1(kg) | all prior hostile rejections still hold (item 6 is additive) | — | — |

Non-vacuity/discrimination argument: every new negative fixture differs from an in-test passing positive **only** on the choice axis (the inline reach-guards), so rejection provably comes from the new gate and not from gates (1)-(5) upstream — the exact "paired positive reach-guard" the review bar requires. The guard test pins both verdict directions with representative instances, and its revert-fail is executed, not hypothesized.

Census requirements (both recorded in the commit message):
1. **Wildcard census:** `rg -n '_ =>' crates/engine/src/game/ability_scan.rs crates/engine/src/analysis/resource.rs` output must be **unchanged vs `2e7ad800c`** (the new matches contribute zero wildcards; grouped `|` arms name every variant; the `replacement_event_matches_life` helper also names every `ReplacementEvent` variant).
2. **Allow-arm census (M5):** `rg -c 'ResolutionChoiceFreedom::FreeUnlessLifeReplacements' crates/engine/src/game/ability_scan.rs` — exactly **2** occurrences inside `effect_resolution_choice_freedom`'s match (the `GainLife` and `LoseLife` arms); the total file count is pinned at implementation time and recorded next to the pinned expected set, stated explicitly: **the allow set is exactly `{Effect::GainLife, Effect::LoseLife}`**. Any future third allow arm must update this census line, the guard test's explicit-set assertion (§5.1(e)), and add a §5.2-grade grounding row — a hand-picked-representative pin alone would not catch it.

Process: `cargo fmt --all` directly; everything else through Tilt (`./scripts/tilt-wait.sh clippy test-engine`, card-data unaffected — no parser change). No direct cargo builds.

---

## 7. CR verification log (every number grep-verified this session)

Command form: `grep -n "^<ref>" docs/MagicCompRules.txt` (first matched line quoted, truncated):

| CR | Matched text (truncated) | Used for |
|---|---|---|
| 732.2a | "…predictable results of the sequence of choices… It can't include conditional actions, where the outcome of a game event determines the next action a player takes." | the fix's primary annotation |
| 732.4 | "If a loop contains only mandatory actions, the game is a draw." | context in docs |
| 732.5 | "No player can be forced to perform an action that would end a loop…" | PR-reply discrepancy note only — NOT used in code |
| 104.4b | "…enters a 'loop' of mandatory actions… the game is a draw." | context |
| 614.7 | "…that event never happens, the replacement effect simply doesn't do anything." | evidence that life.rs's existing "multiple competing" comments miscite; new code uses 616.1 |
| 616.1 | "If two or more replacement and/or prevention effects are attempting to modify… the affected player chooses one to apply…" | environmental guard annotation |
| 601.2c | "The player announces their choice of an appropriate object or player for each target…" | timing docs |
| 603.3c | "If a triggered ability is modal, its controller announces the mode choice when putting the ability on the stack." | modal firewall doc |
| 603.3d | "…If a choice is required when the triggered ability goes on the stack but no legal choices can be made…" | ordering-gate contract doc |
| 608.2c | "The controller of the spell or ability follows its instructions in the order written…" | chain-resolution doc |
| 608.2d | "If an effect of a spell or ability offers any choices other than choices already made as part of casting… the player announces these while applying the effect." | THE resolution-time-choice rule — classifier annotation |
| 701.34a | "To proliferate means to choose any number of permanents and/or players that have a counter…" | n1_o fixture doc |
| 701.21a | "To sacrifice a permanent, its controller moves it…" | guard-test pin comment |
| 119.7 / 119.8 | can't gain / can't lose life | allow-list trace docs |
| 122.1 | "A counter is a marker placed on an object or player…" | projected player-counter axis doc |
| 608.1 / 405.5 | top-of-stack resolution / LIFO | H1 frozen-entry rationale |
| 603.4 | intervening 'if' clause | trigger-condition purity note |
| 704.5a | player at 0 or less life loses | context |
| 104.2a | "A player still in the game wins the game if that player's opponents have all left the game. This happens immediately and overrides all effects…" | extension-point comment at the item (6) seam (winner-predicate grounding) |
| 702.105a | Dethrone definition | referenced only in existing code discussion |

Rule for the implementer: any ADDITIONAL number that ends up in code comments must be grep-verified the same way before writing (validate-cr-annotations skill).

---

## 8. Engine-planner mandatory sections (condensed)

- **Pattern Coverage:** detector-soundness fix — covers every card class that can appear in a growing-cascade certification (all ~210 effect kinds are now *classified*); the allow-list certifies the life-drain cascade class (Blight Priest / Marauding Blight-Priest / Vito / Sanguine Bond / Epicure of Blood style — dozens of cards), everything else is soundly rejected rather than unsoundly certified.
- **Building Blocks consulted:** ability_scan walker + its closure rule (:1-56, :115-225); `active_replacements` (functioning_abilities.rs:322); `replacement_mode_is_optional` (replacement.rs:638); N1 fixture helpers (resource.rs:1953-2022); `cost_component_choice_free` naming precedent (mana_abilities.rs:2553, not reusable — different layer). New helpers justified: no existing surface answers "can this resolver prompt".
- **Logic Placement:** classifier in `game/ability_scan.rs` (AST-adjacent, walker discipline lives there); entry/environment gates in `analysis/resource.rs` (analysis-local consumers, mirrors items 4/5); zero frontend/WASM/AI/parser changes.
- **Rust Idioms:** typed 2-variant enum over bool; exhaustive no-wildcard matches; destructure-without-`..` on allow-list arms and the `ResolvedAbility` walk; `pub(crate)` minimal exposure.
- **Nom Compliance:** N/A — no parser files touched.
- **Extension vs Creation:** extends the established fail-closed scan pattern (items 4/5, commit bceec86e3) with a new conjunct; the guard test extends the 9092a8961 pattern.
- **Analogous Trace (hard gate):** traced item-5 granted-keyword hardening end-to-end: `git show 9092a8961` (guard test, triggers.rs:22566) → `git show bceec86e3` (loop (iv) fix, resource.rs:1017-1042, `granted_keyword_triggers_in_zone`) → consumer `loop_states_cover_modulo_growth` (resource.rs:695) → caller `live_mandatory_loop_winner` (analysis/loop_check.rs:288) → gate `engine.rs:323/:671` → integration `tests/pr65_growing_cascade_win.rs`.
- **Identity/Provenance:** N/A (no "this way"/chosen-source binding added); the only binding-like contract is the `FreeUnlessLifeReplacements` ↔ environmental-guard pairing, made type-visible (§4.3) and hostile-tested (n1_r's multi-authority arm: two mandatory defs).
- **Variant Discoverability:** no engine AST enum variant added; `cargo engine-inventory` + add-engine-variant lens run for the new analysis-local type (§4.3).
- **Scope matrix:** reachable `StackEntryKind`s at the touched boundary enumerated (all four, explicit arms); grown vs non-grown × choice-free vs not × replacement environment covered by n1_o/q/r/s; serialization boundary untouched (new type non-serialized, `pub(crate)`).

## 9. Non-goals / hard scope constraints

1. **ZERO `.claude/skills/` changes on the #4904 branch** — maintainer-sweep hard stop. The checklist edit ships via #4905 (§11). The implementer must not create/modify anything under `.claude/skills/`.
2. No changes to the 2p equality path (`loop_states_equal_modulo_resources`) — its inherited extrapolation assumption is separately documented (resource.rs:628-639) and regression-pinned; out of scope.
3. No `WaitingFor`/`GameAction`/wire-serialized type changes. If implementation appears to require touching one, STOP and report (none identified).
4. No parser changes; no frontend/AI changes; no fixing of the pre-existing life.rs CR 614.7 miscite (noted for follow-up, different file).
5. No attempt to certify damage/ping cascades (`DealDamage` stays `MayPrompt` — its prevention/redirect/ordering surfaces need their own grounding PR).
6. **No speculative pinned-choice code.** The §4.5 structural requirements (fact-producing classifiers, single gate seam, extension-point doc comment) are the ONLY future-facing items permitted: no pin fields, no pin enum variants, no pin parameters, no verdict granularity beyond boolean-class `MayPrompt`. The PR description and the reply to the maintainer must NOT mention any future feature.

## 10. Residual gaps (honest)

- **R1 — matcher coupling of the life guard:** the life-class set `{GainLife, LoseLife, LifeReduced, PayLife}` is derived from the registry MATCHERS (§5.3(b)); the compiler-exhaustive `replacement_event_matches_life` match protects against NEW `ReplacementEvent` variants landing unclassified, but a behavior change to an EXISTING matcher (e.g. someone widening a non-life matcher to also match `ProposedEvent::LifeLoss`) is only grep-enforced (the §5.3(b) derivation grep, re-run when the set is edited), not compile-enforced — a registry-probe pin test is not robust because `lose_life_matcher` is state-dependent (replacement.rs:2468-2479). Also: new *virtual* life candidates in `find_applicable_replacements` would need the guard extended (§5.3(b) sweep; none exist today).
- **R2 — frozen-bottom over-rejection** (false negatives only): non-TriggeredAbility or choice-bearing entries that the extrapolation would never actually resolve still reject the cover. Ceiling documented; upgrade path = model which stack suffix resolves per cycle.
- **R3 — allow-list minimalism:** many genuinely prompt-free kinds (e.g. pure counters/pump classes) are `MayPrompt` until someone funds their resolver traces. False negatives only.
- **R4 — 2p equality path** retains its documented weaker assumption (out of scope, pre-existing, pinned).
- **R5 — post-replacement continuation prompts: CLOSED by guard clause (b), not escalated.** The drain path (`apply_pending_post_replacement_effect`, engine_replacement.rs:1159) CAN prompt — but only when a life-class def carries a body continuation (`execute`/`runtime_execute`), and clause (b) rejects exactly those defs, so `GainLife`/`LoseLife` stay on the allow-list and N2/N3 stay green. Remaining implementer obligation (not a residual hole): confirm the §5.2 prompt inventory — the four life.rs `waiting_for` raises are all NeedsChoice and no non-replacement prompt exists in `resolve_gain`/`resolve_lose`; any additional raise found ⇒ that kind drops to `MayPrompt` and STOP/report.

## 11. SEPARATE DELIVERABLE — draft SKILL.md edit for PR #4905 (branch `docs/add-engine-variant-walker-checklist`; team-lead lands it; NOT on #4904)

Append to step 3 ("Classify the variant in the fail-closed ability-scan walker") of `.claude/skills/add-engine-variant/SKILL.md`, after the existing paragraph:

> The same discipline applies to the walker's **resolution-time choice classifier** (`effect_resolution_choice_freedom` / `ability_resolution_choice_freedom` in `crates/engine/src/game/ability_scan.rs`, consumed by `analysis::resource::loop_states_cover_modulo_growth` item 6): a NEW `Effect` variant fails to compile there until classified. The default classification is `MayPrompt` (fail-closed — an unproven claim only costs a false-negative cover rejection). Classifying a variant as choice-free (`FreeUnlessLifeReplacements`, or any future `Free`-class verdict) is a SOUNDNESS claim — "resolving can never enter a non-priority `WaitingFor`, for ANY state" — and requires (a) a resolver trace cited in the arm's comment (file:line of the handler in `game/effects/` proving no `WaitingFor` raise on any path, including replacement-pipeline calls such as `replace_event`'s `NeedsChoice`), (b) destructuring the arm without `..` so a future field forces re-audit, and (c) updating the pinned guard test (`resolution_choice_verdicts_are_exactly_pinned`). A new field on a `MayPrompt`-classified variant needs no action (`{ .. }` arms are already fail-closed).

## 12. Implementation order

1. ability_scan.rs: enum + `effect_resolution_choice_freedom` + `ability_resolution_choice_freedom` + header note (§5.1 a-d).
2. resource.rs: `stack_entry_resolution_choice_freedom` + `life_event_replacements_may_prompt` + item (6) wiring + doc updates (§5.3), with the §5.2/§5.3(b)/R5 verification sweeps done FIRST.
3. Tests: guard test (§5.1 e), n1_o/q/r/s (§5.4).
4. `cargo fmt --all`; Tilt `clippy` + `test-engine` green; run every §6 mutation, record RED results, restore.
5. Census greps (§6: wildcard census unchanged vs base + allow-arm census recorded); §4.5 structural-requirement check (fact-producing classifiers, single seam, extension-point comment present); commit (conventional commits + `Assisted-by:` trailer), zero `.claude/skills/` paths in the diff.
