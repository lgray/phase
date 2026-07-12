import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import type { IterationCount, ResourceAxis, WinKind } from "../../adapter/types.ts";
import { useCanActForWaitingState } from "../../hooks/usePlayerId.ts";
import { useGameStore } from "../../stores/gameStore.ts";
import { familyOf, UnboundedBadge } from "../hud/HudBadges.tsx";
import { DialogShell } from "./DialogShell.tsx";

/**
 * CR 732.2a/b/c: the interactive loop-shortcut declare + accept-or-shorten
 * modals. Pure display layer — every rendered value is a direct read of an
 * engine schema/proposal field; the frontend derives, filters, and computes
 * nothing. `DeclareShortcut.template` is always `null` (per-iteration pin-capture
 * is deferred to an engine-side assembly), and the engine remains the sole
 * legality authority (`predictability_gate` + `validate_pins`).
 */

// CR 732.1b: render the engine-proposed repeat mode. The count is echoed from the
// schema/proposal verbatim — never chosen or computed here.
function CountLine({ count }: { count: IterationCount }) {
  const { t } = useTranslation("game");
  return (
    <p className="text-sm text-slate-300">
      {count === "UntilLethal"
        ? t("comboShortcut.untilLethal")
        : t("comboShortcut.fixedCount", { count: count.Fixed })}
    </p>
  );
}

// CR 704.5a/704.5c etc.: the certificate's determinate win kind, a pure key lookup.
function WinKindLine({ kind }: { kind: WinKind }) {
  const { t } = useTranslation("game");
  return <p className="text-sm font-semibold text-white">{t(`comboShortcut.winKind.${kind}`)}</p>;
}

// Reuses the engine-authored HUD family mapping (`familyOf`) + badge — no new
// axis logic, no new i18n keys. Dedupes by display family like the HUD caller.
function FamilyBadges({ axes }: { axes: ResourceAxis[] }) {
  const families = [...new Set(axes.map(familyOf))];
  if (families.length === 0) return null;
  return (
    <div className="flex flex-wrap gap-1">
      {families.map((family) => (
        <UnboundedBadge key={family} family={family} />
      ))}
    </div>
  );
}

/**
 * CR 732.2a: the loop's determinate winner (the controller) declares the
 * shortcut. Confirm-only — the engine offers no decline for `LoopShortcut`; the
 * escape hatch lives on the opponent side (`RespondToShortcutModal` → break out).
 */
export function DeclareShortcutModal() {
  const { t } = useTranslation("game");
  const canAct = useCanActForWaitingState();
  const waitingFor = useGameStore((s) => s.waitingFor);
  const dispatch = useGameStore((s) => s.dispatch);

  const handleConfirm = useCallback(() => {
    if (waitingFor?.type !== "LoopShortcut") return;
    // Echo the engine-proposed iteration_count verbatim; pin-capture is deferred,
    // so `template` is always null (matches every live engine + AI declare path).
    dispatch({
      type: "DeclareShortcut",
      data: { count: waitingFor.data.schema.iteration_count, template: null },
    });
  }, [waitingFor, dispatch]);

  if (waitingFor?.type !== "LoopShortcut" || !canAct) return null;

  const { certificate, schema } = waitingFor.data;
  // CR 702.51a: read-only count of untapped creatures the engine will auto-tap
  // for convoke (`select_convoke_taps` re-binds the concrete taps each iteration).
  // `.length` over an engine-provided array is formatting, not game logic.
  const convokeTappable = schema.points.reduce(
    (total, point) =>
      typeof point.kind === "object" && "ConvokeTaps" in point.kind
        ? total + point.kind.ConvokeTaps.tappable.length
        : total,
    0,
  );

  const footer = (
    <div className="flex justify-end">
      <button
        onClick={handleConfirm}
        className="min-h-11 rounded-[16px] bg-cyan-500 px-6 py-2 font-semibold text-slate-950 shadow-[0_14px_34px_rgba(6,182,212,0.28)] transition hover:bg-cyan-400"
      >
        {t("comboShortcut.confirm")}
      </button>
    </div>
  );

  return (
    <DialogShell
      title={t("comboShortcut.declareTitle")}
      subtitle={t("comboShortcut.declareSubtitle")}
      size="md"
      footer={footer}
    >
      <div className="flex flex-col gap-3 px-3 py-3 lg:px-5 lg:py-5">
        <WinKindLine kind={certificate.win_kind} />
        <CountLine count={schema.iteration_count} />
        <FamilyBadges axes={certificate.unbounded} />
        {convokeTappable > 0 && (
          <p className="text-xs text-slate-400">
            {t("comboShortcut.convokeInfo", { count: convokeTappable })}
          </p>
        )}
      </div>
    </DialogShell>
  );
}

/**
 * CR 732.2b/c: after the proposer declares, each other living player, in APNAP
 * order, may accept the shortcut or shorten it (break out to resume manual play).
 * Phase 3 discards `at_iteration` (no finite-K materialization), so "Break out"
 * dispatches a placeholder `at_iteration: 1`.
 */
export function RespondToShortcutModal() {
  const { t } = useTranslation("game");
  const canAct = useCanActForWaitingState();
  const waitingFor = useGameStore((s) => s.waitingFor);
  const dispatch = useGameStore((s) => s.dispatch);

  const handleAccept = useCallback(() => {
    dispatch({ type: "RespondToShortcut", data: { response: "Accept" } });
  }, [dispatch]);

  const handleShorten = useCallback(() => {
    dispatch({ type: "RespondToShortcut", data: { response: { Shorten: { at_iteration: 1 } } } });
  }, [dispatch]);

  if (waitingFor?.type !== "RespondToShortcut" || !canAct) return null;

  const { proposal } = waitingFor.data;

  const footer = (
    <div className="flex flex-col gap-3 sm:flex-row sm:justify-end">
      <button
        onClick={handleAccept}
        className="min-h-11 rounded-[16px] bg-cyan-500 px-6 py-2 font-semibold text-slate-950 shadow-[0_14px_34px_rgba(6,182,212,0.28)] transition hover:bg-cyan-400"
      >
        {t("comboShortcut.accept")}
      </button>
      <button
        onClick={handleShorten}
        className="min-h-11 rounded-[16px] border border-white/8 bg-white/5 px-6 py-2 font-semibold text-slate-200 transition hover:bg-white/8"
      >
        {t("comboShortcut.shorten")}
      </button>
    </div>
  );

  return (
    <DialogShell
      title={t("comboShortcut.respondTitle")}
      subtitle={t("comboShortcut.respondSubtitle")}
      size="md"
      footer={footer}
    >
      <div className="flex flex-col gap-3 px-3 py-3 lg:px-5 lg:py-5">
        <WinKindLine kind={proposal.win_kind} />
        <CountLine count={proposal.count} />
        <FamilyBadges axes={proposal.unbounded} />
      </div>
    </DialogShell>
  );
}
