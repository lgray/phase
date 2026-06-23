# MSH-E: modal "choose up to X" with dynamic N — Ruinous Wrecking Crew + Hawkeye Master Marksman

Worktree: /private/tmp/wt-msh-modal-choose, branch card/msh-modal-choose, off UPSTREAM/MAIN 3a844e56d
(per stale-fork hazard). Tilt DOWN → direct nightly cargo. card-data/coverage gitignored; query MAIN
checkout /Users/lgray/vibe-coding/phase-rs-workdir/phase/data/.

## Both resolver-flagged (gap_count=0, supported=false) — root cause: dynamic modal max dropped to 1

### Ruinous Wrecking Crew
"~ enters with X +1/+1 counters on it. When ~ enters, choose up to X — • Discard a card, then draw a
card. • Target opponent loses 2 life. • Destroy target token. • Each player sacrifices a creature of
their choice."
- enters-with-X-counters PARSES: PutCounter{count: Ref(CostXPaid), self}.
- The modal PARSES the 4 modes but with WRONG count: execute.modal = { min_choices: 1, max_choices: 1,
  mode_count: 4 }. It should be min_choices: 0 (it's "up to"), max_choices = X (the cast value =
  CostXPaid). The dynamic-X max was dropped to fixed 1 → only 1 mode resolvable instead of up-to-X.

### Hawkeye, Master Marksman
"First strike, reach / Trick Arrows — Whenever Hawkeye becomes tapped, you may pay {1} up to three
times. When you do, choose up to that many. • Net — Target creature can't block this turn. • Explosive
— Hawkeye deals 2 damage to target player. • Boomerang — Discard a card, then draw a card."
- Taps trigger PARSES; the "When you do, choose up to that many" modal PARSES the 3 modes but with
  min_choices: 1, max_choices: 1 (WRONG — should be 0..N where N = the number of times {1} was paid).
- "pay {1} up to three times" PARSES as a SINGLE PayCost{Mana generic:1} — the REPEATED-OPTIONAL
  payment (0..3 times) is DROPPED to one payment. So the payment-count that should feed the modal max
  is lost too.

## add-engine-variant GATE — dynamic modal max
ModalChoice.max_choices is a FIXED `usize` (types/ability.rs:12564). NO dynamic-max field. "choose up
to X" (variable) / "choose up to that many" (payment-count ref) cannot be represented.
- parse_modal_choose_count (oracle_modal.rs:1641-1643) ALREADY handles "choose up to N —" → (0, N) for
  a FIXED parse_number N. For "up to X"/"up to that many", parse_number fails → falls through to the
  default → effectively (1,1). THAT is the gap.
- max_choices has a 177-ref blast radius (game/parser/AI/frontend). Changing its TYPE to QuantityExpr
  is a multi-file refactor — REJECT. Instead ADD an additive `Option<QuantityExpr> dynamic_max_choices`
  field (serde default None → existing fixed behavior preserved; when Some, the resolver uses it to
  compute the cap at resolution). Gate: this is EXTEND_OK (new field, additive, serde-safe) within the
  modal-choice mechanic (CR 700.2). The resolver/AI/frontend that read max_choices must, when
  dynamic_max_choices is Some, resolve the QuantityExpr to the runtime cap. Serialized-surface audit:
  ModalChoice is in card-data export + game state + WaitingFor (modal choice) + frontend modal UI + AI
  legal-modes — the new Option field needs serde default + the resolver/AI/frontend reading it.
  ⚠ This is the meatiest part — a new serialized field consumed across resolver/AI/frontend. Likely a
  /review-engine-plan-gated change.

## SPLIT DECISION (lead asked for judgment)
- SHARED building block: dynamic_max_choices: Option<QuantityExpr> on ModalChoice + parser for "choose
  up to X"/"choose up to that many" + resolver/AI/frontend consuming it.
- Ruinous needs ONLY: dynamic_max_choices = Some(Ref(CostXPaid)) + min_choices 0. (cast-X modal.)
- Hawkeye needs ADDITIONALLY: (a) "pay {1} up to three times" REPEATED-OPTIONAL payment (0..3) — a
  substantially separate cost mechanic producing a count; (b) "when you do, choose up to THAT MANY" —
  binding the modal dynamic_max to the repeated-payment count (an EventContext/payment-count ref, not
  CostXPaid). Hawkeye's repeated-{1}-payment is a distinct mechanic from Ruinous's cast-X.
RECOMMENDATION: SPLIT into 2 PRs. PR1 = the shared dynamic-modal-max building block + Ruinous (cast-X).
PR2 = Hawkeye's repeated-optional-payment + "that many" binding (builds ON PR1's dynamic_max field).
This keeps one core building block per PR for clean review, and PR2 depends on PR1's field. BUT: let
the planner confirm — if Hawkeye's repeated-payment count can reuse an existing payment-count
QuantityRef and the "that many" binding is small, the two could co-ship; the heavier part is the shared
field which both need. Plan PR1 (Ruinous + the field) FIRST regardless; decide PR2 vs co-ship after the
planner sizes Hawkeye's repeated-payment mechanic.

## LESSON FROM MSH-D (apply here): any new prefix-matched type word / parser token needs word-boundary
guards + negative tests. Not directly relevant (no new CoreType), but the general discipline: when
adding "up to that many"/"up to X" tags, ensure they don't shadow existing "up to N" fixed parses
(regression test the fixed "choose up to two —" still → (0,2)).

## Tests (non-vacuous, discriminating, revert-probe)
- Ruinous: parse → execute.modal { min_choices: 0, dynamic_max_choices: Some(Ref(CostXPaid)) } (NOT
  min 1/max 1). Fail-before: (1,1) fixed. Runtime (strongest): cast with X=3, ETB → may choose up to 3
  of the 4 modes (resolve 3 distinct modes); with X=0 → choose 0 (no modes). Revert-probe the dynamic
  max → caps at 1.
- Hawkeye (if co-shipped or PR2): pay {1} twice → choose up to 2; pay 0 → choose 0; the modal max =
  payment count. Repeated-payment 0..3 enforced.
- Regression: a FIXED "choose up to two —" modal still → (0,2) fixed, dynamic_max_choices None.
CR: 700.2 (modal), 700.2a/700.2d (choose up to N), 601.2b (choosing modes), 700.2i (budget modal — for
the max_choices reinterpretation precedent). Grep-verify each. Hawkeye's repeated payment: 601.2f/602
(cost payment) — verify.

## add-engine-variant GATE VERDICT: APPROVED — ModalChoice.dynamic_max_choices: Option<QuantityExpr>
- Stage 1 DOES_NOT_EXIST: no dynamic/QuantityExpr max-choices on ModalChoice. ConditionalMaxChoices
  (ModalSelectionConstraint, ability.rs:12608) carries a FIXED usize gated on a condition — not a
  dynamic count. The pawprint/mode_pawprints budget reinterprets max_choices but stays fixed. No
  variable-max representation exists.
- Stage 2 EXTEND_OK: ModalChoice is a struct; additive Option<QuantityExpr> field (default None →
  existing fixed behavior preserved) is the minimal serde-safe extension. Changing max_choices: usize →
  QuantityExpr is REJECTED (177-ref blast radius across game/parser/AI/frontend).
- Stage 3 WITHIN_SECTION: modal-choice mechanic, CR 700.2 (single rule section).
- CR grep-verified: 700.2 (line 3199), 700.2a (3201), 700.2d (3207), 601.2b (2457). APPROVED.
- Field: `#[serde(default, skip_serializing_if = "Option::is_none")] pub dynamic_max_choices:
  Option<QuantityExpr>` on ModalChoice. When Some, the resolver computes the cap from the QuantityExpr
  at modal-choice time (Ruinous: Ref(CostXPaid); Hawkeye: the repeated-payment count). min_choices: 0
  for "up to". The runtime sites reading max_choices (177 refs; key: the WaitingFor modal-choice cap +
  AI legal-modes + frontend modal UI) must, when dynamic_max_choices.is_some(), resolve it to the cap.
  Serialized-surface: card-data export + game state + WaitingFor + frontend + AI — serde default makes
  existing data load; add a round-trip test + a runtime cap test.
