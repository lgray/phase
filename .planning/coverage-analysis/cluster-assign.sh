#!/usr/bin/env bash
# cluster-assign.sh <tag> — assign EVERY unsupported card in out/<tag>/ to exactly one
# implementation cluster via an ordered ruleset over full oracle_text + gap handler.
# No card is left unassigned: the policy is "all cards must be implemented" — the
# ruleset has named clusters for the hard cases too (heavy-infra clusters still get a
# home; they're gated by /review-engine-plan, not dropped). S99-UNCLUSTERED must be 0.
#
# Run coverage-breakdown.sh --<filter> first to produce out/<tag>/unsupported.tsv.
# Output: out/<tag>/cluster-assignment.tsv  (name \t gap_count \t handler \t cluster)
#         + a printed cluster summary.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
COV="$REPO/data/coverage-data.json"
TAG="${1:?usage: cluster-assign.sh <tag>  (e.g. standard, MSH, modern+commander)}"
OUT="$HERE/out/$TAG"
[ -f "$OUT/unsupported.tsv" ] || { echo "missing $OUT/unsupported.tsv (run coverage-breakdown.sh first)" >&2; exit 1; }

# name \t gap_count \t handler-set \t full-oracle(newlines->/)
cut -f1 "$OUT/unsupported.tsv" > "$OUT/.names.tmp"
jq -r --slurpfile names <(jq -R . "$OUT/.names.tmp" | jq -s .) '
  ($names[0]|INDEX(.)) as $set
  | .cards[] | select(.card_name as $n | $set[$n]!=null)
  | [ .card_name, (.gap_count|tostring),
      ([.gap_details[]?|.handler]|unique|join(";")),
      ((.oracle_text//"")|gsub("[\n\t]";" / ")) ] | @tsv
' "$COV" > "$OUT/oracle-full.tsv"
rm -f "$OUT/.names.tmp"

awk -F'\t' -f - "$OUT/oracle-full.tsv" > "$OUT/cluster-assignment.tsv" <<'AWK'
function classify(g, h, o,   lo) {
  lo = tolower(o)
  # ---- resolver-flagged (parses fully; runtime/silent-drop gap) ----
  if (g == "0") {
    if (lo ~ /start your engines|max speed/)              return "R1-speed-mechanic"
    if (lo ~ /level up/)                                  return "R3-level-up"
    if (lo ~ /as long as/)                                return "R2-aslongas-conditional-static"
    if (lo ~ /can.?t (attack|block|be|have|cast|activate)/) return "R4-cant-restriction-static"
    return "R5-runtime-bespoke"
  }
  # ---- parser-gap ----
  if (h ~ /(^|;)Cost:/)             return "S23-alt-cost-parse"
  if (h ~ /AlternativeKeywordCost/) return "S26-alt-keyword-cost"
  if (h ~ /target-fallback/)        return "S17-anaphoric-target"
  if (h ~ /trigger-subject/)        return "S27-trigger-subject-anaphora"
  if (h ~ /Condition_If/) {
    if (lo ~ /activate only if/)                                                          return "S04-activate-only-if"
    if (lo ~ /rather than pay|you may pay [{][^}]*[}] rather|costs [{][^}]*[}] less to cast if|cast this (spell|card) (for|as though|from)|cast .* from your graveyard (for|if|by|in)|as though it had flash if|you may cast this card from your graveyard/) return "S05-alt-cost-if"
    if (lo ~ /add a lore counter|\(as this saga/)                                         return "S06-saga-chapter-if"
    if (lo ~ /if (this spell|it) (was|wasn.?t) cast|if you.?(ve)? cast|mana was spent to cast|if it was kicked/) return "S02-cast-context-if"
    if (lo ~ /whenever[^.\/]*, if |when [^.\/]*enters?, if |at the beginning[^.\/]*, if /) return "S03-intervening-if-trigger"
    if (lo ~ /(^| |\/ )if (it|that creature|that card|that permanent|that spell|that token|he|she|they|excess|its )/) return "S01-reflexive-if-rider"
    return "S07-condition-if-bespoke"
  }
  if (h ~ /DynamicQty/) {
    if (lo ~ /prime number|consecutive/)  return "S09-advanced-count-qty"
    if (lo ~ /for each/)                   return "S08-foreach-count-qty"
    return "S10-dynamic-qty-bespoke"
  }
  if (h ~ /Duration_This|Duration_Until/) return "S11-duration-grant"
  if (h ~ /Optional_YouMay/)        return "S12-optional-youmay-subeffect"
  if (h ~ /Condition_AsLongAs/)     return "S13-aslongas-parse"
  if (h ~ /Condition_Unless/ || h ~ /Unsupported unless/) return "S14-unless-clause"
  if (h ~ /ActivateOnlyDuring/)     return "S15-activate-only-during"
  if (h ~ /(^|;)Effect:for/) {
    if (lo ~ /for each (other )?(player|opponent)|each (other )?player|each player (who|chooses)/) return "S16-foreach-player-object-HEAVY"
    return "S18-foreach-simple-count"
  }
  if (h ~ /(^|;)Trigger:/)          return "S19-new-trigger-matcher"
  if (h ~ /orphaned_copy_retarget/) return "S20-copy-retarget"
  if (h ~ /static_structure/)       return "S21-static-ability"
  if (h ~ /Effect:choose/)          return "S22-choose-effect"
  if (h ~ /Effect:unknown/)         return "S24-unknown-effect-bespoke"
  if (h ~ /(^|;)Effect:/)           return "S25-effect-verb-bespoke"
  return "S99-UNCLUSTERED"
}
{ print $1 "\t" $2 "\t" $3 "\t" classify($2,$3,$4) }
AWK

echo "=== cluster assignment: $TAG ($(wc -l < "$OUT/cluster-assignment.tsv" | tr -d ' ') cards) ==="
cut -f4 "$OUT/cluster-assignment.tsv" | sort | uniq -c | sort -rn
UNC=$(awk -F'\t' '$4=="S99-UNCLUSTERED"' "$OUT/cluster-assignment.tsv" | wc -l | tr -d ' ')
echo "UNCLUSTERED (must be 0): $UNC"
echo "[written] $OUT/cluster-assignment.tsv"
