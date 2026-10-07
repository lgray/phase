# Phase 16 executor report r2

MODEL: claude-sonnet-5-5
Mode: implementation/fix (phase mode). BASE_SHA/PHASE_BASE_SHA = bbce5c6bdab3c24a727d5d5f776cda093c46b15a. START_SHA = fb7fd960e600f858c3ca57f5507402c7cbc310ee. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.

## Verdict
Done. No stop-and-return items. All evidence is PREPARATORY.

## Diff (2 scope paths, the last two of scope.nul)
- `client/src/components/animation/cardVfx/cardAnchors.ts`: local `pileSeat(zone, ownerId)` = `resolvePileSeat(useGameStore.getState().gameState, ...)`; used by ZONE_SURFACES.Library, ZONE_SURFACES.Graveyard, PROVISIONAL_SURFACES.Graveyard (all 3 `data-(library|graveyard)-pile="${...}"` keyed sites from `git grep` in this file; the other grep hits are the AnimationOverlay sites routed in r1 and the components' own attributes).
- `.../__tests__/cardAnchors.test.ts`: `shared pile anchors` describe (2 rows), store reset in afterEach.

## Worktree record
Start: HEAD == START_SHA, clean, 0 staged. End: HEAD == START_SHA, 0 staged, delta == the 2 authorized paths.

## Checks (PREPARATORY)
- `pnpm exec tsc -b --noEmit --force` rc=0; eslint on both files clean.
- `pnpm exec vitest run --coverage.enabled=false src/components/animation/cardVfx src/components/animation/__tests__`: 27 files / 322 tests pass (includes AnimationOverlay tests). (Coverage-threshold error appears only when coverage is on for a subset run.)

## Coverage map
- Claim: card-flight anchors find the shared pile's node for a non-holder-owned card. Seam: `pileSeat` in the 3 selectors. Entry: `sourceElement` / `ownNode` / `provisionalNode` (used by `resolveAim`). Test: `V16-r2: a card owned by a non-holder seat resolves to the holder's pile in every pile selector` (reach-guard: owner-seat nodes also mounted). Revert (`pileSeat` returns `ownerId`): that test FAILS (1 failed / 14 passed, run shown), restored then 15/15.
- Sibling: `V16-r2: a per-player format keeps each owner's own pile` (passes under revert by design; guards the unchanged per-player path). Pre-existing rows run with a null gameState and stay green.

## Maintainer-simulation (single row)
Authority: engine `derived.shared_piles` holder (r1). Bound/read live per call from `useGameStore.getState().gameState` at selector build time (live predicate, no snapshot). Consumers: `zoneSurface`, `provisionalNode`. Invalidation: state without descriptor -> `ownerId`. Serde/protocol impact: none.

## CR / other gates
No CR annotations added. Parser gate and field-threading sweep: not applicable (no parser, no new field).

## Risks
`pileSeat` reads the store at selector time, so it follows the committed store state, not an animation-held snapshot; AnimationOverlay r1 sites use the same source.
