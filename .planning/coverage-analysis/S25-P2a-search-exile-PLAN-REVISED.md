# S25-P2a REVISED — Multi-zone name-matched search-and-exile (interactive "any number" class)

**Increment:** P2a (FINAL classification B1 — genuine creation, now a **≥4-card** class)
**Worktree:** `/home/lgray/vibe-coding/s25-impl-wt` · branch `feat/std-s25-completion` · HEAD `1852b5d9f`
**Skills applied:** `.claude/skills/oracle-parser/SKILL.md` (nom mandate, dispatch, AST), `.claude/skills/add-engine-effect/SKILL.md` (reuse-`SearchLibrary` lifecycle), `.claude/skills/add-engine-variant/SKILL.md` (Stage-1 existence gate), `.claude/skills/card-test/SKILL.md` (discriminating, revert-to-red).
**Supersedes:** `S25-P2a-search-exile-PLAN.md` (banked, stamped `3abcfef7b`, never reviewed). Original preserved for audit/diff.

---

## 0. Revision delta vs banked plan

Two independent measurements (each cited below, verified against HEAD `1852b5d9f`) reshape the work-item set. **Neither** the banked plan's W1/W2/W3 (three fresh parser items: new recognizer + new AST arm + new dispatch) **nor** the driver-brief's proposed reduction (two possessive-`alt()` extensions rerouting through the *general* `parse_search_library_details` path) is correct. The truth is in between, and the card class is larger than believed.

| # | Change | Why (measured) |
|---|---|---|
| **D1** | **REJECT the brief's two-`alt()` reroute.** Do NOT edit `parse_multi_search_zones` (search.rs:887) or `parse_search_target_player` (search.rs:613). | The general filter path maps **Deadly Cover-Up's "with that name" → `TargetFilter::HasChosenName`** (search.rs:2242-2249), which resolves against `source.chosen_attributes` `CardName` (filter.rs:1886-1893). Deadly has **no "choose a card name" clause** → zero matches. The reroute silently mis-parses Deadly. It cannot be globally flipped to `SameNameAsParentTarget` (that breaks Lost Legacy / the "choose a card name … up to four cards with that name" search at search.rs:2760, where `HasChosenName` is correct). |
| **D2** | **REPLACE banked W1+W2+W3 with ONE parser item (W1'): generalize the *existing* recognizer's quantifier axis.** `try_parse_multi_zone_same_name_exile` (imperative.rs:2170) **already** matches every owner possessive incl. `its owner's`→`ParentTargetOwner` / `its controller's`→`ParentTargetController` (imperative.rs:2174-2189), the GY/hand/library zone list (2191-2198), and the same-name suffix for **both** "with that name" and "with the same name as that `<type>`" (2209-2231). The **sole** thing declining "any number" is `tag("all cards")` at **imperative.rs:2201**. Generalize that one tag to `alt(("all cards" \| "any number of cards" \| "up to N cards"))`, thread the quantifier out, and branch the lowering (imperative.rs:2834): `all cards`→`ChangeZoneAll` (unchanged); `any number`/`up to N`→`Effect::SearchLibrary` with a **hard-coded** `SameNameAsParentTarget` filter (identical to the ChangeZoneAll sibling at imperative.rs:2841) + the owner axis on `target_player`. This reuses the recognizer's correct grammar and correct name semantics — sidestepping D1's `HasChosenName` landmine entirely. |
| **D3** | **Class is 4 cards, not 2.** `Crumble to Dust` ("its controller's" + any number + "same name as that land") and `Surgical Extraction` ("its owner's" + any number + "same name as that card") share the exact grammar and currently gap identically — the design-boundary test `name_hate_any_number_spells_do_not_auto_exile` (tests.rs:31809-31826) only asserts they must NOT become `ChangeZoneAll`; it does **not** assert any positive lowering, i.e. they are `Unimplemented{"search"}` today. W1' fixes all four uniformly. Build for the class. |
| **D4** | **W4 CONFIRMED REQUIRED** (banked §3 wrongly implied the resolver "already" resolves the Typed axis). `resolve_library_owner` (search_library.rs:25-46) resolves only (a) a pre-resolved `TargetRef::Player` and (b) the **bare** `TargetFilter::ParentTargetController` (line 40). A `Typed(controller=ParentTargetOwner/Controller)` **falls through to `ability.controller`** (the caster searches their own zones = wrong player). The extension is small: read the controller ref off a `Typed` `target_player` and call the existing `controller_ref_player` (filter.rs:730, which already handles both axes at lines 750-751). |
| **D5** | **W6 REDUCED to a one-line-class fix.** `extract_player_anchor` (mod.rs:15744) already reads `Effect::SearchLibrary { target_player }` and normalizes `Typed(controller=ParentTargetOwner/Controller)`→the bare anaphor (mod.rs:15746-15772); `apply_anchor_subject` (mod.rs:15827) already rebinds a trailing `Shuffle` — but has **no `Effect::Draw` arm**. That missing arm is exactly the probe-observed `Draw{target:Controller}` bug. Add one arm. |
| **D6** | **W5 CONFIRMED REAL; W7 CONFIRMED REAL & Deadly-only.** (details in §4.) |
| **D7** | Anchors refreshed to HEAD; stale names corrected: fn is `parse_search_and_creation_ast` (imperative.rs:2325), `search.rs`/`imperative.rs`/`mod.rs` are under `parser/oracle_effect/`, and `engine_resolution_choices.rs` is under `game/` (not `game/effects/`). The frozen `game/effects/mod.rs` is **distinct** from the parser orchestrator `parser/oracle_effect/mod.rs` that W6 edits. |

**Net work-item count:** parser **1** (W1', replacing banked 3) + resolver **1** (W4) + counter **1** (W5) + anchor-draw **1** (W6) + Deadly seed flag **1** (W7) = **5 items**, **0 new engine enum variants**, **0 frozen-file edits**, covering **4 cards**.

---

## 1. Exact Oracle text + shared decomposition

**Deadly Cover-Up** — `{3}{B}{B}` Sorcery (MKM):
> As an additional cost to cast this spell, you may collect evidence 6.
> Destroy all creatures. If evidence was collected, exile a card from an opponent's graveyard. Then search its owner's graveyard, hand, and library for any number of cards with that name and exile them. That player shuffles, then draws a card for each card exiled from their hand this way.

**The End** — `{2}{B}{B}` Instant (OTJ):
> This spell costs {2} less to cast if your life total is 5 or less.
> Exile target creature or planeswalker. Search its controller's graveyard, hand, and library for any number of cards with the same name as that permanent and exile them. That player shuffles, then draws a card for each card exiled from their hand this way.

**In-class (D3) — same grammar, currently gapping identically:**
- **Crumble to Dust** — "Exile target nonbasic land. Search **its controller's** graveyard, hand, and library for **any number of cards with the same name as that land** and exile them. Then that player shuffles." (controller axis, targeted seed)
- **Surgical Extraction** — "Choose target card in a graveyard. Search **its owner's** graveyard, hand, and library for **any number of cards with the same name as that card** and exile them. Then that player shuffles." (owner axis, targeted seed)

**Shared core primitive (byte-identical tail across all four modulo the same-name noun):**
`search <possessive> graveyard, hand, and library for any number of cards with <same-name-ref> and exile them.` (+ shuffle/draw rider on the two S25 cards).

| Axis | Deadly | The End | Crumble | Surgical |
|---|---|---|---|---|
| Owner axis | `its owner's`→`ParentTargetOwner` (GY card, CR 108.3/109.4) | `its controller's`→`ParentTargetController` (battlefield permanent) | `ParentTargetController` | `ParentTargetOwner` |
| Same-name ref | **"that name"** (→ landmine, see D1) | "same name as that permanent" | "same name as that land" | "same name as that card" |
| Seed | **non-targeted** exiled GY card → needs **W7** `forward_result` | targeted exiled permanent | targeted exiled land | targeted card in GY |
| Draw rider (S25) | present | present | (no rider) | (no rider) |

> **Measured corrections that STAND from the banked plan:** The End targets **"creature or planeswalker", NO legendary restriction**. Deadly uses **owner** axis (GY card, CR 108.3/109.4); The End uses **controller** axis (battlefield permanent, CR 109.4).

---

## 2. Existing building blocks + the precise gap (all verified at HEAD)

### 2a. The dispatch gate and why the clause is unclaimed today
`parse_search_and_creation_ast` (imperative.rs:2325) tries the multi-zone recognizer first (imperative.rs:2348), then the library-search gate (imperative.rs:2351-2366):
1. **imperative.rs:2348** `try_parse_multi_zone_same_name_exile(lower)` — handles `its owner's`/`its controller's` (2174-2189) but hard-requires `tag("all cards")` (**2201**) ⇒ declines "any number of cards" ⇒ `None`.
2. **imperative.rs:2351-2366** gate needs ONE of: `starts_with_possessive(lower,"search","library")` (word after possessive is "graveyard" ⇒ **false**) · `parse_multi_search_zones(lower).is_some()` (its `opt(alt(...))` at search.rs:894-901 strips only `your `/`their `/`target player's `/`target opponent's `/`an opponent's ` — **not** `its owner's `/`its controller's `, so `parse_zone_word` fails on "its owner's graveyard" ⇒ **None**) · the `search target/an opponent's library` nom branch (**false**).
⇒ all three false ⇒ clause falls to `Effect::unimplemented("search", …)`. **This is the measured gap for all four cards.**

### 2b. Building blocks reused as-is (verified — do NOT reinvent)
| Capability | Location (HEAD) | Note |
|---|---|---|
| Multi-zone search effect | `Effect::SearchLibrary { source_zones, filter, count, reveal, target_player, selection_constraint, split }` — types/ability.rs:9600 | `source_zones` already supports `[Graveyard,Hand,Library]` |
| "any number"/"up to N" count | `QuantityExpr::UpTo { max }` — types/ability.rs:5447; ctor `QuantityExpr::up_to(count)` (imperative.rs:190/988). Doc on `SearchLibrary.count` confirms this encoding (no separate `up_to` bool on the effect). | CR 107.1c / CR 701.23b |
| Same-name filter | `FilterProp::SameNameAsParentTarget`; hard-coded by the ChangeZoneAll sibling at imperative.rs:2841 | resolves vs. parent target's name (LKI-safe) |
| Owner/controller axes | `ControllerRef::ParentTargetOwner`/`ParentTargetController`; resolver `controller_ref_player` (filter.rs:730) → `parent_target_owner_player`/`parent_target_controller_player` (filter.rs:675/662) → `resolve_effect_player_ref` | building block for W4 |
| Existing recognizer grammar | `try_parse_multi_zone_same_name_exile` (imperative.rs:2170-2236) — owner alt (2174-2189) + zone list (2191-2198) + same-name suffix "with that name" / "with the same name as that `<type>`" (2209-2231) | reused by W1' |
| SearchChoice completion → continuation exile | `engine_resolution_choices.rs:1845-1969`; found set → `cont.chain.targets` (:1953-1966) then `drain_pending_continuation` (:1968) → SearchDestination `ChangeZone{Exile}` + `Shuffle` + `Draw` | intrinsic `SearchDestination` attaches for `SearchLibrary` (sequence.rs:4071/4127-4134) |
| Player-anchor carry (shuffle/draw axis) | `extract_player_anchor` (mod.rs:15744, already reads `SearchLibrary.target_player` + normalizes Typed→anaphor) + `apply_anchor_subject` (mod.rs:15827, has Shuffle arm, **lacks Draw arm**) | W6 |
| Hand-exile counter | `state.exiled_from_hand_this_resolution` (game_state.rs:7215); **sole increment** at change_zone.rs:1423 (inside the `ChangeZoneAll` mass-move loop); read by `QuantityRef::ExiledFromHandThisResolution` (quantity.rs:2261); reset engine.rs:220 / game/effects/mod.rs:5196 | W5 |
| forward_result seed threading | AST flag on a `ChangeZone` binding the moved object into the parent slot | W7 (Deadly only) |

### 2c. Design boundary that MUST hold (CR 701.23b)
`name_hate_any_number_spells_do_not_auto_exile` (tests.rs:31809) asserts "any number" name-hate must **not** lower to `ChangeZoneAll` (mandatory exile-all is wrong when the searcher may fail to find, CR 701.23b). W1' routes "any number"→**interactive `SearchLibrary`** (`SearchLibrary` ≠ `ChangeZoneAll`), so this test stays green and the four cards now lower to a *positive* effect. `name_hate_spells_parse_multi_zone_same_name_exile_chain` (tests.rs:31761, "all cards"→ChangeZoneAll) and `name_hate_owner_axis_shuffle_inherits_parent_target_owner` (tests.rs:31831) must remain green (W1' leaves the `all cards` branch untouched).

---

## 3. Target lowered chain (corrected)

For the search clause (all four cards), W1' emits — **directly, hard-coded, mirroring imperative.rs:2834-2849** — not via `parse_search_library_details`:

```rust
Effect::SearchLibrary {
    source_zones: vec![Zone::Graveyard, Zone::Hand, Zone::Library],
    // Hard-coded — NOT the general filter parser (which yields HasChosenName for "with that name", D1).
    filter: TargetFilter::Typed(
        TypedFilter::default().properties(vec![FilterProp::SameNameAsParentTarget]),
    ),
    // "any number" → UpTo{Fixed{i32::MAX}} (resolver floors to matching-set size); "up to N" → UpTo{Fixed{N}}.
    count: QuantityExpr::up_to(QuantityExpr::Fixed { value: i32::MAX }),
    reveal: false,
    // Typed encoding keeps searcher_is_library_owner == false (caster searches, CR 701.23a asymmetric);
    // W4 teaches resolve_library_owner to resolve the .controller ref to the parent target's owner/controller.
    target_player: Some(TargetFilter::Typed(
        TypedFilter::default().controller(<ParentTargetOwner | ParentTargetController>),
    )),
    selection_constraint: SearchSelectionConstraint::None,
    split: None,
}
// intrinsic SearchDestination continuation (sequence.rs:4071) → ChangeZone{dest:Exile}
// trailing clauses (parsed independently): Shuffle{target:<axis>} , Draw{count:Ref(ExiledFromHandThisResolution), target:<axis>}
```

**Why `Typed(controller=<axis>)` and not bare `ParentTargetController`:** the bare variant makes `searcher_is_library_owner` return **true** (search_library.rs:54-68, line 64) ⇒ the *owner* would search their own zones — wrong (and load-bearing for Assassin's Trophy self-search, tests.rs:9861/9948/10029). The `Typed` wrapper returns **false** ⇒ caster is the searcher (correct, CR 701.23a). W4 supplies the missing owner resolution for the `Typed` case without touching the bare-variant branch.

---

## 4. `/add-engine-variant` gate + concrete work items

### `/add-engine-variant` — Stage-1 existence verification (re-run vs `data/engine-inventory.json`, read-only)
| Element | Verdict | Evidence |
|---|---|---|
| Multi-zone search effect | EXISTS_SAME_NAME | `Effect::SearchLibrary` types/ability.rs:9600 |
| "any number"/"up to" count | EXISTS_SAME_NAME | `QuantityExpr::UpTo` types/ability.rs:5447 |
| same-name filter | EXISTS_SAME_NAME | `FilterProp::SameNameAsParentTarget` (imperative.rs:2841) |
| owner/controller axes | EXISTS_SAME_NAME | `ControllerRef::ParentTargetOwner`/`ParentTargetController` (filter.rs:750-751) |
| hand-exile count ref | EXISTS_SAME_NAME | `QuantityRef::ExiledFromHandThisResolution` (quantity.rs:2261) |
| seed threading | EXISTS_SAME_NAME | `forward_result` flag on `ChangeZone` |

**Gate verdict: ZERO new engine enum variants.** W1' threads a quantifier through a **parser-internal AST** (`SearchCreationImperativeAst::MultiZoneSameNameExile` in `oracle_ir/ast.rs`) — adding a field to a parser AST variant is *not* one of the `/add-engine-variant`-gated engine enums (`Effect`, `QuantityRef`, `FilterProp`, `TargetFilter`, `ControllerRef`, …). No Stage-2/Stage-3 needed.

### Work items

**W1' (parser — `imperative.rs`, replaces banked W1+W2+W3):** Generalize the quantifier axis of `try_parse_multi_zone_same_name_exile` (imperative.rs:2170).
- Change `tag("all cards")` (imperative.rs:2201) → `alt((value(Quantifier::All, tag("all cards")), value(Quantifier::AnyNumber, tag("any number of cards")), map(preceded(tag("up to "), parse_number_or_x), Quantifier::UpTo)))` (nom-composed; the number path reuses `nom_primitives::parse_number_or_x`). Return `(owner, quantifier)`.
- Carry the quantifier onto the AST (`MultiZoneSameNameExile { owner, quantifier }` — parameterize the existing variant; a sibling `MultiZoneSameNameSearch` is an acceptable lower-churn alternative if the reviewer prefers not to alter the "all cards" return path).
- Branch lowering at imperative.rs:2834: `All`→existing `ChangeZoneAll` (unchanged); `AnyNumber`/`UpTo(n)`→the `Effect::SearchLibrary` of §3 (owner axis on `target_player`, count `UpTo`, hard-coded `SameNameAsParentTarget`).
- Update both call sites of the recognizer (imperative.rs:2348 and mod.rs:6263).
- **nom mandate:** the quantifier is ONE `alt()` axis appended to the existing composed grammar — no flat full-string `tag` permutations, no `find`/`split_once`/`contains`/`starts_with`.
- **Do NOT** touch `parse_multi_search_zones` / `parse_search_target_player` (D1) — those feed the general path and would re-introduce the `HasChosenName` mis-parse for Deadly.

**W4 (resolver — `search_library.rs:25`, NOT frozen) — KEPT/REAL:** In `resolve_library_owner`, after the `TargetRef::Player` and bare-`ParentTargetController` branches, add:
```rust
// CR 701.23a + CR 108.3/109.4: a Typed target_player carries the searched player as a
// controller-ref (ParentTargetOwner/Controller). The caster is still the searcher
// (searcher_is_library_owner(Typed{..}) == false); only the *searched zones' owner* is derived here.
if let TargetFilter::Typed(tf) = target_player {
    if let Some(ctrl) = tf.controller {
        if let Some(pid) = controller_ref_player(state, ability.source_id, Some(ability.controller), Some(ability), &ctrl) {
            return pid;
        }
    }
}
```
Leave `searcher_is_library_owner` and the bare-`ParentTargetController` branch **unchanged** (Assassin's Trophy self-search preserved). Confirm `matches_target_filter_in_owner_zone` (imported at search_library.rs:2) evaluates `SameNameAsParentTarget` against the library-owner's ownership-substituted candidates (CR 109.5) — it is the existing owner-zone matcher already used by every multi-zone search.

**W5 (counter — NOT frozen) — KEPT/REAL:** `exiled_from_hand_this_resolution` is incremented at **exactly one** production site, change_zone.rs:1423, inside the `ChangeZoneAll` per-object mass-move loop (guard `per_object_origin == Hand && dest_zone == Exile`). The interactive found-set exile runs through the pending continuation's single `ChangeZone` (engine_resolution_choices.rs:1968 → SearchDestination), which never reaches that loop ⇒ the shared draw rider counts **0** on Deadly & The End without a fix.
- **Correctness criterion (match the existing site's rigor):** increment **once per chosen card whose pre-move zone == Hand and post-move zone == Exile**, scoped to *this search's found set* (never a blanket hand→exile counter — that would over-count discards/other exiles; CR "this way" is set-scoped).
- **Preferred site:** the found-set exile execution path so the count is taken **post-move** (like change_zone.rs:1411-1417's post-move check). If the single-`ChangeZone` per-object move helper (change_zone.rs, not frozen) is the shared execution point, gate the increment there on the search-continuation context; otherwise take it at the `SearchChoice` completion (engine_resolution_choices.rs:1874, not frozen) over `chosen` filtered to `zone==Hand`, gated on the pending continuation's destination being `Exile`. **Flag for review:** pre-move counting at the completion site is simpler but over-counts if a replacement prevents an exile — prefer the post-move option.
- Verify the counter survives the `SearchChoice` pause (it is a `GameState` field, compared in `game_state.rs:8651`) and is read by the later `Draw` before the resolution-end reset (engine.rs:220 / game/effects/mod.rs:5196).

**W6 (parser — `parser/oracle_effect/mod.rs:15827`, NOT frozen) — KEPT, REDUCED:** Add an `Effect::Draw` arm to `apply_anchor_subject`, mirroring the existing `Shuffle` arm (rewrite `target` when it is `Controller | Player | ParentTargetController` → `anchor`):
```rust
Effect::Draw { target, .. }
    if matches!(*target, TargetFilter::Controller | TargetFilter::Player | TargetFilter::ParentTargetController) =>
{ *target = anchor.clone(); }
```
`extract_player_anchor` (mod.rs:15746-15772) already yields the correct anchor from `SearchLibrary.target_player` (incl. Typed→anaphor normalization), and the `Shuffle` rebind already works — so this single arm fixes the probe-observed `Draw{target:Controller}` for all four cards (controller axis for The End/Crumble, owner axis for Deadly/Surgical). Confirm `apply_anchor_subject` is invoked on the Draw effect in the chain (it descends via `extract_player_anchor_in_chain`, mod.rs:15790); if the draw is a sub-ability, ensure the anchor application walks the sub-chain.

**W7 (parser — Deadly only, NOT frozen) — KEPT:** Deadly's seed ("exile a card from an opponent's graveyard") is a **non-targeted** `ChangeZone` (probe: `forward_result:false`) ⇒ it is not in `ability.targets`, so `SameNameAsParentTarget` binds to nothing. Set `forward_result:true` on that seed-exile `ChangeZone` in the parser lowering so the chosen exiled card seeds the parent slot for the downstream search/shuffle/draw. **The End / Crumble / Surgical need no W7** — their seed is the targeted exiled permanent/land/card already in `ability.targets`. Confirm `forward_result` populates the sub-chain `targets` (so `first_object_target` / `SameNameAsParentTarget` find it), not merely `source_id`.

---

## 5. Files to touch (dependency order — all NON-frozen)

1. `crates/engine/src/parser/oracle_effect/imperative.rs` — **W1'** (quantifier axis at :2201, AST field, lowering branch at :2834, caller at :2348); **W7** `forward_result` on Deadly's seed-exile `ChangeZone` lowering.
2. `crates/engine/src/parser/oracle_effect/mod.rs` — **W1'** recognizer caller at :6263; **W6** `Draw` arm in `apply_anchor_subject` (:15827). *(This is the parser orchestrator — distinct from the frozen `game/effects/mod.rs`.)*
3. `crates/engine/src/game/effects/search_library.rs` — **W4** `resolve_library_owner` Typed-controller resolution (:25). NOT frozen.
4. `crates/engine/src/game/engine_resolution_choices.rs` **or** `crates/engine/src/game/effects/change_zone.rs` — **W5** hand-exile counter on the interactive found-set exile. Both NOT frozen.
5. Colocated tests (§7).

**Frozen-file check (F):** `game/effects/mod.rs`, `game/filter.rs`, `game/effects/delayed_trigger.rs` — **no design edit required in any.** `filter.rs::controller_ref_player` / `SameNameAsParentTarget` / `parent_target_*_player` are **read/reused as-is** (W4 calls `controller_ref_player` from the non-frozen `search_library.rs`). No W5 candidate site is frozen. W6 edits `parser/oracle_effect/mod.rs`, not `game/effects/mod.rs`.

---

## 6. CR annotations (all re-verified vs `docs/MagicCompRules.txt`)
| CR | Verified text | Applies to |
|---|---|---|
| **107.1c** | "any number" → any positive number or zero (line 460) | W1' quantifier; `UpTo{MAX}` |
| **701.23a** | search a zone; look at all cards even if hidden (line 3465) | W1'/W4 search; caster=searcher (asymmetric) |
| **701.23b** | hidden-zone stated-quality search — player isn't required to find (line 3467) | mandates interactive `SearchLibrary`, forbids `ChangeZoneAll` (§2c) |
| **701.24** | "Shuffle" (701.24a randomize) | shuffle step. *(Codebase mis-annotates shuffle as CR 701.18a in places — 701.18 is "Play"; use CR 701.24 in new annotations.)* |
| **201.2 / 201.2a** | two objects "have the same name" iff ≥1 name in common | `SameNameAsParentTarget` |
| **400.7** | zone change → new object, no memory | seed name via LKI; per-object origin |
| **108.3** | owner = player who started with the card (line 564) | Deadly/Surgical "its owner's" → `ParentTargetOwner` |
| **109.4** | only stack/battlefield objects have a controller (line 594) | Deadly uses owner (GY card); The End/Crumble use controller (battlefield/exiled-permanent) |
| **701.59 / 701.59a-c** | Collect Evidence N; linked "evidence was collected" | Deadly additional cost + `AdditionalCostPaid` (already parsed) |

---

## 7. Test plan (`/card-test` — discriminating, revert-to-red)

**Foot-gun compliance:** `GameScenario` + `GameRunner::cast(...).resolve()`, assert on `CastOutcome` deltas / observed zones; never hand-write `TargetRef` vectors; submit the full `SearchChoice` selection; take the hand baseline at the right point; every negative assertion non-vacuous (a same-named decoy that MUST survive).

### 7a. Parser tests (fast loop) — colocated in `imperative.rs`/`tests.rs`
1. **All four cards** — `parse_effect_chain(full_text)` chain must contain `Effect::SearchLibrary` with `source_zones==[Graveyard,Hand,Library]`, `count` matching `QuantityExpr::UpTo{..}`, `filter` carrying `FilterProp::SameNameAsParentTarget`, `target_player==Some(Typed(controller==<ParentTargetOwner|ParentTargetController per card>))`; chain contains `ChangeZone{dest:Exile}`; **root effect is NOT `Unimplemented`.** (Deadly/The End additionally: `Shuffle{<axis>}`, `Draw{Ref(ExiledFromHandThisResolution), target==<axis>}`.)
2. **Deadly-specific** — seed-exile `ChangeZone` has `forward_result==true` (W7); additional cost `CollectEvidence{6}` present; filter is `SameNameAsParentTarget` (**assert it is NOT `HasChosenName`** — the D1 discriminator).
3. **Discrimination / no-collateral (revert-to-red):**
   - `name_hate_any_number_spells_do_not_auto_exile` (tests.rs:31809) stays green — Crumble/Surgical now lower to `SearchLibrary` (still ≠ `ChangeZoneAll`).
   - `name_hate_spells_parse_multi_zone_same_name_exile_chain` (tests.rs:31761) + `name_hate_owner_axis_shuffle_inherits_parent_target_owner` (tests.rs:31831) stay green — the `all cards` branch is untouched.
   - Assassin's Trophy `SearchLibrary{target_player: bare ParentTargetController}` self-search unchanged (tests.rs:9861/9948/10029) — W4 only adds a `Typed` branch.
   - **Non-vacuity per item:** revert W1' → assertion 1 flips to `Unimplemented{"search"}` (measured current state). Revert D1 discipline (route via general path) → Deadly's filter becomes `HasChosenName` and runtime 7b-Deadly finds nothing. Revert W4 → runtime searches the caster's zones. Revert W6 → draw goes to caster. Revert W5 → draw count 0. Revert W7 → Deadly finds nothing.

### 7b. Runtime cast tests (`GameScenario`/`GameRunner`) — anti-hollow-win observables
**The End (controller axis, targeted seed):** P0 caster; P1 controls `Grizzly Bears` on battlefield + same-named `Grizzly Bears` in each of GY/hand/library + a decoy `Llanowar Elves` in each zone; **P0** holds its own `Grizzly Bears` in hand (scoping decoy). Cast targeting P1's battlefield Bears; drive `SearchChoice` selecting all offered. **Assert:** battlefield Bears + all three P1 Bears (GY/hand/lib) exiled; P1's library shuffled; **P1 drew exactly 1** (one Bears from P1's hand) to **P1's** hand. **Discriminating negatives (all must hold):** every P1 `Llanowar Elves` survives; **P0's own hand Bears survives** (proves caster searched P1's zones, W4). *(Crumble to Dust is the parser-level controller-axis twin; a parser test suffices — no draw rider.)*

**Deadly Cover-Up (owner axis, non-targeted seed, evidence gating):** P0 caster; destroy-all fodder on battlefield; P1 owns a `Grizzly Bears` in GY (seed) + same-named copies across GY/hand/lib + decoy; P0 owns a `Grizzly Bears` in hand (scoping decoy); both have libraries.
- **Case A (no evidence):** cast without paying collect-evidence → only `Destroy all creatures`; seed stays in GY; nothing exiled from hand/lib; `exiled_from_hand_this_resolution==0`; P1 draws 0.
- **Case B (evidence paid):** collect evidence 6, choose the P1-GY seed, drive `SearchChoice` selecting all → seed + all P1 Bears exiled across all three zones; P1 shuffles; **P1 drew exactly #(Bears exiled from P1's hand)**; P0's hand Bears survives; decoy survives.
- **Non-vacuity:** Case A vs Case B is the discriminating pair; reverting W7 makes Case B find nothing; reverting W5 makes Case B's draw 0. *(Surgical Extraction is the owner-axis twin with a targeted seed; a parser test suffices — no draw rider, no W7.)*

---

## 8. Risks / open questions for `/review-engine-plan`

1. **[HIGH] W5 site & scoping.** The draw rider on both S25 cards depends on `exiled_from_hand_this_resolution`, written today only by the `ChangeZoneAll` loop (change_zone.rs:1423). Confirm: (a) the chosen site increments **post-move**, hand-origin, exile-dest, scoped to this search's found set (not a blanket hand→exile counter — would corrupt discards/other exiles); (b) it survives the `SearchChoice` pause and is reset only at resolution top; (c) no *other* interactive search (a tutor into hand) accidentally starts counting.
2. **[HIGH] W4 asymmetry.** Confirm: (a) the new `Typed` branch in `resolve_library_owner` resolves both axes via `controller_ref_player` (filter.rs:750-751) for a targeted seed (The End/Crumble/Surgical) **and** the forwarded seed (Deadly, after W7); (b) `searcher_is_library_owner(Typed{..})==false` (search_library.rs:54-68) so the caster is the searcher and makes the fail-to-find choice; (c) the bare-`ParentTargetController` branch (Assassin's Trophy) is byte-untouched.
3. **[MED] Name-snapshot lifetime.** The End's targeted permanent is exiled *before* the search resolves; `SameNameAsParentTarget`/`parent_target_*` must read the LKI name (filter.rs:714-728 `effective_controller` + LKI cache). Confirm the exile writes LKI (via `change_zone`) and the parent slot still points to the exiled id. For Deadly, confirm `forward_result` (W7) populates the sub-chain `targets`, not only `source_id`.
4. **[MED] Coverage-regression on the shared parse path.** W1' widens ONE `alt()` axis (the quantifier) on an already-strict recognizer (possessive + exact zone-list permutation + same-name suffix + "exile them"). Run the CI card-data coverage-regression check — a broadened `alt()` can silently swallow clauses on unrelated cards (MEMORY: "parser-coverage-regression-ci-only"). The recognizer's tight full-grammar gate is the mitigation; verify no card matching only part of the grammar is newly claimed.
5. **[MED] "any number" ⇄ "all cards" boundary.** W1' must keep `all cards`→`ChangeZoneAll` and `any number`/`up to N`→`SearchLibrary` cleanly separated on the quantifier axis only; keep `name_hate_any_number_spells_do_not_auto_exile` and `..._exile_chain` both green.
6. **[LOW] W1' AST shape choice.** Parameterizing `MultiZoneSameNameExile { owner, quantifier }` (CLAUDE.md "parameterize, don't proliferate") vs. a sibling `MultiZoneSameNameSearch { owner }` variant (lower churn on the `all cards` return path). Reviewer to pick; both are parser-internal AST, neither is a gated engine variant.
7. **[LOW] Multiplayer/AI/frontend lifecycle.** Reusing `Effect::SearchLibrary` pre-satisfies targeting, multiplayer visibility (visibility.rs), the `SearchChoice` frontend UI, and the AI `search_pick` enumerator. Confirm the AI enumerator picks "all" under `up_to` for these (it should) and that the searcher (caster) sees candidates while opponents don't.

---
_Verification: every file:line anchor in this document was read at HEAD `1852b5d9f`. Banked original preserved at `S25-P2a-search-exile-PLAN.md`. No source code edited during planning._

---

## 9. REVIEW OUTCOME (/review-engine-plan, opus/xhigh) — AUTHORITATIVE; supersedes conflicting text above

**VERDICT: APPROVE-WITH-REQUIRED-REVISIONS.** Architecture (reuse SearchLibrary, generalize the existing
recognizer's quantifier, W4/W5/W6/W7, zero engine variants, zero frozen edits) CONFIRMED sound. D2's claim that
W1' "sidesteps the HasChosenName landmine" is FALSE — it steps on it from the sibling side. The following R1-R4
are HARD IMPLEMENTATION GATES.

### R1 [BLOCKER] — object-relative possessive guard on the NEW quantifier arms
The sibling `try_parse_multi_zone_same_name_exile` is tried BEFORE the general HasChosenName gate (imperative.rs:2348,
mod.rs:6261) and hard-codes SameNameAsParentTarget (imperative.rs:2837-2842). Widening its `tag("all cards")`:2201 to
"any number"/"up to N" WITHOUT a guard STEALS 10 chosen-name cards (Unmoored Ego, The Stone Brain, Ancient Vendetta,
The Rise of Sozin, Lost Legacy, Memoricide, Slaughter Games, Stain the Mind, Infinite Obliteration, Necromentia) →
mis-maps their "with that name" (HasChosenName) to SameNameAsParentTarget → exiles nothing.
**FIX:** keep the `all cards` arm accepting all possessives (byte-unchanged); the NEW `any number`/`up to N` arms match
ONLY when `owner ∈ {ParentTargetOwner, ParentTargetController}` (object-relative). One typed `match` guard in the
recognizer → both callers inherit it. nom-compliant (match on already-returned typed ControllerRef).
**DRIVER-VALIDATED (jq over data/card-data.json, "with that name and exile" corpus):** the possessive/name-source
split is EXACT — OBJ:its-owner ⟺ Deadly Cover-Up (only seed, no-choose); ALL 12 player-possessive cards are CHOSEN.
Zero cross-contamination. R1's guard is provably correct for the whole class.

### R2 [BLOCKER] — chosen-name regression tests through REAL dispatch
The existing test multi_zone_chosen_name_exile_search_has_exile_destination (search.rs:2755) calls
parse_search_library_details DIRECTLY, bypassing the sibling at :2348/mod.rs:6261 — stays green even if W1' steals the
card. Add a test via `parse_effect_chain("Choose a card name. Search target opponent's graveyard, hand, and library
for up to four cards with that name and exile them. ...")` asserting SearchLibrary.filter == HasChosenName, NOT
SameNameAsParentTarget, root != Unimplemented. Revert-to-red: without R1 guard the filter flips (measured). No
chosen-name card is currently guarded in tests.rs (0 hits) — this class is unguarded today.

### R3 [REQUIRED] — class count 4→≥6; add Test of Talents + Deicide
W1' via the unambiguous "same name as that <noun>" branch (imperative.rs:2217-2229) + object-relative gate also
newly-claims: **Test of Talents** ("Counter target spell. Search its controller's ... any number ... same name as
that spell ... That player shuffles, then draws...") — a THIRD draw-rider card → independently exercises W5+W6; add
as runtime test. **Deicide** ("Exile target enchantment. If the exiled card is a God card, search its controller's
...") — filter correct but the leading "if the exiled card is a God card" conditional must survive (clause_shell
peel, not W1's job) — VERIFY it isn't dropped (else Deicide auto-exiles unconditionally). Update D3 to "≥6 cards,
3 with draw rider."

### R4 [REQUIRED] — enumerate coverage-regression corpus + run CI checks
§8 risk #4 must list the newly-claimed set + classification (reviewer appendix: 30-card corpus). Run CI card-data
coverage-regression AND `cargo semantic-audit` confirming (a) the 10 chosen-name cards keep HasChosenName under R1;
(b) Branch-B "that player's" hazards Kotose the Silent Spider + Reap Intellect (any number + same-name-as-that-card +
that player's) stay DECLINED under the object-relative gate (they'd resolve TargetPlayer against no player target →
search caster; Reap also flattens a for-each loop). MEMORY: parser-coverage-regression is CI-only.

### Residual risks for the IMPLEMENTER (confirmed-sound design; pin these at impl time)
1. W5 site [HIGH]: pin the post-move, hand-origin, exile-dest, found-set-scoped increment of
   exiled_from_hand_this_resolution; ensure read by later Draw before resolution-top reset (engine.rs:220); no
   tutor-to-hand search increments it. Sole current increment: change_zone.rs:1422-1424 (ChangeZoneAll-only).
   Interactive found-set exile bypasses via engine_resolution_choices.rs:1953-1968.
2. W7 interactive timing: Deadly's seed-exile is interactive; confirm its ZoneChanged lands in the forward_result
   window after the player choice resolves. forward_result inserts moved obj into sub.targets (game/effects/mod.rs:7306,
   the ability.targets.is_empty() else-branch) — resolves SameNameAsParentTarget/first_object_target/W4.
3. LKI snapshot [MED]: The End's target exiled before search resolves; SameNameAsParentTarget must read LKI name +
   parent slot still points to exiled id (filter.rs:714-728 effective_controller + lki_cache).
4. AST shape: parameterize `MultiZoneSameNameExile { owner, quantifier }` (CLAUDE.md parameterize-don't-proliferate);
   both callers (imperative.rs:2348, mod.rs:6261) thread the field.

### CONFIRMED-SOUND (blessed — do not re-litigate)
D1, W4 (resolve_library_owner Typed-controller branch via controller_ref_player filter.rs:730/750/751; bare
ParentTargetController + Assassin's Trophy untouched; searcher_is_library_owner stays false for Typed),
W5 mechanism, W6 (apply_anchor_subject mod.rs:15827-15852 has no Draw arm → add one mirroring Shuffle),
W7 (forward_result → sub.targets), variant gate (SearchCreationImperativeAst is parser-internal, ungated),
Effect::SearchLibrary shape (no up_to bool; count=QuantityExpr::UpTo{Fixed{i32::MAX}} is the idiom, ability.rs:9600-9650),
frozen files untouched, CR annotations (107.1c/108.3/109.4/701.23a/701.23b/701.24a all grep-verified; use 701.24 not
701.18 for shuffle), nom mandate, design-boundary tests (tests.rs:31761/31809/31831 stay green).
