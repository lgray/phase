/**
 * The scheduled-collapse ∞ badge: `unboundedFamilyViews` (the pure family dedup + tag join) and
 * the rendered `UnboundedBadge` it feeds.
 *
 * DATA SOURCE IS LABELLED PER ROW. The two engine-emitted goldens
 * (`unbounded-token-wire.json`, `unbounded-counter-wire.json`) each carry exactly ONE axis, one
 * player-0 row and a matching `scheduled_collapse`, so only the two GOLDEN-DRIVEN rows below can
 * come from them; every multi-axis, cross-player or rows-empty case is COMPOSED against the
 * exported prop contract and says so.
 */
import { act } from "react";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { DerivedViews, UnboundedResourceView } from "../../../adapter/types.ts";
import { useGameStore } from "../../../stores/gameStore.ts";
import { useMultiplayerStore } from "../../../stores/multiplayerStore.ts";
import { buildGameState } from "../../../test/factories/gameStateFactory.ts";
import tokenWire from "../../../test/fixtures/unbounded-token-wire.json";
import { PlayerHud } from "../PlayerHud.tsx";
import { unboundedFamilyViews } from "../HudBadges.tsx";

const PLAIN_TOKENS = "Unbounded tokens (∞)";
// Passive voice on purpose: the badge renders on opponent HUDs, and a victim-attributed axis puts
// it on the victim's seat while the loop's CONTROLLER is the one prompted to name N — so any
// second-person phrasing here is addressed to the wrong player.
const SCHEDULED_TOKENS = "Unbounded tokens (∞) — collapse pending; a finite amount will be chosen";

// COMPOSED axis literals. `ResourceAxis` is externally tagged: unit variants are bare strings,
// data variants are single-key objects — both shapes appear below on purpose.
const TOKENS = "TokensCreated" as UnboundedResourceView["axis"];
const CHARGE = { Counter: ["Other", "Other"] } as unknown as UnboundedResourceView["axis"];
const BURDEN = { Counter: ["Other", "Generic"] } as unknown as UnboundedResourceView["axis"];
const POISON = { Poison: 0 } as unknown as UnboundedResourceView["axis"];
const row = (
  axis: UnboundedResourceView["axis"],
  player = 0,
  scheduled?: boolean,
): UnboundedResourceView => ({ player, axis, ...(scheduled === undefined ? {} : { scheduled }) });

describe("unboundedFamilyViews", () => {
  // No store, no mount ⇒ no `beforeEach` reset and no `afterEach(cleanup)` needed here.
  //
  // These rows used to test a client-side `(player, axis)` join against `scheduled_collapse`.
  // That join is gone: both channels key on the engine's ATTRIBUTION player, so two controllers
  // draining one victim produce identically-keyed rows that must still disagree about `scheduled`
  // — unrepresentable by any key join. The engine now answers it on the controller key, and the
  // discriminating fixture lives where the controller identity does:
  // `derived_views::tests::two_controllers_draining_one_victim_do_not_cross_schedule`.
  // What remains testable here is only the family fold.

  it("U3/fold: carries each row's engine flag onto its family", () => {
    // COMPOSED — neither golden carries two axes.
    const views = unboundedFamilyViews([row(TOKENS), row(CHARGE, 0, true)]);
    expect(views).toHaveLength(2);
    expect(views.find((v) => v.family === "tokens")?.scheduled).toBe(false);
    expect(views.find((v) => v.family === "counters")?.scheduled).toBe(true);
  });

  it("U3b/absent: an omitted `scheduled` is falsy, not scheduled", () => {
    // The engine omits the field when false (`skip_serializing_if`), so it arrives `undefined`.
    // A truthiness bug here would render `∞→N` for every unscheduled loop on the board — the
    // single most visible way this badge could lie. Matched positive prevents a constant `false`.
    expect(unboundedFamilyViews([row(TOKENS)])).toEqual([{ family: "tokens", scheduled: false }]);
    expect(unboundedFamilyViews([row(TOKENS, 0, true)])).toEqual([
      { family: "tokens", scheduled: true },
    ]);
  });

  it("U3c/over-report: one scheduled axis marks its whole family, including a different axis", () => {
    // The documented over-report, asserted rather than only described. `Counter` and `Poison` both
    // map to the "counters" family, so a scheduled counter axis paints an unscheduled poison axis
    // — a genuinely different axis, not another `Counter(..)`. This pins the CHOSEN direction of
    // imprecision: if the fold were ever tightened to "iff ALL", this row reds and the doc comment
    // above `families.set` must change with it.
    const views = unboundedFamilyViews([row(CHARGE, 0, true), row(POISON)]);
    expect(views).toEqual([{ family: "counters", scheduled: true }]);
  });

  it("U5/rows-drive: no ∞ rows ⇒ no views", () => {
    // ENGINE-REACHABLE, not a mere prop contract: `derive_views` drops a TOKEN-axis ∞ row whose
    // entire registered pile has left the battlefield while the accepted collapse stash — and
    // therefore its `scheduled_collapse` tag — survives, because the boundary still cashes that
    // axis out. (Token axis specifically: the engine has no per-axis backing authority for counter
    // axes yet, so this mechanism cannot orphan a counter tag.) Produced through the production
    // `zones::move_to_zone` chokepoint and witnessed by
    // `combo_infinite_pile::object_growth_infinity_row_dies_with_its_last_pile_member`, whose
    // subject arm asserts both halves — tag present, row absent. This row pins the display
    // consequence: an orphan tag renders nothing.
    expect(unboundedFamilyViews([])).toEqual([]);
    // PINNED POSITIVE, in the same `it`: without it a constant `return []` satisfies the row.
    const present = unboundedFamilyViews([row(TOKENS, 0, true)]);
    expect(present).toHaveLength(1);
    expect(present[0].family).toBe("tokens");
    expect(present[0].scheduled).toBe(true);
  });

  it("U6/fold: a family is scheduled iff ANY member axis is, in either row order", () => {
    // COMPOSED — two distinct `Counter(..)` axes in one family, only one scheduled. A last-wins
    // fold gets one order right and the other wrong, which is why both are asserted.
    for (const rows of [
      [row(CHARGE, 0, true), row(BURDEN)],
      [row(BURDEN), row(CHARGE, 0, true)],
    ]) {
      const views = unboundedFamilyViews(rows);
      expect(views).toEqual([{ family: "counters", scheduled: true }]);
    }
  });
});

describe("UnboundedBadge + usePlayerDesignations", () => {
  beforeEach(() => {
    useMultiplayerStore.setState({ activePlayerId: 0 });
    useGameStore.setState({ gameState: buildGameState() });
  });

  afterEach(() => {
    cleanup();
  });

  const seed = (derived: DerivedViews) => {
    act(() => {
      useGameStore.setState({ gameState: buildGameState({ derived }) });
    });
    render(<PlayerHud />);
  };

  it("U1/scheduled: renders ∞→N and the scheduled tooltip from the engine golden", () => {
    // GOLDEN-DRIVEN — `scheduled_collapse` and `unbounded_resources` are both read out of the
    // regenerated engine golden, never authored here.
    seed(tokenWire as unknown as DerivedViews);
    expect(screen.getAllByLabelText(/Unbounded/)).toHaveLength(1);
    const badge = screen.getByLabelText(SCHEDULED_TOKENS);
    expect(badge).toBeInTheDocument();
    expect(badge.textContent).toContain("∞→N");
  });

  it("U2/unscheduled: the same golden with the ROW flag cleared renders plain ∞", () => {
    // GOLDEN-DRIVEN, matched negative — identical rows, `scheduled` cleared on the row.
    //
    // This used to strip `scheduled_collapse` instead, and it caught its own obsolescence: after
    // the flag moved into the engine's row loop, deleting the tag channel no longer unschedules
    // anything, so the badge kept rendering `∞→N` and this row went red. That is the correct
    // behaviour and the right failure — the ROW is what the display reads now, and the tag channel
    // is the accepted-collapse contract rather than the render input. Keeping the old version
    // passing would have required re-introducing the join this change exists to delete.
    const wire = tokenWire as unknown as DerivedViews;
    const untagged = {
      ...wire,
      unbounded_resources: (wire.unbounded_resources ?? []).map(({ scheduled: _s, ...r }) => r),
    };
    seed(untagged);
    expect(screen.getAllByLabelText(/Unbounded/)).toHaveLength(1);
    const badge = screen.getByLabelText(PLAIN_TOKENS);
    expect(badge).toBeInTheDocument();
    expect(badge.textContent).toContain("∞");
    expect(badge.textContent).not.toContain("∞→N");
  });

  it("U4/viewer: another seat's SCHEDULED row does not schedule this seat's badge", () => {
    // COMPOSED. Exercises the hook's per-player filter, which is why it stays render-level.
    //
    // Rewritten when the flag moved into the engine. The previous version put an unscheduled row
    // on seat 0 and a TAG on seat 1, but the tag channel no longer reaches the render path at all,
    // so it had become vacuous — a component that never schedules anything satisfied it. The live
    // hazard now is the per-seat filter: seat 1's row genuinely carries `scheduled: true`, so if
    // `forPlayer` leaked it, seat 0 would render `∞→N` off another player's collapse.
    seed({
      unbounded_resources: [row(TOKENS, 0), row(TOKENS, 1, true)],
    } as DerivedViews);
    expect(screen.getAllByLabelText(/Unbounded/)).toHaveLength(1);
    expect(screen.getByLabelText(PLAIN_TOKENS)).toBeInTheDocument();
    expect(screen.queryByLabelText(SCHEDULED_TOKENS)).toBeNull();

    // MATCHED POSITIVE — same shape, flag on THIS seat. Without it the assertions above pass
    // against a badge that can never render `∞→N`, and the filter would be untested in the
    // direction that matters.
    cleanup();
    seed({
      unbounded_resources: [row(TOKENS, 0, true), row(TOKENS, 1)],
    } as DerivedViews);
    expect(screen.getByLabelText(SCHEDULED_TOKENS)).toBeInTheDocument();
  });
});
