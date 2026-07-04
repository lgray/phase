# P2c — "Cloak from a non-library source" — Implementation PLAN

> Driver: s25-impl-revived. Planner: p2c-planner (Opus/xhigh, read-only). Base HEAD 910c56358.
> HARD BAR (never edit): game/effects/delayed_trigger.rs, game/filter.rs, game/effects/mod.rs.

## ⚑ DRIVER DECISIONS (adjudicated on the planner's §7 open questions)

- **Q1 axis width → BINARY `object_source: Option<TargetFilter>` (APPROVED).** Drop the master plan's 3-way `CloakSource{TopOfLibrary|ChosenFromZone|Objects}`. `None` = CR 701.58a library-top (serde back-compat); `Some(filter)` = explicit-objects (tracked set / ExiledBySource). Vannifar's interactive hand-pick is DELEGATED to the existing `Effect::ChooseFromZone` + `WaitingFor::ChooseFromZoneChoice` stack (already wired in AI/frontend/visibility) — no new WaitingFor. Rationale: Stage-3 categorical boundary is sound (zone-selection is ChooseFromZone's responsibility, not Cloak's); mirrors the Manifest `profile`/`enters_under` field precedent.
- **Q2 composite lowering → APPROVED.** Vannifar mode-1 lowers to a two-effect composite (ChooseFromZone parent + Cloak sub_ability), precedent imperative.rs:10004-10024 (SearchLibrary sub_ability chain).
- **Q3 Expose → SPLIT into its own gate (Group-C).** SHIP VANNIFAR FIRST under this plan. Expose the Culprit (exile-into-face-down-pile + Shuffle.pile field + "with disguise" creature filter that does NOT exist yet + potential frozen effects/mod.rs dep) gets its OWN /engine-planner → /review-engine-plan → executor cycle AFTER Vannifar lands. Expose MUST still ship (no deferral) — it is just a separate coherent unit. Use the REUSE path (ChooseFromZone + Exile/ChangeZone(face_down)) to avoid frozen effects/mod.rs; if a new Effect variant is unavoidable → cross-driver dep to s07.
- **Q4 Shuffle hidden-info fidelity (Expose)** → deferred to Expose gate; lean = randomize exiled tracked-set order via state.rng (no modeled pile zone), grounded in CR 708.5.
- **Q5 Vannifar mode-2** (PutCounterAll + colorless filter) parses today — executor confirms at dispatch (non-blocking).

## ⚑ REVIEW OUTCOME — APPROVE-WITH-CONDITIONS (review a22b0158; executor MUST satisfy each)

Architecture CONFIRMED (binary object_source field passes the variant gate; serde(default) back-compat proven for Cryptic Coat/Ransom Note; morph::manifest_card is source-agnostic; #4904 walker touch compile-forced; CRs correct; ChooseFromZone stack fully wired). ONE material defect in the data-flow — fixed by C1/C2 (stays non-frozen).

**C1 — (BLOCKING) Resolve `object_source` from the sub_ability's `ability.targets`, NOT an unpublished `TrackedSet`.** REFUTED by review: the ChooseFromZoneChoice submit handler writes chosen cards to `cont.chain.targets` (engine_resolution_choices.rs:2280) but only calls `publish_fresh_tracked_set` when `chain_references_tracked_set(&cont.chain)` is true (:2276-2278) — and that returns FALSE for a `Effect::Cloak` continuation (walks `effect_references_tracked_set`→`Effect::Cloak.target_filter()` returns `None`, ability.rs:12314→12376). So `TrackedSet(0)` is NEVER populated for Cloak → cloaks nothing → test #4 red. The cloak.rs resolver (non-frozen) MUST resolve `object_source`'s objects from the already-populated `ability.targets` (use `effect_object_targets(filter, &ability.targets)` — the block-D gap#2 building block). Checkable: grep final cloak.rs for a TrackedSet read with no publish; run test #4.

**C2 — (BLOCKING) Do NOT edit `effects/mod.rs` (FROZEN) to make TrackedSet work.** The only way the TrackedSet approach works is teaching `effect_references_tracked_set`/`chain_references_tracked_set` (effects/mod.rs:3275,3314) about Cloak's new field — HARD BAR. C1 avoids this entirely. Also do NOT surface `object_source` through `Effect::target_filter()` (ability.rs:12314) — that enrolls Cloak into cast-time target-slot / CR 608.2b re-validation (broad blast radius for a context-ref filter). Checkable: `git diff` shows zero changes to effects/mod.rs, filter.rs, delayed_trigger.rs.

**C3 — Build the composite at the INTERCEPT level, not the bare-Effect map arm.** imperative.rs:10256 returns a bare `Effect` which "cannot express a chain — only ParsedEffectClause.sub_ability can" (in-tree comment :10006-10008). Build the ChooseFromZone-parent + Cloak-sub_ability composite at the intercept level like the SearchLibrary precedent (imperative.rs:10009-10036). Thread a hand-vs-library source discriminant through `ImperativeFamilyAst::Cloak` (currently only `{target,count}`, imperative.rs:8465) so lowering can distinguish them.

**C4 — Walker arm guarded:** `object_source` is `Option<TargetFilter>` → the ability_scan.rs:1274 arm must be `if let Some(f) = object_source { acc = acc.or(scan_target_filter(f)); }`, mirroring `ChangeTargets.forced_to` (:1254-1256) and `Manifest.enters_under` (:1268-1270), not an unconditional scan.

**C5 — Test literals assert `object_source: None` explicitly (not `..`-masked)** on the library-top round-trip (#3) so back-compat is proven, not masked. tests.rs:8681,8696 + cloak.rs:81 are compile-forced struct patterns.

**No-new-gap confirmations (review):** no second cloak hardwire (morph::cloak is the sole entry); visibility/redaction needs NO new code (standard .face_down(profile) pipeline morph.rs:398-404 + existing ChooseFromZone redaction visibility.rs:465-475 — keep a passing visibility assertion); mode-2 PutCounterAll+colorless parses (ColorCount EQ 0).

## 0. Headline classification (load-bearing verdict)
P2c is **NOT parser-only** — confirmed by end-to-end resolver trace. `Effect::Cloak` hard-wires source to library-top. A parser-only arm would compile + flip coverage while cloaking the WRONG card (library top instead of the chosen hand card) = hollow-win trap. P2c requires **source parameterization of Effect::Cloak threaded through the resolver** + the fail-closed walker touch.

## 1. Per-card

### Vannifar, Evolved Enigma (jq-verified)
```
At the beginning of combat on your turn, choose one —
• Cloak a card from your hand.
• Put a +1/+1 counter on each colorless creature you control.
```
- Trigger + modal "choose one" parse standard. Mode 2 → Effect::PutCounterAll (ability.rs:9309) + colorless-creature-you-control filter ("colorless" recognized at parser/oracle_nom/filter.rs:434). Parses today.
- **Mode-1 gap:** cloak arm (imperative.rs:8433) `all_consuming`-requires `tag("cloak the top ")` + "of your library" (imperative.rs:8437-8459). "cloak a card from your hand" fails → arm returns None → falls through to Effect::Unimplemented{name:"cloak", desc:"Cloak a card from your hand."} (fallback oracle_effect/mod.rs:18087).
- **Target lowering (CORRECTED per review — see ⚑ REVIEW CONDITIONS):** `ChooseFromZone{count:1, zone:Hand, zone_owner:You, filter:None}` (ability.rs:10312) `.sub_ability =` `Cloak{target:Controller, count:1, object_source:Some(<context-ref filter>)}`. The ChooseFromZoneChoice submit handler writes the chosen cards into the sub_ability chain's `ability.targets` (engine_resolution_choices.rs:2280); the cloak.rs resolver resolves `object_source` from THOSE `ability.targets` (via `effect_object_targets(filter, &ability.targets)` — the block-D gap#2 pattern), then calls morph::manifest_card(…, cloaked_2_2()). ⛔ NOT via `TrackedSet(0)` (never published for a Cloak continuation — that path is REFUTED and would force a frozen effects/mod.rs edit).

### Expose the Culprit (jq-verified) — SPLIT to its own gate (see Q3)
```
Choose one or both —
• Turn target face-down creature face up.
• Exile any number of face-up creatures you control with disguise in a face-down pile,
  shuffle that pile, then cloak them.
```
- Mode 1 → Effect::TurnFaceUp (ability.rs:10762). Parses today.
- Mode-2 gaps: (1) "…in a face-down pile" — exile chosen battlefield creatures face-down into a pile (only ExileTop.face_down exists today); (2) "shuffle that pile" — Effect::Shuffle resolver (shuffle.rs:40-46) shuffles a player LIBRARY only; (3) "cloak them" — fails tag("cloak the top ") → Unimplemented.
- Target lowering (Expose gate): ChooseFromZone{zone:Battlefield, up_to:true, filter:<face-up disguise creatures you control>} → sub Exile(face_down→pile) → Shuffle{pile:Some(TrackedSet)} → Cloak{object_source:Some(...)}.

## 2. Resolver trace (load-bearing) — Cloak IS hard-wired to library-top
- Dispatch: effects/mod.rs:3177 `Effect::Cloak { .. } => cloak::resolve(...)`.
- cloak.rs:22-48 — destructures Effect::Cloak{target, count}, resolves player from target (a PLAYER ref = "whose library"), loops count× calling morph::cloak(state, player, events) (cloak.rs:46).
- morph.rs:469-484 pub fn cloak — calls top_library_object(state, player) (morph.rs:474) then manifest_card(…, cloaked_2_2()).
- morph.rs:412-444 top_library_object — returns player_state.library.front(). **Library top, FIXED.**
- Resolver doc-comment (cloak.rs:14-16) admits: "first pass covers top-of-library; cloaking from hand or a face-down pile is deferred (those need a player-selected source)."
- Sibling: Effect::Manifest has the identical hardwire (manifest.rs:62 top_library_object), grew profile/enters_under FIELDS without a source axis. Reusable source-agnostic primitive = morph::manifest_card(state, player, object_id, source_id, profile, controller, events) (morph.rs:360) — accepts ANY object_id in ANY zone. Fix = give Cloak an object-set to feed manifest_card.

## 3. Files to touch (NONE frozen) — Vannifar scope

| # | File | Change |
|---|------|--------|
| 1 | types/ability.rs | Add `#[serde(default, skip_serializing_if="Option::is_none")] object_source: Option<TargetFilter>` to Effect::Cloak (:10750). |
| 2 | game/effects/cloak.rs | Branch resolver on object_source: None → existing library-top loop; Some(filter) → resolve object ids (tracked-set/ExiledBySource) and manifest_card(…, cloaked_2_2()) per object. Update destructure at :22. |
| 3 | game/ability_scan.rs | **#4904 fail-closed:** Effect::Cloak{target,count} at :1274 is exhaustive destructure (no `..`) → compile-forced. Add object_source + scan via scan_target_filter. |
| 4 | parser/oracle_effect/imperative.rs | Extend cloak alt() at :8436 with tag("cloak a card from your hand")/article forms (and later "cloak them" for Expose) beside tag("cloak the top "). At lowering (:10256) emit the composite (ChooseFromZone parent + Cloak sub_ability), mirroring SearchLibrary sub_ability chain (imperative.rs:10004-10024). |
| 5 | parser/oracle_effect/tests.rs + cloak.rs tests + new runtime test | Compile-forced literal updates at tests.rs:8681,8696 and cloak.rs:81 (exhaustive Effect::Cloak{target,count} matches); add discriminating tests (§6). |

Construction-site churn from the new field (add object_source): imperative.rs:10256, cloak.rs:81 (test), tests.rs:8681, tests.rs:8696. `#[serde(default)]` covers existing cloak cards (Cryptic Coat, Ransom Note) — back-compat.
No frozen edits: effects/mod.rs dispatch arms use `{ .. }` → object_source handled inside cloak::resolve, not a new dispatch arm.

## 4. add-engine-variant 3-stage gate — Cloak object_source field
- Stage 1 Existence: no axis expresses "cloak specific objects" (Cloak.target is a player ref; manifest_card is source-agnostic but Effect hardwires top_library_object). Extension warranted.
- Stage 2 Parameterize: add OPTIONAL field default None = CR 701.58a library-top (back-compat). Mirrors Manifest profile/enters_under field precedent. NO new Effect variant.
- Stage 3 Categorical boundary: "cloak specific objects" = same keyword action (CR 701.58a) as "cloak library top", differs only in WHICH cards → source axis on single Cloak. Interactive "chosen-from-zone" is DELEGATED to Effect::ChooseFromZone (owns zone-selection); adding a ChosenFromZone Cloak variant would duplicate ChooseFromZoneChoice's wired stack → sibling-cluster smell → REJECTED.
- **VERDICT: PASS — parameterize (binary source), reuse ChooseFromZone for interactivity.**
- engine-inventory.json: not present in worktree (no regen — tree write). Sibling reasoning stands on code: ChooseFromZone/ChooseFromZoneChoice wired: AI candidates.rs:1099, frontend client/src/components/modal/CardChoiceModal.tsx + waitingForRegistry.ts, visibility visibility.rs:122,465.
- **Walker (#4904):** ability_scan.rs:1274 no-`..` destructure → object_source compile-forces a new scan arm (scan_target_filter). Required fail-closed touch.

## 5. CR annotations (grep-verified this session)
- 701.58 / 701.58a Cloak (3779/3781) — face-down 2/2 ward {2}. On the field + resolver.
- 701.58e (3789) — "cloak multiple cards from a single library … one at a time" — count loop; library-specific (does not constrain Objects source).
- 701.58f (3791) — prohibited-entry → card isn't cloaked — manifest_card error path.
- 701.58h → rule 708 (3795) — face-down handling.
- 701.40 / 701.40a Manifest (3630/3632) — sibling hardwire ref. ⚠️ **Master plan's "CR 701.34 Manifest" is WRONG: 701.34 = Proliferate (3592). Use 701.40.**
- 701.24 / 701.24a Shuffle (3488/3490) — "shuffle a library OR a face-down pile of cards" — CONFIRMS 701.24a for Expose pile shuffle.
- 701.24e (3501) — library-shuffle triggers — pile shuffle must NOT emit these.
- 708.5 (5711) — controller may always look at own face-down permanents; can't look at opponents' → Expose shuffle = opponent-facing scramble only.

## 6. Test plan (discriminating, revert-to-red) — model on effect_cloak_rejects_unsupported_source_suffix (tests.rs:8706)
**Parser round-trip:**
1. Vannifar mode 1: parse_effect("Cloak a card from your hand") → ChooseFromZone{zone:Hand,count:1} whose sub_ability is Effect::Cloak{object_source:Some(_)} (NOT Unimplemented, NOT bare Cloak object_source:None). Revert: remove new alt() tag → Unimplemented → fail.
3. Negative sibling (source discrimination — anti-hollow-win seam): parse_effect("Cloak the top card of your library") STILL lowers to Effect::Cloak{object_source:None, target:Controller, count:1} — library-top path untouched, sources don't collapse. Extends effect_cloak_top_card (tests.rs:8674).
**Runtime (GameScenario + cast().resolve()) — MUST prove correct SOURCE cloaked:**
4. **DECISIVE — Vannifar cloaks the CHOSEN HAND CARD, not library top:** hand={creature A}; library top = distinguishable card B (e.g. Sorcery, can't turn face up). Resolve mode 1, submit SelectCards([A]) to ChooseFromZoneChoice. Assert: A on battlefield face_down, power==2, toughness==2, ward {2}, zone==Battlefield; A LEFT hand; B STILL on top of library (untouched). Revert: point object_source at library-top (or None) → B cloaked / A stays in hand → fail. This separates real fix from hollow win.
6. Back-compat: existing cloak_top_card_enters_face_down_with_ward_two (cloak.rs:70) still passes with object_source:None.

## 7. Risks / open questions — ADJUDICATED above (see ⚑ DRIVER DECISIONS)
Remaining executor-time confirms: (Q5) Vannifar mode-2 PutCounterAll+colorless runtime; back-compat serde for Cryptic Coat/Ransom Note; the composite lowering shape.

### Critical files
- crates/engine/src/types/ability.rs
- crates/engine/src/game/effects/cloak.rs
- crates/engine/src/parser/oracle_effect/imperative.rs
- crates/engine/src/game/ability_scan.rs
- crates/engine/src/game/morph.rs (source-agnostic manifest_card — reuse anchor)
