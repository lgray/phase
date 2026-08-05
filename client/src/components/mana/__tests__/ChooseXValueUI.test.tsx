import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

import { ChooseXValueUI } from "../ChooseXValueUI";
import { DialogHost } from "../../modal/DialogHost.tsx";
import { useGameStore } from "../../../stores/gameStore";
import type { GameState, PendingCast, WaitingFor } from "../../../adapter/types";
import { buildGameObjectWithCoreTypes, buildObjectMap } from "../../../test/factories/gameObjectFactory.ts";
import {
  buildChooseXValueWaitingFor,
  buildGameState,
  buildManaPaymentWaitingFor,
  buildPendingCast,
  buildPriorityWaitingFor,
} from "../../../test/factories/gameStateFactory.ts";
import { setGameStoreForTest } from "../../../test/helpers/gameStoreHelpers.ts";

function createGameState(overrides: Partial<GameState> = {}): GameState {
  return buildGameState({
    objects: buildObjectMap(
      buildGameObjectWithCoreTypes(["Instant"], {
        id: 42,
        card_id: 1,
        name: "Nature's Rhythm",
        zone: "Stack",
      }),
    ),
    next_object_id: 100,
    waiting_for: buildManaPaymentWaitingFor(),
    has_pending_cast: true,
    ...overrides,
  });
}

function createPendingCast(): PendingCast {
  return buildPendingCast({
    object_id: 42,
    card_id: 1,
    cost: { type: "Cost", shards: ["X", "G", "G"], generic: 0 },
  });
}

function chooseXWaitingFor(max: number, min?: number): WaitingFor {
  return buildChooseXValueWaitingFor({
    data: {
      player: 0,
      max,
      ...(min === undefined ? {} : { min }),
      pending_cast: createPendingCast(),
    },
  });
}

describe("ChooseXValueUI", () => {
  beforeEach(() => {
    useGameStore.getState().reset();
  });

  afterEach(() => {
    cleanup();
  });

  it("renders nothing when not in ChooseXValue state", () => {
    setGameStoreForTest({
      gameState: createGameState(),
      waitingFor: buildPriorityWaitingFor(),
    });

    const { container } = render(<ChooseXValueUI />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows card name and dispatches ChooseX with selected value on confirm", () => {
    const dispatch = vi.fn().mockResolvedValue([]);
    const waitingFor = chooseXWaitingFor(5);

    setGameStoreForTest({
      gameState: createGameState({ waiting_for: waitingFor }),
      waitingFor,
      dispatch,
    });

    render(<ChooseXValueUI />);

    expect(screen.getByText(/Choose a value for X/)).toBeInTheDocument();
    expect(screen.getByText(/Nature's Rhythm/)).toBeInTheDocument();

    const input = screen.getByLabelText("Enter X value") as HTMLInputElement;
    // min 0 renders the one-sided hint; the engine window is shown, not the control's
    // native min/max, which a text box does not carry.
    expect(screen.getByText("max 5")).toBeInTheDocument();

    fireEvent.change(input, { target: { value: "3" } });
    fireEvent.click(screen.getByRole("button", { name: "Confirm X = 3" }));

    expect(dispatch).toHaveBeenCalledWith({ type: "ChooseX", data: { value: 3 } });
  });

  it("supports manual numeric input and stepper buttons", () => {
    const dispatch = vi.fn().mockResolvedValue([]);
    const waitingFor = chooseXWaitingFor(5);

    setGameStoreForTest({
      gameState: createGameState({ waiting_for: waitingFor }),
      waitingFor,
      dispatch,
    });

    render(<ChooseXValueUI />);

    const input = screen.getByLabelText("Enter X value") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "4" } });
    expect(screen.getByRole("button", { name: "Confirm X = 4" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Increase X value" }));
    expect(screen.getByRole("button", { name: "Confirm X = 5" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Increase X value" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Decrease X value" }));
    fireEvent.click(screen.getByRole("button", { name: "Decrease X value" }));
    expect(screen.getByRole("button", { name: "Confirm X = 3" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Confirm X = 3" }));
    expect(dispatch).toHaveBeenCalledWith({ type: "ChooseX", data: { value: 3 } });
  });

  // DELIBERATE BEHAVIOUR CHANGE (binding ruling D2). This test previously asserted the
  // COERCION this change removes: "99" under max=5 committed 5, and "0" under min=2
  // committed 2 — values the caster never chose. Three shipped assertions flip here: the
  // upper-bound clamp, the lower-bound blur reset, and the stepper gating (the steppers are
  // now the recovery anchor and must stay LIVE while the entry is invalid).
  it("rejects out-of-range manual X input instead of coercing it", () => {
    const waitingFor = chooseXWaitingFor(5, 2);

    setGameStoreForTest({
      gameState: createGameState({ waiting_for: waitingFor }),
      waitingFor,
    });

    render(<ChooseXValueUI />);

    const input = screen.getByLabelText("Enter X value") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "99" } });
    expect(input.value).toBe("99");
    expect(screen.getByRole("button", { name: /^Confirm X/ })).toBeDisabled();
    expect(
      screen.getByText("Enter a whole number between 2 and 5"),
    ).toBeInTheDocument();

    fireEvent.change(input, { target: { value: "0" } });
    expect(input.value).toBe("0");
    expect(screen.getByRole("button", { name: /^Confirm X/ })).toBeDisabled();
    expect(
      screen.getByText("Enter a whole number between 2 and 5"),
    ).toBeInTheDocument();
    // The recovery anchor, asserted exactly where the old gating lived.
    expect(
      screen.getByRole("button", { name: "Decrease X value" }),
    ).not.toBeDisabled();

    // Pins that the deleted onBlur reset does not mutate the entry.
    fireEvent.blur(input);
    expect(input.value).toBe("0");
  });

  // DELIBERATE BEHAVIOUR CHANGE (binding ruling D2): the intermediate "1" no longer reads as
  // a committed 10. The multi-digit-typing capability it guards SURVIVES — the final dispatch
  // assertion is unchanged.
  it("allows typing a multi-digit X value that starts below the minimum", () => {
    const dispatch = vi.fn().mockResolvedValue([]);
    const waitingFor = chooseXWaitingFor(20, 10);

    setGameStoreForTest({
      gameState: createGameState({ waiting_for: waitingFor }),
      waitingFor,
      dispatch,
    });

    render(<ChooseXValueUI />);

    const input = screen.getByLabelText("Enter X value") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "1" } });
    expect(input.value).toBe("1");
    expect(screen.getByRole("button", { name: /^Confirm X/ })).toBeDisabled();

    fireEvent.change(input, { target: { value: "15" } });
    expect(input.value).toBe("15");
    fireEvent.click(screen.getByRole("button", { name: "Confirm X = 15" }));
    expect(dispatch).toHaveBeenCalledWith({ type: "ChooseX", data: { value: 15 } });
  });

  // Enter bypasses the button's `disabled` attribute entirely, so this is the ONLY route on
  // which `handleCommit`'s null-guard is observable. Matched pair: the negative alone would
  // be satisfied by a component that never dispatches at all.
  it("CX/enter: Enter submits a valid X and refuses to coerce an out-of-range one", () => {
    const dispatch = vi.fn().mockResolvedValue([]);
    const waitingFor = chooseXWaitingFor(5, 2);

    setGameStoreForTest({
      gameState: createGameState({ waiting_for: waitingFor }),
      waitingFor,
      dispatch,
    });

    render(<ChooseXValueUI />);

    const input = screen.getByLabelText("Enter X value");
    fireEvent.change(input, { target: { value: "99" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(dispatch).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: "4" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(dispatch).toHaveBeenCalledWith({ type: "ChooseX", data: { value: 4 } });
  });

  it("dispatches CancelCast when cancel is clicked", () => {
    const dispatch = vi.fn().mockResolvedValue([]);
    const waitingFor = chooseXWaitingFor(3);

    setGameStoreForTest({
      gameState: createGameState({ waiting_for: waitingFor }),
      waitingFor,
      dispatch,
    });

    render(<ChooseXValueUI />);

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(dispatch).toHaveBeenCalledWith({ type: "CancelCast" });
  });

  it("honors min value and resets to a valid value when ChooseXValue state re-enters", () => {
    const dispatch = vi.fn().mockResolvedValue([]);
    const waitingFor = chooseXWaitingFor(10, 1);

    setGameStoreForTest({
      gameState: createGameState({ waiting_for: waitingFor }),
      waitingFor,
      dispatch,
    });

    const { rerender } = render(<ChooseXValueUI />);

    const input = screen.getByLabelText("Enter X value") as HTMLInputElement;
    // min > 0 renders the two-sided hint; the window is shown, not the control's native min.
    expect(screen.getByText("min 1 / max 10")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Confirm X = 1" })).toBeInTheDocument();
    fireEvent.change(input, { target: { value: "7" } });
    expect(screen.getByRole("button", { name: "Confirm X = 7" })).toBeInTheDocument();

    // Simulate re-entering ChooseXValue (e.g., after cost reduction changes max)
    const nextWaitingFor = chooseXWaitingFor(4, 2);
    setGameStoreForTest({
      gameState: createGameState({ waiting_for: nextWaitingFor }),
      waitingFor: nextWaitingFor,
      dispatch,
    });

    rerender(<ChooseXValueUI />);

    expect(screen.getByRole("button", { name: "Confirm X = 2" })).toBeInTheDocument();
  });

  it("renders nothing for impossible min greater than max bounds", () => {
    const waitingFor = chooseXWaitingFor(0, 1);

    setGameStoreForTest({
      gameState: createGameState({ waiting_for: waitingFor }),
      waitingFor,
    });

    const { container } = render(<ChooseXValueUI />);
    expect(container).toBeEmptyDOMElement();
  });

  // RETARGETED, not deleted. #2427's fix is DialogHost.tsx's residual-`transform` handling,
  // which broke `<input type="range">` hit-testing in bottom-anchored panels. This test never
  // asserted hit-testing — `fireEvent.change` dispatches on the node and happy-dom performs no
  // layout — so it is, and always was, a MOUNT-INTEGRATION guard: the control renders and
  // stays operable under DialogHost's motion wrapper and pointerEvents logic. Retargeting to
  // the box preserves exactly that strength; the stepper half is added because the steppers
  // are now the drag-replacement surface a #2427-class regression would break next.
  it("amount box and steppers accept input when mounted inside DialogHost (#2427)", () => {
    const dispatch = vi.fn().mockResolvedValue([]);
    const waitingFor = chooseXWaitingFor(5);

    setGameStoreForTest({
      gameState: createGameState({
        turn_decision_controller: 0,
        active_player: 0,
        waiting_for: waitingFor,
      }),
      waitingFor,
      dispatch,
    });

    render(
      <DialogHost>
        <ChooseXValueUI />
      </DialogHost>,
    );

    fireEvent.change(screen.getByLabelText("Enter X value"), {
      target: { value: "4" },
    });
    expect(screen.getByRole("button", { name: "Confirm X = 4" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Increase X value" }));
    expect(screen.getByRole("button", { name: "Confirm X = 5" })).toBeInTheDocument();
  });
});
