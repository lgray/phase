import { useMemo } from "react";
import type { ObjectId, UnboundedCounterView } from "../adapter/types.ts";
import { useGameStore } from "../stores/gameStore.ts";

// CR 732.2a / CR 701.34a: stable empty refs so a permanent with no unbounded counter
// (the dominant case) never re-renders on identity churn.
const EMPTY_UNBOUNDED_ROWS: ReadonlyArray<UnboundedCounterView> = [];
const EMPTY_ROWS: CounterRow[] = [];

/**
 * One renderable counter pill. `type` matches the object's `counters` map key
 * (e.g. "charge", "P1P1"); `count` is the live count; `isUnbounded` is the
 * render-time distinction, never an engine-published boolean.
 */
export interface CounterRow {
  type: string;
  count: number;
  isUnbounded: boolean;
}

/**
 * CR 732.2a / CR 701.34a: every counter pill for this object, `∞`-marked ones first.
 *
 * The single union point for "the engine's `∞` counter rows" and "the object's own
 * counters", so the four already-drifting `Object.entries(obj.counters).filter(...)`
 * spellings do not become five. Engine rows come from
 * `DerivedViews::unbounded_counters` and carry the ENGINE's count; every remaining
 * `obj.counters` entry follows as a finite row.
 *
 * WHY THE ENGINE'S COUNT AND NOT `obj.counters[type]`: an engine row may legitimately
 * have no entry in the object's map at all. A pair an accepted loop pumps from `0 -> 1`
 * is registered while the live object carries none of that counter, so the row's count
 * is `0` and there is nothing to join back to. Re-deriving a count here would also be
 * the frontend inferring game state, which this codebase forbids.
 *
 * ORDER: engine rows first, then the object-map remainder — so a permanent with both
 * marked and unmarked counters renders its `∞` pills ahead of its finite ones, where it
 * previously rendered in `obj.counters` insertion order. Deliberate; match assertions by
 * pill content, not by index.
 *
 * ZUSTAND v5 HAZARD, and why this shape is not stylistic: there is no equality argument
 * and no `shallow` default in v5 — the selector result IS React's `getSnapshot` return,
 * compared with `Object.is`. A selector that ALLOCATES therefore returns a fresh
 * reference on every store read, fails React's getSnapshot cache check, and produces
 * "The result of getSnapshot should be cached to avoid an infinite loop" plus a render
 * loop. `tsc` cannot see it. So both subscriptions below return store-owned refs or the
 * module constant, and the one allocation lives in a `useMemo` OUTSIDE them — the house
 * pattern from `useCastableZoneObjects.ts`.
 *
 * Subscribed today by exactly three render sites: `board/PermanentCard`,
 * `card/ArtCropCard`, and `card/CardPreview`'s `CardInfoPanel`. Two counter render
 * sites remain unsubscribed, each with a measured blocker:
 *   - FU-A `controls/AttackTargetPicker` (`StackLabel`) — a chip stands for N grouped
 *     objects and the mark is per object id, so it needs an `.every()` intersection
 *     threaded through `groupByName`/`AttackerStack` (mirroring `isUnboundedPile`),
 *     not a representative lookup, or it renders a FALSE `∞`.
 *   - FU-B `hud/DialogAttachmentCard` — no component test file exists for it.
 * Do not claim this hook covers every counter render site until both land.
 */
export function useUnboundedCounterRows(objectId: ObjectId): CounterRow[] {
  // Both selectors return store-owned refs / the module constant — never an allocation.
  const engineRows = useGameStore(
    (s) => s.gameState?.derived?.unbounded_counters?.[String(objectId)] ?? EMPTY_UNBOUNDED_ROWS,
  );
  const objectCounters = useGameStore((s) => s.gameState?.objects?.[String(objectId)]?.counters);

  return useMemo(() => {
    if (engineRows.length === 0) {
      if (!objectCounters) return EMPTY_ROWS;
      const finiteOnly = Object.entries(objectCounters)
        .filter((entry): entry is [string, number] => entry[1] != null)
        .map(([type, count]) => ({ type, count, isUnbounded: false }));
      return finiteOnly.length === 0 ? EMPTY_ROWS : finiteOnly;
    }
    const marked = new Set(engineRows.map((r) => r.counter));
    return [
      ...engineRows.map((r) => ({ type: r.counter, count: r.count, isUnbounded: true })),
      ...Object.entries(objectCounters ?? {})
        .filter((entry): entry is [string, number] => entry[1] != null && !marked.has(entry[0]))
        .map(([type, count]) => ({ type, count, isUnbounded: false })),
    ];
  }, [engineRows, objectCounters]);
}
