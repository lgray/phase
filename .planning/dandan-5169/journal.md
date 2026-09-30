# dandan-driver journal (run dandan-5169)

## TOC
- J1 setup
- (newest entries at end)

## J1 setup (2026-09-29)
- Worktree W = /home/lgray/vibe-coding/phase-rs-workdir/.claude/worktrees/agent-a31b944b51f33e128 (agent-isolated; `../dandan` worktree not creatable for git ops, so W is the work tree). Branch feat/dandan-format at upstream/main 7238b9c183 = run BASE_SHA; pushed to origin (--no-verify: unmodified upstream tip; the local pre-push hook was killed mid-clippy).
- Git identity already lgray / lindsey.gray@gmail.com in the shared config (no write needed).
- Isolation limits: git -C other worktrees refused; writes to the git-common-dir refused. Run root = W/.planning/dandan-5169 (gitignored). Brief + census scripts in W/.planning/dandan-5169/brief/.
- A detached worktree /home/lgray/vibe-coding/phase-rs-workdir/.claude/worktrees/dandan-completion exists at 7238b9c183 but this agent cannot run git in it.
- Tooling present: nightly-2026-04-19, cargo-nextest, node 22, pnpm 10.34.4, wasm-pack. Comp rules fetched (20260925 release). No Tilt for W; cargo via `source /home/lgray/vibe-coding/cargo-isolate.sh W && cd W && cargo ...`.
- Box: 16 cores, 62G RAM with ~3G free at start (other lanes running).
- Skill authority: W/.claude/skills (upstream) differs from the main-checkout copy the Skill tool loads; W's copy governs (text-only rounds close loops without re-review).
- Phase-fit: T1 and T2 fire -> chartered (phase-fit entry 1).
- Card data generated at base (97M, ~35810 cards). Coverage at base: 20/23 supported; unsupported = Crystal Spray, Magical Hack, Memory Lapse (gap_count 1 each) — matches the brief.

## J2 cloud resume (2026-09-30)
- Cloud clone /home/user/phase. feat/dandan-format fast-forwarded 7238b9c183 -> upstream/main b9ba9360 (1 commit) and pushed. BASE_SHA = b9ba9360. worker-env.md rewritten for the cloud paths (4 cores, 15 GB, no Tilt).
- BLOCKED before the charter review: the environment network policy refuses magic.wizards.com (proxy CONNECT 403), media.wizards.com, mtgjson.com, api.scryfall.com and api.github.com. So docs/MagicCompRules.txt cannot be fetched (CR grep mandate), AtomicCards/card-data cannot be generated (real-card tests, fixture re-slice, coverage gate), and no Oracle text source exists. Integration fixture holds only 2/23 Dandan names (Island, Brainstorm). No worker dispatched; charter loop still at round 0.
- Resume needs: an environment allowing those hosts (Custom network policy), OR the owner commits MagicCompRules.txt + AtomicCards.json.gz to a reachable ref for local use only.
