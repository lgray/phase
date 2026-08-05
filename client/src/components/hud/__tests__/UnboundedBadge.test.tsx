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
const SCHEDULED_TOKENS = "Unbounded tokens (∞) — collapse pending; you'll choose how many";

// COMPOSED axis literals. `ResourceAxis` is externally tagged: unit variants are bare strings,
// data variants are single-key objects — both shapes appear below on purpose.
const TOKENS = "TokensCreated" as UnboundedResourceView["axis"];
const CHARGE = { Counter: ["Other", "Other"] } as unknown as UnboundedResourceView["axis"];
const BURDEN = { Counter: ["Other", "Generic"] } as unknown as UnboundedResourceView["axis"];
const row = (axis: UnboundedResourceView["axis"], player = 0): UnboundedResourceView => ({
  player,
  axis,
});

describe("unboundedFamilyViews", () => {
  // No store, no mount ⇒ no `beforeEach` reset and no `afterEach(cleanup)` needed here.

  it("U3/join: joins by (player, axis), not by the tag being non-empty", () => {
    // COMPOSED — neither golden carries two axes.
    const views = unboundedFamilyViews([row(TOKENS), row(CHARGE)], [row(CHARGE)]);
    expect(views).toHaveLength(2);
    expect(views.find((v) => v.family === "tokens")?.scheduled).toBe(false);
    expect(views.find((v) => v.family === "counters")?.scheduled).toBe(true);
  });

  // U3 above holds every row at the default seat, so it passes identically whether the join keys
  // on `axis` or on `(player, axis)` — it never discriminated the "(player, axis)" half of its own
  // name. This row does. Review found the code keyed on axis ALONE while three separate docs
  // specified `(player, axis)`, so an unfiltered tag for seat 1 marked seat 0's badge scheduled.
  // Not reachable through today's callers — all four pre-filter by seat via
  // `usePlayerDesignations` — which is precisely why it belongs at the function's own level and
  // not at the hook's: an unreachable-today contract violation is still what the fifth caller
  // walks into. Reverting the key to `JSON.stringify(s.axis)` reds this row alone.
  it("U3c/seat: a tag for ANOTHER seat must not schedule this seat's badge", () => {
    const views = unboundedFamilyViews([row(TOKENS, 0)], [row(TOKENS, 1)]);
    expect(views).toEqual([{ family: "tokens", scheduled: false }]);

    // Matched pair — same axis, SAME seat, so the seat is the only thing that differs between the
    // two halves. Without this, a join that never schedules anything satisfies the assertion above.
    const same = unboundedFamilyViews([row(TOKENS, 1)], [row(TOKENS, 1)]);
    expect(same).toEqual([{ family: "tokens", scheduled: true }]);
  });

  it("U3b/structural: matches a data-variant axis structurally, not by reference", () => {
    // COMPOSED — object-shaped axis on BOTH sides, built as two DISTINCT objects so a `===`
    // join can never match them. The committed counter golden really does carry
    // `{"Counter":["Other","Other"]}`, so a reference join would silently under-report every
    // data-variant axis while the string-shaped `TokensCreated` rows kept working.
    const views = unboundedFamilyViews(
      [row({ Counter: ["Other", "Other"] } as unknown as UnboundedResourceView["axis"])],
      [row({ Counter: ["Other", "Other"] } as unknown as UnboundedResourceView["axis"])],
    );
    expect(views).toEqual([{ family: "counters", scheduled: true }]);
  });

  it("U5/rows-drive: no ∞ rows ⇒ no views, even with a non-empty tag", () => {
    // ENGINE-REACHABLE, not a mere prop contract: `derive_views` drops an object-growth ∞ row
    // whose entire registered backing set has left the battlefield while the accepted collapse
    // stash — and therefore its `scheduled_collapse` tag — survives, because the boundary still
    // cashes that axis out. Produced through the production `zones::move_to_zone` chokepoint and
    // witnessed by
    // `combo_infinite_pile::object_growth_infinity_row_dies_with_its_last_pile_member`. This row
    // pins the display consequence: an orphan tag renders nothing.
    expect(unboundedFamilyViews([], [row(TOKENS)])).toEqual([]);
    // PINNED POSITIVE, in the same `it`: without it a constant `return []` satisfies the row.
    const present = unboundedFamilyViews([row(TOKENS)], [row(TOKENS)]);
    expect(present).toHaveLength(1);
    expect(present[0].family).toBe("tokens");
    expect(present[0].scheduled).toBe(true);
  });

  it("U6/fold: a family is scheduled iff ANY member axis is, in either row order", () => {
    // COMPOSED — two distinct `Counter(..)` axes in one family, only one tagged. A last-wins
    // fold gets one order right and the other wrong, which is why both are asserted.
    for (const rows of [
      [row(CHARGE), row(BURDEN)],
      [row(BURDEN), row(CHARGE)],
    ]) {
      const views = unboundedFamilyViews(rows, [row(CHARGE)]);
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

  it("U2/unscheduled: the same golden without the tag renders plain ∞", () => {
    // GOLDEN-DRIVEN, matched negative — identical rows, tag removed.
    const { scheduled_collapse: _tag, ...untagged } = tokenWire as unknown as DerivedViews;
    seed(untagged);
    expect(screen.getAllByLabelText(/Unbounded/)).toHaveLength(1);
    const badge = screen.getByLabelText(PLAIN_TOKENS);
    expect(badge).toBeInTheDocument();
    expect(badge.textContent).toContain("∞");
    expect(badge.textContent).not.toContain("∞→N");
  });

  it("U4/viewer: a tag on another seat does not schedule this seat's badge", () => {
    // COMPOSED — no golden carries a player-1 row. The local seat (0) owns the ∞ row; only
    // player 1 is tagged, so seat 0's badge must stay unscheduled. This is the only row that
    // exercises the hook's per-player filter, which is why it stays render-level.
    seed({
      unbounded_resources: [row(TOKENS, 0)],
      scheduled_collapse: [row(TOKENS, 1)],
    } as DerivedViews);
    expect(screen.getAllByLabelText(/Unbounded/)).toHaveLength(1);
    expect(screen.getByLabelText(PLAIN_TOKENS)).toBeInTheDocument();
    expect(screen.queryByLabelText(SCHEDULED_TOKENS)).toBeNull();
  });
});
