# PR-4b — Engine-B breadth: remaining effect/trigger families + life symmetry — Implementation Plan

_Worktree: `/home/lgray/vibe-coding/wt-combo-pr4` (branch `feat/combo-detect-pr4`, base **`f3cbf07e6`** = the shipped PR-4a commit, PR #4493). Builds **on top of** PR-4a in the SAME worktree/branch._
_Plan written to the gitignored main-worktree `.planning/` — NEVER part of any PR._
_**Tilt does NOT watch this worktree** — verify with cargo directly (see §9). `cargo fmt --all` always runs directly._
_All file:line anchors re-read in the worktree at plan time. The PR-4a parent plan (`PR4-PLAN.md`) §3.2/§3.3/§3.5 + revision logs are the authority for every modeling convention; this plan only flips Unmodeled→Modeled arm bodies and None→Some trigger arms._

---

## 0. One-paragraph orientation

PR-4a shipped a complete, exhaustive-no-wildcard Engine-B extractor (`crates/engine/src/analysis/ability_graph.rs`, 2013 lines). Its **four drift gates** are already exhaustive and MUST STAY exhaustive: `effect_projection` over all 207 `Effect` variants (`:360`), `trigger_axis` over all 169 `TriggerMode` variants (`:663`), `impl From<&ResourceAxis> for AxisKey` over all 16 `ResourceAxis` variants (`:141`), and `fold_cost` over all 29 `AbilityCost` variants (`:995`). PR-4b changes **only arm bodies inside those matches** (and the helper fns/`Proj` builder they call) — it adds **zero new `Effect`/`AxisKey`/`ResourceVector`/`TriggerMode` variants** and never touches match exhaustiveness. Every `AxisKey` the new arms need (`Life`, `Draw`, `Library`, `Tokens`, `Etb`, `Ltb`, `Death`, `Sac`, `ExtraTurn`, `Combat`) and every `ResourceVector` field (`life`, `cards_drawn`, `library_delta`, `tokens_created`, `etb_triggers`, `ltb_triggers`, `death_triggers`, `sac_triggers`, `extra_turns`, `combat_phases`) **already exist** — confirmed at `ability_graph.rs:91-132` and `resource.rs:39-100`.

---

## 1. Objective + NON-goals

### Objective
Lift Engine-B from the 5 PR-4a priority families (mana/counter/damage/tap/cast) to the full corpus breadth by:
1. Reclassifying the cleanly-mappable `effect_projection` Unmodeled arms to `Modeled` (draw, mill, life, tokens, zone-change, search-as-library, extra-turns/extra-combat, sacrifice/destroy effect side).
2. Landing the **life axis as an atomic symmetric pair** (`Effect::GainLife`/`LoseLife` together with flipping the `AbilityCost::PayLife` cost arm from no-op to negative life) — R3-LIFE-SYMMETRY.
3. Flipping the `trigger_axis` arms whose **matching producers become modeled in 4b** from `None` to `Some(..)` (lifegain→`Life`, dies/destroy→`Death`, sac→`Sac`, LTB→`Ltb`, ETB→`Etb`, token→`Tokens`, draw→`Draw`, mill→`Library`, damage→`Damage`, untap→`Tap`) — this unlocks the trigger-event edges that are the real payoff (aristocrats/dies + Heliod-class lifegain-feedback loops).
4. Tighter cost-side magnitude/polarity where 4a under-approximated, plus the two reviewer-deferral decisions (Conjure; Sacrifice-Death-filter scoping).
5. Non-vacuous discriminating tests whose **headline** cases are the trigger-event-edge payoffs, plus a full 14-family corpus smoke (export-gated graceful-skip).

### NON-goals (hard STOP boundaries)
- **PR-5 is out of scope. STOP before PR-5.** No `cargo combo-verify` CLI binary, no `.cargo/config.toml` alias, no confirmer wiring. PR-4b is library-surface arm bodies + tests only.
- **No new variants on ANY gated engine enum** (`Effect`, `QuantityRef`, `TargetFilter`, `Keyword`, `TriggerMode`, …) and **no new `AxisKey` / `ResourceVector` / `ResourceAxis` variant.** If a family genuinely needs a new axis (P/T, control, win/lose, poison-as-new-key), it **STAYS `Unmodeled`** — see §4 (the categorical-boundary rule forbids inventing an axis; that would be a separate gated `ResourceVector` change, out of PR-4 scope entirely).
- **No resolver/stack/SBA/GameState/reducer/parser/frontend/WASM/phase-ai changes.** Engine-B is still purely-additive offline analysis over `&CardFace` ASTs.
- **Do NOT touch match structure** of the 4 drift gates (no added/removed/reordered enumerated variants). Only arm *bodies* and the helper fns/`Proj`/`NodeAcc` builder change. The one allowed signature change is widening `trigger_axis` input (§3.4) — it keeps matching exhaustively on `TriggerMode`.
- **No `LoopCertificate` reuse for candidates** (§3.8 of parent plan) — still `CandidateCycle`.

This PR changes ZERO game behavior; it is verified by inline `#[cfg(test)]` unit tests + the export-gated corpus smoke, exactly as PR-4a.

---

## 2. Building-block reuse (what 4b touches)

| Building block | Location | 4b use |
|---|---|---|
| `Proj` builder (`add_mana`/`add_counter`/`add_damage`/`mark`) | `ability_graph.rs:206-254` | **Add sibling helpers** `add_life(pid, amount, mag)`, `add_library(pid, amount, mag)`, `add_tokens(amount, mag)`, `add_draw(amount, mag)`, `add_extra_turn`, `add_combat`, plus a field-less `produce(AxisKey)` for `Etb`/`Ltb`/`Death`/`Sac`. These mirror the existing `add_*` shape (write the `ResourceVector` field + `mark` the produced axis). |
| `count_seed(&QuantityExpr)` | `ability_graph.rs:260` | Reuse verbatim for every new production magnitude (`Fixed`→`Fixed(n)`, dynamic→`Unbounded`). |
| `CounterClass::from_counter_type` | `resource.rs` (`pub(crate)` since 4a) | Reuse for counter arms (already used). |
| `net_axis_components` / `add_into` | `ability_graph.rs:1087/1139` | **No change** — they already walk every `ResourceVector` field, so new field writes flow into produces/requires + SCC summation automatically (HIGH-2 invariant). |
| `candidate_coverable` / `unbounded_axes_for` / `classify_win_kind` | `ability_graph.rs:1338`, `resource.rs:507`, `loop_check.rs:343` | **No change** — they already key life (consumed), library (gained, mill = opponent-negative), damage, extra-turns. The new arms just feed them. |
| `target_player(&TargetFilter) -> PlayerId` | **NEW small helper in `ability_graph.rs`** | The §3.6 sentinel convention finally needed for player-keyed axes: `Controller`/`SelfRef` → `CONTROLLER`; everything else → `OPPONENT`. Used by `Mill`, `LoseLife`, `GainLife`, `SearchLibrary`. (4a's `add_damage` hardcodes `OPPONENT`; leave it — damage is opponent-only-by-convention, see §5 residual.) |
| `sac_produces_death(&TargetFilter) -> bool` | **NEW small helper** | Single authority for the Sacrifice-Death-filter decision (§3.5), used by `AbilityCost::Sacrifice` (existing arm), `Effect::Sacrifice`, `Effect::Destroy`, `Effect::DestroyAll`. |

No new types. `target_player`/`sac_produces_death`/the `add_*` helpers are private fns in `ability_graph.rs`.

---

## 3. Per-arm change list

### 3.1 `effect_projection` arms to flip `Unmodeled` → `Modeled` (the table)

All anchors are the `Effect` variant declaration in `crates/engine/src/types/ability.rs`. Currently every row below sits in the bulk Unmodeled arm at `ability_graph.rs:465-647`; flipping = move each out of that arm into a new modeled arm above the Unmodeled terminal. **Magnitude rule is uniform**: production amounts go through `count_seed` (`Fixed`→`Fixed(n)`, dynamic `QuantityExpr`→`Unbounded`); mass/“all”/“double”/“equal to N” effects are `Unbounded`. Polarity: `+` = produced/gained, `−` = consumed.

| Effect variant (ability.rs line) | Family | ResourceVector axis + polarity | Magnitude | AxisKey surfaced |
|---|---|---|---|---|
| `Draw { count, target }` (`:7846`) | draw | `cards_drawn += seed` (scalar, controller-implicit; **+**) | `count_seed(count)` | `Draw` (produces) |
| `Mill { count, target, .. }` (`:8042`) | mill | `library_delta[target_player(target)] -= seed` (**−**, mill drives library down) | `count_seed(count)` | `Library` (gained-axis; opponent-negative ⇒ Decking via `classify_win_kind`) |
| `GainLife { amount, player }` (`:7960`) | **life (symmetric)** | `life[target_player(player)] += seed` (**+**) | `count_seed(amount)` | `Life` (produces) |
| `LoseLife { amount, target }` (`:7971`) | **life (symmetric)** | `life[target_player_opt(target)] -= seed` (**−**; `None`⇒`CONTROLLER` "you lose", `Some(opp)`⇒`OPPONENT` drain) | `count_seed(amount)` | `Life` (opponent-negative ⇒ LethalDamage; controller-negative ⇒ consumed-axis, vetoed unless covered) |
| `Token { count, .. }` (`:7922`) | tokens | `tokens_created += seed` **and** `etb_triggers += seed` (CR 603.6a token entry IS an ETB) | `count_seed(count)` | `Tokens` + `Etb` (produces) |
| `CopyTokenOf { count, .. }` (`:8527`) | tokens | `tokens_created += seed` + `etb_triggers += seed` | `count_seed(count)` | `Tokens` + `Etb` |
| `CreateTokenCopyFromPool { count, .. }` (`:8582`) | tokens | `tokens_created += seed` + `etb_triggers += seed` | `count_seed(count)` | `Tokens` + `Etb` |
| `Investigate` (`:8336`) | tokens | `tokens_created += 1` + `etb_triggers += 1` (Clue token) | `Fixed(1)` | `Tokens` + `Etb` |
| `ChangeZone { origin, destination, .. }` (`:8114`) | zone-change | dest `Battlefield` ⇒ `etb_triggers += 1`; origin `Battlefield`+dest `Graveyard` ⇒ `ltb_triggers += 1` **and** `death_triggers += 1` (creature dies, CR 700.4); origin `Battlefield` (other dest) ⇒ `ltb_triggers += 1` | `Fixed(1)` | `Etb` / `Ltb` / `Death` (produces) |
| `ChangeZoneAll { .. }` (`:8159`) | zone-change | same logic, **mass** ⇒ `Unbounded` | `Unbounded` | `Etb`/`Ltb`/`Death` |
| `Bounce { .. }` (`:8300`) / `BounceAll` (`:8319`) | zone-change | origin Battlefield→hand ⇒ `ltb_triggers += seed` | `Fixed(1)`/`Unbounded` | `Ltb` |
| `Sacrifice { target, count, .. }` (`:8015`) | sac/destroy | `sac_triggers += seed`, `ltb_triggers += seed`, `death_triggers += seed` **iff `sac_produces_death(target)`** (§3.5) | `count_seed(count)` | `Sac` + `Ltb` (+`Death`) — **same polarity as the 4a cost arm** |
| `Destroy { target, .. }` (`:7866`) | sac/destroy | `ltb_triggers += 1`, `death_triggers += 1` **iff** `sac_produces_death(target)` (no `Sac` — destroy ≠ sacrifice) | `Fixed(1)` | `Ltb` (+`Death`) |
| `DestroyAll { target, .. }` (`:8107`) | sac/destroy | mass ⇒ `ltb_triggers`/`death_triggers` `Unbounded` (Death gated by filter) | `Unbounded` | `Ltb` (+`Death`) |
| `ExtraTurn { target }` (`:10169`) | combat/turns | `extra_turns += 1` (**+**) | `Fixed(1)` | `ExtraTurn` (produces) ⇒ ExtraTurns win |
| `AdditionalPhase { phase, .. }` (`:10229`) | combat/turns | `phase` is a combat phase ⇒ `combat_phases += seed` | `count_seed(count)` | `Combat` (produces) |
| `SearchLibrary { count, target_player, .. }` (`:8955`) | search | `library_delta[searched_player] -= seed` (cards leave library; `target_player` `Some`⇒that player, else `CONTROLLER`) | `count_seed(count)` | `Library` (opponent-negative ⇒ Decking, e.g. Bribery; self ⇒ Advantage/self-mill) |
| `Seek { count, .. }` (`:10348`) | search | `library_delta[CONTROLLER] -= seed` | `count_seed(count)` | `Library` |

**Damage** (`DealDamage`/`DamageAll`/`DamageEachPlayer`/`EachDealsDamageEqualToPower`) is **already modeled in 4a** (`ability_graph.rs:429-438`) — no change. **Counters** (`PutCounter`/`MultiplyCounter`/`RemoveCounter`/`Proliferate`) likewise (`:374-427`). **Cast/copy** likewise (`:451-463`).

### 3.2 `effect_projection` arms that **STAY `Unmodeled`** in 4b (explicit decisions — no axis exists)

The categorical-boundary rule (parent CLAUDE.md) forbids inventing a `ResourceVector` axis. These families have **no existing axis**, so modeling them would require a gated `ResourceVector` extension = out of PR-4 scope. They remain in the bulk Unmodeled arm (still flagged `any_unmodeled`, preserving the confidence signal):

- **P/T** — `Pump` (`:7852`), `PumpAll` (`:8065`), `DoublePT` (`:8769`), `DoublePTAll` (`:8781`), `SwitchPT` (`:8474`): no power/toughness axis in `ResourceVector` (CR 208/209 are a separate rule section; unbounded +1/+1 already covered via the *counter* axis). **STAY Unmodeled.**
- **control** — `GainControl` (`:8240`), `GainControlAll` (`:8252`), `ControlNextTurn` (`:8256`), `GiveControl` (`:10401`), `ExchangeControl` (`:10097`): no control axis; one-shot theft is not a repeatable resource. **STAY Unmodeled.**
- **combat-status** — `ForceBlock` (`:9193`), `ForceAttack` (`:9198`), `Goad` (`:10059`), `GoadAll` (`:10066`), `RemoveFromCombat` (`:10410`): no axis; these manipulate combat assignment, not a countable resource. **STAY Unmodeled.** (Only `ExtraTurn`/`AdditionalPhase` from the "combat/turns" group map — §3.1.)
- **dice** — `RollDie` (`:9561`), `FlipCoin`/`FlipCoins` (`:9586`/`:9606`), `FlipCoinUntilLose` (`:9620`): the **container** has no own resource; its `win_effect`/`lose_effect`/`results` branches are **already walked by `collect_effects_in_effect`** (`:883-905`) and projected independently. **Containers STAY Unmodeled** (correct — the payload is the resource, and it is already collected). No change needed.
- **win/lose** — `WinTheGame` (`:9548`), `LoseTheGame` (`:9539`): no `ResourceVector` axis maps to `WinKind::ImmediateWin`; `classify_win_kind` reads only resource axes. Surfacing ImmediateWin from a static net would need a new axis (gated). **STAY Unmodeled** with a one-line note flagging it as the only family with a known recall gap (a repeatable "target opponent loses" loop is missed); deferred to a future ResourceVector-extension PR.
- **set/exchange life** — `SetLifeTotal` (`:10365`), `ExchangeLifeWithStat` (`:10380`), `ExchangeLifeTotals` (`:10388`): these *set* a total (not a signed ± delta) — no clean per-cycle life delta. **STAY Unmodeled.**
- **Conjure** decision (reviewer deferral b) — see §3.5.

> **Discipline note for the executor:** keep the Unmodeled arm's `|`-list exhaustive. As you flip rows out of it, delete exactly those variants from the bulk arm so the match still enumerates all 207 with no wildcard. After editing, the only acceptable compile outcome is success — a non-exhaustive-match error means a variant was dropped from both arms.

### 3.3 The LIFE AXIS — atomic symmetric pair (R3-LIFE-SYMMETRY) — land as ONE commit/change

These three edits MUST land together — never any one alone (keying one life position in isolation creates the coverability false-negative the parent plan §3.3 documents):

1. **`Effect::GainLife { amount, player }` (`ability.rs:7960`)** → `add_life(target_player(player), +seed, mag)` — positive life on the gainer (default `CONTROLLER`). New modeled arm in `effect_projection`.
2. **`Effect::LoseLife { amount, target }` (`ability.rs:7971`)** → `add_life(target_player_opt(target), -seed, mag)` — negative life (`None`⇒`CONTROLLER` self-loss; `Some`⇒resolved player, typically `OPPONENT` drain). New modeled arm.
3. **`AbilityCost::PayLife` cost-fold arm (`ability_graph.rs:1066`)** → flip from the 4a no-op to `acc.net.life[CONTROLLER] -= 1` (unit, recall-safe under-approx of a dynamic life cost per HIGH-1). Move `AbilityCost::PayLife` OUT of the no-op `|`-bucket at `:1066-1079` into its own arm.

Sign semantics confirmed from `resource.rs`: `life` is a **Consumed** axis (`resource.rs:components`); controller-negative net ⇒ `candidate_coverable` veto (`ability_graph.rs:1346-1350`) unless `unbounded_production` covers `Life`; opponent-negative ⇒ drain win surfaced by `unbounded_axes_for` (`resource.rs:514-521`) ⇒ `classify_win_kind` → `LethalDamage` (`loop_check.rs:308-313`). So the symmetric pair makes a gain-and-pay loop net to ≥0 (coverable) while keeping a pure drain as a win — exactly the R3-LIFE-SYMMETRY contract.

### 3.4 `trigger_axis` arms to flip `None` → `Some` (the table) + the ChangesZone signature widening

**Signature widening (required, allowed):** `TriggerMode::ChangesZone`/`ChangesZoneAll` is the shared encoding for BOTH ETB and dies/LTB triggers — the disambiguator (`destination`/`origin`) lives on `TriggerDefinition` (`ability.rs:15490` `mode`, `:15496` `origin`, `:15514` `destination`), **not** on the bare `TriggerMode`. The 4a signature `trigger_axis(mode: &TriggerMode)` (`ability_graph.rs:663`) cannot see those fields. **Change the signature to `trigger_axis(trig: &TriggerDefinition) -> Option<AxisKey>`** and `match &trig.mode { .. }` inside — this keeps the match **exhaustive over all 169 `TriggerMode` variants** (the drift gate is untouched) while letting the `ChangesZone` arm branch on `trig.destination`/`trig.origin`. Update the two call sites in `build_nodes`: `:1227` `trigger_axis(&trig.mode)` → `trigger_axis(trig)` and `:1243` `trigger_axis(&trigger.mode)` → `trigger_axis(trigger)`.

Flip these arms (currently in the bulk `None` group at `ability_graph.rs:681-836`) — flip ONLY consumers whose matching producer is modeled by end of 4b:

| TriggerMode (triggers.rs) | New return | Matching 4b producer |
|---|---|---|
| `ChangesZone` / `ChangesZoneAll` | branch on def: dest `Battlefield`⇒`Some(Etb)`; origin `Battlefield`+dest `Graveyard`⇒`Some(Death)`; origin `Battlefield`⇒`Some(Ltb)`; else `None` | `Effect::Token`/`ChangeZone`→Etb; `ChangeZone`/`Sacrifice`/`Destroy`→Death/Ltb |
| `LeavesBattlefield` (`:219`) | `Some(Ltb)` | `Sacrifice`/`Destroy`/`Bounce`/`ChangeZone`→Ltb |
| `Destroyed` (`:288`) | `Some(Death)` | `Destroy`/`DestroyAll`→Death |
| `Sacrificed` / `SacrificedOnce` (`:287`) | `Some(Sac)` | `AbilityCost::Sacrifice`/`Effect::Sacrifice`→Sac |
| `LifeGained` | `Some(Life)` | `Effect::GainLife`→Life |
| `LifeLost` / `LifeChanged` / `PayLife` | `Some(Life)` | `Effect::LoseLife`/`AbilityCost::PayLife`→Life |
| `TokenCreated` / `TokenCreatedOnce` | `Some(Tokens)` | `Effect::Token`/`CopyTokenOf`→Tokens |
| `Drawn` | `Some(Draw)` | `Effect::Draw`→Draw |
| `Milled` / `MilledOnce` / `MilledAll` | `Some(Library)` | `Effect::Mill`/`SearchLibrary`→Library |
| `DamageDone` / `DamageDoneOnce` / `DamageAll` / `DamageDealtOnce` / `DamageDoneOnceByController` | `Some(Damage)` | `Effect::DealDamage` (4a)→Damage |
| `Untaps` / `UntapAll` | `Some(Tap)` | `SetTapState{Untap}`/`AbilityCost::Untap`→Tap |

**Do NOT flip** (no modeled producer ⇒ flipping would only add inert noise): `Attacks`/blocks/combat family, `LandPlayed` (no `LandfallTriggers` producer modeled), `CounterRemoved` (no counter-removed event producer), `DamageReceived`/`ExcessDamage`/`DamagePreventedOnce`, all the bespoke-mechanic modes. Leave them `None` in their explicit arms (the gate stays exhaustive). The 4a `Some` set ({cast, counter, tap, mana} at `:665-680`) is unchanged.

### 3.5 Reviewer-deferral decisions

**(a) Sacrifice-Death-filter scoping** (deferral a). Now that dies-trigger consumers go live (§3.4), unconditional `death_triggers += 1` from every sacrifice over-produces the `Death` axis for non-creature sacrifices (sac a land/Treasure/Clue), forming spurious Death edges. **Decision: scope `Death` to creature-or-undeterminable sacrifice filters** via a single-authority helper `fn sac_produces_death(filter: &TargetFilter) -> bool`:
- Returns `false` ONLY when the filter **provably excludes creatures** (a `TargetFilter::Typed(TypedFilter)` whose core-type constraint is a non-creature type with no creature alternative — e.g. land-only, artifact-only-non-creature). Inspect `TypedFilter`'s core types (`TargetFilter::Typed` at `ability.rs:3454`).
- Returns `true` for `Any`/`SelfRef`/`Controller`/untyped/creature-typed/undeterminable filters (**recall-first**: over-producing an event axis is recall-safe; a false Death edge is filtered by PR-5).
Apply to **all four sites** (single authority): the existing `AbilityCost::Sacrifice` arm (`ability_graph.rs:1027-1031` — wrap the `death_triggers += 1` in `if sac_produces_death(&sac_cost.target)`), `Effect::Sacrifice`, `Effect::Destroy`, `Effect::DestroyAll`. `Sac`/`Ltb` stay unconditional (any sacrifice is a sacrifice and an LTB). If inspecting `TypedFilter` proves awkward, the recall-safe fallback is "always `true`" (= current 4a behavior) — but prefer the provably-non-creature exclusion since consumers are now live.

**(b) Conjure** (deferral b). `Effect::Conjure { cards, destination }` (`ability.rs:10417`) creates *real cards* (not tokens) into a zone — it is **NOT a cast** (it does not put a spell on the stack; `casts_this_step` would be wrong). **Decision: Conjure does NOT join the cast family.** Model it consistently with the zone-change family by `destination`: `destination == Battlefield` ⇒ `etb_triggers += 1` (a permanent enters, CR 603.6a) ⇒ produces `Etb`; any other destination (hand/library/graveyard) ⇒ **STAY Unmodeled** (card creation into a non-battlefield zone has no clean repeatable `ResourceVector` axis, and conjure is a digital-only edge case). This keeps Conjure→battlefield consistent with `ChangeZone`→battlefield and avoids the cast-axis miscategorization.

**(c) CR-cite cleanup** (deferral c). The `EffectCost` recursion arm comment at `ability_graph.rs:1056` reads `// CR 118.3:`. Verify against `docs/MagicCompRules.txt`: CR 118 is the Life section / 119 is life payment, but the cost-recursion rule is **CR 118** (costs) generally. Re-grep and correct to the verified subsection (likely `CR 118.1`/`CR 118` for "a cost is an action or payment"). See §6.

---

## 4. Idiomatic-Rust + exhaustive-match compliance

- All four matches stay **exhaustive, no `_ =>`**. PR-4b moves variants between existing arms (Unmodeled→Modeled, None→Some) only.
- New production logic flows through `Proj::add_*` sibling helpers (mirror the existing `add_mana`/`add_counter`/`add_damage` shape) and the field-less `Proj`/`NodeAcc` `produces`/`requires` sets for `Etb`/`Ltb`/`Death`/`Sac` — never ad-hoc field writes scattered across arms.
- `target_player`/`sac_produces_death` are typed helpers (return `PlayerId`/`bool`), single-authority, no string matching, no per-card logic.
- `count_seed` is the single magnitude authority — reuse it; never re-derive Fixed-vs-dynamic.
- `BTreeMap`/`BTreeSet` ordering preserved (tests stay deterministic).
- **add-engine-variant gate (run mentally — confirmed):** PR-4b adds **no** variant to any gated enum (`Effect`, `TriggerMode`, `ResourceAxis`, `AxisKey`, `QuantityRef`, `TargetFilter`, …). It only flips arm bodies and adds two private helper fns + `Proj` methods. The gate is **NOT triggered** — no skill invocation required. The four exhaustive no-wildcard matches remain the drift gates.

---

## 5. Player convention + residual

- `target_player(&TargetFilter) -> PlayerId`: `Controller | SelfRef ⇒ CONTROLLER`, else `OPPONENT` (the §3.6 sentinel convention from the parent plan, finally implemented for the player-keyed life/library axes). `target_player_opt(&Option<TargetFilter>)`: `None ⇒ CONTROLLER`, `Some(f) ⇒ target_player(f)`.
- **Residual (documented, not fixed in 4b):** `add_damage` (`ability_graph.rs:240`) still hardcodes `OPPONENT`. Damage to a self/controller target ("deals N damage to you") would mis-key — but no combo *payoff* deals self-damage as its win axis, so this stays an accepted recall-safe over-approximation (a controller self-damage loop is not a win and PR-5 filters it). Leave 4a's `add_damage` unchanged.
- `library_delta` sign: **negative = library shrinks** (mill/search remove cards). Confirmed against `classify_win_kind` (`loop_check.rs:322-330`: opponent-negative library ⇒ Decking) and `unbounded_components` (`resource.rs:438-441`: any nonzero library is surfaced).
- **Residual (L2, documented, not fixed in 4b):** `target_player` defaults every non-`Controller`/`SelfRef` filter to `OPPONENT`. A mill/search whose subject filter is ambiguous ("target player mills", "target player searches") is therefore keyed to `OPPONENT` — so a *self*-mill or *self*-tutor that happens to use a non-controller filter shape would be classified as opponent-decking rather than self-advantage. This is a recall-safe over-approximation (the candidate still surfaces; only its `win_kind`/victim player may be too aggressive) and is filtered/corrected by PR-5's stateful confirmation. The common cases ("you mill", "search your library" with `target_player: None`) key correctly to `CONTROLLER`.

---

## 6. CR annotations (grep-verify against `docs/MagicCompRules.txt` BEFORE writing)

`docs/MagicCompRules.txt` is gitignored/absent in the worktree — run `./scripts/fetch-comp-rules.sh` first, then `grep -n "^XXX" docs/MagicCompRules.txt` for each. **Do not write any annotation you have not grep-confirmed.** Candidate numbers for the new arms:

| 4b concept | Candidate CR | Verify |
|---|---|---|
| life gain/loss/payment axis | CR 119.3 (gain/lose) / CR 118 (costs, for PayLife) | `grep -n "^119.3" / "^118." docs/MagicCompRules.txt` |
| draw axis | CR 120.x? → **CR 121** (drawing a card; verify exact: `grep -n "^121" `) | verify |
| mill axis | CR 701.x mill / CR 104.3c + CR 704.5b decking | `grep -n "^701" + "^104.3c" + "^704.5b"` |
| token creation = ETB | CR 111.1 (token) + CR 603.6a (ETB trigger) | verify both |
| dies = LTB + Death | CR 700.4 (dies) + CR 603.6c (LTB) | verify (4a already cites these at `ability_graph.rs:123-127`) |
| sacrifice | CR 701.21a | already verified in 4a (`:1026`) |
| destroy | CR 701.19a | verify |
| extra turns | CR 500.7 | verify |
| additional phase/combat | CR 500.8 | verify |
| search/library | CR 701.23a | verify |
| **EffectCost recursion cite fix** (deferral c) | CR 118 (costs) | re-grep `:1056` `CR 118.3` → correct |

Run the `validate-cr-annotations` skill discipline on every `// CR` you add/modify.

---

## 7. Non-vacuous discriminating test plan

All inline `#[cfg(test)] mod tests` in `ability_graph.rs`, extending the existing block (`:1449`). Reuse the existing fixture helpers (`raw_node`, `activated`, `mana_effect`, `put_counter`, `deal_damage`, `set_tap`, `build_node`, `candidate_cycles_from_nodes`, `P1P1`). Add helpers `gain_life(n)`, `lose_life_opp(n)`, `token(n)`, `dies_trigger(execute)`, `etb_trigger(execute)`, `lifegain_trigger(execute)` (build `TriggerDefinition` with the right `mode`/`origin`/`destination`).

### A. Per-arm projection tests (revert = delete/flip the arm)
- `gain_life_projects_positive_controller_life` / `lose_life_projects_negative_opponent_life` — paired sign + player split; `GainLife`→`life[CONTROLLER]=+n`, `LoseLife{Some(opp)}`→`life[OPPONENT]=-n`.
- `pay_life_cost_is_negative_controller_life` (symmetric partner) — `fold_cost(PayLife)`→`life[CONTROLLER]=-1`. Revert (PayLife→no-op) flips it to 0.
- `token_projects_tokens_and_etb` — `Effect::Token{count}`→`tokens_created=n` AND `etb_triggers=n`; `produces ⊇ {Tokens, Etb}`. Revert (drop the Etb half) loses the aristocrats edge.
- `sacrifice_effect_produces_sac_ltb_death` — `Effect::Sacrifice` (creature filter) → `produces ⊇ {Sac, Ltb, Death}`, matching the 4a cost-arm polarity. Sibling: a provably-non-creature filter ⇒ `Death ∉ produces` (pins `sac_produces_death`).
- `destroy_effect_produces_ltb_death_not_sac` — `Effect::Destroy`→`{Ltb, Death}`, `Sac ∉ produces`.
- `change_zone_to_battlefield_produces_etb` / `change_zone_bf_to_graveyard_produces_death_ltb` — the `ChangeZone` destination/origin branch.
- `mill_projects_negative_opponent_library` / `extra_turn_projects_extra_turns` / `draw_projects_cards_drawn` — one each, sign + axis.
- `pt_and_control_stay_unmodeled` — `Effect::Pump` and `Effect::GainControl` ⇒ `Projection::Unmodeled` (pins the §3.2 decision; negative sibling proving those arms remain inert).

### B. trigger_axis tests (revert = flip the arm back to None)
- `changes_zone_disambiguates_etb_vs_death` — `trigger_axis` on a dest=Battlefield def ⇒ `Some(Etb)`; on origin=Battlefield/dest=Graveyard ⇒ `Some(Death)`; on origin=Battlefield/dest=Hand ⇒ `Some(Ltb)`. **This test requires the signature widening** — it pins it.
- `lifegain_trigger_requires_life` / `dies_trigger_requires_death` / `token_trigger_requires_tokens` — each consumer maps to its axis. Revert (→None) drops the requirement.

### C. HEADLINE payoff tests (the load-bearing 4b deliverable — trigger-event-edge SCCs)

**C1. Aristocrats/dies loop** `dies_token_aristocrats_loop_is_candidate`:
- Node A (dies→token+drain): `TriggerDefinition{ mode: ChangesZone, origin: Battlefield, destination: Graveyard }` ⇒ `trigger_axis`→`Some(Death)` ⇒ `requires {Death}`. `execute` = `Effect::Token{count:1}` with a `sub_ability` `Effect::LoseLife{Some(opponent), 1}` ⇒ `produces {Tokens, Etb}`, `life[OPPONENT] -= 1`.
- Node B (ETB→sac): `TriggerDefinition{ mode: ChangesZone, destination: Battlefield }` ⇒ `trigger_axis`→`Some(Etb)` ⇒ `requires {Etb}`. `execute` = `Effect::Sacrifice{ creature filter, count:1 }` ⇒ `produces {Sac, Ltb, Death}`.
- Edges: A produces `Etb`, B requires `Etb` ⇒ A→B; B produces `Death`, A requires `Death` ⇒ B→A ⇒ **Tarjan SCC {A,B}**.
- Assert: exactly **1 candidate**, `faces ⊇ {A,B}`, `win_kind == LethalDamage` (opponent life −1 each cycle), `unbounded ⊇ {Life(OPPONENT)}` (and Tokens/Etb/Sac/Ltb/Death).
- **Revert probe 1 (trigger arm):** revert `ChangesZone`→`None` (or `Destroyed`→`None`) ⇒ A requires nothing ⇒ no B→A edge ⇒ no SCC ⇒ **0 candidates**.
- **Revert probe 2 (effect producer):** revert `Effect::Sacrifice`→`Unmodeled` ⇒ B produces nothing ⇒ **0 candidates**. (Pins that BOTH the trigger consumer being `Some` AND the effect producer being modeled are jointly necessary.)

**C2. Heliod-class lifegain-feedback loop** `lifegain_feedback_loop_is_candidate`:
- Node H (Heliod): `lifegain_trigger` (`mode: LifeGained`) ⇒ `requires {Life}`. `execute` = `Effect::PutCounter{Plus1Plus1, 1}` ⇒ `produces {Counter(P1P1,Creature)}`.
- Node F (Spike-Feeder-class): activated; `cost = Composite{ PayLife(1), RemoveCounter{OfType(Plus1Plus1), 1} }` ⇒ `requires {Counter(P1P1,Creature)}` + `life[CONTROLLER] -= 1`; `execute = Effect::GainLife(2)` ⇒ `produces {Life}`, `life[CONTROLLER] += 2`.
- Edges: H produces `Counter`, F requires `Counter` ⇒ H→F; F produces `Life`, H requires `Life` ⇒ F→H ⇒ **SCC {H,F}**.
- Net life: `+2 (gain) − 1 (pay) = +1` (controller, positive) ⇒ coverable; counters net 0. Assert: **1 candidate**, `win_kind == Advantage`, `unbounded ⊇ {Life(CONTROLLER)}`.
- **Revert probe 1 (trigger arm):** revert `LifeGained`→`None` ⇒ H requires nothing ⇒ no F→H edge ⇒ **0 candidates**.
- **Revert probe 2 (life position keyed ALONE — the R3-LIFE-SYMMETRY discriminator):** keep `AbilityCost::PayLife`→negative but revert `Effect::GainLife`→`Unmodeled`. Then net life = `0 (no gain) − 1 (pay) = −1` (controller-negative) ⇒ `candidate_coverable` veto (`:1346`) ⇒ **0 candidates** (a false-negative). This proves the cost side must NEVER be keyed without the effect side — the exact bug the symmetric pair prevents. Re-asserting **1 candidate** with both modeled is the discriminator.

Both headline tests run in CI (synthetic fixtures, no export dependency) and are non-vacuous: each has ≥1 revert that flips the assertion.

### D. Real-card-data corpus smoke (export-gated graceful-skip, mirrors `corpus_priority_family_combo_yields_candidate` at `:1983`)
- `corpus_lifegain_feedback_yields_candidate` — load a real Heliod-class pair via `db.get_face_by_name` (executor verifies names exist in the export, e.g. **Heliod, Sun-Crowned + Walking Ballista** or **+ Spike Feeder**); assert ≥1 candidate naming `Life` and/or `Counter`. Graceful `return` if absent.
- `corpus_aristocrats_dies_loop_yields_candidate` — load a real dies+sac+token/recursion combo (executor verifies, e.g. a persist/aristocrats pair); assert ≥1 candidate naming a `Death`/`Sac`/`Life` axis. Graceful skip.
- A coverage sweep test `corpus_family_smoke_all_14` that, when the export is present, loads a curated card list spanning all 14 families and asserts `candidate_cycles` produces the expected per-family axis on each documented combo, skipping individually-absent rows. Keep recall-first: assert ≥1 candidate per present combo; never fail on extra candidates.

> **Non-vacuity evidence required in the PR report:** for each revert in A/B/C, show the assertion that flips (paste the failing assert). C1/C2 are the headline proofs — explicitly show that each forms a candidate ONLY with the trigger arm `Some` AND (for C2) both life positions keyed.

---

## 8. File-by-file change list

1. **`crates/engine/src/analysis/ability_graph.rs`** (the only non-test file changed):
   - `Proj`: add `add_life`/`add_library`/`add_tokens`/`add_draw`/`add_extra_turn`/`add_combat` + a field-less `produce(AxisKey)` helper (mirror `:222-245`).
   - `effect_projection` (`:360`): add the §3.1 modeled arms; delete those variants from the bulk Unmodeled `|`-list (`:465-647`). Keep §3.2 families in the Unmodeled arm.
   - `trigger_axis` (`:663`): widen signature to `&TriggerDefinition`; flip the §3.4 arms `None`→`Some`; add the `ChangesZone` destination/origin branch.
   - `fold_cost` (`:995`): move `AbilityCost::PayLife` out of the no-op bucket (`:1066`) into a `life[CONTROLLER] -= 1` arm; gate `AbilityCost::Sacrifice`'s `death_triggers` (`:1030`) behind `sac_produces_death`.
   - `build_nodes` (`:1219`): update the two `trigger_axis(&..mode)` call sites to pass the def.
   - Add private `target_player`/`target_player_opt`/`sac_produces_death` helpers.
   - Fix the `EffectCost` CR cite (`:1056`).
   - Add §7 tests to the inline `mod tests`.
2. **No `mod.rs` change** — public surface (`candidate_cycles`, `AbilityGraph`, `CandidateCycle`) is unchanged; 4b only deepens internal projection.
3. **No `Cargo.toml`, `.cargo/config.toml`, resolver, parser, frontend, AI change.**

---

## 9. Verification (cargo-direct — Tilt does NOT watch this worktree)

```
cargo fmt --all                                            # always direct
cargo build -p engine                                      # exhaustiveness: must compile (drift gates)
cargo test -p engine analysis::ability_graph               # the inline unit + headline tests
cargo clippy -p engine -- -D warnings                      # idiom gate
```
Run from `/home/lgray/vibe-coding/wt-combo-pr4`. The headline C1/C2 tests + their reverts are the acceptance signal. Because this worktree is outside the Tilt watch set, there is no target-lock contention concern — run cargo directly. Capture the test output + the revert-flip evidence for the PR report.

---

## 10. Risks + STOP triggers

| Risk | Mitigation / STOP |
|---|---|
| **A family seems to need a new axis** (P/T, control, win/lose). | Per §3.2 it **STAYS Unmodeled**. Do NOT add a `ResourceVector`/`AxisKey` variant — that is gated and out of PR-4 scope. If a corpus row genuinely needs it, note it as a deferred recall gap and move on. |
| **`trigger_axis` signature widening ripples.** | Only two call sites (`build_nodes` `:1227`/`:1243`); both already hold the `TriggerDefinition`. The match stays exhaustive over `TriggerMode`. If any other caller appears (it shouldn't), STOP and re-scope. |
| **ChangesZone disambiguation wrong** (dest/origin fields differ from assumption). | Re-read `TriggerDefinition` (`ability.rs:15489+`) and a real parsed dies/ETB trigger from the export before finalizing the branch; the headline C1 test pins the three cases. |
| **`sac_produces_death` TypedFilter inspection awkward.** | Recall-safe fallback = always `true` (= 4a behavior). Prefer the provably-non-creature exclusion, but never let it return `false` for an undeterminable filter (would drop real dies edges = recall miss). |
| **Over-approximation explosion** on full decks (more candidates from trigger edges). | By design (two-stage; PR-5 confirms). Not a 4b blocker; note candidate counts for PR-5 perf. |
| **Concurrent edits** to `resource.rs`/`loop_check.rs`/`ability_graph.rs`. | Re-read before editing; per CLAUDE.md wait ~10 min on unrelated compile errors. 4b only edits `ability_graph.rs`, minimizing collision surface. |
| **Temptation to reuse `LoopCertificate` / collapse life axis to one position.** | Forbidden (§3.8 parent / R3-LIFE-SYMMETRY). The C2 revert-probe-2 is the guard. STOP if proposed. |
| **Scope creep into PR-5.** | Hard STOP. No CLI, no alias, no confirmer wiring. Library + tests only. |

---

## 11. Summary of key decisions

1. **Life axis = atomic symmetric triple** (`GainLife`+`LoseLife`+`PayLife`-cost-flip) landing together; C2 revert-probe-2 proves keying one position alone causes a coverability false-negative.
2. **Token creation produces `Etb`** (CR 603.6a) and **dies/sac/destroy produce `Death`/`Ltb`/`Sac`** — this is what closes the aristocrats SCC purely on trigger-event edges (Death↔Etb), with no need for a non-existent "creature-presence" axis.
3. **`trigger_axis` widened to `&TriggerDefinition`** so the shared `ChangesZone` mode disambiguates ETB vs dies vs LTB via `destination`/`origin` — the only signature change, match stays exhaustive.
4. **Conjure does NOT join the cast family**; it maps to `Etb` only when `destination == Battlefield`, else stays Unmodeled.
5. **Sacrifice-Death scoped** via single-authority `sac_produces_death` (recall-first: Death unless provably non-creature), applied to all four sac/destroy sites.
6. **P/T, control, combat-status, win/lose, set/exchange-life, dice-containers STAY Unmodeled** — no axis exists and inventing one is gated/out-of-scope; win/lose is flagged as the one known recall gap.
7. **Zero new variants** on any gated enum — add-engine-variant gate not triggered; the four exhaustive no-wildcard matches remain the drift gates.

### Residual risks
- `add_damage` still hardcodes `OPPONENT` (accepted: no self-damage win axis exists; recall-safe over-approx).
- `win/lose` effects unmodeled ⇒ a repeatable "target opponent loses" loop is missed (would need a gated `WinKind` axis; deferred).
- `target_player` self/opponent classification is a sentinel approximation (CR 101 static convention) — multi-opponent nuance is filtered by PR-5.
- `sac_produces_death` may over-include Death for undeterminable filters (recall-safe by construction).
