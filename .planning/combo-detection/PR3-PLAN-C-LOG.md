# PR-3 Option C planning log (dual memory)

Task: flesh out Option C (persisted GameState loop-detection ring) — chosen by user over D+B.
Worktree: /home/lgray/vibe-coding/wt-combo-pr3 (branch feat/combo-detect-pr3 off v0.7.0).
§7/§8/§9 building blocks ALREADY implemented (uncommitted) — reuse verbatim. §10 wiring in
run_auto_pass_loop is the dead round-2 hook to dispose of.

## Measured anchors (this round, file:line)
- Resolution SEAM (shared by ALL drivers): `priority.rs handle_priority_pass_with_limit`
  (priority.rs:31). When all living pass + non-empty stack → `resolve_next_with_limit` (stack.rs:1625,
  returns `consumed`), then reset_priority (98) → Priority{active}. Lines 77-105.
- ALL drivers route through this seam:
  - per-beat apply(PassPriority): engine.rs:1719 arm → pass_priority_once_with_pipeline (399) →
    handle_priority_pass_with_limit (416). ✓
  - run_auto_pass_loop: engine.rs:1213 pass_priority_once_with_pipeline → seam. ✓
  - resolve_all_fast_forward: engine_resolve_batch.rs:116 apply_action_boundary_with_stack_limit →
    apply_action → seam. ✓
  - server MP / bench: plain apply() → seam. ✓
- DETECTION home: `reconcile_terminal_result` (engine.rs:219), called at apply_action_boundary_with_stack_limit
  :196 (after apply_action) AND :200 (after run_auto_pass_loop). Existing 0-life SBA at :228-229
  (has_pending_player_loss_sba → check_state_based_actions), ensure_game_over_if_terminal :235,
  GameOver transition :236-239. → add loop-shortcut AFTER :239, guarded !GameOver. SBAs fire FIRST
  (CR 704 before CR 732). Mutually exclusive: victim at 0 → SBA; victim >0 → shortcut.
- GameState struct: types/game_state.rs:5488. MANY `#[serde(skip)]` derived fields already (5684,5687,
  5689, etc.). eq is HAND-WRITTEN (7822); skip fields "INTENTIONALLY omitted from impl PartialEq" (5682).
- normalize_for_loop (7754): clones, zeros state_revision/next_timestamp/next_object_id/next_pip_id/
  layers_dirty/public_state_dirty. → ADD `clone.loop_detect_ring.clear()` so snapshots empty-ringed.
- loop_fingerprint (7712): O(N) hash incl player.life → USELESS for drain win pre-filter (life changes
  each cycle). Win path must NOT pre-filter by raw fingerprint (that's why §10 win find_map doesn't).
- project_out_resources (resource.rs:588): normalize_for_loop + zero life/mana/poison/counters + stack-id
  canon (§7, :742-744). Does NOT zero turn_number/phase/priority — modulo comparison still requires those
  exact (resource.rs:557-558) → cross-turn/phase samples can't falsely match. SOUND.
- §8 live_mandatory_loop_winner (loop_check.rs:181) pub(crate): cycle_start=normalized snapshot,
  cycle_end=RAW live state (firewall on raw), delta from ResourceVectors. living==2, single life-faller,
  no library/poison loss, cant-lose/cant-win firewall, detect_loop confirm WinKind::LethalDamage.
- §9 no_living_player_has_meaningful_priority_action (engine.rs:508, private fn): probes EACH living
  player as priority holder (clears auto_pass, sets waiting_for=Priority{p}), legal_actions +
  has_meaningful_priority_action. NON-VACUOUS: castable instant → CastSpell legal → `_ => true`
  (ai_support/mod.rs:723-730) → meaningful → gate returns false → blocks shortcut. Excludes pass +
  non-meaningful mana abilities.
- §10 win wiring (engine.rs:1261-1305): find_map over loop_window LOCAL calling live_mandatory_loop_winner
  + §9 gate + GameOver. UNREACHABLE for idx 17/18 (run_auto_pass_loop sessions die on refill). DISPOSE: remove.
  KEEP strict CR 104.4b DRAW block (1230-1259) + loop_window local UNTOUCHED.

## Serialization surface (Explore-verified)
- Full GameState serialized DIRECTLY (serde_json) for: WASM export/restore (engine-wasm lib.rs:1113/1132/
  1187), server restore (session.rs:621), phase-ai saved_state. MP server→client: filter_state_for_player
  returns a FULL GameState (filter.rs:7), broadcast directly (no separate view type). WASM→JS:
  ClientGameState{state:GameState, derived} (derived_views.rs:228).
- `#[serde(skip)]` field INVISIBLE to ALL of these. engine-inventory.json = enum variants ONLY
  (engine-inventory-gen main.rs:106 Item::Enum) → struct field = NO change.
- GameState IS Send+Sync (derived). RNG=ChaCha20Rng (Send+Sync), already #[serde(skip,default=default_rng)],
  rehydrated from rng_seed on deserialize (8426). No existing Arc/Rc. AI search SEQUENTIAL (no rayon/threads),
  clones extensively (phase-ai 89,129,264,275). im::HashMap/im::Vector = O(1) structural-sharing clone.
  ⇒ `Arc<GameState>` viable; even plain GameState clone is cheap (im sharing).

## Corpus (Explore-verified)
- idx17=Sanguine+Exquisite (gated None, LethalDamage, Drain) :238. idx18=Blight-Priest+Conqueror
  (gated None, LethalDamage, Drain, FULLY NON-TARGETED) :245. idx19=Niv+Curiosity (DrawDamage) :252 excluded.
- DRIVEN_ROW_INDICES = [0,1,4,6,9,10,12,13,14,49] (:1927) = OFFLINE detect_loop driver set (LoopProbe +
  assert certificate, run_combo :1091). NEITHER 17 nor 18 in it. confirmed_drivers_match_expected (:1942)
  asserts gated_on.is_none() only. DRAIN FEEDBACK bucket doc (:1908) names 17/18 "bespoke driver follow-up".

## CR greps (verified docs/MagicCompRules.txt)
- 732.2a @6372: priority holder may shortcut a predictable sequence (loop) ending at a priority point;
  no conditional actions. 732.4 @6383: loop of ONLY MANDATORY actions = DRAW (the untouched strict path).
  732.5 @6385: no player forced past an available loop-ending action (the §9 gate). 704.3 @5485: SBAs
  checked whenever a player would get priority. 704.5a @5492: 0-or-less life loses. 104.4b @366 mandatory
  draw (strict, untouched).
- RECONCILIATION: net-progress drain ≠ 732.4 draw (that's net-zero). It's a 732.2a shortcut to the
  predictable 704.5a loss. The strict 104.4b/732.4 block fires on net-zero; the win block on net-progress.

## DESIGN DECIDED
- Field: `#[serde(skip, default)] pub loop_detect_ring: VecDeque<Arc<GameState>>` (normalized snapshots).
  Zero serialized surface; transient, rebuilt from play; server-authoritative determinism. Omit from eq.
- Maintenance: at seam (priority.rs), AFTER reset_priority. Push Arc::new(normalize_for_loop) when
  refilled (stack.len()_after >= _before) && Priority && non-empty; else clear. Cheap REFILL GATE keeps
  ring empty in normal play (shrinking resolutions skip). Cap=16, pop_front evict. Also clear at
  apply_action entry on any non-PassPriority action.
- Detection: reconcile_terminal_result after :239, guard !GameOver && Priority && non-empty stack &&
  !ring.is_empty(). find_map ring→live_mandatory_loop_winner(prior, state_raw, delta); then §9 gate;
  emit GameOver{Some(winner)} + handle_game_over_transition. Reuses GameOver variant — no new GameEvent.
- §10 win wiring removed; strict draw block kept untouched. No visibility bumps (detection in engine.rs
  where §9 private fn + live_mandatory_loop_winner pub(crate) both reachable).

## Soundness for per-beat drive
- §9 gate is the ENTIRE firewall here (no run_auto_pass_loop "mandatory by construction"). Probes every
  living player; if victim has a castable response → blocks. CR 732.2a/732.5 legitimacy; net-progress
  anticipates 704.5a; SBAs fire first. Board-equality (cross-turn can't match) + single-faller +
  cant-lose firewall complete the proof.

STATUS: plan written to PR3-PLAN.md + sent to main. DONE.
