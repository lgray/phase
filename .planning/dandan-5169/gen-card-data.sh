#!/usr/bin/env bash
set -euo pipefail
W=/home/lgray/vibe-coding/phase-rs-workdir/.claude/worktrees/agent-a31b944b51f33e128
source /home/lgray/vibe-coding/cargo-isolate.sh "$W"
echo "CARGO_HOME=${CARGO_HOME:-UNSET-SHARED-ISOLATION-FAILED}"
cd "$W"
./scripts/gen-card-data.sh
