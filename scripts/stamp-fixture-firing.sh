#!/usr/bin/env bash
# Stamps the CR 603.7 `TriggerFiring` carriers that upstream #6842 (8121fd1c6)
# made mandatory onto an ALREADY-COMMITTED fixture, in place.
#
# WHY IN PLACE AND NOT A PRISTINE REGENERATION. `migrate-dump-fixture.sh`
# regenerates from the read-only pristine root, which is the stronger provenance
# and is preferred where it applies. It does NOT apply to every fixture in this
# corpus: measured, `dina_conqueror_4p` and `witherbloom_sprout_lumaret_simple_4p`
# differ from their pristine regeneration in exactly one object each (Priest of
# Forgotten Gods' `abilities` / `base_abilities` AST), because the committed
# fixture carries a LATER parser state than the 2026-07-22/25 capture. Rerunning
# those from pristine would silently REVERT that. Stamping in place is additive
# and cannot revert anything, and its arm-1 control is strictly stronger: the
# stamped artifact minus the three new keys must be BYTE-IDENTICAL to what was
# committed, which proves zero collateral change.
#
# The derivation is NOT re-spelled here — it is loaded from
# scripts/lib/trigger-firing.jq, the same single definition
# `migrate-dump-fixture.sh` uses. See that file for the CR 603.1 vs CR 603.7a
# discriminant and for why `UnknownLegacy` is not a legal persisted value.
#
# Usage:
#   scripts/stamp-fixture-firing.sh crates/engine/tests/fixtures/name.json.gz [...]
#   scripts/stamp-fixture-firing.sh --control crates/engine/tests/fixtures/name.json.gz
#
# It stamps TWO field classes, both made mandatory-or-repaired by #6842:
#   1. the CR 603.7 firing carriers themselves; and
#   2. the CR 603.7 delayed-trigger ALLOCATORS
#      (`next_delayed_trigger_token` / `..._instance`), which #6842 repairs at
#      load time on the PRODUCTION decode path only. Left unstamped, a legacy
#      dump restores 0 through a bare `GameState` decode and 1 through the
#      production decoder, so the two paths disagree — and 0 is the value the
#      engine's own coherence validator rejects. Stamping the repaired value on
#      disk keeps the decoders in agreement WITHOUT weakening any assertion.
#
# ALL THREE control arms, and a one- or two-arm check passes vacuously:
#   arm 1 => NO_COLLATERAL=true        stamped minus the 5 stamped keys is
#            byte-identical to the committed fixture, so nothing else moved.
#   arm 2 => CARRIERS_ADDED=true       every firing carrier the dump needs is
#            present (got == need). Keyed on CARRIER COUNT, not on byte
#            difference: gzip/jq re-serialization alone changes bytes without
#            stamping anything, which is the stale-artifact false pass.
#   arm 3 => ALLOCATORS_CANONICAL=true both allocators exist and are >= 1.

set -euo pipefail

CONTROL=0
[ "${1:-}" = "--control" ] && { CONTROL=1; shift; }
[ $# -gt 0 ] || { echo "usage: $0 [--control] <fixture.json.gz>..." >&2; exit 1; }

LIB="$(dirname "${BASH_SOURCE[0]}")/lib/trigger-firing.jq"
[ -f "$LIB" ] || { echo "missing $LIB" >&2; exit 1; }

for tool in jq gzip sha256sum; do
  command -v "$tool" >/dev/null 2>&1 || { echo "required tool not found: $tool" >&2; exit 1; }
done

# Every key this script is allowed to add. Arm 1 deletes exactly these from both
# sides, so anything else that moved shows up as a collateral change.
CARRIERS='del(.gameState.pending_trigger_firing, .gameState.stack_trigger_firings, .gameState.resolving_trigger_firing,
              .gameState.next_delayed_trigger_token, .gameState.next_delayed_trigger_instance)'

rc=0
for FIX in "$@"; do
  [ -f "$FIX" ] || { echo "no such fixture: $FIX" >&2; rc=1; continue; }
  TMP="$(mktemp -t stamp-firing-XXXXXX.json.gz)"
  # `-f` with the lib prepended keeps ONE definition of the derivation.
  if ! gzip -dc "$FIX" \
      | jq -c -f <(printf '%s\nstamp_trigger_firing | stamp_delayed_allocators\n' "$(cat "$LIB")") \
      | gzip -9 -n > "$TMP"; then
    echo "STAMP FAILED (fail-closed, nothing written): $FIX" >&2
    rm -f "$TMP"; rc=1; continue
  fi

  # How many carriers this dump NEEDS, read from the dump itself. A dump that
  # needs none is skipped outright: stamping it is a no-op, and a "the bytes
  # changed" arm over it would be reporting jq re-serialization rather than a
  # stamp — the stale-artifact false pass, inverted.
  NEED="$(gzip -dc "$FIX" | jq -c -f <(printf '%s\ntrigger_carrier_count\n' "$(cat "$LIB")"))"
  GOT="$(gzip -dc "$TMP" | jq -c '((if .gameState.pending_trigger_firing then 1 else 0 end)
                                   + (.gameState.stack_trigger_firings // {} | length)
                                   + (if .gameState.resolving_trigger_firing then 1 else 0 end))')"
  SUMMARY="$(gzip -dc "$TMP" | jq -c '{pending: .gameState.pending_trigger_firing,
                                       stack: (.gameState.stack_trigger_firings // {} | length),
                                       resolving: .gameState.resolving_trigger_firing}')"

  # The allocator repair is a SEPARATE need from the firing carriers: a dump can
  # want one and not the other, so a dump with no triggered record is only truly
  # a no-op when its allocators are already at or above 1.
  ALLOC_NEED="$(gzip -dc "$FIX" | jq -c 'if ((.gameState.next_delayed_trigger_token // 0) < 1)
                                            or ((.gameState.next_delayed_trigger_instance // 0) < 1)
                                         then 1 else 0 end')"
  ALLOC_GOT="$(gzip -dc "$TMP" | jq -c '{tok: .gameState.next_delayed_trigger_token,
                                         inst: .gameState.next_delayed_trigger_instance}')"

  if [ "$NEED" -eq 0 ] && [ "$ALLOC_NEED" -eq 0 ]; then
    echo "SKIP  $(basename "$FIX") needs=0 carriers, allocators already canonical — nothing to stamp"
    rm -f "$TMP"; continue
  fi

  # arm 1 — no collateral change: everything except the carrier keys is identical.
  A="$(gzip -dc "$TMP" | jq -S -c "$CARRIERS")"
  B="$(gzip -dc "$FIX" | jq -S -c "$CARRIERS")"
  if [ "$A" = "$B" ]; then ARM1=true; else ARM1=false; fi
  # arm 2 — the stamp had teeth: every carrier the dump needs is now present.
  # Keyed on CARRIER COUNT, not on byte difference, because gzip/jq
  # re-serialization alone can change bytes without stamping anything.
  if [ "$GOT" -eq "$NEED" ]; then ARM2=true; else ARM2=false; fi
  # arm 3 — the allocator repair landed. Both fields must exist and be >= 1;
  # 0 is the value the engine's own coherence validator rejects.
  if [ "$(gzip -dc "$TMP" | jq -c 'if ((.gameState.next_delayed_trigger_token // 0) >= 1)
                                      and ((.gameState.next_delayed_trigger_instance // 0) >= 1)
                                   then "true" else "false" end')" = '"true"' ]
  then ARM3=true; else ARM3=false; fi

  echo "STAMP $(basename "$FIX") carriers=$SUMMARY needs=$NEED got=$GOT alloc_need=$ALLOC_NEED alloc=$ALLOC_GOT NO_COLLATERAL=$ARM1 CARRIERS_ADDED=$ARM2 ALLOCATORS_CANONICAL=$ARM3"

  if [ "$ARM1" != true ] || [ "$ARM2" != true ] || [ "$ARM3" != true ]; then
    echo "  control arms failed for $FIX — not writing" >&2
    rm -f "$TMP"; rc=1; continue
  fi

  if [ "$CONTROL" -eq 1 ]; then
    rm -f "$TMP"
  else
    mv "$TMP" "$FIX"
    echo "  wrote $FIX sha256=$(sha256sum "$FIX" | cut -d' ' -f1)"
  fi
done
exit $rc
