import { cleanup, fireEvent, isInaccessible, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  AmountAssignment,
  InteractionChoice,
  InteractionChoiceId,
  InteractionId,
  InteractionResponseSpec,
  InteractionShortcutPin,
  InteractionShortcutPoint,
  InteractionShortcutPreview,
  InteractionShortcutPreviewEntry,
  ViewerInteraction,
} from "../../../adapter/generated/interaction";
import type {
  DecisionPoint,
  GameState,
  WaitingFor,
} from "../../../adapter/types.ts";
import { dispatchInteraction } from "../../../game/dispatch.ts";
import {
  buildGameState,
  buildLoopShortcutWaitingFor,
  buildRespondToShortcutWaitingFor,
} from "../../../test/factories/gameStateFactory.ts";
import { setGameStoreForTest } from "../../../test/helpers/gameStoreHelpers.ts";
import { DeclareShortcutModal, RespondToShortcutModal } from "../LoopShortcutModal.tsx";

// The pin route leaves through `dispatchInteraction`; the count-only route leaves through the
// store's own `dispatch`. Both are observed by every routing row, so a regression in either
// direction fires. Safe: none of `DialogShell`, `AmountInput`, `HudBadges`, `usePlayerId` or
// `gameStore` imports this module, so the mock cannot reach the store's own `dispatch`.
vi.mock("../../../game/dispatch.ts", () => ({
  dispatchAction: vi.fn(),
  dispatchInteraction: vi.fn(),
}));

const dispatchMock = vi.fn();

type ShortcutSpec = Extract<InteractionResponseSpec, { type: "shortcut" }>["data"];

/** The engine's published shortcut response spec, delivered on `viewerInteraction` exactly as
 *  `gameStore.legalResultState` assigns it. Defaults mirror the live publisher
 *  (`game/interaction.rs`): a Fixed window and `allow_decline: true`. */
function shortcutInteraction(
  overrides: Partial<ShortcutSpec> = {},
  // The offer's identity. Defaults to the literal every existing row was already written
  // against, so parameterizing it changes no existing row; the A→B rows below pass distinct
  // ids because a rotating id is precisely what they discriminate on.
  interactionId = "session.0.1",
  // The offer's published candidates — the choices its decision points name by id. Defaults to
  // the empty list every point-free row was written against.
  candidates: InteractionChoice[] = [],
): ViewerInteraction {
  const spec: ShortcutSpec = {
    count: { type: "fixed", data: { min: 1, max: 5, suggested: 5 } },
    points: [],
    allowDecline: true,
    preview: [],
    confirm: "explicit",
    ...overrides,
  };
  return {
    waitingForKind: { simultaneous: null, terminal: false, code: "shortcut" },
    authorizedSubmitters: [0],
    canSubmit: true,
    autoPassRecommended: false,
    opportunities: [
      {
        interactionId: interactionId as InteractionId,
        response: {
          type: "schema",
          data: { spec: { type: "shortcut", data: spec }, candidates },
        },
        surfaces: [],
        progress: { selected: 0, minimum: 1, maximum: 1, aggregate: null, confirmable: false },
      },
    ],
    attachmentFans: {},
  attachmentViews: {},
    availability: { type: "inputRequired" },
  };
}

// A ConvokeTaps decision-point with two tappable creatures (informational — the
// engine auto-taps via select_convoke_taps; the modal renders it read-only).
const convokePoint: DecisionPoint = {
  slot: { source: { ThisObject: { source_id: 40, incarnation: null } }, index: 0 },
  kind: { ConvokeTaps: { tappable: [40, 41] } },
};

// ─── Projection builders. Each row states only what it varies; the shapes themselves come from
//     the engine's published tuple table, so a row cannot quietly invent a point the projection
//     cannot mint. ──────────────────────────────────────────────────────────────────────────────
const cid = (id: string) => id as InteractionChoiceId;
const amt = (id: string, amount: number): AmountAssignment => ({ choiceId: cid(id), amount });
const fixedCount = (min: number, max: number, suggested: number): ShortcutSpec["count"] => ({
  type: "fixed",
  data: { min, max, suggested },
});

function targetsPoint(
  group: number,
  ids: string[],
  overrides: Partial<InteractionShortcutPoint> = {},
): InteractionShortcutPoint {
  return {
    group,
    kind: "targets",
    min: 1,
    max: 1,
    unique: false,
    ordered: true,
    readOnly: false,
    candidateIds: ids.map(cid),
    ...overrides,
  };
}

function mayPoint(group: number, ids: string[]): InteractionShortcutPoint {
  return {
    group,
    kind: "mayChoice",
    min: 1,
    max: 1,
    unique: true,
    ordered: false,
    readOnly: false,
    candidateIds: ids.map(cid),
  };
}

/** The two published read-only kinds — the engine mints `min == max == 0` and no candidates. */
function readOnlyPoint(group: number, kind: "convokeTaps" | "manaColor"): InteractionShortcutPoint {
  return {
    group,
    kind,
    min: 0,
    max: 0,
    unique: true,
    ordered: false,
    readOnly: true,
    candidateIds: [],
  };
}

/** A non-read-only kind this modal does not render: each is its own declaration UI. */
function unrenderablePoint(
  group: number,
  kind: "mode" | "unlessBreak",
): InteractionShortcutPoint {
  return {
    group,
    kind,
    min: 1,
    max: 1,
    unique: true,
    ordered: true,
    readOnly: false,
    candidateIds: [cid(`${kind}-0`)],
  };
}

function seatCandidate(id: string, seat: number): InteractionChoice {
  return {
    id: cid(id),
    surfaces: [{ type: "player", data: { role: "target", index: null, seat } }],
    status: { type: "available" },
  };
}

function objectCandidate(id: string, name: string | null, reference: string): InteractionChoice {
  return {
    id: cid(id),
    surfaces: [
      {
        type: "object",
        data: {
          role: "target",
          index: null,
          reference,
          name,
          zone: null,
          controller: null,
          power: null,
          tapped: null,
        },
      },
    ],
    status: { type: "available" },
  };
}

/** A may point's two published options. Their only non-summary surface is the `value` the engine
 *  publishes, which is what the control reads — never the index. */
function mayCandidates(takeId: string, declineId: string): InteractionChoice[] {
  return [
    {
      id: cid(takeId),
      surfaces: [{ type: "value", data: { role: "accept", index: null, value: "take" } }],
      status: { type: "available" },
    },
    {
      id: cid(declineId),
      surfaces: [{ type: "value", data: { role: "accept", index: null, value: "decline" } }],
      status: { type: "available" },
    },
  ];
}

function element(
  count: number,
  allocation: AmountAssignment[],
  entries: InteractionShortcutPreviewEntry[] = [],
): InteractionShortcutPreview {
  return { count, entries, allocation };
}

/** The pins of the single submission the pin route sent. Throws rather than returning a shape, so
 *  a row asserting on it cannot pass against a submission that never happened. */
function submittedPins(callIndex = 0): InteractionShortcutPin[] {
  const call = vi.mocked(dispatchInteraction).mock.calls[callIndex];
  if (!call) throw new Error("dispatchInteraction was not called");
  const response = call[0].response;
  if (response.type !== "shortcut") throw new Error(`not a shortcut submission: ${response.type}`);
  return response.data.pins;
}

const confirmButton = () => screen.getByRole("button", { name: "Take the shortcut" });
const allocationRow = (subject: string) =>
  screen.getByRole("spinbutton", { name: `Repetitions for ${subject}` });
const countBox = () => screen.getByRole("spinbutton", { name: "Number of iterations" });
/** The ranking ▲ control, whose accessible name names the row it moves — so the query matches the
 *  shape rather than a literal. `getAllByRole` returns DOM order, which is the rendered row order,
 *  and the anchors keep a row that lost its subject from still matching. */
const MOVE_EARLIER = /^Move .+ earlier$/;

/** Every control this suite's DOM implementation scores focusable, paired with the name assistive
 *  technology announces for it. The population is a computed property — a non-negative `tabIndex`
 *  as that implementation scores it, restricted to what the accessibility tree exposes — not a
 *  list of tags or roles, so a control of a shape used nowhere here yet is still inside the
 *  invariant; its root is `document.body`, the same root `screen` queries from. Disabled controls
 *  stay in: they score focusable here and the dialog announces them. `aria-hidden` subtrees stay
 *  out: they are not in the accessibility tree and carry no name obligation. Two shapes are
 *  deliberately outside it — an element made a control by an ARIA `role` alone, which no role
 *  taxonomy reachable from this file can tell apart from a live region, and
 *  `<summary>`/`[contenteditable]`, which browsers focus but this harness scores -1. The name is
 *  derived from the two ways this dialog labels a control, then checked against the real
 *  accessible-name computation, so a control named some third way cannot slip through as an empty
 *  string. */
function controlNames(): string[] {
  const controls = [...document.body.querySelectorAll<HTMLElement>("*")].filter(
    (el) => el.tabIndex >= 0 && !isInaccessible(el),
  );
  return controls.map((el) => {
    const name = (el.getAttribute("aria-label") ?? el.textContent ?? "").trim();
    expect(el).toHaveAccessibleName(name);
    return name;
  });
}

// `viewerInteraction` is ALWAYS written (null by default): `setGameStoreForTest` merges into a
// module-level store, so an unset field would leak a previous test's published spec forward.
function seed(
  waitingFor: WaitingFor,
  overrides: Partial<GameState> = {},
  viewerInteraction: ViewerInteraction | null = null,
) {
  const gameState = buildGameState({
    objects: {},
    priority_player: 0,
    waiting_for: waitingFor,
    ...overrides,
  });
  setGameStoreForTest({ gameState, waitingFor, dispatch: dispatchMock, viewerInteraction });
}

describe("LoopShortcutModal", () => {
  beforeEach(() => {
    dispatchMock.mockReset();
    dispatchMock.mockResolvedValue(undefined);
    vi.mocked(dispatchInteraction).mockReset();
    vi.mocked(dispatchInteraction).mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
  });

  // T1: the declare modal renders directly from the engine schema/certificate —
  // win_kind, iteration_count, and the read-only ConvokeTaps count. A wrong field
  // read renders a different/absent string and fails.
  it("renders the offer summary from certificate + schema (T1)", () => {
    seed(buildLoopShortcutWaitingFor({ schema: { points: [convokePoint] } }));
    render(<DeclareShortcutModal />);

    expect(screen.getByText("This loop deals lethal damage.")).toBeInTheDocument();
    expect(screen.getByText("Repeat until the game ends.")).toBeInTheDocument();
    expect(
      screen.getByText("Auto-taps up to 2 creatures for convoke each iteration."),
    ).toBeInTheDocument();
  });

  // T2: confirm dispatches the exact declare payload, echoing the schema's
  // iteration_count (UntilLethal) with template: null.
  it("dispatches DeclareShortcut echoing UntilLethal with template null (T2)", () => {
    seed(buildLoopShortcutWaitingFor());
    render(<DeclareShortcutModal />);

    fireEvent.click(screen.getByRole("button", { name: "Take the shortcut" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: { count: "UntilLethal", template: null },
    });
  });

  // T2 echo-guard: a Fixed(1) schema must dispatch count:{Fixed:1}, proving the
  // count is echoed from the schema, not a hardcoded "UntilLethal".
  it("echoes a Fixed iteration_count into the dispatch (T2 echo-guard)", () => {
    seed(buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 1 } } }));
    render(<DeclareShortcutModal />);

    fireEvent.click(screen.getByRole("button", { name: "Take the shortcut" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: { count: { Fixed: 1 }, template: null },
    });
    // §1b (`fixedCount_one`): CR 732.2b makes a proposal an upper bound, so the modal says
    // "at most" — the ruled wording. Fails against the pre-§1b catalog ("Repeat once.").
    expect(screen.getByText("Repeat at most once.")).toBeInTheDocument();
  });

  // §1b (`fixedCount_other`, CR 732.2c): post-fix the object-growth offer seeds
  // Fixed(MAX_SHORTCUT_CYCLES), and the modal echoes it verbatim — so the ceiling must render with
  // the "at most" wording. Covers the other plural leaf and the {{count}} interpolation; the
  // pre-§1b catalog renders "Repeat 1000 times." and fails.
  it("renders the ceiling with the at-most wording (§1b)", () => {
    seed(buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 1000 } } }));
    render(<DeclareShortcutModal />);

    expect(screen.getByText("Repeat at most 1000 times.")).toBeInTheDocument();
  });

  // T3: display-only — a ConvokeTaps point renders a read-only info line and NO
  // tappable-selection control (the confirm button is the only control), and
  // confirm still dispatches template: null.
  it("shows ConvokeTaps read-only with no selection control (T3)", () => {
    seed(buildLoopShortcutWaitingFor({ schema: { points: [convokePoint] } }));
    render(<DeclareShortcutModal />);

    expect(
      screen.getByText("Auto-taps up to 2 creatures for convoke each iteration."),
    ).toBeInTheDocument();
    // The only interactive controls are confirm + decline — no per-creature tap UI.
    const buttons = screen.getAllByRole("button");
    expect(buttons).toHaveLength(2);
    expect(buttons.map((b) => b.textContent)).toEqual([
      "Take the shortcut",
      "Decline the shortcut",
    ]);

    fireEvent.click(buttons[0]);
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: { count: "UntilLethal", template: null },
    });
  });

  // T3b (CR 732.2a): the declare modal offers a Decline control that dispatches the
  // payloadless DeclineShortcut — suggesting a shortcut is optional. Distinct from the
  // opponent-side Shorten; this is the controller declining their own auto-offer.
  it("dispatches DeclineShortcut on decline (T3b)", () => {
    seed(buildLoopShortcutWaitingFor());
    render(<DeclareShortcutModal />);

    fireEvent.click(screen.getByRole("button", { name: "Decline the shortcut" }));
    expect(dispatchMock).toHaveBeenCalledWith({ type: "DeclineShortcut" });
  });

  // C5a: the picker declares the count the PLAYER picked. Discriminating by construction — the
  // pre-C5 dispatch echoed `schema.iteration_count` ({Fixed:5}), and 2 is neither that, nor the
  // engine's `suggested` (5), nor either window edge (1/5), so no hardcoded value satisfies it.
  it("declares the picked count, not the engine's suggestion (C5a)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction(),
    );
    render(<DeclareShortcutModal />);

    // Opens on the ENGINE's suggested count — the frontend holds no default.
    const box = screen.getByRole("spinbutton");
    expect(box).toHaveValue("5");

    fireEvent.change(box, { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: "Take the shortcut" }));
    // COUNT ONLY, deliberately. `template` is asserted nowhere in the C5 rows: the engine refuses
    // a `template: null` declaration on a point-carrying schema (module header), so pinning the
    // whole payload here would codify a payload the engine does not accept as the end state.
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: expect.objectContaining({ count: { Fixed: 2 } }),
    });
  });

  // C5a bounds: the window is engine-owned. The steppers stop at the published max, and an entry
  // outside [min,max] declares NOTHING. The final legal entry is the paired positive reach-guard —
  // without it "never dispatched" could pass on a modal that renders no working control at all.
  it("steps inside the engine window and refuses an entry outside it (C5a bounds)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 3 } } }),
      {},
      shortcutInteraction({ count: { type: "fixed", data: { min: 1, max: 3, suggested: 2 } } }),
    );
    render(<DeclareShortcutModal />);

    const box = screen.getByRole("spinbutton");
    fireEvent.click(screen.getByRole("button", { name: "Increase the number of iterations" }));
    expect(box).toHaveValue("3");
    expect(
      screen.getByRole("button", { name: "Increase the number of iterations" }),
    ).toBeDisabled();

    fireEvent.change(box, { target: { value: "9" } });
    fireEvent.click(screen.getByRole("button", { name: "Take the shortcut" }));
    expect(dispatchMock).not.toHaveBeenCalled();

    fireEvent.change(box, { target: { value: "1" } });
    fireEvent.click(screen.getByRole("button", { name: "Take the shortcut" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: expect.objectContaining({ count: { Fixed: 1 } }),
    });
  });

  // C5a negative: a window absent from the payload renders NO picker and never invents a
  // client-chosen count — the offer's own `iteration_count` is declared verbatim. Both absent
  // shapes are covered: no interaction projection at all, and an UntilLethal offer.
  it("renders no picker without a published window (C5a negative)", () => {
    seed(buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }));
    render(<DeclareShortcutModal />);

    expect(screen.queryByRole("spinbutton")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Take the shortcut" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: expect.objectContaining({ count: { Fixed: 5 } }),
    });
    cleanup();
    dispatchMock.mockReset();

    seed(
      buildLoopShortcutWaitingFor(),
      {},
      shortcutInteraction({ count: { type: "untilLethal" } }),
    );
    render(<DeclareShortcutModal />);

    expect(screen.queryByRole("spinbutton")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Take the shortcut" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: expect.objectContaining({ count: "UntilLethal" }),
    });
  });

  // C4/§7.5: a count typed into offer A must not survive into offer B. The body is keyed on the
  // offer's `interactionId`, which the engine re-mints on every accepted action.
  //
  // ⚠ The second render MUST be `view.rerender(...)`, never a second `render(...)`. A fresh
  // `render` builds a new tree and mounts a new `DeclareShortcutOffer`, which resets `picked` on
  // the UNFIXED code too — the row would go green against the defect and prove nothing. The rows
  // above use `cleanup()` + `render()` between shapes; that is the opposite of what these need, so
  // do not "fix" these into the house idiom.
  it("starts offer B from its own suggestion, not the count typed into offer A (C4)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction(
        { count: { type: "fixed", data: { min: 1, max: 9, suggested: 5 } } },
        "session.0.1",
      ),
    );
    const view = render(<DeclareShortcutModal />);

    const box = screen.getByRole("spinbutton");
    // Positive reach-guard: the entry actually landed, so a later "not 2" cannot pass vacuously
    // by the picker never having accepted input. `type="text"` + `role="spinbutton"`, so the
    // compared value is a STRING.
    fireEvent.change(box, { target: { value: "2" } });
    expect(box).toHaveValue("2");

    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 7 } } }),
      {},
      shortcutInteraction(
        { count: { type: "fixed", data: { min: 1, max: 9, suggested: 7 } } },
        "session.0.2",
      ),
    );
    view.rerender(<DeclareShortcutModal />);

    expect(screen.getByRole("spinbutton")).toHaveValue("7");
  });

  // The hostile sibling, and it is what kills the plausible wrong fix: offer B publishes a
  // BYTE-IDENTICAL window to A and differs only in `interactionId`. A key built from the window —
  // or from any `waitingFor.data` field — passes the row above and fails this one.
  it("resets on a second offer carrying an identical window (C4 hostile)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction(
        { count: { type: "fixed", data: { min: 1, max: 9, suggested: 5 } } },
        "session.0.1",
      ),
    );
    const view = render(<DeclareShortcutModal />);

    const box = screen.getByRole("spinbutton");
    fireEvent.change(box, { target: { value: "2" } });
    expect(box).toHaveValue("2");

    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction(
        { count: { type: "fixed", data: { min: 1, max: 9, suggested: 5 } } },
        "session.0.2",
      ),
    );
    view.rerender(<DeclareShortcutModal />);

    expect(screen.getByRole("spinbutton")).toHaveValue("5");
  });

  // BL-1 (CR 732.2a), BOTH arms: Decline is offered iff the engine's `allowDecline` says so. The
  // false arm asserts Confirm is still present, so "no Decline button" cannot pass by the modal
  // having failed to render.
  it("renders Decline only when the engine allows it (BL-1)", () => {
    seed(buildLoopShortcutWaitingFor(), {}, shortcutInteraction({ allowDecline: true }));
    render(<DeclareShortcutModal />);
    expect(screen.getByRole("button", { name: "Decline the shortcut" })).toBeInTheDocument();
    cleanup();

    seed(buildLoopShortcutWaitingFor(), {}, shortcutInteraction({ allowDecline: false }));
    render(<DeclareShortcutModal />);
    expect(screen.queryByRole("button", { name: "Decline the shortcut" })).toBeNull();
    expect(screen.getByRole("button", { name: "Take the shortcut" })).toBeInTheDocument();
  });

  // The engine publishes one element per count, and the modal renders the one whose count the
  // player picked — verbatim, never rescaled. The two elements below are DELIBERATELY
  // non-proportional to their counts: a component that rescaled the count-4 element to 2 would
  // show -20, and one that ignored the picker would still show -40.
  it("renders the element matching the picked count and never rescales it", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 4 } } }),
      {},
      shortcutInteraction({
        count: { type: "fixed", data: { min: 1, max: 4, suggested: 4 } },
        preview: [
          {
            count: 2,
            entries: [
              { family: "life", player: 1, amount: -7 },
              { family: "mana", player: null, amount: 3 },
            ],
          },
          {
            count: 4,
            entries: [
              { family: "life", player: 1, amount: -40 },
              { family: "mana", player: null, amount: 12 },
            ],
          },
        ],
      }),
    );
    render(<DeclareShortcutModal />);

    expect(screen.getByText("Repeating 4 times produces:")).toBeInTheDocument();
    expect(screen.getByText("-40 life — P2")).toBeInTheDocument();
    expect(screen.getByText("12 mana")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("spinbutton"), { target: { value: "2" } });
    expect(screen.getByText("Repeating 2 times produces:")).toBeInTheDocument();
    expect(screen.getByText("-7 life — P2")).toBeInTheDocument();
    expect(screen.getByText("3 mana")).toBeInTheDocument();
    expect(screen.queryByText("-40 life — P2")).toBeNull();
    expect(screen.queryByText("-20 life — P2")).toBeNull();
  });

  // The engine samples the count window, so a count inside it may carry no element. The match is
  // exact: neither neighbour's magnitudes may leak in, and nothing may be interpolated between
  // them. The paired positive is the same spec at a count that IS published.
  it("renders no preview lines for a picked count the engine did not publish", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 4 } } }),
      {},
      shortcutInteraction({
        count: { type: "fixed", data: { min: 1, max: 4, suggested: 4 } },
        preview: [
          { count: 1, entries: [{ family: "life", player: 1, amount: -5 }] },
          { count: 4, entries: [{ family: "life", player: 1, amount: -40 }] },
        ],
      }),
    );
    render(<DeclareShortcutModal />);

    // Paired positive: the suggested count IS published and renders.
    expect(screen.getByText("-40 life — P2")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("spinbutton"), { target: { value: "3" } });
    expect(screen.queryByText(/produces:/)).toBeNull();
    expect(screen.queryByText("-5 life — P2")).toBeNull();
    expect(screen.queryByText("-40 life — P2")).toBeNull();
    expect(screen.queryByText("-15 life — P2")).toBeNull();
  });

  // An offer that publishes no magnitudes at all renders no preview block, paired against the
  // same seed carrying one element.
  it("renders no preview block when the engine published no elements", () => {
    const offer = (preview: ShortcutSpec["preview"]) =>
      shortcutInteraction({
        count: { type: "fixed", data: { min: 1, max: 4, suggested: 4 } },
        preview,
      });
    seed(buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 4 } } }), {}, offer([]));
    render(<DeclareShortcutModal />);
    expect(screen.queryByText(/produces:/)).toBeNull();

    cleanup();
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 4 } } }),
      {},
      offer([{ count: 4, entries: [{ family: "life", player: 1, amount: -40 }] }]),
    );
    render(<DeclareShortcutModal />);
    expect(screen.getByText("-40 life — P2")).toBeInTheDocument();
  });

  // T4: the respond window renders the proposal and Accept dispatches Accept.
  it("renders the proposal and dispatches Accept (T4)", () => {
    seed(buildRespondToShortcutWaitingFor());
    render(<RespondToShortcutModal />);

    expect(screen.getByText("This loop deals lethal damage.")).toBeInTheDocument();
    expect(screen.getByText("Repeat until the game ends.")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Accept" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "RespondToShortcut",
      data: { response: "Accept" },
    });
  });

  // T5: "Break out" dispatches the Shorten payload shape (placeholder at_iteration).
  it("dispatches Shorten on break out (T5)", () => {
    seed(buildRespondToShortcutWaitingFor());
    render(<RespondToShortcutModal />);

    fireEvent.click(screen.getByRole("button", { name: "Break out" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "RespondToShortcut",
      data: { response: { Shorten: { at_iteration: 1 } } },
    });
  });

  // T6 (non-vacuity): both modals self-gate — a non-matching waitingFor.type
  // renders nothing and never dispatches.
  it("renders nothing on a non-matching waitingFor type (T6)", () => {
    seed({ type: "Priority", data: { player: 0 } });

    const declare = render(<DeclareShortcutModal />);
    expect(declare.container.firstChild).toBeNull();
    cleanup();

    const respond = render(<RespondToShortcutModal />);
    expect(respond.container.firstChild).toBeNull();

    expect(dispatchMock).not.toHaveBeenCalled();
  });

  // T7 (non-vacuity + MP-safety + site-1 revert-guard): a LoopShortcut whose
  // proposer is the opponent (seat 1) renders nothing for the local seat (0)
  // and never dispatches. `turn_decision_controller: null` rules out the
  // delegated-turn branch, so the ONLY reason it null-renders is the seat gate.
  // (If the usePlayerId site-1 fix were reverted, even a proposer:0 offer would
  // null-render → T1/T2 would fail — so those tests non-vacuously cover site-1.)
  it("renders nothing for a non-actor seat (T7)", () => {
    seed(buildLoopShortcutWaitingFor({ proposer: 1 }), {
      turn_decision_controller: null,
      active_player: 0,
    });

    const { container } = render(<DeclareShortcutModal />);
    expect(container.firstChild).toBeNull();
    expect(dispatchMock).not.toHaveBeenCalled();
  });

  // ═══ The pin-ingress declaration UI ═══════════════════════════════════════════════════════
  //
  // What these rows prove: the modal SENDS what it renders. What they cannot prove: that the
  // engine accepts it — a frontend suite cannot drive the engine. That link is type-level,
  // through the generated bindings, plus the engine-side adapter-contract fixture.

  // P5-1: the dispatched pin carries the SELECTED element's published allocation verbatim. The
  // fixture's split is deliberately non-even, so no plausible client-side rule reproduces it —
  // an even split of 5 over these candidates is not [4,1].
  it("dispatches the published allocation verbatim on a pointed Fixed offer (P5-1)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 5, 5),
          points: [targetsPoint(2, ["k4", "k5", "k6"])],
          preview: [
            element(5, [amt("k4", 4), amt("k5", 1)], [{ family: "life", player: 1, amount: -5 }]),
          ],
        },
        "session.0.1",
        [seatCandidate("k4", 1), seatCandidate("k5", 2), seatCandidate("k6", 3)],
      ),
    );
    render(<DeclareShortcutModal />);

    fireEvent.click(confirmButton());

    expect(dispatchInteraction).toHaveBeenCalledWith({
      interactionId: "session.0.1",
      response: {
        type: "shortcut",
        data: {
          decision: { type: "fixed", data: { iterations: 5 } },
          pins: [
            {
              group: 2,
              choiceIds: ["k4", "k5"],
              amounts: [
                { choiceId: "k4", amount: 4 },
                { choiceId: "k5", amount: 1 },
              ],
            },
          ],
        },
      },
    });
    // Reach-guard: more than one segment went out, so an equality between two empty lists cannot
    // satisfy this row.
    expect(submittedPins()[0].amounts).toHaveLength(2);
    expect(dispatchMock).not.toHaveBeenCalled();
  });

  // P5-2: an authored distribution dispatches as authored, the per-seat LIFE lines go with it,
  // and the invariant families stay. Leg A is legs B/C's paired positive — a state change, not a
  // missing element — and the even-split button proves the gate two-way.
  it("hides the previewed life lines for an authored split and keeps the badges (P5-2)", () => {
    seed(
      buildLoopShortcutWaitingFor({
        schema: { iteration_count: { Fixed: 5 } },
        // Two axes deduping to two display families, so leg C's invariance cannot be satisfied by
        // a 1-vs-0 coincidence.
        certificate: { unbounded: [{ DamageDealt: 1 }, "TokensCreated"] },
      }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 5, 5),
          points: [targetsPoint(2, ["k4", "k5"])],
          preview: [
            element(5, [amt("k4", 4), amt("k5", 1)], [{ family: "life", player: 1, amount: -2 }]),
          ],
        },
        "session.0.1",
        [seatCandidate("k4", 1), seatCandidate("k5", 2)],
      ),
    );
    render(<DeclareShortcutModal />);

    // leg A — unedited: the published split renders, and so do its life lines.
    expect(screen.getByText("-2 life — P2")).toBeInTheDocument();
    const imgCount = screen.getAllByRole("img").length;
    expect(imgCount).toBeGreaterThan(1);
    expect(screen.queryByText(/custom distribution/i)).toBeNull();

    // leg B — authored away from the published split.
    fireEvent.change(allocationRow("P2"), { target: { value: "3" } });
    fireEvent.change(allocationRow("P3"), { target: { value: "2" } });
    expect(screen.queryByText("-2 life — P2")).toBeNull();
    expect(screen.getByText(/custom distribution/i)).toBeInTheDocument();
    fireEvent.click(confirmButton());
    expect(submittedPins()).toEqual([
      { group: 2, choiceIds: ["k4", "k5"], amounts: [amt("k4", 3), amt("k5", 2)] },
    ]);

    // leg C — the survivors: the family badges and the player's own count are untouched.
    expect(screen.getAllByRole("img")).toHaveLength(imgCount);
    expect(countBox()).toHaveValue("5");

    // Hostile sibling: clearing the edit restores the published split, so the gate is two-way.
    fireEvent.click(screen.getByRole("button", { name: "Reset to the even split" }));
    expect(screen.getByText("-2 life — P2")).toBeInTheDocument();
    vi.mocked(dispatchInteraction).mockClear();
    fireEvent.click(confirmButton());
    expect(submittedPins()).toEqual([
      { group: 2, choiceIds: ["k4", "k5"], amounts: [amt("k4", 4), amt("k5", 1)] },
    ]);
  });

  // P5-3: the same route on a one-segment allocation — a POSITIVE row, not an exception.
  it("takes the pin route with a single published victim (P5-3)", () => {
    const offer = (candidateIds: string[]) =>
      shortcutInteraction(
        {
          count: fixedCount(1, 7, 7),
          points: [targetsPoint(2, candidateIds)],
          preview: [element(7, [amt("k4", 7)])],
        },
        "session.0.1",
        [seatCandidate("k4", 1)],
      );

    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 7 } } }),
      {},
      offer(["k4"]),
    );
    render(<DeclareShortcutModal />);
    fireEvent.click(confirmButton());
    expect(submittedPins()).toEqual([{ group: 2, choiceIds: ["k4"], amounts: [amt("k4", 7)] }]);
    expect(dispatchMock).not.toHaveBeenCalled();

    // Hostile sibling: the same point with no published candidate is not renderable, so the offer
    // keeps the count-only route.
    cleanup();
    vi.mocked(dispatchInteraction).mockClear();
    seed(buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 7 } } }), {}, offer([]));
    render(<DeclareShortcutModal />);
    fireEvent.click(confirmButton());
    expect(dispatchInteraction).not.toHaveBeenCalled();
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: { count: { Fixed: 7 }, template: null },
    });
  });

  // P5-4: every non-read-only point gets a pin, each from its OWN candidate list. The engine's
  // decoder refuses a submission missing a pin for any non-read-only point.
  it("pins every non-read-only point from its own candidate list (P5-4)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 18 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 18, 18),
          points: [
            mayPoint(0, ["m0take", "m0dec"]),
            mayPoint(1, ["m1take", "m1dec"]),
            targetsPoint(2, ["k4", "k5"]),
          ],
          preview: [element(18, [amt("k4", 9), amt("k5", 9)])],
        },
        "session.0.1",
        [
          ...mayCandidates("m0take", "m0dec"),
          ...mayCandidates("m1take", "m1dec"),
          seatCandidate("k4", 1),
          seatCandidate("k5", 2),
        ],
      ),
    );
    render(<DeclareShortcutModal />);

    // Reach-guard: a full pin set cannot pass on a modal that dispatches unconditionally.
    expect(confirmButton()).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 1" }));
    expect(confirmButton()).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Decline optional ability 2" }));
    expect(confirmButton()).toBeEnabled();

    fireEvent.click(confirmButton());
    expect(submittedPins()).toEqual([
      { group: 0, choiceIds: ["m0take"], amounts: [] },
      { group: 1, choiceIds: ["m1dec"], amounts: [] },
      { group: 2, choiceIds: ["k4", "k5"], amounts: [amt("k4", 9), amt("k5", 9)] },
    ]);
    expect(dispatchMock).not.toHaveBeenCalled();

    // Hostile sibling: picking the SAME option on both points must still send two different ids,
    // which is what shows each control reads its own point's list rather than a shared index.
    vi.mocked(dispatchInteraction).mockClear();
    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 2" }));
    fireEvent.click(confirmButton());
    expect(submittedPins()).toEqual([
      { group: 0, choiceIds: ["m0take"], amounts: [] },
      { group: 1, choiceIds: ["m1take"], amounts: [] },
      { group: 2, choiceIds: ["k4", "k5"], amounts: [amt("k4", 9), amt("k5", 9)] },
    ]);
  });

  // P5-5: a point-free offer keeps the `GameAction` route, so the count-only path is shown live
  // rather than assumed so.
  it("keeps the GameAction route on a point-free offer (P5-5)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction({ count: fixedCount(1, 5, 5) }),
    );
    render(<DeclareShortcutModal />);
    fireEvent.click(confirmButton());
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: { count: { Fixed: 5 }, template: null },
    });
    expect(dispatchInteraction).not.toHaveBeenCalled();
  });

  // P5-6: the shipped object-growth class — every published point read-only. Both read-only kinds
  // are covered, so the row covers the set rather than a member.
  it("keeps the GameAction route and sends no pins when every point is read-only (P5-6)", () => {
    for (const kind of ["convokeTaps", "manaColor"] as const) {
      seed(
        buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
        {},
        shortcutInteraction({
          count: fixedCount(1, 5, 5),
          points: [readOnlyPoint(0, kind)],
          // A published preview, so `targetsControl` is not what refuses here.
          preview: [element(5, [])],
        }),
      );
      render(<DeclareShortcutModal />);
      fireEvent.click(confirmButton());
      expect(dispatchMock, kind).toHaveBeenCalledWith({
        type: "DeclareShortcut",
        data: { count: { Fixed: 5 }, template: null },
      });
      expect(dispatchInteraction, kind).not.toHaveBeenCalled();
      cleanup();
      dispatchMock.mockClear();
    }
  });

  // P5-7: the order-only branch. An UntilLethal declaration states ORDER only, so `amounts` is
  // empty and no allocation box renders. The projection publishes no preview on this count spec.
  it("declares order only on an UntilLethal offer carrying a targets point (P5-7)", () => {
    seed(
      buildLoopShortcutWaitingFor(),
      {},
      shortcutInteraction(
        { count: { type: "untilLethal" }, points: [targetsPoint(2, ["k4", "k5", "k6"])] },
        "session.0.1",
        [seatCandidate("k4", 1), seatCandidate("k5", 2), seatCandidate("k6", 3)],
      ),
    );
    render(<DeclareShortcutModal />);

    // No allocation amounts — and no count picker either, which is BASE behaviour on UntilLethal.
    // The positive control for this query is P5-1, where it finds the boxes.
    expect(screen.queryAllByRole("spinbutton")).toHaveLength(0);

    // Follow the ENTRY, not the row: move the third entry up, then up again from its NEW position.
    // [k4,k5,k6] -> [k4,k6,k5] -> [k6,k4,k5], which is neither the published order nor a
    // single-swap of it.
    fireEvent.click(screen.getAllByRole("button", { name: MOVE_EARLIER })[2]);
    fireEvent.click(screen.getAllByRole("button", { name: MOVE_EARLIER })[1]);
    expect(screen.getAllByRole("button", { name: MOVE_EARLIER })[0]).toBeDisabled();

    fireEvent.click(confirmButton());
    expect(dispatchInteraction).toHaveBeenCalledWith({
      interactionId: "session.0.1",
      response: {
        type: "shortcut",
        data: {
          decision: { type: "acceptSuggested" },
          pins: [{ group: 2, choiceIds: ["k6", "k4", "k5"], amounts: [] }],
        },
      },
    });
    expect(dispatchMock).not.toHaveBeenCalled();
  });

  // P5-8: an unrenderable point keeps the `GameAction` route AND suppresses every control on the
  // renderable points beside it — a live control whose answer the count-only branch discards is
  // worse than no control. Shape (d) is what makes `pinRoute`'s renderability conjunct an `every`
  // rather than a `some`; shape (e) is the producible construction with a perfectly renderable
  // may point alongside.
  it("keeps the GameAction route on an unrenderable point and renders no may control (P5-8)", () => {
    const shapes: Array<[string, InteractionShortcutPoint[]]> = [
      ["mode", [unrenderablePoint(0, "mode")]],
      ["unlessBreak", [unrenderablePoint(0, "unlessBreak")]],
      ["multi-position targets", [targetsPoint(2, ["k4", "k5"], { max: 2 })]],
      ["mixed targets + mode", [targetsPoint(2, ["k4"]), unrenderablePoint(3, "mode")]],
      [
        "multi-position targets beside a may point",
        [targetsPoint(2, ["k4", "k5"], { max: 2 }), mayPoint(0, ["m0take", "m0dec"])],
      ],
    ];
    for (const [label, points] of shapes) {
      seed(
        buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
        {},
        shortcutInteraction(
          { count: fixedCount(1, 5, 5), points, preview: [element(5, [amt("k4", 5)])] },
          "session.0.1",
          [seatCandidate("k4", 1), seatCandidate("k5", 2), ...mayCandidates("m0take", "m0dec")],
        ),
      );
      render(<DeclareShortcutModal />);
      // The positive control for this query is P5-4 and P5-15, where the identical query in this
      // same file FINDS these buttons on offers that do route.
      expect(screen.queryAllByRole("button", { name: /optional ability/i }), label).toHaveLength(0);
      fireEvent.click(confirmButton());
      expect(dispatchMock, label).toHaveBeenCalledWith({
        type: "DeclareShortcut",
        data: { count: { Fixed: 5 }, template: null },
      });
      expect(dispatchInteraction, label).not.toHaveBeenCalled();
      cleanup();
      dispatchMock.mockClear();
    }
  });

  // P5-9: an offer publishing no preview keeps the `GameAction` route. The isolating member has
  // one renderable targets point and nothing else differing from P5-1; the object-growth shape is
  // the shipped kilo one, over-determined and isolating nothing on its own.
  it("keeps the GameAction route when the offer publishes no preview (P5-9)", () => {
    const shapes: Array<[string, Partial<ShortcutSpec>, number]> = [
      ["isolating", { count: fixedCount(1, 5, 5), points: [targetsPoint(2, ["k4"])] }, 5],
      [
        "object-growth",
        {
          count: fixedCount(1, 1000, 1000),
          points: [targetsPoint(2, ["k4"]), targetsPoint(3, ["k5"]), readOnlyPoint(4, "manaColor")],
        },
        1000,
      ],
    ];
    for (const [label, spec, declared] of shapes) {
      seed(
        buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
        {},
        shortcutInteraction(spec, "session.0.1", [seatCandidate("k4", 1), seatCandidate("k5", 2)]),
      );
      render(<DeclareShortcutModal />);
      expect(countBox(), label).toBeInTheDocument();
      fireEvent.click(confirmButton());
      expect(dispatchInteraction, label).not.toHaveBeenCalled();
      expect(dispatchMock, label).toHaveBeenCalledWith({
        type: "DeclareShortcut",
        data: { count: { Fixed: declared }, template: null },
      });
      cleanup();
      dispatchMock.mockClear();
    }

    // MANDATORY paired positive, in the same invocation: the isolating offer with a published
    // preview added — its only difference — must still reach the pin ingress, or a conjunct that
    // refuses everything would satisfy the shapes above vacuously.
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 5, 5),
          points: [targetsPoint(2, ["k4"])],
          preview: [element(5, [amt("k4", 5)])],
        },
        "session.0.1",
        [seatCandidate("k4", 1)],
      ),
    );
    render(<DeclareShortcutModal />);
    fireEvent.click(confirmButton());
    expect(submittedPins()).toEqual([{ group: 2, choiceIds: ["k4"], amounts: [amt("k4", 5)] }]);
    expect(dispatchMock).not.toHaveBeenCalled();
  });

  // P5-10: a SECOND non-read-only targets point is unanswerable from published data — the
  // published allocation names the first point's candidates and nothing else — so the whole offer
  // keeps the count-only route rather than sending a pin the engine would answer UnknownChoice on.
  it("keeps the GameAction route when a second targets point is published (P5-10)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 5, 5),
          // Disjoint candidate sets: a shared-`effective` implementation cannot pass by
          // coincidence, because the second pin's ids are not in the first point's list.
          points: [targetsPoint(2, ["k4", "k5"]), targetsPoint(3, ["k6", "k7"])],
          preview: [element(5, [amt("k4", 3), amt("k5", 2)])],
        },
        "session.0.1",
        [
          seatCandidate("k4", 1),
          seatCandidate("k5", 2),
          seatCandidate("k6", 3),
          seatCandidate("k7", 0),
        ],
      ),
    );
    render(<DeclareShortcutModal />);
    fireEvent.click(confirmButton());
    expect(dispatchInteraction).not.toHaveBeenCalled();
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: { count: { Fixed: 5 }, template: null },
    });
  });

  // P5-11: no hand-derived bound, per surface, and the row state parses-and-rejects rather than
  // clamping. Each allocation row's ceiling is the PICKED count and moves with it.
  it("reads both windows from the engine and refuses rather than clamping (P5-11)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 5, 5),
          points: [targetsPoint(2, ["k4", "k5"])],
          preview: [
            element(5, [amt("k4", 3), amt("k5", 2)]),
            element(3, [amt("k4", 2), amt("k5", 1)]),
          ],
        },
        "session.0.1",
        [seatCandidate("k4", 1), seatCandidate("k5", 2)],
      ),
    );
    render(<DeclareShortcutModal />);

    // leg A — both windows are the engine's.
    expect(countBox()).toHaveAttribute("aria-valuemin", "1");
    expect(countBox()).toHaveAttribute("aria-valuemax", "5");
    expect(allocationRow("P2")).toHaveAttribute("aria-valuemax", "5");
    fireEvent.change(countBox(), { target: { value: "3" } });
    expect(allocationRow("P2")).toHaveAttribute("aria-valuemax", "3");
    fireEvent.change(countBox(), { target: { value: "5" } });

    // leg B — an out-of-window row entry is REFUSED and left VISIBLE, never coerced into range.
    fireEvent.change(allocationRow("P2"), { target: { value: "6" } });
    expect(allocationRow("P2")).toHaveValue("6");
    expect(allocationRow("P2")).toHaveAttribute("aria-invalid", "true");
    expect(confirmButton()).toBeDisabled();
    fireEvent.click(confirmButton());
    expect(dispatchInteraction).not.toHaveBeenCalled();
    expect(dispatchMock).not.toHaveBeenCalled();

    // Paired positive in the same row: a legal partition re-enables Confirm and is what goes out,
    // so a modal whose Confirm never enables cannot satisfy leg B.
    fireEvent.change(allocationRow("P2"), { target: { value: "4" } });
    fireEvent.change(allocationRow("P3"), { target: { value: "1" } });
    expect(confirmButton()).toBeEnabled();
    fireEvent.click(confirmButton());
    expect(submittedPins()).toEqual([
      { group: 2, choiceIds: ["k4", "k5"], amounts: [amt("k4", 4), amt("k5", 1)] },
    ]);
  });

  // P5-13: offer rotation clears every new piece of state, through the existing `key={offerId}`.
  // Two legs because the allocation and ranking controls are mutually exclusive on one offer: a
  // Fixed offer carries allocation + may, an UntilLethal one carries ranking + may.
  //
  // ⚠ Same warning as the rows above that assert offer A's typed count does not survive into
  // offer B: `view.rerender`, never a second `render`.
  it("clears the allocation, the may pick and the order on a new offer (P5-13)", () => {
    const fixedOffer = (interactionId: string) =>
      shortcutInteraction(
        {
          count: fixedCount(1, 5, 5),
          points: [mayPoint(0, ["m0take", "m0dec"]), targetsPoint(2, ["k4", "k5"])],
          preview: [element(5, [amt("k4", 3), amt("k5", 2)])],
        },
        interactionId,
        [...mayCandidates("m0take", "m0dec"), seatCandidate("k4", 1), seatCandidate("k5", 2)],
      );

    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      fixedOffer("session.0.1"),
    );
    const view = render(<DeclareShortcutModal />);

    // Positive reach-guard: the edits actually LANDED, so "back to default" cannot pass on a
    // control that never accepted input.
    fireEvent.change(allocationRow("P2"), { target: { value: "1" } });
    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 1" }));
    expect(allocationRow("P2")).toHaveValue("1");
    expect(screen.getByRole("button", { name: "Take optional ability 1" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 5 } } }),
      {},
      fixedOffer("session.0.2"),
    );
    view.rerender(<DeclareShortcutModal />);
    expect(allocationRow("P2")).toHaveValue("3");
    expect(screen.getByRole("button", { name: "Take optional ability 1" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    cleanup();

    const rankingOffer = (interactionId: string) =>
      shortcutInteraction(
        {
          count: { type: "untilLethal" },
          points: [mayPoint(0, ["m0take", "m0dec"]), targetsPoint(2, ["k4", "k5"])],
        },
        interactionId,
        [...mayCandidates("m0take", "m0dec"), seatCandidate("k4", 1), seatCandidate("k5", 2)],
      );
    seed(buildLoopShortcutWaitingFor(), {}, rankingOffer("session.0.3"));
    const rankingView = render(<DeclareShortcutModal />);

    fireEvent.click(screen.getAllByRole("button", { name: MOVE_EARLIER })[1]);
    expect(screen.getAllByText(/^P[23]$/).map((n) => n.textContent)).toEqual(["P3", "P2"]);

    seed(buildLoopShortcutWaitingFor(), {}, rankingOffer("session.0.4"));
    rankingView.rerender(<DeclareShortcutModal />);
    expect(screen.getAllByText(/^P[23]$/).map((n) => n.textContent)).toEqual(["P2", "P3"]);
  });

  // P5-14: the seat gate runs ABOVE the routing branch, so a full three-point projection renders
  // nothing for a non-actor seat. Paired with P5-4's identical projection at proposer 0.
  it("renders nothing for a non-actor seat on the pin route too (P5-14)", () => {
    seed(
      buildLoopShortcutWaitingFor({ proposer: 1, schema: { iteration_count: { Fixed: 18 } } }),
      { turn_decision_controller: null, active_player: 0 },
      shortcutInteraction(
        {
          count: fixedCount(1, 18, 18),
          points: [
            mayPoint(0, ["m0take", "m0dec"]),
            mayPoint(1, ["m1take", "m1dec"]),
            targetsPoint(2, ["k4", "k5"]),
          ],
          preview: [element(18, [amt("k4", 9), amt("k5", 9)])],
        },
        "session.0.1",
        [
          ...mayCandidates("m0take", "m0dec"),
          ...mayCandidates("m1take", "m1dec"),
          seatCandidate("k4", 1),
          seatCandidate("k5", 2),
        ],
      ),
    );

    const { container } = render(<DeclareShortcutModal />);
    expect(container.firstChild).toBeNull();
    expect(dispatchMock).not.toHaveBeenCalled();
    expect(dispatchInteraction).not.toHaveBeenCalled();
  });

  // P5-15: a bounded MAY-ONLY offer routes to the pin ingress and answers its may points, with no
  // allocation control. This is the shape the routing rule's placement decides: the
  // `targetsControl !== null` test belongs to `renderable`'s targets arm, never to `pinRoute` —
  // as a conjunct there it would send this whole class to the count-only path.
  it("routes a bounded may-only offer and answers its may points (P5-15)", () => {
    const offer = (mayIds: string[]) =>
      shortcutInteraction(
        {
          count: fixedCount(1, 18, 18),
          points: [mayPoint(0, mayIds.slice(0, 2)), mayPoint(1, mayIds.slice(2))],
          // A may-only bounded offer publishes a preview whose every element carries an EMPTY
          // allocation — there is no announced target to state one over.
          preview: [element(18, [], [{ family: "life", player: 1, amount: -9 }])],
        },
        "session.0.1",
        [...mayCandidates("m0take", "m0dec"), ...mayCandidates("m1take", "m1dec")],
      );

    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 18 } } }),
      {},
      offer(["m0take", "m0dec", "m1take", "m1dec"]),
    );
    render(<DeclareShortcutModal />);

    // Reach-guard: the modal rendered its published state rather than nothing.
    expect(screen.getByText("-9 life — P2")).toBeInTheDocument();
    // The allocation control is absent. The name filter is required: the count picker is itself a
    // spinbutton on a Fixed offer. Positive control for this query: P5-1 and P5-11 find them.
    expect(screen.queryAllByRole("spinbutton", { name: /repetitions for/i })).toHaveLength(0);

    expect(confirmButton()).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 1" }));
    expect(confirmButton()).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Decline optional ability 2" }));
    expect(confirmButton()).toBeEnabled();

    fireEvent.click(confirmButton());
    expect(dispatchInteraction).toHaveBeenCalledWith({
      interactionId: "session.0.1",
      response: {
        type: "shortcut",
        data: {
          decision: { type: "fixed", data: { iterations: 18 } },
          pins: [
            { group: 0, choiceIds: ["m0take"], amounts: [] },
            { group: 1, choiceIds: ["m1dec"], amounts: [] },
          ],
        },
      },
    });
    expect(dispatchMock).not.toHaveBeenCalled();

    // Admitted-member hunt against a mayChoice arm that ignores its own domain: the same shape
    // with both points' candidate lists emptied is not renderable, so it keeps BASE behaviour.
    cleanup();
    vi.mocked(dispatchInteraction).mockClear();
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 18 } } }),
      {},
      shortcutInteraction({
        count: fixedCount(1, 18, 18),
        points: [mayPoint(0, []), mayPoint(1, [])],
        preview: [element(18, [])],
      }),
    );
    render(<DeclareShortcutModal />);
    fireEvent.click(confirmButton());
    expect(dispatchInteraction).not.toHaveBeenCalled();
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "DeclareShortcut",
      data: { count: { Fixed: 18 }, template: null },
    });
  });

  // P5-16: `targetsControl` is computed AFTER `allocationPoint` and is null when it is absent, so
  // an UntilLethal offer with no targets point renders NO ranking control and still answers its
  // may points. Together with P5-7 this shows the ranking control follows the POINT, not the
  // count spec.
  it("renders no ranking control on an UntilLethal offer with no targets point (P5-16)", () => {
    seed(
      buildLoopShortcutWaitingFor(),
      {},
      shortcutInteraction(
        {
          count: { type: "untilLethal" },
          points: [mayPoint(0, ["m0take", "m0dec"]), mayPoint(1, ["m1take", "m1dec"])],
        },
        "session.0.1",
        [...mayCandidates("m0take", "m0dec"), ...mayCandidates("m1take", "m1dec")],
      ),
    );
    render(<DeclareShortcutModal />);

    // Positive control for this query: P5-7, where it finds the ▲ buttons on an UntilLethal offer
    // that DOES publish a targets point.
    expect(screen.queryAllByRole("button", { name: MOVE_EARLIER })).toHaveLength(0);

    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 1" }));
    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 2" }));
    fireEvent.click(confirmButton());
    expect(dispatchInteraction).toHaveBeenCalledWith({
      interactionId: "session.0.1",
      response: {
        type: "shortcut",
        data: {
          decision: { type: "acceptSuggested" },
          pins: [
            { group: 0, choiceIds: ["m0take"], amounts: [] },
            { group: 1, choiceIds: ["m1take"], amounts: [] },
          ],
        },
      },
    });
    expect(dispatchMock).not.toHaveBeenCalled();
  });

  // P5-17: a count the engine published no element for renders zeros and refuses Confirm — no
  // seeded split and no nearest match. The engine samples its window, so an interior count can
  // carry no element; authoring a partition there is what makes it a rendered state, not a dead
  // end. Sibling of the landed "renders no preview lines for a picked count the engine did not
  // publish" row, one level down.
  it("seeds nothing at a count the engine published no element for (P5-17)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 8 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 8, 8),
          points: [targetsPoint(2, ["k4", "k5"])],
          // The window's endpoints only — 5 is unsampled.
          preview: [element(1, [amt("k4", 1)]), element(8, [amt("k4", 4), amt("k5", 4)])],
        },
        "session.0.1",
        [seatCandidate("k4", 1), seatCandidate("k5", 2)],
      ),
    );
    render(<DeclareShortcutModal />);

    // leg A — at the published count the rows read the published split and Confirm is enabled, so
    // a modal that never enables cannot satisfy this row.
    expect(allocationRow("P2")).toHaveAttribute("aria-valuenow", "4");
    expect(allocationRow("P3")).toHaveAttribute("aria-valuenow", "4");
    expect(confirmButton()).toBeEnabled();

    // leg B — the gap. Nothing is seeded from anywhere.
    fireEvent.change(countBox(), { target: { value: "5" } });
    expect(allocationRow("P2")).toHaveAttribute("aria-valuenow", "0");
    expect(allocationRow("P3")).toHaveAttribute("aria-valuenow", "0");
    expect(screen.queryByText(/produces:/)).toBeNull();
    expect(confirmButton()).toBeDisabled();
    fireEvent.click(confirmButton());
    expect(dispatchInteraction).not.toHaveBeenCalled();
    expect(dispatchMock).not.toHaveBeenCalled();

    // leg C — authoring a partition there enables Confirm and dispatches it, under the count the
    // player picked.
    fireEvent.change(allocationRow("P2"), { target: { value: "3" } });
    fireEvent.change(allocationRow("P3"), { target: { value: "2" } });
    expect(screen.getByText(/custom distribution/i)).toBeInTheDocument();
    expect(confirmButton()).toBeEnabled();
    fireEvent.click(confirmButton());
    expect(vi.mocked(dispatchInteraction).mock.calls[0][0].response).toEqual({
      type: "shortcut",
      data: {
        decision: { type: "fixed", data: { iterations: 5 } },
        pins: [{ group: 2, choiceIds: ["k4", "k5"], amounts: [amt("k4", 3), amt("k5", 2)] }],
      },
    });

    // leg D — the sibling that separates "no element" from "a SHORT allocation": at count 1 an
    // element exists carrying one segment, so the zero row is dropped by the positive-parts
    // filter rather than declared.
    vi.mocked(dispatchInteraction).mockClear();
    fireEvent.change(countBox(), { target: { value: "1" } });
    expect(allocationRow("P2")).toHaveAttribute("aria-valuenow", "1");
    expect(allocationRow("P3")).toHaveAttribute("aria-valuenow", "0");
    expect(confirmButton()).toBeEnabled();
    fireEvent.click(confirmButton());
    expect(submittedPins()).toEqual([{ group: 2, choiceIds: ["k4"], amounts: [amt("k4", 1)] }]);
  });

  // P5-18: the candidate label is TOTAL over its type-closed population — the player arm, the
  // object arm's RAW name, and the name -> reference fallback. The object fixture's name is
  // deliberately a real key path in `en/game.json`, so a `t()` passthrough would be visible as
  // "Take the shortcut" instead of the raw string.
  it("labels player, object and unnamed-object candidates (P5-18)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 3 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 3, 3),
          points: [targetsPoint(2, ["k4", "k5", "k6"])],
          preview: [element(3, [amt("k4", 1), amt("k5", 1), amt("k6", 1)])],
        },
        "session.0.1",
        [
          seatCandidate("k4", 1),
          objectCandidate("k5", "comboShortcut.confirm", "obj-55"),
          objectCandidate("k6", null, "obj-77"),
        ],
      ),
    );
    render(<DeclareShortcutModal />);

    // The player arm is the paired positive: it is the arm the routed population actually reaches.
    expect(
      screen.getByRole("spinbutton", { name: "Repetitions for P2" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("spinbutton", { name: "Repetitions for comboShortcut.confirm" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("spinbutton", { name: "Repetitions for obj-77" }),
    ).toBeInTheDocument();
  });

  // P5-19: on the pin route the KEYBOARD entry point refuses in exactly the state the button
  // does, and mints no count the player did not type. `AmountInput` calls `onSubmit`
  // unconditionally on Enter and deliberately does not re-guard, so the refusal has to sit at the
  // top of the handler — a row that clicks a disabled button cannot see this.
  it("refuses on Enter in exactly the state the button refuses (P5-19)", () => {
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 18 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 18, 18),
          points: [
            mayPoint(0, ["m0take", "m0dec"]),
            mayPoint(1, ["m1take", "m1dec"]),
            targetsPoint(2, ["k4", "k5"]),
          ],
          // The published allocation already sums to the count, so only the MAY leg of
          // `declarationComplete` can be unmet in leg B.
          preview: [element(18, [amt("k4", 9), amt("k5", 9)])],
        },
        "session.0.1",
        [
          ...mayCandidates("m0take", "m0dec"),
          ...mayCandidates("m1take", "m1dec"),
          seatCandidate("k4", 1),
          seatCandidate("k5", 2),
        ],
      ),
    );
    render(<DeclareShortcutModal />);

    // leg A — an out-of-window count entry. Enter in that very box must mint nothing.
    fireEvent.change(countBox(), { target: { value: "19" } });
    fireEvent.keyDown(countBox(), { key: "Enter" });
    expect(dispatchInteraction).not.toHaveBeenCalled();
    expect(dispatchMock).not.toHaveBeenCalled();

    // leg B — a legal count with one may point unanswered, fired from BOTH box families, so a
    // repair that guards only the surface the count picker owns cannot pass.
    fireEvent.change(countBox(), { target: { value: "18" } });
    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 1" }));
    fireEvent.keyDown(countBox(), { key: "Enter" });
    fireEvent.keyDown(allocationRow("P2"), { key: "Enter" });
    expect(dispatchInteraction).not.toHaveBeenCalled();
    expect(dispatchMock).not.toHaveBeenCalled();

    // leg C — the instrument fires: with the declaration complete, Enter in the count box sends
    // the whole submission. Without this leg a modal that ignores Enter entirely satisfies A and
    // B vacuously.
    fireEvent.click(screen.getByRole("button", { name: "Decline optional ability 2" }));
    fireEvent.keyDown(countBox(), { key: "Enter" });
    expect(dispatchInteraction).toHaveBeenCalledWith({
      interactionId: "session.0.1",
      response: {
        type: "shortcut",
        data: {
          decision: { type: "fixed", data: { iterations: 18 } },
          pins: [
            { group: 0, choiceIds: ["m0take"], amounts: [] },
            { group: 1, choiceIds: ["m1dec"], amounts: [] },
            { group: 2, choiceIds: ["k4", "k5"], amounts: [amt("k4", 9), amt("k5", 9)] },
          ],
        },
      },
    });
    expect(dispatchMock).not.toHaveBeenCalled();
  });

  // P5-20: the visible-subject class — a control that asks about a SPECIFIC subject renders that
  // subject where a sighted player can read it. Every assertion is on rendered TEXT, never on an
  // accessible name: the accessible names carry the subject whether or not the visible text does,
  // so only a visible-text assertion discriminates on this property. All three members of the
  // class are driven — allocation rows, may panels and ranking rows — which takes two offers,
  // because allocation and ranking cannot coexist: the published count spec selects exactly one
  // `targetsControl` kind.
  it("renders every per-subject control's subject visibly (P5-20)", () => {
    // A — a fixed-count offer with three victims and two may points.
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 6 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 6, 6),
          points: [
            mayPoint(0, ["m0take", "m0dec"]),
            mayPoint(1, ["m1take", "m1dec"]),
            targetsPoint(2, ["k4", "k5", "k6"]),
          ],
          preview: [element(6, [amt("k4", 2), amt("k5", 2), amt("k6", 2)])],
        },
        "session.0.1",
        [
          ...mayCandidates("m0take", "m0dec"),
          ...mayCandidates("m1take", "m1dec"),
          seatCandidate("k4", 1),
          seatCandidate("k5", 2),
          seatCandidate("k6", 3),
        ],
      ),
    );
    render(<DeclareShortcutModal />);

    // Reach-guard: the allocation panel is mounted, and each box carries both names — the
    // accessible one queried here and the visible one asserted below.
    expect(allocationRow("P2")).toBeInTheDocument();
    // Each victim's box states WHICH victim it is. Drop the visible subject and these three
    // spinboxes are indistinguishable on screen, so each of these three queries fails.
    for (const seat of ["P2", "P3", "P4"]) {
      expect(screen.getByText(seat), seat).toBeInTheDocument();
    }
    // Two may panels, two DIFFERENT visible headings. `getByText` throws when more than one node
    // matches, so a call that resolves is itself the proof that the two subjects are distinct.
    expect(
      screen.getByText("Optional ability 1 — repeat this choice each iteration?"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Optional ability 2 — repeat this choice each iteration?"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Take optional ability 1" })).toBeInTheDocument();

    // B — the ranking member of the class, on the only offer shape that reaches it.
    cleanup();
    seed(
      buildLoopShortcutWaitingFor(),
      {},
      shortcutInteraction(
        {
          count: { type: "untilLethal" },
          points: [mayPoint(0, ["m0take", "m0dec"]), targetsPoint(2, ["k4", "k5", "k6"])],
        },
        "session.0.1",
        [
          ...mayCandidates("m0take", "m0dec"),
          seatCandidate("k4", 1),
          seatCandidate("k5", 2),
          seatCandidate("k6", 3),
        ],
      ),
    );
    render(<DeclareShortcutModal />);

    // Reach-guard: the ranking panel is mounted, one row per candidate.
    expect(screen.getAllByRole("button", { name: MOVE_EARLIER })).toHaveLength(3);
    for (const seat of ["P2", "P3", "P4"]) {
      expect(screen.getByText(seat), seat).toBeInTheDocument();
    }
    expect(
      screen.getByText("Optional ability 1 — repeat this choice each iteration?"),
    ).toBeInTheDocument();
  });
  // P5-21: no control in `controlNames`' population is nameless, and no two share a name. P5-20
  // closes the per-subject class on the screen; this closes it for a screen reader, which
  // navigates BY the accessible name — controls sharing one subject-free label are
  // indistinguishable there however clearly the rows read on screen. An invariant over the
  // population `controlNames` computes, not a list of today's controls: a control added later
  // that reaches for a shared subject-free label reds this row without anyone remembering to
  // extend a list. Both offers are driven because the published count spec selects exactly one
  // `targetsControl` kind, and neither branch may hand out a duplicate.
  it("gives every focusable control in the a11y tree a distinct accessible name (P5-21)", () => {
    // A — the allocation branch: a count picker and three victim rows, each an amount control
    // with two steppers, plus two may panels.
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 6 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 6, 6),
          points: [
            mayPoint(0, ["m0take", "m0dec"]),
            mayPoint(1, ["m1take", "m1dec"]),
            targetsPoint(2, ["k4", "k5", "k6"]),
          ],
          preview: [element(6, [amt("k4", 2), amt("k5", 2), amt("k6", 2)])],
        },
        "session.0.1",
        [
          ...mayCandidates("m0take", "m0dec"),
          ...mayCandidates("m1take", "m1dec"),
          seatCandidate("k4", 1),
          seatCandidate("k5", 2),
          seatCandidate("k6", 3),
        ],
      ),
    );
    render(<DeclareShortcutModal />);

    const allocationNames = controlNames();
    // Reach-guard: the enumeration reached all four amount controls, the ones whose steppers a
    // shared label would collapse onto each other, so the assertions below run over a populated
    // set rather than an empty one.
    expect(allocationNames).toEqual(
      expect.arrayContaining([
        "Decrease the number of iterations",
        "Decrease repetitions for P2",
        "Decrease repetitions for P3",
        "Decrease repetitions for P4",
      ]),
    );
    expect(allocationNames.filter((n) => n.length === 0)).toEqual([]);
    expect(new Set(allocationNames).size, allocationNames.join(" | ")).toBe(
      allocationNames.length,
    );

    // B — the ranking branch, whose two move buttons repeat once per row.
    cleanup();
    seed(
      buildLoopShortcutWaitingFor(),
      {},
      shortcutInteraction(
        {
          count: { type: "untilLethal" },
          points: [mayPoint(0, ["m0take", "m0dec"]), targetsPoint(2, ["k4", "k5", "k6"])],
        },
        "session.0.1",
        [
          ...mayCandidates("m0take", "m0dec"),
          seatCandidate("k4", 1),
          seatCandidate("k5", 2),
          seatCandidate("k6", 3),
        ],
      ),
    );
    render(<DeclareShortcutModal />);

    const rankingNames = controlNames();
    expect(rankingNames).toEqual(
      expect.arrayContaining(["Move P2 earlier", "Move P3 earlier", "Move P4 later"]),
    );
    expect(rankingNames.filter((n) => n.length === 0)).toEqual([]);
    expect(new Set(rankingNames).size, rankingNames.join(" | ")).toBe(rankingNames.length);
  });

  // P5-22: the may panel's ordinal counts the panels ON SCREEN, not the published points. The
  // projection numbers every point it publishes — the read-only ones and the targets point
  // included — and `bounded_cycle_pin_slots_for_window` pushes an accepted entry's targets point
  // BEFORE that entry's may point, so an offer whose may points do not lead is the ordinary
  // construction rather than an exotic one. Numbering by `group` would head the only panel
  // "Optional ability 2" and tell the player the dialog is withholding a choice it is obliged to
  // render. Shape A leads with a targets point, shape B with a read-only one and carries TWO may
  // panels, so the numbering is shown contiguous rather than merely offset. Both shapes assert the
  // DISPATCHED group as well: renumbering the wire instead of the display would satisfy every
  // screen assertion here and corrupt the submission.
  it("numbers the may panels by rendered position while pinning by group (P5-22)", () => {
    // A — `[Targets(0), MayChoice(1)]`, the order one accepted entry carrying both publishes.
    seed(
      buildLoopShortcutWaitingFor({ schema: { iteration_count: { Fixed: 6 } } }),
      {},
      shortcutInteraction(
        {
          count: fixedCount(1, 6, 6),
          points: [targetsPoint(0, ["k4", "k5"]), mayPoint(1, ["m1take", "m1dec"])],
          preview: [element(6, [amt("k4", 3), amt("k5", 3)])],
        },
        "session.0.1",
        [seatCandidate("k4", 1), seatCandidate("k5", 2), ...mayCandidates("m1take", "m1dec")],
      ),
    );
    render(<DeclareShortcutModal />);

    // Reach-guard: the offer took the pin route (the allocation control beside the panel is only
    // rendered there), so the single panel below is a rendered may point, not an absent one.
    expect(allocationRow("P2")).toBeInTheDocument();
    // The whole set of headings, so an "ability 1" that renders ALONGSIDE a stray "ability 2"
    // cannot pass. `getAllByText` throws on an empty match, which is the query's own control.
    expect(screen.getAllByText(/^Optional ability \d+ —/).map((n) => n.textContent)).toEqual([
      "Optional ability 1 — repeat this choice each iteration?",
    ]);
    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 1" }));
    fireEvent.click(confirmButton());
    // The published group, not the display ordinal, is what the engine is asked to pin.
    expect(submittedPins()).toEqual([
      { group: 0, choiceIds: ["k4", "k5"], amounts: [amt("k4", 3), amt("k5", 3)] },
      { group: 1, choiceIds: ["m1take"], amounts: [] },
    ]);

    // B — a read-only point leads and both may points follow.
    cleanup();
    vi.mocked(dispatchInteraction).mockClear();
    seed(
      buildLoopShortcutWaitingFor(),
      {},
      shortcutInteraction(
        {
          count: { type: "untilLethal" },
          points: [
            readOnlyPoint(0, "convokeTaps"),
            mayPoint(1, ["m1take", "m1dec"]),
            mayPoint(2, ["m2take", "m2dec"]),
          ],
        },
        "session.0.1",
        [...mayCandidates("m1take", "m1dec"), ...mayCandidates("m2take", "m2dec")],
      ),
    );
    render(<DeclareShortcutModal />);

    expect(screen.getAllByText(/^Optional ability \d+ —/).map((n) => n.textContent)).toEqual([
      "Optional ability 1 — repeat this choice each iteration?",
      "Optional ability 2 — repeat this choice each iteration?",
    ]);
    fireEvent.click(screen.getByRole("button", { name: "Take optional ability 1" }));
    fireEvent.click(screen.getByRole("button", { name: "Decline optional ability 2" }));
    fireEvent.click(confirmButton());
    // Each panel answers ITS OWN point: the ordinals shifted, the pins did not, and the read-only
    // point still receives none.
    expect(submittedPins()).toEqual([
      { group: 1, choiceIds: ["m1take"], amounts: [] },
      { group: 2, choiceIds: ["m2dec"], amounts: [] },
    ]);
  });
});
