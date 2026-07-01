# Infinite-Combo Detection & Confirmation — Feasibility Analysis and PR Plan

_Target codebase: phase-rs/phase @ `upstream/main` `1036d0689`. Status: design proposal (uncommitted)._
_All architecture claims below are grep-verified against the tree at that commit; file:line anchors are inline._

---

## 1. Executive summary

phase.rs is unusually well-positioned to ship an **infinite-combo confirmation tool** — a feature most card engines lack — because it already has (a) a fully typed ability AST, (b) a mature recursive AST-traversal pass (`coverage.rs`), (c) a pure, immutable, cheaply-forkable reducer (`im::` persistent state + `GameRunner::act`), (d) **real CR 104.4b mandatory-loop detection already in the resolution loop** (`GameState::loop_fingerprint()` + a `loop_window` deep-equality buffer → draw; plus `emit_resolution_halt()` for net-progress runaways), and (e) a hand-authored combo-recognition layer (`phase-ai/src/combo`) that already has the *vocabulary* (`WinKind::InfiniteLoop`, `ComboReachability`) but **asserts** loops rather than **verifying** them.

> **Scope, sharpened by review (§11):** the engine already *detects and draws* repeating **mandatory** loops (CR 104.4b/732.4). The genuine gap is **net-progress (beneficial) loops** — which `emit_resolution_halt()` today merely halts, with its own comment noting it "cannot infer an iteration count." The combo tool's job is exactly to fill that one gap (classify the unbounded resource, then shortcut per CR 732.2a or end on lethal), reusing the existing fingerprint/window machinery rather than building parallel infrastructure. This makes the first slice materially smaller than a greenfield estimate.

The headline deliverable — "given a card set, verify a claimed loop and report what goes infinite" — is achievable in a **low-risk, offline, ~1–2 week first slice** (PR clusters 0–2 below) that adds *zero* game-behavior changes. Deeper gameplay integration (loop shortcutting with opponent response windows, ∞-status display, AI coupling) is higher value but higher risk and is factored into separately-reviewable clusters 3–8.

Because Magic is Turing-complete, **no detector can be complete**; the design is deliberately **sound-but-incomplete and bounded** — it never falsely confirms a loop, and it confirms the overwhelmingly common deterministic resource loops (the user's three examples all fall in-scope).

---

## 2. Problem & motivation

In high-power formats (cEDH, Modern, Standard) a confirmed deterministic loop is a *fait accompli* that players resolve by hand, iteration by iteration — tedious and error-prone. CR 732 ("Taking Shortcuts", whose rules 732.1b–732.6 govern loops) already prescribes the fix in paper Magic: shortcut the loop to N iterations (732.2a), draw on an all-mandatory loop (732.4), and let players break/respond to it (732.5–732.6). A digital engine can automate exactly this **if** it can (1) detect a loop, (2) classify what resource goes unbounded, and (3) expose a response window. The same machinery makes the AI play optimally (recognize and execute its own winning loops) and lets the UI render `∞` counters / power / mana instead of forcing manual repetition.

**Driving examples** (each maps to a concrete detection path in §6):
- **Heliod, Sun-Crowned + Walking Ballista** — net-zero counters per cycle, unbounded *damage/life*.
- **Kilo, Apogee Mind + Freed from the Real + Relic of Legends** — *mana-neutral* untap loop (Relic of Legends taps Kilo for 1; Freed's `{U}` untaps it); each tap fires Kilo's "becomes tapped → proliferate" → unbounded *proliferate triggers* (**not** mana).
- **Doc Aurlock, Grizzled Genius + Aang, Swift Savior + Appa, Steadfast Guardian** — airbend exiles a permanent its owner may recast from exile for `{2}`; Doc Aurlock makes cast-from-exile `{2}` cheaper → free recast loop → unbounded *casts-from-exile* (Appa makes a 1/1 Ally per exile-cast → infinite tokens).

Prior art: this is an old question (Martinek's Wolfram-community ability-graph analysis), but production engines punt to **curated combo databases** (Commander Spellbook, Nerd Leagues) rather than algorithmic verification.

---

## 3. Theory & literature

**Undecidability ceiling.** Churchill, Biderman & Herrick, _Magic: The Gathering is Turing Complete_ (arXiv:1904.09828), embed an arbitrary Turing machine into a legal game using only **mandatory** effects — so "does this position loop forever?" is equivalent to the halting problem and is **undecidable in general**. Howe, _A Programming Language Embedded in MTG_ (LIPIcs FUN 2024), reinforces this. **Consequence:** any practical detector must be sound-but-incomplete and resource-bounded. We never claim completeness; we claim *soundness* (no false "infinite") plus coverage of the deterministic loop class that dominates real play.

**Combo detection = cycle detection.** The established practical model (Martinek, Wolfram community t/493458) treats **abilities as nodes** and "effect A enables/triggers ability B" as **edges**, then searches for a cycle that returns to the originating ability. Cycle/feedback detection in a directed graph is polynomial (Tarjan SCC / DFS back-edges); the hard part is *modeling the edges and resource flow correctly*, and pruning to abilities (Martinek ignores activated-ability costs and assumes legal targets — both of which we can do better because we have the full AST + a real reducer).

**Unbounded-resource = Petri-net coverability.** The cleaner formalism for "what goes infinite" is a **Petri net / Vector Addition System with States (VASS)**: places = resource types (mana by color, life per player, counters by type, untapped permanents, storm count, cards in hand, damage on stack), transitions = abilities/steps with an integer production/consumption vector. A repeatable firing sequence with a **non-negative net effect on its inputs and a strictly positive net effect on some "win" place** is exactly an infinite combo. **Karp–Miller coverability** decides reachability of an unbounded marking (place → ω); VASS reachability is decidable (Ackermann-complete; Leroux–Schmitz). For the small card sets a player presents, the coverability sub-problem is tractable. This is the theory behind classifying *which* resource is unbounded (the `∞` the UI shows).

**Confirmation = bounded model checking / state-cycle detection.** Rather than trust a static abstraction (which over-approximates: it ignores targeting legality, timing windows, "may" choices), we **confirm** by driving the real reducer and watching for a **state fixpoint modulo monotone resources**: if applying a candidate action sequence returns the game to a state equal to an earlier one *except* for a monotonically-improved resource vector, with the same actions still available, the loop is real and repeatable (a sound certificate). This is bounded model checking specialized to MTG, made cheap by `im::` structural sharing.

**Synthesis — a two-stage pipeline.** Static graph analysis (over-approximate, fast, works from a card *list*) **proposes** candidate cycles; dynamic simulation (sound, works from a game *state*) **confirms** them and emits a certificate naming the unbounded resource. This mirrors abstract-interpretation-then-refinement and keeps each stage independently useful: the static stage answers "could these cards loop?", the dynamic stage answers "do they, here, and what goes infinite?".

Sources: [arXiv:1904.09828](https://arxiv.org/abs/1904.09828); [LIPIcs FUN 2024 (Howe)](https://drops.dagstuhl.de/storage/00lipics/lipics-vol291-fun2024/LIPIcs.FUN.2024.31/LIPIcs.FUN.2024.31.pdf); [Wolfram community t/493458](https://community.wolfram.com/groups/-/m/t/493458); [Commander Spellbook combo DB (via Nerd Leagues)](https://nerdleagues.com/combos). Standard CS: Tarjan SCC; Karp–Miller coverability; Leroux–Schmitz VASS reachability.

---

## 4. How this maps onto the phase-rs architecture (grep-verified)

| Capability needed | What already exists | Anchor |
|---|---|---|
| Typed ability AST to analyze | `Effect`, `AbilityDefinition`, `TriggerDefinition`, `AbilityCost`, `QuantityExpr`, `RepeatContinuation{UntilStopConditions,WhileCondition}` | `types/ability.rs` (18.5k LOC) |
| Recursive AST walker (graph-extraction precedent) | `ParsedItem` tree + `build_ability_item` / `build_trigger_item` / `build_cost_item` / `build_casting_option_item` | `game/coverage.rs:3240–3557` |
| Cheap state forking for search | `GameState` derives `Clone`, uses `im::` persistent structures (52 sites) | `types/game_state.rs:5369`; `im::` count |
| Canonical state compare/hash | **EXISTS:** `loop_fingerprint()` (FxHash over turn/phase/priority/zones/per-player life+zone-sizes/per-object tapped+damage), `normalize_for_loop()`, `loop_states_equal()` (already ignores volatile counters). Only a **resource-projected** variant is new. | `types/game_state.rs:7410,7452,7468` |
| Existing loop DETECTION (mandatory) | `loop_window` (`FINGERPRINT_AFTER_ITERS=32`, `MAX_LOOP_WINDOW=128`) in the resolution batch → fingerprint pre-filter + deep equality → **CR 104.4b/732.4 draw** | `game/engine.rs:1107–1223` |
| Net-progress loop handling (the GAP) | `emit_resolution_halt()` halts runaways (ceilings `MAX_EVENT_GROWTH=50_000`, `MAX_OBJECT_GROWTH=16_000`, iter-cap) → `GameEvent::ResolutionHalted`; comment: "a net-progress loop is a CR 732.2 shortcut the engine cannot infer an iteration count for" | `game/engine.rs:1100,1290`; `types/events.rs:360` |
| Pure reducer / sim substrate | GameAction dispatch + harness `GameRunner::act(action) -> Result<ActionResult>` | `game/engine.rs`; `game/scenario.rs:1063,1089` |
| Chain recursion guard | `if depth > 20` in `resolve_ability_chain` | `game/effects/mod.rs:4427` |
| Bounded-loop modeling | `RepeatContinuation` (#4030 repeat-process) | `types/ability.rs:12890` |
| "Infinite resource" precedent | `debug_infinite_mana: BTreeSet<PlayerId>` + refill hook + UI toggle | `types/game_state.rs:5808`; `game/engine_debug.rs:405` |
| Resource state to vectorize | counters `HashMap<CounterType,u32>`, per-player `life`, floating mana `ManaPool{mana:Vec<ManaUnit>}`, loyalty, poison, tokens | `types/game_state.rs:203`; `types/mana.rs:1339` |
| Combo vocabulary (to verify, not assert) | `ComboLine`, `WinKind::{ImmediateLoss,InfiniteLoop,LethalDamage}`, `ComboReachability`, `StructuralComboDetector`, hand-authored `ComboRegistry` | `phase-ai/src/combo/{line,detection,registry}.rs` |
| Action enumeration for search | `find_legal_targets`; AI `validate_candidates` / `rank_candidates` | `game/targeting.rs:16`; `phase-ai/src/planner/mod.rs:427,932` |
| Rules basis | CR 732.1b/732.2a (shortcut N iterations), 732.4 (all-mandatory → draw, 104.4b) | `docs/MagicCompRules.txt:6336–6358` |

**Key takeaway:** the two hardest pieces of a from-scratch implementation — a typed action model with a pure reducer, and a recursive AST walker — already exist. The new surface is mostly *pure analysis code* that consumes them. The only foundational gap is a canonical `StateFingerprint`/`ResourceVector` (cluster 0).

The current `phase-ai/src/combo` layer is the **gap made concrete**: `CardPredicate::NameEquals` + a hand-authored `ComboRegistry` recognize *named* combos (a local Commander-Spellbook), and `WinKind::InfiniteLoop` is an author's claim. Nothing drives the reducer to *verify* that a given card set actually loops, nor classifies the unbounded resource. That is exactly what this proposal adds, reusing those types as the output vocabulary.

---

## 5. Proposed design — two cooperating engines

### Engine A — Dynamic loop confirmation (sound, from a game state)
1. Input: a `GameState` + a candidate repeatable action sequence (from the player's claim, from Engine B, or from the `ResolutionHalted` trace).
2. Fork the state (`im` clone), apply the sequence with auto-resolution, recording a `StateFingerprint` and a cumulative `ResourceVector` after each cycle.
3. **Cycle test:** the loop is confirmed when a later fingerprint equals an earlier one **modulo monotone resources** (board/zones/tap-state identical; only monotone resources changed) and the same enabling actions remain legal.
4. **Classify:** the components of `ResourceVector` that strictly increased per cycle with non-negative input cost are the **unbounded resources** (mana/life/damage/counters/tokens/storm). Determine `WinKind` (lethal, mill, immediate-win condition, or pure advantage).
5. Output: `LoopCertificate { cycle_actions, unbounded: ResourceVector, win_kind, mandatory: bool }`. `mandatory` (no "may"/choice in the cycle) drives CR 732.4 (draw) vs CR 732.2a (player picks N).

### Engine B — Static ability-graph candidate generation (over-approximate, from a card list)
1. Reuse `coverage.rs` traversal to build an `AbilityGraph`: nodes = activated/triggered/replacement abilities; edges = "effect of A enables/triggers B" + a per-transition `ResourceVector` (produced/consumed) derived by mapping each `Effect` variant.
2. Find SCCs (Tarjan); for each cycle, sum the net `ResourceVector`; keep cycles whose consumed resources are net-coverable (Petri-net non-negativity) and some win-resource is strictly positive.
3. Emit candidate action sequences → feed Engine A for sound confirmation.

**Pipeline:** B (fast, card-list, may over-claim) → A (sound, stateful, confirms + classifies). Either can run standalone: A confirms a player's explicitly-claimed line without B; B answers "do these cards have a loop at all?" without a board.

### Honest scope boundaries
- **Soundness over completeness** (Turing-complete ceiling): bounded by cycle-search depth + a state budget; misses loops needing large setups, opponent cooperation, or arbitrary computation.
- **Optional vs mandatory** (CR 732.4/732.6): the certificate records whether the cycle contains a choice; the engine/UI then offers "repeat N / until lethal" or declares a draw.
- **Interaction**: a confirmed loop is "infinite *absent interaction*"; the response-window cluster (7) surfaces the chance for opponents to break it (CR 732.5).
- **Randomness**: a cycle containing a shuffle/coin-flip is reported as *probabilistic/bounded*, not infinite, unless the random outcome is irrelevant to repeatability.

---

## 6. Worked examples (applicability)

- **Heliod + Walking Ballista** — Cycle: Ballista removes a +1/+1 counter to deal 1 (lifelink via Heliod) → you gain 1 life → Heliod's "whenever you gain life" puts a +1/+1 counter back on Ballista. After one cycle the board/counters are *identical*; the monotone delta is `damage_dealt_to_opp += 1` (and life). Engine A: fingerprint recurs modulo the damage/life vector → confirmed, `unbounded = {damage, life}`, `win_kind = LethalDamage`, mandatory loop once started.
- **Kilo, Apogee Mind + Freed from the Real (enchanting Kilo) + Relic of Legends** — Cycle: Relic of Legends' "tap an untapped legendary creature: add one mana" taps Kilo (producing `{U}`); Kilo's "whenever Kilo becomes tapped, proliferate" fires; Freed's "`{U}`: untap enchanted creature" spends that `{U}` to untap Kilo. Mana is **net-zero**; board identical each cycle; the monotone delta is **+1 proliferate** per cycle. Engine A: `unbounded = {proliferate triggers}` → win by proliferating poison/loyalty/+1+1. **This is the example that forces `ResourceVector` to count triggers/events, not just board resources.**
- **Doc Aurlock, Grizzled Genius + Aang, Swift Savior + Appa, Steadfast Guardian** — airbend (exile a target permanent; its owner may recast it from exile for `{2}`) + Doc Aurlock ("spells you cast from exile cost `{2}` less") makes each recast **free**. The two airbenders airbend each other on ETB → recast-from-exile → re-trigger → loop at zero net mana. Each iteration is a cast-from-exile (Appa: "whenever you cast a spell from exile, create a 1/1 Ally") → `unbounded = {casts-from-exile → tokens}`; Aang accrues experience counters. Confirms Engine A must vectorize **cast/trigger counts**, not only board resources.

These three exercise the three resource families (damage/life, mana, casts/triggers), which is why the `ResourceVector` (cluster 0) must span all of them from the start.

---

## 7. Prioritized PR clusters (cleanly factored for review)

Ordered by value-per-risk and dependency. Sizes: **S** ≈ 1–2 days, **M** ≈ 3–5 days, **L** ≈ 1–2 weeks. Each cluster is an independently reviewable PR; the diff boundaries are chosen so no PR mixes pure-analysis additions with engine-behavior changes.

### PR-0 — `StateFingerprint` + `ResourceVector` foundation  · **S** · risk: low
- New pure module (e.g. `engine/src/analysis/fingerprint.rs`, `resource.rs`): canonical fingerprint over the game-relevant subset of `GameState` (exclude logs, RNG seed, cosmetic/undo history), and a `ResourceVector` (per-player life, mana by color, counters by `(CounterType, object-class)`, tokens, cards drawn, damage dealt, storm/cast count).
- No behavior change; exhaustive unit tests (equal boards → equal fingerprint; monotone deltas measured correctly). Enables everything else.
- Files: new `analysis/` module; tiny `mod.rs` wiring. **Reviewable in isolation.**

### PR-1 — Analysis simulation harness  · **S–M** · risk: low
- Factor a reusable `analysis::Simulator` around the existing reducer (`ScenarioRunner::act` / `game/engine.rs` dispatch): fork (`im` clone), apply a scripted action with auto-resolution, return `(StateFingerprint, ResourceVector)`. Pure wrapper; **no new game logic**.
- Files: `analysis/sim.rs`; reuses `game/scenario.rs`, `game/engine.rs`. Depends on PR-0.

### PR-2 — Dynamic loop-confirmation engine (Engine A)  · **M–L** · risk: low (offline)
- `analysis::loop_check`: given state + candidate sequence → `LoopCertificate` (cycle detection modulo monotone resources; classify unbounded resources + `WinKind`; `mandatory` flag).
- Fixture tests for the three worked examples via `ScenarioRunner` scenarios. This PR alone delivers the **"verify a claimed combo offline"** headline.
- Files: `analysis/loop_check.rs`, fixtures. Depends on PR-0/1. **No engine behavior change.**

### PR-3 — Classify the existing `ResolutionHalted` bail into a loop signal  · **M** · risk: medium (touches resolution)
- When `stack_size > 100` trips (`stack.rs:6422`), run Engine A on the recent action trace; emit a richer `GameEvent::LoopDetected { unbounded, win_kind, mandatory }` alongside/instead of the bare `ResolutionHalted`. Preserve the safety bail. First live coupling; rules-correct per CR 732.
- Files: `game/stack.rs`, `types/events.rs`, `game/log.rs`. Depends on PR-2. Sensitive area → small, well-tested diff.

### PR-4 — Static ability-graph extractor (Engine B)  · **L** · risk: low (offline), broad surface
- `analysis::ability_graph`: reuse `coverage.rs` traversal to build the node/edge graph + per-`Effect` `ResourceVector`; Tarjan SCC + net-vector coverability → candidate cycles. The `Effect`-variant → resource-vector mapping is broad but mechanical and **incrementally extendable** (start with mana/counter/damage/untap/cast variants; tag the rest "unmodeled").
- Files: `analysis/ability_graph.rs`. Depends on PR-0. Parallelizable with PR-2/3. Ship the mapping in sub-PRs by effect family if it grows.

### PR-5 — Confirmation tool surface (CLI + engine/WASM API)  · **S–M** · risk: low
- `cargo combo-verify` bin + an engine API: input a card set or a `GameState`, run B→A, print confirmed `LoopCertificate`s ("Infinite damage: Heliod + Walking Ballista"). Thin wrapper; reuses the card-data plumbing (`oracle-gen`/`coverage`).
- Files: `engine/src/bin/combo_verify.rs`, `analysis/api.rs`, `.cargo/config.toml` alias. Depends on PR-2 (+ PR-4 for card-list mode).

### PR-6 — Unbounded-resource (`∞`) state + display  · **M** · risk: medium
- Generalize the `debug_infinite_mana` precedent into a first-class "this resource is unbounded (confirmed loop)" representation so the engine resolves "deal ∞ / gain ∞ / make ∞ tokens" as game-ending via shortcut (CR 732.2a) **without literally iterating**, and the UI shows `∞` counters/P-T/mana.
- Files: `types/game_state.rs`, derived views, `engine-wasm` bridge, `client/`. Depends on PR-2/3.

### PR-6.25 — Order-independence soundness (`group_is_order_independent`)  · **M** · risk: medium · **DEFERRED (R3, 2026-06-30)**
- Originally a QoL widening of auto-resolve for order-irrelevant simultaneous trigger groups; adversarial review proved the widening UNSOUND and surfaced a latent CR 603.3b bug (`triggers.rs:3413` auto-orders order-dependent triggers). Reshaped into a correctness PR (fail-closed, compiler-exhaustive event-context + sibling read/write-conflict classifier). Deferred to a funded big push. A prerequisite ENABLER for PR-6.5 (necessary but insufficient).
- Full design (R1-staged C0/C1/C2), counterexample, and measured reachability: **`PR-6.25-DEFERRED-FINDINGS.md`**.

### PR-6.5 — Growing-cascade detector for multiplayer win-acceleration  · **L** · risk: high · **DEFERRED EPIC (funding-gated)**
- 2p combo win-acceleration works; ≥3p fails. All-opponent drains fan out to one trigger per opponent per cycle → the stack grows unboundedly → `loop_states_equal_modulo_resources` never matches a prior state → the §3 live win-shortcut never fires. PR-6.25's order-irrelevance is necessary but insufficient (it does not address unbounded stack growth). Needs a NEW net-progress / growing-cascade detector.
- **Pathway (the major one): distributed-systems failure analysis** — cascading failures in networks (Motter–Lai), branching-process / epidemic-threshold criticality (mean offspring > 1 ⇒ supercritical), termination detection (Dijkstra–Scholten diffusing computations), and especially **Petri-net coverability + Karp–Miller ω-acceleration** (detect/accelerate the unbounded component symbolically without iterating). Lit-search via the board's maximal spanning graph of triggers. Full notes + grounding corpus: **`PR-6.5-EPIC-GROWING-CASCADE.md`**.

### PR-7 — Loop shortcut with opponent response window  · **L** · risk: high (interactive protocol)
- On a confirmed loop in live play, present a CR 732.2a shortcut: a priority window for opponents to respond/break (732.5–732.6), then the controller declares iteration count ("repeat N" / "until lethal"); all-mandatory loop ⇒ draw (732.4). New `WaitingFor`/`GameAction` + frontend modal (mirrors existing interactive-choice patterns).
- Files: `types/actions.rs`, `game/priority.rs`, resolver, `engine-wasm`, `client/`. Depends on PR-2/3/6. Heaviest; protocol-version bump likely.

### PR-8 — AI coupling  · **M** · risk: medium
- Feed confirmed `LoopCertificate`s into `phase-ai` (`search.rs`/`planner`/`combo`): an algorithmically-confirmed winning loop becomes a top-ranked line, augmenting/retiring reliance on the hand-authored `ComboRegistry`. Extend `ComboReachability`/`WinKind` with an "algorithmically confirmed" provenance. Run `cargo ai-gate` with paired-seed baselines.
- Files: `phase-ai/src/{combo,search,planner,policies}`. Depends on PR-2 (+ optionally PR-4). Reuses existing combo types as output.

**Dependency graph:** PR-0 → {PR-1 → PR-2 → {PR-3, PR-5, PR-6 → PR-7, PR-8}}, and PR-0 → PR-4 → (feeds PR-5/PR-2). PR-4 runs in parallel with the PR-1/2 line.

---

## 8. Effort, value, and recommended first slice

- **Offline verifier (PR-0,1,2 + optionally 5):** ~1–2 weeks, low risk, no behavior change. Delivers the user's headline ask (verify claimed loops, name the unbounded resource) and an immediately useful `cargo combo-verify`. **Recommended starting point.**
- **Static card-list analysis (PR-4):** ~1–2 weeks, parallelizable; turns it into a deck-scanner ("does this list contain an infinite?").
- **Live gameplay integration (PR-3,6,7):** the high-value, higher-risk gameplay payoff (auto-shortcut, `∞` display, opponent interaction). Sequence after the offline core proves the detection is sound on a corpus of known combos.
- **AI (PR-8):** last; depends on a trusted certificate.

---

## 9. Risks & open questions

- **False positives** from Engine B (ignores targeting legality/timing) — mitigated by always confirming through Engine A before any user-facing or gameplay claim.
- **State-fingerprint correctness** — must exclude non-game-relevant fields (logs, RNG, undo) yet include everything that affects repeatability (tap state, counters, "this turn" accumulators, monarch, emblems). This is the make-or-break detail of PR-0; needs a careful field-by-field audit of `GameState`.
- **Determinism** — randomized loops (shuffle/coin-flip) reported as bounded/probabilistic, not infinite.
- **Performance** — bounded by search depth + a global state-visit budget; `im` structural sharing keeps forks cheap; cache fingerprints.
- **"May"/optional & "[A] unless [B]" loops** (CR 732.6) — certificate records optionality; the shortcut UI offers iteration choice rather than auto-resolving.
- **Corpus validation** — before trusting it in play, run PR-2/4 against a known-combo corpus (Commander Spellbook export) to measure recall and confirm zero false "infinite" (soundness).

---

## 10. Recommendation

Start with **PR-0 → PR-1 → PR-2** (pure, offline, low-risk) to ship the combo *verifier*, validated against a known-combo corpus. Then branch into **PR-4** (card-list scanning) and the **PR-3/6/7** gameplay-integration line in parallel, with **PR-8** (AI) last. The existing AST + pure reducer + `coverage.rs` walker + `combo/` vocabulary mean this is **substantially less than a from-scratch effort** — the foundation is already in the tree; the work is principled analysis code layered on top, cleanly separable into the PRs above.

---

## 11. Independent review — findings, corrections, and churn minimization

A second pass re-verified every factual claim against `upstream/main` `1036d0689`, the cited literature, and the Comprehensive Rules. Results below; §1/§2/§4 were corrected inline. **The headline correction is that the engine already does most of the loop-detection plumbing, so the plan's scope and churn shrink.**

### 11.1 Codebase claims — verification log
| Claim in draft | Verdict | Ground truth |
|---|---|---|
| Sim harness `ScenarioRunner::act` | ❌ **Hallucinated name** | It's `GameRunner::act` (`scenario.rs:1063` impl, `:1089` fn). |
| Loop-bail is `stack_size > 100` → `ResolutionHalted` | ❌ **Wrong (test mistaken for prod)** | `stack.rs:6422` is a *test* assertion. Production halt is `emit_resolution_halt()` (`engine.rs:1290`) on `MAX_EVENT_GROWTH=50_000` / `MAX_OBJECT_GROWTH=16_000` / iteration-cap (`engine.rs:1100–1130`). |
| Canonical fingerprint is a "gap" to build | ❌ **Already exists** | `GameState::loop_fingerprint()` (`game_state.rs:7410`) + `normalize_for_loop()` (`:7452`) + `loop_states_equal()` (`:7468`, already ignores volatile counters — test `loop_states_equal_ignores_volatile_counters`). |
| No real loop detection (only a crude bail) | ❌ **Understated** | Full CR 104.4b mandatory-loop detection w/ `loop_window` + deep-equality → **draw** (`engine.rs:1180–1223`). |
| Mana modeled as a `mana_pool` field on `GameState` | ⚠️ **Imprecise** | Floating mana is `ManaPool { mana: Vec<ManaUnit> }` (`mana.rs:1339`) + `ManaPoolEmptied` (CR 500.4); not a top-level GameState field of that name. |
| CR 732 = "Handling Loops" | ❌ **Wrong title** | CR 732 is "**Taking Shortcuts**"; loop rules are 732.1b–732.6. Sub-rule cites (732.2a, 732.4) were correct. |
| `coverage.rs` recursive AST walker (`build_ability_item`/`build_trigger_item`/`build_cost_item`) | ✅ Confirmed | `coverage.rs:3240–3557`. |
| `GameState`: `Clone`+`Serialize`, not `Hash/Eq`; `im::` ×52 | ✅ Confirmed | `game_state.rs:5368–5369`. |
| `depth > 20` chain guard | ✅ Confirmed | `effects/mod.rs:4427`. |
| `RepeatContinuation{UntilStopConditions,WhileCondition}` | ✅ Confirmed | `ability.rs:12890`. |
| `debug_infinite_mana` precedent | ✅ Confirmed | `game_state.rs:5808`; `engine_debug.rs:405`. |
| Hand-authored `combo/` layer (`ComboLine`/`WinKind`/`ComboReachability`/`ComboRegistry`, `NameEquals`) | ✅ Confirmed | `phase-ai/src/combo/line.rs` (read in full). |
| `find_legal_targets`; AI `validate_candidates`/`rank_candidates` | ✅ Confirmed | `targeting.rs:16`; `planner/mod.rs:427,932`. |

Net: **3 errors, 1 hallucinated identifier, 1 imprecision — all corrected.** No fabricated APIs survive in the revised plan.

### 11.2 Literature & rules — verification
- MTG Turing-complete → general detection undecidable: **correct** (arXiv:1904.09828; Howe LIPIcs FUN 2024). Soundness-not-completeness framing stands.
- Combo = ability-graph cycle detection (Martinek/Wolfram): **correct**; Tarjan SCC for cycles is standard.
- Petri-net/VASS for "unbounded resource": **correct in spirit**; tightened claim — the relevant cheap sub-problem is **coverability** (Karp–Miller; EXPSPACE-complete, Rackoff) for "place → ω"; full VASS *reachability* is decidable and Ackermann-complete (Leroux–Schmitz upper bound; Czerwiński/Leroux lower bound) but is **not needed** for unbounded-resource classification. Avoid over-implying we need full reachability.
- CR: 104.4b / 104.4f (draw), 732.2a (shortcut N), 732.4 (mandatory loop → draw), 732.5–732.6 (breaking) — **all verified** against `docs/MagicCompRules.txt` and matched to existing engine annotations.
- Worked examples — **all card names now verified against `card-data.json`** (§12 corpus). **Heliod, Sun-Crowned + Walking Ballista** correct (lifelink ping ↔ life-gain counter; net-zero counters; unbounded damage/life). **"Kiko, Apogee Mind" was a hallucinated card name** (caught by the user) → real card **Kilo, Apogee Mind**; its combo's unbounded resource is **proliferate triggers** (mana *net-zero*), **not** infinite mana as first written. Bare **"Aang"/"Appa"** are card *families* → pinned to verified **Aang, Swift Savior + Appa, Steadfast Guardian**.

### 11.3 Revised, churn-minimized PR plan (supersedes §7 sizing)
Re-anchoring to the existing `loop_fingerprint`/`loop_window`/`emit_resolution_halt` machinery removes most greenfield code. LOC = rough **net-new** lines incl. tests.

| PR | Revised scope (reuse-first) | ~LOC | Risk | vs. draft |
|----|----|----|----|----|
| **0** | `ResourceVector` + `loop_states_equal_modulo_resources` (board/zones/tap equal; life/damage/counters/mana/cast-count differ). Reuse `loop_fingerprint`/`normalize_for_loop`; extend the existing "volatile" exclusion set. **No new fingerprint.** | 150–300 | low | ↓ (was "audit all GameState") |
| **1** | Make `loop_fingerprint`/`normalize_for_loop` reachable for analysis (visibility bump or a thin `analysis::` re-export) + an offline driver around `GameRunner` for constructed states. | 80–180 | low | ↓ |
| **2 (core)** | Net-progress detection **hooked into the existing `loop_window` site** (`engine.rs:1193`): when fingerprint-modulo-resources recurs with a positive delta, build a `LoopCertificate{unbounded, win_kind, mandatory}`. Reuse window/iter counters. Fixtures from real card-data. **Delivers the offline verifier.** | 300–500 | low (offline) | ↓ (was "build sim engine") |
| **3** | Replace the net-progress branch of `emit_resolution_halt()` (`engine.rs:1290`) with classify→`GameEvent::LoopDetected` + CR 732.2a shortcut (or lethal end); keep the runaway-ceiling halt as fallback. **Hook is one function.** | 150–300 | med (resolution) | ↓ (precise hook) |
| **4 (optional, deferrable)** | Static ability-graph (Engine B) over a card list: reuse `coverage.rs` traversal + per-`Effect` `ResourceVector`. **Biggest churn lever** — the `Effect` enum is large. **Engine A needs none of this** (it uses the real reducer), so defer; ship incrementally by effect-family. Start with mana/counter/damage/untap/cast/token, tag the rest "unmodeled". | 400–1500 | low, broad | ↓ via subset + deferral |
| **5** | `cargo combo-verify` CLI + engine API wrapping 2 (+4). | 150–300 | low | = |
| **6** | `∞` unbounded-resource state+display, generalizing `debug_infinite_mana`. | 300–500 | med | = |
| **7** | Loop shortcut + opponent response window (`WaitingFor`/`GameAction` + frontend, reuse `ModeChoiceModal`); protocol bump. | 600–1200 | high | = (heaviest) |
| **8** | AI coupling: confirmed `LoopCertificate` → top-ranked line; extend `combo/` provenance; `cargo ai-gate` + paired-seed baselines. | 300–500 | med | = |

### 11.4 Churn-minimization levers (the explicit asks)
1. **Reuse the existing detector, don't rebuild it.** PR-0/1/2/3 attach to `loop_fingerprint`/`loop_window`/`emit_resolution_halt` instead of parallel infra — the single biggest reduction (the draft implied a from-scratch fingerprint + simulator).
2. **Engine A requires zero per-`Effect` mapping** (it drives the real reducer), so the high-churn Engine B (PR-4) is **optional and deferrable** — the headline "verify a claimed loop" ships without it.
3. **The live gameplay hook is one function** (`emit_resolution_halt`) whose own comment names the gap — PR-3 is a targeted replacement, not a new subsystem.
4. **Reuse precedents for the costly UI/protocol PRs**: PR-6 generalizes `debug_infinite_mana`; PR-7 mirrors existing interactive `WaitingFor`/modal patterns; PR-8 reuses `combo/` types.
5. **Fixtures from real `card-data`**, never from memory, to avoid encoding hallucinated card text.

**Minimal viable headline tool = PR-0 + PR-2 (+1 plumbing) ≈ 500–900 net-new LOC, offline, no behavior change** — down from the draft's implied multi-thousand-line greenfield. The original cluster *ordering and boundaries hold*; only the sizing and the "build vs reuse" framing changed.

### 11.5 Residual risks the review surfaced
- `loop_fingerprint`/`normalize_for_loop`/`loop_states_equal` are `pub(crate)` — cross-crate reuse (phase-ai/tool) needs a visibility decision (re-export vs move to an `analysis` module).
- The existing fingerprint **includes** life/damage (correct for *mandatory*-loop equality); the net-progress comparison needs the **complement** (ignore exactly the monotone resources). Getting that projection right — and matching it to `loop_states_equal`'s existing volatile-counter exclusions — is the crux of PR-0 and must be unit-tested both ways (false-draw vs missed-combo).
- `FINGERPRINT_AFTER_ITERS=32` means very short net-progress cycles are only detected after 32 mandatory iters; the combo detector may want a smaller threshold for *beneficial* loops (tune separately).

---

---

## 12. Verified, card-disjoint infinite-combo corpus (50)

Sourced from the **Commander Spellbook backend API** (`?q=results:infinite&ordering=-popularity`, paginated) + canonical combos. **Two invariants enforced programmatically:** (1) every card name exists in `card-data.json`; (2) **the 50 combos are mutually card-disjoint — no card is reused across entries**, so each row is a distinct execution path (selected greedily: a combo is admitted only if all its cards are still unused). Distinct cards used: **108**. Doubles as the §11 soundness-validation corpus.

**Search method (reproducible):** Commander Spellbook — `results:infinite`, `cards>=N`/`cards<=N`, `card:"<name>"`, `coloridentity:<wubrg>`, sort `-popularity`. EDHREC groups the same dataset by color identity and early/late 2-card brackets.

| # | Combo (disjoint card set) | Cards | Produces | Category |
|---|---|---|---|---|
| 1 | Basalt Monolith + Rings of Brighthearth | 2 | infinite colorless mana | mana |
| 2 | Grim Monolith + Power Artifact | 2 | infinite colorless mana | mana |
| 3 | Palinchron + Deadeye Navigator | 2 | infinite mana | mana |
| 4 | Devoted Druid + Vizier of Remedies | 2 | infinite green mana | mana |
| 5 | Dramatic Reversal + Isochron Scepter | 2 | infinite mana (w/ rocks) | mana |
| 6 | Pili-Pala + Grand Architect | 2 | infinite mana | mana |
| 7 | Bloom Tender + Freed from the Real | 2 | infinite mana | mana |
| 8 | Priest of Titania + Umbral Mantle | 2 | infinite mana | mana |
| 9 | Dockside Extortionist + Temur Sabertooth | 2 | infinite Treasures/mana | mana |
| 10 | Selvala, Heart of the Wilds + Staff of Domination | 2 | infinite mana + draw | mana |
| 11 | Faeburrow Elder + Pemmin's Aura | 2 | infinite mana | mana |
| 12 | Marwyn, the Nurturer + Sword of the Paruns | 2 | infinite mana | mana |
| 13 | Heliod, Sun-Crowned + Walking Ballista | 2 | infinite damage + life | damage |
| 14 | Mikaeus, the Unhallowed + Triskelion | 2 | infinite damage | damage |
| 15 | Sanguine Bond + Exquisite Blood | 2 | infinite life drain (win) | drain |
| 16 | Marauding Blight-Priest + Bloodthirsty Conqueror | 2 | infinite life drain | drain |
| 17 | Niv-Mizzet, the Firemind + Curiosity | 2 | infinite draw + damage | draw/damage |
| 18 | Blasphemous Act + Repercussion | 2 | mass damage (win) | damage |
| 19 | Professor Onyx + Chain of Smog | 2 | infinite life drain | drain |
| 20 | Kiki-Jiki, Mirror Breaker + Zealous Conscripts | 2 | infinite hasty tokens | tokens |
| 21 | Splinter Twin + Deceiver Exarch | 2 | infinite hasty tokens | tokens |
| 22 | Midnight Guard + Presence of Gond | 2 | infinite tokens | tokens |
| 23 | Scurry Oak + Ivy Lane Denizen | 2 | infinite +1/+1 + tokens | tokens |
| 24 | Dualcaster Mage + Twinflame | 2 | infinite hasty tokens | tokens |
| 25 | Felidar Guardian + Saheeli Rai | 2 | infinite tokens | tokens |
| 26 | Basking Broodscale + Rosie Cotton of South Lane | 2 | infinite mana + tokens | tokens |
| 27 | Ratadrabik of Urborg + Boromir, Warden of the Tower | 2 | infinite tokens/death | tokens |
| 28 | Niv-Mizzet, Parun + Ophidian Eye | 2 | infinite draw + damage | draw |
| 29 | Narset's Reversal + Twinning Staff | 2 | infinite magecraft | draw |
| 30 | Aggravated Assault + Sword of Feast and Famine | 2 | infinite combat | combat |
| 31 | Combat Celebrant + Helm of the Host | 2 | infinite combat | combat |
| 32 | Time Sieve + Thopter Assembly | 2 | infinite turns | turns |
| 33 | Lotus Cobra + Springheart Nantuko | 2 | infinite landfall + mana | landfall |
| 34 | Ashaya, Soul of the Wild + Quirion Ranger | 2 | infinite landfall/ETB | landfall |
| 35 | Scute Swarm + Retreat to Coralhelm | 2 | infinite tokens/landfall | landfall |
| 36 | Worldgorger Dragon + Animate Dead | 2 | infinite mana + ETB | engine |
| 37 | Food Chain + Eternal Scourge | 2 | infinite creature-cast mana | engine |
| 38 | Tidespout Tyrant + Sol Ring | 2 | infinite mana + storm | engine |
| 39 | Aetherflux Reservoir + Bolas's Citadel + Sensei's Divining Top | 3 | infinite life-loss damage | damage |
| 40 | Abdel Adrian, Gorion's Ward + Restoration Angel + Ephemerate | 3 | infinite tokens/blink | tokens |
| 41 | Underworld Breach + Lion's Eye Diamond + Brain Freeze | 3 | infinite mill | mill |
| 42 | Gravecrawler + Phyrexian Altar + Blood Artist | 3 | infinite death + drain | death |
| 43 | Karmic Guide + Reveillark + Viscera Seer | 3 | infinite recursion/sac | death |
| 44 | Chatterfang, Squirrel General + Warren Soultrader + Academy Manufactor | 3 | infinite drain/tokens | death |
| 45 | Reassembling Skeleton + Ashnod's Altar + Nim Deathmantle | 3 | infinite tokens/mana | death |
| 46 | Thopter Foundry + Sword of the Meek + Krark-Clan Ironworks | 3 | infinite Thopters + life | engine |
| 47 | Spike Feeder + Archangel of Thune | 2 | infinite +1/+1 counters + life | counters |
| 48 | Earthcraft + Squirrel Nest | 2 | infinite tokens | tokens |
| 49 | Grindstone + Painter's Servant | 2 | infinite mill (win) | mill |
| 50 | Helm of Obedience + Rest in Peace | 2 | infinite mill (win) | mill |

**By unbounded-resource family:** mana (12), tokens (10), damage (4), engine (4), death (4), drain (3), landfall (3), mill (3), draw (2), combat (2), draw/damage (1), turns (1), counters (1).

_Card-disjointness verified by script: each of the 108 cards appears in exactly one combo. A few entries (Sanguine Bond + Exquisite Blood; Blasphemous Act + Repercussion; Grindstone + Painter's Servant; Helm of Obedience + Rest in Peace; Sharuum line) are deterministic **win** payoffs rather than unbounded loops — retained to exercise all three `WinKind`s (`ImmediateLoss`/`LethalDamage`/`InfiniteLoop`)._
