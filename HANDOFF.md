# Combo detector (CR 732.2a) — probe & evidence branch

> # ⛔ THIS BRANCH IS **EVIDENCE, NOT A MERGE CANDIDATE.**
> **Nothing here is production code.** It exists so the plan's claims can be **re-run**, not merely read.
> The plan itself lives on **`debug/combo-generator`** → `.planning/combo-detection/COMBO-DETECTOR-LIVE-PLAN.rev4.md`.

**Base: `efc76ca1b`** (the exact commit every measurement was taken against). Branch cut from a detached worktree.

---

## The bug, in one line

**The engine's loop detector rejects the Comprehensive Rules' own worked example of the rule it implements.**
CR 732.2a's Example is **Presence of Gond + Intruder Alarm** (`docs/MagicCompRules.txt:6373`) — an *activated-ability*
token loop in which **no spell is ever cast**.

## What this branch proves (all re-run by team-lead, not taken on trust)

| Claim | How to re-run | Result |
|---|---|---|
| **The canary OFFERS** *(first time in this workstream's history)* | `cargo test -p engine --test integration probe_rev4 -- --nocapture` | ✅ `WaitingFor::LoopShortcut`, `unbounded=[TokensCreated]`, `win_kind=Advantage` |
| **The negative twin does NOT offer** — and is **not vacuous**: it **arms the capture** (`ctx.is_some()`) and still declines, discriminating at the **drive** (2nd activation illegal ⇒ `RecastAbort`), not upstream | same command | ✅ capture `true`, offer `false` |
| **`Off` is byte-identical (#4603)** — the shared CR 603.3b trigger-ordering gate is **untouched** | `cargo test -p engine --lib event_and_sibling_axes_unchanged_for_typed` | ✅ **GREEN and byte-UNMODIFIED** |
| **No regression** | `cargo test -p engine --lib` | ✅ **`16550 passed; 0 failed`** |

> **The `#4603` guard is the load-bearing one.** An earlier revision had to **re-author** that guard to make its change
> pass. This one does not touch it. *A guard you must rewrite to make your change pass is a guard telling you something.*

---

## ⛔⛔ THREE TRAPS — read before you run anything

### 1. `probe_review_r1.rs::r2_min_by_key_refind_picks_the_wrong_permanent` **FAILS BY DESIGN.**
Its `assert_ne!` encodes a hypothesis that was **refuted**. **THE FAILURE *IS* THE EVIDENCE.** Do not "fix" it.
*(Refuted for real cards — `CardId` is minted per physical card, `scenario.rs:229`. **But REAL for tokens:** every
plain token gets `CardId(0)` (`effects/token.rs:813`, `:1060`), so a token-driven loop's `card_id` re-find matches
every token the player controls, **including its own fodder**. The plan fixes this by pinning on **`ObjectId`**.)*

### 2. The engine delta uses a **`thread_local!` STAND-IN** (`ability_scan.rs:351`), **NOT the shipping design.**
The plan (rev4 §P0) mandates an explicit **`ScanMode { Conservative, LoopFirewall }`** threaded through **~30
signatures**. The prototype emulates that split with hidden global state **to measure the semantics only**. It is
**semantically equivalent at every divergence point and the full suite is green — but hidden global state is not
shippable.** This is the plan's **U2**, and it is **the first thing an implementer must write.**

### 3. This tree ships the firewall fix **already applied.**
So the *pre-fix* three-veto firewall state (`{S1Trigger, S2BattlefieldBody, S4StaticModifications}`) is **not visible
here** — you must revert the `resource.rs` / `ability_scan.rs` delta in a scratch copy to observe it.
**Do not infer the pre-fix state from prose. That habit is what produced ~25 errors in this workstream.**

---

## Commit layout

1. `docs` — this file.
2. **`PROTOTYPE (DO NOT MERGE)`** — the engine delta, isolated so it can be read or dropped in one step.
   *(The harnesses in commit 3 depend on its `probe::` reporters, so reverting it breaks them. That is expected: the
   branch is a laboratory, not a stack of mergeable patches.)*
3. `test` — the harnesses: `probe_canary_gond.rs` (the live canary, driven through the **real reducer** —
   `GameAction::ActivateAbility` → `PassPriority`), `probe_rev4.rs` (B1 ±, U3), `probe_review_r1.rs` (the review's B1/B3 probes).

## The four findings a reader should not have to re-derive

1. **REACH is the real gap.** The only path that offers object growth arms **solely on a buyback-paid, token-creating
   SPELL** (`casting_costs.rs:6795`). The canary casts nothing. The fix is the **activated-ability dual of that static
   predicate** — *not* a runtime `battlefield.len()` delta, which is **provably dead** at that beat (an activated
   ability only reaches the **stack**, CR 602.2a).
2. **The ring can never see this loop — by design, not by bug.** `ActivateAbility` trips the deliberate-action clear
   (`engine.rs:3089`); the resolving trigger trips the empty-stack clear (`:2325`). The ring is an instrument for
   **mandatory self-refilling cascades**; CR 732.2a's subject matter is **a player deliberately repeating a sequence of
   choices** — the opposite.
3. **`ability_scan`'s `Axes` is a SHARED authority, not the firewall's private predicate.** `triggers.rs:3893-94` uses
   it for the live CR 603.3b gate with **no `loop_detection` guard** — see the repo's own
   `pr625_c2_distinct_event_auto_orders_even_when_loop_detection_off` (`triggers.rs:23237`). **Making the shared blanket
   precise in place silently flips real games from *prompt* to *auto-order*.** Hence `ScanMode`.
4. **A `_ =>` wildcard is only a hole when its value is the ACCEPTING one.** `_ => CONSERVATIVE` is **fail-closed and
   safe**; `_ => NONE` is the false certificate. This is why the plan **reuses** the shipped
   `keyword_cost_reads_growing_class` authority rather than hand-classifying **198** `Keyword` arms — hand-classifying
   them would be a false-certificate factory. *(`Convoke`/`Delve`/`Improvise`/`Bargain`/`Station` are **unit** variants
   that **read the board** — payload shape alone is unsound.)*

## The governing rule, which outranks everything above

> **A coarse relation may REJECT, never ACCEPT.**
> Too coarse ⇒ a **false certificate** ⇒ **a real game ends wrongly.** Too fine ⇒ a missed offer ⇒ **safe.**

The plan's P0/P1/P2/P3 all move the detector toward **ACCEPT**. Those are the lines to review twice.
Open items are catalogued honestly in the plan's **UNVERIFIED** section (U1, U2, U5, U7, U8) and its
**DEFERRED** list — including **DEFERRED-3**, a *separate* live rules bug found by measurement: `mandatory` is
computed at an **intra-cycle instant** rather than over the **cycle**, a CR 104.4b false-DRAW hazard.
