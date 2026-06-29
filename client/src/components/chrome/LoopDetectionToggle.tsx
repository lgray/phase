import { useTranslation } from "react-i18next";

import { dispatchAction } from "../../game/dispatch";
import { useGameStore } from "../../stores/gameStore";

/**
 * In-game toggle for the live combo (infinite-loop) detector (CR 732.2a).
 *
 * The engine owns the flag (`GameState.loop_detection`) and ALL gating logic;
 * this control is pure display + dispatch. It reads the current mode from
 * engine-provided state and dispatches `SetLoopDetection` on click. Default is
 * OFF, which restores exact pre-detector behavior (no automatic mandatory-loop
 * resolution and no `∞` unbounded-resource display). New game-changing
 * functionality is opt-in (issue #4603).
 */
export function LoopDetectionToggle() {
  const { t } = useTranslation();
  const enabled = useGameStore((s) => s.gameState?.loop_detection?.type === "On");

  const toggle = () => {
    void dispatchAction({
      type: "SetLoopDetection",
      data: { mode: { type: enabled ? "Off" : "On" } },
    });
  };

  return (
    <button
      type="button"
      onClick={toggle}
      role="switch"
      aria-checked={enabled}
      className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm text-gray-300 transition-colors hover:bg-gray-800 hover:text-white"
      title={t("gameMenu.comboDetectorTitle")}
    >
      <span>{t("gameMenu.comboDetector")}</span>
      <span className={`font-mono text-xs ${enabled ? "text-emerald-400" : "text-gray-500"}`}>
        {enabled ? t("gameMenu.comboDetectorOn") : t("gameMenu.comboDetectorOff")}
      </span>
    </button>
  );
}
