# PR-4a (#4493) Review Resolution Plan

Branch/worktree: `ship/combo-detect-pr4a` @ `/home/lgray/vibe-coding/forge.rs-ship-combo-pr4a` (ISOLATED → cargo runs DIRECTLY; Tilt watches main checkout only).
File: `crates/engine/src/analysis/ability_graph.rs` (single file).

## Review items (measured from gh API 2026-06-27)

### BLOCKER — matthewevans: `fold_cost` treats `AbilityCost::OneOf` as `Composite` (AND-fold)
Current `:1058`:
```rust
AbilityCost::Composite { costs } | AbilityCost::OneOf { costs } => {
    for c in costs { fold_cost(acc, c); }
}
```
`OneOf` is **disjunctive** — the payer chooses ONE branch (runtime: `casting.rs` routes through
`ActivationCostOneOfChoice`; `cost_payability.rs` is payable when ANY branch is payable; `OneOf` docs in
`types/ability.rs` describe one-of payment). AND-folding sums every branch → invents required axes /
net-negative mana no single branch pays → for a static candidate generator this is a **false negative**
(candidate suppressed though a payable branch closes the loop).

**Fix = optimistic envelope** (proposer maximizes recall; Engine A is the sound confirmer that filters
false positives). Split `Composite` and `OneOf`:
```rust
// CR 601.2f/602.1: a OneOf cost is disjunctive — the paying player chooses one
// branch (runtime ActivationCostOneOfChoice; payable when any() branch is payable).
// A static proposer must NOT AND-fold (inventing requirements / net-negatives no
// single branch pays → false negatives). Envelope the branches optimistically:
// produces = ∪ (any branch choosable), requires = ∩ (only unavoidable across all
// branches), net = per-axis max (most loop-favorable), unbounded = ∪, unmodeled = ∨.
// Engine A confirms which branch actually sustains the loop.
AbilityCost::Composite { costs } => {
    for c in costs { fold_cost(acc, c); }
}
AbilityCost::OneOf { costs } => fold_one_of(acc, costs),
```
`fold_one_of`:
- temp-fold each branch into a fresh `NodeAcc` (`let mut b = NodeAcc::default(); fold_cost(&mut b, c);`)
- if no branches → return.
- `produces_env = ⋃ b.produces`; `unbounded_env = ⋃ b.unbounded_production`; `unmodeled_env = ⋁ b.any_unmodeled`.
- `requires_env = ⋂ b.requires` (BTreeSet `&` reduce over branches; start from branch[0].requires.clone()).
- `net_env` = per-axis MAX across ALL branches, **missing component = 0** (built from the UNION of branch
  keys, NOT a pairwise fold-into-acc — pairwise drops the "acc has key, branch lacks it → max(neg,0)=0"
  case). Walk every `ResourceVector` field exactly as `net_axis_components`/`add_into` do:
  - `mana[i]`: `branches.iter().map(|b| b.net.mana[i]).max().unwrap_or(0)` for i in 0..6.
  - each map (life, damage_dealt, library_delta, counters, generic_triggers): union keys across branches,
    `m = branches.iter().map(|b| b.net.<map>.get(&k).copied().unwrap_or(0)).max()`, insert iff `m != 0`.
  - each scalar (tokens_created, cards_drawn, casts_this_step, landfall_triggers, combat_phases,
    extra_turns, death_triggers, etb_triggers, ltb_triggers, sac_triggers): `branches.iter().map(|b| b.net.<f>).max()`.
- merge envelope into `acc`: `acc.produces.extend(produces_env); acc.unbounded_production.extend(unbounded_env);
  acc.any_unmodeled |= unmodeled_env; acc.requires.extend(requires_env); add_into(&mut acc.net, &net_env);`

Why never a false negative: ∪ produces (any branch's production captured), ∩ requires (only requirements in
ALL branches kept — a per-branch-dodgeable cost is dropped), per-axis-max net (least cost / most production).
Self-consistency: build_node derives requires from net sign; net_env is negative on an axis ONLY when ALL
branches are negative there (= unavoidable = ∩), matching the explicit-set intersection. Single-branch OneOf
≡ folding that branch. Nested OneOf/Composite inside a branch handled by recursive temp `fold_cost`.

**NOTE — keep `net_env` field walk in sync with `net_axis_components`/`add_into`.** This is a new exhaustive
`ResourceVector` walk; a missing field silently mis-envelopes. Annotate it pointing at `net_axis_components`.

### MEDIUM — gemini: `any_unmodeled: bool` → typed enum (R2 / CLAUDE.md "no raw bool")
Two sites: `AbilityNode.any_unmodeled` (`:935`) and `CandidateCycle.any_unmodeled` (`:1321`).
Introduce in this module:
```rust
/// Whether a node/candidate's effects were fully projected, or at least one
/// `Effect`/cost folded to `Projection::Unmodeled` (candidate-confidence flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCompleteness { FullyModeled, ContainsUnmodeled }
```
First grep `analysis/` for an existing completeness/confidence enum to reuse; only add new if none. Replace
the bool on BOTH structs AND `NodeAcc.any_unmodeled` AND `fold_projection`'s `Projection::Unmodeled` write
(set `ContainsUnmodeled`) AND the `|=` aggregation at `:1417` (use a helper / match, not `|=`). Update the
struct-literal at `:1211`, `:1443`, `:1513` and the test fixtures. A `ModelCompleteness::or(self, other)`
or `From<bool>`-free combine helper keeps the aggregation idiomatic (avoid bool round-trips).

### MEDIUM — gemini: `axis_key_to_resource` hardcodes OPPONENT for Damage/Life/Library (`:1358`)
Apply gemini's suggested dynamic resolution: inspect `net` to attribute the axis to CONTROLLER vs OPPONENT
(controller-directed lifegain / self-mill vs opponent-directed drain / mill). Use the exact code from the
PR comment. NOTE: PR-4a defers PayLife/life to PR-4b, so the Life branch is exercised by a direct unit test
(construct a `net` with `life[CONTROLLER] > 0` and assert `axis_key_to_resource(&Life, &net) == Life(CONTROLLER)`).

## Tests (non-vacuous, discriminating)
1. **Maintainer's regression (OneOf disjunction):** build a candidate where OneOf branch A closes a cycle and
   branch B adds an unrelated/unsustainable requirement. Assert the candidate IS emitted. **Discrimination:**
   revert `fold_one_of` to the AND-fold (`for c in costs { fold_cost(acc, c); }`) and show the candidate
   DISAPPEARS (branch B's spurious requirement suppresses it). Record both outputs.
2. **`axis_key_to_resource` player resolution:** unit test Damage/Life/Library each resolving CONTROLLER vs
   OPPONENT from net sign. Discrimination: flip the net sign, assert the player flips.
3. **ModelCompleteness:** assert a node/candidate with an Unmodeled effect reports `ContainsUnmodeled` and a
   fully-modeled one reports `FullyModeled`. Discrimination: the all-modeled case must NOT report ContainsUnmodeled.

## Verify (cargo-direct in the isolated worktree)
`cargo build -p engine`, `cargo clippy -p engine --all-targets -- -D warnings`,
`cargo test -p engine analysis::ability_graph` (or the module path). Capture lib warning count = 0.

## Ship (lead does this, NOT the executor)
Commit by pathspec (the one file) → `git push --no-verify origin ship/combo-detect-pr4a` (updates #4493) →
reply to maintainer (explain envelope + regression) + gemini comments. Conventional commit + trailer
`Assisted-by: ClaudeCode:claude-opus-4.8`.

## Stacking note
PR-4b (wt-combo-pr4, uncommitted) is stacked on PR-4a and extends these SAME functions (fold_cost gate,
any_unmodeled, axis_key_to_resource life axis). PR-4b MUST rebase onto fixed PR-4a and adopt
ModelCompleteness + the OneOf envelope before it can ship. PR-4b cannot merge before PR-4a (stacked).
