import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { GameState, LoopDetectionMode } from "../../../adapter/types";
import { dispatchAction } from "../../../game/dispatch";
import { useGameStore } from "../../../stores/gameStore";
import { LoopDetectionToggle } from "../LoopDetectionToggle";

vi.mock("../../../game/dispatch.ts", () => ({
  dispatchAction: vi.fn(),
}));

const mockDispatch = vi.mocked(dispatchAction);

afterEach(() => {
  cleanup();
  mockDispatch.mockReset();
});

function setLoopDetection(mode: LoopDetectionMode | undefined) {
  // Only `loop_detection` is read by the toggle; cast the partial state.
  useGameStore.setState({ gameState: { loop_detection: mode } as unknown as GameState });
}

describe("LoopDetectionToggle", () => {
  it("shows Off and dispatches On when the detector is disabled (default)", () => {
    // Absent field is the pre-feature default → treated as Off.
    setLoopDetection(undefined);
    render(<LoopDetectionToggle />);

    const toggle = screen.getByRole("switch");
    expect(toggle).toHaveAttribute("aria-checked", "false");
    expect(toggle).toHaveTextContent("Off");

    fireEvent.click(toggle);
    expect(mockDispatch).toHaveBeenCalledTimes(1);
    expect(mockDispatch).toHaveBeenCalledWith({
      type: "SetLoopDetection",
      data: { mode: { type: "On" } },
    });
  });

  it("shows On and dispatches Off when the detector is enabled", () => {
    setLoopDetection({ type: "On" });
    render(<LoopDetectionToggle />);

    const toggle = screen.getByRole("switch");
    expect(toggle).toHaveAttribute("aria-checked", "true");
    expect(toggle).toHaveTextContent("On");

    fireEvent.click(toggle);
    expect(mockDispatch).toHaveBeenCalledTimes(1);
    expect(mockDispatch).toHaveBeenCalledWith({
      type: "SetLoopDetection",
      data: { mode: { type: "Off" } },
    });
  });

  it("treats an explicit Off mode as disabled", () => {
    setLoopDetection({ type: "Off" });
    render(<LoopDetectionToggle />);

    const toggle = screen.getByRole("switch");
    expect(toggle).toHaveAttribute("aria-checked", "false");
    fireEvent.click(toggle);
    expect(mockDispatch).toHaveBeenCalledWith({
      type: "SetLoopDetection",
      data: { mode: { type: "On" } },
    });
  });
});
