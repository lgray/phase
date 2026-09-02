#!/usr/bin/env bash
set -euo pipefail

# Builds the self-hosted web client into client/dist, and optionally packages it
# as a container image for the phase-server chart's `web.enabled` sidecar.
#
#   ./scripts/build-selfhost-web.sh
#   IMAGE=ghcr.io/you/phase-web:v0.59.0 ./scripts/build-selfhost-web.sh --push
#
# The bundle is deliberately server-agnostic: it carries no lobby address. The
# chart supplies one at runtime through /config.js, so one image serves every
# deployment. See deploy/helm/phase-server/README.md.

cd "$(dirname "$0")/.."

push=()
case "${1:-}" in
  --push) push=(--push) ;;
  "") ;;
  *) echo "usage: $0 [--push]" >&2; exit 2 ;;
esac

# Reuse the upstream data plane by default: these JSONs are large, versioned
# with the card pool rather than the engine, and served with
# `access-control-allow-origin: *`. Point this at your own bucket to self-host
# them too — the manifest below drives every URL from it.
export DATA_BASE_URL="${DATA_BASE_URL:-https://data.phase-rs.dev}"
export AUDIO_BASE_URL="${AUDIO_BASE_URL:-$DATA_BASE_URL/audio}"

# card-data.json is not in data-files.json — the deployed copies are
# content-addressed per release, so it is resolved on its own. A locally
# generated one (./scripts/gen-card-data.sh) is exactly the pool this checkout's
# engine parses, so prefer it and let it ship in the bundle; otherwise fall back
# to the shared copy, which tracks upstream's pool rather than yours.
if [ -f client/public/card-data.json ]; then
  echo "card data: bundling client/public/card-data.json (matches this checkout)"
else
  export CARD_DATA_URL="${CARD_DATA_URL:-$DATA_BASE_URL/card-data.json}"
  echo "card data: $CARD_DATA_URL (run ./scripts/gen-card-data.sh to bundle your own instead)"
fi

# ENGINE_WASM_URL is deliberately left unset so the engine is bundled locally
# rather than pinned to an external object — a self-hosted site should not
# depend on someone else's CDN to start a game.
./scripts/build-wasm.sh release

[ -d client/node_modules ] || (cd client && pnpm install --frozen-lockfile)
(cd client && pnpm build)

# Strip what DATA_BASE_URL now points elsewhere, so the image never double-ships
# those bytes. data-files.json is the single source of truth for the set, the
# same way release.yml's strip step uses it.
shopt -s nullglob
while IFS= read -r f; do
  rm -f "client/dist/$f" "client/dist/$f.br"
done < <(jq -r '.[]' data-files.json)
if [ -n "${CARD_DATA_URL:-}" ]; then
  rm -f client/dist/card-data.json client/dist/card-data.json.br
fi

echo "client/dist: $(du -sh client/dist | cut -f1)"

[ -n "${IMAGE:-}" ] || { echo "set IMAGE=<repo>:<tag> to package it"; exit 0; }

# amd64 is not optional: self-hosters run this on whatever they have, and a
# single-arch manifest is an image most of them cannot pull. The Dockerfile is
# RUN-free precisely so both architectures build here without qemu.
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f deploy/phase-web.Dockerfile \
  -t "$IMAGE" \
  "${push[@]}" \
  client
