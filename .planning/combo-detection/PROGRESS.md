# Infinite-Combo Detector — Implementation Progress

Single source of truth for cross-sub-agent handoff. Spec: `IMPLEMENTATION.md`
(§5 `ResourceVector` field list, §7 PR plan); theory: `FEASIBILITY-AND-PLAN.md`.

## Status board

| PR | Scope | Status | Branch / PR | Head SHA |
|----|-------|--------|-------------|----------|
| **0** | `ResourceVector` + `loop_states_equal_modulo_resources` (reuse fingerprint) | **MERGED** into `upstream/main` | #4092 (merged as `53bff896a`) | `53bff896a` (in main) |
| **1** | analysis sim harness around `GameRunner::act` consuming `ResourceVector` | **MERGED** into `upstream/main` | #4097 (squash-merged as `8a199028d`) | `8a199028d` (in main) |
| **2** | net-progress detector (`analysis/loop_check.rs` `detect_loop` → `LoopCertificate`) + 53-row corpus harness; 10/49 real driven rows | **DONE** (open, unmerged; **both maintainer [HIGH] detector-correctness blockers fixed**; rebased onto current `upstream/main` `a66b79e6e`/v0.3.0; diff is only the 4 analysis files) | `feat/combo-detect-pr2` → https://github.com/phase-rs/phase/pull/4119 | `d84e824bd2ea83c1dab1a39ae1efe53cea724696` |
| 3 | classify `emit_resolution_halt` net-progress → live shortcut | **PR-3 STARTS HERE** (see below) | — | — |
| 4 | (optional) static ability-graph (Engine B) | NOT STARTED | — | — |
| 5 | `cargo combo-verify` CLI over corpus | NOT STARTED | — | — |
| 6 | `∞` unbounded-resource display (generalize `debug_infinite_mana` → `unbounded_resources: BTreeMap<PlayerId, BTreeSet<ResourceAxis>>` over the whole `ResourceAxis` class; engine-owned `DerivedViews` projection `UnboundedResourceView{player,axis}`; mana byte-preserved; field excluded from PartialEq/normalize/fingerprint) | **MERGED** ✅ (squash-merged as `22b212fab`, 2026-06-30; authfix folded in) | `feat/combo-detect-pr6` → https://github.com/phase-rs/phase/pull/4603 | `22b212fab` (in main) |
| **6.25** | order-independence soundness — make `group_is_order_independent` provably sound (event-context + sibling read/write-conflict, fail-closed); fixes a latent CR 603.3b auto-ordering bug | **DEFERRED (R3, 2026-06-30)** — review killed the QoL "Case A" as unsound; reshaped to a correctness PR; funded big push. Notes: `PR-6.25-DEFERRED-FINDINGS.md` | — | — |
| **6.5** | growing-cascade detector for **multiplayer** win-acceleration (≥3p drains grow the stack unboundedly → exact loop-equality never matches → §3 never fires) | **DEFERRED EPIC** — funding-gated. Pathway: distributed-systems cascade-failure analysis / Petri-net coverability. Notes: `PR-6.5-EPIC-GROWING-CASCADE.md` | — | — |
| 7 | loop shortcut + opponent response window (CR 732.2a/732.5) | NOT STARTED | — | — |
| 8 | AI coupling (`LoopCertificate` → top line) | NOT STARTED | — | — |

PR-0 was developed in an isolated worktree off `upstream/main` (`9cf4315b6`),
now removed. Branch pushed to `origin` (`lgray/phase` fork); PR targets
`phase-rs/phase` base `main`.

## PR convention (NON-NEGOTIABLE for every combo-detector PR)

The combo-detector PRs are a single staged series (PR-0 … PR-8). **Every PR body
MUST link its immediate predecessor** so a human (or agent) can follow the
implementation trail end to end without leaving GitHub. Concretely, each PR body
includes a `## Combo-detector series` section with:

1. The full series table (Pos | PR `#number` | Delivers), with the current PR's
   row bolded and marked `(this PR)`.
2. An explicit **`Predecessor: PR-X — #YYYY — <full URL>`** line (number *and*
   URL, not just `#YYYY`), with a one-clause note on what the predecessor
   delivered and how this PR builds on it.

PR-0…PR-3 followed this (PR-3 #4480 is the gold-standard format). PR-4a #4493,
PR-4b #4534, PR-5 #4547 were back-filled to match on 2026-06-28 (their trail
links had been missing). Series map for copy-paste into future PRs:

| Pos | PR | Delivers |
|-----|----|----------|
| PR-0 | #4092 | `ResourceVector` + modulo-resource loop equality (additive). |
| PR-1 | #4097 | Analysis sim harness feeding `ResourceVector`. |
| PR-2 | #4119 | Net-progress `detect_loop` → `LoopCertificate` + corpus harness. |
| PR-3 | #4480 | Live mandatory-loop winner shortcut (drain-cascade, CR 704.5a). |
| PR-4a | #4493 | Engine B static ability-graph extractor (scaffold + 5 families + SCC). |
| PR-4b | #4534 | Engine B effect/trigger breadth + life-symmetry cost. |
| PR-5 | #4547 | `cargo combo-verify` CLI over the 53-row corpus. |
| PR-6 | #4603 | `∞` unbounded-resource display — generalize infinite-mana to the whole `ResourceAxis` class (engine-owned `DerivedViews` projection). |
| PR-6.25 | (deferred R3) | Order-independence soundness fix for `group_is_order_independent` (latent CR 603.3b). Notes: `PR-6.25-DEFERRED-FINDINGS.md`. |
| PR-6.5 | #4904 | Growing-cascade detector for multiplayer win-acceleration (ω-coverability modulo growth) + C0 fail-closed walker + C2 gated order-independent auto-resolve. Notes: `PR-6.5-EPIC-GROWING-CASCADE.md`. |
| PR-6.75 | (planning) | C0-full + C1 precision pass — read/write conflict-profile module (`ability_rw.rs`), latent CR 603.3b same-event fix. Plan: `PR-6.75-C0FULL-C1-PLAN.md`. |

When you open the next combo PR (PR-7+), append its row here and link PR-6 (or the
latest merged predecessor) in its body.

## Artifacts created by PR-0

### Module
- `crates/engine/src/analysis/mod.rs` — module doc + re-exports.
- `crates/engine/src/analysis/resource.rs` — all PR-0 logic + 8 unit tests.
- `crates/engine/src/lib.rs` — added `pub mod analysis;` (one line; the only
  edit to an existing file).

### `ResourceVector` field list AS IMPLEMENTED

```rust
pub struct ResourceVector {
    pub mana: [i64; 6],                                       // [W,U,B,R,G,C], summed across pools  (state-readable)
    pub life: BTreeMap<PlayerId, i64>,                        // per-player life                      (state-readable)
    pub damage_dealt: BTreeMap<PlayerId, i64>,                //                                       (EVENT-FED)
    pub library_delta: BTreeMap<PlayerId, i64>,               // absolute library size at snapshot     (state-readable)
    pub tokens_created: i64,                                  //                                       (EVENT-FED)
    pub cards_drawn: i64,                                     //                                       (EVENT-FED)
    pub casts_this_step: i64,                                 //                                       (EVENT-FED)
    pub landfall_triggers: i64,                               //                                       (EVENT-FED)
    pub combat_phases: i64,                                   //                                       (EVENT-FED)
    pub extra_turns: i64,                                     //                                       (EVENT-FED)
    pub death_triggers: i64,                                  //                                       (EVENT-FED)
    pub etb_triggers: i64,                                    //                                       (EVENT-FED)
    pub ltb_triggers: i64,                                    //                                       (EVENT-FED)
    pub sac_triggers: i64,                                    //                                       (EVENT-FED)
    pub counters: BTreeMap<(CounterClass, ObjectClass), i64>, // incl. +1/+1, loyalty, poison, energy  (state-readable)
    pub generic_triggers: BTreeMap<TriggerKind, i64>,         // proliferate/magecraft/...             (EVENT-FED)
}
```

All §5 axes are present. **Methods:** `snapshot(&GameState)`, `delta(before, after)`,
`is_net_progress()`, `unbounded_components() -> Vec<(ResourceAxis, i64)>`.

### Helper enums

```rust
pub enum ObjectClass  { Creature, Planeswalker, Battle, Player, Other }
pub enum CounterClass { Plus1Plus1, Minus1Minus1, Loyalty, Defense, Poison, Energy, Other }
pub enum TriggerKind  { Proliferate, Magecraft, Constellation, Landfall, Other }
pub enum ResourceAxis { Mana(ManaType), Life(PlayerId), DamageDealt(PlayerId),
                        LibraryDelta(PlayerId), Counter(CounterClass, ObjectClass),
                        Trigger(TriggerKind), TokensCreated, CardsDrawn, Casts,
                        LandfallTriggers, CombatPhases, ExtraTurns,
                        DeathTriggers, EtbTriggers, LtbTriggers, SacTriggers }
```

### Comparison function signature

```rust
pub fn loop_states_equal_modulo_resources(a: &GameState, b: &GameState) -> bool
```

Implemented via a private `project_out_resources(&GameState) -> GameState` that
clones through the existing `normalize_for_loop` and additionally zeroes the
monotone resources (player life/mana/poison/energy/player_counters and the
per-turn life/draw trackers; per-object damage_marked, counters, and the
counter-derived power/toughness/loyalty/defense), then delegates to the existing
`loop_states_equal`.

### Visibility changes
**None.** `loop_fingerprint` / `normalize_for_loop` / `loop_states_equal` remain
`pub(crate)` in `crates/engine/src/types/game_state.rs` and are reached from the
`analysis` module within the engine crate (no `pub` bump needed for PR-0).

## §5 deviation a follow-up MUST know

- **`counters` key type:** spec said `(CounterType, ObjectClass)`. The engine's
  `CounterType` derives neither `Ord` (BTreeMap key requirement) nor a small
  closed set (it has `Generic(String)`, `Keyword(KeywordKind)`, parameterized
  `PowerToughness`). Adding `Ord` crate-wide (and to `KeywordKind`) is a larger,
  non-additive change, so PR-0 introduced the analysis-owned `CounterClass`
  (`Ord`) and maps `CounterType -> CounterClass` in `snapshot`. If a later PR
  needs finer counter granularity, extend `CounterClass` (and the mapping) — do
  NOT add `Ord` to the core `CounterType` just for this.

## Decisions / notes for follow-ups

- **State-readable vs event-fed:** `snapshot()` fills only mana/life/library/
  counters from a `GameState`. Everything else (damage_dealt, tokens_created,
  cards_drawn, casts_this_step, landfall/combat/turn counts, all `*_triggers`,
  `generic_triggers`) is left at `Default` and MUST be fed externally by the
  PR-1 harness from the event stream. They are events, not totals a single
  `GameState` retains.
- **`library_delta` naming:** `snapshot` stores absolute library size; the field
  becomes a true delta only after `delta()`. A mill loop surfaces as a *negative*
  `library_delta`; `unbounded_components()` reports library on `!= 0` (not `> 0`)
  precisely so mill is captured.
- **Consumed vs gained axes:** `is_net_progress` treats mana and life as
  *consumed* (net-negative ⇒ not sustainable ⇒ false); all other axes are
  *gained* and may be individually negative (mill) without disqualifying.
- **Modulo projection is the crux:** the projection zeroes counter-derived
  power/toughness/loyalty/defense alongside counters; this is required so a +1/+1
  or loyalty pump loop compares as the same board. A non-counter P/T change
  (rare, via continuous effect) would also be blurred — acceptable for PR-0
  (false-negative-safe; the strict path still guards mandatory draws). Revisit
  if PR-2 reports false positives.

## Artifacts created by PR-1

### Module additions
- `crates/engine/src/analysis/sim.rs` — the harness. `accumulate_events(&mut
  ResourceVector, &[GameEvent])` folds one action's event stream into the
  **event-fed** axes; `LoopProbe<'r>` wraps a `&mut GameRunner`, snapshots the
  state-readable axes at iteration boundaries, accumulates events across an
  iteration via `act`, and `iteration_delta()` returns the per-iteration vector
  (state-readable half = snapshot delta; event-fed half = the per-iteration event
  tally taken VERBATIM, not differenced — differencing a steady per-cycle gain
  would cancel it to zero). 7 tests (4 building-block + 3 real-pipeline).
- `crates/engine/src/analysis/mod.rs` — registered `pub mod sim;` and re-exported
  `accumulate_events` / `LoopProbe` (the only edit to an existing file).

### Event-feed mapping AS IMPLEMENTED (event-driven, covers a class)
`damage_dealt` ← `DamageDealt{Player}` + `CombatDamageDealtToPlayer`;
`tokens_created` ← `TokenCreated`; `cards_drawn` ← `CardDrawn` (per-card) +
`CardsDrawn{count}` (batch, disjoint paths); `casts_this_step` ← `SpellCast`
(copies excluded); `combat_phases` ← `PhaseChanged{BeginCombat}`; `extra_turns` ←
`TurnStarted`; `etb_triggers`/`landfall_triggers` ← `ZoneChanged{to:Battlefield}`
(landfall iff record core types contain `Land`); `ltb_triggers`/`death_triggers`
← `ZoneChanged{from:Battlefield}` (death iff `to:Graveyard`); `sac_triggers` ←
`PermanentSacrificed`; `generic_triggers[Proliferate]` ←
`PlayerPerformedAction{Proliferate}`. State-readable axes are deliberately NOT
routed through the event feed (snapshot owns them; no double-count).

### Visibility / inventory
**No `pub` bumps** (analysis stays in-crate). `data/engine-inventory.json`
unchanged (no Effect/ability variants).

## Artifacts created by PR-2

PR-2 stayed **off `upstream/main`** — it depends on PR-0 (#4092) + PR-1 (#4097),
neither merged. Base: `feat/combo-detect-pr1` head `c9c62c9ff`. `origin/feat/
combo-detect-pr1` had not advanced at branch time, so no rebase. Head SHA:
`f2eb0c8b377d3be1f9144530579010e2915acbaf`. PR: https://github.com/phase-rs/phase/pull/4119.

### Module additions
- `crates/engine/src/analysis/loop_check.rs` — `detect_loop(&start, &end,
  &delta, mandatory) -> Option<LoopCertificate>`: dual gate
  (`loop_states_equal_modulo_resources` + `is_net_progress`), controller-aware
  net-progress, `LoopCertificate{unbounded, win_kind, mandatory}`, `WinKind`
  classification (`LethalDamage`, `LifeLoss`, `Decking`, `Poison`, `Advantage`,
  …). Building-block unit tests cover every `WinKind` arm + soundness negatives.
- `crates/engine/src/analysis/corpus_tests.rs` — the §8 acceptance corpus: a
  53-row `CORPUS` table (shape-locked), a card-availability test, and a reusable
  driving seam (`build_board`, `build_board_with_vanilla`, `build_board_green`,
  `attach_aura`, `seed_subtype_creatures`, `ability_index_where`,
  `activate_and_resolve`, `resolve_to_priority`, `run_combo`, `assert_combo`).
- `crates/engine/src/analysis/resource.rs` / `mod.rs` — controller-aware
  `is_net_progress` fix + re-exports.

### Real driven rows: 10 / 49 (`DRIVEN_ROW_INDICES`)
0 Heliod+Ballista (`LethalDamage`), 1 Kilo+Freed+Relic (proliferate triggers),
4 Grim Monolith+Power Artifact, 6 Devoted Druid+Vizier, 9 Bloom Tender+Freed,
**10 Priest of Titania+Umbral Mantle (PR-2)**, 12 Selvala+Staff, 13 Faeburrow+
Pemmin, **14 Marwyn+Sword of the Paruns (PR-2)**, 49 Spike Feeder+Archangel.
Each is revert-discriminating (omit the loop-closing action ⇒ no certificate).
Plus synthetic `drive_damage_loop_certificate` + negatives
`drive_board_change_is_not_a_loop` / `drive_idle_board_is_not_a_loop`.

### Drivability model (governs the honest `#[ignore]`s)
`loop_states_equal_modulo_resources` compares objects by `ObjectId`, `battlefield`
order, and `next_object_id`, so PR-2 can only drive **in-place** loops (same
objects tap/untap or gain/lose counters; no object re-entry; continuous P/T is
projected out). Remaining undriven rows fall into measured buckets documented in
`corpus_tests.rs` ("Remaining corpus rows"): object-re-entry (fresh ObjectId),
extra-turn/combat (turn/phase advance), color-converting net-loss (Pili-Pala),
mandatory unbounded drain/draw cascades the engine halts (Sanguine+Exquisite,
Blight-Priest+Conqueror, Niv+Curiosity — no single-step stack seam), ability-copy
churn (Basalt+Rings), and the 4 card-gated rows.

### Honesty corrections this PR
Removed two **measured-false** bucket-doc reasons: "CONTINUOUS-P/T untap engines"
(Priest+Umbral now drives — `project_out_resources` zeroes computed P/T and
`GameState::PartialEq` excludes `transient_continuous_effects`) and "PARSE GAPS /
Marwyn+Sword" (Sword's untap is a real `SetTapState::Untap` in a `ChooseOneOf`
modal). Corrected "45 non-gated" → "49".

### Visibility / inventory
**No `pub` bumps** (analysis stays in-crate). `data/engine-inventory.json`
unchanged (no Effect/ability variants — analysis types only).

### Gate
`cargo fmt --all` clean; `cargo test -p engine --lib analysis::` → 70 passed,
0 failed (all 10 drivers + both `_requires_untap` revert-probes + soundness
negatives + meta-tests). Plan-review 1 round (sound), impl-review 2 rounds
(round 1 → 2 LOW comment nits fixed; round 2 → CLEAN). Scope: 4 analysis files.

### Rebase onto new PR-1 head (2026-06-22)
PR-2 rebased from the OLD PR-1 head `c9c62c9ff` onto the **re-fixed PR-1 head
`065aca47e`** (PR-0 now merged into `upstream/main`; PR-1 #4097 rebased onto
current main). Clean rebase (no conflicts). Resolves the maintainer [HIGH]
stack-boundary finding: PR-2's diff vs its stacked base is now exactly the 4
`analysis/` files (no PR-0/PR-1 API bleed). The 3 prior `wip`/`GRIND` commits
were **squashed** into one production-quality commit `3449ec14e` (new head).
Two review nits fixed (comment-only): dropped stale `next_object_id` clause
(`normalize_for_loop` zeroes it; real discriminator is fresh-`ObjectId` id-keyed
`objects_content_eq` miss + `battlefield` order) and removed inaccurate
`CR 605.3b` tag (605.3b = "mana abilities don't use the stack", unrelated).
Verification: full engine lib suite 13253 passed / 0 failed / 6 ignored;
CI-equivalent clippy (`engine/proptest -D warnings`) clean; `engine-inventory.json`
unchanged. **Export gotcha for future runs:** a stale/cross-checkout
`card-data.json` whose schema is ahead of this branch (e.g. `QuantityModification::
Double` on main vs this base's `Times { factor: 2 }`) FAILS `from_export`, so the
12+ export-gated drivers silently SKIP. Regenerate the export from THIS branch's
own `oracle-gen` so the deserialization schema matches. Restore tracked
`known-tokens.toml` from HEAD after any full `gen-card-data.sh` (the jq token-set
step empties it).

### Rebase-to-main + review fixes (2026-06-22, PR-1 now merged)
**PR-1 #4097 squash-merged into `upstream/main` as `8a199028d`** (PR-0 #4092 also
merged earlier), orphaning PR-2's old stacked base `065aca47e`. Replayed ONLY
PR-2's own commit onto current main:
`git rebase --onto upstream/main 065aca47e feat/combo-detect-pr2` (clean, no
conflicts). New head `ca2a9adf8` (prev `3449ec14e`). **Diff vs `upstream/main` is
now exactly the 4 `analysis/` files (+2623/−1)** — the destructive 74-file/−10570
diff is gone; `mergeStateStatus` `DIRTY` → `BEHIND` (normal merge-queue freshness,
not destructive). `engine-inventory.json` unchanged. This fully resolves both
@matthewevans CHANGES_REQUESTED reviews (both were the single stack-boundary
[HIGH]; no inline code comments — Gemini only hit its quota).

**Honesty proof this round (the plan-review blocker):** the export-gated corpus
drivers silently early-`return;` when `card-data.json` is absent OR schema-stale,
so a green "passed" can be vacuous. Confirmed the gotcha was live: the attached
export predated **#4102 `a0a58753d`** (`QuantityModification::Double` → `Times {
factor }`), failing `from_export` (`unknown variant 'Double'`), so the drivers
were skipping (runtime 0.20s). **Regenerated a schema-matching export from THIS
branch's `oracle-gen`** (cached MTGJSON symlinked in; `known-tokens.toml` left
untouched) → corpus suite `18 passed / 0 skipped`, runtime 5.33s (drivers
actually build boards + drive `apply()`). **Discrimination proven:** adding the
loop-closing untap to `drive_combo_14_marwyn_sword_requires_untap` makes it FAIL
(`detect_loop` returns `Some(cert)`) — the `cert.is_none()` assertion is
load-bearing; reverted. OFFLINE invariant re-verified (no callers outside
`analysis/`). All 65 cited CRs grep-resolve. One review nit fixed: reworded two
`corpus_tests.rs` comments that falsely claimed `#[ignore]` placeholders (rows are
honest card-presence data via `gated_on`, zero `#[ignore]` in the module).

### Gate (rebase-to-main round)
`cargo fmt --all -- --check` clean; `clippy -p engine --all-targets -D warnings`
clean; full engine lib suite **13289 passed / 0 failed / 6 ignored**; corpus
suite **18 passed / 0 skipped** with regenerated export; `engine-inventory.json`
unchanged. Pipeline: `/engine-implementer` — plan 1 round, plan-review 1 round
(corpus-honesty blocker → resolved via export regen + discrimination proof),
impl-review 2 rounds (round 1 → 1 LOW comment nit; round 2 → CLEAN). Pushed
`3449ec14e...ca2a9adf8` via `--force-with-lease`. Reply:
https://github.com/phase-rs/phase/pull/4119#issuecomment-4769640960

### Detector-correctness round (2026-06-22, two [HIGH] blockers @matthewevans)
Maintainer's 3rd review (@ `ca2a9adf8`, CHANGES_REQUESTED) raised two
detector-correctness [HIGH]s (both grep-verified against the CR, no Gemini
hallucination):

**[HIGH] #1 — `project_out_resources` projected away activation-LIMIT state**
(false positive). Blanket-clearing `activated_abilities_this_turn` / `_this_game`
/ trigger tallies erased the gates that make a once-per-turn (CR 602.5b) or
loyalty (CR 606.3) action non-repeatable, so a one-shot activation could compare
as the same loop state and falsely certify. **Fix:** classify the clear-block per
field — `activated_abilities_this_turn`/`_this_game` `.retain` only keys whose
ability carries a per-turn (`OnlyOnceEachTurn`/`MaxTimesEachTurn`) / per-game
(`OnlyOnce`) `ActivationRestriction` (they're bumped *unconditionally*, so neither
blanket-keep nor blanket-clear is correct — selective retain is); **stop clearing**
`triggers_fired_this_turn`/`trigger_fire_counts_this_turn` (single-writer
`record_trigger_fired` writes them only for `OncePerTurn`/`MaxTimesPerTurn`,
CR 603.2h); add an analysis-local per-object `loyalty_activations_this_turn`
compare in `loop_states_equal_modulo_resources` (CR 606.3 — `objects_content_eq`
doesn't compare it, and the strict comparator must NOT be widened);
`ability_resolutions_this_turn` stays cleared (NthResolution is a one-shot branch
selector, CR 603.4, not a repetition gate).

**[HIGH] #2 — controller inferred from every battlefield permanent** (false
negative). `detect_loop` derived the controller set from `surviving_controllers`,
so when the opponent controlled any permanent (every normal board) a drain/mill
loop against them was rejected/downgraded. **Fix:** explicit `controller:
PlayerId` param threaded from the driver's `active_player` (mirrors `mandatory`);
`is_progress`/`unbounded_axes_for`/`classify_win_kind` scope to it; deleted
`surviving_controllers`.

**Tests (revert-proven load-bearing):** activated once-per-turn/-game gate
(negative + unrestricted positive control), trigger OncePerTurn/MaxTimesPerTurn
gate, loyalty CR 606.3 gate, gate-predicate partition unit, and Finding-2
drain/mill positives **where the opponent also controls a permanent** (revert
probe: `classify_win_kind(victim_as_controller)` → Advantage vs real controller →
LethalDamage/Decking). Independently revert-proved in impl-review.

### Gate (detector-correctness round)
`cargo fmt --all -- --check` clean; `clippy -p engine --lib -D warnings` clean;
`cargo test -p engine --lib analysis::` **78 passed / 0 failed**; all CRs
(602.5b, 603.2h, 603.4, 606.3, 704.5a, 104.3c, 121.4) grep-resolve;
`engine-inventory.json` unchanged; OFFLINE invariant re-verified (no callers
outside `analysis/`, strict comparator untouched). Pipeline:
`/engine-implementer` — plan 2 rounds (plan-review round 1 found the
trigger-gate/NthResolution completeness BLOCKER → revised; round 2 CLEAN),
impl-review 1 round → CLEAN with independent revert-proof. Rebased onto current
`upstream/main` `a66b79e6e` (v0.3.0) and pushed `ca2a9adf8...d84e824bd` via
`--force-with-lease`. Reply:
https://github.com/phase-rs/phase/pull/4119#issuecomment-4770728214

### Controller-only-damage round (2026-06-22, one [HIGH] blocker @matthewevans)
Review @ `d84e824bd` (2026-06-22T17:03:00Z): **`classify_win_kind`'s damage
branch was controller-blind.** It classified `WinKind::LethalDamage` via
`delta.damage_dealt.values().any(|&n| n > 0)` while the sibling life-loss and
decking branches already require `*pid != controller`. A self-ping loop whose
controller's life is offset by lifegain (so `is_progress` doesn't reject
controller life @ `loop_check.rs:196`) was certified as a direct `LethalDamage`
win even though no opponent loses (CR 704.5a — verified `MagicCompRules.txt:5464`,
"a player at 0 or less life loses": the victim must be an OPPONENT). Finding
confirmed by reading the actual code (siblings @ 251/267 already correct;
`damage_dealt` is `BTreeMap<PlayerId,i64>` keyed by the damaged player).

**Fix (single private-fn body change + 1 test, in `loop_check.rs` ONLY):**
damage branch now `delta.damage_dealt.iter().any(|(pid, &n)| n > 0 && *pid !=
controller)`, in parity with life/library. Controller-only damage falls through
to `WinKind::Advantage` — a well-formed CR 732.2a beneficial loop that still
names its `DamageDealt(controller)` axis (`unbounded_components` surfaces it for
any positive pid, so `detect_loop` returns `Some`, not `None`/panic), mirroring
self-mill (`Advantage`, not `Decking`) and self-life-loss. Mixed deltas (damage
to both controller and opponent) still classify `LethalDamage` — opponent dies.

Regression `classify_win_kind_controller_only_damage_is_not_lethal`:
`damage_dealt[P0]` (controller) ⇒ `Advantage`; `damage_dealt[P1]` (opponent) ⇒
`LethalDamage`; plus `detect_loop(...)` well-formedness (`Some`, `Advantage`,
`covers([DamageDealt(P0)])`). Discrimination revert-proved twice (executor +
impl-review): reverting only the predicate ⇒ test FAILS `left: LethalDamage,
right: Advantage`; restore ⇒ PASS.

### Gate (controller-only-damage round)
`cargo fmt --all` clean; `cargo test -p engine --lib analysis::loop_check`
**15 passed / 0 failed** (incl. the new regression; all prior-round tests still
green so repetition-gate projection + explicit-controller threading preserved);
CR 704.5a / 732.2a grep-resolve in `MagicCompRules.txt`; `engine-inventory.json`
byte-identical; OFFLINE invariant re-verified (`detect_loop`/`classify_win_kind`
no callers outside `analysis/`; strict CR 104.4b comparator untouched). Pipeline:
`/engine-implementer` — plan-review 1 round CLEAN, impl-review 1 round CLEAN with
independent revert-proof. Rebased onto current `upstream/main`
`f421952150` and pushed `d84e824bd...13a2c2ea6` via `--force-with-lease`
(lease held; remote head confirmed `13a2c2ea6`). Reply:
https://github.com/phase-rs/phase/pull/4119#issuecomment-4771185542

## PR-3 STARTS HERE

Per `IMPLEMENTATION.md` §6/§7, **PR-3 = the LIVE shortcut**: classify
`emit_resolution_halt` net-progress at the live `loop_window` site
(`game/engine.rs`) so the engine recognizes an infinite loop during real play
(CR 732.2a) instead of halting on the runaway ceiling — turning PR-2's offline
`detect_loop` certificate into an in-game shortcut.

- **Predecessor:** PR-2 → https://github.com/phase-rs/phase/pull/4119 (this brings
  in `analysis::loop_check::detect_loop` + `LoopCertificate` + the corpus harness).
- **Branch base:** #4092 AND #4097 (PR-1) are now MERGED into `upstream/main`
  (`analysis::{resource,sim}` are in main). If #4119 (PR-2) has also merged,
  branch off `upstream/main` and confirm `analysis::loop_check::{detect_loop,
  LoopCertificate}` are present. Otherwise branch off `feat/combo-detect-pr2`
  (current head `13a2c2ea6065c68af135e873b31667850e0a694b`, rebased onto current
  main `f421952150` and carrying the controller-only-damage fix) so PR-3
  inherits the detector.
- PR-3 is the first PR that **does** change gameplay (the live shortcut + the
  opponent-response window is PR-7). Wire `detect_loop` into the real
  `loop_window` / `emit_resolution_halt` path; mandatory unbounded cascades
  (Sanguine+Exquisite class) become drivable once the live one-step seam exists —
  promote those corpus rows from `#[ignore]` to driven as PR-3 lands.
- Soundness is paramount: a live false positive ends a real game. Reuse PR-2's
  soundness negatives + add live-play no-false-positive coverage.
