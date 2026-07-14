# Combo detector — the LIVE plan

**2026-07-14 · Revision 4.** Supersedes `COMBO-DETECTOR-LIVE-PLAN.rev3.md` (REJECTed), which it **absorbs**.
**Code truth:** `phase-rs-workdir @ efc76ca1b` (`main`). **Laboratory:** `combo-probe-wt @ efc76ca1b`.

> ## ⭐ **REV 3 WAS REJECTED BECAUSE IT WROTE CODE FOR SITES IT NEVER OPENED.**
> **Rev 4's rule: every structural claim below is a MEASUREMENT I personally ran.** Type enumerations come
> from the **AST** (`ast-grep`), not a `sed` range. Every behavioral claim was **instrumented and run**. Every
> discriminator was **REVERT-PROBED** — the guarded term deleted, the assertion required to **FLIP TO FAIL**.
> Anything I did not run is listed as **UNVERIFIED** in §10.
>
> **Rev 3's five blockers are all discharged, and U3 is discharged. Both headline results are POSITIVE:**
> - ⭐⭐ **THE CANARY OFFERS** (B1) — and its negative twin is **measurably non-vacuous**.
> - ⭐⭐ **U3's `projected` term is LOAD-BEARING** — the revert-probe **flips to FAIL**.
> - ⭐⭐ **B3's live CR 603.3b regression is ELIMINATED** — `Off` is byte-identical, **measured**.

---

## 0. What we are implementing *(preserved from Rev 1/2/3 — unchanged, still correct)*

**Shortcutting a loop is OPTIONAL.** CR 732.2a (`docs/MagicCompRules.txt:6372`): the player *"**may** suggest a
shortcut."*

> ## ⇒ **THE DETECTOR STAYS OPT-IN.** `LoopDetectionMode::Off` is the "we are not shortcutting loops in this
> game" setting and **must remain the default**.

**USER DIRECTIVE (binding):** `LoopDetectionMode` keeps all three variants `{Off, On, Interactive}` and both UI
toggles. **P5 is doc-comment-only. Not re-opened.**

**PRESERVED SOUNDNESS RULE (verbatim):** *"A coarse relation may **REJECT**, never **ACCEPT**."* A coarse relation
⇒ a false certificate ⇒ **a real game ends wrongly.** Too fine ⇒ a missed offer ⇒ **safe.**

> ### ⭐ The omission that is the whole design — and it is correct
> **The spec says "repeat the ACTIONS." It never says the state must return to where it started.** CR 732.2a's own
> worked example — Presence of Gond + Intruder Alarm (`docs/MagicCompRules.txt:6373`) — **ADDS A TOKEN EVERY
> ITERATION.** Its state provably never recurs, and the rules shortcut it a million times.

### 0.1 ⛔ A REFINEMENT OF THE "NO WILDCARD" MANDATE — read this before writing any walker arm

The brief says *"a `_ =>` or `..` in any scan the firewall consumes is a FALSE-CERTIFICATE HOLE."* **That is true
only when the wildcard's value is the ACCEPTING value.** Precisely:

| Wildcard maps to | Direction | Verdict |
|---|---|---|
| `Axes::NONE` / `false` ("reads nothing") | **ACCEPT** | ⛔⛔ **FORBIDDEN.** A new variant is silently certified inert. |
| `Axes::CONSERVATIVE` / `true` ("might read") | **REJECT** | ✅ fail-closed. Safe, merely imprecise. |

This is why the shipped `modification_grants_growing_cost_keyword` (`ability_scan.rs:4080`) may use `_ => false`
for **its** question and we may not for **ours**. **Rev 4 mandates exhaustive, no-wildcard matches anyway** wherever
practical, because that makes `rustc` enumerate the type for us — *a measurement, not a trace* — and turns a missed
variant into a **compile error** instead of a silent wrong answer.

---

## 1. §1 — THE REV-4 MEASUREMENTS

All run in `combo-probe-wt` (warm target, no Tilt contention). `cargo fmt --all` run before every reported result.

### 1.1 ⭐⭐ M7 — **B1 IS DEAD, AND THE FIX MAKES THE CANARY OFFER.**

Rev 3 put the capture in the `ActivateAbility` handler behind `state.battlefield.len() > battlefield_len_before`.
**An activated ability only goes on the STACK at that beat (CR 602.2a, `docs/MagicCompRules.txt:2529`)** — the token
appears on **RESOLUTION**, a later beat. The handler is `game/engine.rs:3220-3286` (`main`); the non-mana branch
dispatches to `casting::handle_activate_ability` at **`engine.rs:3286`**.

**The fix is to MIRROR THE RECAST ARM PROPERLY.** The recast's gate (`game/casting_costs.rs:6789`, `:6795`) is
**STATIC** — `matches!(ability.effect, Effect::Token { .. })` — *not* a runtime battlefield delta. Rev 3's claim that
its gate was *"the same settle discipline as the recast"* is **FALSE**.

**✅ MEASURED** (`probe_rev4.rs::rev4_b1_canary_offers`, `rev4_negative_twin_arms_capture_but_does_not_offer`):

| | bf at the `ActivateAbility` beat | **Rev 3's gate** `bf.len() > before` | **Rev 4 STATIC capture** | **OFFER** |
|---|---|---|---|---|
| **canary** (Gond + Intruder Alarm) | `3 → 3` (stack=1) | ⛔ **`false`** — STRUCTURALLY DEAD | ✅ **`true`** | ⭐ **`true`** |
| **negative twin** (Gond, NO untapper) | `2 → 2` (stack=1) | `false` | ✅ **`true`** | ✅ **`false`** |

```
===== B1+: canary (Gond + Intruder Alarm) =====
  battlefield BEFORE ActivateAbility = 3
  battlefield AFTER  ActivateAbility = 3   (stack=1)
  ⛔ Rev3's DEAD gate `bf.len() > before` = false
  ✅ Rev4 STATIC capture armed          = true
  ⇒ OFFER (WaitingFor::LoopShortcut)    = true
     certificate.unbounded = [TokensCreated]
     certificate.win_kind  = Advantage
     certificate.mandatory = false
```

> ## ⇒ **ACCEPTANCE CRITERION #1 IS REACHABLE AND MET.** `WaitingFor::LoopShortcut`, `unbounded=[TokensCreated]`.

#### ⭐ The precise diagnosis — and which alternative I did NOT build

Rev 3's error is best stated **not** as *"captured at the wrong beat"* but as:

> ## **"INVENTED A RUNTIME BATTLEFIELD-LENGTH DELTA WHERE THE EXISTING PATTERN USES A STATIC EFFECT PREDICATE."**

The recast arm it claimed to mirror uses **`matches!(ability.effect, Effect::Token{..})`** — **no runtime delta
anywhere**. The activated-ability **dual** of that predicate is `collect_effects(obj.abilities[i])` ∋ `Effect::Token`,
which arms at the `ActivateAbility` beat, needs **no resolution seam**, and is consistent with the shipped arm.

**⇒ A resolution-beat capture is UNNECESSARY, and I did not build one.** The evidence is positive, not an appeal to
parsimony: **the static predicate is measurably SUFFICIENT — it carries the canary all the way to an OFFER (M7) and
still rejects the negative twin (M8).** There is no residual behavior for a resolution-seam capture to add, so building
one would add a seam, a beat, and a survival question for **zero** measured gain. *(Honest scope: I therefore did NOT
build and race a resolution-beat variant — there was nothing left for it to fix. **UNVERIFIED — U11.**)*

### 1.2 ⭐⭐ M8 — **THE NEGATIVE TWIN IS *NOT* VACUOUS. MEASURED.**

The reviewer ruled `activation_loop_without_untapper_does_not_offer` **VACUOUS** — correctly, **given B1** (nothing
ever offered, so the negative passed for free). **With B1 fixed, it discriminates,** and I measured *why*:

> ## **The negative twin ARMS THE CAPTURE (`armed = true`) — the static predicate is IDENTICAL — and STILL DOES NOT
> OFFER.** The rejection therefore happens at the **DRIVE** (the 2nd `ActivateAbility` on the clone is illegal — the
> bear is tapped and nothing untaps it ⇒ `Err(RecastAbort)`), **NOT at an upstream conjunct.**

The test **asserts `armed == true`** precisely so that a future regression that kills the capture cannot make this
negative pass for the wrong reason. **That assertion is the non-vacuity guard.**

### 1.3 ⭐⭐⭐ M9 — **U3 DISCHARGED: THE `projected` TERM IS LOAD-BEARING. REVERT-PROBE FLIPS TO FAIL.**

Rev 3's own words: *"the single most safety-critical unmeasured claim in this plan."* **It is now measured.**

**Fixture** (`analysis/resource.rs` `#[cfg(test)]`, `projected_reading_modification_still_vetoes_the_firewall`): a
battlefield static with `modifications: [SetDynamicPower { value: Ref(LifeTotal{Controller}) }]` and **no
condition** — so the modification is the **sole** read surface.

**⭐ AXIS ISOLATION — this is what makes the probe CONCLUSIVE rather than confounded.** Measured on `main`'s walker:

| node | `event` | `sibling` | `projected` | where |
|---|---|---|---|---|
| `QuantityRef::LifeTotal { player }` | `false` | **`false`** | **`true`** | `ability_scan.rs:1580-1586` |
| `QuantityRef::ObjectCount { filter }` | `false` | **`true`** | `false` | `ability_scan.rs:1603-1610` |

The test **asserts both axes** (`sibling == false`, `projected == true`) *before* asserting the veto. So the surviving
`sibling` term **cannot** back-stop the veto — the probe measures the `projected` term and nothing else.

| run | result |
|---|---|
| **positive** (veto = `sibling \|\| projected`) | ✅ `... ok` |
| ⛔ **REVERT-PROBE** — `\|\| ...reads_projected_resource(m)` **DELETED** | ⭐ **`FAILED`** — *"U3: a PROJECTED-reading modification MUST veto the firewall"* |

> ## ⇒ **VERDICT: LOAD-BEARING. The veto MUST be `sibling || projected`.** `fire_time_conditions_read_projected_resource`
> (`resource.rs:2152`) scans a static's **`condition` only** — it has **no `modifications` scan** — so the `:1539`
> sibling blanket is *incidentally* the only thing protecting the object/fodder covers from a projected-reading
> modification. Vetoing on `sibling` alone **opens a real false-certificate hole.** **P2-d ships with both axes.**
>
> ⚠️ **CONFOUND WARNING FOR THE IMPLEMENTER.** This probe is only conclusive because `scan_continuous_modification`
> **descends** `SetDynamicPower` into `scan_quantity_expr`. Under a `_ => CONSERVATIVE` walker the modification
> classifies `sibling=true` **as well**, the surviving term back-stops the veto, and **the revert-probe silently
> passes — proving nothing.** The two axis-isolation asserts are **not decoration; they are the probe.**

### 1.4 ⭐⭐⭐ M10 — **B3 DISCHARGED: `Off` IS BYTE-IDENTICAL. THE LIVE CR 603.3b REGRESSION IS ELIMINATED.**

`ability_scan`'s `Axes` is a **SHARED AUTHORITY**. `game/triggers.rs:3894-3895` computes the **LIVE** CR 603.3b
trigger-ordering gate from it:
```rust
let c2_order_independent = !ability_scan::ability_uses_event_context(&reference)
                        && !ability_scan::ability_reads_sibling_mutable(&reference);
```
**There is no `loop_detection` gate anywhere near it.** Rev 3's P2-b made the blanket precise **in place**, flipping
`c2` `false → true` for token-bodied triggers ⇒ **the engine AUTO-ORDERS instead of PROMPTING, in every game,
including `loop_detection == Off`.** That violates **#4603**.

> ## ⭐⭐ **THE SMOKING GUN IS THE REPO'S OWN TEST NAME.**
> **`game/triggers.rs:23237` — `fn pr625_c2_distinct_event_auto_orders_even_when_loop_detection_off()`** ✅ *(verified
> to exist at that exact line)*. **The repo already ships a test asserting that the C2 trigger-ordering gate operates
> WHEN `loop_detection` IS OFF.** That is direct, in-repo proof that the CR 603.3b path is **loop-detection-INDEPENDENT**
> — so **any** change to `ability_scan`'s `Axes` for `Effect::Token` / `Effect::Mana` changes live trigger prompting in
> **detector-OFF games**. It is the strongest available evidence that the firewall and the ordering gate **must stop
> sharing one answer.**
>
> ✅ **MEASURED — that test PASSES under the Rev-4 split, unmodified:**
> ```
> $ cargo test -p engine --lib pr625_c2
> test game::triggers::tests::pr625_c2_distinct_event_auto_orders_even_when_loop_detection_off ... ok
> ```

**✅ MEASURED — the consumer map** (`grep -rn "ability_scan::" crates/` on `main`):

> ## **The ONLY non-loop-detection consumer of the `event`/`sibling` axes is `game/triggers.rs` — exactly TWO call
> sites (the CR 603.3b gate).** Every other consumer is `analysis/resource.rs` (the CR 732.2a firewall).
> And `fire_time_conditions_read_growing_class` (`resource.rs:1457`) is called from **exactly two** places —
> **`:968`** and **`:1131`**, the object-growth and fodder-growth covers. **It is the firewall and nothing else.**

**⇒ THE SEPARATION (P0, below): the two questions stop sharing one answer.**

**✅ MEASURED — with the split in place:**

| probe | Rev 3 | **Rev 4** |
|---|---|---|
| `resource.rs:3926` `event_and_sibling_axes_unchanged_for_typed` — **the repo's OWN over-edit guard for the shared authority** | ⛔ **RED** (Rev 3 had to *re-author* it) | ✅ **GREEN, UNMODIFIED** |
| `c2_order_independent` for a vanilla `Effect::Token` ability | `true` ⇒ **AUTO-ORDERS** | ✅ **`false`** ⇒ **PROMPTS** — *identical to `main`* |
| the canary still OFFERS (the firewall keeps its precision) | — | ✅ **yes** (§1.1) |

```
===== R3 (the reviewer's own B3 probe), Rev-4 split =====
  ability_uses_event_context    = true
  ability_reads_sibling_mutable = true
  c2_order_independent (triggers.rs:3894) = false     ← MAIN BASELINE. Byte-identical.
```

> ## ⇒ **`Off` is byte-identical BY CONSTRUCTION: not one CONSERVATIVE-mode arm changes.** The `:3926` tripwire —
> which Rev 3 broke and had to rewrite — **passes untouched**, and that is the strongest available signal that the
> shared authority is intact.

**✅ MEASURED — the full engine lib suite, with P0 + P1 + P2-d + P3 all applied:**

| | full suite (`cargo test -p engine --lib`) |
|---|---|
| **Rev 3's own baseline** | `16547 passed; ` ⛔ **`1 failed`** — *"the `Typed` arm keeps `sibling:true`"* (the shared-authority guard, RED) |
| ⭐ **Rev 4** | ✅ **`16550 passed; 0 failed; 7 ignored`** |

> ## ⇒ **REV 3 SHIPPED A RED SHARED-AUTHORITY GUARD AND CALLED IT "a revert-probe to be re-authored." REV 4 DOES NOT
> BREAK IT AT ALL.** A guard you have to rewrite to make your change pass is a guard that is telling you something.
> **The +3 are Rev 4's new tests. Zero regressions.**

### 1.5 M11–M14 — **THE TYPE ENUMERATIONS. `rustc`/AST MEASURED, NOT EYEBALLED.**

| # | Type | Rev 3 said | ✅ **MEASURED** (ast-grep over the AST) |
|---|---|---|---|
| **M11** | `ContinuousModification` | "**41** variants, `:19350–:19599`" | ⛔ **53 variants**, `:19350–:19710`. **B5 CONFIRMED.** |
| **M12** | `ManaProduction` | "a struct with a `count: QuantityExpr`" | ⛔ **a 15-variant ENUM**, `:1678–:1832`. **B2 CONFIRMED.** |
| **M12b** | `Effect::Mana` | "1 field (`produced`)" | ⛔ **5 fields** — `produced`, `restrictions`, `grants`, `expiry`, **`target: Option<TargetFilter>`**. |
| **M13** | `Keyword` | (7 sampled by the reviewer) | ⛔ **198 variants.** **B4 is far larger than stated.** |
| **M14** | `StaticMode` | (119, per the reviewer) | ✅ **119 variants** — confirms `AddStaticMode` has **no walker**. |
| — | `PtValue` | 3 variants | ✅ **3** (`Fixed`/`Variable`/`Quantity`). |

### 1.6 ⛔ M15 — **TWO NEW DEFECTS I FOUND THAT NEITHER REV 3 NOR THE REVIEW CAUGHT**

#### ⛔ M15-a — **THERE ARE *TWO* F1 CONJUNCTS. REV 3 (AND THE REVIEW) NAMED ONLY ONE.**
`last_recast_context` is compared in **two** cover gates, not one:

| site | function | Rev 3 |
|---|---|---|
| **`analysis/resource.rs:662`** | `loop_states_equal_modulo_resources` | ⛔ **NEVER MENTIONED** |
| `analysis/resource.rs:1444` | `eq_except_growable` | named ✅ |

**Both are ONE-SIDED-SAFETY discriminators and BOTH must be renamed and KEPT.** Renaming only `:1444` leaves `:662`
**failing to compile** (which is fail-safe) — but a plan that names one of two comparison sites has not audited the
field. There are also **two paired tests** (`resource.rs:5672`, `:5697`).

#### ⛔ M15-b — **`normalize_recast_frame`'s STRIP IS *NOT* A NO-OP UNDER `Activate`. IT WOULD DELETE THE DRIVING PERMANENT.**
Rev 3's P1-d claim #3: *"Under `Activate` there is no such card ⇒ the strip is a **no-op**."* **REFUTED.**
`normalize_recast_frame` (`engine.rs:1599`) removes **every object matching `(card_id, from_zone, controller)`**. An
activation's context has **`from_zone == Zone::Battlefield`** ⇒ the filter matches **the driving permanent itself**
and deletes it from every comparison frame. **The strip must be `Recast`-only** (dispatched on `ctx.action`).

### 1.7 BANKED — the Rev-3 measurements the reviewer REPRODUCED (do not re-derive)

**M0/M1/M2/M4/M5 reproduce verbatim.** The canary was RED. **The ring can NEVER see an activated-ability loop by
design** — the deliberate-action clear (`engine.rs:3089-3093`) and the empty-stack clear (`:2325`) mean
`bf_prior == bf_cur` at **every** bridge entry, so the ring's delta is structurally **zero**. **Bridge (B)**
(`engine.rs:445-464`) is the only path that OFFERS, and **every conjunct was green except `last_recast_context`**.
**REACH was the real gap — and P1 closes it (§1.1).** The three firewall vetoes are exactly
`{S1Trigger, S2BattlefieldBody, S4StaticModifications}`. The fodder Elf is a **pure vanilla token** (`triggers=0
statics=0 abilities=0 keywords=0`) and satisfies `object_is_inert` outright.

---

## 2. What already exists — **DO NOT REBUILD ANY OF THIS**

| Stage | On `main` | Where |
|---|---|---|
| **1 · capture** | `last_recast_context` | **written** `game/casting_costs.rs:6795`; **read** `game/engine.rs:450` |
| **2 · repeat** | real replay on a clone, 2 iterations / 3 settle frames, re-entrancy-guarded | `game/engine.rs:1688-1696`; driver `drive_recast_iteration` `:1451` |
| **2 · unbounded (LIVE)** | `loop_states_cover_modulo_fodder_growth` | `analysis/resource.rs:1095`, called `engine.rs:1732` |
| **3 · classify** | Path A `:498` · Path B `:536` · Path C `:577` · `WinKind` | `game/engine.rs` · `analysis/loop_check.rs:83` |
| **4 · present** | `WaitingFor::LoopShortcut` · `IterationCount` | `types/game_state.rs:4458` · `analysis/decision_template.rs:281` |
| **5 · apply** | `apply_confirmed_shortcut` → `apply_until_lethal_shortcut` / `materialize_fixed_shortcut` | `engine.rs:855`, `:906`, `:1325` |
| **— · determinism** | **static** `spell_ability_bears_randomness` (`:1684`) + **runtime** RNG word-position delta (`:1713`) | |

> ## ⇒ **The pipeline is complete end to end.** It was never a capability problem — it was **REACH**, and §1.1 closes it.

---

## 3. The architectural spine

The three vetoes are **blankets that refuse to descend**:
- `Effect::Token { .. } => Axes::CONSERVATIVE` (`ability_scan.rs:447`) — never looks at **what the token is**.
- `Effect::Mana { .. } => Axes::CONSERVATIVE` (`ability_scan.rs:862`) — never looks at **what it produces**.
- `!def.modifications.is_empty() => true` (`resource.rs:1539`) — never looks at **what the modification does**.

A *sound, general* descent means reasoning about arbitrary ability programs. **We do not have to solve that problem.**
Compose: **battlefield-only** (CR 113.6 + CR 400.2) · **no nested loops** · **not Turing-complete** · **and the TWO
CONCRETELY OBSERVED ITERATIONS the clone-drive already produces** — and the question collapses to:

> # **"Does any BATTLEFIELD ability's fire-time condition read THE SPECIFIC AXIS that THIS OBSERVED loop grows?"**

### 3.1 ⛔ NON-GOALS — state these and hold them
- ❌ No fixpoint / abstract interpretation / e-graphs / symbolic execution. **Ever.**
- ❌ No general program analysis of ability bodies. The walk is a **finite, exhaustive, single-pass AST match**.
- ❌ **No nested-loop support.** Grant-realization depth is **1**; a grant-of-a-grant is **fail-closed ⇒ REJECT**.
- ❌ No Turing-complete combo class. Out of scope by construction, forever.
- ❌ **No new cover.** (M4: none is needed.)
- ❌ **No change to the ring, the sampler, or the deliberate-action clear.** They are correct for their class.

### 3.2 ⭐ The soundness asymmetry is PRESERVED, not traded away
Narrowing the input class **BUYS** the precision; it does not spend safety. Any shape outside the recognized class —
deeper-than-depth-1 grants, non-battlefield function, an unclassifiable axis, a **new enum variant** — **still fails
closed ⇒ REJECT.**

---

## 4. THE PHASES — with DIRECTION OF SOUNDNESS

> ⚠️ **Four phases move the detector toward ACCEPT.** Those can emit a false certificate and end a real game wrongly.

| Phase | What | Direction | Discriminating negative (revert-probed) |
|---|---|---|---|
| **P0** | ⭐ **THE SHARED-AUTHORITY SPLIT** (`ScanMode`) | ✅ **neutral by construction** | `Off`-byte-identity tripwires (§1.4) |
| **P1** | REACH — capture + drive a repeated **ACTIVATION** | ⚠️ **ACCEPT** | ✅ M8 negative twin (**arms, does not offer**) |
| **P2** | FIREWALL — make the blankets DESCEND | ⚠️ **ACCEPT** | ✅ U3 (M9) + Gaea's Cradle axis assert |
| **P3** | `Typed` NAMES a type; it does not COUNT one | ⚠️ **ACCEPT** | `:1606` `ObjectCount` still vetoes |
| **P4** | CR 400.2 hidden-zone leak | ✅ **REJECT (safe)** | library-card fixture |
| **P5** | `LoopDetectionMode` doc comment | ⚪ neutral | — |
| **P6** | scan (6) delayed-trigger blanket | ⚠️ **ACCEPT** | non-empty-store fixture |

> ## ⇒ **P0 IS A HARD PREREQUISITE OF P2 AND P3.** Landing P2/P3 without it ships the B3 regression.
> **Ship order: P0 → P2 → P3 → P1 → P4 → P6 → P5.**

---

### ⭐⭐ P0 — **THE SHARED-AUTHORITY SPLIT** *(NEW in Rev 4 — the architectural crux; discharges B3)*

> ## ⛔ **THE FIREWALL NEEDS A *PRECISE* PREDICATE. THE CR 603.3b GATE NEEDS A *CONSERVATIVE* ONE. THEY ARE
> DIFFERENT QUESTIONS AND MUST STOP SHARING ONE ANSWER.**

They want **opposite approximations**:

| Consumer | Wants | Because a wrong answer means |
|---|---|---|
| **CR 603.3b** `triggers.rs:3894-3895` | **CONSERVATIVE** (over-approximate reads) | under-reading ⇒ **auto-order a group the player must order** ⇒ rules-wrong |
| **CR 732.2a firewall** `resource.rs:1457` | **PRECISE** (no spurious reads) | over-reading ⇒ a missed offer (**safe**); under-reading ⇒ **false certificate** |

**Parameterize the walker with an explicit mode** (CLAUDE.md: *parameterize, don't proliferate* — one walker, one
axis of variation, **not** two copies):

```rust
/// Which QUESTION the `Axes` walk is answering. The two consumers need OPPOSITE
/// approximations, so they must not share one answer (B3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanMode {
    /// CR 603.3b trigger ordering + EVERY pre-existing consumer. Blankets STAY blanket.
    /// #4603: this mode is BYTE-IDENTICAL to pre-Rev-4 `main`.
    Conservative,
    /// CR 732.2a loop firewall ONLY. Descends `Effect::Token` / `Effect::Mana`;
    /// `TargetFilter::Typed` NAMES a type rather than COUNTING one.
    LoopFirewall,
}
```

- **Thread `mode: ScanMode` through the `scan_*` walk.** Mechanical, **compiler-enforced**, no hidden state.
- **Every existing public entry point keeps its signature and passes `ScanMode::Conservative`** —
  `ability_uses_event_context`, `ability_reads_sibling_mutable`, `ability_definition_axes`,
  `ability_reads_projected_resource`, the `*_condition_reads_*` family. ⇒ **`triggers.rs` is not touched at all.**
- **Add `LoopFirewall`-mode twins** used **only** by `analysis/resource.rs`'s firewall:
  `ability_definition_reads_sibling_mutable_for_loop`, etc.
- **`fire_time_conditions_read_growing_class` (`resource.rs:1457`) — and only it — runs in `LoopFirewall` mode.**
  It is called from exactly `:968` and `:1131` (**measured**).
- ⛔ **`fire_time_conditions_read_projected_resource` (`:2152`) STAYS `Conservative`.** It feeds the ω/drain cover
  (`:831`), which is regression-pinned. **Minimum blast radius.**

#### ⭐ Why this makes the `Off`-byte-identity proof STRUCTURAL, not statistical
**Not one `Conservative`-mode arm changes.** The three divergent arms branch on `mode`. Therefore **no pre-existing
consumer can observe any difference** — and `LoopFirewall` mode is reachable **only** from a firewall that is itself
reachable only under `loop_detection.samples()`.

#### ⭐⭐ **`batch_conflict` IS MOOT FOR REV 4 — BY CONSTRUCTION, NOT BY MEASUREMENT. STATE THIS HONESTLY.**

The terminal C2 decision is `c2_order_independent && !batch_conflict` (`triggers.rs:3654`).

The reviewer flagged `batch_conflict == false` as **UNVERIFIED (a code trace, not a run)** — and they were right to. But
note **what that question was FOR**: it establishes whether Rev 3's `c2` flip was *observable*. If `batch_conflict` were
`true`, the conjunction would stay `false` and Rev 3's flip would have been harmless. **`batch_conflict` is load-bearing
for grading the SEVERITY OF REV 3's BUG.**

> ## ⇒ **IT IS NOT LOAD-BEARING FOR REV 4's SAFETY.** Rev 4 leaves `c2_order_independent` **byte-unchanged**
> (**MEASURED**: `false` for a vanilla token ability, identical to `main`). A conjunction with **one invariant conjunct
> and one untouched conjunct is invariant — for EVERY value of `batch_conflict`.** Rev 4 does not need to know what it
> is.

**Empirical backstop (not a substitute for the argument — a check on it):** the **entire trigger-ordering corpus** is
green under the split — full lib suite **`16550 passed; 0 failed`**, including
`pr625_c2_distinct_event_auto_orders_even_when_loop_detection_off` itself.

> ⛔ **STANDING CONDITION FOR ALL FUTURE REVISIONS.** The moment **any** revision changes a `Conservative`-mode arm,
> `batch_conflict` becomes **load-bearing again** and **MUST be measured with a runtime fixture** (the reviewer's
> suggested shape: **two identical *"whenever this creature deals combat damage to a player, create a 1/1"* creatures**,
> which yields a distinct-event C2 group). **UNVERIFIED to this day — see U10.** *Do not let a later change silently
> inherit Rev 4's exemption.*

#### P0 — tests *(`game/ability_scan.rs` `#[cfg(test)]`)*

| Test | Asserts | **Revert-probe (must FLIP to FAIL)** |
|---|---|---|
| ⭐ `conservative_mode_token_axes_are_unchanged` | a vanilla `Effect::Token` ability ⇒ `Conservative` gives `event=true, sibling=true` (i.e. `CONSERVATIVE`) | make the `Token` arm descend unconditionally ⇒ **FLIPS** |
| ⭐ `conservative_mode_mana_axes_are_unchanged` | same for `Effect::Mana` | same |
| ⭐⭐ `cr_603_3b_gate_is_byte_identical_for_a_token_trigger` | reconstruct `c2_order_independent` (`triggers.rs:3894-3895`) for a token-bodied trigger ⇒ **`false`** (engine PROMPTS) | descend the shared arm ⇒ **FLIPS to `true`** ⇒ auto-order |
| `loop_firewall_mode_token_axes_descend` | the same ability in `LoopFirewall` mode ⇒ `Axes::NONE` | bind `count` to `_` ⇒ FLIPS |
| ✅ `event_and_sibling_axes_unchanged_for_typed` (`resource.rs:3926`) | **KEEP VERBATIM. DO NOT RE-AUTHOR. DO NOT `#[ignore]`.** It must stay **GREEN** — it is the shared authority's over-edit guard. | — |

> ⚠️ **`ordering_parity_sweep` (`triggers_ordering_parity_tests.rs:492`) never touches `ability_scan`** — the Rev-3
> regression would have landed **SILENTLY GREEN**. P0 closes that by construction, and the three tripwires above are
> the *targeted* guard. **Additionally run `FORGE_TEST_FULL_DB=1 cargo test -p engine ordering_parity_sweep` and
> report the delta (expected: ZERO).**

---

### ⭐ P1 — **REACH: the ACTIVATED-ABILITY dual of the recast capture**

**The class.** CR 602.1 (`:2514`): *"Activated abilities have a cost and an effect."* CR 732.2a's worked example is an
**activated-ability** loop in which **no spell is ever cast**. Today the engine can capture only a repeated
**CastSpell**. **P1 captures a repeated `ActivateAbility` too** — the *other half* of "a player repeats an action."
**Together they are the CR 732.2a action space.**

**Card count.** Every token-creating activated ability a board can sustain: Presence of Gond + any untapper, Marneus
Calgar, Ivy Lane Denizen chains, every `{T}: create a token` + untapper. **Hundreds of Commander/Modern combos.**

#### P1-a — **PARAMETERIZE, DON'T PROLIFERATE.** `RecastContext` → `LoopActionContext`

⛔ **DO NOT add a sibling `last_activation_context` field.** Two reasons, one a **soundness hazard**:
1. **CLAUDE.md's parameterization rule.** A recast and an activation are two **leaf parameterizations of one
   structural axis**: *the repeated action that drives the loop.*
2. ⛔⛔ **THE SOUNDNESS HAZARD.** `impl PartialEq for GameState` **EXCLUDES** this field
   (`game_state.rs:11019-11025`), so **two** cover gates compare it by hand as a ONE-SIDED-SAFETY discriminator.
   A **new sibling field would be excluded from `PartialEq` too and would NOT be added to either** —
   reintroducing exactly the hole those conjuncts exist to close. **Parameterizing inherits the protection for free.**

```rust
/// CR 732.2a: the repeated ACTION that drives a captured loop. Two leaf shapes of one axis —
/// a re-cast spell (CR 601.2a) and a re-activated ability (CR 602.2a).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopAction {
    /// CR 601.2a + CR 702.27a: a self-returning (buyback) recast.
    Recast { from_zone: Zone, uses_buyback: BuybackUsage },
    /// CR 602.2a: a re-activated ability of a STABLE battlefield permanent.
    Activate { source_id: ObjectId, ability_index: usize },
}

pub struct LoopActionContext {          // was: RecastContext
    pub card_id: CardId,
    pub controller: PlayerId,
    pub action: LoopAction,             // ← the parameterization
    pub convoke: Option<ConvokeMode>,
}
```

Rename the field `last_recast_context` → **`last_loop_action_context`**.

> ## ⛔⛔ **M15-a: THERE ARE *TWO* COMPARISON CONJUNCTS. RENAME AND KEEP *BOTH*.**
> | site | function |
> |---|---|
> | **`analysis/resource.rs:662`** | `loop_states_equal_modulo_resources` ← **Rev 3 never mentioned this one** |
> | `analysis/resource.rs:1444` | `eq_except_growable` |
>
> Both are ONE-SIDED-SAFETY discriminators. **Keep both.** Update the two paired tests (`:5672`, `:5697`).

> ### ⛔ **G2 — SERIALIZED-SURFACE AUDIT** *(`add-engine-variant` Step 7 — Rev 3 said nothing about this)*
> The field **IS** on the serialized surface: `GameState: Serialize` ships **whole** to the client
> (`derived_views.rs:286-290` → `engine-wasm/src/lib.rs:1105`) and is persisted whole
> (`TrustedGameStateEnvelope`, `game_state.rs:3395`); the multiplayer viewer filter does **not** redact it.
> **BUT it is a WRITE-ONLY LEAF: zero consumers** — no `client/` hit, no golden-fixture hit, no `combo-verify` hit,
> and the client never sends it inbound. With `#[serde(default)]` and no `deny_unknown_fields`, an old save would
> **silently drop** it.
> **⇒ ADD `#[serde(alias = "last_recast_context")]` on the renamed field (one line, zero cost).** Blast radius is
> tiny (capture is gated on `.samples()`, default OFF) but a silent lossy drop is not something we accept in writing.

#### P1-b — the setter ⭐ **(THE B1 FIX)**

`game/casting_costs.rs:6795` keeps its arm **semantically byte-unchanged** (now constructing `LoopAction::Recast`).

**NEW arm — in the non-mana branch of the `GameAction::ActivateAbility` handler, `game/engine.rs:3286`** (the branch
that calls `casting::handle_activate_ability`). **Armed on a STATIC predicate, genuinely mirroring the recast's
`is_token_creating` (`casting_costs.rs:6789`):**

```rust
// CR 602.2a + CR 732.2a: capture a repeated activation as the loop's driving action.
// ⛔ A `state.battlefield.len() > before` gate here is STRUCTURALLY DEAD (B1, MEASURED):
// an activated ability only goes on the STACK at this beat — the token appears on
// RESOLUTION, a later beat. Mirror the recast's STATIC predicate instead.
let creates_token = state.objects.get(&source_id)
    .filter(|o| o.zone == Zone::Battlefield)          // CR 602.5a
    .and_then(|o| o.abilities.get(ability_index))
    .map(|def| {
        let mut effects = Vec::new();
        crate::analysis::ability_graph::collect_effects(def, &mut effects);   // the shipped walker
        effects.iter().any(|e| matches!(e, Effect::Token { .. }))
    })
    .unwrap_or(false);
state.last_loop_action_context = (state.loop_detection.samples() && creates_token)
    .then_some(LoopActionContext { card_id, controller: *player,
        action: LoopAction::Activate { source_id, ability_index }, convoke: None });
```

> ⛔ **DO NOT try to statically prove the ability is re-activatable.** The canary's `{T}` cost is only repeatable
> because Intruder Alarm untaps it. **The clone-drive IS the oracle** — if the 2nd activation is illegal it returns
> `Err(RecastAbort)` and **no offer is made** (✅ **MEASURED**, M8). Fail-closed, and it costs nothing.

#### P1-c — the drive ⭐ **(THE G3 + G4 FIX)**

Rename `drive_recast_iteration` (`engine.rs:1451`) → **`drive_loop_action_iteration`** and **dispatch the OPENER on
`ctx.action`**. Everything from `:1481` down (the `beat_cap` loop, the `ManaPayment`/convoke arm, the `Priority` +
empty-stack **settle boundary** at `:1537-1541`, the fail-closed `_ => Err(RecastAbort)`) is **action-agnostic and is
reused verbatim.**

> ## ⛔⛔ **G3 — DO NOT RE-FIND BY `card_id` + `min_by_key`. IT IS BROKEN FOR TOKENS.**
> The reviewer **refuted** the two-copies hypothesis for real cards (`CardId` is minted per physical card,
> `scenario.rs:229`) — **but every plain token is created with `CardId(0)`** (`effects/token.rs:813`, `:1060`).
> ⇒ if the loop's **driver is itself a token** — *the exact classes P1 names* — the filter matches **EVERY TOKEN THAT
> PLAYER CONTROLS, INCLUDING THE FODDER THE LOOP IS MANUFACTURING**, and `min_by_key` picks the lowest-id token, which
> need not be the driver.
>
> ## ⇒ **PIN THE SOURCE BY `ObjectId`.** An activation's source is a **stable battlefield permanent** — it never
> changes zones, so its `ObjectId` survives both `state.clone()` and the loop's growth. This is *strictly better* than
> a re-find and it sidesteps `CardId(0)` entirely. **(A recast card genuinely churns incarnations per CR 400.7 and
> must keep its `card_id` re-find — that is why the re-find is `Recast`-only.)**

```rust
match &ctx.action {
    LoopAction::Recast { from_zone, uses_buyback } => { /* :1460-1480 VERBATIM */ }
    // CR 602.2a: re-activate the SAME ability of the SAME permanent.
    LoopAction::Activate { source_id, ability_index } => {
        let src = clone.objects.get(source_id).ok_or(RecastAbort)?;
        // G4: the abilities vec is LAYER-DERIVED — a positional index can silently address a
        // DIFFERENT ability if the granted set changes. Fail closed on any drift.
        if src.zone != Zone::Battlefield || src.controller != ctx.controller
            || src.card_id != ctx.card_id
            || src.abilities.get(*ability_index) != Some(&expected_def) { return Err(RecastAbort); }
        apply_action(clone, ctx.controller,
            GameAction::ActivateAbility { source_id: *source_id, ability_index: *ability_index }, None)
            .map_err(|_| RecastAbort)?;
    }
}
```

> ### ⛔ **G4 — `ability_index` IS A POSITIONAL INDEX, NOT AN IDENTITY.**
> It indexes the **layer-derived** `obj.abilities` vec; the canary's ability exists **only** because an Aura grants it.
> **`AbilityDefinition` derives `PartialEq, Eq` (`types/ability.rs:15612`)** — so the fix is cheap and exact:
> **capture the `AbilityDefinition` at the offer hook, and require `Eq` at every drive iteration; abort otherwise.**
> Thread it as a local (do **not** store it on `GameState` — it would bloat the serialized surface).
> **Hostile fixture (REQUIRED): two auras granting two abilities to one creature.**

The `OptionalCostChoice` (buyback) arm becomes `LoopAction::Recast`-only; under `Activate` it is a **fail-closed
abort** (an activation that opens an optional-cost window is not a pinned shortcut — CR 732.2a *"can't include
conditional actions"*).

#### P1-d — the hook

`try_offer_object_growth_shortcut` (`engine.rs:1656`) — **every downstream line is reused unchanged**: the RNG
word-position backstop (`:1713`), `derived_fodder_class` (`:1633`), the fodder cover (`:1732`), the sign checks, the
certificate. **Three edits:**

1. `let ctx = state.last_loop_action_context.clone()?;`
2. **The static randomness gate (`:1684`) is spell-only.** Under `Activate`, scan the **activated ability's**
   definition instead — same authority (`spell_ability_bears_randomness`), different subject:
   `source.abilities[ability_index]`. ⛔ **Do NOT skip this and lean on the runtime backstop alone.**
   *(CR 705.1 / CR 706.1a / CR 701.9a — a randomness-bearing repeated action is not a legal shortcut.)*
3. ⛔⛔ **M15-b — `normalize_recast_frame` (`:1599`) MUST BE `Recast`-ONLY.** Rev 3 called the strip "a no-op" under
   `Activate`. **IT IS NOT.** An `Activate` ctx has `from_zone == Battlefield`, so the strip would **DELETE THE
   DRIVING PERMANENT** from every comparison frame. Dispatch on `ctx.action`: strip **only** for `Recast`; keep the
   `last_created_token_ids` / `last_revealed_ids` / `last_zone_changed_ids` clears for **both** (they churn per cycle
   for both shapes).

#### P1 — tests *(`crates/engine/tests/integration/loop_shortcut_activation.rs`; add `mod` to `tests/integration/main.rs`)*

| Test | Asserts | **Revert-probe (must FLIP to FAIL)** |
|---|---|---|
| ⭐ `activation_loop_gond_intruder_alarm_offers_shortcut` | the canary reaches `WaitingFor::LoopShortcut`, `unbounded` naming `TokensCreated` | delete the `LoopAction::Activate` setter arm ⇒ **no offer** ✅ *(the arm is the sole capture)* |
| ⭐⭐ `activation_loop_without_untapper_does_not_offer` | **Gond alone (no Intruder Alarm)** ⇒ **NO offer** — **AND ASSERT `last_loop_action_context.is_some()`** | ⭐ **THE NON-VACUITY GUARD.** The capture MUST still arm; the rejection MUST come from the DRIVE. If the capture is dead, this negative passes for free (**that is exactly how Rev 3 died**). ✅ **MEASURED, M8.** |
| `activation_loop_declare_shortcut_materializes_n_tokens` | `DeclareShortcut { count: Fixed(50) }` ⇒ battlefield grows by exactly 50 Elves | — |
| `activation_loop_randomness_bearing_ability_does_not_offer` | `{T}: flip a coin, if heads create a token` + untapper ⇒ **NO offer** | delete the P1-d static gate ⇒ must still fail via the `:1713` RNG backstop; if it does **not**, the backstop is broken |
| ⭐ `activation_loop_token_driver_does_not_misbind` **(G3)** | the driving permanent is **itself a token** (`CardId(0)`) with fodder tokens present ⇒ the drive addresses the **driver**, never the fodder | revert the `ObjectId` pin to `card_id` + `min_by_key` ⇒ **must FLIP** (matches its own fodder) |
| ⭐ `activation_loop_two_auras_ability_index_is_revalidated` **(G4, HOSTILE)** | two auras grant two abilities to one creature ⇒ the drive re-validates the `AbilityDefinition` by `Eq` and never drives the wrong ability | delete the `Eq` re-validation ⇒ **must FLIP** |
| `activation_loop_heterogeneous_context_does_not_cover` | two DIFFERENT `ability_index` values across cycles ⇒ the cover's context conjunct rejects | ⭐ delete **either** `resource.rs:662` **or** `:1444` conjunct ⇒ **must FLIP to a false certificate** (P1-a's soundness proof, **both sites**) |

---

### ⭐ P2 — **Make the blankets DESCEND** *(runs in `ScanMode::LoopFirewall` ONLY — see P0)*

#### P2-a — `scan_continuous_modification` — the walker the code says does not exist

**New**, in `game/ability_scan.rs`, beside the shipped precedent `modification_grants_growing_cost_keyword` (`:4080`).
**EXHAUSTIVE, NO `_` WILDCARD** — a future variant **fails to compile** until classified.

**⭐ ALL 53 VARIANTS, classified by MEASURED payload type** (`ast-grep` over `types/ability.rs`, §1.5):

| Class | n | Variants | Verdict |
|---|---|---|---|
| **Read-free structural** *(payloads: `String`/`i32`/`u32`/`CoreType`/`ManaColor`/`Supertype`/`BasicLandType`/`SubtypeSet`/unit — **no** `QuantityExpr`, `TargetFilter`, `Keyword`, `AbilityDefinition`, `StaticMode`, `StaticDefinition`)* | **33** | `SetName` `RemoveAllAbilities` `AddType` `RemoveType` `AddSubtype` `RemoveSubtype` `SetCardTypes` `RemoveAllSubtypes` `AddAllCreatureTypes` `AddAllBasicLandTypes` `AddAllLandTypes` `AddChosenSubtype` `AddChosenColor` `AddChosenKeyword` `RemoveChosenKeyword` `SetColor` `AddColor` `SwitchPowerToughness` `AssignDamageFromToughness` `AssignDamageAsThoughUnblocked` `AssignNoCombatDamage` `ChangeController` `SetBasicLandType` `SetChosenBasicLandType` `SetChosenName` `AddSupertype` `RemoveSupertype` `SetStartingLoyalty` `RemoveManaCost` + ⭐ the **ANTHEM class** `AddPower{i32}` `AddToughness{i32}` `SetPower{i32}` `SetToughness{i32}` | `Axes::NONE` |
| ⭐ **the ANTHEM argument** | | *An anthem **READS NOTHING**. It **applies to** each member of a growing class; it does not **read** a mutable aggregate.* **This corrects the firewall doc's own wrong justification** (`resource.rs:1452-55`). | |
| **Quantity-bearing (descend)** | **8** | `SetDynamicPower` `SetDynamicToughness` `SetPowerDynamic` `SetToughnessDynamic` `AddDynamicPower` `AddDynamicToughness` `AddDynamicKeyword{kind, value}` · ⭐⭐ **`AddCounterOnEnter { counter_type, count: QuantityExpr, if_type }` (`:19686`)** | `scan_quantity_expr(value)` |
| ⛔ **B5's CATCH** | | **`AddCounterOnEnter` LOOKS structural and carries a `QuantityExpr`.** Rev 3 never named it. Sweeping it into the read-free bucket **certifies a dynamic counter count as inert.** | |
| ⭐ **Keyword-bearing (descend — B4)** | **2** | `AddKeyword { keyword }` · `RemoveKeyword { keyword }` | `scan_keyword(keyword)` — **see P2-b′** |
| **Ability-bearing (descend, depth ≤ 1)** | **2** | `GrantAbility { definition: Box<AbilityDefinition> }` → `ability_definition_axes` (`:3632`) · `GrantTrigger { trigger }` → `scan_trigger_condition` + the trigger's body | **`depth > 0` ⇒ `Axes::CONSERVATIVE`** (§3.1: no nested grants) |
| ⛔⛔ **FAIL-CLOSED — NO WALKER EXISTS** | **8** | ⭐ **`CopyValues`** → `CopiableValues` holds `Arc<Vec<ReplacementDefinition>>` (the module header `:32-36` **names it as not-descended**) · ⭐ **`AddStaticMode { mode: StaticMode }`** → **`StaticMode` has 119 variants and there is NO `scan_static_mode`** · ⭐ **`GrantStaticAbility { definition: Box<StaticDefinition> }`** → carries `StaticMode` **and** `per_player_condition: Option<ParsedCondition>` — **neither has a walker** · `GrantAllActivatedAbilitiesOf` · `GrantAllTriggeredAbilitiesOf` · `AddKeywordWithDerivedCost` · `RetainPrintedTriggerFromSource` · `RetainPrintedAbilityFromSource` | **`Axes::CONSERVATIVE`** |

> ## ⛔ **B5's SECOND HALF: DEMOTE `CopyValues` / `AddStaticMode` / `GrantStaticAbility` TO CONSERVATIVE.**
> Rev 3 put them in "descend depth ≤ 1", but that descent **reaches `ReplacementDefinition` / `StaticMode` (119
> variants) / `ParsedCondition` — for which NO WALKERS EXIST.** A depth-limited descent that stops short of them is a
> **false certificate**. §3.2's own asymmetry demands fail-closed. **Building the three missing walkers is a
> follow-up (DEFERRED-4), not this plan.**
>
> **53 = 33 + 8 + 2 + 2 + 8.** ✅ *(Rev 3 said 41 and named 43.)*

#### P2-b — `Effect::Token { .. }` DESCENDS — **exhaustive destructure, NO `..`**

`ability_scan.rs:447`. Precedent: `resolved_ability_axes` (`:148-155`). **Rev 3's 14-field list is COMPLETE ✅ — but
it MIS-BINNED `keywords`.**

| field | type | verdict |
|---|---|---|
| `power` / `toughness` | `PtValue` (3 variants ✅) | **descend** via new `scan_pt_value` (`Fixed`/`Variable` ⇒ `NONE`; `Quantity(q)` ⇒ `scan_quantity_expr`) |
| `count` | `QuantityExpr` | **descend** `scan_quantity_expr` |
| `owner` | `TargetFilter` | **descend** `scan_target_filter` |
| `attach_to` | `Option<TargetFilter>` | **descend** |
| `static_abilities` | `Vec<StaticDefinition>` | **descend** the condition **and** the modifications (P2-a's walker) |
| `enter_with_counters` | `Vec<(CounterType, QuantityExpr)>` | **descend** `scan_quantity_expr` |
| ⛔⛔ **`keywords`** | **`Vec<Keyword>`** | ⛔ **REV 3 BOUND THIS TO `_` AS READ-FREE. IT IS NOT.** **descend** `scan_keyword` — **B4** |
| `name` `types` `colors` `tapped` `enters_attacking` `supertypes` | — | `_` with a one-line justification each *(reviewer-confirmed read-free ✅)* |

#### ⭐ P2-b′ — `scan_keyword` **(B4)** — **`Keyword` HAS 198 VARIANTS, AND PAYLOAD SHAPE ALONE IS UNSOUND**

**✅ MEASURED payload histogram:** `100` unit · `43` `ManaCost` · `28` `u32` · `5` `String` · **`2` `QuantityExpr`**
(`Mobilize` `keywords.rs:859`, `Firebending` `:939`) · `2` `AbilityCost` · `1` each of `TargetFilter` (`Enchant`),
`TypedFilter` (`Affinity`), `WardCost`, `ProtectionTarget`, `HexproofFilter`, … .

> ## ⛔⛔ **THE TRAP — AN IMPLEMENTER WILL CLASSIFY BY PAYLOAD SHAPE AND BE WRONG.**
> **`Convoke`, `Delve`, `Improvise`, `Bargain`, `Station` are UNIT variants that carry NO payload — and they all
> READ THE BOARD.** The engine's own shipped authority says so: `keyword_cost_reads_growing_class`
> (`ability_scan.rs:3867`) — an **exhaustive `Keyword` match** — returns `true` for all of them. **The read lives in
> the keyword's SEMANTICS, not in its payload type.**

**⇒ `scan_keyword` = the SHIPPED SEMANTIC AUTHORITY `.or()` a PAYLOAD DESCENT.** Reuse, do not re-derive:

```rust
/// CR 613.1 + CR 702: a granted keyword's read surface. EXHAUSTIVE, NO `_` WILDCARD.
fn scan_keyword(kw: &Keyword, mode: ScanMode) -> Axes {
    // (a) the COST/board surface — the shipped exhaustive authority (:3867). 17 keywords
    //     (Convoke/Delve/Improvise/Affinity/Crew/Craft/Bargain/Casualty/…) read a growing
    //     board or graveyard class. Their payloads do NOT reveal this.
    let mut acc = if keyword_cost_reads_growing_class(kw) { Axes::SIBLING } else { Axes::NONE };
    // (b) the PAYLOAD surface — the dynamic values the authority above does not look at.
    acc = acc.or(match kw {
        Keyword::Mobilize(q) | Keyword::Firebending(q) => scan_quantity_expr(q, mode),
        Keyword::Enchant(f)                            => scan_target_filter(f, mode),
        Keyword::CumulativeUpkeep(c) | Keyword::Escalate(c) => scan_ability_cost(c, mode),
        // ... payload types with NO walker ⇒ explicitly CONSERVATIVE (fail-closed):
        Keyword::Ward(_) | Keyword::Affinity(_) | Keyword::Craft { .. } | Keyword::HexproofFrom(_)
        | Keyword::Protection(_) | Keyword::Companion(_) | Keyword::Gift(_) | ... => Axes::CONSERVATIVE,
        // ... the ~170 unit / u32 / String / ManaCost payloads: structurally read-free.
        Keyword::Flying | Keyword::Trample | ... => Axes::NONE,
    });
    acc
}
```
**The `|`-chains are long but MECHANICAL, and `rustc` enforces completeness.** The **semantic** read surface is
delegated to a reviewed, shipped, exhaustive authority — so a mis-binning in the read-free chain can only lose a
*structural payload* read, and those variants are explicitly enumerated.

**⇒ `ContinuousModification::AddKeyword`/`RemoveKeyword` route through the same `scan_keyword`** — closing the
identical defect Rev 3 introduced in P2-a. *(Precedent: `modification_grants_growing_cost_keyword` (`:4080`) — **the
very function Rev 3 cited as its model** — already routes `AddKeyword` to `keyword_cost_reads_growing_class`.)*

#### P2-c — `Effect::Mana { .. }` DESCENDS **(B2 — REBUILT ON THE REAL TYPE)**

> ## ⛔⛔ **REV 3 MODELLED `ManaProduction` AS A STRUCT WITH A `count`. IT IS A 15-VARIANT ENUM (`:1678–:1832`).**
> "Descend into `count`" hands the count-less variants **`Axes::NONE`** = **A FALSE CERTIFICATE**. Rev 3 replaced a
> **correct rejection** with a **false acceptance**.

**`Effect::Mana` has 5 fields — exhaustive destructure, no `..`:**
`produced: ManaProduction` · `restrictions: Vec<ManaSpendRestriction>` · `grants: Vec<ManaSpellGrant>` ·
`expiry: Option<ManaExpiry>` · ⭐ **`target: Option<TargetFilter>`** (read-bearing — Jeska's Will: *"Add {R} for each
card in **target opponent's** hand"*) ⇒ **`scan_target_filter`**.

**`scan_mana_production` — EXHAUSTIVE over all 15 variants, no `_`:**

| Variants | Verdict |
|---|---|
| `Fixed` · `Colorless{count}` · `Mixed` · `AnyOneColor{count,..}` · `AnyCombination{count,..}` · `ChosenColor{..}` | `scan_quantity_expr(count)` where a `count` exists, else `Axes::NONE` |
| ⭐ **`DistinctColorsAmongPermanents { filter: TargetFilter }` (`:1810`) — NO `count`** | ⛔ **`scan_target_filter(filter)`** — a **board-aggregate reader** (Faeburrow Elder). Under Rev 3: `NONE`. |
| ⭐ **`TriggerEventManaType` (`:1832`) — unit variant**, doc: *"Resolves from `state.current_trigger_event` at resolution time"* | ⛔ **`Axes::CONSERVATIVE`** (the **event** axis). Under Rev 3: `NONE`. |
| `AnyCombinationOfObjectColors { count, scope: ObjectScope }` · `AnyTypeProduceableBy { count, land_filter }` · `AnyOneColorAmongPermanents { count, filter, .. }` | descend **count AND** the filter/scope |
| `ChoiceAmongExiledColors` (reads `state.exile_links`) · `ChoiceAmongCombinations` · `OpponentLandColors` · `AnyInCommandersColorIdentity` | **`Axes::CONSERVATIVE`** (fail-closed) |

⇒ **Gaea's Cradle** (`{T}: Add {G} for each creature you control`) **still REJECTS — via a measured READ, not a
blanket.** *(Its production is a board-aggregate shape, **not** the `count` read Rev 3 asserted — which is why Rev 3's
`gaeas_cradle_mana_ability_still_vetoes` test design was also wrong.)*

#### P2-d — ⛔⛔ **THE SAFETY-CRITICAL WIRING. `sibling || projected`. MEASURED LOAD-BEARING (U3 / M9).**

Replace `analysis/resource.rs:1539`:
```rust
if !def.modifications.is_empty() { return true; }                       // ⛔ BEFORE (blanket)
```
```rust
// CR 613.1: descend — a modification vetoes iff it READS a mutable aggregate (sibling)
// OR a projected player resource. A fixed anthem reads NEITHER.
if def.modifications.iter().any(|m| {
    scan::continuous_modification_reads_sibling_mutable(m)
        || scan::continuous_modification_reads_projected_resource(m)   // ⛔⛔ BOTH AXES — U3
}) { return true; }                                                     // ✅ AFTER
```

> ## ⛔ **THE VETO MUST BE `sibling || projected`. THIS IS NOT A STYLE POINT — IT IS MEASURED (M9).**
> **DO NOT MERGE P2 UNTIL `projected_reading_modification_still_vetoes_the_firewall` IS SHOWN TO GO **RED** ON THE
> REVERT.** ✅ **It does (§1.3).** And it is only conclusive because the walker **descends** `SetDynamicPower` —
> see the CONFOUND WARNING in §1.3.

#### P2 — tests

| Test | Asserts | **Revert-probe** |
|---|---|---|
| `fixed_anthem_modification_reads_nothing` | `AddPower{2}` ⇒ `Axes::NONE` both axes | — |
| `dynamic_pt_modification_reads_sibling` | `SetDynamicPower{Ref(ObjectCount)}` ⇒ `sibling == true` | flip the arm to `NONE` ⇒ FAIL |
| ⭐ `token_effect_with_dynamic_enter_counters_reads_sibling` | `enter_with_counters: [(P1P1, Ref(ObjectCount))]` ⇒ `sibling == true` | bind `enter_with_counters` to `_` ⇒ **must FLIP** |
| ⭐ `token_effect_with_dynamic_static_ability_reads_sibling` | `static_abilities: [{modifications:[SetDynamicPower{Ref(ObjectCount)}]}]` ⇒ `sibling == true` | bind `static_abilities` to `_` ⇒ **must FLIP** |
| ⭐⭐ **`token_effect_with_growing_cost_keyword_reads_sibling` (B4)** | a token with **`Keyword::Convoke`** (a UNIT variant!) ⇒ `sibling == true` | ⛔ **bind `keywords` to `_` ⇒ MUST FLIP to a false `NONE`.** *This is the B4 discriminator, and it is deliberately a **unit** keyword so it also proves payload-shape classification is insufficient.* |
| ⭐⭐ **`add_counter_on_enter_modification_reads_sibling` (B5)** | `AddCounterOnEnter{count: Ref(ObjectCount)}` ⇒ `sibling == true` | ⛔ sweep it into the read-free bucket ⇒ **must FLIP** |
| ⭐⭐ **`mana_production_distinct_colors_among_permanents_vetoes` (B2)** | `DistinctColorsAmongPermanents{filter}` ⇒ **`sibling == true`** | ⛔ **descend only `count` (Rev 3's design) ⇒ MUST FLIP to `NONE`.** *The B2 false-certificate discriminator.* |
| ⭐ `mana_production_trigger_event_type_is_conservative` (B2) | `TriggerEventManaType` (unit) ⇒ `event == true` | bin it `NONE` ⇒ FLIP |
| ⭐ `gaeas_cradle_mana_ability_still_vetoes` | real Gaea's Cradle from `card-data.json` ⇒ firewall `true` **AND assert the AXIS on the parsed `Effect::Mana`** — a blanket cannot produce an axis | **a pass-only assertion here is VACUOUS** (Rev 1 shipped exactly that) |
| ⭐⭐ **`projected_reading_modification_still_vetoes_the_firewall` (U3)** | `SetDynamicPower{Ref(LifeTotal)}` ⇒ firewall `true`; **plus the two AXIS-ISOLATION asserts** (`sibling == false`, `projected == true`) | ⛔⛔ **drop `\|\| ...reads_projected_resource(m)` ⇒ MUST FLIP TO FAIL.** ✅ **MEASURED (M9).** |

---

### P3 — **`Typed` NAMES a type; it does not COUNT one** *(ONE LINE, in `LoopFirewall` mode only)*

`game/ability_scan.rs:2418-2422`:
```rust
TargetFilter::Typed(tf) => Axes {
    event: true,                                       // :2419  ⛔ UNCHANGED
    sibling: mode == ScanMode::Conservative,           // :2420  ⇐ was `true`
    projected: typed_filter_reads_projected(tf),       // :2421  unchanged
},
```

**A `Typed` filter is a PREDICATE — `"creature"`, `"creature you control"`. It SELECTS a set. It does not read the
set's CARDINALITY.** Counting is `QuantityRef::ObjectCount`, **a different node**.

#### ⭐ Why this cannot open the catastrophic hole — **STRUCTURALLY IMPOSSIBLE, and MEASURED**
```rust
QuantityRef::ObjectCount { filter } => {
    let mut acc = Axes { event: false, sibling: true, projected: false };  // :1606 ← INDEPENDENT LITERAL
    acc = acc.or(scan_target_filter(filter));                              // :1609 ← .or() only ADDS
    acc
}
```
> ## ⇒ **`ObjectCount`'s `sibling: true` is its OWN literal at `:1606`. It does NOT come from `scan_target_filter`.**
> ## ⇒ **Relaxing `:2420` PROVABLY CANNOT un-reject the counting class.** Identically for `ObjectCountDistinct`
> (`:1618`) and `ObjectCountBySharedQuality` (`:1631`). **The COUNT-vs-NAME distinction is ALREADY IN THE CODE.
> P3 does not invent it — P3 stops `:2420` from lying about it.**

#### P3-b — the over-edit guard
1. **`event` at `:2419` STAYS `true` in BOTH modes.**
2. ✅ **Under P0, `event_and_sibling_axes_unchanged_for_typed` (`resource.rs:3926`) STAYS GREEN AND UNMODIFIED**
   (**MEASURED**, §1.4). ⛔ **DO NOT re-author it, do not `#[ignore]` it.** *(Rev 3 had to rewrite it — that was the
   smell that the shared authority was being moved.)*
3. ⚠️ **G5 (reviewer): the `sibling` axis has MORE live consumers than Rev 3 claimed** — `resource.rs:1512`,
   `:1610`, and the `ability_definition_reads_sibling_mutable` family at `:1472/:1495/:1519/:1573`. **All are inside
   the firewall** ⇒ all run in `LoopFirewall` mode ⇒ all intended. **The completeness claim is now measured, not
   asserted.**

---

### P4 — Scope the covers to **VISIBLE, FUNCTIONING** objects *(CR 400.2 / CR 113.6 — REJECT direction, safe)*

> ## **HONEST LABEL: this clears ZERO of V1/V2/V3.** Every canary veto is battlefield-resident. It ships because it is
> a **real information leak**, not because the canary needs it.

Three of the seven scans iterate `state.objects.values()` over **every zone, including library and hand**:
- **scan (1)** `resource.rs:1460` → `active_trigger_definitions` (`functioning_abilities.rs:391`), whose filter is
  **bare `true`** (`:405-409`). **A trigger def on a card in the LIBRARY is returned as "active."**
- **scan (3)** `resource.rs:1501` → `active_replacements`, filtered **only** by `object_functions`.
- **scan (4)** `resource.rs:1527` — `state.objects.values()`, all zones.

**Two independent rules, either one fatal:**
1. **CR 113.6** (`:771`) — a non-instant/sorcery object's abilities *"usually function only while that object is on
   the battlefield."*
2. ⭐ **CR 400.2** (`:1935`) — ***"Library and hand are hidden zones."*** The offer's **presence or absence is itself
   observable to every player.** If it depends on hidden-zone contents, **the engine leaks hidden information into
   observable game state. That is a rules violation, not a conservative approximation.**

#### ⛔⛔ **THE TRAP — BLACKLISTED BY NAME**
> ## **`functioning_abilities::object_functions` (`:108`) IS NOT A ZONE-OF-FUNCTION AUTHORITY.**
> It checks **phased-out** and **Command-zone-non-emblem**, then `return true`. ⇒ **IT RETURNS `true` FOR A CARD IN
> THE LIBRARY.** **An implementer WILL reach for it, because scan (3) already calls it.**

**Call the engine's real authorities** — `triggers::trigger_definition_functions_in_zone` (`triggers.rs:1057` —
⚠️ **private; widen to `pub(crate)`**) and `functioning_abilities::static_functions_in_zone` (`:187`, already
`pub(crate)` ✅). **Exemplar: `granted_keyword_triggers_in_zone` (`triggers.rs:423-437`) already does exactly this.**

⚠️ **`active_trigger_definitions` is a SHARED authority** — the live trigger pipeline has its own zone gate
(`triggers.rs:1040`). **Fix the FIREWALL's scope; do NOT change `active_trigger_definitions`' contract.**

**scan (3)'s replacement zone-of-function authority is UNVERIFIED (U1)** — `ReplacementDefinition` has no measured
zone field. **The implementer must determine it first.** If none exists, restrict to `Zone::Battlefield |
Zone::Graveyard` (CR 113.6b) and **file the gap.** ⛔ Do NOT hand-roll a broader list.

**P4 test:** `hidden_zone_content_does_not_change_the_offer` — take the canary board, shuffle a loud card (a
`SetDynamicPower{Ref(ObjectCount)}` anthem) into P0's **LIBRARY**, assert the offer is **byte-identical**.
⛔ **Revert-probe: without P4 this FAILS** (the library card vetoes). **A test that passes both ways is vacuous.**

---

### P5 — `LoopDetectionMode`: ⛔ **KEEP ALL THREE MODES. TOUCH NOTHING.** *(USER DIRECTIVE)*

**The only change is a DOC COMMENT** on `LoopDetectionMode::On` (`types/game_state.rs`) recording that `On` is
**ANALYSIS-shaped**: it auto-resolves a lethal drain without an offer window (`engine.rs:420-427`), which is correct
for its offline consumer — the `combo-verify` corpus classifier — but is a live rules question as a *game* mode.
**⇒ DEFERRED-1.** **P5 touches no Rust, no TypeScript, no test.**

---

### P6 — scan (6): the delayed-trigger blanket *(⚠️ ACCEPT direction)*

`analysis/resource.rs:1582-1589` blanket-rejects on **non-emptiness** of the delayed/deferred/epic stores. **Any real
Commander board with ONE live delayed trigger dies here.** Once P2's walker exists this is a cheap descent: scan each
store's **ability body** with `ability_definition_reads_sibling_mutable` (in `LoopFirewall` mode). **Veto on
`sibling || projected`, exactly as P2-d.**

> ## ⛔⛔ **DISAMBIGUATION — AN IMPLEMENTER WILL CONFLATE THESE.**
> | | **scan (6)** `resource.rs:1582` | **`GameState::PartialEq`'s `delayed_triggers` conjunct** |
> |---|---|---|
> | What it does | **VETOES** if the store is **NON-EMPTY** | **COMPARES** the store across frames to decide RECURRENCE |
> | Direction | a **firewall** (rejects) | a **cover** (equality) |
> | This plan | **P6 RELAXES it** | ## ⛔ **DO NOT TOUCH.** It is what stops us certifying a loop **whose growth axis dies at the next end step.** |

---

## 5. Mandatory architectural sections *(`/engine-planner` Step 4)*

**Pattern Coverage.** A **WALKER** and a **CAPTURE GENERALIZATION**, not a card fix.
- **P1** covers **every repeated activated ability that grows the board** — the *other half* of CR 732.2a's action
  space. **Hundreds of Commander/Modern token engines.**
- **P2-a** classifies **all 53** `ContinuousModification` variants — every static/aura/anthem/equipment grant.
- **P2-b′** classifies **all 198** `Keyword` variants; **P2-c** all **15** `ManaProduction` variants.
- **P3** covers **every `Typed` target filter in the engine** — the most common filter node there is.
- **Card count: the entire enchantment / aura / anthem / token / mana surface — thousands.**
- ⭐ **The canary is an ACCEPTANCE TEST, not a GOAL.** Every phase discharges a **class** property.

**Building Blocks.** Compose from what exists — **no new analysis machinery, NO NEW COVER**:
`ability_scan::scan_quantity_expr` (`:2112`) · `scan_quantity_ref` (`:1573`) · `scan_target_filter` (`:2399`) ·
`scan_static_condition` (`:2926`) · `scan_ability_cost` (`:3784`) · ⭐ **`keyword_cost_reads_growing_class` (`:3867`
— the shipped exhaustive `Keyword` authority; B4 REUSES it rather than re-deriving 198 arms)** ·
`ability_definition_axes` (`:3632`) · `Axes` + `Axes::or` · ⭐ **`analysis::ability_graph::collect_effects` (the
shipped effect walker — P1-b's static capture predicate)** · `triggers::trigger_definition_functions_in_zone`
(`:1057`) · `functioning_abilities::static_functions_in_zone` (`:187`) · `drive_recast_iteration` (`engine.rs:1451`
— **90% action-agnostic already**) · `derived_fodder_class` · `normalize_recast_frame` ·
`loop_states_cover_modulo_fodder_growth` · the RNG word-position backstop (`engine.rs:1713`).
**Three new helpers, each justified:** `scan_continuous_modification` (the walker `resource.rs:1452-55` says is
missing), `scan_pt_value` (`PtValue::Quantity` has no scanner), `scan_keyword` (**no walker for `Keyword`'s
*payload* surface exists** — the shipped one covers only the *cost* surface).

**Logic Placement.** **All AST classification lives in `game/ability_scan.rs`** — the only module that may know a
variant's read surface. **`analysis/resource.rs` only CONSUMES `bool`s** (today's `:1539` blanket **is** exactly that
leak, and P2-d removes it). **Zone-of-function lives in `game/triggers.rs` / `game/functioning_abilities.rs`** and is
**called, never mirrored** (P4). **The loop-action capture is a `types/game_state.rs` TYPE + a `game/engine.rs`
reducer arm.** **Transport layers see nothing. Frontend: ZERO changes.**

**Rust Idioms.**
- **Exhaustive `match`, NO `_` wildcard** in every ACCEPT-direction walker — a future variant **must fail to
  compile**. *(See §0.1 for the precise rule: a wildcard to `CONSERVATIVE` is fail-closed and permitted; a wildcard to
  `NONE` is forbidden.)*
- **Exhaustive destructure, no `..`**, in `Effect::Token` and `Effect::Mana` — *this is precisely how Rev 2 missed
  three fields and Rev 3 missed `keywords`.*
- **`LoopAction` is a typed enum, not a `bool`/`Option` pair** — and it inherits **both** ONE-SIDED-SAFETY conjuncts
  (`resource.rs:662` **and** `:1444`) for free.
- **`ScanMode` is a typed enum, not a `bool`** — self-documenting at 30+ call sites.
- **Reuse `RecastAbort`** — do not introduce a second abort type.

**Extension vs Creation.** P0 **parameterizes** an existing walker (one new axis, no second copy). P1
**parameterizes** an existing context type. P2 **extends** the existing `scan_*` family. **No new pattern is created.**

**Analogous Trace.** Traced the recast capture end to end: `game/casting_costs.rs:6789/6795` (static
`is_token_creating` capture) → `types/game_state.rs:371` (`RecastContext`) → `game/engine.rs:450` (bridge (B) gate) →
`:1656` (`try_offer_object_growth_shortcut`) → `:1451` (`drive_recast_iteration`) → `:1599` (`normalize_recast_frame`)
→ `analysis/resource.rs:1095` (fodder cover) → `:1457` (firewall). **P1 is the activation-shaped dual of exactly this
chain, and reuses every stage but the opener.**

**Nom Compliance.** **N/A — no file under `crates/engine/src/parser/` changes.**

**CR Annotations.** Every number **grep-verified against `docs/MagicCompRules.txt`**: CR 732.2a (`:6372`) · the Gond
worked example (`:6373`) · CR 104.4b (`:366`) · CR 602.1 (`:2514`) · CR 602.2a (`:2529`) · CR 602.5a (`:2543`) ·
CR 113.6 (`:771`) · CR 400.2 (`:1935`). **No NEW CR number is introduced by Rev 4.**

---

## 6. ⭐ `/add-engine-variant` GATE — run for the new `LoopAction` enum *(G1: Rev 3 never ran it)*

| Stage | Verdict |
|---|---|
| **1 · Existence verification** | `grep -rn "enum LoopAction\|LoopActionContext" crates/` ⇒ **ZERO hits.** No existing type expresses "the repeated action that drives a captured loop." **NEW.** |
| **2 · Parameterization filter** | ✅ **THIS IS THE REFACTOR, NOT THE SMELL.** The alternative — a sibling `last_activation_context` field beside `last_recast_context` — **is** the sibling-cluster smell, and it is also a **soundness hazard** (P1-a). `LoopAction` **collapses two would-be siblings into one parameterized axis.** **EXTEND_OK.** |
| **3 · Categorical boundary** | ✅ **WITHIN_SECTION.** Both variants live in **CR 6xx — "casting spells and activating abilities"** (CR 601.2a / CR 602.2a), and both are *"a game action a player repeats"* under **CR 732.2a**. No cross-section unification. |
| **Step 2 · exhaustive matches** | `cargo check -p engine` — no wildcard fallbacks. |
| ⭐ **Step 3 · ability-scan / `ability_rw` classification** | **`LoopAction` is NOT traversed by `ability_scan` or `ability_rw`** — it is a *decision context* on `GameState`, not an ability AST node. **No walker arm is required.** ⚠️ **This is the step whose ABSENCE let Rev 3's B3 through** — so it is answered explicitly rather than skipped. |
| **Step 4 · runtime status** | Fully wired (capture + drive + offer). **No type-only stub.** |
| ⭐ **Step 7 · SERIALIZED-SURFACE AUDIT** | **G2 — see P1-a.** The field IS serialized (client + saves), has **ZERO consumers**, and `#[serde(default)]` + no `deny_unknown_fields` ⇒ an old save **silently drops** it. **⇒ ship `#[serde(alias = "last_recast_context")]`.** |

---

## 7. The file-by-file change set

| File | Phase | Change |
|---|---|---|
| `game/ability_scan.rs` | **P0, P2-a/b/b′/c, P3** | **NEW `ScanMode` enum + thread `mode` through the `scan_*` walk**; existing public entries pass `Conservative` (**byte-identical**) + new `*_for_loop` entries pass `LoopFirewall`. **NEW** `scan_continuous_modification` (53 variants) · `scan_pt_value` · `scan_keyword` (198 variants, reusing `keyword_cost_reads_growing_class`) · `scan_mana_production` (15 variants). `Effect::Token` (`:447`) + `Effect::Mana` (`:862`) descend (**exhaustive destructure, all fields**). `TargetFilter::Typed` (`:2420`) `sibling: mode == Conservative`. |
| `types/game_state.rs` | **P1-a, P5** | **NEW `LoopAction` enum**; `RecastContext` → `LoopActionContext`; field → `last_loop_action_context` **+ `#[serde(alias)]`**. **P5**: one doc comment on `LoopDetectionMode::On`. |
| `game/casting_costs.rs` | **P1-b** | `:6795` — construct `LoopAction::Recast`. **Semantics byte-unchanged.** |
| `game/engine.rs` | **P1-b/c/d** | **NEW `ActivateAbility` STATIC capture arm** (non-mana branch, `:3286`); `drive_recast_iteration` (`:1451`) → `drive_loop_action_iteration` with an action-dispatched opener (**`ObjectId` pin + `AbilityDefinition` `Eq` re-validation**); `try_offer_object_growth_shortcut` (`:1656`) reads the new field + action-dispatched randomness gate; **`normalize_recast_frame` (`:1599`) strip becomes `Recast`-ONLY (M15-b)**. |
| `analysis/resource.rs` | **P2-d, P3-b, P4, P6** | `:1539` blanket → **`sibling \|\| projected`** descent; **`:662` AND `:1444` conjuncts renamed — KEEP BOTH (M15-a)**; `fire_time_conditions_read_growing_class` (`:1457`) runs in **`LoopFirewall`** mode; scans (1)/(3)/(4) zone-scoped; scan (6) descends. **`event_and_sibling_axes_unchanged_for_typed` (`:3926`) is UNCHANGED.** |
| `game/triggers.rs` | **P4** | `trigger_definition_functions_in_zone` (`:1057`) private → `pub(crate)`. **No behavior change.** ⛔ **The CR 603.3b gate at `:3894-3895` is NOT TOUCHED.** |
| `tests/integration/loop_shortcut_activation.rs` | **P1** | **NEW** — 7 named tests (§4-P1). Add `mod` to `tests/integration/main.rs`. |

**Not touched, deliberately:** the ring · the sampler (`engine.rs:2323`) · the deliberate-action clear (`:3093`) ·
Path A/B/C · `GameState::PartialEq` · any cover · **`triggers.rs`'s CR 603.3b gate** · any frontend file.

---

## 8. Verification matrix — **every behavioral claim has a revert-failing assertion**

| Claim | Seam | Production entry | Test | Revert-probe ⇒ **FLIPS TO FAIL** | Status |
|---|---|---|---|---|---|
| the canary OFFERS | `engine.rs:3286` capture | `apply(ActivateAbility)` → `WaitingFor::LoopShortcut` | `activation_loop_gond_intruder_alarm_offers_shortcut` | delete the capture arm | ✅ **MEASURED (M7)** |
| the negative twin does NOT offer, **and the capture still arms** | the drive | `apply(ActivateAbility)` | `activation_loop_without_untapper_does_not_offer` | *(non-vacuity guard: asserts `ctx.is_some()`)* | ✅ **MEASURED (M8)** |
| `Off` is byte-identical (#4603) | `ScanMode::Conservative` | `triggers.rs:3894` | `cr_603_3b_gate_is_byte_identical_for_a_token_trigger` + **`resource.rs:3926` unchanged** | descend the shared arm ⇒ `c2` flips `false→true` | ✅ **MEASURED (M10)** |
| the `projected` veto term is load-bearing | `resource.rs:1539` | the fodder cover | `projected_reading_modification_still_vetoes_the_firewall` | drop `\|\| ...projected` | ✅ **MEASURED (M9)** |
| `Effect::Token.keywords` is read-bearing | `ability_scan.rs:447` | the firewall | `token_effect_with_growing_cost_keyword_reads_sibling` | bind `keywords` to `_` | ⬜ planned |
| `ManaProduction`'s count-less variants read | `ability_scan.rs:862` | the firewall | `mana_production_distinct_colors_among_permanents_vetoes` | descend only `count` (Rev 3's design) | ⬜ planned |
| `AddCounterOnEnter` is read-bearing | `scan_continuous_modification` | the firewall | `add_counter_on_enter_modification_reads_sibling` | bin it read-free | ⬜ planned |
| a token-driven loop does not misbind | the drive re-find | `apply(ActivateAbility)` | `activation_loop_token_driver_does_not_misbind` | revert to `card_id` + `min_by_key` | ⬜ planned |
| the ability index is re-validated | the drive | `apply(ActivateAbility)` | `activation_loop_two_auras_ability_index_is_revalidated` | delete the `Eq` check | ⬜ planned |
| hidden-zone content cannot change the offer | scans (1)/(3)/(4) | the offer | `hidden_zone_content_does_not_change_the_offer` | revert P4 | ⬜ planned |

**Identity / Provenance Contract (check 10).** *"The repeated action that drives this loop."*
**Authority:** `LoopActionContext`. **Bound at:** the `ActivateAbility` beat (`engine.rs:3286`) / the recast's cost
beat (`casting_costs.rs:6795`). **Selected id:** ⭐ **`ObjectId`** for `Activate` (a stable battlefield permanent —
survives the clone AND the growth; ⛔ **never `CardId`**, which is `CardId(0)` for *every* token) and **`CardId` +
`from_zone`** for `Recast` (the card churns incarnations per CR 400.7 — a re-find is **required** there).
**Live vs snapshotted:** the `(source_id, ability_index)` pair is **snapshotted**; the `AbilityDefinition` it names is
**re-validated LIVE by `Eq` at every drive iteration** (G4) — a layer re-eval that changes the granted set ⇒
**`Err(RecastAbort)`, fail-closed.** **Stored:** `GameState::last_loop_action_context` (`#[serde(default)]` +
`#[serde(alias)]`, gated on `.samples()` so `Off` never writes it — #4603). **Consumed:** `engine.rs:450` (bridge (B)),
`:1656` (offer), `:1451` (drive), `resource.rs:662` + `:1444` (the two cover conjuncts). **Invalidation:**
`engine.rs:1078` / `:1817` / `:1977`. **Multi-authority hostile fixture:** ⭐ **two auras granting two abilities to one
creature** (`activation_loop_two_auras_ability_index_is_revalidated`).

---

## 9. Deferred / filed — **NOT fixed here**

- **DEFERRED-1 — `LoopDetectionMode::On` in a real game.** The drain path auto-wins with no offer window (CR 732.2a
  does not sanction that as a *game* mode). **User-deferred. A frontend question first.**
- **DEFERRED-2 — `fire_time_conditions_read_projected_resource` (`resource.rs:2152`) has no `modifications` scan.**
  Pre-existing latent gap on the `:784` ω-cover. **P2-d preserves today's incidental protection on the object/fodder
  covers** (U3/M9 proves the term is what does it); it does not extend the projected twin.
- **DEFERRED-3 — `mandatory` is computed at an INTRA-CYCLE INSTANT, not over the CYCLE.** CR 104.4b (`:366`):
  *"Loops that contain an optional action don't result in a draw."* Feeds the Path B DRAW gate ⇒ a live **false-DRAW
  hazard**. **Not currently exploitable via the ring** (the ring's delta is always zero, so Path B's `is_net_progress`
  conjunct rejects first) — which is why it is **filed, not fixed**. ⛔ **It MUST be analysed before anything widens
  the ring or relaxes Path B.**
- ⭐ **DEFERRED-4 — NEW: three missing walkers.** `ReplacementDefinition`, **`StaticMode` (119 variants)**, and
  `ParsedCondition` have **no `ability_scan` walker**. Until they exist, `CopyValues` / `AddStaticMode` /
  `GrantStaticAbility` **fail closed to `CONSERVATIVE`** (P2-a). Building them would unlock the copy/grant-static
  class. **Out of scope — this plan fails closed instead.**
- ⭐ **DEFERRED-5 — NEW: `ordering_parity_sweep` does not cover `ability_scan`.** The Rev-3 B3 regression would have
  landed **silently green**. P0 makes it structurally impossible, and P0's three tripwires guard it — but the sweep's
  blind spot is real and remains.

---

## 10. UNVERIFIED — **things I did NOT measure**

| # | Claim | Why unverified |
|---|---|---|
| **U1** | **Scan (3)'s replacement zone-of-function authority.** `ReplacementDefinition` has no measured zone field. | I did not find one. **P4's implementer must determine it first** and file the gap if none exists. |
| **U2** | **The `ScanMode` threading itself, in its SHIPPED form.** | I measured the **SEMANTICS** of the split via a **thread-local stand-in** in the probe (identical divergence points, identical results). **The shipped form is an explicit `mode: ScanMode` parameter.** The parameter threading is mechanical and compiler-enforced, but **I did not write all ~30 signatures.** |
| **U3** | ~~the P2-d `projected` term is load-bearing~~ | ✅ **DISCHARGED — MEASURED LOAD-BEARING (M9).** |
| **U4** | **The 33 "read-free" `ContinuousModification` variants.** | Classified by **MEASURED payload type** (no `QuantityExpr`/`TargetFilter`/`Keyword`/`AbilityDefinition`/`StaticMode`/`StaticDefinition` in any of them — that IS a measurement). But I did **not** execute each arm. **The compiler enforces completeness; the per-variant judgement is the implementer's to re-check.** |
| **U5** | **The ~170 "read-free" `Keyword` variants.** | Same: classified by measured payload shape, **with the semantic surface delegated to the shipped exhaustive `keyword_cost_reads_growing_class`.** I did not execute each arm. ⚠️ **This is the largest hand-classification surface in the plan (198 arms) and the most likely place for an implementer to introduce a false certificate.** |
| **U6** | **P6's descent on real Commander boards.** | Not measured. Scan (6) is absent from the canary's limb list, which is all I verified. |
| **U7** | **`FORGE_TEST_FULL_DB=1 ordering_parity_sweep` delta.** | **NOT RUN.** Expected ZERO (P0 changes no `Conservative` arm), but **unmeasured**. |
| **U8** | **`ShortcutDecisionSchema { iteration_count: Fixed(1) }`.** | ⚠️ **OBSERVED in the live canary certificate.** I did **not** verify that `DeclareShortcut { count: Fixed(N) }` materializes **N** tokens for the *activation* shape. **The implementer must confirm** (it is Rev 3's `activation_loop_declare_shortcut_materializes_n_tokens`, which I did not run). |
| **U9** | **Perf of the capture gate.** | The static predicate walks the ability's effect chain on **every non-mana activation** — cheaper than a clone-drive, but **not benchmarked**. *(Rev 3's `battlefield.len()` was O(1) but semantically dead.)* |
| ⭐ **U10** | **`batch_conflict == false` for token/mana-bodied triggers.** *(inherited from the Rev-3 review — still a CODE TRACE, NOT A RUN)* | **MOOT for Rev 4 by construction** (§P0: `c2_order_independent` is byte-unchanged ⇒ the conjunction is invariant for *any* `batch_conflict`). ⛔ **But it is NOT discharged, and it becomes LOAD-BEARING the instant any future revision touches a `Conservative`-mode arm.** Runtime fixture then required: **two identical *"whenever this creature deals combat damage to a player, create a 1/1"* creatures** (a distinct-event C2 group). *The group shape is confirmed reachable in code (`triggers.rs:23237`); **no printed card was ever bound to it.**_ |
| **U11** | **A resolution-beat capture variant.** | **Not built and not raced.** The static predicate is measurably **sufficient** (M7 + M8), so there was no residual behavior for it to fix. Recorded so the choice is visible rather than assumed. |
| **U12** | **M3's PRE-FIX 3-limb firewall list.** | **NOT re-derived.** The probe worktree ships P2/P3 **already applied**, so I only ever observed the *post*-fix state. Rev 3 reported the pre-fix limbs as `{S1Trigger, S2BattlefieldBody, S4StaticModifications}`; **that is inherited from its prose, not re-measured by me.** Seeing it would require reverting P2/P3 in a scratch copy. **Not load-bearing for any Rev-4 claim.** |

---

## 11. Acceptance criteria

1. ⭐ **The canary OFFERS.** `WaitingFor::LoopShortcut`, `unbounded` naming `TokensCreated`. ✅ **MEASURED.**
2. ⭐⭐ **The negative twin does NOT offer — AND ITS CAPTURE STILL ARMS.** Without the second half, criterion 1 is
   vacuous and criterion 2 is worthless. ✅ **MEASURED.**
3. ⭐⭐ **`Off` IS BYTE-IDENTICAL (#4603).** `c2_order_independent` for a token-bodied trigger is **`false`**
   (PROMPTS), and **`event_and_sibling_axes_unchanged_for_typed` passes UNMODIFIED.** ✅ **MEASURED.**
4. ⭐⭐ **`projected_reading_modification_still_vetoes_the_firewall` FLIPS TO FAIL** when the `|| projected` term is
   deleted. ✅ **MEASURED.**
5. **Gaea's Cradle STILL REJECTS — via a measured `Effect::Mana` READ, not a blanket.** The test must assert the
   **axis**, not just the rejection.
6. **`token_effect_with_growing_cost_keyword_reads_sibling` FLIPS TO FAIL** when `keywords` is bound to `_`.
7. **`mana_production_distinct_colors_among_permanents_vetoes` FLIPS TO FAIL** under Rev 3's "descend `count`" design.
8. **`hidden_zone_content_does_not_change_the_offer` FLIPS TO FAIL** when P4 is reverted.
9. ⭐ **The full suite is green with ZERO failures**, and `event_and_sibling_axes_unchanged_for_typed` is **NOT
   re-authored**. ✅ **MEASURED: `16550 passed; 0 failed`** *(Rev 3's baseline was `16547 passed; 1 failed`).*
10. **`FORGE_TEST_FULL_DB=1 cargo test -p engine ordering_parity_sweep` delta = ZERO.**
11. **`cargo fmt --all`** + Tilt `clippy` / `test-engine` / `card-data` green.
