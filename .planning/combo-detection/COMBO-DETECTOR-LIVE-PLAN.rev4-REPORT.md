# Rev 4 — planner's report

**Deliverable:** `COMBO-DETECTOR-LIVE-PLAN.rev4.md` · **Laboratory:** `combo-probe-wt @ efc76ca1b`
**Code truth:** `phase-rs-workdir @ efc76ca1b` (read-only; **zero `crates/` edits** — verified below).

---

# 1. HEADLINE — both halves of the mandate came back POSITIVE

| | Result |
|---|---|
| ⭐⭐ **B1** | **THE CANARY OFFERS.** `WaitingFor::LoopShortcut`, `unbounded=[TokensCreated]`, `win_kind=Advantage`. |
| ⭐⭐ **U3** | **LOAD-BEARING.** The revert-probe **FLIPS TO FAIL**. The `projected` term ships. |
| ⭐⭐ **B3** | **REGRESSION ELIMINATED.** `Off` is byte-identical — **measured**, and the full suite is **`16550 passed; 0 failed`** (Rev 3's own baseline was `16547 passed; **1 failed**`). |
| ⭐ **non-vacuity** | The negative twin **arms the capture** and still **does not offer** — it discriminates at the *drive*, not upstream. |

---

# 2. EVERY PROBE I RAN (command → actual output)

All in `combo-probe-wt`; `cargo fmt --all` before every reported result.

### P1 — B1: the capture gate *(`probe_rev4.rs`, NEW)*
```
$ cargo test -p engine --test integration probe_rev4 -- --nocapture --test-threads=1
===== B1+: canary (Presence of Gond + Intruder Alarm) =====
  battlefield BEFORE ActivateAbility = 3
  battlefield AFTER  ActivateAbility = 3   (stack=1)
  ⛔ Rev3's DEAD gate `bf.len() > before` = false
  ✅ Rev4 STATIC capture armed          = true
  ⇒ OFFER (WaitingFor::LoopShortcut)    = true
     certificate.unbounded = [TokensCreated]
     certificate.win_kind  = Advantage
     certificate.mandatory = false
===== B1-: NEGATIVE TWIN (Gond, NO untapper) =====
  battlefield BEFORE/AFTER ActivateAbility = 2 / 2
  ✅ Rev4 STATIC capture armed          = true      ← NON-VACUITY: it ARMS
  ⇒ OFFER (WaitingFor::LoopShortcut)    = false     ← and STILL does not offer
test result: ok. 2 passed; 0 failed
```
**The fix:** Rev 3's gate ran at a beat where the battlefield provably has **not** moved (an activated ability only
goes on the **STACK**, CR 602.2a). Rev 4 arms on a **STATIC** predicate — `collect_effects(def).any(Effect::Token)` —
**genuinely** mirroring the recast's `is_token_creating` (`casting_costs.rs:6789`), at the real handler
(`engine.rs:3286`, the non-mana branch).

### P2 — U3: the P2-d merge gate *(`resource.rs` `#[cfg(test)]`, NEW)*
```
$ cargo test -p engine --lib projected_reading_modification_still_vetoes
test analysis::resource::tests::projected_reading_modification_still_vetoes_the_firewall ... ok

# REVERT-PROBE — `|| ...reads_projected_resource(m)` DELETED from the scan-(4) veto:
thread '...' panicked at crates/engine/src/analysis/resource.rs:5099:9:
  U3: a PROJECTED-reading modification MUST veto the firewall
test result: FAILED. 0 passed; 1 failed
```
⇒ **IT FLIPS. LOAD-BEARING. VERDICT: the veto MUST be `sibling || projected`.**

### P3 — B3: the shared-authority split
```
$ cargo test -p engine --lib review_probe_r3 -- --nocapture     # the REVIEWER's own B3 probe
  ability_uses_event_context    = true
  ability_reads_sibling_mutable = true
  c2_order_independent (triggers.rs:3894) = false      ← MAIN BASELINE. Engine PROMPTS.

$ cargo test -p engine --lib event_and_sibling_axes_unchanged_for_typed
test result: ok. 1 passed        ← the repo's OWN over-edit guard, GREEN and UNMODIFIED

$ cargo test -p engine --lib
test result: ok. 16550 passed; 0 failed; 7 ignored
```

### Type enumerations — **AST-measured (`ast-grep`), never a `sed` range**
```
$ ast-grep scan --inline-rules '<enum_variant inside ContinuousModification>' types/ability.rs | jq length
53          # Rev 3 said 41
$ ... ManaProduction  → 15   (an ENUM, not a struct with a `count`)
$ ... Keyword         → 198  (the review sampled 7)
$ ... StaticMode      → 119  (⇒ AddStaticMode genuinely has NO walker)
$ ... PtValue         → 3    ✅ as claimed
$ ... Effect::Mana    → 5 fields (produced, restrictions, grants, expiry, target)
```

---

# 3. HOW EACH BLOCKER IS DISCHARGED

| | Discharge |
|---|---|
| **B1** — capture gate dead | ✅ **FIXED + MEASURED.** Static `Effect::Token`-in-chain predicate at `engine.rs:3286`, mirroring the recast. **Canary offers.** Acceptance #1 reachable, #2 no longer vacuous. |
| **B2** — `ManaProduction` wrong type | ✅ **REBUILT.** `scan_mana_production`, exhaustive over **15** variants + `Effect::Mana`'s **5** fields. ⭐ `DistinctColorsAmongPermanents { filter }` and `TriggerEventManaType` (no `count`, read board/event) are the **false-certificate** cases Rev 3 would have created — each gets a **named discriminator test** whose revert-probe is *Rev 3's own design*. |
| **B3** — shared-authority trap | ✅ **SEPARATED + MEASURED.** New `ScanMode { Conservative, LoopFirewall }` (**P0**, a hard prerequisite of P2/P3). See §4. |
| **B4** — `Effect::Token.keywords` | ✅ **RECLASSIFIED read-bearing**, + `ContinuousModification::AddKeyword`/`RemoveKeyword`. See §5 — **it is bigger than the review said.** |
| **B5** — `ContinuousModification` = 53 | ✅ **ALL 53 classified** by measured payload type (33 read-free / 8 quantity-bearing incl. ⭐`AddCounterOnEnter` / 2 keyword / 2 ability / **8 FAIL-CLOSED**). ⭐ **`CopyValues` / `AddStaticMode` / `GrantStaticAbility` DEMOTED to `CONSERVATIVE`** — their "depth ≤ 1" descent reaches `ReplacementDefinition` / `StaticMode` (**119 variants**) / `ParsedCondition`, **for which no walkers exist** (⇒ **DEFERRED-4**). |
| **`min_by_key`** | ✅ **FIXED at the root, not patched.** Do **not** re-find by `CardId` for an activation at all: **pin the source by `ObjectId`** — it is a *stable battlefield permanent* whose id survives the clone **and** the growth. This sidesteps `CardId(0)` (which every plain token carries) entirely. A **recast** card genuinely churns incarnations (CR 400.7) and keeps its `card_id` re-find — so the re-find becomes **`Recast`-only**. Hostile test: a **token-driven** loop with fodder present. |
| **G4** — positional `ability_index` | ✅ `AbilityDefinition` derives `PartialEq, Eq` (`ability.rs:15612`) ⇒ **re-validate the definition by `Eq` at every drive iteration**; any layer-driven drift ⇒ `RecastAbort`, fail-closed. **Hostile fixture: two auras granting two abilities to one creature.** |
| **G2** — serialized surface | ✅ **AUDITED** (`add-engine-variant` Step 7): serialized, **zero consumers**, `#[serde(default)]` + no `deny_unknown_fields` ⇒ silent lossy drop. **⇒ ship `#[serde(alias = "last_recast_context")]`.** |
| **G1** — variant gate not run | ✅ **`/add-engine-variant` run for `LoopAction`** — §6 of the plan, all three stages + Steps 2/3/4/7. |

---

# 4. THE B3 SEPARATION DESIGN — and the proof `Off` is byte-identical

**The measurement that made the design obvious.** Mapping every `ability_scan::` consumer:

> **The ONLY non-loop-detection consumer of the `event`/`sibling` axes is `game/triggers.rs` — exactly TWO call sites
> (the CR 603.3b gate).** Everything else is `analysis/resource.rs`. And `fire_time_conditions_read_growing_class`
> (`resource.rs:1457`) is called from **exactly two** places, `:968` and `:1131` — **it is the firewall and nothing
> else.**

The two consumers want **opposite approximations** (CR 603.3b: under-reading ⇒ auto-order a group the player must
order ⇒ **rules-wrong**. Firewall: under-reading ⇒ **false certificate**; over-reading ⇒ a missed offer ⇒ **safe**).

**⇒ `ScanMode { Conservative, LoopFirewall }`, threaded explicitly through the walk.**
- Every existing public entry keeps its signature and passes **`Conservative`** ⇒ **`triggers.rs` is not touched at all.**
- Only `fire_time_conditions_read_growing_class` runs in **`LoopFirewall`**. `..._read_projected_resource` (`:2152`)
  **stays `Conservative`** (it feeds the regression-pinned ω/drain cover) — **minimum blast radius.**
- The three divergent arms (`Effect::Token`, `Effect::Mana`, `TargetFilter::Typed`) branch on `mode`.

**Why the #4603 proof is STRUCTURAL, not statistical: not one `Conservative`-mode arm changes.**

### ⭐⭐ The smoking gun is the repo's OWN test name *(team-lead's steer — adopted, verified)*
**`game/triggers.rs:23237` — `fn pr625_c2_distinct_event_auto_orders_even_when_loop_detection_off()`** ✅ *(verified at
that exact line)*. The repo **already ships a test asserting the C2 ordering gate operates when `loop_detection` is
OFF** ⇒ direct, in-repo proof the CR 603.3b path is **loop-detection-INDEPENDENT**. Cited in the plan's P0.
✅ **It PASSES under the Rev-4 split, unmodified:**
```
$ cargo test -p engine --lib pr625_c2
test game::triggers::tests::pr625_c2_distinct_event_auto_orders_even_when_loop_detection_off ... ok
```

### ⭐⭐ `batch_conflict` — **MOOT for Rev 4, BY CONSTRUCTION. Not "discharged."**
The terminal decision is `c2_order_independent && !batch_conflict` (`triggers.rs:3654`). The reviewer's UNVERIFIED
`batch_conflict == false` establishes whether **Rev 3's** `c2` flip was *observable* — i.e. it grades **the severity of
Rev 3's bug**.
> **Rev 4 leaves `c2_order_independent` byte-unchanged (MEASURED: `false`, identical to `main`). A conjunction with one
> invariant conjunct and one untouched conjunct is invariant for EVERY value of `batch_conflict`.** My design does not
> lean on it, so I did not measure it — **and I am not claiming it discharged.**

⛔ **STANDING CONDITION (filed as U10):** the moment any future revision changes a `Conservative`-mode arm,
`batch_conflict` becomes **load-bearing again and MUST be measured** (fixture: two identical *"whenever this creature
deals combat damage to a player, create a 1/1"* creatures). **The group shape is reachable in code; no printed card was
ever bound to it.** Do not let a later change silently inherit Rev 4's exemption.

**Measured:**

| | Rev 3 | **Rev 4** |
|---|---|---|
| `c2_order_independent`, vanilla `Effect::Token` ability | `true` ⇒ **AUTO-ORDERS** (silent live regression) | ✅ **`false`** ⇒ **PROMPTS** = `main` |
| `event_and_sibling_axes_unchanged_for_typed` (`resource.rs:3926`) | ⛔ **RED** — Rev 3 *re-authored the guard* | ✅ **GREEN, UNMODIFIED** |
| full lib suite | `16547 passed; **1 failed**` | ✅ **`16550 passed; 0 failed`** |
| canary still offers (firewall keeps precision) | — | ✅ **yes** |

> **A guard you must rewrite to make your change pass is a guard that is telling you something.** Rev 3 rewrote it.
> Rev 4 does not touch it.
>
> ⚠️ **`ordering_parity_sweep` never touches `ability_scan`** — Rev 3's regression would have landed **silently
> green**. P0 makes it structurally impossible; three targeted tripwires guard it; the sweep's blind spot is filed as
> **DEFERRED-5**. (`FORGE_TEST_FULL_DB=1 ordering_parity_sweep` delta: **expected zero, NOT RUN — U7.**)

---

# 5. WHAT I FOUND THAT THE BRIEF AND THE REVIEW DID **NOT**

### ⛔ N1 — **There are TWO F1 conjuncts. Rev 3 and the review both named only one.**
`last_recast_context` is compared in **two** cover gates:

| site | function | Rev 3 / review |
|---|---|---|
| **`analysis/resource.rs:662`** | `loop_states_equal_modulo_resources` | ⛔ **never mentioned** |
| `analysis/resource.rs:1444` | `eq_except_growable` | named ✅ (review: "CONFIRMED") |

**Both are ONE-SIDED-SAFETY discriminators; BOTH must be renamed and KEPT** (+ two paired tests, `:5672`/`:5697`).
Renaming one leaves the other failing to compile — *fail-safe*, but a plan that audits one of two comparison sites has
not audited the field.

### ⛔ N2 — **`normalize_recast_frame`'s strip is NOT a no-op under `Activate`. It would DELETE the driving permanent.**
Rev 3's P1-d claim #3 (*"there is no such card ⇒ the strip is a no-op"*) is **REFUTED**. `normalize_recast_frame`
(`engine.rs:1599`) removes every object matching `(card_id, from_zone, controller)`. An activation's context has
**`from_zone == Battlefield`** ⇒ it matches **the driver itself**. **The strip must be `Recast`-only.**

### ⛔ N3 — **B4 is much larger than stated, and payload-shape classification is UNSOUND.**
`Keyword` has **198 variants** (the review sampled 7). And the read does **not** live in the payload:
> **`Convoke`, `Delve`, `Improvise`, `Bargain`, `Station` are UNIT variants carrying NO payload — and they all READ
> THE BOARD.** The engine's own shipped exhaustive authority `keyword_cost_reads_growing_class` (`ability_scan.rs:3867`)
> says so.

⇒ `scan_keyword` **= the shipped semantic authority `.or()` a payload descent.** Reuse, don't re-derive 198 arms.
The B4 discriminator test therefore uses **`Keyword::Convoke`** — a *unit* keyword — so it *also* proves that
classifying by payload shape is insufficient.

### ⚠️ N4 — `ShortcutDecisionSchema { iteration_count: Fixed(1) }` observed in the live canary certificate.
I did **not** verify that `DeclareShortcut { count: Fixed(N) }` materializes **N** tokens for the *activation* shape.
**Filed as U8** for the implementer.

---

# 6. THINGS IN THE BRIEF I REFUTED / CORRECTED

1. **"A `_ =>` or `..` in any scan the firewall consumes is a FALSE-CERTIFICATE HOLE."** — **Correct only when the
   wildcard's value is the ACCEPTING value.** `_ => Axes::CONSERVATIVE` is **fail-closed** (safe, merely imprecise);
   `_ => Axes::NONE` is the hole. This matters *practically*: it is the difference between hand-classifying 198
   `Keyword` arms (each a potential false certificate) and reusing a shipped authority. **§0.1 of the plan states the
   refined rule.** *(I still mandate exhaustive no-wildcard matches wherever practical — because that makes `rustc`
   enumerate the type, which is a measurement.)*
2. **The reviewer's finding (d), "`activation_loop_without_untapper_does_not_offer` is VACUOUS"** — correct **given
   B1**, but it is **not intrinsically vacuous**. With B1 fixed it **discriminates**, and I measured *why*: the capture
   **arms** and the rejection comes from the **drive**. The plan now **asserts `ctx.is_some()`** in that test as the
   permanent non-vacuity guard.
3. **The brief's framing that B3 needs the blanket "made precise but guarded"** — the *sharper* answer is that the two
   questions must **stop sharing one answer**. Once separated, `Off`-byte-identity is **structural**, and the repo's
   own tripwire passes **unmodified** rather than being re-authored.

---

# 7. DESIGN DECISIONS YOU MAY WANT TO OVERRULE

1. ⭐ **`ScanMode` threading is invasive** (~30 `scan_*` signatures). I judged it correct over the alternatives (a
   duplicated walker = two sources of truth; a thread-local = hidden state). **It is mechanical and
   compiler-enforced.** If you want a smaller diff, the fallback is a `Scanner` struct holding `mode` — same
   semantics, different ergonomics. **Say so now; it is the first thing the implementer writes.**
2. **`fire_time_conditions_read_projected_resource` (`:2152`) stays `Conservative`.** This deliberately leaves the
   ω/drain cover imprecise. Rationale: it is regression-pinned and **not** what the canary needs — minimum blast
   radius. **Overrule if you want the drain path to get the precision too** (it would need its own regression run).
3. **`CopyValues` / `AddStaticMode` / `GrantStaticAbility` fail closed.** This *loses* the copy/grant-static class
   until the three missing walkers exist (DEFERRED-4). The alternative — descending them — is a **false certificate**.
   I chose safety. **This is the single biggest coverage cost in the plan.**
4. **P0 is a hard prerequisite of P2/P3.** Ship order **P0 → P2 → P3 → P1**. If P2/P3 land without P0, **the B3
   regression ships**.

---

# 8. UNVERIFIED (the honest list — full detail in plan §10)

| # | What |
|---|---|
| **U2** | ⭐ **The `ScanMode` threading in its SHIPPED form.** I measured the **SEMANTICS** of the split via a **thread-local stand-in** (identical divergence points, identical results, full suite green). **I did not write all ~30 explicit signatures.** |
| **U5** | ⭐ **The ~170 "read-free" `Keyword` arms.** Classified by *measured payload shape*, with the semantic surface delegated to the shipped authority. **The largest hand-classification surface in the plan and the most likely place for an implementer to introduce a false certificate.** |
| **U4** | The 33 "read-free" `ContinuousModification` arms (classified by measured payload type; not each executed). |
| **U1** | Scan (3)'s replacement zone-of-function authority — `ReplacementDefinition` has no measured zone field. |
| **U7** | `FORGE_TEST_FULL_DB=1 ordering_parity_sweep` delta — expected zero, **not run**. |
| **U8** | `DeclareShortcut { Fixed(N) }` ⇒ N tokens for the *activation* shape (N4 above). |
| **U6 / U9** | P6 on real Commander boards; perf of the static capture predicate. |
| ⭐ **U10** | **`batch_conflict == false`** — still a code trace. **MOOT for Rev 4 by construction** (see §4), **NOT discharged.** Becomes load-bearing again the instant a `Conservative` arm is touched. |
| **U11** | A **resolution-beat** capture variant — **not built, not raced.** The static predicate is measurably *sufficient* (canary offers; twin rejects), so nothing was left for it to fix. |
| **U12** | **M3's PRE-FIX 3-limb firewall list** — not re-derived. The probe worktree ships P2/P3 **already applied**, so I only saw the *post*-fix state. Rev 3's `{S1Trigger, S2BattlefieldBody, S4StaticModifications}` is **inherited from its prose, not re-measured by me.** Not load-bearing for any Rev-4 claim. |

**Carried forward honestly, NOT discharged (per team-lead):** **U1** (scan (3)'s replacement zone-of-function
authority — still unfound), **U6** (P6 on real Commander boards), **U9** (capture-gate perf), **U12** (M3 pre-fix
limbs). ⭐ **M6 (full-suite count) IS now discharged: `16550 passed; 0 failed`.**

### Probe-suite hygiene — the FAIL-BY-DESIGN tripwire was never tripped
I **never ran the whole probe suite.** Every run was a **targeted named test**: `probe_rev4`, `probe_canary_gond`, and
specific `--lib` tests. ⇒ **`probe_review_r1.rs::r2_min_by_key_refind_picks_the_wrong_permanent` (fails by design) was
never invoked, never "fixed", never deleted.** `probe_canary_gond.rs` is **intact** (291 lines, unmodified).
*(One note: `probe_canary_gond::m3_m4_covers_and_firewall_on_live_frames` now fails — **because the canary OFFERS**, so
its manual drive loop can no longer submit `ActivateAbility` while `WaitingFor::LoopShortcut`. **That failure IS the B1
success**, not a regression. It is a stale probe-harness expectation and NOT part of the change set.)*

---

# 9. WORKTREE STATE

```
$ git -C /home/lgray/vibe-coding/combo-probe-wt diff --stat
 crates/engine/src/analysis/resource.rs  | 305 ++++++++++++++++++++++++++-
 crates/engine/src/game/ability_scan.rs  | 169 ++++++++++++++-
 crates/engine/src/game/casting_costs.rs |   1 +
 crates/engine/src/game/engine.rs        | 215 +++++++++++++++----
 crates/engine/src/types/game_state.rs   |  19 ++
 crates/engine/tests/integration/main.rs |   3 +
 6 files changed, 672 insertions(+), 40 deletions(-)

untracked:
  crates/engine/tests/integration/probe_canary_gond.rs   ← INTACT (291 lines). NOT deleted, NOT modified.
  crates/engine/tests/integration/probe_review_r1.rs     ← the Rev-3 reviewer's. Untouched.
  crates/engine/tests/integration/probe_rev4.rs          ← NEW (mine): B1 + the negative twin.
```
**My additions to the reviewer's 409-insertion baseline:** the B1 capture/drive/normalize/offer prototype
(`engine.rs`, `game_state.rs`, `casting_costs.rs`), the **B3 `ScanMode` split** (`ability_scan.rs` + the firewall
wrapper in `resource.rs`), the **precise dynamic-P/T walker arms**, and the **U3 fixture** (`resource.rs`).
**The U3 revert-probe was applied, measured RED, and FULLY RESTORED** (verified: the `|| ...projected` term is back at
`resource.rs:1543`; the only remaining `REVERT-PROBE` strings are doc comments).

```
$ git -C /home/lgray/vibe-coding/phase-rs-workdir status --short -uno
 M client/src/wasm/engine_wasm.d.ts        ← PRE-EXISTING, not mine
```
## ✅ **ZERO `crates/` FILES IN THE MAIN CHECKOUT. No `cargo` was ever run there.**
