/**
 * ∞-channel cross-seam pin. Both JSON files are ENGINE-EMITTED by
 * `combo_infinite_pile::real_4p_object_growth_accept_writes_infinite_pile` and
 * `kilo_live_offer_from_real_dump::kilo_accept_marks_pentad_charge_as_unbounded_display_target`,
 * each driving a REAL 4-player dump through the REAL APNAP accept. Regenerate with
 * `UPDATE_WIRE_GOLDEN=1 cargo test -p phase-engine --test integration <fn>`. Never hand-edit them.
 * Every existing client test that touches these channels hand-writes its own `derived` block, so
 * this file is the only place the engine's wire shape and the client's readers meet.
 * Both goldens are captured AFTER the accept, while a finite collapse is merely SCHEDULED — the
 * engine defers APPLYING the growth to the next CR 500.5 boundary (an engine deviation,
 * pre-existing and deliberate), and the marks stay live through that window, so the ∞ channels are
 * still populated. If the engine went back to hiding them there, both goldens would regenerate
 * empty and every assertion below would red.
 * The `unbounded_pile → Set` hop is performed here rather than by `gameStateView.ts`, because
 * driving that function would require committing a whole `GameState`; the ids, the field name and
 * the value encoding — the parts that actually differ across the language boundary — are
 * engine-authored.
 */
import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { DerivedViews, GameObject, ObjectId, ResourceAxis } from "../../adapter/types";
import { familyOf } from "../../components/hud/HudBadges";
import { useUnboundedCounterTypes } from "../../hooks/useUnboundedCounterTypes";
import { buildGameObject } from "../../test/factories/gameObjectFactory";
import { buildGameState } from "../../test/factories/gameStateFactory";
import counterWire from "../../test/fixtures/unbounded-counter-wire.json";
import tokenWire from "../../test/fixtures/unbounded-token-wire.json";
import { setGameStoreForTest } from "../../test/helpers/gameStoreHelpers";
import { groupByName } from "../battlefieldProps";

const saproling = (id: ObjectId, tapped: boolean): GameObject =>
  buildGameObject({ id, name: "Saproling", tapped, card_id: 0, controller: 0, owner: 0 });

describe("unbounded ∞ wire seam (engine-emitted goldens)", () => {
  // RESIDUAL: this closes the ID/shape half only — the TS-side `GameObject`s are factory-built, so
  // the test cannot see an engine/client group-PARTITION mismatch, and `isUnboundedPile`'s
  // `members.every(...)` (`battlefieldProps.ts`) degrades such a mismatch silently to `×N` rather
  // than failing, which is exactly the user's symptom class.

  it("emits populated ∞ channels and omits the empty ones", () => {
    // (1) reach-guard: the engine emitted a populated pile, so the group assertions below are
    // not run against an empty set.
    expect(tokenWire.unbounded_pile).toEqual([402, 403, 404, 407]);
    // (2) reach-guard + the two counter seam facts: the map key is a JSON STRING, and
    // `CounterType` serializes FLAT ("charge", not {"Generic":"charge"}). A regressed Serialize
    // would silently blank every ∞ pill.
    expect(counterWire.unbounded_counters).toEqual({ "405": ["charge"] });
    // (3) omit-when-empty, engine-attested in BOTH directions.
    expect("unbounded_pile" in counterWire).toBe(false);
    expect("unbounded_counters" in tokenWire).toBe(false);
    // (4) the `scheduled_collapse` TAG channel, engine-emitted. Both goldens are captured after
    // the accept while a finite collapse is merely SCHEDULED, so both carry a populated tag.
    expect(tokenWire.scheduled_collapse).toEqual([{ axis: "TokensCreated", player: 0 }]);
    // (5) the tag on the second family — pins the externally-tagged `ResourceAxis` encoding
    // across the language boundary: a data variant is a single-key OBJECT, not a bare string.
    expect(counterWire.scheduled_collapse[0].axis).toEqual({ Counter: ["Other", "Other"] });
    // (6) THE TWO ENGINE ANSWERS AGREE. `scheduled_collapse` (the accepted-collapse contract) and
    // each row's `scheduled` flag (its display shadow) are computed by different loops over
    // different state, so this cross-checks one against the other: every tagged axis has a row
    // flagged scheduled, and no row is flagged without a tag naming it.
    //
    // The client no longer performs this join — it reads `row.scheduled`, because a
    // `(player, axis)` join is not even expressible for two controllers draining one victim (both
    // rows share the key). This assertion is a WIRE-CONSISTENCY check on the engine's two outputs,
    // not a reconstruction of the flag.
    //
    // THE TWO DIRECTIONS ARE NOT EQUALLY GENERAL, and this check cannot tell them apart — both
    // goldens carry a single non-mana axis, so it is these goldens that are asserted, not a law:
    //
    // - row→tag ("no row is flagged without a tag naming it") IS general, but only because both
    //   channels read the one `scheduled_display_axes` authority. It was NOT general while the
    //   `Mana(_)` scope limit lived in the tag loop alone: a mana row shipped flagged with no tag.
    //   Pinned engine-side, on a mana axis these goldens do not contain, by
    //   `loop_shortcut_mana_engine::scheduled_drive_still_renders_the_already_spendable_mana_badge`
    //   (R4/agree). If that authority is ever split in two again, that test reds, not this one.
    // - tag→row is NOT general: a token-axis row whose entire pile has left is dropped while its
    //   tag survives (the boundary still cashes that axis out), leaving an orphan tag that
    //   correctly renders nothing. Witness:
    //   `combo_infinite_pile::object_growth_infinity_row_dies_with_its_last_pile_member`.
    const agrees = (wire: { scheduled_collapse: { axis: unknown; player: number }[]; unbounded_resources: { axis: unknown; player: number; scheduled?: boolean }[] }): boolean => {
      const tagKeys = new Set(wire.scheduled_collapse.map((t) => JSON.stringify([t.player, t.axis])));
      const flaggedKeys = new Set(
        wire.unbounded_resources.filter((r) => r.scheduled).map((r) => JSON.stringify([r.player, r.axis])),
      );
      return (
        tagKeys.size > 0 &&
        flaggedKeys.size > 0 &&
        [...tagKeys].every((k) => flaggedKeys.has(k)) &&
        [...flaggedKeys].every((k) => tagKeys.has(k))
      );
    };
    expect(agrees(tokenWire as never)).toBe(true);
    expect(agrees(counterWire as never)).toBe(true);
  });

  it("drives the real groupByName pile predicate off engine ids", () => {
    const unboundedPileIds: ReadonlySet<ObjectId> = new Set(tokenWire.unbounded_pile);
    const objects: GameObject[] = [
      ...[402, 403, 404, 407].map((id) => saproling(id, true)),
      ...[406, 408, 409, 410].map((id) => saproling(id, false)),
      buildGameObject({
        id: 401,
        name: "Witherbloom, the Balancer",
        tapped: true,
        card_id: 9001,
        controller: 0,
        owner: 0,
      }),
    ];

    const groups = groupByName(objects, new Set(), unboundedPileIds);
    const groupOf = (id: ObjectId) => {
      const group = groups.find((g) => g.ids.includes(id));
      expect(group, `no group contains ${id}`).toBeDefined();
      return group!;
    };

    // NEGATIVES FIRST, POSITIVE LAST — deliberate. A failing `expect` throws and skips the rest of
    // the `it`, and the regression class this file exists to catch (the engine stops emitting the
    // pile) reds the POSITIVE. Asserting the negatives first keeps them observable as the paired
    // control in that same run instead of being skipped by the positive's throw.
    //
    // (5) paired NEGATIVE from the SAME groupByName call: same name, differs only on `tapped`.
    expect(groupOf(406).ids).toEqual([406, 408, 409, 410]);
    expect(groupOf(406).isUnboundedPile).toBe(false);
    // (6) free third negative: tapped, but not a pile member — so it is not "everything tapped".
    expect(groupOf(401).isUnboundedPile).toBe(false);
    // (4) paired POSITIVE: the tapped Saprolings the engine named.
    expect(groupOf(402).ids).toEqual([402, 403, 404, 407]);
    expect(groupOf(402).isUnboundedPile).toBe(true);
  });

  it("decodes both externally-tagged axis shapes through the real familyOf", () => {
    // (7) unit variant — a bare string on the wire.
    expect(familyOf(tokenWire.unbounded_resources[0].axis as ResourceAxis)).toBe("tokens");
    // (8) data variant — a single-key object on the wire.
    expect(
      familyOf(counterWire.unbounded_resources[0].axis as unknown as ResourceAxis),
    ).toBe("counters");
    // (9) redundant reinforcement, kept as documentation of intent: it cannot fail unless (7) or
    // (8) already has.
    expect(familyOf(tokenWire.unbounded_resources[0].axis as ResourceAxis)).not.toBe(
      familyOf(counterWire.unbounded_resources[0].axis as unknown as ResourceAxis),
    );
  });

  it("feeds the real useUnboundedCounterTypes hook from the engine wire", () => {
    setGameStoreForTest({
      gameState: buildGameState({ derived: counterWire as unknown as DerivedViews }),
    });
    // (10) paired POSITIVE through the real zustand selector.
    expect(renderHook(() => useUnboundedCounterTypes(405)).result.current).toEqual(["charge"]);
    // (11) paired NEGATIVE: 404 is on the same battlefield and carries no ∞ mark.
    expect(renderHook(() => useUnboundedCounterTypes(404)).result.current).toEqual([]);
  });
});
