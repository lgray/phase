# Combo-Detector Series — Deferral Audit (2026-07-04)

**Method:** 5-agent parallel sweep (planning docs · code markers at the PR-6.75 head · GitHub tracker · journals/§7 accounting · catch-all grep), every item quote-backed, synthesized and spot-verified by team-lead (workflow `wf_c69c11e3-c04`, 584k subagent tokens, 112 tool calls).
**Head state at audit:** PR #5072 (PR-6.75) open with CHANGES_REQUESTED, HIGH-1 fix in flight (option C ruled); series worktree `pr675-wt` @ 17c8aa90f + uncommitted HIGH-1 diff.

**Verdict: no silent deferrals.** Every open item is in-flight, ledgered with a named owner/upgrade path, or an explicitly accepted fail-safe conservatism.

---

## 1. In flight right now

| Item | Where | Status |
|---|---|---|
| **PR #5072** — delivery vehicle for two prior deferrals: PR-6.25 R3-deferred order-independence fix + #4904's deferred C0-full+C1 | github.com/phase-rs/phase/pull/5072 | CHANGES_REQUESTED; HIGH-1 fix applied (option C: full discriminator + predicate-keyed over-prompt class for the 48 flips), per-card classifier running; HIGH-2 queued serial |
| **HIGH-1** — same-event member-bound triggers could auto-order (hole in this PR's new CR 603.3b predicate; `source_independent()` omitted `reads_member_bound`) | `ability_rw.rs:745`, `:960` (pr675-wt) | Fix applied, uncommitted; flips 48 corpus cards auto→prompt, ≥1 GENUINE under-prompt caught (Mimic Vat pair) |
| **HIGH-2** — non-phase delayed triggers bypass `begin_trigger_ordering` (pre-existing structural bypass; `check_delayed_triggers` dispatches directly) | `triggers.rs:5148–5329` (byte-identical base↔HEAD) | Queued after HIGH-1; full plan→impl→review pipeline; phase-delayed path (:5547, #5048) is the template |
| **PR #5086 (D6)** — dual-walk discipline docs for `add-engine-variant` skill | github.com/phase-rs/phase/pull/5086 | **NEW FIND (gh-verified):** beyond depending on #5072, matthewevans put a policy CHANGES_REQUESTED — `.claude/skills/**` is direct-maintainer-review only; AI pipeline won't approve/enqueue. Waits on maintainer manually |
| **Issue #4809** — Braids, Arisen Nightmare end-step trigger not batched with simultaneous end-step triggers | github.com/phase-rs/phase/issues/4809 | **NEW FIND:** very likely a live reproduction of the HIGH-2 bypass (delayed "next end step" trigger never joins the PendingTriggerContext batch). Handed to the fix driver as real-card test candidate; close it if the HIGH-2 fix resolves it |
| **Issue #4787** — Conqueror × Enduring Tenacity loop not detected | github.com/phase-rs/phase/issues/4787 + `.investigation-conqueror-enduring-tenacity.md` | Engine proven bit-for-bit identical to the working Sanguine Bond pair; stalled awaiting reporter's environment details |

## 2. PR-6.75's own follow-up ledger (PLAN §7, 5 rows)

Location: `.planning/combo-detection/PR-6.75-C0FULL-C1-PLAN.md:402–406` (+ §6 risks).

1. **Batch-path widening of the 12 retained-prompt refs** — frozen-proven but kept prompting; needs runtime evidence + user sign-off (the C2-gating precedent).
2. **Generic-visitor consolidation** of `ability_scan` + `ability_rw` — dual exhaustive walks are acknowledged debt (`// ponytail: second exhaustive walk shares no code with ability_scan; fold both into one generic visitor after inc2b lands and C2 migrates to the profile`). D6 (#5086) documents the discipline; the consolidation itself is this row.
3. **Observer-departed zone-check freeze upgrade** — dies-observer that itself died co-batched is treated live ⇒ conservative prompt; measured printed exposure ≈ 0; fail-safe, upgrade path named.
4. **Axis-3 precision opportunities** — noticed during the pass, explicitly out of scope per coordinator refinement #1 (axis-3 byte-identity invariant).
5. **Source-actor residual constructive close** — lifelink/deathtouch granted state, `DamagedPlayerIsEventSourceOwner`, CR 800.4a player-loss cascade are invisible to the read-based conflict formula. Commutation is proven *modulo* this residual (`ability_rw.rs:16–33` module doc). Firebrand Archer witness; 12-card measured surface; constructive close priced (flips the Court of Embereth class). Inherited fail-open from the pre-C1 unconditional short-circuit — not a regression.

## 3. Documented fail-safe residuals shipped inside #5072

All conservative-direction (over-prompt or sweep-backstopped), each with an inline rationale:

- **18-row `DOCUMENTED_OVER_PROMPT` ledger + 1 genuine (Day of the Dragons)** — `triggers_ordering_parity_tests.rs:128–202`. §7 accounting: 19 = 18 documented-conservative (17 same-event + 1 batch Skyfisher Spider) + 1 genuine. Option C adds a new predicate-keyed class (~48 cards, `same_event ∧ source_independent ∧ reads_member_bound`, all auto→prompt direction) as a separate bucket.
- **L8 idempotence recognizer deliberately NOT built** — 12 monotone/self-limiting intervening-if cards stay conservative prompts ("a wrong recognizer would UNDER-prompt = rules-wrong; the conservative prompt is fail-closed = rules-correct"). Optional-polish follow-up; Osseous Sticktwister rides free if ever built.
- **2 context-free-unclassifiable singletons** — Paroxysm (revealed-card read indistinguishable from board read in AST), Flamewake Phoenix (commutes only via runtime source power). Ratified residual, no recognizer.
- **2 parse-blocked commutes** — Deep-Sea Kraken, Ichorplate Golem (commute underneath; mis-parse hides the structure).
- **Your Inescapable Doom** — parse-blocked GENUINE order-dependence, expected-UNHIT (asserted); unblocking = **parser-debt: typed comparative-life target selector** ("opponent with the highest/lowest life total" currently parses to `{type:Any}`). Future scoped project; also shrinks the Any-parse over-broadness class.
- **`reads_event_live` same-event fail-open (review LOW-1)** — event-object write×read same-event group auto-orders; deliberately left open: closing it ADDs a prompt = a D3 widening needing its own proof (`ability_rw.rs:33–46`).
- **Vote arm incomplete (LOW-2)** — `Effect::Vote` subject=Objects unmodeled, `outcome_template` dropped; pre-existing, sweep-covered (`ability_rw.rs:4614–4617`).
- **`target_recipient` `_` wildcard (LOW-3)** — fail-open on the life dimension for exotic player-referent damage targets; no concrete in-corpus miss (`ability_rw.rs:3222`).
- **`JournalCast` dead feed row (MINOR-3)** — no in-resolution write feeds the cast journal (`ability_rw.rs:113`).
- **`{ .. }` field-elision future-FIELD hole** — non-referential condition/filter arms compile-error on future *variants* but not future *fields*; low risk, sweep-backstopped (`ability_rw.rs:1814/:2045/:2230`).
- **n2 TapState→membership-count gap** — idempotent tap-all vs membership census; very low reachability, on full-DB watch.
- **StateKind lattice coarseness ceiling** — split kinds further only when the parity sweep names a printed victim (PLAN §6 risk 1).
- **Latent parser bugs flagged-not-fixed** by step-4 triage: 5 Ordeals `CountersOn{Source}` should be enchanted-creature; magistrate `ChangeZone{SelfRef}`; YID `PutCounter{Any}` for "this scheme" should be SelfRef.
- **Dead leaf-hooking cleanup** — `flag_legacy_write_target`/`target_is_legacy_ref` subsumed by the c3 position-agnostic visitor; harmless, deletion on team-lead's board (`ability_rw.rs:1531/:1554`).

## 4. Filed issues (open)

- **#5073** — wire the `FORGE_TEST_FULL_DB` trigger-ordering parity sweep into a nightly/release CI gate (cumulative-review LOW-6). Until then a future under-prompt or classifier-population shrink passes default CI.
- **#4809** — see §1 (likely HIGH-2 repro).
- **#4787** — see §1 (stalled on reporter).

## 5. Series stages not started

- **PR-7 — loop shortcut + opponent response window (CR 732.2a/732.5)**. Additional gate: poison enablement requires re-keying `ResourceVector`'s poison axis by victim `PlayerId` first (PR6-PLAN.md:68).
- **PR-8 — AI coupling (`LoopCertificate` → top line)**. Waiting on it: the `WinKind` wire ride-along (PR-6 §4.5, explicitly out of scope — bare `∞ <resource>` badge shipped instead), the `loop_check.rs:72` WinKind mapping, and the hand-authored cEDH registry expansion (`phase-ai/src/combo/registry.rs:1–5` — Thoracle/Consult, Isochron/Reversal, Underworld Breach, Food Chain, Dockside storm lines).
- **PR-6.25 row** in PROGRESS.md stays "deferred (R3)" until #5072 merges (it is the delivery vehicle; blessed R1-staged design preserved at `PR-6.25-DEFERRED-FINDINGS.md`).
- **Future feature (user, 2026-07-02):** player-pinned "fixed choices" for conditionally-infinite combos (CR 732.2a-sound). Design constraints recorded; no speculative code shipped.

## 6. Pre-6.x accepted conservatisms (all documented, recall-safe)

**Engine B (ability graph):**
- `WinTheGame`/`LoseTheGame` stay Unmodeled — **the one known recall gap**: a repeatable "target opponent loses" loop is missed; deferred to a future ResourceVector-extension PR (`ability_graph.rs:925–926`, PR4b-PLAN.md:90).
- `add_damage` hardcodes OPPONENT (self-damage mis-keyed); `target_player` defaults ambiguous filters to OPPONENT — recall-safe over-approximations, filtered/corrected by PR-5's stateful confirmation (`ability_graph.rs:242/:366`).
- R3-MANA-COLLAPSE — single color-agnostic `AxisKey::Mana`; per-color precision deferred unless measured FP rate warrants (`ability_graph.rs:87/:93`).

**PR-3 (loop check):**
- Pure net-zero mandatory DRAW undetected under the per-beat drive — explicitly out of scope ("Do not attempt to fix; do not claim draw coverage", PR3-PLAN.md:353).
- Decking (CR 704.5c mill-out) live-shortcut deferred — mandatory-loop winner fires only on the CR 704.5a life axis; pure opponent mill ⇒ None (`loop_check.rs:1041–1042`).
- Modulo-fingerprint pre-filter — deferred optimization, not needed for correctness with the 16-cap ring (PR3-PLAN.md:215).

**Corpus:**
- **37/53 rows structurally deferred** (buckets asserted in `corpus_tests.rs:113`): object-re-entry (fresh ObjectId), extra-turn/combat, color-converting net-loss (Pili-Pala), ability-copy churn (Basalt+Rings), Other (e.g. idx 22 Kiki-Jiki).
- **4 card-gated rows** — Doc Aurlock, Professor Onyx, Animate Dead, Grindstone+Painter's Servant; `#[ignore = "needs <card>"]` until those reach 0-Unimplemented.

**PR-0 (projection):**
- Modulo-projection blurs non-counter (continuous-effect) P/T changes — false-negative-safe; revisit if PR-2-era FPs appear (PROGRESS.md:153–156).
- CounterClass granularity — extend `CounterClass` if finer granularity is ever needed; never add `Ord` to core `CounterType` (PROGRESS.md:125–134).

**phase-ai:**
- `combo_line.rs:68` — `TODO(cedh-perf): cache reachable_lines()` per state-hash.

## 7. Team-lead debt ledger tied to the series

- CR 701.x renumber-drift docs sweep (forge/effect.rs:455, regenerate.rs:519/:558, oracle_ir/ast.rs:815, + life.rs 614.7→616.1) — standalone docs-only PR post-queue.
- Dead leaf-hooking deletion (§3 last row).
- M7 "floors don't guard Mixed" comment one-liner (trivia ledger).

## 8. Surfaced but out-of-series (excluded from the verdict)

- Cross-pause co-departed LTB observation ignored test (CR 603.10a, `triggers.rs:24684`) — pre-series (2026-05-30, #1449/#1477 follow-up), multi-day Unit B redesign sketch preserved inline.
- Ward cost mapping shortcuts (`triggers.rs:183–190`) and #2277 additional-cost parser ignore — out of series.

---

*Generated by team-lead from workflow `wf_c69c11e3-c04`; agent-level results at the session transcript dir if re-verification is needed. Predecessor context: PROGRESS.md series table, `.pr675-driver-log.md`, `.pr675-SECTION7-PRBODY.md`.*
