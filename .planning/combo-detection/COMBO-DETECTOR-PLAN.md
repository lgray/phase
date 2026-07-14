# Combo detector — the plan *(Rev 1)*

> # ⛔⛔ SUPERSEDED — DO NOT IMPLEMENT. ITS ROOT CAUSE IS **MEASURED FALSE**.
> ## ⇒ **The plan of record is [`COMBO-DETECTOR-LIVE-PLAN.rev4.md`](./COMBO-DETECTOR-LIVE-PLAN.rev4.md).**
> **Lineage:** Rev 1 *(this doc)* → [`COMBO-DETECTOR-PLAN-REVIEW.md`](./COMBO-DETECTOR-PLAN-REVIEW.md) **(REJECT, 8
> blockers)** → [`COMBO-DETECTOR-PLAN-REVISED.md`](./COMBO-DETECTOR-PLAN-REVISED.md) *(Rev 2)* →
> [`COMBO-DETECTOR-LIVE-PLAN.rev3.md`](./COMBO-DETECTOR-LIVE-PLAN.rev3.md) →
> [`COMBO-DETECTOR-LIVE-PLAN.rev3-REVIEW.md`](./COMBO-DETECTOR-LIVE-PLAN.rev3-REVIEW.md) **(REJECT, 5 blockers)** →
> **Rev 4**.
>
> **What this document got WRONG (each refuted by running the engine, not by argument):**
> 1. ⛔ **§3's root cause — "the covers read HIDDEN ZONES" — is NOT the blocker.** The CR 400.2 hidden-zone leak is
>    **real** and worth fixing, but the canary's **three vetoes are ALL BATTLEFIELD-RESIDENT.** Scoping the covers to
>    visible zones unblocks **ZERO**. §4's P2 header — *"(the rules fix — AND the reachability fix)"* — is **false**.
> 2. ⛔ **The real gap is REACH**, which this document never mentions: the only path that offers object growth arms
>    **solely on a buyback-paid, token-creating SPELL** (`casting_costs.rs:6795`), and CR 732.2a's own worked example
>    **casts nothing.**
> 3. ⛔ **§4's P4 ("DELETE `LoopDetectionMode::On`") is not a refactor** — `On` ships in the `combo-verify` binary and
>    crosses the WS protocol, the WASM bridge, saved games and localStorage. **User directive: all three modes stay.**
> 4. ⛔ **§5's Gaea's Cradle acceptance criterion is VACUOUS** — it fails closed via an unrelated `Effect::Mana`
>    blanket and would still pass with the `sibling` axis deleted entirely.
>
> **What it got RIGHT and what the successors kept:** §0 (the detector is **opt-in**; `Off` is the default and IS the
> CR 732.2a opt-in), §1 (the five-stage spec, and the ⭐ observation that **the rules never require state recurrence** —
> the rulebook's own example adds a token every iteration), §2's conclusion (**not a capability problem — the detector
> cannot be REACHED**), §5's doctrine (**the combos are CANARIES, not GOALS**), and §6's soundness rule
> (***a coarse relation may REJECT, never ACCEPT***). **Its RULES reasoning has never failed a single audit.**

**2026-07-14** · Every code citation measured against **`main` @ `efc76ca1b`**.
*(The six prior docs are **STALE** — written against a tree 768 commits behind `main`. Read them for rules
reasoning only, never for a code fact.)*

---

## 0. What we are implementing

**Shortcutting a loop is OPTIONAL.** CR 732.2a: the player *"**may** suggest a shortcut."* Nobody is compelled to
propose one, and no opponent is compelled to accept the proposed count.

> ## ⇒ **THE DETECTOR STAYS OPT-IN. Turning it on IS the table agreeing to use the optional shortcut rule.**
> **`LoopDetectionMode::Off` is not dead code and is not a wart — it is the "we are not shortcutting loops in this
> game" setting, and it must remain the default.**

**Not gated on Rules Enforcement Level.** MTR §4.4 carries **zero** REL qualifiers (measured: 0 hits for
`Competitive|Professional|Regular|Enforcement` across its 49 lines), and the *no-conditional-actions* core is in
**CR 732.2a** itself. The regime is identical at Regular / Competitive / Professional and in casual play.
**No "tournament mode." Nothing to gate. The opt-in above is the only switch that exists.**

---

## 1. The spec — the whole feature, in five stages

| | Stage | Rule |
|---|---|---|
| **1** | **CAPTURE** the player's performed actions, **as FIXED choices** — a loop is a *sequence of actions*, not a decision tree. | CR 732.2a: *"a sequence of game choices… **can't include conditional actions**."* |
| **2** | **REPEAT** that exact sequence; determine whether it yields an **unbounded resource**. | CR 732.1b: a loop is *"a set of **actions** [that] could be repeated indefinitely."* |
| **3** | **CLASSIFY** it — **ADVANTAGE** or **WIN** *(and **DRAW**, which the rules add: CR 104.4b — a **mandatory** loop with no way to stop; "loops that contain an optional action don't result in a draw")*. | CR 704.5a · CR 104.4b |
| **4** | **PRESENT** it to the player. If accepted, **pass priority around the table** so every opponent may interact or shorten it. | CR 732.2b/c · MTR 4.4 |
| **5** | If accepted and un-interacted-with: **emit the certificate and APPLY the state changes.** | CR 732.2a |

> ### ⭐ The omission that is the whole design — and it is correct
> **The spec says "repeat the ACTIONS." It never says the game state must return to where it started.** Neither
> does the rulebook. **CR 732.2a's own worked example — Presence of Gond + Intruder Alarm
> (`docs/MagicCompRules.txt:6373`) — ADDS A TOKEN EVERY ITERATION**, so its state provably never recurs, and the
> rules shortcut it a million times.
>
> ## ⇒ **A detector that requires STATE RECURRENCE must reject the rulebook's own worked example.**

---

## 2. PR-7 already built all five stages — **DO NOT REBUILD ANY OF THIS**

| Stage | On `main` | Where |
|---|---|---|
| **1 · capture** | `last_recast_context` + `loop_detect_ring` | `game/engine.rs:450`, `:537` |
| **2 · repeat** | **Drives the captured sequence on a CLONE** (2 iterations, 3 settle frames, re-entrancy-guarded) — a real replay, not a static re-derivation | `try_offer_object_growth_shortcut`, `game/engine.rs:1656`, `:1688` |
| **2 · unbounded** | object-growth cover (~40 assertions) · counter-growth cover | `analysis/resource.rs:924` · `:784` |
| **3 · win / draw / advantage** | Path A (CR 704.5a; multiplayer-safe — exactly one non-faller, CR 104.2a) · Path B (CR 104.4b/732.4) · `WinKind::Advantage` | `game/engine.rs:498`, `:536` · `analysis/loop_check.rs:83` |
| **4 · present** | `WaitingFor::LoopShortcut` + `IterationCount::{Fixed, UntilLethal}` | `types/game_state.rs:4458` · `analysis/decision_template.rs:203` |
| **4 · accept / decline / interact** | `DeclareShortcut { count, template }` · `DeclineShortcut` · `RespondToShortcut` (APNAP, multiplayer-shaped) | `types/actions.rs:834` · `game/engine.rs:4376`, `:1992` |
| **5 · apply** | `apply_confirmed_shortcut` → `apply_until_lethal_shortcut` / `materialize_fixed_shortcut`; re-validates proposer + winner at consumption (CR 800.4a) | `game/engine.rs:855`, `:906`, `:1325` |
| **— · reject non-deterministic loops** | static gate + runtime backstop (CR 705.1 / 706.1a / 701.9a-b) | `game/engine.rs:1684` · `ability_scan.rs:4407` |

> ## ⇒ **The pipeline is complete end to end. It is not a capability problem. The detector cannot be REACHED.**

---

## 3. ⛔⛔ THE ROOT CAUSE — the covers read HIDDEN ZONES, and that is rules-wrong twice

**The fire-time firewall** — `fire_time_conditions_read_growing_class` (`analysis/resource.rs:1457`), consumed by
**two** gates (`:968`, `:1131`) — **vetoes the entire detection if ANY object in scope has an ability that reads a
"sibling-mutable" axis.** It has four scans. **Gate (2) is battlefield-scoped. Gates (1), (3) and (4) iterate
`state.objects.values()` — every object in EVERY zone, including LIBRARY and HAND.**

**And nothing downstream saves them:** `functioning_abilities::active_trigger_definitions`
(`game/functioning_abilities.rs:391`) — despite the name — **applies no zone filter at all** (one Command-zone
special case, then `true`). A trigger definition on a card **in the library** is returned as "active."

### Why that is wrong — two independent rules, either one fatal

1. **CR 113.6** (`docs/MagicCompRules.txt:771`): abilities of non-instant/sorcery objects *"usually function only
   while that object is on the battlefield"* (113.6b: an ability that names its zones functions only from those).
   **An ability in the library does not function. It cannot fire during the loop. Scanning it is not
   conservatism — it is scanning an ability that does not exist.**
2. ## ⭐ **CR 400.2** (`:1935`, verbatim): ***"Library and hand are hidden zones, even if all the cards in one such zone happen to be revealed."***
   **A cover may only read VISIBLE board state.** The offer's *presence or absence is itself observable to every
   player.* If it depends on hidden-zone contents, **the engine leaks hidden information into observable game
   state.** **That is not a conservative approximation. It is a rules violation.**

### The composition failure — this is why the feature is dead

- **Individually, every fail-closed default is defensible.** Failing closed can only ever cost a **missed offer**;
  it can never falsely certify a loop and wrongly end a game. **And each was FREE** — `sibling: true` costs a
  contributor nothing and trips no test. **Measured on `main`: 84 such sites in one file** (54 `Axes::CONSERVATIVE`
  + 30 `sibling: true`; `ability_scan.rs:2420`, whose own comment says they *"stay CONSERVATIVE"*).
- **Composed, they are a near-certain veto.** The firewall is a **disjunction over every object in scope**, and a
  4-player Commander board holds ~100 permanents **plus four ~90-card libraries.** **One conservative arm anywhere
  — even face-down in a library — kills detection for the whole table.**

> ## ⇒ **The covers wrote the combo detector OUT OF THE GAMEPLAY LOGIC BY CONSTRUCTION.**

**The suite could not see it, for two compounding reasons:**
1. **Its fixtures build boards that cannot exist** — no lands, empty library, stub oracle. **The detector was only
   ever exercised on boards with nothing on them.**
2. **The guards are a ONE-SIDED RATCHET.** There is a discriminating **negative** guard (*Gaea's Cradle MUST fail
   closed* — it **counts** a mutable creature set, `ability_scan.rs:4840`) and **no discriminating positive guard**:
   the only *"must NOT trip"* assertion uses `fixed_drain` = `GainLife { Fixed(1), Controller }`
   (`ability_scan.rs:5215`), **which references no object filter at all.** ⇒ **Over-acceptance is structurally
   detectable; over-rejection is invisible.** The conservative arm always won — 84 times.

---

## 4. The plan

### P1 — Prove the diagnosis before building anything *(≈1 worktree, hours)*
In a throwaway worktree cut from **`main`**: **stub the firewall to ALWAYS ACCEPT**, port the acceptance fixture
+ tests (they exist **only** on `debug/combo-generator` and have **never been run against `main`**), and run them.
- 🟢 **GREEN** ⇒ the covers are the only blocker. Proceed to P2.
- 🔴 **RED** ⇒ something else is broken too. **Instrument it against `main`.** ⛔ **Do NOT inherit the stale plan's
  RC-1…RC-4 — two of their premises are already refuted.**

**This strictly dominates auditing the 84 sites: an audit *assumes* the firewall is the blocker; this *tests* it.**
A worktree has its own `target/`, so it will not contend with Tilt's cargo lock on `main`.

### P2 — Scope every cover to VISIBLE, FUNCTIONING objects *(the rules fix — and the reachability fix)*
Gates (1), (3), (4) of `fire_time_conditions_read_growing_class` must stop iterating `state.objects.values()`.
- **Exclude hidden zones outright** — **CR 400.2**: library and hand. **Non-negotiable; this is the information
  leak.**
- **Scope the rest by where the ability actually FUNCTIONS — CR 113.6.**
  ## ⛔ **CALL the engine's runtime functioning authority. DO NOT hand-roll a zone list.**
  Graveyards are **public** and some abilities **do** function from them (dredge, flashback, escape — CR 113.6b).
  **A hand-mirrored zone predicate already shipped once in this workstream and silently dropped dredge.** Mirror
  nothing; **call the authority the runtime calls.**
- ⚠️ **`active_trigger_definitions` is a SHARED authority** — the live trigger system uses it. **Fix the firewall's
  scope, not that function's contract**, unless a full call-site audit says otherwise.

**Class gate:** *the verdict must be invariant under **any** hidden-zone content.* Shuffle an arbitrary card into a
library ⇒ **the offer must not change.** That is the test, and it is the CR 400.2 property stated as an assertion.

### P3 — Re-derive `sibling` from a POSITIVE definition *(the 84 sites)*
Today `sibling: true` means *"we didn't think about it."* It must mean something:

> **Does this ability COUNT a mutable set (⇒ conservative), or does it merely NAME a type (⇒ not)?**

**Both halves of the acceptance are mandatory:**
> ## **Intruder Alarm must UN-REJECT** *(CR 732.2a's own worked example)* **AND Gaea's Cradle must STILL FAIL CLOSED** *(it counts a mutable creature set)*.
> **A flip that un-rejects BOTH is a HOLE in the catastrophic direction, not a fix.**

**And install the missing positive guard** — an ability that *references a typed object filter but does not count
it* **must not trip** — so the ratchet becomes symmetric. **Without it, the next 84 defaults land the same way.**

### P4 — Collapse the modes to a binary
`LoopDetectionMode { Off, On, Interactive }` (`types/game_state.rs:5942`) → **`{ Off, Interactive }`**.
- **`Off`** — **KEEP. It is the default and it is the opt-in (§0).**
- **`On`** — **DELETE.** It auto-wins a mandatory lethal drain **without offering it**, which is rules-wrong: CR
  732.2a makes suggesting a shortcut **optional** and gives opponents a response window. *(Its only
  production-shaped call sites, `match_flow.rs:669`/`:744`, are inside `#[cfg(test)]` — `mod tests` opens at
  `:360`.)*
- **`Interactive`** — the feature. It becomes the sole active mode.

---

## 5. Acceptance — class-level, not card-level

**The two combos are CANARIES, not goals.** A change that turns them green without discharging a class property is
the purpose-built patch this plan exists to prevent.

| Phase | Class property |
|---|---|
| **P2** | The verdict is **invariant under any hidden-zone content** (CR 400.2). |
| **P3** | **Intruder Alarm un-rejects AND Gaea's Cradle stays fail-closed**, and a *names-a-type-but-doesn't-count-it* ability **does not trip**. |
| **P4** | `Off` still fully restores pre-feature behavior; no path auto-wins without an offer. |

---

## 6. Soundness — the rule that outranks everything above

> **A coarse relation may REJECT, never ACCEPT.** Too coarse ⇒ a **false certificate** ⇒ **a real game ends
> wrongly.** Too fine ⇒ a missed offer ⇒ **safe.**

**P2 and P3 both narrow a REJECT gate ⇒ fewer rejections ⇒ MORE accepts. They are the only phases that move the
detector toward ACCEPT. Review every line of them twice.**

**P2's warrant for moving that direction is not "be less conservative" — it is that the scanned abilities DO NOT
FUNCTION and MAY NOT BE SEEN.** Removing them removes **noise**, not **safety**. **That argument holds only if the
zone predicate is exactly right — which is why P2 forbids hand-rolling it.**

### Do not "fix" these — they are already correct
- ⛔ **No non-determinism gate.** Exists twice (static + runtime), CR-annotated. `engine.rs:1684`, `ability_scan.rs:4407`.
- ⛔ **No REL / tournament-mode toggle.** §0.
- ⛔ **Do not relax `GameState::PartialEq`'s `delayed_triggers` conjunct** — it stops certifying a loop whose growth
  axis dies at the next end step.
- ⛔ **Do not DELETE the covers. Make them PRECISE.** A fail-closed default consumed as a **precise predicate** is
  the defect — not the fail-closed default itself.

---

## 7. Sources

`docs/MagicCompRules.txt` — **CR 104.4b** `:366` · **113.6** `:771` · **400.2** `:1935` · **732.1b** `:6366` ·
**732.1c** `:6368` · **732.2a + Example** `:6372`/`:6373`.
[MTR, eff. 2026-02-27](https://media.wizards.com/ContentResources/WPN/MTG_MTR_2026_Feb27_EN.pdf) §4.2/§4.4 ·
[judge annotations](https://blogs.magicjudges.org/rules/mtr4-4/).

**Superseded:** `LOOP-SHORTCUT-SPEC-AND-STATE.md` (folded in here) and the six STALE docs.
