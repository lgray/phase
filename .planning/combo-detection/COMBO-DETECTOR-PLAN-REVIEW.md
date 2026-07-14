# Review of `COMBO-DETECTOR-PLAN.md` — **VERDICT: REJECT**

**2026-07-14** · Framework: `.claude/skills/review-engine-plan/SKILL.md` (all required checks), run per the
`/engine-implementer` Step-2 plan-review gate. Reviewer: isolated agent, fresh context.
**All code measured against `main` @ `efc76ca1b`, read-only.** (Tilt was up; no `cargo` was run in the main
checkout, no `crates/` file was edited.)

> **Scope note.** The plan's **RULES layer is settled and correct** — a user directive, and the reviewer
> independently confirmed every CR citation resolves verbatim in `docs/MagicCompRules.txt`
> (732.2a `:6372` + Example `:6373` · 400.2 `:1935` · 113.6 `:771` · 104.4b `:366` · 732.1b `:6366` ·
> 732.1c `:6368` · 704.5a `:5492` · 104.2a `:330` · 800.4a `:6408` · 732.4 `:6383`).
> **This review attacks CODE claims and implementability only.** The prior six planning docs are STALE and were
> excluded as a baseline — they may be rules-wrong as well as code-wrong.

**Code layer:** 2 wrong file paths · 7 WRONG citation rows · 4 DRIFTED · 1 PARTIAL · 1 UNVERIFIED.

---

## Team-lead independent verification

Every load-bearing citation below was **re-measured by team-lead against `main` @ `efc76ca1b`** before being
accepted. This workstream has produced 19 documented errors and **every one was a code claim asserted from
memory**; a review is not exempt from its own prime directive.

| Reviewer claim | Measured |
|---|---|
| `if !def.modifications.is_empty() { return true; }` — the blanket that vetoes Presence of Gond | ✅ **CONFIRMED** `analysis/resource.rs:1539` |
| `Effect::Token { .. } => Axes::CONSERVATIVE` | ✅ **CONFIRMED** `game/ability_scan.rs:447` |
| `Effect::Mana { .. } => Axes::CONSERVATIVE` ⇒ the Gaea's Cradle guard is **vacuous** | ✅ **CONFIRMED** `game/ability_scan.rs:862` |
| The engine's own doc comment names the missing building block | ✅ **CONFIRMED** `analysis/resource.rs:1452-55`, verbatim: *"default-CONSERVATIVE: **no `scan_continuous_modification` walker exists**"* — and it enumerates **seven** scans, not four |
| `TargetFilter::Typed(tf) => Axes { event: true, sibling: true, .. }` | ✅ **CONFIRMED** `game/ability_scan.rs:2418-21` |
| `LoopDetectionMode::On` ships in the `combo-verify` binary | ✅ **CONFIRMED** `analysis/corpus.rs:2039` |
| An over-edit guard whose doc *is* P3's revert-probe | ✅ **CONFIRMED** `analysis/resource.rs:3926` `event_and_sibling_axes_unchanged_for_typed` |
| `object_functions` is **not** a zone-of-function authority — returns `true` for a **library** card | ✅ **CONFIRMED** `game/functioning_abilities.rs:108-116` (phased-out + Command only) |
| The zone-filtering exemplar to mirror | ✅ **CONFIRMED** `game/triggers.rs:423-435`, filters via `trigger_definition_functions_in_zone` at `:434` |
| The cited "unbounded cover" is offline-only dead code | ✅ **CONFIRMED** `analysis/loop_check.rs:221`: *"This is the OFFLINE classifier only — no live/reducer path."* |

---

## ⛔ BLOCKERS

### B1 — The diagnosis is **insufficient**. P2 + P3 do not make the canary green.

CR 732.2a's own worked board (Presence of Gond + Intruder Alarm, **all battlefield**) was traced through every
limb of `fire_time_conditions_read_growing_class` (`analysis/resource.rs:1457`). **Three independent vetoes.
The plan addresses one.**

| # | Veto | Site | Cleared by the plan? |
|---|---|---|---|
| 1 | Intruder Alarm trigger → `SetTapState { target: Typed(Creature) }` → `sibling: true` | `game/ability_scan.rs:2420` | ✅ P3 |
| 2 | Enchanted creature's granted `{T}: Create token` → blanket | `game/ability_scan.rs:447` `Effect::Token{..} => CONSERVATIVE` | ❌ **P3 never mentions `Effect::Token`. P2 irrelevant — battlefield-resident.** |
| 3 | Presence of Gond's own `static_abilities[0].modifications = [GrantAbility]` (measured in `data/card-data.json`) | `analysis/resource.rs:1539` — a raw `!is_empty()` blanket that **never reads `sibling`**, firing on a **visible, functioning, battlefield** permanent | ❌ **Neither P2 nor P3 touches it.** |

**The code names the missing building block itself** (`resource.rs:1452-55`): *"no `scan_continuous_modification`
walker exists."* **That is the real third root cause — unplanned, unscoped, untested, unbudgeted.**

### B2 — Sufficiency answered: **P2 unblocks NOTHING.**

All three vetoes are **battlefield-resident**. P2's header — *"(the rules fix — and the reachability fix)"* — is
**FALSE**. The CR 400.2 hidden-zone leak is **real** and all three limbs confirmed: scan (1) via
`active_trigger_definitions` (`game/functioning_abilities.rs:391`; Command special-case `:405-408`, then a bare
`true` at `:409` — **no zone filter**); scan (3) via `active_replacements` (`:446-459`), filtered only by
`object_functions` (`:108-116` — **returns `true` for a library card**); scan (4) all-zones (`resource.rs:1527`).

⇒ **It is a rules fix, not a reachability fix. The plan's causal story AND its phase ordering are both wrong.**

### B3 — P1's GREEN is a false confirmation.

Stubbing the **whole** firewall bypasses all three vetoes ⇒ GREEN ⇒ the plan concludes *"the covers are the only
blocker, proceed to P2"* ⇒ **P2 then fixes the one limb that was not responsible.** The probe cannot answer the
only question it exists to answer. Needs a **per-limb** probe (and within scan (4), the `condition` branch
separately from `!modifications.is_empty()`). And there is still **no RED plan** beyond *"instrument it."*

### B4 — P3's mandatory negative (Gaea's Cradle) is **VACUOUS**.

It fails closed via `game/ability_scan.rs:862` `Effect::Mana { .. } => Axes::CONSERVATIVE` — a blanket that
**never descends to the count** (`ManaProduction.count: QuantityExpr`, `types/ability.rs:1689-1701`). **It would
still fail closed if the `sibling` axis were deleted entirely.** It cannot discriminate.

> ### ⛔ CORRECTION — this review made a code claim from memory, and it was WRONG.
> The paragraph above originally continued: *"Worse: **Gaea's Cradle is absent from `card-data.json`** — the
> criterion cannot be run as a real card."* **That is FALSE.** Caught by the planner, then **re-measured by
> team-lead**: `jq 'has("gaea'\''s cradle")' data/card-data.json` → **`true`**. The root object is keyed by
> **lowercase** card name; an **exact-case** probe (`has("Gaea's Cradle")`) returns `false`, and that is what the
> reviewer ran.
>
> **Gaea's Cradle IS available as a real-card fixture.** The **vacuity** finding (the `Effect::Mana` blanket at
> `:862`) **stands and is confirmed** — only the *"not in the corpus"* aggravator was wrong.
>
> **The lesson is the workstream's own prime directive, and this review was not exempt from it:** *every one of the
> 19 prior errors was a code claim asserted from memory, and every one was caught the same way — someone
> re-measured.* **Error #20 was committed by the review that exists to catch them.**

The plan's own warning (*"a flip that un-rejects BOTH is a HOLE in the catastrophic direction"*) is **more right
than it knows** — that danger lives at `:862`, not at the filter arm it points to.

### B5 — P3 collides with an over-edit guard the plan never mentions.

`analysis/resource.rs:3926` `event_and_sibling_axes_unchanged_for_typed`, whose doc **literally states the
revert-probe for P3's flip** (*"Revert-probe: setting the arm's `event`/`sibling` to `false` flips these."*).
**P3 turns it RED.** Also unstated: `sibling` has a **live second consumer** — `game/triggers.rs:3893-95`
(CR 603.3b trigger auto-resolve). Flipping **only** `sibling` at `:2420` is safe (that gate is `!event &&
!sibling` and `event` stays `true` at `:2419`) — but the plan never commits to touching `sibling` only, and the
comment at `:2416` treats the two as a pair.

### B6 — P4 is **not a refactor**.

*"`match_flow.rs:669`/`:744` are inside `#[cfg(test)]`"* is **CONFIRMED** (`mod tests` at `:360`/`:361`). But
*"those are the only production call sites"* is **FALSE**:

- **`analysis/corpus.rs:2039`** — `state.loop_detection = LoopDetectionMode::On;` under
  `#[cfg(any(test, feature = "combo-verify"))]` ⇒ **ships in the `cargo combo-verify` binary** (`Cargo.toml:62-64`).
  Deleting `On` breaks it.
- `#[serde(tag = "type")]` (`types/game_state.rs:5941`) ⇒ wire form `{"type":"On"}`, crossing the **WS protocol**
  (`server-core/protocol.rs:124`/`:392`/`:428`), the **WASM bridge** (`engine-wasm/lib.rs:642-46` →
  `unwrap_or_else(|_| default())` ⇒ **silently degrades to `Off`, no error**), **saved games**
  (`game_state.rs:7508`), and **localStorage** (`multiplayerStore.ts:175`/`:192` → `HostSetup.tsx:227`).
- **Two live UI toggles** (`HostSetup.tsx:542-49`, `GameSetupPage.tsx:553-66`), a `?loop=on` URL param, 7 locale keys.
- 5 tests need **re-authoring, not deleting** (`corpus_tests.rs:1478-95` needs **two distinct modes** to be
  non-vacuous; `triggers.rs:23198`; `match_config.rs:89`/`:97`; `loop_shortcut.rs:338`; `pr65_growing_cascade_win.rs:111`).
- **The premise is partly wrong.** The object-growth bridge (`engine.rs:441-460`) gates on `.samples()`
  (`= On | Interactive`), **not** on the mode match — so **`On` already OFFERS on that path.** The
  rules-wrongness is confined to the **drain path** (`engine.rs:356-429`), which auto-wins without an offer.

### B7 — §2's "DO NOT REBUILD" citation table is ~50% wrong.

The table is the plan's instruction to an implementer. A wrong row sends them to the wrong code — or asserts a
stage exists when it does not.

- The file is **`game/ability_scan.rs`**, not `analysis/`.
- **The determinism gate is INVERTED.** `ability_scan.rs:4407` is `effect_is_randomness_bearing`, whose own doc
  at `:4406` says *"the **static**, compile-time-exhaustive half."* **Both** cited gates are static. The real
  runtime backstop is **uncited**: `engine.rs:1713` (rng word-position check). §6 says *"⛔ don't fix — exists
  twice"* while pointing at the static one **twice**.
- **Both "unbounded cover" citations are OFFLINE-ONLY DEAD CODE.** `resource.rs:924` and `:1326` are called only
  from `detect_loop` (`loop_check.rs:223`/`:230`), which has **no live reducer caller** (`loop_check.rs:221`).
  **The LIVE cover is `resource.rs:1095` `loop_states_cover_modulo_fodder_growth`** (called from
  `engine.rs:1732`) — **never cited anywhere in the plan.**
- `resource.rs:784` is the **ω/stack-growth** cover, not counter-growth (that is `:1326`).
- `engine.rs:450`/`:537` are **reads**, not capture (real capture: `casting_costs.rs:6795`,
  `game_state.rs:10581`). `engine.rs:536` is the **Path B** gate, not Path A. `loop_check.rs:83` is
  `pub enum WinKind {` (`Advantage` is at `:107`). `decision_template.rs:203` is the **field** (the enum is at
  `:281`). `actions.rs:834` is **Declare only** (`Respond` `:841`, `Decline` `:848`).

> **CREDIT — Stage 2 is CONFIRMED as characterized:** a real replay on a clone (`SimulationProbeGuard`,
> `state.clone()`, 2 iterations / 3 settle frames, `engine.rs:1688-96`; `drive_recast_iteration` `:1451` really
> re-applies `CastSpell` via `apply_action` `:1469`). Stage-5 CR 800.4a re-validation is real (`:860-878`).
> ## ⇒ **The plan's headline conclusion — "this is NOT a capability problem" — SURVIVES. The citations offered to prove it do not.**

### B8 — It is a strategy memo, not a plan.

Fails `review-engine-plan` checks **1, 2, 3, 9, 11**. No claim-to-test map. **Not one named test function or
file. No revert-failing assertion for any phase.** No production entry point named — the live path runs through
`apply()` → `WaitingFor::LoopShortcut` → `GameAction`, so **helper-only tests are barred by check 9.** **No
file-by-file change set for P2/P3/P4.** No analogous-feature trace. **None** of `/engine-planner` Step 4's eight
mandatory sections are present. **An implementer literally cannot open a file.**

---

## Material gaps

- **M1 — "four scans" → there are SEVEN** (1, 2, 3, 4, 5, 5b, 6). The plan's *selection* of (1)(3)(4) as the
  hidden-zone leaks is **CORRECT** — scan 5b (`resource.rs:1557`) is already zone-filtered internally via
  `granted_keyword_triggers_in_zone` → `trigger_definition_functions_in_zone` (`triggers.rs:434`) — but the
  **count is wrong** and scans (5)/(5b)/(6) are never mentioned.
- **M2 — scan (6) (`resource.rs:1580-88`) is an unaddressed blanket.** ANY non-empty `delayed_triggers` /
  `deferred_triggers` / `pending_trigger` / `epic_effects` ⇒ veto. **Any real Commander board with one live
  delayed trigger dies here.** §6's *"do not relax `GameState::PartialEq`'s `delayed_triggers` conjunct"* is a
  **different mechanism** — an implementer **will** conflate them. Must be disambiguated explicitly.
- **M3 — "84 sites" → 80 real**, and **the model is wrong.** (54 `Axes::CONSERVATIVE` incl. 3 in comments ⇒ 51;
  30 `sibling: true` incl. the `CONSERVATIVE` definition itself at `:133` ⇒ 29.) **They are NOT disjoint:
  `Axes::CONSERVATIVE` *contains* `sibling: true`** (`:131-135`). A 3-axis **blanket** (the walk does **not**
  descend) and a 1-axis **targeted literal** (the walk **does** descend) require **opposite** treatment — and
  **P3's Gaea's Cradle guarantee depends on a blanket staying put.** *"84 clicks of a one-sided ratchet"* is the
  wrong model and **invites a uniform sweep — the catastrophic direction.**
- **M4 — P3 is mis-sized in BOTH directions.** **28 of the 29** real `sibling: true` literals **already** encode
  *"counts a mutable set"* (20 in `scan_quantity_ref`, 4 `trigger_condition`, 3 `static_condition`, 1
  `player_filter`). **Only `:2420` names-without-counting ⇒ the literal half is a ONE-LINE change.** The
  **blanket** half (`Effect::Token` `:447`, `Effect::Mana` `:862`, the missing `scan_continuous_modification`) is
  far larger and **completely unscoped**.

---

## ✅ Bank this — P2's prescription is implementable; it just names nothing

P2 says: *"⛔ CALL the engine's runtime functioning authority. DO NOT hand-roll a zone list."* **Correct, and the
authorities exist:**

- `trigger_definition_functions_in_zone(def, zone)` — **`game/triggers.rs:1057`**
- `static_functions_in_zone(obj, def)` — **`game/functioning_abilities.rs:187`**
- **Exemplar to mirror:** `granted_keyword_triggers_in_zone` (`game/triggers.rs:423-435`) **already does exactly
  this** at `:434` — which is precisely **why scan (5b) is not a leak while (1)/(3)/(4) are.** Trace it and P2
  writes itself.

> ### ⚠️ TRAP THE REVISED PLAN MUST BLACKLIST BY NAME
> An implementer **will** reach for **`object_functions`** (`game/functioning_abilities.rs:108`) because scan (3)
> already uses it — and **it is NOT a zone-of-function authority** (phased-out + Command only; **returns `true`
> for a library card**). **Calling it reintroduces the exact bug.**

Also correct and worth keeping: the caution that `active_trigger_definitions` is a **shared** authority (the live
pipeline has its own gate at `triggers.rs:1040`) ⇒ **fix the firewall's scope, not that function's contract.**

---

## Soundness direction — §6's risk table is wrong

**§6's core claim — *"a coarse relation may REJECT, never ACCEPT"* — is RIGHT. Preserve it verbatim.** The table
beneath it is not:

| Phase | Plan says | Measured |
|---|---|---|
| **P1** | zero-risk probe | **Not artifact-free** — its conclusion is the sole input to a decision rule that is unsound (B3). |
| **P2** | moves toward ACCEPT | Moves toward ACCEPT **by ZERO** on any real board (B2). |
| **P3** | moves toward ACCEPT; highest risk | Correct — but **blast radius understated** (live consumer `triggers.rs:3894`). |
| **P4** | pure refactor, safe | Does not relax a rejection, so the *soundness* claim survives — **but it is not neutral.** It breaks a shipped binary, the WS protocol, the WASM bridge (**silently → `Off`**), saved games, and localStorage. **§6 marks it safe-to-not-double-review; it is the phase most likely to break users.** |

---

## Residual assumptions (NOT discharged)

1. **The engine was not run.** B1 is proven **statically**: measured card-data AST → measured scan arms → `Axes`
   → `.sibling` → `return true`. Every link cited. No runtime fixture (a cold build is ~30 min; the static chain
   was judged conclusive).
2. **One B1 link is INFERRED** — that the granted `{T}: Create token` appears in `obj.abilities` on the flushed
   current (veto #2). Inferred from the gates' own comment (`resource.rs:961-62`: *"FLUSHED current so
   layer-derived P/T / **abilities** / keywords are realized"*). **B1 does not depend on it** — veto #3
   (Presence of Gond's own non-empty `modifications`) is unconditional and needs no layer realization.
   **B1 stands regardless.**
3. *"~40 assertions"* (`resource.rs:924`) — **UNVERIFIED**.
4. **Per-site reachability of the 51 `CONSERVATIVE` blankets** — argued broadly reachable via scan (2),
   **not enumerated site by site.**
5. The full `active_trigger_definitions` call-site audit **that P2 itself asks for** was not run.
6. **P1's likely GREEN (B3) is a prediction, not a measurement.**

---

## One-sentence version

> §3's root cause is **real, but it is one of at least three — and it is the only one that is not on the
> battlefield.** The plan's own canary, **CR 732.2a's worked example, still fails closed after P2 and P3**,
> because Presence of Gond's `modifications: [GrantAbility]` trips a raw `!is_empty()` blanket
> (`analysis/resource.rs:1539`) that **no phase touches** — and the engine's own comment names the building block
> nobody has built: **`scan_continuous_modification`**.

---

**Successor:** `COMBO-DETECTOR-PLAN-REVISED.md`.
