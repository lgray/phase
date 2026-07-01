# MSH-E Implementation Plan — ROUND 2 — Hawkeye, Master Marksman + The Ruinous Wrecking Crew

Branch: `feat/msh-e-marksman-ruinous` (worktree `/home/lgray/vibe-coding/wt-msh-f`).
**Base RE-MEASURED at commit `aa4e88ec4`** (v0.8.0 + MSH-F). Every line number below was
re-located in this base by grepping the function name / distinctive code; old-base numbers from
MSH-E-PLAN.md / MSH-E-REVIEW-r1.md were treated as stale hints only. Planning only — no engine
edits, no commits.

This r2 folds in every finding from `MSH-E-REVIEW-r1.md`:
- **A1 BLOCKER** (detector regressed 2 green cards) → fixed with the `"modal":{` node gate, re-measured fully discriminating on the current corpus.
- **A2** (dead `:null` branch) → key on absence of `"dynamic_max_choices":{`; serde attr re-cited.
- **A3** (loose bare `"choose up to x"`) → recognizer proven independently safe on greens; em-dash form preferred.
- **B1** (variant gate) → re-run on current inventory; APPROVED.
- **B2** (wrong seam; new driver is first-class) → re-pointed to `repeated_full_chain` + the up-front gate; the Fixed-count per-iteration-optional + early-stop driver is scoped as explicit first-class work.
- **B3** (flag-carryover mis-located) → tied to the new driver at `repeated_full_chain`.
- **B4** (CR precision) → **corrected beyond the review**: CR 700.2b (not 601.2b *or* 700.2e) is the rule for an illegal mode in a reflexive *triggered* modal (measured below).

---

## Audit findings (RE-MEASURED — the gap is RUNTIME-classification, not parser)

Both cards: `gap_count=0`, every `parse_details` node `supported:true`, yet card `supported:false`
(measured in `data/coverage-data.json`). The flip is produced by **`check_silent_drops`**
(`coverage.rs:5123-5141`), called from `analyze_coverage` at **`coverage.rs:4464`** — NOT by
`check_resolver_features` (4460), NOT by `check_parse_warnings` (4469); neither card has any
`Effect::Unimplemented` or fired swallow detector today.

### Shared mechanism (re-located, new base)
`check_silent_drops` flags a card when `count_effective_oracle_lines > count_effective_parsed_items`.
- `count_effective_oracle_lines` (`coverage.rs:5377-5434`) folds a modal's bullet lines into its
  header **only when `is_modal_header_line` recognizes the header** (call at `5413`).
- `is_modal_header_line` (`coverage.rs:5438-5464`) — `CHOOSE_PHRASES` list (5439-5462) matches
  `"choose up to one".."ten"`, `"choose any number"`, `"choose x."` — but **NOT `"choose up to x"`
  or `"choose up to that many"`**. Idiom: `CHOOSE_PHRASES.iter().any(|p| lower.contains(p))`.
- `count_effective_parsed_items` (`coverage.rs:5589-5601`) counts each top-level item + its DIRECT
  children (`1 + children.len()`).

**Hawkeye** (measured parse_details: `[FirstStrike c0, Reach c0, Taps-trigger c1]`) →
`effective_parsed = 1+1+(1+1) = 4`. Oracle lines: `"First strike, reach"`(1), header line(2, NOT
recognized), 3 bullets(3,4,5) → `effective_oracle = 5`. `5 > 4` → **`SilentDrop:4_of_5`**.

**Ruinous** (measured parse_details: `[ChangesZone-trigger c1, Moved-replacement c1]`) →
`effective_parsed = (1+1)+(1+1) = 4`. Oracle lines: enters-with-X(1), header(2, NOT recognized),
4 bullets(3,4,5,6) → `effective_oracle = 6`. `6 > 4` → **`SilentDrop:4_of_6`**.

### Runtime reality per card (determines false-green risk)
**Ruinous — runtime FULLY WORKS; the gap is purely the coverage marker.**
- Destroy-token mode: `FilterProp::Token => obj.is_token` (`filter.rs:3179`). ✓
- Each-player-sacrifice mode: `Sacrifice { player_scope: All }` iterates APNAP per-player with
  controller rebind (test `player_scope_all_sacrifice_iterates_each_player`, `sacrifice.rs:1064`). ✓
- Lose-2-life / discard-then-draw modes: standard. ✓
- Replacement "enters with X +1/+1 counters": `cost_x_paid = ability.chosen_x`
  (`casting_costs.rs:5601`) stamped `obj.cost_x_paid = Some(x)` at `casting_costs.rs:5641`
  (CR 107.3m); read by `resolve_quantity(CostXPaid)`; Walking Ballista test
  (`engine.rs:7979 walking_ballista_enters_with_x_counters_and_survives_zero_zero_sba`). ✓
- modal `dynamic_max_choices: Some(Ref CostXPaid)` resolved+clamped to `mode_count` at runtime
  (`modal_choice_for_player`, `ability_utils.rs:571-573`). ✓
  ⟹ **Ruinous needs only the coverage line-counter fix. No engine/parser change. No false green.**

**Hawkeye — the coverage marker MASKS genuine semantic gaps; a coverage-only fix would FALSE-GREEN.**
Measured AST (`card-data.json`, exported trigger): `PayCost{ optional:true, repeat_for:Fixed(3),
sub_ability: { condition:WhenYouDo, modal:{ min_choices:1, max_choices:1, mode_count:3,
dynamic_max_choices: ABSENT } } }`. Three defects:
1. Modal max fixed `(1,1)` — the dynamic cap (= times `{1}` paid) is dropped; the player is forced
   to choose exactly one mode.
2. Optionality + repeat structure wrong (see B2): one up-front yes/no, then 3× mandatory.
3. No QuantityRef expresses resolution-local "times a cost was paid" (every existing `*Paid`/`*Count`
   ref is cast-time); plus `cost_payment_failed_flag` is not cleared per iteration in the
   repeated path.
  ⟹ **Hawkeye needs a real engine+parser feature; the coverage marker must stay RED until it lands.**

### CR basis (every number grep-verified in `docs/MagicCompRules.txt`)
| CR | line | text (abbrev) |
|---|---|---|
| **603.12** | 2656 | reflexive "when [a player] does …" triggers based on whether the event occurred during resolution |
| **603.12a** | 2659 | *"if a resolving spell or ability includes a choice to pay a cost multiple times and creates a triggered ability that triggers when that payment is made, paying that cost one or more times causes the reflexive triggered ability to trigger only once."* — **exactly Hawkeye** |
| **700.2** | 3203 | modal = bulleted options preceded by "choose a number" instruction |
| **700.2a** | 3205 | modal **spell/activated**: illegal mode can't be chosen (see 601.2b) |
| **700.2b** | 3207 | modal **triggered ability**: controller chooses mode(s) when put on stack; **illegal mode can't be chosen**; if no mode chosen, removed from stack |
| **700.2d** | 3211 | can't choose more distinct modes than allowed; "same mode more than once" only if permitted (the dynamic-cap clamp) |
| **107.3m** | 488 | ETB ability/replacement X = the spell's chosen cast X — Ruinous |
| **608.2c** | 2793 | controller follows instructions in order; resolution-time semantics |
| **701.26** | 3518 | Tap and Untap (the tap trigger) |
| **701.21** | 3449 | Sacrifice (Ruinous mode) |
| **119.3** | 1065 | lose life (Ruinous mode) |
| **122.1** | 1178 | counter (Ruinous replacement) |

**B4 correction (measured, supersedes review):** the review proposed CR 700.2e for "illegal mode
can't be chosen." Grep shows **700.2e (line 3213)** is *"a player other than their controller
chooses a mode"* — NOT applicable (Hawkeye's controller chooses). **601.2b (line 2459)** is modal
announcement *during casting* — also N/A to a reflexive *triggered* modal. The precise rule for
Hawkeye's reflexive triggered modal's illegal-mode handling is **CR 700.2b (line 3207)**. r2 uses
**700.2b + 700.2d**; it drops 601.2b and 700.2e from the reflexive-modal annotations.

---

## Decision: SPLIT into two independently-committable sub-plans (unchanged)

Ruinous has zero runtime gap; Hawkeye has a large one. They share exactly one root cause (the
coverage line-counter). Sub-plan A lands the line-counter fix + a discriminating honesty guard that
keeps Hawkeye (and Tranquil Frillback) RED until Sub-plan B makes Hawkeye genuinely correct. Two
clean commits, never a false green.

---

# Sub-plan A — The Ruinous Wrecking Crew (coverage-audit fix)

### Scope
Teach the coverage silent-drop line-counter to recognize **dynamic modal headers** as a class, and
add one discriminating swallow detector so a card whose dynamic modal max was *dropped* stays
unsupported. Net: Ruinous → `supported:true`; Hawkeye + Tranquil Frillback → still `supported:false`
(now held by the principled guard, not a phrase-list gap).

### A1 RE-MEASUREMENT — the modal-node gate is fully discriminating, ZERO green regressions

Marker class = cards whose oracle (lowercased) matches `choose up to (x|that many)`. Re-measured on
the **current** `data/card-data.json` / `coverage-data.json` (10 cards — same class size the review
reported). For each: `"modal":{` = a parsed modal node is present in the serialized AST;
`"dynamic_max_choices":{` = the modal carries a dynamic cap. The **fixed** detector fires iff
`"modal":{` present **AND** `"dynamic_max_choices":{` absent **AND** oracle has the dynamic header:

| card | supported | gap | `"modal":{` | `"dynamic_max_choices":{` | detector fires | result |
|---|---|---|---|---|---|---|
| Hawkeye, Master Marksman | false | 0 | ✅ | ❌ | **FIRES** | stays RED (honesty guard) ✓ |
| The Ruinous Wrecking Crew | false | 0 | ✅ | ✅ | no | **greened by line-counter fix** ✓ |
| Tranquil Frillback | false | 0 | ✅ | ❌ | FIRES | stays red (exact Hawkeye shape) ✓ |
| Bumi, King of Three Trials | false | 1 | ✅ | ❌ | FIRES | already red (gap 1); reason explicit ✓ |
| Riku of Many Paths | false | 1 | ✅ | ❌ | FIRES | already red (gap 1) ✓ |
| Discordant Dirge | false | 1 | ❌ | ❌ | no | already red (gap 1) ✓ |
| Reap Intellect | false | 1 | ❌ | ❌ | no | already red (gap 1) ✓ |
| Suppression Ray | false | 1 | ❌ | ❌ | no | already red (gap 1) ✓ |
| **Heroic Feast** | **true** | 0 | **❌** | ❌ | **no** | **stays GREEN — NO REGRESSION** ✓ |
| **Temporal Firestorm** | **true** | 0 | **❌** | ❌ | **no** | **stays GREEN — NO REGRESSION** ✓ |

The two previously-green cards have **no parsed modal node** — their "choose up to X/that many
<nouns>" is a *non-modal selection clause* (Heroic Feast: "choose up to that many target creatures
you control"; Temporal Firestorm: "Choose up to X creatures … where X is the number of times this
spell was kicked"). The `"modal":{` requirement is exactly what excludes them. **BLOCKER A1
RESOLVED, re-confirmed on the current corpus.**

### A3 — the line-counter recognizer is independently safe on greens (measured)
Heroic Feast and Temporal Firestorm have **0 bullet lines** (`grep -c '•'`). Recognizing their
"choose up to" line as a header sets `in_modal=true`, but the next non-empty line is non-bullet → the
loop clears `in_modal` and counts it normally → `effective_oracle` **unchanged** → no flip. Ruinous
has 4 bullets → folding drops `effective_oracle` 6→2 (≤ `effective_parsed`=4) → green. **Hawkeye
has 3 bullets → the recognizer ALONE would fold 5→2 and FALSE-GREEN it** — which is exactly why the
detector (A1) is mandatory, not optional.

### Files & changes

**A-1. `crates/engine/src/game/coverage.rs` — `is_modal_header_line` (5438-5464).**
Recognize the dynamic header forms as a class. Compose, don't enumerate: add recognition of a
`"choose up to "` lead followed by `"x"`, `"that many"`, or an already-listed number word. **Prefer
the em-dash header form** where practical (`"choose up to x —"`, `"choose up to that many —"`) to
keep the bare loose match (A3) from matching non-modal selection clauses; fall back to the bare form
only as needed (the detector, not this recognizer, is the load-bearing honesty gate, so a loose
recognizer match cannot false-green a card on its own — A3 proven above). This file is `game/`, not
`parser/`, and the function is a line-classifier: matching the established `lower.contains(p)` idiom
is the consistent choice (mark new arms `// allow-noncombinator` like siblings if lint requires).
- CR annotation: `// CR 700.2 + CR 107.3m: a dynamic modal header ("choose up to X / up to that
  many —") plus its bulleted modes is one logical unit; fold the bullets into the header so a parsed
  modal (1 parent + N children) is not miscounted as N+1 dropped Oracle lines.`

**A-2. `crates/engine/src/parser/swallow_check.rs` — new detector `detect_modal_dynamic_max_dropped`.**
Register it in `check_swallowed_clauses` after the existing detectors (insert in the call block
`105-117`, before the closing brace at `118`). Build on the established pattern: it consumes the
precomputed `ast_json` (`swallow_check.rs:103`) and the `cleaned` lowercased text, exactly like
`detect_dynamic_qty` (`swallow_check.rs:1355`) using the `json_has_any` helper (`swallow_check.rs:1342`).
It fires iff **all three** hold:
1. `cleaned` contains a dynamic modal header marker — `"choose up to that many"` OR a `"choose up to
   x"` header (require the `"choose "` lead; mark `// allow-noncombinator: swallow detector marker
   scan on classified text`).
2. **`ast_json` contains `"modal":{`** — a parsed modal node exists. *(A1 fix — this is the new
   load-bearing gate. `modal: Option<ModalChoice>` at `ability.rs:13275` has no
   `skip_serializing_if`, so it serializes `"modal":null` when `None` and `"modal":{` only when
   `Some` — the substring is an exact proxy for "a modal node was parsed".)*
3. **`ast_json` does NOT contain `"dynamic_max_choices":{`.** *(A2 — `dynamic_max_choices:
   Option<QuantityExpr>` at `ability.rs:12924` carries `#[serde(default, skip_serializing_if =
   "Option::is_none")]`, so the field is OMITTED when `None` and emitted as `"dynamic_max_choices":{…}`
   when `Some`. Key on ABSENCE of the `:{` form — there is no `:null` form to test.)*
   Emit `OracleDiagnostic::SwallowedClause { detector: "Modal_DynamicMaxDropped", … }` (mirror the
   `detect_dynamic_qty` emit shape; use `truncate` for the fragment).
- No `coverage.rs` support-predicate change needed: `parse_warning_gap_label` (`coverage.rs:5204`)
  already maps `SwallowedClause{detector}` → `"Swallow:{detector}"`, and `check_parse_warnings`
  (`coverage.rs:5184-5193`, called at `4469`) folds that into `missing`. The detector participates
  automatically via `face.parse_warnings`. `check_swallowed_clauses` early-returns on
  `any_ability_has_unimplemented` (`swallow_check.rs:93`) so it never double-reports an Unimplemented.
- CR annotation: `// CR 700.2 + CR 700.2d: a "choose up to X / up to that many" MODAL header whose
  dynamic cap was not captured (a "modal":{ node exists but dynamic_max_choices is None) silently
  mis-sizes the modal; surface it so coverage stays honest. The "modal":{ gate excludes non-modal
  "choose up to X <nouns>" selection clauses (Heroic Feast / Temporal Firestorm).`

### Pattern coverage (A)
- A-1 covers **every** dynamic-modal-header card (all #4186 cast-X cards + every future "choose up
  to that many" card).
- A-2 covers the **class** "a modal header was parsed but its dynamic cap was dropped" — any current
  or future card where the parser produces a modal node without `dynamic_max_choices`. Today that
  is Hawkeye + Tranquil Frillback (Frillback is the same reflexive-modal shape); Bumi/Riku are also
  flagged but already red. Neither change is a one-card patch.

### Verification matrix (Sub-plan A)

| # | Claim | Seam (new line) | Production entry | Discriminating revert-fail test (names the exact line whose reversion flips the assertion) | Hostile / negative |
|---|---|---|---|---|---|
| A-i | Dynamic header folds its bullets | `is_modal_header_line` (`coverage.rs:5438`) | `count_effective_oracle_lines` (real `card-data`) | New unit test mirroring `count_effective_oracle_lines_recognizes_choose_up_to_four` (`coverage.rs:11211`): input `"…choose up to X —\n• …\n• …\n• …\n• …"` ⇒ returns **2** (enters line + folded header), not 6. **Revert** = drop the new `is_modal_header_line` arm ⇒ returns 6 ⇒ assert fails. | A *non-modal* `"choose up to that many target creatures you control.\nPut a +1/+1 counter on each of them."` (Heroic Feast text, 0 bullets) ⇒ count is **unchanged** by the recognizer (no bullets to fold). Fixed `"choose up to two —"` over 2 bullets still folds (regression guard). |
| A-ii | Ruinous becomes supported | `is_modal_header_line` + `check_silent_drops` (`coverage.rs:5123`) | `analyze_coverage` over real `card-data` | After A-1, `jq '.cards[]|select(.card_name=="The Ruinous Wrecking Crew").supported'` on regenerated `coverage-data.json` == `true`. **Revert** A-1 ⇒ `effective_oracle` 6>4 ⇒ `SilentDrop:4_of_6` ⇒ `false` ⇒ fails. | Diff the full supported set before/after; assert **only** Ruinous flips green (no unrelated card). |
| A-iii | Hawkeye stays unsupported (honesty guard) | `detect_modal_dynamic_max_dropped` (new, `swallow_check.rs`) | `check_swallowed_clauses` over real AST → `check_parse_warnings` | Unit test: build a `ParsedAbilities` whose serialized AST has `"modal":{…}` with `max_choices < mode_count` and **no** `dynamic_max_choices`, oracle `"…choose up to that many."` ⇒ diagnostics contain `SwallowedClause{detector:"Modal_DynamicMaxDropped"}`. **Revert** = remove the detector registration at `swallow_check.rs:~117` ⇒ no diagnostic ⇒ with A-1 also applied, Hawkeye false-greens ⇒ fails. | (a) modal **with** `dynamic_max_choices:Some` + same oracle ⇒ **silent** (proves it keys on the AST cap, not the phrase — Ruinous). (b) **non-modal** `"choose up to X creatures"` AST (no `"modal":{`) ⇒ **silent** (the A1 fix — Heroic Feast / Temporal Firestorm). (c) AST with `Effect::Unimplemented` ⇒ `check_swallowed_clauses` early-returns (`:93`) ⇒ silent (no double-report). |
| A-iv | End-to-end coverage honesty | `analyze_coverage` | `card-data` Tilt resource | On regenerated `coverage-data.json`: Ruinous `supported==true`; Hawkeye **and** Tranquil Frillback `supported==false`; Heroic Feast & Temporal Firestorm **remain** `true`. | Full supported-set diff: assert the ONLY green flip is Ruinous and ZERO cards turn red. |

**Non-vacuity evidence:** A-i/A-ii name `is_modal_header_line`'s new arm (revert ⇒ count 6, red);
A-iii names the detector registration (revert ⇒ no diagnostic, false-green). The Ruinous→true /
Hawkeye→false / Heroic-Feast-stays-true triple is the discriminator proving the fix folds headers
*without* greening an un-implemented dynamic modal *and without* regressing a non-modal green.

### Commit (A)
`fix(coverage): fold dynamic "choose up to X / that many" modal headers; flag dropped dynamic max`
(Self-contained: Ruinous's parser+runtime already exist on the base.)

---

# Sub-plan B — Hawkeye, Master Marksman (engine + parser feature)

### Card / rules
`First strike, reach` (supported). `Trick Arrows — Whenever Hawkeye becomes tapped, you may pay {1}
up to three times. When you do, choose up to that many. • Net — Target creature can't block this
turn. • Explosive — Hawkeye deals 2 damage to target player. • Boomerang — Discard a card, then draw
a card.`

Semantics (CR 701.26 tap trigger; CR 603.12a; CR 700.2/700.2b/700.2d): on Hawkeye becoming tapped,
the controller may pay `{1}` **up to three times**. Let **K** = number of successful payments
(0..3). Paying one or more times creates the reflexive **once** (CR 603.12a); the controller then
chooses **up to K** modes (cap = `min(K, mode_count=3)`, CR 700.2d), minimum 0. If K = 0 the
reflexive does nothing (no "do" occurred).

### Analogous trace (hard gate) — re-located, new base
The #4186 dynamic-modal-max spine, traced end-to-end:
`ModalChoice.dynamic_max_choices` (`ability.rs:12924`) → `modal_choice_for_player`
(`ability_utils.rs:543-576`, dynamic clamp `571-573`) → `WaitingFor::AbilityModeChoice`
(`game_state.rs:3607`) → `GameAction::SelectModes` (`actions.rs:333`) → `handle_ability_mode_choice`
(`engine_modes.rs:19`) → `build_chained_resolved` (`ability_utils.rs:201`) → `resolve_ability_chain`.
**Hawkeye reuses this entire spine** — it only feeds a *different QuantityExpr* into
`dynamic_max_choices`. The per-iteration optional template is the `kind_driven`/`member_driven`
branch in `resolve_chain_body` (`effects/mod.rs:5836-5865`).

### B-seam re-location (corrects MSH-E-PLAN.md's `drive_repeat_for_outermost` citation)
- `drive_repeat_for_outermost` (`effects/mod.rs:3972`) is gated by
  `repeat_for_outermost_with_scope_or_unless` (`effects/mod.rs:3962-3967`) which requires
  `player_scope.is_some() || unless_pay.is_some()`. **Hawkeye has neither ⇒ never enters this
  driver.** (Review B1 confirmed; the round-1 plan was wrong.)
- **The REAL path, all inside `resolve_chain_body` (`effects/mod.rs:4815-5923`):**
  1. Up-front optional gate `effects/mod.rs:5403-5448`: `if ability.optional &&
     !has_kind_driven_repeat && !has_member_driven_repeat_after_hydration` → fires ONE
     `WaitingFor::OptionalEffectChoice` (`:5441`), stashes `pending_optional_effect` (`:5428`),
     returns (`:5447`). Hawkeye (optional, no kind/member) ⇒ **one up-front yes/no**.
  2. Accept → `handle_optional_effect_choice` (`engine_payment_choices.rs:28`) →
     `resolve_optional_effect_decision` (`effects/mod.rs:1733-1807`): sets `optional=false` (`:1740`)
     + `optional_effect_performed=true` (`:1743`), re-enters `resolve_ability_chain` (`:1747`).
  3. Now optional is false → up-front gate skipped → the Fixed-count loop runs via the
     **`repeated_full_chain` branch** (`effects/mod.rs:5800-5922`):
     `repeated_full_chain = ability.repeat_for.is_some() && effective.sub_ability.is_some()`
     (`:5800-5801`); for `Fixed(3)`, `member_driven=false`, `kind_driven=false`, so each iteration
     hits `if repeated_full_chain` (`:5854`) → `resolve_ability_chain(full_chain_iteration)` (`:5859`)
     which **re-resolves PayCost AND the modal every iteration**; `return Ok(())` at `:5920-5922`.
- **B2 CONFIRMED:** the plain Fixed-count path has **no per-iteration optional handling**.
  Per-iteration `OptionalEffectChoice` exists only for `kind_driven`/`member_driven` (`:5836`
  clears `repeat_for`; `:5860-5865` resolves the optional iteration via `resolve_ability_chain`).
  Plain `repeat_for: Fixed(N)` applies optionality ONCE up front (`:5403`) then runs the inner unit
  N times mandatorily. Delivering "pay {1} up to three times" requires BOTH (a) the parser to put
  the dynamic cap on the modal (B4) AND (b) a **NEW Fixed-count per-iteration-optional + early-stop
  driver** modeled on `:5836-5865`.
- **B3 flag:** `cost_payment_failed_flag` is set by the PayCost handler (`pay.rs:40`, `233`, `257`)
  and at `effects/mod.rs:5005`; it is cleared per-iteration only inside the **player_scope** loop
  (`effects/mod.rs:5078`) — NOT in `repeated_full_chain`. The WhenYouDo eval is
  `effects/mod.rs:6885-6887`: `WhenYouDo => !(matches!(ability.effect, Effect::PayCost{..}) &&
  state.cost_payment_failed_flag)`.

### The four pieces

**B1 — New QuantityRef variant (gated, APPROVED).** Add
`QuantityRef::TimesCostPaidThisResolution`. `/add-engine-variant` gate, re-run on current inventory:
- *Stage 1 (existence):* `engine-inventory.json` `*Paid/*Count` set = `CostXPaid` (`ability.rs:4500`),
  `KickerCount` (`:4503`), `AdditionalCostPaymentCount` (`:4507`), `AdditionalCostPaymentCountFor`
  (`:4511`), `ConvokedCreatureCount` (`:4516`) — **all cast-time** tallies read from the source
  spell/object (CR 601.2 / 702.33 / 702.51). No resolution-local payment-count ref. → DOES_NOT_EXIST.
- *Stage 2 (parameterization filter):* the cast-time refs read object/cast tallies (announcement);
  this reads a **resolution-local** transient counter (CR 603.12a / 608.2c). Different CR section ⇒
  NOT a leaf-parameterization of the cast-time refs ⇒ EXTEND_OK (a sibling, not a refactor of them).
- *Stage 3 (categorical boundary):* the axis lies within one CR section (603.12a, resolution-local
  payment count). → WITHIN_SECTION. **APPROVED.**
- Registration points (lockstep; compiler enumerates exhaustive matches via `cargo check`):
  - `types/ability.rs` `enum QuantityRef` (`:3954`) — add the variant with
    `// CR 603.12a: number of times the controller paid the repeated optional cost during THIS
    ability's resolution (resolution-local; distinct from cast-time CostXPaid/KickerCount).`
  - `game/quantity.rs` — `resolve_quantity_with_ctx` value arm reads the GameState counter (B2);
    plus the scope/dependency classification arms (`quantity.rs:405`, `653`, `836` and the
    resolution-only predicate) so it is treated as resolution-scoped, never snapshotted/cached.
  - `game/coverage.rs` `quantity_ref_feature` (`:6297`) — classify `=> ("TimesCostPaidThisResolution",
    Handled)`.
  - `parser/oracle_modal.rs` — produce it from `"choose up to that many"` (B4).
  - serde derive is automatic; no card-data migration risk (new optional `Some` only on the new card).

**B2 — NEW Fixed-count per-iteration-optional repeated payment driver (FIRST-CLASS WORK).**
This is the core engine deliverable, not a side effect. In `resolve_chain_body`:
- Add a class predicate (private fn), e.g. `is_repeated_optional_payment(ability)` =
  `ability.optional && matches!(*ability.effect, Effect::PayCost{..}) &&
  matches!(ability.repeat_for, Some(QuantityExpr::Fixed{..})) &&
  ability.sub_ability.as_ref().is_some_and(|s| s.condition == Some(AbilityCondition::WhenYouDo))`.
  Mirror the existing `has_kind_driven_repeat`/`has_member_driven_repeat` predicates
  (`effects/mod.rs:3931/3945`).
- **Suppress the up-front gate for this class:** add `&& !is_repeated_optional_payment(ability)` to
  the `:5403` condition (exactly as `!has_kind_driven_repeat` / `!has_member_driven_repeat_after_hydration`
  already gate it). The "you may pay" is now per-iteration, not one up-front yes/no.
- **New driver branch** in the loop region (`:5800-5922`), taken when `is_repeated_optional_payment`
  (instead of the generic `repeated_full_chain` branch). Model: the `kind_driven`/`member_driven`
  per-iteration optional path (`:5836-5865`) plus the resumable stash (`pending_repeat_iteration`,
  `:5904-5914`). It must:
  (a) Add a resolution-scoped counter to `GameState` — `optional_cost_payments_this_resolution: u32`
      — cleared in the `depth == 0` prelude of `resolve_ability_chain` (`effects/mod.rs:4619`,
      next to the sibling `state.exiled_from_hand_this_resolution = 0` at `:4643`). `resolve_quantity(TimesCostPaidThisResolution)`
      reads it. *(Resolution-scope is REQUIRED: `modal_choice_for_player` calls `resolve_quantity(state,
      expr, player, source_id)` with no ability — `ability_utils.rs:572` — so K must live on `state`.)*
  (b) For each iteration `0..N` (N = `Fixed` bound): fire a per-iteration `WaitingFor::OptionalEffectChoice`
      for the PayCost-only unit (clone with `repeat_for=None`, `sub_ability=None`); reuse the stash
      machinery so the engine round-trips `DecideOptionalEffect` per iteration. On **Accept**: clear
      `cost_payment_failed_flag` *before* the payment (B3), resolve the PayCost effect, and if
      `!cost_payment_failed_flag` afterward, increment `optional_cost_payments_this_resolution`.
      On **Decline**: stop the loop early (CR: "up to" lets the player stop).
  (c) After the loop: if `K >= 1`, clear `cost_payment_failed_flag`, then resolve the reflexive
      `sub_ability` (the modal) **exactly once** via `resolve_ability_chain` (B3). If `K == 0`, skip
      the reflexive entirely (CR 603.12a — no "do" occurred). The continuation stash carries
      `{ remaining_iterations, reflexive_sub }` so the post-loop reflexive runs after the last
      `DecideOptionalEffect`.
- CR annotations: `// CR 603.12a: count successful repeated payments; a decline ends the repeated
  optional payment early and the reflexive triggers only once.` and `// CR 608.2c: reset the
  per-iteration cost-failure flag so an earlier decline can't suppress a later payment or the
  post-loop reflexive.`

**B3 — Reflexive resolves once + WhenYouDo gate (tied to the new driver, NOT the old site).**
The reflexive modal is resolved **once after** the payment loop by B2(c), gated on `K >= 1`. The
existing WhenYouDo eval (`effects/mod.rs:6885-6887`) keys on `Effect::PayCost && cost_payment_failed_flag`;
because the reflexive sub's own effect is the modal (not PayCost) and B2(c) clears the flag before
the single post-loop resolution, the eval returns true for K≥1 and the driver's explicit `K==0`
skip is the authority for the zero case. The implementer must verify the reflexive's WhenYouDo gate
along the new path resolves true only when K≥1 (add a `// CR 603.12a` guard at `:6885` keyed on
`optional_cost_payments_this_resolution >= 1` if the post-loop resolution path re-presents the
PayCost effect to the eval — otherwise the driver's K-gate suffices and `:6885` is unchanged for the
single-payment Guide-of-Souls class it already serves, tests `when_you_do_*` at
`effects/mod.rs:10619/10683`). **The discriminator that proves this is right is the "Explosive once,
not 2×K" test (B-iii) plus "decline iteration 2 after paying iteration 1 ⇒ K=1, reflexive STILL
fires" (B-i).**

**B4 — Parser (nom only) + modal wiring.**
- `parser/oracle_modal.rs` `parse_modal_choose_count` (`:1662` enum `ModalCountSpec`; dynamic arm at
  `:1713-1719`): add a sibling `alt()` arm recognizing `"choose up to that many"` (em-dash form
  `"choose up to that many —"` preferred), producing the dynamic cap bound to
  `QuantityRef::TimesCostPaidThisResolution`. **Idiomatic refactor (parameterize, don't proliferate):**
  replace `ModalCountSpec::DynamicCostX` with `ModalCountSpec::Dynamic { qty: QuantityRef }` (two
  dynamic siblings = a parameterization smell). The `count_spec` mapping at `oracle_modal.rs:600-609`
  becomes `ModalCountSpec::Dynamic { qty } => (0, usize::MAX, Some(QuantityExpr::Ref { qty }))`;
  `DynamicCostX`'s call site supplies `qty: CostXPaid`, the new arm supplies
  `qty: TimesCostPaidThisResolution`. This sets `min_choices=0` and the live cap automatically.
- No `contains()/find()` dispatch — the new arm is a `value()/tag()` sibling in the existing `alt()`.
- The repeat/optional structure (`repeat_for: Fixed(3)`, `optional: true`) and the WhenYouDo modal
  sub already parse on the base (measured). B4's only AST change is the modal cap (min 0 + dynamic
  ref). The runtime cap clamp is then handled unchanged by `modal_choice_for_player`
  (`ability_utils.rs:571-573`, `min(K, mode_count)` — CR 700.2d).
- Parser test: assert that Hawkeye's trigger's modal sub parses with `min_choices==0`,
  `mode_count==3`, and `dynamic_max_choices == Some(Ref(TimesCostPaidThisResolution))`.

### Registration-points checklist (B, per /add-engine-effect + /add-interactive-effect)
- types: `QuantityRef` variant (`ability.rs:3954`); `GameState.optional_cost_payments_this_resolution`
  field.
- parser: `oracle_modal.rs` (`"that many"` → dynamic ref + `ModalCountSpec::Dynamic` refactor). Nom only.
- resolver: `quantity.rs` (resolve new ref from the state counter + scope classification);
  `effects/mod.rs` new driver in `resolve_chain_body` (suppress `:5403` gate for the class; new
  per-iteration-optional loop + K counter + early-stop + flag clear + reflexive-once); depth==0
  clear (`:4619`).
- targeting: per-mode targets already handled (`build_target_slots_labelled`, `ability_utils.rs:386`);
  Net (CantBlock target creature) / Explosive (DealDamage target player) reuse existing slots. No change.
- interactive: **reuse** `WaitingFor::OptionalEffectChoice` + `DecideOptionalEffect`
  (`game_state.rs:3630`, `actions.rs:444`) per iteration, and `WaitingFor::AbilityModeChoice` +
  `SelectModes` (`game_state.rs:3607`, `actions.rs:333`) for the reflexive. Verify the per-iteration
  prompt re-enters K times via the continuation stash.
- MP filter: `acting_player`/`acting_players` (`game_state.rs:4542/4689`) already cover both
  `AbilityModeChoice` (`:4609`, in `acting_player`) and `OptionalEffectChoice` (`:4630`, in
  `acting_players`) — **no session.rs arm** (grep of `server-core/src/session.rs` for these variants
  = 0 hits; routing reuses `acting_player`/`acting_players`).
- AI: `candidates.rs` already generates `DecideOptionalEffect` yes/no (`:441/446`) and `SelectModes`
  combinations off the resolved `AbilityModeChoice` cap (`:1310/1372/1396`). Verify it offers decline
  across the K repeated iterations and reads cap K off the resolved `WaitingFor` (runtime resolves
  the dynamic max before emitting the choice — `modal_choice_for_player`).
- frontend: `ModeChoiceModal.tsx` + the optional-effect UI already render these; no new component.
- AI gate: resolution-path behavior change ⇒ run `cargo ai-gate` (CLAUDE.md).

### Pattern coverage (B)
Covers the **class** "pay {cost} up to N times during resolution; a reflexive scales with the number
of payments" (CR 603.12a) — not just Hawkeye. The `TimesCostPaidThisResolution` ref +
bounded-optional-repeat-with-count + reflexive-once driver are reusable for every future card of this
shape. The new driver also fixes the latent `repeat_for + optional` per-iteration flag-clear gap for
that whole class.

### Verification matrix (B) — runtime is the KEY (resolver-flagged card)

| # | Claim | Seam (new line) | Production entry | Discriminating revert-fail test | Hostile / negative |
|---|---|---|---|---|---|
| B-i | K captured from repeated payment; decline ends early | new driver + counter (`effects/mod.rs:5800-5922`, `:4619`) | tap Hawkeye, drive `DecideOptionalEffect`×K through `GameAction` | After 3 Accepts, `resolve_quantity(TimesCostPaidThisResolution)` (read through the modal cap) == 3; 2 Accepts then 1 Decline ⇒ K==2 and loop stops (no 3rd prompt); 0 Accepts ⇒ K==0. **Revert** the increment in B2(b) ⇒ K stays 0 ⇒ the K=3 case fails. | **Decline iteration 2 AFTER paying iteration 1** ⇒ K==1 and the reflexive STILL fires (proves the B3 per-iteration flag clear at the new site; revert that clear ⇒ the loop-terminating decline's flag suppresses the reflexive ⇒ fails). |
| B-ii | Modal cap = min(K, 3), min 0 | `modal_choice_for_player` (`ability_utils.rs:571-573`) + B4 parser | drive tap trigger → `AbilityModeChoice` | Pay 2 ⇒ `effective.max_choices == 2`, `min_choices == 0`; pay 3 ⇒ cap 3 (== mode_count). **Revert** the B4 dynamic-max wiring (modal back to fixed 1,1) ⇒ cap 1 ⇒ fails. | Cap never exceeds `mode_count`: bound N=3 == modes 3, so K>mode_count is impossible here; assert the `.min(mode_count)` clamp holds (revert the clamp at `ability_utils.rs:573` ⇒ if a future N>modes, cap overflows). |
| B-iii | Reflexive resolves exactly once | new driver post-loop reflexive (B2c) + WhenYouDo eval (`effects/mod.rs:6885`) | full resolution, pay K≥1, choose modes via `SelectModes` | Choosing Explosive once ⇒ **exactly 2 damage** to the target player, not 2×K. **Revert** B3 reflexive-once (keep the modal nested under the repeat at `:5854/5859`) ⇒ damage applied K times ⇒ fails. | **K==0** ⇒ the modal is NEVER offered and no mode resolves (the driver's K≥1 skip; revert the skip ⇒ a no-payment tap still prompts a modal ⇒ fails). |
| B-iv | Each mode resolves correctly | mode resolvers via `build_chained_resolved` (`ability_utils.rs:201`) | choose each mode via `SelectModes` | Net ⇒ target creature gains CantBlock until EOT; Explosive ⇒ target player −2 life; Boomerang ⇒ hand size net 0 (discard 1, draw 1). Each assertion reverts on dropping that mode's wiring. | Choosing fewer modes than K (K=3, choose 1) resolves only the chosen mode. |
| B-v | Coverage honest after B | A-2 detector + B4 parser | `card-data` resource | With B4 emitting `dynamic_max_choices:Some`, the A-2 detector no longer fires on Hawkeye (its `"modal":{` now carries `"dynamic_max_choices":{`) ⇒ Hawkeye `supported:true`. **Revert** B4 ⇒ detector fires ⇒ `false`. | Confirm Tranquil Frillback remains red (B does not touch its parse) and no other card flips. |

**Non-vacuity evidence:** every row names the exact code whose reversion flips the assertion. The
K∈{0,2,3} sweep + "Explosive once, not 2×K" + "decline iter2 ⇒ K=1 reflexive still fires" are the
discriminators that separate "modal driven by payment count, reflexive once" from BOTH the old
fixed-(1,1) behavior AND the misparse that repeats the modal per payment.

### Identity / Provenance Contract (B)
"choose up to that many" binds the modal cap to a **resolution-local** value:
source phrase → authority = count of successful optional `{1}` payments in **this** resolution →
bound at resolution time (live read via `resolve_quantity` when `modal_choice_for_player` builds the
cap, `ability_utils.rs:572`) → stored in `GameState.optional_cost_payments_this_resolution`, cleared
at the `depth==0` prelude (`effects/mod.rs:4619`) → consumed by `modal_choice_for_player` (clamped to
`mode_count`, CR 700.2d) → invalidated when the next top-level resolution begins (counter reset).
**Multi-authority hostile fixture:** two Hawkeye taps in the same turn (or another repeated-payment
source resolving between them) must NOT leak K — each resolution starts from a cleared counter; the
test resolves two tap triggers with different K and asserts each modal cap matches its own K.
Ruinous's `CostXPaid` provenance (cast X → `obj.cost_x_paid`, CR 107.3m) is pre-existing and already
tested (Walking Ballista, `engine.rs:7979`).

### Card-test harness notes (per /card-test)
Use `GameScenario` + `GameRunner::cast(...).resolve()`; reach the tap trigger via a real tap (attack
or a tapping cost), then drive `DecideOptionalEffect`×K and `SelectModes` through real `GameAction`s.
Do NOT hand-construct `WaitingFor` for the KEY claims (production-path requirement). Exercise the K=0
path (decline immediately) and the K≥1 "choose 0 modes" path. Assert via `CastOutcome`/state deltas
(life, hand size, CantBlock), not AST flags.

### Commits (B, two)
1. `fix(engine): reset cost-payment-failed flag per repeated optional payment iteration` — the
   general per-iteration flag clear, independently testable (B3 flag piece) if it can be split from
   the new driver; otherwise fold into commit 2.
2. `feat(engine): repeated optional payment count + reflexive modal (Hawkeye, Master Marksman)` — the
   `TimesCostPaidThisResolution` ref, the GameState counter, the new Fixed-count per-iteration-optional
   + early-stop driver, reflexive-once, the `ModalCountSpec::Dynamic` parser refactor + `"that many"`
   arm, and the coverage delta.

---

## Architectural sections (both sub-plans)

**Pattern Coverage.** A: every dynamic-modal-header card (line-counter) + every "modal parsed,
dynamic max dropped" misparse (detector). B: every "pay {cost} up to N times → scale a reflexive by
the count" card + the `repeat_for + optional` per-iteration flag fix. No single-card special cases.

**Building Blocks.** Reuse: `modal_choice_for_player`, `build_chained_resolved`,
`build_target_slots_labelled` (`ability_utils.rs`); `WaitingFor::OptionalEffectChoice`/`DecideOptionalEffect`,
`WaitingFor::AbilityModeChoice`/`SelectModes`; `Effect::PayCost` + `AbilityCondition::WhenYouDo`;
`resolve_quantity` (`quantity.rs`); the `kind_driven`/`member_driven` per-iteration optional template
+ `pending_repeat_iteration` stash (`effects/mod.rs:5836-5914`); the coverage `ast_json` introspection
+ `json_has_any` (`swallow_check.rs:1342`); the `count_effective_oracle_lines` test pattern
(`coverage.rs:11211`). New: one `QuantityRef` variant, one `GameState` counter, one swallow detector,
one parser arm — each justified above.

**Logic Placement.** Coverage classifier + honesty detector → `coverage.rs` / `swallow_check.rs`
(audit layer). Payment count, early-stop, flag reset, reflexive-once → engine resolver (`effects/`).
Quantity resolution → `quantity.rs`. Text→AST (min 0, dynamic cap ref) → parser (`oracle_modal.rs`).
Modal cap clamp → already in engine (`ability_utils.rs`). Frontend unchanged.

**Rust Idioms.** New `QuantityRef` is a typed variant (not a bool/sentinel); `ModalCountSpec` is
parameterized (`Dynamic { qty }`) rather than growing a second dynamic sibling. Exhaustive `match`
in `resolve_quantity`/`quantity_ref_feature` (compiler-enforced). Detector introspects typed
serialized fields, not verbatim Oracle strings. Parser composes a `value()/tag()` sibling in the
existing `alt()`.

**Nom Compliance.** Only B touches `parser/`. The `"that many"` arm is a new `alt()` sibling on the
`parse_modal_choose_count` dynamic combinator (`oracle_modal.rs:1713`); no `contains()/find()`
dispatch. `is_modal_header_line` and the swallow detector live in `game/` / `parser/swallow_check.rs`
(classifier/audit code, the files' established `contains` idiom, flagged `// allow-noncombinator`
like siblings) — not parser dispatch.

**Extension vs Creation.** A extends an existing classifier + adds a sibling detector (mirrors
`detect_dynamic_qty`). B extends the #4186 dynamic-modal-max spine with a new quantity source; the
only genuinely new primitives are the resolution-local count ref + counter + the Fixed-count
per-iteration-optional driver, justified by CR 603.12a's distinct categorical boundary.

**Variant Discoverability.** `engine-inventory.json` consulted; the `*Paid/*Count` QuantityRef set
is entirely cast-time. `/add-engine-variant` gate run for `TimesCostPaidThisResolution`
(existence → DOES_NOT_EXIST; filter → EXTEND_OK; boundary → WITHIN_SECTION). APPROVED.

**Analogous Trace.** A: `is_modal_header_line` → `count_effective_oracle_lines` →
`check_silent_drops` → `analyze_coverage`; detector mirrors `detect_dynamic_qty`
(`swallow_check.rs:1355`). B: #4186 dynamic-modal-max end-to-end (`ability.rs:12924` →
`ability_utils.rs:571` → `game_state.rs:3607` → `actions.rs:333` → `engine_modes.rs:19` →
`ability_utils.rs:201` → `resolve_ability_chain`), plus the per-iteration-optional template
(`effects/mod.rs:5836-5865`) and `Effect::PayCost`/`WhenYouDo` (`pay.rs`, `effects/mod.rs:6885`).

---

## Deviations / risks
1. **False-green avoidance is the core constraint.** Recognizing dynamic headers in the line-counter
   alone would green Hawkeye (and Tranquil Frillback) prematurely; the `Modal_DynamicMaxDropped`
   detector with the `"modal":{` gate is the mandatory honesty guard. A1 re-measured: zero green
   regressions across the full 10-card class.
2. **Detector breadth.** The detector also (correctly) keeps Bumi/Riku red (modal node, no dynamic
   max) — they are already red via `gap_count 1`; the detector makes the reason explicit and
   regresses no green card (verified by the supported-set diff in A-iv).
3. **B's new driver (B2) is the highest-risk item** and is scoped as first-class work, not an
   implied side effect. The continuation stash for the per-iteration `OptionalEffectChoice` round-trip
   is the trickiest part; model it on `pending_repeat_iteration` (`effects/mod.rs:5904`). The
   "Explosive once, not 2×K" (B-iii) and "decline iter2 ⇒ K=1 reflexive fires" (B-i) tests guard the
   reflexive-once + flag-clear correctness.
4. **B3 WhenYouDo eval:** decide during impl whether the post-loop reflexive path re-presents a
   PayCost effect to the `:6885` eval (then add a K≥1 guard there) or resolves the modal sub directly
   (then the driver's K-gate suffices and `:6885` is unchanged for its existing single-payment class).
   Either way the B-iii K==0 hostile test is the discriminator.
5. **Verification cadence (CLAUDE.md risk-scaled).**
   - **Sub-plan A** = parser/coverage-classifier only → `cargo fmt --all`, the new `coverage.rs` /
     `swallow_check.rs` unit tests, then let Tilt `card-data` regenerate `coverage-data.json` and
     confirm the supported-set delta. Do not block the small change on full clippy/test.
   - **Sub-plan B** = engine plumbing + resolution + AI → full gate: `cargo fmt --all`, then
     `./scripts/tilt-wait.sh clippy test-engine card-data` (+ `test-ai`), **`cargo ai-gate`** with the
     paired-seed report, and the B-i…B-v runtime tests before marking fixed. Do NOT run
     `cargo build/clippy/test` directly (target locks) — read Tilt resources.
6. **mtgish is out of scope** (dormant). Card-data regeneration is the implementer's responsibility
   via the `card-data` Tilt resource; the symlinked `data/*.json` in this worktree must not be
   hand-regenerated during planning.

---

## r2 review resolution (2026-06-27) — APPROVED with committed decisions (this section is authoritative for the implementer)

Adversarial r2 review verdict: **0 BLOCKER · 2 HIGH · 3 MED · 2 LOW** — no fundamental redesign. A1 BLOCKER genuinely fixed (gate fully discriminating, zero green regressions, cell-for-cell re-measured). The reviewer RESOLVED both hedges; the decisions below are now committed (lead-verified the two load-bearing ones against code). Implement these, NOT the earlier hedged text.

- **[HIGH-1 — B3 COMMITTED] Reflexive resolves via DIRECT sub-resolution at depth ≥ 1; `effects/mod.rs:6885` WhenYouDo gate is UNCHANGED.** LEAD-VERIFIED: the gate is `!(matches!(ability.effect, Effect::PayCost{..}) && state.cost_payment_failed_flag)` and `evaluate_condition` receives the ability that carries the condition. Hawkeye's reflexive sub has `effect = GenericEffect` (the modal wrapper), so `matches!(PayCost)` is false → gate returns `true` UNCONDITIONALLY (flag-independent). Therefore: the new driver resolves the reflexive `sub_ability` DIRECTLY (passes it as `ability` to `resolve_ability_chain`) **at depth ≥ 1**, and the driver's explicit **`K==0` skip is the SOLE authority** for the zero case. DO NOT modify `:6885`. DELETE the old deviation-#4 either/or. AVOID the alternative path (re-presenting the parent `PayCost`, evaluated at `:6264` with `ability=parent`).
- **[HIGH-2 — B-i test RE-TARGETED] The "revert flag-clear ⇒ reflexive suppressed" assertion is VACUOUS — replace it.** Measured: the flag is never set on a decline and never cleared on a successful payment (`pay.rs:241`), and B2b clears it before each payment, so it is already false at the reflexive; under HIGH-1 the reflexive's effect is `GenericEffect` so the flag never gates it. Re-target into TWO discriminating tests: (1) **reflexive discriminator = the K-gate** — `K==0 ⇒ reflexive skipped; K≥1 ⇒ reflexive fires` (revert the `K==0` skip ⇒ reflexive fires at K=0 ⇒ test fails); (2) **flag-clear discriminator = K-accounting** — a stale-true `cost_payment_failed_flag` carried from a prior iteration must be cleared before the next payment, else that payment's success is misread and K under-counts (revert the per-iteration clear-before-payment ⇒ K wrong on the "fail-then-succeed" sweep ⇒ test fails). Keep B-i's behavioral claim "decline iter 2 after paying iter 1 ⇒ K=1 AND reflexive still fires" (that is the K-gate test).
- **[MED-1 — B4 parser arm] Match Hawkeye's PERIOD/bare header, NOT em-dash.** LEAD-VERIFIED in `data/card-data.json`: Hawkeye's header is `"...choose up to that many."` (period; bullets on following lines) — there is NO em-dash. The new arm = `tag("choose up to that many")` mirroring the existing `DynamicCostX` `"choose up to x"` arm's matching, with a guard that EXCLUDES the non-modal selection clauses that also start this way (VERIFIED present: `"choose up to that many target creatures you control"`, `"...creatures tapped this way"`, and `"where X is"` redefinitions) — i.e. require the header to be terminated (period / clause end / followed by bullets), not followed by a noun phrase. DROP the "em-dash preferred" guidance for the PARSER arm. (The `is_modal_header_line` *line-counter* em-dash preference is harmless per LOW-2, but the parser arm must match the period form or it misses the only target card.)
- **[MED-2 — A2/A1 serde rationale] Correct the `modal` None-omission mechanism.** It is the `AbilityDefinitionRepr` proxy's `skip_serializing_if` at `ability.rs:13381` that omits `modal` when `None` — NOT field-level absence. The conclusion is unchanged: `"modal":{` substring presence in `ast_json` is an exact proxy for "a modal node was parsed"; the detector keys on it. Fix the comment/rationale to cite the proxy.
- **[MED-3 — B2 driver scope] The new driver is genuinely NEW continuation plumbing — scope it first-class.** Extend the stash/drain (`drain_pending_repeat_iteration` family) with per-iteration-optional-`PayCost` + `K` counter + once-after-loop-reflexive semantics; the existing drain has none of these. INVARIANT: the post-loop reflexive MUST resolve at **depth ≥ 1**, because the `depth==0` prelude (`effects/mod.rs:4619`/`4643`) zeroes the resolution-scoped `optional_cost_payments_this_resolution` counter before `modal_choice_for_player` reads K (`ability_utils.rs:572`). Realizable, no blocker.
- **[LOW-1] Drop `Copy` from `ModalCountSpec`** when `DynamicCostX → Dynamic{qty}` lands (`QuantityRef` is not `Copy`). Add to the registration checklist.
- **[LOW-2] Reconcile the A-1 em-dash narrative + one stale line cite** (cosmetic; the line-counter em-dash preference is harmless, see MED-1 for the parser-arm distinction).

Cadence (unchanged): A = parser/coverage-classifier (fmt + parser gate + targeted coverage asserts, let Tilt confirm card-data/test-engine); B = full engine+AI gate incl. `cargo ai-gate`. Ship A first (independently committable), then B. Base = `wt-msh-f` @ aa4e88ec4 (v0.8.0 + MSH-F).
