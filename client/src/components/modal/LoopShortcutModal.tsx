import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

import type {
  InteractionResponseSpec,
  InteractionShortcutPreview,
  ViewerInteraction,
} from "../../adapter/generated/interaction";
import type { IterationCount, ResourceAxis, WaitingFor, WinKind } from "../../adapter/types.ts";
import { useCanActForWaitingState } from "../../hooks/usePlayerId.ts";
import { useGameStore } from "../../stores/gameStore.ts";
import { familyOf, UNBOUNDED_FAMILY_LABEL_KEY, UnboundedBadge } from "../hud/HudBadges.tsx";
import { AmountInput, parseAmount } from "../mana/AmountInput.tsx";
import { DialogShell } from "./DialogShell.tsx";

/**
 * CR 732.2a/b/c: the interactive loop-shortcut declare + accept-or-shorten
 * modals. Pure display layer — every rendered value is a direct read of an
 * engine schema/proposal/response-spec field; the frontend derives, filters, and
 * computes nothing. `DeclareShortcut.template` is always `null`: building pins is not a
 * client authority, and the engine remains the sole legality authority
 * (`predictability_gate` + `validate_pins`).
 *
 * MEASURED LIMIT, stated rather than assumed — `null` is what the client can honestly send,
 * NOT a payload the engine accepts everywhere. `handle_declare_shortcut`
 * (`game/engine.rs`, the `!offer.schema.points.is_empty()` block) REFUSES a `template: null`
 * declaration unless the proposer controls the recorded loop period, so only the point-free
 * drain shape declares successfully at this base. Carrying the engine's own issued
 * declaration through the manual declare path is an ENGINE-side repair; a client that
 * reconstructed a template would be inventing rules authority it does not have.
 */

/** The engine's published shortcut response spec — the count window, `allowDecline`, and the
 *  engine-computed preview. A lookup into what the engine already sent, never a derivation. */
type ShortcutSpec = Extract<InteractionResponseSpec, { type: "shortcut" }>["data"];

function shortcutSpec(interaction: ViewerInteraction | null): ShortcutSpec | null {
  for (const opportunity of interaction?.opportunities ?? []) {
    if (opportunity.response.type !== "schema") continue;
    const { spec } = opportunity.response.data;
    if (spec.type === "shortcut") return spec.data;
  }
  return null;
}

// CR 732.1b: render the engine-proposed repeat mode — the offer's own stated count, echoed
// verbatim. The picker below narrows WITHIN the engine's published window; this line is the
// offer, not the pick.
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
 * CR 732.2a: what the offer's stated count actually DOES, per axis. Every magnitude is read
 * straight off the engine's published preview — already multiplied, already signed. The heading
 * names `preview.count` because these numbers describe that count and no other, so a player can
 * never read them against a different one; the display layer multiplies nothing.
 */
function PreviewLines({ preview }: { preview: InteractionShortcutPreview }) {
  const { t } = useTranslation("game");
  if (preview.entries.length === 0) return null;
  return (
    <div className="flex flex-col gap-1 rounded-lg bg-white/5 px-3 py-2">
      <p className="text-xs font-semibold tracking-wide text-slate-400 uppercase">
        {t("comboShortcut.previewTitle", { count: preview.count })}
      </p>
      {preview.entries.map((entry, index) => (
        <p
          key={`${entry.family}-${entry.player ?? "all"}-${index}`}
          className="text-sm text-slate-200 tabular-nums"
        >
          {entry.player === null
            ? t("comboShortcut.previewEntry", {
                amount: entry.amount,
                resource: t(UNBOUNDED_FAMILY_LABEL_KEY[entry.family]),
              })
            : t("comboShortcut.previewEntryPlayer", {
                amount: entry.amount,
                resource: t(UNBOUNDED_FAMILY_LABEL_KEY[entry.family]),
                // Seat display numbering, the same +1 formatting `LifeTotal` uses on the engine's
                // seat id. Formatting, not derivation.
                player: t("lifeTotal.playerLabel", { seat: entry.player + 1 }),
              })}
        </p>
      ))}
    </div>
  );
}

/**
 * CR 732.2a: the priority holder (the proposer) may declare the shortcut OR decline it —
 * "the player with priority may suggest a shortcut" is
 * optional. Declining dispatches `DeclineShortcut`, which restores ordinary
 * priority engine-side; the opponent-side escape hatch (accept/shorten) lives in
 * `RespondToShortcutModal`.
 */
export function DeclareShortcutModal() {
  const canAct = useCanActForWaitingState();
  const waitingFor = useGameStore((s) => s.waitingFor);
  // `shortcutSpec` returns a reference INTO store state (or null), so the selector is stable.
  const spec = useGameStore((s) => shortcutSpec(s.viewerInteraction));

  if (waitingFor?.type !== "LoopShortcut" || !canAct) return null;

  // The offer body is mounted only while the offer is live, so the picker's entry cannot survive
  // into a later offer: this component itself never unmounts (GamePage keeps both modals mounted
  // and they self-gate), which is exactly how a stale typed count would otherwise leak.
  return <DeclareShortcutOffer data={waitingFor.data} spec={spec} />;
}

function DeclareShortcutOffer({
  data,
  spec,
}: {
  data: Extract<WaitingFor, { type: "LoopShortcut" }>["data"];
  spec: ShortcutSpec | null;
}) {
  const { t } = useTranslation("game");
  const dispatch = useGameStore((s) => s.dispatch);
  const { certificate, schema } = data;

  // CR 732.2a: the count window is ENGINE-OWNED. `null` when this offer publishes no finite
  // window (UntilLethal) or when the transport published no interaction projection at all — in
  // both cases no picker renders and the offer's own count is declared verbatim, as before.
  const countSpec = spec?.count.type === "fixed" ? spec.count.data : null;
  // No client-side default: until the player types, the box shows the ENGINE's suggested count.
  const [picked, setPicked] = useState<string | null>(null);
  const raw = picked ?? (countSpec === null ? "" : String(countSpec.suggested));
  // `parseAmount` is the shared sanitization authority — it REJECTS out-of-window entries rather
  // than clamping, so a count the engine did not offer can never be declared.
  const chosen = countSpec === null ? null : parseAmount(raw, countSpec.min, countSpec.max);
  const confirmDisabled = countSpec !== null && chosen === null;

  const handleConfirm = useCallback(() => {
    // `template: null` is unchanged by C5 (see the module header's measured limit) — the picker
    // moves the COUNT only.
    if (countSpec === null) {
      dispatch({
        type: "DeclareShortcut",
        data: { count: schema.iteration_count, template: null },
      });
      return;
    }
    // Refused entry ⇒ submit nothing. The guard lives here once (AmountInput deliberately does
    // not re-guard), and the confirm button is disabled in the same state.
    if (chosen === null) return;
    dispatch({ type: "DeclareShortcut", data: { count: { Fixed: chosen }, template: null } });
  }, [dispatch, countSpec, chosen, schema.iteration_count]);

  const handleDecline = useCallback(() => {
    // CR 732.2a: decline the auto-offer; the engine restores ordinary priority.
    dispatch({ type: "DeclineShortcut" });
  }, [dispatch]);

  // CR 702.51a: engine-computed count of untapped creatures the engine will auto-tap
  // for convoke — read directly from the schema (the engine owns the derivation).
  const convokeTappable = schema.convoke_tappable_count;

  const footer = (
    <div className="flex flex-col gap-3 sm:flex-row sm:justify-end">
      <button
        onClick={handleConfirm}
        disabled={confirmDisabled}
        className={`min-h-11 rounded-[16px] bg-cyan-500 px-6 py-2 font-semibold text-slate-950 shadow-[0_14px_34px_rgba(6,182,212,0.28)] transition hover:bg-cyan-400 ${
          confirmDisabled ? "cursor-not-allowed opacity-50 hover:bg-cyan-500" : ""
        }`}
      >
        {t("comboShortcut.confirm")}
      </button>
      {/* CR 732.2a: declining is offered only when the engine says this offer may be declined. */}
      {(spec?.allowDecline ?? true) && (
        <button
          onClick={handleDecline}
          className="min-h-11 rounded-[16px] border border-white/8 bg-white/5 px-6 py-2 font-semibold text-slate-200 transition hover:bg-white/8"
        >
          {t("comboShortcut.decline")}
        </button>
      )}
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
        {countSpec && (
          <AmountInput
            raw={raw}
            onRawChange={setPicked}
            min={countSpec.min}
            max={countSpec.max}
            onSubmit={handleConfirm}
            labels={{
              input: t("comboShortcut.countAria"),
              decrease: t("mana.decreaseAmount"),
              increase: t("mana.increaseAmount"),
            }}
          />
        )}
        {spec?.preview && <PreviewLines preview={spec.preview} />}
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
