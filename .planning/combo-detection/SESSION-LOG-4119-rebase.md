# PR #4119 (combo-detection PR-2) — rebase + review session log

## Session start: 2026-06-22
- Lease base recorded: origin/feat/combo-detect-pr2 = 3449ec14e75cefbe78d7a25631788c8e778d0e74
- upstream/main head: cf6a9a32013983c8eaf0fb0ceb3eea0519b459d3
- Disk free: 192Gi (OK)

## STEP 1 — rebase (DONE, verified non-destructive)
- PR-2's own commits: exactly ONE = 3449ec14e (065aca47e..feat/combo-detect-pr2)
- 065aca47e IS ancestor of branch (old PR-1 head, now squash-merged)
- Command: git rebase --onto upstream/main 065aca47e feat/combo-detect-pr2 → exit 0, NO conflicts
- New rebased HEAD: 7673c39a951d7a527b8becc176f335faf4a80179
- VERIFIED diff --stat upstream/main HEAD = ONLY 4 analysis files:
  - corpus_tests.rs (A, +1985), loop_check.rs (A, +578), mod.rs (M, +12/-1), resource.rs (M, +48)
- data/engine-inventory.json: UNCHANGED (empty diff). GOOD.
- PR-1 #4097 MERGED 13:19Z (8a199028d); PR-0 #4092 MERGED 11:52Z. Both in upstream/main.
- ResourceAxis confirmed present in main resource.rs (PR-1). PR-2 changes compose cleanly (additive).

## Review commentary (fetched)
- reviewDecision: CHANGES_REQUESTED. Two reviews by @matthewevans, BOTH the stack-boundary [HIGH].
- 2nd review (on 3449ec14e, 14:06Z): "Rebase/cherry-pick PR-2 onto current main" — EXACTLY what STEP 1 did.
- No inline pulls/4119/comments. Issue comments: only Gemini quota warning + prior lgray bot summary.
- Prior nits already applied: no stale CR 605.3b; next_object_id refs are legit test setup.
- => The ONLY actionable finding is the rebase, which is now resolved by STEP 1.

## STEP 2 — /engine-implementer (in progress)

## STEP 2 plan-review (general-purpose /review-engine-plan) — 1 BLOCKER raised
- Approved architecture/scope/CR-gate/OFFLINE/composition.
- BLOCKER: corpus drivers silently skip (build_board uses card_db()?) when export absent — a green "N passed" is indistinguishable from vacuous. Must PROVE drivers RAN non-vacuously.
- Nits: "40 CRs" should be 38; V2/V4 were not-yet-measured (now: clippy exit 0 MEASURED, fmt clean MEASURED).

## VERIFICATION findings (foreground cargo, Tilt down)
- fmt --all --check: exit 0 (clean).
- clippy -p engine --all-targets -- -D warnings: exit 0 (CLEAN). [bets0q4yw]
- analysis lib suite (no export): 70 passed/0 failed/0 ignored. But export-gated drivers SKIPPED vacuously.
- ROOT CAUSE of skip: symlinked main-checkout export (08:03, from commit 72d4008b4) fails from_export with:
  "unknown variant `Double`, expected one of `Times`,`Half`,`Plus`,`Minus`,`Prevent`"
  => PR #4102 (a0a58753d, IN upstream/main) refactored QuantityModification::Double -> Times{factor}.
     Main checkout (72d4008b4) PREDATES it, so its export is schema-stale vs the rebased branch.
  => NOT a PR-2 defect; purely a stale-export verification-env issue.
- FIX: regenerate a schema-matching export from the worktree (upstream/main code) via oracle-gen,
  using cached MTGJSON AtomicCards.json (symlinked from main checkout data/mtgjson).
  Then re-run corpus suite to prove drivers RUN + DISCRIMINATE.
- Diagnostic test was added + REMOVED (working tree clean, diff still only 4 analysis files).

## SCHEMA-MATCHING EXPORT regenerated (oracle-gen from worktree code)
- Symlinked cached MTGJSON AtomicCards.json/Meta.json/SetList.json from main checkout into worktree data/mtgjson/.
- Built target/tool/oracle-gen --features cli (2m20s), then: oracle-gen data --names-out ... --sidecar-dir client/public > client/public/card-data.json
- 91.7MB, 35366 names. known-tokens.toml UNCHANGED (verified; no token-catalog regen).

## CORPUS DRIVERS NOW RUN NON-VACUOUSLY (blocker resolved)
- Re-run corpus suite WITH fresh export: 18 passed, 0 failed, 0 skipped. NO "skipping" messages.
- Runtime 0.20s -> 5.33s (drivers actually build boards + drive apply()).
- corpus_cards_present_* and drive_heliod_ballista_certificate now RUN (were skipping before).

## DISCRIMINATION PROOF (non-vacuous, per global instruction)
- Temporarily ADDED the loop-closing untap step to drive_combo_14_marwyn_sword_requires_untap.
- Result: test FAILED at corpus_tests.rs:1781 panic "without the Sword untap, Marwyn stays tapped — no loop"
  => detect_loop produced Some(cert) once the loop closed => cert.is_none() assertion is LOAD-BEARING.
- Reverted mutation; working tree clean; diff still only 4 analysis files.
- This proves the revert-probes discriminate (loop-closed vs loop-not-closed) and detect_loop produces REAL certificates.
