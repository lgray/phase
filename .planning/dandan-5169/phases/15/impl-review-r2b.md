# Phase 15 impl review r2b (delta, phase mode) — ad382d02f3 .. 6b8b2ace54 (PHASE_BASE 236a35f0b5)

MODEL: claude-sonnet-5-5

STATUS: DRAFT (probes pending; scratch build running)

## Read-only so far
- Diff: 4 files. `plain` now = clone-with-sub_ability-cleared fed to shared `has_no_resolution_riders`; `optional_player.is_none()` added to the shared predicate once; `scope_player_for_member` deleted, `zone_axis_admits` made pub(crate) and reused in resolve_all.
- Census (card-data, jq /home/lgray/vibe-coding/dandan-run/scratch/census15.jq): wheel-shaped chains carry NO optional_player/repeat_for/else/duration on any move/shuffle/draw node. Head nodes with `sub_link == SequentialSibling` exist: Time Spiral, Once More with Feeling, Turtles in Time (also head optional). The shared predicate checks `sub_link == ContinuationStep`, which the old gate did not -> candidate refuses these heads; flip under probe below.

## Status at handback: INCOMPLETE. No probe ran (scratch build of the candidate had not finished compiling the engine crate).
Pending probes: control green; old gate -> hostile rows red; optional_player removed -> unit row red; extra shapes + Time Spiral head via /home/lgray/vibe-coding/dandan-run/scratch/p15probe.rs (append to scratch effects/mod.rs; env-toggle old gate in scratch).

## Findings by reading / census (unprobed)
**[MED] [behavior] (suspected, probe pending)** The new `plain` feeds the HEAD node to `has_no_resolution_riders`, which requires `sub_link == ContinuationStep`; the old gate never checked it. Printed wheel heads with `sub_link == SequentialSibling` (card-data census): Time Spiral, Once More with Feeling (Turtles in Time too, but its head is also `optional`, which the old gate already refused). Shapes that were split by the base gate are now left as parsed under a shared-library format. Fix: in `plain`, also `own.sub_link = SubAbilityLink::ContinuationStep` (non-head nodes are already filtered by `continuation()`), plus a row with a SequentialSibling head.
Reachability: only Dandan has shared_zones library; Dandan supplies a fixed deck, so latent (still a flip).

## Case analysis (3): zone_axis_admits vs scope_player_for_member
Equivalent. Predicate = zone in origins && owner == seat (non-battlefield). Battlefield: licensed None -> admits(player) only, as before. Per-seat zones: zone_storage_seat(z,x)=x, so old = admits(player), new = admits(player) with no other seat sharing -> same. Shared zones: every seat shares the container, old tested owner, new tests player then every other seat, owner is among them -> same.

## Callers of the shared predicate (2)
draw.rs dealer gate, scoped_library_search shape gate, wheel gate (+ tests). Only change to the shared predicate is `optional_player.is_none()`. Census: 52 nodes / 50 cards carry optional_player, 52 have optional==true (0 flips; control: total=52 nonzero). So that addition flips no printed shape for the dealer or search gates.
