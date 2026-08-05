import { useId } from "react";
import { useTranslation } from "react-i18next";

import { gameButtonClass } from "../ui/buttonStyles.ts";

/**
 * Shared bounded-amount control for the engine's amount prompts
 * (PayAmountChoice / ChooseXValue / AssistPayment).
 *
 * The `[min, max]` window is ENGINE-OWNED and arrives as props — this component holds no bound of
 * its own, no default, and no fallback. `parseAmount` is the single sanitization authority; it
 * REJECTS (returns null) rather than coercing, so a player never submits a number they did not type.
 */

/** Digit reading of `raw`, ignoring the window. Recovery uses this; SUBMISSION uses `parseAmount`. */
function digitsOf(raw: string): number | null {
  return /^\d+$/.test(raw) ? Number(raw) : null;
}

export function parseAmount(raw: string, min: number, max: number): number | null {
  // Digits only. `Number()` alone is NOT sufficient: MEASURED, Number("") === 0,
  // Number(" 7 ") === 7, Number("1.5") === 1.5, Number("1e3") === 1000, Number("+2") === 2 and
  // Number("0x10") === 16 all land INSIDE a typical window.
  const value = digitsOf(raw);
  return value !== null && value >= min && value <= max ? value : null;
}

export interface AmountInputLabels {
  /** aria-label for the numeric text box. */
  input: string;
  /** aria-label for the − stepper. */
  decrease: string;
  /** aria-label for the + stepper. */
  increase: string;
}

export function AmountInput({
  raw,
  onRawChange,
  min,
  max,
  onSubmit,
  labels,
}: {
  raw: string;
  onRawChange: (raw: string) => void;
  min: number;
  max: number;
  /** Called on Enter. MUST itself reject an invalid amount — AmountInput deliberately does not
   *  re-guard, because a second guard would make the caller's guard unobservable and untestable. */
  onSubmit: () => void;
  labels: AmountInputLabels;
}) {
  const { t } = useTranslation("game");
  const amount = parseAmount(raw, min, max);
  const hintId = useId();
  const errorId = useId();

  // Recovery anchor. With the slider deleted the steppers are the only non-typing way out of an
  // invalid entry, so they stay LIVE while `amount === null` and snap back into [min, max]. They
  // step from the DIGIT reading, not from `amount`: `parseAmount` collapses "junk" and "out of
  // range" into the same null, so stepping from `amount ?? min` would throw away a perfectly
  // readable 1001 and jump to min. `parseAmount` gates SUBMISSION; `step` performs RECOVERY
  // toward the window.
  const step = (delta: number) =>
    onRawChange(String(Math.min(Math.max((digitsOf(raw) ?? min) + delta, min), max)));
  const decDisabled = amount !== null && amount <= min;
  const incDisabled = amount !== null && amount >= max;

  // ponytail: no showSlider/showSteppers flag — the slider is deleted, not configurable.
  // ponytail: no role="alert" — assertive per-keystroke announcements are the anti-pattern;
  //   aria-invalid + aria-describedby is the association.
  // ponytail: no pattern="[0-9]*" — inputMode="numeric" carries the modern-iOS keypad; add back
  //   only on a legacy-iOS report.
  // ponytail: the null-guard lives once, in the caller's handleCommit — a second guard in
  //   onKeyDown would make it unobservable.
  // ponytail: no role="spinbutton" — zero in-repo precedent, and it forces aria-valuenow/min/max
  //   upkeep.
  return (
    <div className="mb-4 px-2">
      <div className="flex items-center justify-center gap-2">
        <button
          type="button"
          onClick={() => step(-1)}
          disabled={decDisabled}
          aria-label={labels.decrease}
          className={gameButtonClass({
            tone: "neutral",
            size: "xs",
            disabled: decDisabled,
            className: "h-9 w-9 px-0 text-base",
          })}
        >
          −
        </button>
        <input
          type="text"
          inputMode="numeric"
          autoComplete="off"
          value={raw}
          onChange={(e) => onRawChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              onSubmit();
              return;
            }
            // type="text" has no native stepping; the accessibility floor requires arrows to step.
            if (e.key === "ArrowUp") {
              e.preventDefault();
              step(1);
            } else if (e.key === "ArrowDown") {
              e.preventDefault();
              step(-1);
            }
          }}
          aria-label={labels.input}
          aria-invalid={amount === null}
          // The window is ANNOUNCED, not merely displayed. `type="range"` announced min/max/now
          // natively; a text box announces nothing, so the range hint is permanently associated
          // and the error message is appended to it while invalid.
          aria-describedby={amount === null ? `${hintId} ${errorId}` : hintId}
          // `w-24` not `w-20`: four digits must fit for the 1000 case.
          className={`h-9 w-24 rounded-lg border bg-gray-950/80 px-2 text-center font-mono text-base font-semibold shadow-inner outline-none transition focus:ring-2 ${
            amount === null
              ? "border-red-400/60 text-red-200 focus:ring-red-400/30"
              : "border-cyan-400/30 text-cyan-100 focus:ring-cyan-400/30"
          }`}
        />
        <button
          type="button"
          onClick={() => step(1)}
          disabled={incDisabled}
          aria-label={labels.increase}
          className={gameButtonClass({
            tone: "neutral",
            size: "xs",
            disabled: incDisabled,
            className: "h-9 w-9 px-0 text-base",
          })}
        >
          +
        </button>
        <span id={hintId} className="shrink-0 text-xs text-gray-500">
          {min > 0 ? t("mana.minMax", { min, max }) : t("mana.maxOnly", { max })}
        </span>
      </div>

      {amount === null && (
        <p id={errorId} className="mt-2 text-center text-xs text-red-300">
          {t("mana.amountOutOfRange", { min, max })}
        </p>
      )}
    </div>
  );
}
