# syntax=docker/dockerfile:1
#
# The phase web client, for the phase-server chart's `web.enabled` sidecar.
# Build the payload first with scripts/build-selfhost-web.sh, which leaves a
# ready-to-copy tree in client/dist:
#
#   ./scripts/build-selfhost-web.sh
#   docker buildx build -f deploy/phase-web.Dockerfile \
#     --platform linux/amd64,linux/arm64 -t <repo>/phase-web:<tag> --push client
#
# KEEP THIS RUN-FREE. With only FROM and COPY there is nothing to execute in the
# target rootfs, so buildx assembles both architectures on either host with no
# qemu emulation — the SPA payload is arch-independent bytes. Adding a single RUN
# step silently reintroduces a binfmt/qemu dependency, which breaks arm64 builds
# on an amd64 host (and needs setup-qemu-action in CI).
#
# nginx.conf is deliberately NOT baked in: the chart mounts its own from a
# ConfigMap, so one image works for every deployment. Same for /config.js — the
# copy here is the empty placeholder from client/public, and the chart serves its
# own over it.
FROM nginxinc/nginx-unprivileged:1.27-alpine

COPY dist/ /usr/share/nginx/html/
