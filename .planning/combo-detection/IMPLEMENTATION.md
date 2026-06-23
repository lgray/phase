# Infinite-Combo Detection — Test-Driven Implementation Plan

_Target: phase-rs/phase @ `upstream/main` `1036d0689`. Companion to `FEASIBILITY-AND-PLAN.md` (theory, literature, survey). **This** doc is the buildable spec: it centers the driving + corpus combos as the acceptance test suite and orders the work by test coverage. All codebase claims re-verified against the tree at that commit (anchors inline; verification log §11)._

## 1. Thesis — the corpus is the spec

The detector is "done" when, for each combo in the acceptance suite, it **confirms the loop and emits a `LoopCertificate { unbounded: ResourceVector, win_kind, mandatory }`** naming the correct unbounded resource — and emits **no false positive** (soundness). The suite:

- **3 driving combos** (Heliod + Walking Ballista; Kilo, Apogee Mind + Freed from the Real + Relic of Legends; Doc Aurlock + Aang, Swift Savior + Appa, Steadfast Guardian).
- **50 card-disjoint corpus combos** (`FEASIBILITY-AND-PLAN.md` §12) — 113 distinct cards total, no card reused, every name verified in `card-data.json`, every combo verified as a real listed combo on Commander Spellbook.

**49 of the 53 are end-to-end testable on today's engine** (§3). Detector code is **decoupled** from card coverage — the harness `xfail`s the 4 combos whose cards aren't fully implemented yet.

## 2. The three driving combos — expected certificates (acceptance)

| Combo | Loop (one cycle) | Expected `LoopCertificate` | Testable now? |
|---|---|---|---|
| **Heliod, Sun-Crowned + Walking Ballista** | remove +1/+1 → deal 1 (lifelink) → gain 1 → Heliod returns the +1/+1; board identical | `unbounded={damage(opp), life}`, `win_kind=LethalDamage`, `mandatory=once-started` | ✅ |
| **Kilo, Apogee Mind + Freed from the Real + Relic of Legends** | Relic taps Kilo for 1 → Kilo "becomes tapped → proliferate" → Freed `{U}` untaps Kilo; **mana net-zero** | `unbounded={proliferate triggers}`, `win_kind` = via proliferated poison→`ImmediateLoss` / +1+1 / loyalty, `mandatory=optional` | ✅ |
| **Doc Aurlock, Grizzled Genius + Aang, Swift Savior + Appa, Steadfast Guardian** | airbend exiles each other → recast from exile (free via Doc Aurlock) → re-trigger | `unbounded={casts-from-exile, Ally tokens, experience}`, `win_kind=advantage/tokens`, `mandatory=optional` | ⛔ gated (Doc Aurlock has Unimplemented parts) |

These three exercise the three hardest `ResourceVector` axes: **board-resource** (damage/life), **trigger/event count** (proliferate — *mana is net-zero, so a mana-only model misses it*), and **cast/token count** (recast-from-exile).

## 3. Card-support prerequisite (measured today)

Of the **113** distinct corpus cards, **108 are fully implemented** (parse to 0 `Effect::Unimplemented`); **5 carry Unimplemented parts**, gating **4 combos**:

| Gated combo | Card(s) needing completion |
|---|---|
| D3 (Doc Aurlock airbend) | Doc Aurlock, Grizzled Genius |
| #19 Professor Onyx + Chain of Smog | Professor Onyx |
| #36 Worldgorger Dragon + Animate Dead | Animate Dead |
| #49 Grindstone + Painter's Servant | Grindstone, Painter's Servant |

→ **49/53 combos are end-to-end testable now.** The remaining 4 unblock automatically as those cards are completed by the ongoing standard-coverage work (no detector change). The harness marks them `#[ignore]`/xfail with the blocking card noted.

## 4. What already exists (re-verified — corrected vs. earlier drafts)

| Capability | Status | Anchor |
|---|---|---|
| Canonical fingerprint + deep-equality | **EXISTS** — `loop_fingerprint()` (FxHash: turn/phase/priority, zones, per-player life + zone-sizes, per-object tapped+damage), `normalize_for_loop()`, `loop_states_equal()` (already ignores volatile counters) | `types/game_state.rs:7410,7452,7468` |
| Mandatory-loop detection → **draw** | **EXISTS** — `loop_window` (`FINGERPRINT_AFTER_ITERS=32`, `MAX_LOOP_WINDOW=128`), deep-equality confirm → CR 104.4b/732.4 `GameOver{winner:None}` | `game/engine.rs:1102–1223` |
| Net-progress loop handling | **THE GAP** — `emit_resolution_halt()` halts on `MAX_EVENT_GROWTH=50_000`/`MAX_OBJECT_GROWTH=16_000`/iter-cap; its comment: "a net-progress loop is a CR 732.2 shortcut the engine cannot infer an iteration count for" | `game/engine.rs:1102,1172,1290` |
| Sim substrate | `GameRunner::act(GameAction)->Result<ActionResult>` over `im::`-persistent `GameState` (cheap fork) | `game/scenario.rs:1089` |
| Floating mana | `ManaPool { mana: Vec<ManaUnit> }` | `types/mana.rs:1339` |
| Combo vocabulary (to reuse as output) | `ComboLine`/`WinKind{ImmediateLoss,InfiniteLoop,LethalDamage}`/`ComboReachability` (hand-authored — to be driven algorithmically) | `phase-ai/src/combo/` |

**Implication:** the work is *narrow* — extend the existing mandatory-loop detector to also recognize **net-progress** loops (same fingerprint/window machinery, a resource-projected comparison) and classify the unbounded resource. Not a greenfield detector.

## 5. `ResourceVector` — concrete spec (derived from the corpus families)

The corpus produces these unbounded-resource families (count → field):

```
mana(12)         -> mana: [i64; 6]            (W,U,B,R,G,C; from ManaPool deltas)
tokens(11)       -> tokens_created: i64
damage(6)        -> damage_dealt: BTreeMap<PlayerId, i64>
death(4)/engine  -> death_triggers, etb_triggers, ltb_triggers, sac_triggers: i64
drain(3)         -> life: BTreeMap<PlayerId, i64>
mill(3)          -> library_delta: BTreeMap<PlayerId, i64>
landfall(3)      -> landfall_triggers: i64
draw(3)          -> cards_drawn: i64
combat(2)/turns  -> combat_phases: i64, extra_turns: i64
counters(2)      -> counters: BTreeMap<(CounterType, ObjectClass), i64>   (incl. +1/+1, loyalty, poison)
proliferate(1)   -> generic_triggers: BTreeMap<TriggerKind, i64>  (proliferate, magecraft, …)
storm/casts      -> casts_this_step: i64
```

`ResourceVector` = a struct of these monotone counters; a **net-progress cycle** is one where, between two `loop_states_equal`-equal-modulo-resources states, ≥1 component strictly increased and no consumed component went net-negative. The "win" component (damage/poison/mill/etc.) sets `WinKind`. **This field list is the PR-0 deliverable.**

## 6. Detection architecture (corpus-aligned)

- **Engine A (dynamic) — the corpus validator.** Hook the existing `loop_window` site (`engine.rs:1188`): in addition to the strict-equality mandatory check, test `loop_states_equal_modulo_resources` (board/zones/tap identical, resources differ) with a positive `ResourceVector` delta → emit `LoopCertificate`. Reuses `loop_fingerprint`/`normalize_for_loop`. **Every one of the 49 testable combos is confirmable here — no per-`Effect` mapping required** (the real reducer plays the cards).
- **Engine B (static) — optional, off the corpus critical path.** Ability-graph + per-`Effect` resource vectors for **card-list** scanning ("do these cards contain a loop without a board?"). The corpus does **not** need it; defer.

## 7. Implementation plan, re-evaluated by corpus coverage (LOC = net-new incl. tests)

| PR | Scope (reuse-first) | ~LOC | Corpus unlocked | Risk |
|----|----|----|----|----|
| **0** | `ResourceVector` (§5) + `loop_states_equal_modulo_resources`; reuse `loop_fingerprint`/`normalize_for_loop` | 150–300 | measurement substrate for all 49 | low |
| **1** | analysis sim harness around `GameRunner::act`; expose `loop_fingerprint` (visibility) | 80–180 | (enables fixtures) | low |
| **2** | net-progress detection at `engine.rs:1188` → `LoopCertificate{unbounded,win_kind,mandatory}` | 300–500 | **all 49 testable combos** | low (offline) |
| **3** | classify `emit_resolution_halt` net-progress → `GameEvent::LoopDetected` + CR 732.2a shortcut | 150–300 | live-play shortcut for the same | med (resolution) |
| 4 | *(optional)* static ability-graph (Engine B) for card-list scanning | 400–1500 | none (not corpus path) | low, broad |
| 5 | `cargo combo-verify` CLI/API over the corpus | 150–300 | corpus as a CLI suite | low |
| 6 | `∞` unbounded-resource display (generalize `debug_infinite_mana`) | 300–500 | UI for confirmed loops | med |
| 7 | loop shortcut + opponent response window (CR 732.2a/732.5) | 600–1200 | interactivity | high |
| 8 | AI coupling (`LoopCertificate` → top line; drives `combo/`) | 300–500 | AI uses corpus loops | med |

**Corpus-passing milestone = PR-0 + PR-1 + PR-2 ≈ 500–900 net-new LOC** — offline, zero gameplay change, validated by **49 concrete tests**. PR-3 adds the live shortcut; PR-4 (the only high-churn item) is explicitly **not** required to pass the corpus.

## 8. Test harness (data-driven, one row per combo)

A single corpus table drives all cases (mirrors the existing `GameRunner` scenario tests):

```
for (cards, loop_actions, expected_unbounded, expected_winkind, gated_on) in CORPUS:
    runner = GameRunner::from_cards(cards)        # build board from card-data
    install_combo_pieces(runner)                  # zones/attachments per the combo
    for _ in 0..K: runner.act(loop_actions)       # drive a few iterations
    cert = detect_loop(runner.state)              # Engine A
    assert cert.unbounded ⊇ expected_unbounded
    assert cert.win_kind == expected_winkind
```

- 53 rows; the 4 `gated_on`-nonempty rows are `#[ignore = "needs <card>"]` until card-support lands.
- Each row is **discriminating**: reverting the detector (or the modulo-resource projection) flips the assertion (revert-probe required, per repo convention).
- Soundness row-set: a handful of **non-loop** boards must yield **no** certificate (no false positives).

## 9. Definition of Done

- **PR-2 merged ⇒ 49/53 corpus combos confirmed** with correct `unbounded`+`win_kind`, 0 false positives on the soundness set.
- **PR-3 merged ⇒** the same loops are auto-shortcut in live play (CR 732.2a) or drawn if all-mandatory (CR 732.4), with opponent priority preserved (deferred detail → PR-7).
- The 4 gated combos flip from `#[ignore]` to green automatically as Doc Aurlock / Professor Onyx / Animate Dead / Grindstone+Painter's Servant reach 0-Unimplemented.

## 10. Risks & honest caveats

- **Card-support prerequisite** — an end-to-end combo test needs all its cards fully modeled; the detector is decoupled but the *acceptance* of a given row isn't. Tracked per-row.
- **Fingerprint projection** — the existing fingerprint *includes* life/damage (right for mandatory equality); the net-progress compare needs the *complement* (ignore exactly the monotone resources). This projection, reconciled with `loop_states_equal`'s existing volatile-counter exclusions, is the crux of PR-0; unit-test both directions (no false draw / no missed combo).
- **Randomness** — combos with shuffle/coin-flip in the cycle are reported probabilistic/bounded, not infinite (corpus avoids these).
- **Optional / "may" loops (CR 732.6)** — the certificate records optionality; the shortcut offers an iteration count rather than auto-resolving.
- **Undecidability bound** — Turing-complete ⇒ sound-but-incomplete; the corpus is the *floor* of what must work, not a completeness claim.

## 11. Verification log (this document)

- **Codebase anchors re-checked on `1036d0689`:** `loop_fingerprint`/`normalize_for_loop`/`loop_states_equal` @ `game_state.rs:7410/7452/7468`; `loop_window` + `FINGERPRINT_AFTER_ITERS=32`/`MAX_LOOP_WINDOW=128` + `MAX_EVENT_GROWTH=50_000`/`MAX_OBJECT_GROWTH=16_000` + `emit_resolution_halt` @ `engine.rs:1102–1290`; `GameRunner::act` @ `scenario.rs:1089`; `ManaPool` @ `mana.rs:1339`; `combo/` layer @ `phase-ai/src/combo/`. No hallucinated identifiers.
- **Corpus card audit:** 113 distinct cards, **0 missing** from `card-data.json`, **108 fully implemented**, 5 with gaps (→ 4 gated combos, enumerated §3). Card-disjointness re-parsed from the written §12 table (each card in exactly one combo).
- **Combo audit vs. EDH databases:** the 50 are sourced from the **Commander Spellbook backend API** (`results:infinite`); canonical additions spot-checked as real listed combos (e.g., Grindstone + Painter's Servant → infinite mill; Earthcraft + Squirrel Nest → infinite tokens — both confirmed present, 1 variant each). The 3 driving combos verified card-by-card + mechanism (Kilo = proliferate, *mana-neutral*; airbend = recast-from-exile).
