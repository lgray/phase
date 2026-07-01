# PR-6.25 — DEFERRED FINDINGS (R3, 2026-06-30)

**Status:** DEFERRED via R3 (user, token conservation → funded big push). Decision cycle concluded.
No code shipped; no commits; no pushes. This doc preserves everything so the big push resumes from here,
not from zero. Branch context: `feat/combo-detect-pr65` @ upstream/main `ce8fbf96e`; predecessor PR-6
(#4603, MERGED 22b212fab). Forward work folds into the PR-6.5 growing-cascade epic.

Origin: PR-6.25 was scoped as two "sound sub-wins" — (a) widen `group_is_order_independent` to
auto-resolve order-irrelevant simultaneous trigger groups (CR 603.3b), (b) 2-living mutual-drain → Draw.
Adversarial review KILLED (a) as conceived and surfaced a real latent bug; (b) was dropped as
synthetic-only. The work pivoted to a correctness PR (make the predicate provably sound). Then deferred.

---

## 1. VERDICT — original "Case A" widening is UNSOUND

**Case A** (the proposed widening): auto-resolve, with NO prompt, a same-controller group of ≥2 pending
triggers that share: no ordering input; identical normalized ability; identical
condition/subject_match_count/may_trigger_origin; AND `ability_uses_trigger_event_context == false` —
for DISTINCT firing events of ANY class (dropping today's ZoneChanged-departure-batch-only restriction).

**The theorem ("any resolution order ⇒ identical final state") is FALSE.** Decisive counterexample
(representable, traced, code-verified):

- Ability body = "put a +1/+1 counter on each creature you control" + "draw a card if this creature's
  power is 6 or greater". Two byte-identical copies S1 (live power 3), S2 (live power 4 — asymmetry from a
  pre-existing counter/aura; the ABILITY AST is identical, only the OBJECT power differs).
- Threshold = 6. Order [S1,S2]: S1 counters all (S1 3→4, S2 4→5), Source power 4 < 6 no draw; S2 counters
  all (S1→5, S2→6), Source power 6 ≥ 6 DRAW ⇒ **1 card**. Order [S2,S1]: S2 first (S1→4,S2→5) Source=5 no
  draw; S1 (S1→5,S2→6) Source=5 no draw ⇒ **0 cards**. Final hand/library differ — order-observable.
- ALL Case A gates pass: `trigger_has_no_ordering_input` holds ("each creature you control" is a filter
  scope, draw has no chosen target — triggers.rs:3435-3443); condition equal
  (`AbilityCondition::QuantityCheck{lhs:Power{scope:Source}, GreaterOrEqual, Fixed(6)}`, ability.rs:14465);
  normalized abilities equal (`normalize_ability_identity` strips only `source_id`, triggers.rs:3340);
  and crucially **`ability_uses_trigger_event_context == FALSE`** because the only refs are
  `ObjectScope::Source` / `QuantityCheck` / `PutCounterAll` — `ObjectScope::Source` serializes as the bare
  string `"Source"` (ability.rs:3987), NOT in the 12-entry allowlist (triggers.rs:3352-3366).

**Why the proof's corollary is wrong:** the corollary claimed "self-source ⇒ effects on DIFFERENT objects
⇒ disjoint ⇒ commute". That leap is false — a **controller-scoped WRITE** ("each creature you control")
mutates the SIBLING source, and a **non-linear (threshold) self-power READ** makes the two resolution
functions non-commute. "No event context" ≠ "reads nothing a sibling can mutate".

**Why (a0) allowlist-hardening does NOT fix it:** event-context detection is a DIFFERENT AXIS. The hole
here is a read-set/write-set conflict on sibling-mutable game state, not an event-context reference. A
complete event-context detector still admits this counterexample (it reads `ObjectScope::Source`, not the
firing event). Both axes must be gated.

Independent verification (team-lead, against live main): allowlist fail-open + drifted
(triggers.rs:3350-3381, `unwrap_or(true)`, `TriggeringSourceController` absent); `TriggeringSourceController`
genuinely event-context (targeting.rs:796-798 `let event = event?`); same-event short-circuit returns at
:3413 BEFORE the context gate at :3416. Counterexample sound, order-observable, zero event-context.

---

## 2. THE REAL LATENT BUG (pre-existing, ships today)

`group_is_order_independent` can **auto-order order-DEPENDENT triggers TODAY**, a latent **CR 603.3b**
violation (CR 603.3b: the controller orders simultaneous triggers — a choice the engine is silently
removing). `trigger_events_match_for_ordering` (triggers.rs:3408) returns `true` whenever
`first.trigger_event == candidate.trigger_event` (line 3413) — BEFORE the event-context / any
order-sensitivity check at 3416. So two identical no-input triggers off ONE event auto-order with no
soundness check at all. Case A would have WIDENED this latent same-event bug to all distinct-event classes.

### Reachability measurement (card-data.json, 35,396 cards; parsed `triggers[].execute` ASTs + oracle)
Unsound class = two byte-identical no-ordering-input triggers off one resolution context whose resolution
READS sibling-mutable state AND the sibling WRITES it. Split by stage:

- **C1 (same-event short-circuit :3413, the "today" bug): 0 real cards.** A same-event identical pair
  needs a PLAYER/controller event ("whenever you cast/gain/upkeep", "whenever a creature you control
  dies") firing ≥2 identical copies that both read+write board state. Measured count = **0**. ⇒ C1 is
  **latent-but-unreached** today; its fix needs a SYNTHETIC discriminator test (the counterexample),
  framed "fixes a latent CR 603.3b unsoundness; no current printed card reaches it." Lower live urgency.

- **C2 (distinct-event widen, the conditional/droppable stage): the REAL cards live here.** Non-legendary
  SELF-pump reading a board aggregate the sibling mutates = **Rubblebelt Rioters** and **Orcish
  Siegemaster** ("Whenever this creature attacks, IT gets +X/+0, X = greatest power among creatures you
  control"). Two copies attack → per-creature attack events = DISTINCT `trigger_event` → they **PROMPT
  correctly TODAY** (ZoneChanged-only fallback returns false → prompt). The C2 widening would WRONGLY
  auto-resolve them unless C0's read-set gate excludes them. ⇒ **C2 is the genuinely dangerous stage;
  C0's read-set gate is load-bearing precisely there.** This is the real-card reason C2 is
  conditional/droppable.

- **Vetted SAFE / non-reachable:** `obscura ascendancy`, `soul echo`, `ed-e` (read/write their OWN
  counters → disjoint → commute); `edgar master machinist`, `tuya bearclaw`, `selvala`, `the great henge`,
  `the skullspore nexus` (LEGENDARY → no identical pair); `impetuous protege` (reads OPPONENTS' board,
  pumps self → disjoint); `pathbreaker ibex` and the board-wide "creatures you control get +X = greatest
  power" variant = order-INVARIANT (symmetric running-max: total added = X1+X2 either order).

**Net steer:** real-card exposure is entirely in the C2 distinct-event widening (2 cards), all currently
correct. The same-event C1 fix is latent-only hardening (synthetic test). C0's read-set gate is the
load-bearing piece for safely enabling C2.

---

## 3. THE BLESSED DESIGN — R1-staged (for the big push to pick up)

Reframe (a) from "widen auto-resolve" to **"make `group_is_order_independent` provably SOUND"**.
`group_is_order_independent` is currently UNSOUND (#1-hard-rule correctness defect) — fix regardless of
any QoL widening.

- **C0 — the sound building block.** A fail-closed, compiler-EXHAUSTIVE (no wildcard `_ =>`) classifier
  over BOTH axes:
  - (i) **event-context reads** — replaces the fail-open 12-entry string allowlist
    (`value_contains_trigger_event_context_ref`, triggers.rs:3350) AND folds in the (a0) hardening
    (+ `ObjectScope::EventSource`, which also serializes to a bare string missed by the allowlist).
  - (ii) **sibling-mutable read-set / write-set conflict** — reject any ability that READS
    source/recipient/controller/board-scoped mutable state (`ObjectScope::Source`/`Recipient` in
    Power/Toughness/counter `QuantityRef`s; `QuantityRef` Max/Min aggregates over a battlefield set;
    `AbilityCondition::QuantityCheck` / `ControllerControlsMatching` / `SourcePowerAtLeast` /
    `SourceHasCounterAtLeast`) when a sibling WRITES that state. Honest sufficient condition: "identical
    ability that reads nothing a sibling can mutate."
  - **Non-negotiables:** (1) the AST WALK must ITSELF be exhaustive — a gap in the walker is as fail-open
    as a gap in the allowlist; REUSE an existing exhaustive ResolvedAbility visitor (candidates found:
    `game/coverage.rs` exhaustive formatters `fmt_target`:459 / `FilterProp`:566 / `QuantityRef` matches
    — proves those enums maintain exhaustive arms; `analysis/ability_graph.rs`; the `engine-inventory-gen`
    crate — evaluate which truly traverses sub_ability/else_ability/nested before hand-rolling). (2) New
    enum variants must FAIL TO COMPILE without an explicit context/conflict decision (exhaustive match,
    not allowlist + reflection test). Categorical-boundary rule: event-context and read-set conflict are
    DISTINCT axes — separate predicates/combinators, not one tag bag.

- **C1 — apply C0 at the same-event short-circuit (triggers.rs:3413). MUST-HAVE; fixes the latent bug.**
  Non-vacuity test (A3 is vacuous — can't force non-identity order on an auto-resolved group): the
  discriminating assertion is **"the counterexample group now PROMPTS instead of auto-ordering"** — revert
  C1 ⇒ it auto-orders (no prompt). Synthetic, since reachability C1 = 0 real cards. Equivalent to R2.

- **C2 — apply C0 at the distinct-event path, relaxing the ZoneChanged restriction. QoL widening.
  CONDITIONAL + DROPPABLE.** "Rides free" is FALSE (the coverage-regression gate is parse-only — zero
  runtime signal). C2 needs RUNTIME regression evidence: a no-OrderTriggers-regression run + an exact
  player-ordering reproduction at runtime. If not clean+cheap, DROP to C0+C1 (= R2). Watch
  `zone_changes_are_same_departure_batch` (triggers.rs:3383) going dead-code → clippy `-D warnings` fail:
  reduce `trigger_events_match_for_ordering` to the idiomatic
  `first.trigger_event == candidate.trigger_event || (!ability_uses_trigger_event && reads_no_sibling_mutable)`
  and delete the helper + rewrite the 3459-3473 doc comment in the same commit.

- **(b) 2-living mutual-drain → Draw: DROPPED.** Synthetic-only (no real card pair: of 26 "each player
  loses N life" cards, the 9 recurring are once-per-turn/combat-gated/optional; CR 104.4b excludes
  optional-action loops). The natural OFF path already produces the correct draw, just slower. A sound
  Draw also requires the Path-1 simultaneity proof (life_profile + single_shared_drain_step) — CR 104.4a
  needs SIMULTANEOUS loss; staggered crossings are a sequential WIN (CR 104.3b/704.5a). Not worth it.

- **PR/commit shape (when resumed):** one PR; C0 (classifier + tests) → C1 (apply at :3413 + synthetic
  prompt-discriminator test) → C2 (if runtime evidence clean). Roadmap edits are LOCAL-ONLY .planning/
  bookkeeping, NEVER committed (.planning/ is gitignored).

---

## 4. OTHER MEASURED FINDINGS WORTH KEEPING

- **Event-context allowlist is fail-open and DRIFTED.** `value_contains_trigger_event_context_ref`
  (triggers.rs:3350-3366) is a 12-entry hand-maintained string allowlist; `ability_uses_trigger_event_context`
  (3377) `unwrap_or(true)` on serialize failure but FALSE for any unlisted string. Live event-context
  variants that ESCAPE it (measured against ability.rs enums):
  - `TargetFilter::TriggeringSourceController` (ability.rs:3729) — resolution reads the firing event
    (targeting.rs:796 `extract_source_from_event(event)?`). The proof-breaking escapee.
  - `ObjectScope::EventSource` (ability.rs:3997, "the object referenced by the current trigger event") —
    serializes `"EventSource"`, missed.
  - `TargetFilter::ParentTargetSlot`, `QuantityRef::TimesCostPaidThisResolution`,
    `CastManaObjectScope::TriggeringSpell`, `RestrictionPlayerScope::ParentTargetedPlayer` — all missed.
  - (`EventContextSource*Power/Toughness/ManaValue` are FORMER variants, subsumed by
    `effect_context_object` per ability.rs:4002 — survive only in doc comments, NOT live escapees.)
- **`drain_order_triggers_with_identity` (triggers.rs:3605) is NOT test-only** — pub fn with production
  callers (engine.rs:13779/13960/19271/20801, casting.rs:20144, casting_costs.rs:9056/9280,
  effects/change_zone.rs:4739, database/synthesis.rs, game/scenario.rs; doc: "engine's auto-advance path").
  Do NOT route the widening through it (it submits identity for EVERY prompt incl order-sensitive). The
  soundness authority is `group_is_order_independent`; this function stays untouched.
- **coverage-regression CI gate is PARSE-ONLY** (.github/workflows/ci.yml runs
  scripts/coverage-regression-check.sh diffing coverage-data.json from `analyze_coverage`, parser-gap
  analysis only — no runtime resolution). Zero signal for a pure trigger-ordering change. Use runtime
  semantic tests + a cast→resolve→assert-GameState integration test as the real safety net.
- **`zone_changes_are_same_departure_batch` (triggers.rs:3383)** has exactly one caller
  (`trigger_events_match_for_ordering`:3422) → dead-code under C2. The ZoneChanged restriction is
  LOAD-BEARING (not over-conservatism, contra the original plan): departing sources resolve self-refs via
  LKI on a board they don't asymmetrically perturb — that's WHY it's safe. The author's code comment
  rationale ("a CounterAdded trigger can create more CounterAdded events") MISIDENTIFIES the reason
  (CR 603.3b/704.3: abilities created during resolution go on the stack in a LATER pass, don't retro-join
  the current group); correct the comment when touched.
- **Chokepoint architecture CONFIRMED sound:** `begin_trigger_ordering` (triggers.rs:3515) is the sole
  production ordering decision; callers `process_triggers`:3234 and `drain_deferred_trigger_queue_unchecked`:4562
  both route through `group_is_order_independent` at :3544; `trigger_events_match_for_ordering`'s only
  caller is `group_is_order_independent`:3491. `prune_pending_trigger_order` /
  `build_next_order_triggers_prompt` only refresh already-decided groups (don't call the predicate).
- **`normalize_ability_identity` CONFIRMED sound** — strips only `source_id`, recurses sub/else; any
  under-normalization of nested abilities is the SAFE direction (more prompts, never a false auto-resolve).
- **Existing predicate tests (triggers.rs:26127-26346) all stay green** under any of these changes
  (they key on event-equality / subject_match_count / allowlisted TriggeringSource / description /
  controller — none flips). `combat_damage_order_triggers_no_hang.rs` also stays green (distinct effects).
- **CR numbers (grep-verified in docs/MagicCompRules.txt):** 603.3b (controller orders simultaneous
  triggers), 603.4 (intervening-if), 704.3 (SBAs as one event), 704.5a (0 life loses), 104.2a (sole
  survivor win), 104.3b (0-life player loses at next priority/SBA), 104.4a (simultaneous loss → draw),
  104.4b (mandatory-loop draw, excludes optional-action loops).

---

## 5. POINTERS

- Full 5-reviewer per-finding output was at (session-scoped /tmp, will NOT survive — essential content
  distilled into §1-4 above): `/tmp/claude-1000/-home-lgray-vibe-coding-phase-rs-workdir/`
  `4dafeda3-bac2-49f0-b3b1-25f7e1c39124/tasks/w17n3bp16.output` (438 lines; reviews = lines 1-318,
  rest is agent telemetry). Review run id `wf_a98a79bf-b27` (4 dimensions: soundness-A, blast, tests-b,
  idiom-roadmap + synthesis; verdict = blocker).
- Driver working memory (worktree, also session-scoped): `wt-combo-pr65/.pr65-research-log.md`,
  `wt-combo-pr65/.pr625-plan.md` (the BLOCKED plan with the verdict block + re-scope options).
- Roadmap entries (PR-6.25 reshaped, PR-6.5 deferred-epic + maximal-spanning-graph lit-note, PROGRESS.md
  PR-6 MERGED-22b212fab fix): handled by team-lead in this .planning/ tree.
- Key code anchors (live main): triggers.rs 3340/3350/3377/3383/3408/3413/3474/3515/3605/3544/26127-26346;
  targeting.rs:796; ability.rs 3729/3987/3997/4002/14465; .github/workflows/ci.yml coverage gate.
