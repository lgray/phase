# PR-6.5 — Growing-Cascade Detector for Multiplayer Combo Win-Acceleration (DEFERRED EPIC)

**Status:** DEFERRED EPIC — gated on explicit user authorization to fund (a "big push"). No
code. This doc is the integrated resume point so the funded session starts from the finding and
the design direction, not from zero.

**Series position:** combo-detector PR-6 → **PR-6.25** (order-independence soundness, also
deferred — see `PR-6.25-DEFERRED-FINDINGS.md`) → **PR-6.5** (this doc) → PR-7 → PR-8.
Predecessor: PR-6 (#4603, MERGED `22b212fab`).

---

## 1. The problem (one sentence)

Live combo **win-acceleration** (recognize "this loop wins" and short-cut to the win via CR
732.2a, instead of iterating) works for **2 players** but silently fails to fire for **3+
players** on the most common multiplayer combos — the all-opponent drains.

## 2. Why — the decisive finding (measured, with code anchors)

The §3 live win-shortcut (`game/engine.rs` ~294-336) only fires when there is a **confirmed
bounded loop**: it is guarded by a non-empty loop-detect ring, and the ring is populated by the
§2 sampler (`game/engine.rs` ~590-606) which accumulates a state only when the post-resolution
wait-state is `WaitingFor::Priority{active_player}` and **clears** the ring on any other window
(including `OrderTriggers`). A loop is "confirmed" when `loop_states_equal_modulo_resources`
(`analysis/resource.rs` ~611-613; canonicalizes stack-entry ids to positions, then requires an
**exact** stack-depth / phase / priority match) matches the current state against a prior sample.

**The ≥3-player failure:** every all-opponent drain ("each opponent loses N life",
"each opponent mills N", etc.) fans out to **one trigger per opponent per cycle**. With ≥2
opponents, each cycle pushes *more* triggers than the last resolves, so the **stack grows
without bound** and never returns to a previously-seen board state. Therefore
`loop_states_equal_modulo_resources` never matches a prior sample (the depths differ every
cycle), the ring never confirms a loop, and §3 never fires. The engine just keeps iterating
(or bails on a resolution-depth guard) instead of declaring the win.

**Why 2p works:** with exactly one opponent there is no fan-out — one drain trigger per cycle,
the loop is **constant-depth and bounded**, the state recurs, equality matches, §3 fires.

**The order-irrelevance work (PR-6.25 / `group_is_order_independent`, `triggers.rs:3474`) is
NECESSARY but INSUFFICIENT.** Auto-resolving order-independent `OrderTriggers` windows keeps the
§2 sampler from clearing the ring (so the loop can be *sampled* in the multi-trigger case at
all) — but it does nothing about the *unbounded stack growth*, which is what actually defeats
the exact-match equality. Even with perfect order-irrelevance, the depths still differ each
cycle. So PR-6.25 is a prerequisite enabler, not the fix.

## 3. Why the obvious patches don't work

- **Relax the equality to ignore stack depth.** Unsound in general: two states with different
  stacks are genuinely different game states; equating them would false-positive non-looping
  positions. The growing cascade is a loop *modulo a monotonically growing, net-progressing
  resource* — that has to be recognized structurally, not by loosening the fingerprint.
- **Raise the iteration / resolution-depth cap.** Doesn't terminate — the cascade is unbounded
  by construction; a bigger cap just spends more before bailing without a certificate.
- **Special-case "each opponent loses life."** Violates build-for-the-class — the pattern is any
  per-opponent (per-object) fan-out that nets progress each cycle (drain, mill, token-make,
  counter-add). Needs the general detector.

## 4. Design direction (the user's lit-search ask)

Do a **literature search** on detecting **growing cascades** efficiently by analyzing the
board's **maximal spanning graph of triggers** (or a similar structural object), rather than by
iterating the cascade. The intuition: model the per-cycle trigger fan-out as a graph whose
edges are "resolving trigger X creates trigger(s) Y"; a cascade that grows unboundedly but makes
**monotone net progress** on some `ResourceVector` axis every cycle is a *win certificate*
without needing the state to literally recur at constant depth.

### Grounding corpus — distributed-systems failure analysis (the major pathway forward)

The growing trigger cascade is structurally a **cascading failure in a distributed system**:
each resolving trigger "messages" its successors into existence, and "does this cascade
terminate or grow unboundedly while making net progress" is a well-studied question. Ground the
planning in this corpus rather than inventing from scratch:

- **Cascading failures in complex networks / power grids** — Motter & Lai (2002) load-redistribution
  cascade model and the large follow-on literature on cascade size, criticality, and containment.
  Frames the per-cycle fan-out as load propagating through a dependency graph.
- **Branching processes & epidemic thresholds** — a Galton–Watson branching process (or the SIR
  basic reproduction number R₀) gives the exact criticality test: if the **mean offspring per
  trigger** (expected successor triggers created per resolution) exceeds 1, the cascade is
  *supercritical* and grows without bound; ≤ 1 it dies out. This is the principled "is it a growing
  cascade" criterion, computable from the trigger graph without iterating.
- **Termination detection in distributed systems** — Dijkstra–Scholten (1980) diffusing computations
  and the Dijkstra–Feijen–van Gasteren termination-detection algorithms model exactly a graph of
  message-spawning processes; the *dual* (provable NON-termination / unbounded growth) is what we
  need, and the diffusing-computation graph is the trigger fan-out graph.
- **Petri-net coverability & Karp–Miller ω-acceleration** — the single most directly applicable
  algorithm: model the stack as an unbounded Petri-net place and triggers as token-spawning
  transitions; the Karp–Miller (1969) coverability tree introduces ω to denote an unboundedly
  growing place and **detects the growing component symbolically, without iterating it** — the
  textbook formalization of "accelerate over the loop instead of running it."
- **Well-structured transition systems (WSTS) / vector addition systems (VAS)** — the general theory
  (Finkel–Schnoebelen) behind coverability and the backward algorithm; supplies termination /
  boundedness decidability results and the framework to prove the detector sound.

The synthesis to aim for: a **net-progress + criticality certificate** — use the branching-factor /
coverability view to recognize the unbounded-but-net-progressing cascade, and a Karp–Miller-style
ω-acceleration to project its terminal effect (every opponent crosses lethal in finitely many
computable cycles) onto the `ResourceVector`, discharging the win via CR 732.2a without literally
iterating. Funded session: pull the current surveys for each thread and verify exact citations
before grounding the design.

Open questions the funded work must answer (seed list — not exhaustive):
- What is the right structural object — the trigger dependency multigraph per cycle, its SCCs,
  a spanning tree/forest, a generating-function/growth-rate argument on stack depth vs. net
  resource delta?
- Net-progress certificate: per cycle, does some opponent's losing resource decrease by a fixed
  positive amount independent of cycle index? (CR 104.5a / 104.3b: the game ends the moment a
  player would lose; the certificate must establish *every* opponent crosses the lethal
  threshold in finite, computable cycles.)
- Multiplayer rules-correctness (carry over from the merged PR-6 MP audit): CR 104.4a
  simultaneous vs. CR 104.3b sequential losses, CR 104.4b drawn-game vs. last-player-standing
  win, the draw-from-empty-library interaction (CR 104.4a / 120.3).
- Interaction with the §2/§3 ring + `loop_states_equal_modulo_resources`: is the growing-cascade
  detector a *replacement* equality (loop-modulo-growth), a *parallel* certificate path, or a
  pre-pass that collapses the cascade to a constant-depth equivalent before sampling?

## 5. Relationship to PR-6.25 and gating

- **PR-6.25** (order-independence soundness) is the deferred prerequisite — see
  `PR-6.25-DEFERRED-FINDINGS.md` for the full R1-staged design (C0 fail-closed dual-axis
  classifier, C1 latent-bug fix, C2 conditional widen) and the measured reachability. PR-6.25 is
  a *correctness* PR independently worth doing; PR-6.5 builds on top.
- **Multiplayer-correctness is non-negotiable** (per the combo-detector MP audit and the
  `combo-detector-supports-multiplayer` standing rule): the detector must be correct for 2+
  players and never gated to 2p-only.
- **Gating:** DEFERRED EPIC. Do not start without explicit user authorization to fund. The
  research + design alone is a substantial effort; the implementation is larger.

## 6. Code anchors (live `main`, verify before resuming)

- §3 live win-shortcut: `crates/engine/src/game/engine.rs` ~294-336 (guarded by non-empty
  loop-detect ring).
- §2 ring sampler: `crates/engine/src/game/engine.rs` ~590-606 (accumulates only on
  `WaitingFor::Priority{active_player}`; clears otherwise).
- Loop equality: `crates/engine/src/analysis/resource.rs` ~611-613 + ~779-803 (exact
  stack/phase/priority; entry-ids canonicalized to positions).
- Order-irrelevance enabler: `group_is_order_independent` `crates/engine/src/game/triggers.rs`
  ~3474 (and PR-6.25's soundness work on the same function).
