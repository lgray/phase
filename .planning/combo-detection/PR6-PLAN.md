# PR-6 — `∞` Unbounded-Resource Display: Implementation Plan (FINAL, r3)

**Series:** combo-detector PR-6 · Predecessor **PR-5 — #4547 — https://github.com/phase-rs/phase/pull/4547** · **Worktree:** `/home/lgray/vibe-coding/wt-combo-pr6` (`feat/combo-detect-pr6`, base `upstream/main 6d8c30cd7`) · cargo runs directly (own target dir, no Tilt).

> Plan-review history: draft → review r1 (2 BLOCKER + 1 HIGH + 2 MED + 2 LOW, all resolved) → r2 → review r2 (all r1 fixes confirmed correct & discriminating against source, no regression; **1 new BLOCKER**: `BTreeSet<ResourceAxis>` needs `Ord`, which `ResourceAxis`/`ManaType` lack) → **r3 (this doc)** resolves that BLOCKER + 2 LOWs. All compile-critical facts re-verified against worktree source (Ord chain complete after the two derive additions; `ManaType` has no manual Ord impl; the inventory generator records variant names/docs/CR, NOT derives → byte-identical).

## 1. Objective
Generalize the single-purpose `debug_infinite_mana: BTreeSet<PlayerId>` mechanism into an engine-authoritative **unbounded-resource** model covering the whole `ResourceAxis` class (mana, life-drain, damage, mill, poison, counters, tokens, draws, casts, triggers, combats, extra turns), surface it across the serialization boundary via `DerivedViews`, and render `∞` badges on the HUD — with **zero gameplay change** (the existing infinite-mana pool top-up + end-of-step keep behavior is byte-preserved). Data source is the detector's `LoopCertificate.unbounded` (`Vec<ResourceAxis>`).

## 2. Pattern Coverage (design for the class)
- Storage/authority covers all 16 `ResourceAxis` variants uniformly — not a mana special case. The debug toggle is one producer (the six mana axes); the certificate path (`mark_unbounded_loop(controller, &cert.unbounded)`) is the general producer PR-7 reuses for every win/advantage family the corpus already proves.
- Display covers every axis via one exhaustive axis→family map; no per-card/per-mechanic branch.

## 3. Analogous Trace (end-to-end) — mirrors `player_status`
`types/game_state.rs` (authoritative state) → `game/static_abilities.rs` authorities → `game/derived_views.rs::derive_views` builds `Vec<PlayerStatusView>` → serialized in `ClientGameStateRef { state, derived }` via `engine-wasm/src/lib.rs::to_js` (:63) at `get_game_state`/`get_filtered_game_state` (:966/:985) → TS mirror `DerivedViews.player_status` → `usePlayerDesignations::forPlayer` filters by `player` → `PlayerHud.tsx`/`OpponentHud.tsx` render `ConditionBadge` (exhaustive `CONDITION_GLYPH`). PR-6 substitutes `unbounded_resources`/`UnboundedResourceView` along the identical seam.

## 4. Architecture & Authority Decisions

### 4.1 REPLACE (not parallel) — lead #2
Rename `debug_infinite_mana: BTreeSet<PlayerId>` → `unbounded_resources: BTreeMap<PlayerId /*controller*/, BTreeSet<ResourceAxis>>`. Justification: a parallel field is a sibling-cluster smell (mana-only twin); CLAUDE.md "parameterize, don't proliferate."

**Blast radius (complete; reviewer-confirmed NO other readers — `filter_state_for_viewer` is clone-based → carries the field automatically):**
- `types/game_state.rs`: field+doc (:6118, with §4.9 exclusion doc comment); init (:7784); manual `impl PartialEq for GameState` (:8056) — `unbounded_resources` EXCLUDED (§4.9); `normalize_for_loop` (:7962) EXCLUDED; `loop_fingerprint` (:7920) EXCLUDED; new methods `mark_unbounded_loop`/`clear_unbounded_loop`.
- `types/mana.rs`: **`ManaType` (:46) += `PartialOrd, Ord`** (required so `ResourceAxis: Ord` for `BTreeSet`; see §4.2). Additive derive only.
- `analysis/resource.rs`: serde + Ord derives (§4.2).
- `game/mana_payment.rs`: refill (:61/:76/:79) + tests (:3980,:4022,:4041,:4063).
- `game/turns.rs`: keep gate (:249).
- `game/engine_debug.rs`: toggle (:406,:410).
- `game/derived_views.rs`: NEW projection (`UnboundedResourceView`, `attribution_player`, `derive_views` push loop).
- `analysis/corpus.rs:842`: doc-comment prose update (cosmetic).

### 4.2 `ResourceAxis` storage requires `Ord`; keep types in place, add serde + Ord — open Q4 + r2 BLOCKER
The field stores `BTreeSet<ResourceAxis>`, which requires `ResourceAxis: Ord`. Today (VERIFIED):
- `ResourceAxis` (resource.rs:534) derives `Debug, Clone, Copy, PartialEq, Eq` — **no `Ord`**.
- Its payload `ManaType` (mana.rs:46) derives `…, Hash, Serialize, Deserialize` — **no `Ord`** (no manual impl).
- The other payloads already have `Ord`: `PlayerId` (player.rs:84), `CounterClass` (resource.rs:77), `ObjectClass` (resource.rs:53), `TriggerKind` (resource.rs:117).

**Decision:** add `PartialOrd, Ord` to **`ManaType`** (mana.rs:46) and to **`ResourceAxis`** (resource.rs:534); add `Serialize, Deserialize` to `ResourceAxis`(:534) + `ObjectClass`(:53) + `CounterClass`(:77) + `TriggerKind`(:117). `game_state.rs` references `crate::analysis::resource::ResourceAxis`. Do NOT relocate any type.

**Inventory byte-identical (lead #5) — re-verified, holds despite the `types/mana.rs` edit:** the generator (`engine-inventory-gen/src/main.rs`) scans only `crates/engine/src/types/` (`TARGET_DIR` :77) and records **variant names + doc comments + CR annotations** — it does **NOT** read or record `#[derive(...)]` attributes (its only `#[derive]` occurrences are on its own output structs, lines 24–71). Therefore adding `PartialOrd, Ord` to `ManaType`'s existing derive line changes no variant/doc/CR and leaves `data/engine-inventory.json` BYTE-IDENTICAL. Both derive edits land on an existing attribute line (no line-number shift). `WinKind` gets no serde (not on the wire). The `types→analysis` edge (game_state.rs → `crate::analysis::resource::ResourceAxis`) is legal Rust (`pub mod analysis` unconditional).

### 4.3 Projection vs raw field
Raw `unbounded_resources` field is authoritative state and still serializes; frontend reads ONLY the `DerivedViews` projection. The TS `GameState` mirror omits the raw field (as it omits `debug_infinite_mana` today).
```rust
DerivedViews.unbounded_resources: Vec<UnboundedResourceView>  // #[serde(default, skip_serializing_if = "Vec::is_empty")]
struct UnboundedResourceView { pub player: PlayerId, pub axis: ResourceAxis }
```
`player` = engine attribution (the HUD the badge attaches to); `axis` = engine-provided identity the FE formats to a label. Reusing `ResourceAxis` costs zero new vocabulary. FE is forbidden+tested from reading any inner pid for placement.

**Serde rename caveat (LOW-7) — intentional:** `debug_infinite_mana` had `#[serde(default)]` (always serialized). `unbounded_resources` gets `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]` (omit-when-empty + new key) — minimal wire for the dominant empty case; `serde(default)` covers missing keys. Old persisted snapshots keyed `debug_infinite_mana` silently drop an active toggle across the rename — acceptable (debug toggle). Keeping the raw field serialized (NOT `serde(skip)`) is correct: the refill/keep gates read it, so it must survive reconnect/save-resume. The derived projection is recomputed by pure `derive_views` and never deserialized back → no raw-vs-derived consistency hazard.

### 4.4 Attribution rule (engine-decided, exhaustive) — lead #3
`fn attribution_player(axis: ResourceAxis, controller: PlayerId) -> PlayerId` in `game/derived_views.rs`, exhaustive (no wildcard):
- `Life(p) | DamageDealt(p) | LibraryDelta(p) => p` — axis names the player it acts on → that HUD; routes opponent damage/drain/mill to the victim, controller self-mill/lifegain to the controller (keyed off the payload PlayerId, NOT permanent control).
- all aggregate axes (`Mana`/`Counter`/`Trigger`/`TokensCreated`/`CardsDrawn`/`Casts`/`LandfallTriggers`/`CombatPhases`/`ExtraTurns`/`DeathTriggers`/`EtbTriggers`/`LtbTriggers`/`SacTriggers`) => controller.

**Poison rules-honesty (MED-4) — REQUIRED CODE COMMENT at `attribution_player`:**
```rust
// CR 704.5c: a player with ten or more poison counters loses the game — so the
// *afflicted* player owns the win condition, and a poison ∞ belongs on the VICTIM's
// HUD. But `Counter(Poison, ObjectClass::Player)` is AGGREGATE-keyed in ResourceVector
// (no victim PlayerId; loop_check.rs:239-246 reads the summed (Poison, Player) pair),
// so it falls into the aggregate `=> controller` arm and is controller-attributed here.
// This is correct ONLY because no live producer emits a poison axis in PR-6 (the mana
// toggle is the sole producer). PR-7 MUST NOT wire a live poison loop until the analysis
// poison axis is re-keyed by victim PlayerId, or ∞ would render on the wrong HUD.
```
Plan-level corollary: PR-7's poison enablement is gated on re-keying `ResourceVector`'s poison axis by victim `PlayerId` first.

### 4.5 WinKind ride-along — OUT OF SCOPE
Bare `∞ <resource>` badge. No `WinKind` on the wire → no serde on `WinKind`.

### 4.6 Display scope — per-axis-family HUD badges (+ optional mana-pool ∞ marker)
One `∞` badge per distinct display-family per player. Engine emits faithful per-axis rows (6 `Mana(color)` rows for the mana toggle); FE maps each `ResourceAxis` to a family and renders one badge per distinct family (presentation de-dup via `new Set`). Families: mana, life, damage, mill, counters, tokens, cards, casts, combats, turns, triggers. Plus a small `∞` marker on `ManaPoolSummary` when the mana family is present.

### 4.7 `mark_unbounded_loop` = single reusable write authority — lead #4
```rust
pub fn mark_unbounded_loop(&mut self, controller: PlayerId, axes: &[ResourceAxis])  // sole write path; idempotent set-union
pub fn clear_unbounded_loop(&mut self, controller: PlayerId)
```
Signature `&[ResourceAxis]` not `&LoopCertificate`: stores exactly what it is given; the mana toggle has no certificate; PR-7 passes `&cert.unbounded`. The toggle delegates, never inlines. **Whole-player clear (LOW-6) — intentional for PR-6:** with the mana toggle as the only producer this equals today's all-or-nothing disable; PR-7 may add an axis-scoped clear when multiple producers coexist on one controller.

### 4.8 Mana byte-preservation — exact mapping — lead #1
- Enable: `state.mark_unbounded_loop(player_id, &INFINITE_MANA_AXES)` (new const `[Mana(W),Mana(U),Mana(B),Mana(R),Mana(G),Mana(C)]` in `mana_payment.rs`, parallel to `INFINITE_MANA_TYPES`), then `refill_infinite_mana(state)` exactly as today.
- Disable: `state.clear_unbounded_loop(player_id)`.
- Refill gate (`mana_payment.rs:76`): flagged = players whose `unbounded_resources` entry contains ANY `Mana(_)` axis; per-player top-up still iterates all 6 `INFINITE_MANA_TYPES` to `PER_TYPE` — byte-for-byte unchanged.
- Keep gate (`turns.rs:249`): `unbounded_resources.get(&pid).is_some_and(|axes| axes.iter().any(|a| matches!(a, ResourceAxis::Mana(_))))`; CR 500.5 comment preserved.
- Choice (6 axes vs sentinel): insert all 6 — stored set faithfully says "all 6 colors unbounded"; gate decoupled as "any `Mana(_)` → top up all 6" → identical regardless of stored colors; robust if PR-7 ever records a single-color loop.

### 4.9 Loop-detection equality exclusion (B2) — load-bearing
**Decision:** `unbounded_resources` is EXCLUDED from `impl PartialEq for GameState` (:8056), `normalize_for_loop` (:7962), and `loop_fingerprint` (:7920) — mirroring the (today silent) treatment of `debug_infinite_mana`, same family as the documented exclusions `static_gate_truth` (:8083) and `devour_eligible_snapshot` (:8205).
**Why mandatory:** `unbounded_resources` is display/annotation state, not equality state. `loop_states_equal` (:8007) reuses this `PartialEq`; `loop_states_equal_modulo_resources` → `project_out_resources` (resource.rs:641) → `normalize_for_loop` → `loop_states_equal` routes through the SAME `PartialEq` (verified: `project_out_resources` does not compare fields independently). The PR-3 ring (`record_loop_detect_sample` → `normalize_for_loop`) compares via the same equality. So the single `PartialEq` exclusion transitively covers PR-0/PR-2 modulo and PR-3 ring paths. Including the field would make a populated live state differ from the empty-`unbounded_resources` ring snapshots → `loop_states_equal == false` → PR-2/PR-3 false negatives (CR 104.4b/732.2a regression) + AI-search dedup break. The manual `eq` has NO `..` fallthrough, so the rename will NOT compile-error if the field is forgotten or wrongly added — the guards are this decision + the field doc comment + guard test 8.
**Field doc comment (REQUIRED):**
```rust
/// Per-controller set of resource axes a detected/forced unbounded loop pumps,
/// the engine-authoritative source for the `∞` HUD projection (`derive_views`)
/// and the byte-preserved infinite-mana refill/keep gates.
///
/// INTENTIONALLY EXCLUDED from `PartialEq`, `normalize_for_loop`, and
/// `loop_fingerprint` (same family as `static_gate_truth` /
/// `devour_eligible_snapshot`): this is display/annotation state, not rules
/// state for equality. CR 104.4b/CR 732.2a loop detection (`loop_states_equal`)
/// and AI-search position dedup compare two states reached at different times;
/// a populated live state must still compare equal to the empty-`unbounded_resources`
/// ring snapshots, or loop detection yields false negatives. (`debug_infinite_mana`
/// relied on this same exclusion implicitly; it is now explicit.)
#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
pub unbounded_resources: BTreeMap<PlayerId, BTreeSet<ResourceAxis>>,
```

## 5. Logic Placement
- Authoritative state + write authority + equality exclusion: `types/game_state.rs`.
- Mana behavior (byte-preserved): `mana_payment.rs` refill, `turns.rs` keep.
- Producer adapter: `engine_debug.rs` toggle delegates to the write authority.
- Projection (engine-decided attribution): `game/derived_views.rs`.
- Display only: frontend renders the projection; `axisTag`/family mapping is presentation formatting of engine-provided identities.

## 6. Building Blocks
- Reused as-is: `LoopCertificate`/`.unbounded`/`detect_loop`; `refill_infinite_mana`/`INFINITE_MANA_TYPES`/`PER_TYPE`; `DerivedViews`/`derive_views`/`ClientGameStateRef`/`to_js`; corpus `pinger_scenario`(:553)/`drive_one_ping`(:577)/`LoopProbe`; FE `usePlayerDesignations::forPlayer`, HUD badge components, `ManaPoolSummary`.
- New helpers: `attribution_player`; `INFINITE_MANA_AXES`; `mark_unbounded_loop`/`clear_unbounded_loop`; `UnboundedResourceView`.
- FE tag-extraction helper (MED-5):
```ts
type ResourceAxisTag = 'Mana' | 'Life' | 'DamageDealt' | 'LibraryDelta' | 'Counter'
  | 'Trigger' | 'TokensCreated' | 'CardsDrawn' | 'Casts' | 'LandfallTriggers'
  | 'CombatPhases' | 'ExtraTurns' | 'DeathTriggers' | 'EtbTriggers' | 'LtbTriggers' | 'SacTriggers';
const axisTag = (axis: ResourceAxis): ResourceAxisTag =>
  typeof axis === 'string' ? axis : (Object.keys(axis)[0] as ResourceAxisTag);
const UNBOUNDED_FAMILY: Record<ResourceAxisTag, FamilyKey> = { /* exhaustive */ };
const familyOf = (axis: ResourceAxis): FamilyKey => UNBOUNDED_FAMILY[axisTag(axis)];
```
Exhaustive `Record<ResourceAxisTag, …>` forces a TS compile update on a new axis tag (drift guard). (Serde shapes verified: unit variants → bare strings; `{"Mana":"Red"}`; `{"Life":0}` (PlayerId transparent); `{"Counter":["Poison","Player"]}`.)

## 7. Rust Idioms
Typed `BTreeMap<PlayerId, BTreeSet<ResourceAxis>>` (no bools); exhaustive `match` in `attribution_player`; `matches!(a, ResourceAxis::Mana(_))` gate; single write authority; `#[serde(default, skip_serializing_if=...)]` on both raw field and projection vec. `PartialOrd+Ord` derived together (clippy `derive_ord_xor_partial_ord` clean).

## 8. Variant Discoverability
`UnboundedResourceView` is a `DerivedViews` presentation struct (peer of `CommanderDamageView`/`PlayerStatusView`), not a gated enum variant. No new `DebugAction`; no new `ResourceAxis` variant. `engine-inventory.json` byte-identical (generator records variants/doc/CR, not derives — §4.2). Verify: `cargo engine-inventory` + `git diff --exit-code data/engine-inventory.json`.

## 9. Identity / Provenance Contract
Source: CR 732.2a net-progress loop's unbounded axes (`LoopCertificate.unbounded`). Authority: `mark_unbounded_loop(controller, axes)` keyed by controller. Binding: populate-time snapshot. Storage: `GameState.unbounded_resources` (serialized skip-empty; survives reconnect/save). Consumers: `derive_views` + refill/keep gates. Invalidation: `clear_unbounded_loop` (whole-player) / replace. Hostile fixture: victim-controls-permanent drain proves attribution keyed off payload not permanent control (test 9).

## 10. File-by-File (anchors)
ENGINE state/authority: (1) `types/game_state.rs` — field replace :6118 + §4.9 doc; init :7784; `mark_unbounded_loop`/`clear_unbounded_loop`; `use crate::analysis::resource::ResourceAxis`; manual `impl PartialEq` (:8056) EXCLUDED; `normalize_for_loop`(:7962)+`loop_fingerprint`(:7920) EXCLUDED. (2) `analysis/resource.rs` — `Serialize, Deserialize` on `ResourceAxis`(:534)/`ObjectClass`(:53)/`CounterClass`(:77)/`TriggerKind`(:117) + `PartialOrd, Ord` on `ResourceAxis`(:534). (3) `types/mana.rs` — `PartialOrd, Ord` on `ManaType`(:46).
ENGINE mana: (4) `mana_payment.rs` — `INFINITE_MANA_AXES`; refill gate swap :76/:79; adapt tests :3980,:4041,:4063. (5) `turns.rs` — keep gate :249, CR 500.5 preserved. (6) `engine_debug.rs` — toggle :406/:410 delegates. (7) `analysis/corpus.rs:842` doc.
ENGINE projection: (8) `game/derived_views.rs` — `UnboundedResourceView`; `DerivedViews.unbounded_resources`; `attribution_player` (+MED-4 poison comment + CR 704.5c/732.2a/704.5a); `derive_views` push before commander short-circuit (~:445; every format).
FRONTEND: (9) `client/src/adapter/types.ts` mirrors + `ResourceAxisTag`; (10) `client/src/hooks/usePlayerDesignations.ts` `unboundedResources` via `forPlayer`; (11) `client/src/components/hud/HudBadges.tsx` `axisTag`+`UNBOUNDED_FAMILY`+`familyOf`+`UnboundedBadge`; (12) `PlayerHud.tsx`+`OpponentHud.tsx` one badge per distinct family; (13) `ManaPoolSummary.tsx` optional ∞ marker; (14) i18n en. No `DebugPlayerActions` change.

## 11. Discriminating Test Map (revert-probes)
1. **Cert→projection (real damage pinger):** `pinger_scenario(1)` is a pure damage pinger (`with_life(P1,40)` = opponent life; NO controller lifegain). Drive via `LoopProbe` as `drive_damage_loop_certificate`(:613); real cert `{DamageDealt(P1), Life(P1)}` (P1 life negative; `unbounded_axes_for` resource.rs:510-518; `Life(P0)` impossible — delta.life[P0]==0 dropped by `map_delta`). Assert `mark_unbounded_loop(P0, &cert.unbounded)`→`derive_views`→projection contains `{player:P1, axis:DamageDealt(P1)}` AND `{player:P1, axis:Life(P1)}` (both victim P1). Revert: delete projection loop→empty→fail; without mark→empty.
2. **Toggle enable byte-preservation:** `unbounded_resources[p0] ⊇` 6 `Mana(_)`; pool tops `PER_TYPE`/color (existing asserts kept); 6 `Mana` rows on P0; disable→cleared. Revert: break `Mana(_)` match in refill gate→pool stops→fail.
3. **Non-mana axis: no top-up:** `mark_unbounded_loop(P0, &[TokensCreated])`→pool empty; projection shows `TokensCreated`. Revert: broaden gate to `!is_empty()`→pool fills→fail.
4. **Serde round-trip:** `[Mana(Red), Life(P1), Counter(Poison,Player), Trigger(Proliferate), TokensCreated]`→json→back→`Eq`. Revert: remove `Deserialize`→fail.
5. *(opt)* **Wire shape:** serialize `ClientGameStateRef`→`derived.unbounded_resources` present non-empty; empty omits key.
7. **`attribution_player` unit (both directions):** controller-self `Life(P0),P0)==P0`, `LibraryDelta(P0),P0)==P0`, `DamageDealt(P0),P0)==P0`; victim `Life(P1),P0)==P1`, `DamageDealt(P1),P0)==P1`, `LibraryDelta(P1),P0)==P1`; aggregates `Mana(Red),P0)==P0`, `Counter(Plus1Plus1,Creature),P0)==P0`, `Trigger(Proliferate),P0)==P0`, `TokensCreated,P0)==P0`. Revert: change `Life|DamageDealt|LibraryDelta => p` arm to `=> controller`→3 victim asserts fail.
8. **Loop-detection equality guard:** two GameStates identical except `unbounded_resources` (empty vs `mark_unbounded_loop(P0, &INFINITE_MANA_AXES)`). Assert `a == b` (PartialEq) AND `loop_states_equal(&a,&b) == true` AND **`loop_states_equal_modulo_resources(&a,&b) == true`** (directly guards the PR-0/PR-2 modulo path, not just transitively). Revert: add `&& self.unbounded_resources == other.unbounded_resources` to manual `eq`→fails. In `game_state.rs` `#[cfg(test)]`.
9. **Attribution hostile e2e (lead #3):** hand-built cert `{DamageDealt(P1), Life(P1)}` where P1 (victim) controls a permanent→both rows on P1's HUD. Revert: `attribution_player` returns `controller` for `DamageDealt`/`Life`→rows on P0→fail.
FRONTEND (`PlayerHud.test.tsx`): 10. `derived.unbounded_resources=[{player:0, axis:"TokensCreated"}]`→∞ badge on P0; `[]`→none; two `Mana` rows→one ∞ mana badge (de-dup). Revert: stop reading `derived.unbounded_resources`→absent→fail.

## 12. CR Annotations (grep-verified against docs/MagicCompRules.txt)
CR 732.2a (:6372), CR 500.5 (:2119)/500.4 (:2117), CR 704.5a (:5492), CR 704.5b (decking/LibraryDelta context), CR 704.5c (:5496 poison comment), CR 104.4b (exclusion doc + test 8), CR 106.4 (:416)/122.1 (:1178). All verified; none flagged.

## 13. Maintainer-Sim Matrix
Authority: `mark_unbounded_loop` (write) + `attribution_player` (HUD routing). Binding: populate-time snapshot. Storage: `GameState.unbounded_resources` serialized skip-empty. Consumers: `derive_views` + refill/keep. Invalidation: whole-player clear / replace. **Serialized-surface:** raw field additive skip-empty; `DerivedViews` +1 optional skip-empty; `engine-inventory.json` byte-identical (serde+Ord derives not recorded by the generator, incl. the `ManaType` Ord add); TS `GameState` unchanged + `DerivedViews` +1 optional. EXCLUDED from `PartialEq`(:8056)/`normalize_for_loop`(:7962)/`loop_fingerprint`(:7920) — display state; preserves CR 104.4b/732.2a equality + AI-dedup; guarded by test 8 (incl. modulo path). Hostile fixtures: victim-controls-permanent (9); non-mana vs mana (3); empty (1/10); real pinger cert (1); serde all families (4); populated-vs-empty equality+modulo (8); attribution both directions (7).

## 14. Risks
- **Mana regression** — mitigated: all-6 body preserved + `any(Mana(_))` gate + revert-probed tests (2/3).
- **Manual-eq trap (B2) — highest severity:** rename does NOT compile-error at hand-written `impl PartialEq`(:8056). Mitigations: §4.9 decision + field doc comment + matrix row + guard test 8.
- **`Ord` on `ManaType`/`ResourceAxis`** — additive derives; `ManaType` has no manual Ord impl (verified); the derived order is declaration-order (semantically meaningless, used only for `BTreeSet` determinism). Inventory byte-identical (generator ignores derives).
- **Poison wrong-HUD (MED-4)** — no live producer in PR-6; code comment + CR 704.5c gate PR-7.
- **TS union drift (MED-5)** — `axisTag` + exhaustive `Record<ResourceAxisTag, FamilyKey>`.
- **Multi-agent collisions** — surgical `Edit`s, re-read before edit, no whole-file `Write`.
- **First `types→analysis` edge** — legal (`pub mod analysis` unconditional), annotated.

## 15. Verification
```
cargo fmt --all
cargo engine-inventory   # then: git diff --exit-code data/engine-inventory.json  (expect byte-identical)
cargo clippy --workspace --exclude phase-tauri --all-targets --features engine/proptest -- -D warnings
cargo test -p engine     # incl. tests 1–9 (esp. attribution unit test 7 + PartialEq guard test 8)
pnpm -C client type-check
pnpm -C client lint
pnpm -C client test       # incl. test 10
```
`.planning/` never in the PR; `mtgish` untouched.
