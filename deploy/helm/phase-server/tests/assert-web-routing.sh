#!/usr/bin/env bash
set -euo pipefail

# Asserts the routing contract that `web.enabled` depends on: the SPA takes the
# catch-all, and every HTTP endpoint the server actually mounts still reaches the
# server. Renders the chart itself rather than taking pre-rendered files (as
# assert-compression-boundary.sh does) because the server-surface check below
# reads the Rust router, so the script is repo-rooted either way.

chart_dir=$(cd "$(dirname "$0")/.." && pwd)
repo_root=$(cd "$chart_dir/../../.." && pwd)
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

render() {
  local out=$1; shift
  helm template phase-server "$chart_dir" \
    --set ingress.host=phase.example.test \
    --set ingress.tls.clusterIssuer=letsencrypt \
    --set server.adminTokenSecret=phase-admin \
    "$@" > "$out"
}

render "$work_dir/web.yaml" --set web.enabled=true
render "$work_dir/web-scaleout.yaml" --set web.enabled=true --set scaleOut.enabled=true
render "$work_dir/noweb.yaml"

extract_doc() {
  awk -v kind="$1" -v name="$2" '
    BEGIN { RS = "---"; ORS = "" }
    $0 ~ "kind: " kind "\\n" && $0 ~ "metadata:\\n  name: " name "\\n" { print }
  ' "$3"
}

fail() { echo "assert-web-routing: $*" >&2; exit 1; }

# ── The server's real HTTP surface ──────────────────────────────────────────
# Walked from the router rather than recalled, so a route added to the server
# without a matching ingress rule fails here instead of being silently swallowed
# by the SPA catch-all. `/metrics` is deliberately absent: it is mounted on a
# separate listener outside build_router and must not be publicly routed.
server_src="$repo_root/crates/phase-server/src/main.rs"
prefixes=$(
  { awk '/^fn build_router\(/,/^}/' "$server_src"
    awk '/^fn mount_admin_routes\(/,/^}/' "$server_src"; } |
    tr '\n' ' ' |
    grep -oE '\.route\( *"[^"]+"' |
    grep -oE '"[^"]+"' | tr -d '"' |
    sed 's|\(/[^/]*\).*|\1|' | sort -u
)
# Live instrument: an extractor that silently stopped matching would otherwise
# report an empty surface, and every "is it routed?" check below would pass
# vacuously.
grep -qx '/ws' <<<"$prefixes" || fail "route extractor found no /ws — it is broken, not the chart ($(tr '\n' ' ' <<<"$prefixes"))"
[ "$(wc -l <<<"$prefixes")" -ge 3 ] || fail "route extractor found only $(wc -l <<<"$prefixes") prefixes: $(tr '\n' ' ' <<<"$prefixes")"
echo "server HTTP prefixes: $(tr '\n' ' ' <<<"$prefixes")"

while IFS= read -r prefix; do
  grep -q "path: $prefix\$" "$work_dir/web.yaml" ||
    fail "$prefix is mounted by the server but no Ingress routes it — the SPA catch-all would swallow it"
  grep -qF "PathPrefix(\`$prefix\`)" "$work_dir/web-scaleout.yaml" ||
    fail "$prefix is mounted by the server but no IngressRoute rule routes it under scaleOut"
done <<<"$prefixes"

# ── Plain Ingress topology: SPA on "/", server endpoints on the server port ──
# Ingress matching is longest-prefix, so "/" losing to every endpoint above is
# structural rather than an ordering we have to assert.
web_ing="$work_dir/web-ingress.yaml"
extract_doc Ingress phase-server-web "$work_dir/web.yaml" > "$web_ing"
test -s "$web_ing" || fail "web.enabled rendered no phase-server-web Ingress"
grep -q 'path: /$' "$web_ing" || fail "phase-server-web does not take the / catch-all"
grep -q 'name: web$' "$web_ing" || fail "phase-server-web does not target the web Service port"
for name in phase-server phase-server-ws phase-server-backup; do
  extract_doc Ingress "$name" "$work_dir/web.yaml" | grep -q 'name: http$' ||
    fail "$name stopped targeting the server port with web.enabled"
done

# ── scaleOut IngressRoute: catch-all is the SPA, every longer rule is the server ──
# Traefik sorts routers on an entrypoint by descending rule-string length when no
# `priority:` is set, so "longer than the catch-all" IS the priority contract.
# See templates/ingressroute.yaml for why no explicit priority is ever pinned.
entry="$work_dir/entry.yaml"
extract_doc IngressRoute phase-server "$work_dir/web-scaleout.yaml" > "$entry"
test -s "$entry" || fail "no entry IngressRoute rendered"
if grep -q '^      priority:' "$entry"; then
  fail "entry IngressRoute pins an explicit priority — see templates/ingressroute.yaml"
fi

awk '
  /^    - kind: Rule/ { if (m != "") print m "\t" sticky "\t" port; m=""; sticky="no"; port=""; next }
  /^      match: / { m = substr($0, 14) }
  /^          sticky:/ { sticky = "yes" }
  /^          port: / { port = $2 }
  END { if (m != "") print m "\t" sticky "\t" port }
' "$entry" > "$work_dir/rules.tsv"

catchall=$(awk -F'\t' '$1 !~ / && / { print $1 }' "$work_dir/rules.tsv")
[ -n "$catchall" ] || fail "entry IngressRoute has no catch-all rule"
[ "$(wc -l <<<"$catchall")" -eq 1 ] || fail "entry IngressRoute has more than one catch-all rule: $catchall"

web_port=$(extract_doc Service phase-server "$work_dir/web.yaml" |
  awk '/- name: web$/{found=1} found && /port: /{print $2; exit}')
[ -n "$web_port" ] || fail "could not read the web port out of the rendered Service"

while IFS=$'\t' read -r match sticky port; do
  if [ "$match" = "$catchall" ]; then
    [ "$port" = "$web_port" ] || fail "catch-all targets port $port, not the SPA's $web_port"
    # The SPA and /ws are two load balancers over different server lists sharing
    # one cookie name; a sticky SPA would re-mint it with a value /ws cannot
    # resolve. templates/ingressroute.yaml carries the full reasoning.
    [ "$sticky" = "no" ] || fail "catch-all carries a sticky cookie, which would collide with the /ws balancer's"
    continue
  fi
  [ "${#match}" -gt "${#catchall}" ] ||
    fail "rule \"$match\" is not longer than the catch-all \"$catchall\" — Traefik would not rank it first"
  case "$match" in
    "$catchall"' && PathPrefix(`'*'`)') ;;
    *) fail "rule \"$match\" is not the catch-all plus a PathPrefix suffix" ;;
  esac
  [ "$sticky" = "yes" ] || fail "server rule \"$match\" lost its sticky cookie"
done < "$work_dir/rules.tsv"

# ── Opt-in stays opt-in ─────────────────────────────────────────────────────
if grep -q 'name: phase-server-web$' "$work_dir/noweb.yaml"; then
  fail "web resources render with web.enabled=false"
fi

echo "assert-web-routing: PASS"
