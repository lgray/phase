# Role briefs — run dandan-5169 (read your section in full)

Common to all roles:
- Read `/home/user/phase/.planning/dandan-5169/worker-env.md` first (environment + binding rules; cargo MUST be prefixed `CARGO_BUILD_JOBS=2`, one cargo command at a time, no Tilt).
- Original task: "Implement phase-rs/phase#5169 (the Dandan format) per the issue body's refreshed design brief and its Settled decisions; build for the class, the machinery, not just one decklist; prepare it for a PR to phase-rs/phase targeting main." Budget: standard.
- Brief: `/home/user/phase/.planning/dandan-5169/brief/issue-5169.md` (Settled decisions binding).
- Charter (frozen, r3): `/home/user/phase/.planning/dandan-5169/phase-charter`. Your phase index k and its entry are given in your prompt; the phase's deferral allowlist is that entry's "Deferral list".
- Phase directory: `/home/user/phase/.planning/dandan-5169/phases/<k>/` (plan.md, plan-review-rN.md, impl-review-rN.md, executor-rN.md).
- First line of your final reply: `MODEL: <your exact model id>`. Keep the final reply short; files hold the detail.
- Measured claims only: every claim about code rests on a command you ran. CR numbers grepped in `/home/user/phase/docs/MagicCompRules.txt`. Oracle text from `client/public/card-data.json` (jq) or Scryfall, verbatim.
- Never git stash/checkout/restore/reset/commit/push/switch branch. Only the orchestrator commits.

## Planner (Step 1, phase-plan mode)
Read `/home/user/phase/.claude/skills/engine-planner/SKILL.md` and run it in **phase-plan mode** for phase k, plus the skill checklists it names. Inputs: charter, phase entry, deferral allowlist, prior phases' accepted summaries (in `phases/summaries.md` if present), attempt history (in your prompt). Emit the full plan with every mandatory section including Sizing, Verification Matrix (DEFERRED(phase n) vocabulary), scope matrix listing literal paths. Write it to `phases/<k>/plan.md` (overwrite on revision; the orchestrator keeps prior copies). Edit nothing else.

## Plan reviewer (Step 2, phase-plan mode)
Read `/home/user/phase/.claude/skills/review-engine-plan/SKILL.md` and run it in **phase-plan mode**, declaring the phase-fit context (Sizing consistency check is blocking). Review the whole plan at `phases/<k>/plan.md` against the charter entry and the code at HEAD. Tag every blocking finding `behavior`, `text` (quote the old string + supply replacement text) or `machinery`. Write `phases/<k>/plan-review-rN.md`. Read-only otherwise.

## Executor (Steps 3/4)
Read `/home/user/phase/.claude/skills/engine-implementer/executor.md` in full (absolute path) and follow it in **phase mode**. Work in IMPLEMENTATION_WORKTREE=/home/user/phase. Edit only SCOPE_PATHS given in your prompt; a needed out-of-list site is a stop-and-return with evidence. Never commit. Write your report to `phases/<k>/executor-rN.md`.

## Implementation reviewer (Step 6, phase mode)
Read `/home/user/phase/.claude/skills/review-engine-impl/SKILL.md` and run it in **phase mode** against the immutable range `PHASE_BASE_SHA..CANDIDATE_SHA` given in your prompt (use `git diff`/`git show` on those SHAs; the working tree may be at the candidate). Inputs: charter, phase entry, allowlist, SCOPE_PATHS, reviewed plan `phases/<k>/plan.md`, prior findings, verification results in your prompt. Tag each HIGH/MED finding `behavior`/`text`/`machinery`; `text` findings quote the old string and supply the replacement. Write `phases/<k>/impl-review-rN.md`. Read-only.
