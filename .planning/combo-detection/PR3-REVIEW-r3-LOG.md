# PR-3 Option C plan review (r3) — independent adversarial gate

Reviewer: independent (did NOT write plan). Auditing Option C (GameState detection ring) against ACTUAL code in wt-combo-pr3.

## Worktree state (measured)
- Modified (building blocks §7/§8/§9/§10 implemented): loop_check.rs, resource.rs, engine.rs, sba.rs
- NOT modified yet (the NEW Option C surface to be added): game_state.rs (the ring field, normalize_for_loop clear, PartialEq), priority.rs (maintenance seam).
- So §1/§2/§3 (ring field, maintenance, reconcile detection) + §6 (remove §10 win) are the proposed-new work under review.

## STATUS: REVIEW COMPLETE

**VERDICT: CHANGES REQUIRED — 0 blockers, 0 high, 1 MEDIUM (comment-only), 4 LOW.**
Core design is soundness-CLEAN and approvable; the only required change is a documentation/precision fix (the MED). serde-skip MP/replay determinism = SOUND. No false-positive vector in the refill gate. SBA ordering = correct. §9 firewall = non-vacuous. (Full reasoning below; itemized findings in the FINDINGS section.)

## MEASURED (wt-combo-pr3)
- All drivers reach BOTH seam + detection: apply()→apply_action_boundary_with_stack_limit→reconcile(196/200); resolve_all_fast_forward→boundary(116); server session.rs:1092/1122/1207→apply(). Maintenance seam = priority.rs handle_priority_pass_with_limit (resolution branch 77-105).
- reconcile_terminal_result(219-240): 704.5a SBA FIRST (228-229), then ensure_game_over(235), GameOver transition(236-239). §3 appends AFTER 239 w/ !GameOver guard ⇒ never preempts/double-fires; anticipates only FUTURE loss. Winner identical to SBA (CR 104.2a).
- run_auto_pass_loop default arm `_ => break` (1375) ⇒ GameOver from line-196 reconcile breaks the loop ⇒ line-200 reconcile skips (idempotent).
- §9 firewall non-vacuous: has_meaningful_priority_action (ai_support/mod.rs:723) `_ => true` for any CastSpell/meaningful-activate/sac-for-mana; no_living_player_... (471-480) probes EACH living player w/ fresh Priority{p}. Victim w/ castable instant ⇒ false ⇒ suppress.
- project_out_resources (resource.rs:588) re-normalizes BOTH inputs (589) ⇒ normalized-prior-vs-raw-state asymmetry is moot; firewall uses RAW cycle_end (live statics). objects_content_eq compares full per-object content. SOUND.
- normalize_for_loop (7754) preserves players(life)/objects/stack; zeros only revision/timestamp/next_object_id/next_pip_id/dirty. §1.2.1 clear of ring is REQUIRED + correctly specified.
- GameState: manual impl PartialEq (7822) ⇒ new field auto-EXCLUDED ("do nothing" correct). Send+Sync pinned at compile time (7117). derive(...,Serialize,Deserialize) ⇒ #[serde(skip,default)] valid; static_source_index(5685) is the real skip+eq-excluded precedent.
- Server authoritative (apply()); guest gets filtered GameState (filter.rs:7), renders only. serde-skip ⇒ ring never crosses wire. Winner always from independent 704.5a SBA ⇒ ring loss only DEFERS, never changes winner. MP/replay determinism SOUND.
- CR greps resolve: 732.2a/732.4/732.5/704.3/704.5a/104.4b/104.2a/810.8a(correctly EXCLUDED in firewall).

## FINDINGS
- [MED] Plan eq-exclusion justification cites WRONG fields: §1.1 doc + §1.2.2 say public_state_dirty/state_revision/layers_dirty are eq-omitted, but ALL are COMPARED (7848/7857/7858). Correct precedent = static_source_index/static_gate_truth/devour_eligible_snapshot. Comment-only fix; behavior correct.
- [LOW] §2.3 clear must go after ALL preference early-returns (CancelAutoPass 1507, SetPhaseStops 1522, ReorderHand 1540, Debug); plan names only 2. Harmless if it clears on a no-op anyway.
- [LOW] Step-0 is FIRST live exercise of the modulo-match win (the §10 path was NEVER live per IMPL-STOP; U1-U10 use start=end.clone). Must OBSERVE real GameOver before promoting DRIVEN_ROW_INDICES. Plan mandates this.
- [LOW/info] Pure net-ZERO mandatory DRAW still undetected under per-beat drive (run_auto_pass_loop breaks immediately ⇒ DRAW block never runs). PRE-EXISTING; Option C adds only WIN path. Out of scope per plan.
- [LOW] 732.2a is player-SUGGESTED shortcut; engine automating a forced no-action loop is sound (704.5a = win authority, 732.5 = §9 gate). Framing accurate; no change.

VERDICT: 0 blockers, 0 high, 1 med (comment-only), 4 low. Design SOUND + realizable as specified.
