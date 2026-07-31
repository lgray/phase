#!/usr/bin/env bash
# Regenerates a saved-game test fixture under crates/engine/tests/fixtures/ from
# its READ-ONLY pristine dump, stamping the `effect_kind` field that upstream
# #6718 (0468df1f4) added to `TargetSelectionSlot` without `#[serde(default)]`.
#
# WHY A REGENERATION AND NOT A SERDE SHIM. The maintainer publicly declined both
# `#[serde(default)]` and an upstream save migration for this field
# (https://github.com/phase-rs/phase/pull/6718#issuecomment-5111207689 — "alpha
# means it may not load ... if you have a use-case then do the save changes
# locally"). Migrating the fixture locally is the maintainer's own named path,
# and it keeps the production decoder STRICT: an un-migrated save must still be
# rejected, which a serde default would silently prevent.
#
# WHY `--effect-kind` IS AN EXPLICIT ARGUMENT and never a jq name->variant table:
# such a table would re-derive `impl From<&Effect> for EffectKind` in jq, and
# that mapping is not the identity (`Effect::SetTapState` fans out to several
# kinds). The ENGINE stays the authority for the migrated value; the reading
# test beside `load_dellian_dump` in `game/engine.rs` asserts the stamped slots
# equal what `ability_utils::build_target_slots` builds for that board, so a
# wrong `--effect-kind` argument fails a tracked row rather than shipping.
#
# The pristine directory is READ-ONLY: this script only ever reads from it.
#
# Usage:
#   scripts/migrate-dump-fixture.sh \
#     --pristine  /path/to/dump.zip \
#     --expect-sha256 <sha256 of that zip> \
#     --effect-kind LoseLife \
#     --out crates/engine/tests/fixtures/name.json.gz
#
#   # Control mode: re-run the FULL recipe and check it against the committed
#   # fixture, then check the patch had teeth. Runnable by anyone, at any time,
#   # with no engine build.
#   scripts/migrate-dump-fixture.sh --pristine ... --expect-sha256 ... \
#     --effect-kind LoseLife \
#     --out crates/engine/tests/fixtures/name.json.gz --control
#
# BOTH control arms matter, and a one-arm check passes vacuously:
#   arm 1 => BYTE_IDENTICAL=true  the PATCHED regeneration reproduces the
#            committed fixture byte for byte, so the committed bytes are exactly
#            what this recipe produces from the read-only pristine dump.
#   arm 2 => PATCHED_DIFFERS=true the same recipe run WITHOUT the patch differs
#            from arm 1's output, so the jq filter actually REACHES target_slots.
#            Without this arm, a filter that silently matched nothing would also
#            report BYTE_IDENTICAL=true.
#
# ⚠ ARM 1 IS BASELINED ON THE MIGRATED FIXTURE, and it has to be. The committed
# fixture IS the patched artifact; comparing an UNPATCHED regeneration against it
# fails by construction post-migration (measured: BYTE_IDENTICAL=false, exit 1),
# which reads as "the fixture is corrupt" when it means "migrated, as designed".
# So the unpatched regeneration is arm 2's operand, never arm 1's expectation.
#
# TOOLCHAIN. Byte-identity is toolchain-coupled: gzip's deflate output and jq's
# key ordering are implementation details, not standards. The recipe below was
# established under the pinned versions; on any other version the control falls
# back to a canonical `jq -S` content comparison, which is toolchain-independent
# and still discriminating (it just cannot certify byte equality).

set -euo pipefail

PINNED_JQ="jq-1.7.1"
PINNED_GZIP="gzip 1.14"

PRISTINE=""
EXPECT_SHA=""
EFFECT_KIND=""
OUT=""
CONTROL_MODE=0

usage() {
  sed -n '2,57p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit "${1:-1}"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --pristine)      PRISTINE="${2:?--pristine needs a path}"; shift 2 ;;
    --expect-sha256) EXPECT_SHA="${2:?--expect-sha256 needs a hash}"; shift 2 ;;
    --effect-kind)   EFFECT_KIND="${2:?--effect-kind needs an EffectKind variant name}"; shift 2 ;;
    --out)           OUT="${2:?--out needs a path}"; shift 2 ;;
    --control)       CONTROL_MODE=1; shift ;;
    -h|--help)       usage 0 ;;
    *) echo "unknown argument: $1" >&2; usage 1 ;;
  esac
done

[ -n "$PRISTINE" ]   || { echo "missing --pristine" >&2; exit 1; }
[ -n "$EXPECT_SHA" ] || { echo "missing --expect-sha256" >&2; exit 1; }
[ -n "$OUT" ]        || { echo "missing --out" >&2; exit 1; }
# Control mode needs --effect-kind too: arm 1 re-runs the FULL recipe, patch
# included, because the committed fixture is the patched artifact.
[ -n "$EFFECT_KIND" ] || { echo "missing --effect-kind" >&2; exit 1; }

for tool in unzip jq gzip sha256sum; do
  command -v "$tool" >/dev/null 2>&1 || { echo "required tool not found: $tool" >&2; exit 1; }
done

# 1. Verify the pristine input. Abort rather than migrate an unexpected dump —
#    fixture<->dump correspondence is by CONTENT, never by filename (the name
#    trap is real: witherbloom-sprout-lumaret-works-slow.zip maps to
#    witherbloom_sprout_lumaret_SIMPLE_4p.json.gz).
ACTUAL_SHA="$(sha256sum "$PRISTINE" | cut -d' ' -f1)"
if [ "$ACTUAL_SHA" != "$EXPECT_SHA" ]; then
  echo "pristine sha256 mismatch for $PRISTINE" >&2
  echo "  expected: $EXPECT_SHA" >&2
  echo "  actual:   $ACTUAL_SHA" >&2
  exit 1
fi

JQ_VERSION="$(jq --version)"
GZIP_VERSION="$(gzip --version | head -1)"

# 2. Patch + 3. compress. ONE filter, applied to every slot in the prompt.
#    ONE definition of the recipe, used by BOTH the migration and the control —
#    a control that re-spelled the recipe would certify its own copy.
regenerate() {   # regenerate <patched|unpatched> <destination>
  local mode="$1" dest="$2" filter='{gameState:.gameState}'
  if [ "$mode" = patched ]; then
    filter='.gameState.waiting_for.data.target_slots |= map(. + {effect_kind: $k}) | {gameState:.gameState}'
  fi
  mkdir -p "$(dirname "$dest")"
  unzip -p "$PRISTINE" | jq -c --arg k "$EFFECT_KIND" "$filter" | gzip -9 -n > "$dest"
}

if [ "$CONTROL_MODE" -eq 1 ]; then
  # 5. Control mode, TWO arms, both mandatory. Runnable by anyone, at any time,
  #    with no engine build. Arm 1 re-runs the full recipe and holds it against
  #    the committed fixture; arm 2 re-runs it WITHOUT the patch and requires the
  #    result to differ, which is what proves the patch filter has teeth.
  [ -f "$OUT" ] || { echo "control mode needs an existing committed fixture at $OUT" >&2; exit 1; }
  PATCHED="$(mktemp -t migrate-dump-patched-XXXXXX.json.gz)"
  UNPATCHED="$(mktemp -t migrate-dump-unpatched-XXXXXX.json.gz)"
  trap 'rm -f "$PATCHED" "$UNPATCHED"' EXIT
  regenerate patched   "$PATCHED"
  regenerate unpatched "$UNPATCHED"

  echo "CONTROL pristine=$(basename "$PRISTINE") sha256=$ACTUAL_SHA"
  echo "CONTROL effect_kind=$EFFECT_KIND out=$OUT"
  echo "CONTROL jq=$JQ_VERSION gzip=$GZIP_VERSION"

  # ARM 1 — the patched regeneration reproduces the committed fixture.
  case "$JQ_VERSION:$GZIP_VERSION" in
    "$PINNED_JQ:$PINNED_GZIP"*)
      if cmp -s "$PATCHED" "$OUT"; then
        echo "CONTROL BYTE_IDENTICAL=true"
      else
        echo "CONTROL BYTE_IDENTICAL=false" >&2
        exit 1
      fi
      ;;
    *)
      # Toolchain drift: byte equality is not certifiable, but content equality
      # is, and it still catches a recipe that reads the wrong dump.
      echo "CONTROL toolchain differs from pinned ($PINNED_JQ / $PINNED_GZIP) — falling back to canonical content comparison"
      if [ "$(gzip -dc "$PATCHED" | jq -S -c .)" = "$(gzip -dc "$OUT" | jq -S -c .)" ]; then
        echo "CONTROL CANONICALLY_EQUAL=true BYTE_IDENTICAL=unknown"
      else
        echo "CONTROL CANONICALLY_EQUAL=false" >&2
        exit 1
      fi
      ;;
  esac

  # ARM 2 — the patch reached target_slots. Compared as canonical CONTENT, not
  # bytes: this arm asserts a DIFFERENCE, and a difference in compressed bytes
  # alone would also be produced by gzip drift, which is not what it claims.
  if [ "$(gzip -dc "$PATCHED" | jq -S -c .)" = "$(gzip -dc "$UNPATCHED" | jq -S -c .)" ]; then
    echo "CONTROL PATCHED_DIFFERS=false — the jq filter matched nothing; arm 1 above would pass vacuously" >&2
    exit 1
  fi
  echo "CONTROL PATCHED_DIFFERS=true stamped=$(gzip -dc "$PATCHED" | jq -c '[.gameState.waiting_for.data.target_slots[]?.effect_kind]') unpatched=$(gzip -dc "$UNPATCHED" | jq -c '[.gameState.waiting_for.data.target_slots[]?.effect_kind]')"
  exit 0
fi

regenerate patched "$OUT"

OUT_SHA="$(sha256sum "$OUT" | cut -d' ' -f1)"
SLOTS="$(gzip -dc "$OUT" | jq -c '[.gameState.waiting_for.data.target_slots[]?.effect_kind]')"

# 4. Record the provenance on stdout so a commit message can quote it.
echo "MIGRATED pristine=$(basename "$PRISTINE") sha256=$ACTUAL_SHA"
echo "MIGRATED effect_kind=$EFFECT_KIND stamped_slots=$SLOTS"
echo "MIGRATED out=$OUT sha256=$OUT_SHA"
echo "MIGRATED jq=$JQ_VERSION gzip=$GZIP_VERSION"
