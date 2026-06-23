#!/usr/bin/env bash
# coverage-breakdown.sh — coverage gap analysis for a phase.rs card group.
#
# Slices the engine-authoritative coverage report (data/coverage-data.json) by
# either a format-legality filter or a set-membership filter, then classifies
# every UNSUPPORTED card in that group by the coverage tool's own gap handler.
#
# Two distinct unsupported categories are separated, because they need different
# work:
#   * parser gap   (gap_count > 0): the parser dropped/failed a clause. The gap
#                  handler comes from `gap_details[].handler` (e.g.
#                  Swallow:Condition_If, Effect:for, Effect:unknown).
#   * resolver-flagged (gap_count == 0): the card parses fully (all
#                  parse_details supported) but the coverage tool's silent-drop /
#                  resolver audit still marks it unsupported — i.e. it needs
#                  RUNTIME work, not parser work.
#
# DATA SHAPE NOTES (verified 2026-06-22, do not re-derive):
#   data/card-data.json      : object keyed by LOWERCASED name; .value.name is
#                              the display name; .value.legalities.<fmt>=="legal";
#                              .value.printings is a [set_code] array.
#   data/coverage-data.json  : .cards[] keyed by DISPLAY name (.card_name);
#                              .supported (global bool); .gap_count;
#                              .gap_details[] = {handler, source_text};
#                              .parse_details[] (recursive, has .children);
#                              .coverage_by_format.<fmt> = {total,supported,pct}.
#   JOIN KEY: coverage .card_name == card-data .value.name (display name).
#
# Refresh inputs first with ./scripts/gen-card-data.sh (needs nightly cargo:
#   export PATH="$HOME/.cargo/bin:$PATH").
#
# Filters are repeatable and INTERSECTED (AND): a card is a group member only if
# it is legal in EVERY --format AND printed in EVERY --set given. With one filter
# this is just that format/set; with several it is the intersection.
#
# Usage:
#   .planning/coverage-analysis/coverage-breakdown.sh --format standard
#   .planning/coverage-analysis/coverage-breakdown.sh --format modern --format commander
#   .planning/coverage-analysis/coverage-breakdown.sh --set MSH --top 40
#   .planning/coverage-analysis/coverage-breakdown.sh --format pioneer --set SPM
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
COV="$REPO/data/coverage-data.json"
CARD="$REPO/data/card-data.json"
OUTROOT="$HERE/out"

FMTS=(); SETS=(); TOP=25
while [ $# -gt 0 ]; do
  case "$1" in
    --format) FMTS+=("${2:?--format needs a format name}"); shift 2;;
    --set)    SETS+=("${2:?--set needs a set code}");        shift 2;;
    --top)    TOP="${2:?--top needs a number}";              shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
[ "${#FMTS[@]}" -gt 0 ] || [ "${#SETS[@]}" -gt 0 ] || {
  echo "usage: $0 [--format <fmt>]... [--set <CODE>]... [--top N]  (filters are intersected)" >&2; exit 2; }
[ -f "$COV" ]  || { echo "missing $COV (run ./scripts/gen-card-data.sh)" >&2; exit 1; }
[ -f "$CARD" ] || { echo "missing $CARD (run ./scripts/gen-card-data.sh)" >&2; exit 1; }

# JSON arrays of the requested filters ([] when none); TAG = filters joined by '+'.
FMTS_JSON=$(printf '%s\n' "${FMTS[@]+"${FMTS[@]}"}" | jq -R 'select(length>0)' | jq -cs .)
SETS_JSON=$(printf '%s\n' "${SETS[@]+"${SETS[@]}"}" | jq -R 'select(length>0)' | jq -cs .)
TAG=$(printf '%s\n' "${FMTS[@]+"${FMTS[@]}"}" "${SETS[@]+"${SETS[@]}"}" | sed '/^$/d' | paste -sd+ -)
OUT="$OUTROOT/$TAG"
mkdir -p "$OUT"

# 1. Membership: display names legal in EVERY format AND printed in EVERY set
#    (empty filter list => `all` is vacuously true, so it imposes no constraint).
jq -r --argjson fmts "$FMTS_JSON" --argjson sets "$SETS_JSON" '
  .[] as $c
  | select(
      ($fmts | all(($c.legalities[.] // "") == "legal"))
      and
      ($sets | all(. as $s | (($c.printings // []) | index($s)) != null))
    )
  | $c.name
' "$CARD" | sort -u > "$OUT/members.txt"
MEMBERS=$(wc -l < "$OUT/members.txt" | tr -d ' ')

# 2. Per-card unsupported detail across ALL cards (cached at out root; cheap to rebuild).
#    Columns: name \t gap_count \t printings(;) \t handlers(;) \t first-source snippet(160)
ALL="$OUTROOT/all-unsup-detail.tsv"
if [ ! -f "$ALL" ] || [ "$COV" -nt "$ALL" ]; then
  jq -r '.cards[] | select(.supported==false)
    | (.card_name // "?") as $n
    | (.gap_details // []) as $gd
    | ([ $gd[] | (.handler // "?") ] | unique | join(";")) as $handlers
    | ([ $gd[] | (.source_text // "") | gsub("\\[warning:swallowed-clause\\] ";"") | gsub("[\n\t\r]";" ") ]
        | map(select(.!="")) | .[0] // "") as $src
    | [ $n, (.gap_count|tostring), ((.printings // [])|join(";")),
        (if $handlers=="" then "NO-GAP-DETAILS" else $handlers end), ($src[0:160]) ] | @tsv
  ' "$COV" > "$ALL"
fi

# 3. Restrict to this group's members (set membership on the display-name key).
awk -F'\t' 'NR==FNR{m[$0]=1; next} ($1 in m)' "$OUT/members.txt" "$ALL" | sort > "$OUT/unsupported.tsv"
UNSUP=$(wc -l < "$OUT/unsupported.tsv" | tr -d ' ')
SUP=$(( MEMBERS - UNSUP ))
PCT=$(awk -v s="$SUP" -v m="$MEMBERS" 'BEGIN{ if(m>0) printf "%.2f", 100*s/m; else print "n/a" }')

# Split parser-gap (gap_count>0) vs resolver-flagged (gap_count==0).
awk -F'\t' '$2!="0"' "$OUT/unsupported.tsv" > "$OUT/parser-gap.tsv"
awk -F'\t' '$2=="0"' "$OUT/unsupported.tsv" > "$OUT/resolver-flagged.tsv"
PGAP=$(wc -l < "$OUT/parser-gap.tsv" | tr -d ' ')
RFLAG=$(wc -l < "$OUT/resolver-flagged.tsv" | tr -d ' ')

# 4. Report.
{
echo "================================================================"
echo " coverage breakdown: $TAG"
echo " inputs: $(jq -r '.[]?|.name' "$CARD" >/dev/null 2>&1; date -u +%Y-%m-%dT%H:%M:%SZ) (coverage-data + card-data)"
echo "================================================================"
echo "members (in group)        : $MEMBERS"
echo "supported                 : $SUP  (${PCT}%)"
echo "unsupported               : $UNSUP"
echo "  parser gap (gap_count>0): $PGAP   -> handler buckets below"
echo "  resolver-flagged (==0)  : $RFLAG   -> parses fully, runtime-incomplete"
echo
echo "--- gap-handler histogram (parser-gap cards; per handler occurrence) ---"
cut -f4 "$OUT/parser-gap.tsv" | tr ';' '\n' | sort | uniq -c | sort -rn | head -"$TOP"
echo
echo "--- resolver-flagged cards (parse OK, need runtime work) ---"
cut -f1 "$OUT/resolver-flagged.tsv"
echo
echo "--- parser-gap cards grouped by handler-set (name | snippet) ---"
sort -t$'\t' -k4,4 -k1,1 "$OUT/parser-gap.tsv" \
  | awk -F'\t' '{printf "%-30s | %-28s | %s\n", substr($4,1,30), substr($1,1,28), substr($5,1,84)}'
} | tee "$OUT/report.txt"

echo
echo "[written] $OUT/{members.txt,unsupported.tsv,parser-gap.tsv,resolver-flagged.tsv,report.txt}"
