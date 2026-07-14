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

# Reviewer mandate — combo-plan-egg

**File relay.** Messaging has failed twice this session (two reviewers' reports never reached team-lead).
This file is the authoritative copy of the mandate. If you are `combo-plan-egg`, read it and confirm.

**Target:** `.planning/combo-detection/REAL-BOARD-RCA-AND-PLAN.md` @ **69fe6c3ea** (FROZEN — will not move).

---

## Three deliverables, all first-class

### 1. §5b VERDICT — adopt `egg`, reject it, or spike it with a named kill criterion

`egg` is **NOT a requirement**. The user has **pre-authorized "disregard it."** A well-evidenced **no** is
as valuable as a yes. Adjudicate:

- **(a) Is `Axes` (`ability_scan.rs`) actually a semilattice** — is its combination operator a *join*
  (associative / commutative / idempotent), or order-dependent / non-monotone? **If it is not a join it
  cannot be an `egg::Analysis` and §5b.2 collapses. Highest-value single check.**
- **(b) Can the ability AST be hosted as an `egg::Language`** (fixed-arity ops over e-class ids)? Do
  `Effect` / `TargetFilter` / `Condition` / `QuantityExpr` carry non-structural payloads (String, HashMap,
  ObjectId) that don't fit? **If the encoding is a rewrite of `types/ability.rs`, say so.**
- **(c) ⭐ THE CATEGORY-ERROR CHECK.** An `egg::Analysis` is **bottom-up and context-free per e-class.**
  But the correct predicate depends on **which growth axis / which quotient / which zone** — properties of
  the **LOOP**, not of the AST. **If the right predicate is context-SENSITIVE, e-class analysis is
  structurally the wrong tool and §5b.2 is a category error. ATTACK THIS HARDEST.**
  Discriminator: an `Analysis` must **un-reject Intruder Alarm** (`SetTapState{target: Typed[Creature],
  scope: All, state: Untap}` — CR 732.2a's own worked example) **AND keep Gaea's Cradle failing closed**
  (`AnyOneColor{count: Ref(ObjectCount{Creature,You})}`; guard test
  `ability_scan::mana_production_scan_tests::for_each_creature_production_still_fails_closed`).
  **If congruence + analysis cannot separate those two, the option is dead.**
- **(d) RULES RISK.** Plan scopes the spike to **congruence + `Analysis`, ZERO semantic rewrite rules**
  (every rewrite is a CR claim: destroy ≠ sacrifice ≠ put-into-graveyard under CR 701.15 / 702.12 / 614).
  **Is that scoping sufficient? Can congruence ALONE change engine behavior?**
- **(e) WASM cost.** Engine ships to WASM (`opt-level='z'`, LTO); the detector is on the live in-game path
  and **cannot be feature-gated out** (verify). Measure the bundle delta if you can.
- **(f) Is the plan right that egg is UNSOUND for STATE equality?** §5b.3: congruence/bisimulation is
  coarser than isomorphism and **collapses multiplicity** (3 vs 4 identical Saprolings = same term = same
  e-class) — so accepting on congruence certifies iteration N ≡ N+1 **exactly when tokens grew.** Verify or
  refute. And assess **Murφ scalarset symmetry reduction** (normalization first ⇒ errs fine ⇒ fail-closed;
  canonicalization as the exact upgrade) as the right match for RC-4.

### 2. P5 DELETION SOUNDNESS — the only place the plan makes the detector LESS conservative

- **(g) ⚠️ IS DELETING R6 SOUND? THE MOST DANGEROUS LINE IN THE PLAN.** R6 rejects on any non-empty
  `state.delayed_triggers`. The plan claims a Kiki-Jiki token's *"sacrifice it at the beginning of the next
  end step"* delayed trigger **fires in the drive** and lands in Δ. **DOES IT?** The drive settles at an
  empty-stack `Priority` beat; that trigger fires at the **beginning of the next end step** — plausibly a
  **later step, outside the drive window entirely.** **If it never fires in the drive, deleting R6
  certifies a loop whose tokens all die — a FALSE CERTIFICATE. MEASURE IT. RUN A TEST.**
- **(h) Is "the drive measures every effect the current board produces" TRUE?** Find a counterexample: a
  battlefield ability that fires during the loop but whose impact does **not** land in the measured Δ
  (state outside the `object_content_eq` / `ResourceVector` partition; `summoning_sick`; effects that only
  manifest at a later step; effects on other players).
- **(i) Is the two-item blind-spot taxonomy EXHAUSTIVE?** (monotone depletion + discontinuity). Enumerate
  structurally from `Effect`, replacement effects, `ActivationRestriction`, `Condition`, turn-based actions:
  **what can change between iteration 3 and iteration N? A third category ⇒ the deletions are unsound.**

### 3. ⭐ COHERENCE AUDIT — read the document END TO END

**The plan has been revised SIX times in place. Nobody has read it whole since.** Sections written under a
superseded understanding sit next to the sections that refuted them. **This is exactly the failure mode
that produced its first ten errors.**

Report each contradiction as: the two conflicting passages (with § numbers) → which is CORRECT (evidence) →
what must be deleted/rewritten.

**Calibration example — I found this one and deliberately did NOT fix it. Assume there are more:**
> **§4.10 vs §5 DIRECTLY CONTRADICT.** §4.10 claims `Quotient::ObjectIdentity` unlocks **all 13**
> `ObjectReentry` rows and is *"worth more than Phases 1–5 combined"* (payoff table: *"~17 of 37 deferrals
> from ONE parameterization"*). **§5, written afterwards from the round-4 review, says it lifts ZERO
> directly** — Group A (6 rows) is blocked by R6/RC-1/RC-3, id churn is irrelevant to them; Group B (7 rows)
> needs id-canonicalization, *"the riskiest change in the program."* **Appendix B #7 records the
> refutation. §4.10's payoff table is stale.**

**Seams to check (each was edited independently and may no longer agree):**
- **§4.7** (turn-crossing OUT, "an engineering cut") vs **§4.10** (`TurnCount`/`CombatCount` quotient claims
  to *retire* that cut) vs **§6 P0's WAIVED partition** (rows 32/33/34). **In or out? Pick one.**
- **Exec-summary box** ("two new subsystems, not three") vs **§4.6** (C3 "mostly a deletion") vs **§6 P5**
  vs **§5b** (P2 *and* P5 collapse into an egg analysis ⇒ smaller still). **What IS the new surface?**
- **§4.10's `Quotient` enum** vs **§6 P6** ("take the formalism P5.5 selects"). **Is `Quotient` the plan of
  record, or was it superseded by scalarset canonicalization? The document must say ONE thing.**
- **§3.1's "catch-22"** — Appendix B #8 says its original evidence (Witherbloom's affinity) is measured
  FALSE and the correct evidence is Intruder Alarm. **Does §3.1's body actually reflect that?**
- **Phase numbering.** Dangling "Phase 2.5"? Do P0…P6 / P5.5 cross-references resolve?
- **Every line-number citation.** Three already drifted (`no_living_player_has_meaningful_priority_action`
  = engine.rs:**2367** not 1765; `ability_has_per_turn_activation_gate` = resource.rs:**2848** not 2842;
  `fire_time_conditions_read_growing_class` = resource.rs:**1451** not 1468). **The document is cited as
  authority downstream — a wrong line number sends an implementer to the wrong code. List every wrong one.**
- **Repeated numbers** (corpus is **53**, not 55).

**The test:** *read it as an implementer would, top to bottom, and execute it — do you build the right
thing?* **A plan that requires you to already know which parts are stale is not a plan.**

---

## Rules

- Cite **file:line** and **CR numbers** for everything.
- Mark anything unchecked **UNVERIFIED**. **Never guess; never fill from memory** — this plan has been
  wrong **ten times**, every one a code claim from memory. An honest *"did not reach this"* beats a
  plausible claim that costs a cycle to refute.
- Tilt is OFF — **you MAY and SHOULD run `cargo test -p engine ...`** where a claim needs runtime
  measurement. Prior reviews read code but executed nothing; that is now the biggest gap.
- **Put the ENTIRE report in your FINAL RESPONSE MESSAGE, not in a file.** Two prior reviewers' reports
  never reached team-lead.
- **Review only — modify no files.**
