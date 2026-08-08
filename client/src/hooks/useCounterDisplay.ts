import type { CounterRowView, ObjectCounterDisplay, ObjectId } from "../adapter/types.ts";
import { useGameStore } from "../stores/gameStore.ts";

// CR 732.2a / CR 701.34a: stable empty refs so an object with no counter row (the dominant
// case) never re-renders on identity churn.
const EMPTY_DISPLAY: ObjectCounterDisplay = {};
const EMPTY_PILLS: ReadonlyArray<CounterRowView> = [];

/**
 * CR 122.1 + CR 306.5c: every counter row this object renders, exactly as the engine
 * partitioned and ordered them.
 *
 * The engine's `counter_display` projection is the SINGLE authority for counter display. It
 * already dropped zero-count entries (CR 122.1 — a zero map entry is not a marker), split the
 * loyalty TOTAL out of the pill strip (CR 306.5c), deduplicated across seats, and ordered the
 * rows (`∞` first, then `CounterType` order). So this hook joins nothing, filters nothing, sorts
 * nothing, and interprets no counter type — it is one keyed lookup.
 *
 * THERE IS NO FALLBACK TO `objects[id].counters`, AND ONE MUST NOT BE ADDED — not here, not in a
 * render site, not in `groupKey`. A frame that arrives with no `derived` renders NO counter pills
 * at all, where the superseded hook still rendered the finite ones. That is the correct outcome
 * of deleting a second authority: `adapter/types.ts` states of `derived` that "Consumers MUST
 * treat absence as 'no data' and MUST NOT synthesize grouped values client-side — that's a
 * CLAUDE.md violation", and the deleted fallback was itself a standing violation of that
 * contract. The consequence is that a dropped-`derived` adapter regression — `ws-adapter.ts`
 * records a real past one — now fails VISIBLY instead of silently half-correct.
 *
 * ZUSTAND v5 HAZARD, eliminated rather than mitigated: there is no equality argument and no
 * `shallow` default in v5 — the selector result IS React's `getSnapshot` return, compared with
 * `Object.is`. A selector that ALLOCATES returns a fresh reference on every store read, fails
 * React's getSnapshot cache check, and produces "The result of getSnapshot should be cached to
 * avoid an infinite loop" plus a render loop. `tsc` cannot see it. The single selector below
 * returns only a store-owned ref or a module constant, so there is nothing left to memoize.
 *
 * SUPERSEDED — kept as a record of why, because deleting it would invite the same design again.
 * This hook's doc once prescribed an `.every()` intersection through `groupByName`/`AttackerStack`,
 * mirroring `isUnboundedPile`. That solves only the FALSE-`∞` half: `.every()` degrades a group
 * whose members disagree to `×N`, which HIDES a real `∞` and contradicts the polarity
 * `derive_views` states for this subsystem. `groupKey` instead keys on the engine's rendered rows,
 * so members that render differently never group at all — no false `∞` and no hidden real one, and
 * the fix lands at every `groupByName` consumer at once instead of per chip. `isUnboundedPile`'s
 * `.every()` stays as written: it is a fail-safe over a channel `groupKey` does not key on.
 *
 * Subscribed today by exactly three render sites: `board/PermanentCard`, `card/ArtCropCard`, and
 * `card/CardPreview`'s `CardInfoPanel`. Two counter render sites remain unsubscribed, each a
 * "wire it up", neither a hazard:
 *   - FU-A `controls/AttackTargetPicker` (`StackLabel`) reads `obj.counters` itself and renders
 *     no `∞` at all. With group splitting on the counter rows in `groupKey`, subscribing it is a
 *     safe three-line change.
 *   - FU-B `hud/DialogAttachmentCard` — a four-line conversion blocked on a missing test file.
 */
export function useCounterDisplay(objectId: ObjectId): ObjectCounterDisplay {
  return useGameStore(
    (s) => s.gameState?.derived?.counter_display?.[String(objectId)] ?? EMPTY_DISPLAY,
  );
}

/** The pill rows, in engine order. Never sort or filter the result. */
export const pillsOf = (display: ObjectCounterDisplay): ReadonlyArray<CounterRowView> =>
  display.pills ?? EMPTY_PILLS;

/**
 * The single spelling of the engine enum → render-time distinction, so the three render sites
 * cannot drift. An absent `magnitude` is the serde default, `"Finite"`.
 */
export const isUnbounded = (row?: CounterRowView): boolean => row?.magnitude === "Unbounded";
