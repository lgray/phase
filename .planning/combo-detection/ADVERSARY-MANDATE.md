> # ⛔⛔ STALE — HISTORICAL ONLY. DO NOT ACT ON THIS DOCUMENT.
> **Superseded 2026-07-14 by [`LOOP-SHORTCUT-SPEC-AND-STATE.md`](./LOOP-SHORTCUT-SPEC-AND-STATE.md).**
>
> Every `file:line` below was measured against a tree **768 commits behind `main`** and **no longer resolves
> there.** Several central claims are **refuted by measurement on `main`** — including *"there is no live
> object-growth path"* and *"the offer carries no iteration count"*, **both FALSE on `main` today**
> (`game/engine.rs:1656`, `analysis/decision_template.rs:203`).
>
> **The engine described here no longer exists.** PR-7 shipped most of the machinery; the live blocker is
> REACHABILITY — the fail-closed covers veto on any real board. See the successor doc, §4.
>
> ## ⇒ Read this for the **RULES** reasoning (which has held — 40/40 CR citations) and the **SOUNDNESS** rules.
> ## ⇒ **NEVER for a code fact.**

---

# Adversarial review mandate — `combo-plan-adversary`

**File relay.** Messaging has dropped bodies repeatedly in this session. This file is the authoritative
copy. Read it from the MAIN tree: `/home/lgray/vibe-coding/phase-rs-workdir/`.

**TARGET:** `.planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md` @ **`e677fefb1`** — **FROZEN. It will not
move while you review.** 1,865 lines, phases P0–P10, 18-row Appendix B.

**CONTEXT (read both):** `.planning/combo-detection/PLANNER-BRIEF.md` (the doctrine that produced it) and
the plan's own **Appendix B** (18 errors — read it first; it tells you exactly how this plan fails).

---

## THE FRAMEWORK IS THE REPO'S, NOT YOURS

**You MUST apply `.claude/skills/review-engine-plan/SKILL.md` — all ELEVEN required checks**, and reject on
its stated criteria. Read it before you read the plan. Its checks 1 (class vs card), 3 (trace), 9
(verification matrix), 10 (identity/provenance) and 11 (scope matrix) are the ones this plan is most likely
to fail. Also consult `.claude/skills/engine-planner/SKILL.md` for the mandatory architectural sections a
plan owes, and `.claude/skills/add-engine-variant/SKILL.md` if the plan proposes any new enum variant.

**This is an ARCHITECTURAL GATE, not a proofread. Reject the plan if any required dimension is missing,
superficial, or contradicted by code evidence.**

---

## THE PRIME DIRECTIVE OF THIS WORKSTREAM

> **19 errors so far. EVERY SINGLE ONE was a CODE claim asserted from memory.**
> **The RULES layer has never failed once — 40/40 CR citations, 32/32 Oracle texts, across six audits.**

**⇒ Attack the CODE claims. Grep before you believe. Put the `file:line` in your sentence.**
Three of the 19 were committed *while writing the document*, by two different agents, and **every one was
caught the same way: someone re-measured.** Be that someone.

**Tilt is OFF (the user killed it). You MAY and SHOULD run `cargo test -p engine ...` and `cargo check`.**
Four earlier review rounds read code and executed nothing; **that was the single biggest gap.** Runtime
measurement beats any amount of reading.

---

## HIGHEST-VALUE REFUTATION TARGETS (ranked — attack in this order)

### ⭐ 1. THE SOUNDNESS ASYMMETRY IS THE PLAN'S LOAD-BEARING CLAIM. TEST IT.
The plan asserts: *"a coarse relation may REJECT, never ACCEPT"*, and therefore **"P4 and P7 are the ONLY
two phases that move the detector in the ACCEPT direction; every other phase errs safe."**

**ENUMERATE ALL ELEVEN PHASES (P0–P10) AND CLASSIFY EACH BY DIRECTION YOURSELF.** Does any *other* phase
also remove a rejection?
- **P5 (bounded transient) relaxes the cover** — does it not ACCEPT more? The plan claims it errs safe. **Prove or refute.**
- **P10 (RC-4, object identity / scalarset)** — a coarser state equality **accepts more**. The plan claims
  normalization-first errs fine. **Does it?**
- **P8 (C5 deferred execution) / P9 (C6 ∞-fixpoint)** — new ADMIT paths?
- **P0 (delete `LoopDetectionMode::On`) / P1 (split `WinKind`)** — claimed pure refactors. **Are they?**

**If even ONE other phase relaxes a rejection, the plan's central risk table is WRONG and the review is a
BLOCKER.** This is the single highest-value check you can run.

### ⭐ 2. IS P7's CLASS GATE ACTUALLY DISCRIMINATING? **BUILD BOTH FIXTURES AND RUN THEM.**
The plan's acceptance criterion for its biggest phase is: **Intruder Alarm un-rejects AND Gaea's Cradle
still fails closed.** Do not take this on faith.
- Construct the Intruder Alarm shape (`SetTapState { target: Typed[Creature], scope: All, state: Untap }`)
  and the Cradle shape (`ManaProduction::AnyOneColor { count: Ref(ObjectCount{Creature}) }`).
- **Flip `TargetFilter::Typed`'s `sibling: true → false` (`ability_scan.rs:2456`) IN A THROWAWAY WORKTREE and
  RUN BOTH.** Does the pair actually separate? **Or does the flip also un-reject Gaea's Cradle** — which
  would make it a HOLE, not a fix, in the catastrophic direction?
- **REVERT-PROBE:** with the flip reverted, does the positive guard FLIP TO FAIL? If it still passes, the
  gate is **vacuous** and P7's acceptance criterion is worthless. *(This plan has already shipped one vacuous
  discriminator in its own root-cause claim — Appendix B #17. Assume it shipped another.)*

### ⭐ 3. THE `~88 SITES` NUMBER — IS IT REACHABLE, OR INFLATED?
Measured: **57 `Axes::CONSERVATIVE` + 31 `sibling: true` in `ability_scan.rs`.** But **how many are actually
reachable from the loop detector's consumption path** (`ability_reads_sibling_mutable` /
`scan_target_filter` / `scan_effect`, consumed by `resource.rs` gates 1–4)? **If most are unreachable, the
"one-sided ratchet, 88 clicks" framing is inflated and P7 is mis-sized.** The plan files its true size as
**UNVERIFIED (§8 Q0)** — **check that it does not silently assume a size ANYWHERE else in the document.**

### 4. P4's "CALL THE AUTHORITY" — DOES IT TYPECHECK, OR IS IT HAND-WAVED?
The plan says: factor the functioning clause out of `object_replacement_candidate_applies`
(`replacement.rs:4829`, whose gate is the five-clause block at `:4890-4896`) into a shared predicate taking
`event: Option<&ProposedEvent>`, where `None` = analysis time = fail-CLOSED.
- **Is that refactor actually mechanically possible?** The five clauses are *event-derived*
  (`entering_object_id`, `discarding_object_id`, `stack_self_moving_object_id`, dredge-on-`Draw`). **Can they
  be evaluated with `None`?** What does "fail-closed" concretely mean for each — and does the plan SAY, or
  does it wave?
- **Direction check:** gate (3) REJECTS. **Confirm that the analysis' new predicate is a SUPERSET of the
  runtime's functioning set** (over-inclusive = safe). **If it is a subset anywhere, that is a
  FALSE-CERTIFICATE GENERATOR and a BLOCKER.** The plan already made this exact error once (#18).

### 5. DOES EVERY PHASE DISCHARGE A **CLASS**? (the user's governing rule)
> *"Combo A and Combo B are ACCEPTANCE TESTS, not GOALS. A change that turns a combo green without
> discharging a class property IS the purpose-built patch this plan exists to prevent."*

**Audit each of P0–P10 against this.** Does any phase secretly fix the user's two cards? **Is any phase's
"class gate" satisfiable by the canary alone?** Per `review-engine-plan` check 1: **reject one-card work.**

### 6. VERIFICATION MATRIX (`review-engine-plan` check 9) — THE MOST MECHANICAL REJECTION
- Every behavioral claim mapped to a test? **Every negative assertion paired with a positive reach-guard?**
  (A bare `!detector(...)` that an upstream short-circuit satisfies vacuously is **not a test** — and this
  workstream has already been burned by exactly that.)
- **Revert-failing assertions named** for each? Hostile fixtures? Sibling/negative cases?
- Does any planned test go through a helper when the production path runs through `apply()` /
  `WaitingFor`/`GameAction` / the stack? **Check 9 rejects helper-only tests for those.**

### 7. APPENDIX B IS ITSELF A CODE-CLAIM DOCUMENT. **SPOT-CHECK IT.**
**Pick at least 3 of the 18 error entries and verify them against the code.** If an entry in the
*error catalogue* is itself wrong, that is the most damning finding available — and it is exactly the
failure mode the catalogue exists to prevent.

### 8. CR VERIFICATION (`review-engine-plan` check 7)
**Grep EVERY CR number in the plan against `docs/MagicCompRules.txt`.** The rules layer has never failed —
**so if you find a bad CR number, that is genuinely new and important.** Especially check the load-bearing
ones: **CR 732.2a/b/c, 113.6, 400.7, 614.12, 608.2n, 702.52a/b, 104.4b, 302.6, 603.7, 702.10.**

---

## RULES (non-negotiable)

1. **REVIEW ONLY. MODIFY NO FILES.** Zero edits under `crates/`. Zero edits to the plan.
   **If you need to flip a line to measure something, do it in a THROWAWAY WORKTREE and tear it down.**
   Verify before you finish: `git -C /home/lgray/vibe-coding/phase-rs-workdir status --short -uno` must show
   **no `crates/` files**. *(`client/src/wasm/engine_wasm.d.ts` is pre-existing and NOT ours — never touch it.)*
2. **Cite `file:line` and CR numbers for EVERYTHING.** A claim without a citation is not a finding.
3. **Mark anything you did not check `UNVERIFIED`. NEVER fill from memory.** An honest *"did not reach this"*
   beats a plausible claim that costs a cycle to refute. **This is the rule the plan broke 19 times.**
4. **You MAY spawn your own isolated sub-agents** to parallelize (they are read-only; there is no race).
   Give them meaningful names. **Do NOT let two of them flip the same production line at once** — that
   already voided one measurement in this workstream. Worktree-isolate any probe that flips a line.
5. **PUT THE ENTIRE REPORT IN YOUR FINAL RESPONSE MESSAGE, NOT IN A FILE.** Two reviewers' reports have
   already been lost to messaging in this session.
6. **Do not soften.** A well-evidenced **REJECT** is worth more than a polite pass. The plan's author and
   the team-lead have both been wrong repeatedly and **want** to be caught.

## OUTPUT

Lead with **BLOCKERS**, then material gaps, then nits. For each: **the passage (§ + line) → the evidence
(`file:line`, test output) → the required revision.** End with an explicit verdict: **ACCEPT / ACCEPT WITH
REVISIONS / REJECT**, and name every residual assumption you could not discharge.
