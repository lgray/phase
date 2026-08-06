import { act } from "react";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { useGameStore } from "../../../stores/gameStore.ts";
import { useMultiplayerStore } from "../../../stores/multiplayerStore.ts";
import { buildGameState } from "../../../test/factories/gameStateFactory.ts";
import { PlayerHud } from "../PlayerHud.tsx";

describe("PlayerHud designations", () => {
  beforeEach(() => {
    useMultiplayerStore.setState({ activePlayerId: 0 });
    useGameStore.setState({ gameState: buildGameState() });
  });

  afterEach(() => {
    cleanup();
  });

  describe("Monarch", () => {
    it("renders the crown when the local player is the monarch", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ monarch: 0 }) });
      });
      render(<PlayerHud />);
      expect(screen.getByLabelText("Monarch")).toBeInTheDocument();
    });

    it("does not render the crown when an opponent is the monarch", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ monarch: 1 }) });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText("Monarch")).toBeNull();
    });

    it("does not render the crown when no one is the monarch", () => {
      render(<PlayerHud />);
      expect(screen.queryByLabelText("Monarch")).toBeNull();
    });
  });

  describe("Initiative", () => {
    it("renders when the local player has the initiative", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ initiative: 0 }) });
      });
      render(<PlayerHud />);
      expect(screen.getByLabelText("Initiative")).toBeInTheDocument();
    });

    it("does not render when an opponent has the initiative", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ initiative: 1 }) });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText("Initiative")).toBeNull();
    });

    it("does not render when no one has the initiative", () => {
      render(<PlayerHud />);
      expect(screen.queryByLabelText("Initiative")).toBeNull();
    });
  });

  describe("City's Blessing", () => {
    it("renders when the local player has the blessing", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ city_blessing: [0] }) });
      });
      render(<PlayerHud />);
      expect(screen.getByLabelText("City's Blessing")).toBeInTheDocument();
    });

    it("does not render when only an opponent has the blessing", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ city_blessing: [1] }) });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText("City's Blessing")).toBeNull();
    });

    it("does not render when no one has the blessing", () => {
      render(<PlayerHud />);
      expect(screen.queryByLabelText("City's Blessing")).toBeNull();
    });
  });

  describe("Ring level", () => {
    it("renders the ring counter at level 3 for the local player", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ ring_level: { "0": 3 } }) });
      });
      render(<PlayerHud />);
      expect(screen.getByLabelText(/the ring tempts you \(level 3\)/i)).toBeInTheDocument();
    });

    it("does not render at level 0", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ ring_level: { "0": 0 } }) });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText(/the ring tempts you/i)).toBeNull();
    });

    it("does not render when only an opponent is tempted", () => {
      act(() => {
        useGameStore.setState({ gameState: buildGameState({ ring_level: { "1": 2 } }) });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText(/the ring tempts you/i)).toBeNull();
    });
  });

  describe("Energy", () => {
    it("renders the energy counter when the local player has energy", () => {
      const gameState = buildGameState();
      gameState.players[0].energy = 5;
      act(() => {
        useGameStore.setState({ gameState });
      });
      render(<PlayerHud />);
      expect(screen.getByLabelText("5 energy counters")).toBeInTheDocument();
    });

    it("uses singular form for one energy", () => {
      const gameState = buildGameState();
      gameState.players[0].energy = 1;
      act(() => {
        useGameStore.setState({ gameState });
      });
      render(<PlayerHud />);
      expect(screen.getByLabelText("1 energy counter")).toBeInTheDocument();
    });

    it("does not render at zero energy", () => {
      render(<PlayerHud />);
      expect(screen.queryByLabelText(/energy counter/)).toBeNull();
    });
  });

  describe("Dungeon", () => {
    it("renders the dungeon badge when the local player is venturing", () => {
      act(() => {
        useGameStore.setState({
          gameState: buildGameState({
            dungeon_progress: {
              "0": { current_dungeon: "LostMineOfPhandelver", current_room: 1, completed: [] },
            },
          }),
        });
      });
      render(<PlayerHud />);
      expect(screen.getByLabelText("Venturing in Lost Mine, room 2")).toBeInTheDocument();
    });

    it("does not render when the player has progress but no active dungeon", () => {
      act(() => {
        useGameStore.setState({
          gameState: buildGameState({
            dungeon_progress: {
              "0": { current_dungeon: null, current_room: 0, completed: ["TombOfAnnihilation"] },
            },
          }),
        });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText(/venturing in/i)).toBeNull();
    });

    it("does not render when only an opponent is venturing", () => {
      act(() => {
        useGameStore.setState({
          gameState: buildGameState({
            dungeon_progress: {
              "1": { current_dungeon: "Undercity", current_room: 0, completed: [] },
            },
          }),
        });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText(/venturing in/i)).toBeNull();
    });
  });

  // CR 732.2a: the `∞` HUD badge is driven ONLY by the engine projection
  // `derived.unbounded_families` — the FE derives neither which axes are unbounded, nor the
  // family they group into, nor whether a collapse is coming.
  describe("Unbounded resources (∞)", () => {
    it("renders an ∞ badge for the local player's engine-attributed family", () => {
      act(() => {
        useGameStore.setState({
          gameState: buildGameState({
            derived: {
              unbounded_families: [
                { player: 0, family: "tokens", state: { type: "Unscheduled" } },
              ],
            },
          }),
        });
      });
      render(<PlayerHud />);
      // REVERT-PROBE: stop reading `derived.unbounded_families` (or remove the
      // PlayerHud map) → the badge is absent → this assertion fails.
      expect(screen.getByLabelText("Unbounded tokens (∞)")).toBeInTheDocument();
    });

    it("does not render when there are no unbounded resources", () => {
      act(() => {
        useGameStore.setState({
          gameState: buildGameState({ derived: { unbounded_families: [] } }),
        });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText(/Unbounded/)).toBeNull();
    });

    it("does not render when only an opponent has an unbounded family", () => {
      act(() => {
        useGameStore.setState({
          gameState: buildGameState({
            derived: {
              unbounded_families: [
                { player: 1, family: "tokens", state: { type: "Unscheduled" } },
              ],
            },
          }),
        });
      });
      render(<PlayerHud />);
      expect(screen.queryByLabelText(/Unbounded/)).toBeNull();
    });

    // The "two mana axes collapse to one badge" case MOVED TO THE ENGINE as
    // `derived_views::tests::two_mana_axes_fold_to_one_family_row`; migrating it IS the evidence
    // that the fold left the display layer. What remains here is the render-level consequence:
    // the engine hands down one row per family, so the HUD renders one badge per row.
  });
});
