I have all the facts I need. Here is the plan.

---

# PLAN — Combo-detection PR-5: `cargo combo-verify` CLI over the corpus

## 1. Summary + skills followed

**Goal.** Turn the existing 53-row combo corpus (today locked inside a `#[cfg(test)]` module) into a runnable CLI, `cargo combo-verify`, that drives each combo through the **existing** detector (`analysis::loop_check::detect_loop` for offline combos, `live_mandatory_loop_winner` for the two live drain combos) and reports per row: **confirmed / gated / deferred / FAIL**, exiting non-zero only on a true FAIL. Zero new game logic; the engine owns all detection. The bulk is a *refactor-and-expose* of the corpus table + driver toolkit so both the test suite and the new bin call one shared implementation.

**Skills read and followed:**
- **`engine-planner`** (this output is its product): Step-1 skill scan, Step-2 analogous trace, Step-3 read-every-file-touched, Step-4 architectural sections, Step-5 file-by-file with verified CR handling.
- **`project-reference`** — tool-binary + cargo-alias conventions. The `[[bin]]` + `required-features` pattern and the `run --profile tool [--features X] --bin Y -- data/` alias shape are taken directly from existing entries (cited in §3).
- **`add-engine-effect`** — consulted; **its layered checklist does not apply** here. There is no new `Effect`, parser pattern, targeting, replacement, multiplayer filter, frontend overlay, or AI policy. The only overlapping steps are *tests* and *verification*, addressed in §4–§5.
- **`add-engine-variant`** — consulted; **does not apply.** No variant is added to any gated engine enum (`QuantityRef`, `FilterProp`, `Effect`, `TargetFilter`, …). The new enums (`RowStatus`, `RowReport`, `DeferralBucket`, `ComboDriver`) are *tooling* types in the analysis/CLI layer, absent from `data/engine-inventory.json`, so the variant gate is out of scope (I verified there is no sibling-cluster collision because these types do not exist yet).

**CR annotations:** No new rules-touching code is added. The CLI is a harness over already-CR-annotated detection (`detect_loop`, `classify_win_kind`, `live_mandatory_loop_winner`, `project_out_resources`). Moving driver code **preserves** its existing CR annotations verbatim (e.g. `CR 602.1`, `CR 608`, `CR 613`, `CR 119.3`, `CR 603.3`). I will not invent any CR number; none is required.

**Key design decision (resolved):** Extract the corpus rows **and** the board-install + `GameAction` driver toolkit out of `#[cfg(test)] mod corpus_tests` into a new module `crates/engine/src/analysis/corpus.rs`, gated `#[cfg(any(test, feature = "combo-verify"))]` and re-exported from `analysis/mod.rs` under the same cfg. The single test-only dependency (`card_db()` → `crate::test_support::shared_card_db()`, which is `#![cfg(test)]` at `test_support.rs:1`) is removed from the shared code by **parameterizing every driver on `db: &CardDatabase`**. The existing `#[test] fn` wrappers stay in `corpus_tests.rs`, become thin (call the shared driver, then assert), and the meta-tests/soundness tests stay test-only. This shares one implementation with **no logic duplication** and **no weakening** of the suite (under `cargo test` the `test` cfg compiles `corpus.rs`; under the bin the `combo-verify` feature does).

---

## 2. Trace of the closest analogous feature (file:line)

There are two analogues; I traced both end-to-end.

**(A) A feature-gated tool bin that consumes the public `engine` API** — `parser-gap-analyzer` / `oracle-gen`:
- Bin target & gating: `crates/engine/Cargo.toml:42-44` (`parser-gap-analyzer`, `test = false`) and `crates/engine/Cargo.toml:47-50` (`oracle-gen`, `required-features = ["cli"]`); the gating feature itself at `crates/engine/Cargo.toml:8` (`cli = ["tracing-subscriber"]`).
- Bin source pattern: `crates/engine/src/bin/parser_gap_analyzer.rs:1-201` — `use engine::...` (line 4, proving bins see only **`pub`** items of the lib), manual arg loop (`args.iter().skip(1)`, lines 17-41), `data/` root resolution with `PHASE_CARDS_PATH` fallback (43-49), `CardDatabase::from_export(&path.join("card-data.json"))` (72-83), JSON to stdout + human to stderr (152-200), `process::exit(1)` on error.
- Cargo alias: `.cargo/config.toml:12` `parser-gaps = "run --profile tool --bin parser-gap-analyzer -- data/"`; the **feature-carrying** variants `.cargo/config.toml:13` `rules-audit = "run --profile tool --bin rules-audit --features audit --"` and `.cargo/config.toml:14` `semantic-audit = "run --profile tool --features cli --bin oracle-gen -- semantic-audit"` (the exact `--features X --bin Y` shape I will mirror). `[profile.tool]` at `Cargo.toml:59-62` (`inherits = "dev"`, `opt-level = 1`).

**(B) The corpus harness itself** (what I am exposing) — `crates/engine/src/analysis/corpus_tests.rs`:
- Module decl: `analysis/mod.rs:40-41` (`#[cfg(test)] mod corpus_tests`).
- Data: `ComboRow` `corpus_tests.rs:71-87`; `ResourceFamily` `91-107`; `CORPUS` (53 rows) `112-510`; `ResourceFamily::expected_axis` `512-539`; `family_matches_axis` `1347-1378`; `assert_combo` `1332-1344`.
- Card DB seam (the test-only dependency to break): `card_db()` `710-712` → `test_support::shared_card_db()` (`test_support.rs:22`, file `#![cfg(test)]` at line 1).
- Driver toolkit: `install_on_battlefield` `718-749`; `ComboBoard` `761-764`; `build_board` `771-796`; `build_board_green` `806-817`; `build_board_with_vanilla` `824-858`; `float_single_color`/`float_mana`/`settle_layers`/`attach_aura` `864-919`; `ability_index_where` `923-934`; `activate_and_resolve` `939-956`; `resolve_to_priority` `961-1090`; `run_combo` (`WARMUP=2`, `STEADY=3`) `1096-1123`; predicates `1126-1164`; `seed_subtype_creatures` `1592-1618`.
- The 10 **offline** drivers (each calls `run_combo` → `detect_loop`): Heliod `1182-1268` (+ `drive_ballista_ping` `1273-1320`); Devoted/Vizier `1384-1399`; Grim/Power `1405-1421`; Spike/Archangel `1429-1452`; Bloom/Freed `1459-1475`; Faeburrow/Pemmin `1481-1504`; Selvala/Staff `1516-1543`; Kilo/Freed/Relic `1551-1586`; Priest/Umbral `1631-1654`; Marwyn/Sword `1690-1722`.
- The 2 **live** drivers (per-beat `apply(PassPriority)` → `GameOver` via the reconcile shortcut): `build_drain_board` `1981-2005`; `seed_lifegain_cascade` `2012-2027`; `drive_pass_priority` `2042-2071`; `first_gameover_beat` `2075-2082`; idx18 `2097-2138`; idx17 `2155-2182`.
- Driven-set lock: `DRIVEN_ROW_INDICES` (the 12 confirmable rows `0,1,4,6,9,10,12,13,14,17,18,49`) `1940-1953`; meta-test `confirmed_drivers_match_expected` `1957-1967`; shape lock `543-571`; availability test `604-640`; the "Remaining corpus rows" structural-bucket prose (object-re-entry / extra-turn-combat / color-converting / drain-feedback-live / card-gated) `1878-1933`.
- Detector entry points reused unchanged: `detect_loop` `analysis/loop_check.rs:152`; `LoopCertificate` `107`; `WinKind` `74`; `live_mandatory_loop_winner` (`pub(crate)`) `212`; `LoopCertificate::covers` `128`. `ResourceAxis` `analysis/resource.rs:535`.

---

## 3. File-by-file changes

### 3a. NEW `crates/engine/src/analysis/corpus.rs` (shared corpus + driver API)
Gate the whole module `#[cfg(any(test, feature = "combo-verify"))]`. **Move** (do not copy) from `corpus_tests.rs` and make `pub`:
- The data layer: `ComboRow` (with `pub` accessor methods `name()/cards()/family()/win_kind()/gated_on()` — keep fields private, expose read-only getters), `ResourceFamily`, `CORPUS`, `ResourceFamily::expected_axis`, and `family_matches_axis` (rename `pub fn family_matches_axis(family, axis) -> bool`).
- The driver toolkit (all the helpers listed in trace B), each that previously called `card_db()` now takes `db: &CardDatabase`: `install_on_battlefield`, `build_board`, `build_board_green`, `build_board_with_vanilla`, `float_*`, `settle_layers`, `attach_aura`, `ability_index_where`, `activate_and_resolve`, `resolve_to_priority`, `run_combo`, `seed_subtype_creatures`, predicates, `ComboBoard` (struct stays private; only the driver fns need its fields). `build_board`'s `.expect("corpus card must be present…")` becomes a graceful `Option` return (so a missing card on a custom export is reported, not a panic).
- The 10 offline driver **bodies** refactored into `fn drive_offline_<name>(db: &CardDatabase) -> Option<LoopCertificate>` (private to the module; each is the existing body minus the `.expect()`/`assert_combo`, returning the `run_combo` result). The Heliod body returns its `detect_loop(...)` `Option` likewise.
- The 2 live driver bodies refactored into `fn drive_live_drain(db, idx) -> Option<(usize /*beat*/, PlayerId /*winner*/)>` (idx17/idx18 share one body — they differ only by `CORPUS[idx].cards`, matching the existing code which is identical apart from the targeted-vs-untargeted note).

**Net-new (the only real new code in this file):**
```rust
/// Per-row classification result (tooling type — not an engine enum).
pub enum RowStatus {
    /// Driven through the existing detector; cert/family/win_kind match the row.
    Confirmed { unbounded: Vec<ResourceAxis>, win_kind: WinKind },
    /// Driver ran but produced no/mismatched confirmation — a regression.
    Failed { detail: String },
    /// Card-gated on an unimplemented card (the 4 §3 rows).
    Gated { card: &'static str },
    /// Testable but no driver yet (measured structural bucket).
    Deferred { bucket: DeferralBucket },
}
pub enum DeferralBucket { ObjectReentry, ExtraTurnOrCombat, ColorConverting, /* … */ }

/// Confirmation mechanism for a driven row.
enum ComboDriver {
    Offline(fn(&CardDatabase) -> Option<LoopCertificate>),
    LiveDrain, // dispatches to drive_live_drain(db, idx)
}
/// Static map idx -> driver, replacing the hand-listed DRIVEN_ROW_INDICES.
const DRIVERS: &[(usize, ComboDriver)] = &[ /* the 12 rows */ ];

pub struct RowReport { pub name: &'static str, pub expected_family: ResourceFamily,
                       pub expected_win_kind: WinKind, pub status: RowStatus }

/// THE shared entry point both the tests and the CLI call.
pub fn drive_row(db: &CardDatabase, idx: usize) -> RowReport { /* dispatch */ }
pub fn corpus_len() -> usize { CORPUS.len() }
pub fn row(idx: usize) -> &'static ComboRow { &CORPUS[idx] }
```
`drive_row` logic (pure dispatch, no game logic): if `row.gated_on.is_some()` → `Gated`. Else look up `idx` in `DRIVERS`: `Offline(f)` → run `f(db)`; if `Some(cert)` and `cert.win_kind == row.win_kind` and `cert.unbounded.iter().any(|a| family_matches_axis(row.family, a))` → `Confirmed`, else `Failed`. `LiveDrain` → `drive_live_drain(db, idx)`; `Some((_, winner))` with `winner == controller (P0)` and `row.win_kind == LethalDamage` → `Confirmed{ win_kind: LethalDamage, unbounded: vec![ResourceAxis::Life(victim)] }`, else `Failed`. Not in `DRIVERS` and not gated → `Deferred{ bucket: classify_bucket(idx) }`, where `classify_bucket` is a small `match idx { … }` over the documented structural buckets (the prose at `corpus_tests.rs:1878-1933` becomes typed data — declarative, not game logic).

### 3b. `crates/engine/src/analysis/mod.rs`
- Add `#[cfg(any(test, feature = "combo-verify"))] pub mod corpus;` next to the existing `pub mod` lines (`mod.rs:35-38`).
- Add a re-export under the same cfg: `pub use corpus::{drive_row, RowReport, RowStatus, DeferralBucket, ResourceFamily, corpus_len, row};`.
- Leave `#[cfg(test)] mod corpus_tests;` (`mod.rs:40-41`) and all existing re-exports (`43-49`) untouched.

### 3c. `crates/engine/src/analysis/corpus_tests.rs` (slim down; keep test-only)
- Delete the moved items; add `use crate::analysis::corpus::{self, ComboRow_getters…, family_matches_axis, run_combo, build_board, …};` (only what the remaining tests need).
- `card_db()` (`710-712`) **stays** (test-only). Each existing `#[test] fn drive_combo_*` becomes: `let cert = corpus::drive_offline_<name>(card_db()).expect("…"); assert_combo(idx, &cert);` — i.e. the asserts are preserved, the body is shared. (Expose the per-combo `drive_offline_*` fns as `pub(crate)` so the tests can call them while the CLI calls them via `drive_row`.)
- Replace `DRIVEN_ROW_INDICES` (`1940-1953`) usage: `confirmed_drivers_match_expected` now iterates `corpus::DRIVERS` indices (kept in sync by construction). Keep `corpus_table_shape_is_locked`, `corpus_cards_present_and_implementation_status_matches_gating`, and all soundness/regression tests (synthetic pinger tests `1766-1876`; live regression tests `2208-2414`) exactly as-is — they continue to exercise the live reducer and the offline gates.
- **Add one new test** locking the CLI classifier (see §4).

### 3d. `crates/engine/Cargo.toml`
- `[features]` (after line 11): add `combo-verify = []`.
- New `[[bin]]` mirroring `oracle-gen` (`47-50`):
```toml
[[bin]]
name = "combo-verify"
path = "src/bin/combo_verify.rs"
required-features = ["combo-verify"]
test = false
```

### 3e. NEW `crates/engine/src/bin/combo_verify.rs` (the thin harness)
Mirror `parser_gap_analyzer.rs` exactly: manual arg loop; resolve `data/` root (positional, else `PHASE_CARDS_PATH`); `let db = CardDatabase::from_export(&root.join("card-data.json"))` with a clear error + `process::exit(2)` if absent (distinct from FAIL). Then:
```rust
let mut fails = 0;
let mut counts = [confirmed, gated, deferred, failed counters];
for idx in 0..engine::analysis::corpus_len() {
    let r = engine::analysis::drive_row(&db, idx);
    // print one human table line: status glyph, name, expected family/win_kind, actual axes/win_kind
    if matches!(r.status, RowStatus::Failed{..}) { fails += 1; }
    // tally
}
// print summary line: "N confirmed / G gated / D deferred / F failed (of 53)"
if json_flag { println!("{}", serde_json::to_string_pretty(&summary)?); }
process::exit(if fails > 0 { 1 } else { 0 });
```
Human table to stdout, machine-readable JSON to stdout under `--json` (matching repo convention where JSON is the stdout artifact and humans read the table/stderr). Exit `0` = no FAIL (gated + deferred are expected, never failures); `1` = ≥1 driven-row regression; `2` = export missing/load error. The bin contains **no** game logic — it only calls `drive_row` and formats.

### 3f. `.cargo/config.toml`
Add one alias mirroring `semantic-audit`/`rules-audit` (the proven `--features X --bin Y` shape):
```toml
combo-verify = "run --profile tool --features combo-verify --bin combo-verify -- data/"
```

**Out of scope (noted, not built):** FEASIBILITY §7's secondary "input a card set / GameState → Engine B (`candidate_cycles`) → A" mode. The task is corpus-driving; the Engine-B card-list scan is a clean future extension (the `ability_graph::candidate_cycles` re-export already exists at `analysis/mod.rs:43`).

---

## 4. Discriminating, non-vacuous tests + revert-probe

The detector and every driver are **already** revert-probed in their own modules (`loop_check.rs` soundness tests `513-979`; `resource.rs` `844-1207`; `sim.rs` `257-826`; the per-combo revert-probes `drive_combo_10_..._requires_untap` `1660-1677` and `..._14_..._requires_untap` `1727-1751`; the live `drive_drain_idx18_victim_with_out_is_not_eliminated` `2304-2316` and `drive_finite_stack_keeps_ring_empty` `2330-2414`). The refactor preserves all of these by construction (they call the shared drivers). The **new** surface to test is exactly the CLI classifier `drive_row`.

**New test `corpus::drive_row` classifier (in `corpus_tests.rs`, test-only, runs on `card_db()`):**
1. **Confirmed (offline), non-vacuous:** `drive_row(card_db(), 6)` (Devoted Druid + Vizier) → `RowStatus::Confirmed` whose `unbounded` contains a `Mana(_)` axis and `win_kind == Advantage`. This is the same assertion the existing test makes, now routed through `drive_row` — proving the dispatch reaches the real driver and the real `detect_loop`.
2. **Confirmed (live):** `drive_row(card_db(), 18)` → `Confirmed { win_kind: LethalDamage, .. }` (drain shortcut), and `drive_row(card_db(), 17)` likewise. Proves the live branch of the dispatch confirms via `live_mandatory_loop_winner`/the reconcile shortcut.
3. **Gated:** `drive_row(card_db(), 2)` (Doc Aurlock, `gated_on = Some(..)`) → `RowStatus::Gated { card: "Doc Aurlock, Grizzled Genius" }`, **never** `Failed` — and assert the same for idx 19/36/49-gated rows. Discriminator: this is the "gated is expected, not a failure" contract.
4. **Deferred:** pick a known non-driven testable row (e.g. Kiki-Jiki + Zealous Conscripts, an object-re-entry row) → `RowStatus::Deferred { bucket: ObjectReentry }`, never `Failed`.
5. **REVERT-PROBE (the discriminating core):** a `classify_status(row, outcome)` helper unit test fed a *fabricated* `LoopCertificate` whose `win_kind` is deliberately **wrong** for the row (e.g. an offline mana row handed a `LethalDamage` cert, or a damage row handed an `Advantage` cert) MUST return `RowStatus::Failed`. Proof of non-vacuity: with the comparison reverted (drop the `cert.win_kind == row.win_kind && family_matches_axis(..)` check, i.e. classify any `Some(cert)` as `Confirmed`), this assertion flips from `Failed` to `Confirmed` and the test fails. This pins that the CLI actually *compares against the spec* rather than rubber-stamping any cert.
6. **FAIL on missing confirmation:** feed `classify_status` an offline driver result of `None` → `Failed`. Revert (treat `None` as `Confirmed`) flips it.

**Driven-set lock (kept honest):** `confirmed_drivers_match_expected` is rewritten to iterate `corpus::DRIVERS`; it still asserts every driven idx is in range and non-gated, so adding a driver for a gated row is caught.

Each new assertion is non-vacuous because (a) it runs the real driver against real card data (cases 1–4), and (b) the revert-probe (case 5/6) is constructed so the *current* code passes and the *reverted* code fails — demonstrated by the explicit reverted predicate.

---

## 5. Verification + regression plan

Per CLAUDE.md risk-scaling and the Tilt-first rule (do **not** run cargo build/clippy/test directly; `cargo fmt --all` is the one exception):
1. `cargo fmt --all` (always direct).
2. **Compile + lint of the moved/feature-gated code:** read Tilt `clippy` and `test-engine` logs (`tilt logs clippy --tail 60 --since 2m`; `./scripts/tilt-wait.sh clippy test-engine`). The moved `corpus.rs` is linted under the **test** cfg (Tilt's `clippy --all-targets` compiles the test target), and the slimmed `corpus_tests.rs` runs under `test-engine`. This proves the refactor preserves the suite (same tests, same asserts, now via the shared driver).
3. **The combo-verify *bin* is feature-gated**, so Tilt's default `clippy`/`test-engine` (no `--features combo-verify`) will **skip** it — exactly as they skip `oracle-gen`/`rules-audit`. To cover it, run two targeted commands directly (they don't fight Tilt's default target because of the distinct feature set, but to be safe run them only after Tilt is idle): `cargo clippy -p engine --bin combo-verify --features combo-verify -- -D warnings` and the end-to-end smoke `cargo combo-verify` (the alias) against the real `data/card-data.json`.
4. **Acceptance smoke (the deliverable working):** `cargo combo-verify` over the full export must print 12 **confirmed** (the `DRIVERS` set), 4 **gated**, the remaining ~37 **deferred**, **0 FAILED**, and exit `0`. A non-zero exit or any FAIL row is a real regression in the detector/drivers.
5. **Regression guard for the rest of the workspace:** because `corpus.rs` is `#[cfg(any(test, feature))]` and the default lib/WASM build excludes it, there is no change to the shipped engine/WASM surface — confirm `wasm` and `server` Tilt resources stay green (they should be unaffected; this is the soundness of feature-gating). No card-data/coverage impact (no parser change), so `card-data` coverage is untouched.
6. Before marking done: `./scripts/tilt-wait.sh clippy test-engine wasm` green, plus the two feature-gated direct commands in step 3, plus the step-4 smoke output captured.

**Discrimination evidence to attach in the PR:** (a) the reverted-predicate run of new test #5 failing; (b) the `cargo combo-verify` summary line (12/4/37/0); (c) `test-engine` green proving the moved tests still pass through the shared driver.

---

## 6. Risks & open questions

1. **Spec's 3-bucket framing vs measured 12-driven reality.** The task says "confirmed / gated / FAIL," but only **12 of 49** non-gated rows have drivers today (`DRIVEN_ROW_INDICES` `corpus_tests.rs:1940-1953`); the other 37 are documented as not-yet-drivable on the current loop model (object-re-entry, extra-turn/combat, color-converting — `1878-1933`). Reporting those 37 as FAIL would make the tool red and wrong. **Resolution:** add a fourth status `Deferred` (expected, exit-0), with the structural bucket typed from the existing prose. FAIL is reserved for a *driven* row that regresses. This is the honest, evidence-based reading; flag for reviewer sign-off that "deferred ≠ fail" is acceptable.
2. **idx 17/18 are confirmed via the LIVE path, not `detect_loop`.** They produce a `GameOver` winner through `live_mandatory_loop_winner` + the reconcile shortcut, not a `LoopCertificate`. The plan unifies both under `RowStatus::Confirmed` via the `ComboDriver::{Offline,LiveDrain}` split. If the reviewer prefers the CLI to be strictly `detect_loop`-only, the fallback is to mark 17/18 `Deferred{DrainFeedbackLiveOnly}` and confirm 10 — fewer moves, but undercounts the corpus. I recommend including the live drivers (they already exist; extraction is mechanical).
3. **Feature-gate vs always-compiled.** I chose `#[cfg(any(test, feature = "combo-verify"))]` to mirror the repo's `cli`/`audit` tool-bin convention and keep test-scenario-driving code out of the shipped lib/WASM. Trade-off: Tilt's default `clippy`/`test-engine` won't lint the *bin* (consistent with `oracle-gen`/`rules-audit`); covered by the explicit `--features combo-verify` commands in §5.3. Alternative: always-compiled `pub mod corpus` (simpler cfg, fully Tilt-linted, DCE-stripped from WASM) — viable if the reviewer prefers no new feature flag. Open question for reviewer.
4. **Export availability in CI.** `data/card-data.json` is the 97 MB gitignored export (present locally; generated in the release/CI pipeline before coverage steps). The CLI errors with exit `2` if it's absent (it is a maintainer tool, like `coverage`). It is **not** wired into the default Tilt loop, so a fresh checkout never fails on its absence. The fixture-based `corpus_tests.rs` tests already skip-or-fixture gracefully and are unaffected.
5. **Index fragility.** Drivers reference rows by absolute `CORPUS[idx]` (e.g. `CORPUS[6]` is Devoted/Vizier). This coupling pre-exists; `corpus_table_shape_is_locked` (`543-571`) + the rewritten `confirmed_drivers_match_expected` guard against accidental row reordering. No change to this risk; just inherited.
6. **`ComboRow` encapsulation.** Exposing the rows publicly: I keep fields private with getters so the public surface is read-only and stable (the CLI only reads). Minor: `gated_on`/`cards` are `&'static str`, so getters are trivial.

---
# BINDING AMENDMENTS (round-1 adversarial review — these OVERRIDE the plan above on any conflict)

1. **[HIGH] Correct gated CORPUS array indices = 2, 21, 38, 51** (Doc Aurlock=2, Professor Onyx=21, Animate Dead=38, Grindstone=51). The plan's 19/36/49 are WRONG — they are the ComboRow doc-comment's FEASIBILITY §12 numbering, NOT the array index. **idx 49 is a CONFIRMED DRIVER** (in DRIVEN_ROW_INDICES = [0,1,4,6,9,10,12,13,14,17,18,49]) — asserting Gated{49} would contradict the driver set and fail. Use 2/21/38/51 in the Gated test; add a comment warning that doc-comment numbering ≠ array index.

2. **[HIGH] DeferralBucket must be DECLARATIVE typed data, not `match idx`.** Add a field to `ComboRow` (e.g. `deferral: Option<DeferralBucket>`) populated for ALL 53 rows, mutually exclusive with `gated_on` and the driven set. Fully enumerate `DeferralBucket` (remove the `/* … */`); add an `Unclassified`/`Other` catch-all if any deferred row doesn't map to a named structural bucket. Add a **partition shape-lock test**: driven ∪ gated ∪ deferred = all 53 rows, pairwise disjoint. The `match idx` over 37 absolute indices is the per-card special-casing the repo forbids — do not write it.

3. **[MED] Visibility:** `ComboBoard`, `BeatTrace` (corpus_tests.rs:2031), and EVERY moved helper the retained live-regression tests (corpus_tests.rs:2208-2414) still call — `build_drain_board`, `seed_lifegain_cascade`, `drive_pass_priority`, `first_gameover_beat`, plus `board.runner`/`board.ids` access and `BeatTrace` fields (`wf`/`ring_len`/`stack_len`/`p0_life`/`p1_life`) — must be `pub(crate)` with `pub(crate)` fields. **Add `BeatTrace` to the move list** (the plan omitted it).

4. **[MED] JSON output:** do NOT add `#[derive(Serialize)]` to `ResourceAxis` (resource.rs:534) or `WinKind` (loop_check.rs:73) — they live in always-compiled modules (real engine/WASM surface). Instead build a plain **string DTO** in the bin (axis/win_kind rendered as strings) for `--json`. The engine enums need no serde.

5. **Keep (reviewer-confirmed correct):** feature-gate `#[cfg(any(test, feature="combo-verify"))]`; `[[bin]]`+`required-features`+alias mirroring oracle-gen/rules-audit; parameterize every driver on `db: &CardDatabase` (breaks the test-only `card_db()`/`test_support` dependency); zero game logic in the bin (engine owns detection); `drive_row` dispatch (Offline vs LiveDrain); the new `classify_status` revert-probe tests (fabricated wrong-win_kind cert → Failed; None → Failed).

6. **Acceptance smoke:** `cargo combo-verify` over `data/` prints **12 confirmed / 4 gated / 37 deferred / 0 failed**, exit 0. Any FAIL row = a real detector/driver regression. Capture this output as ship evidence.
