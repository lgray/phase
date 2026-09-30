# Worker environment — run dandan-5169 (read in full before acting)

## Identity
- First line of your final report: `MODEL: <your exact model id>` (from your system prompt).
- Original task: "Implement phase-rs/phase#5169 (the Dandan format) per the issue body's refreshed design brief and its Settled decisions; build for the class, the machinery, not just one decklist; prepare it for a PR to phase-rs/phase targeting main." Budget: standard.
- The draft design brief (issue body) is `.planning/dandan-5169/brief/issue-5169.md`. Its "Settled decisions" section is binding. Its anchors were written at `d95cc66019`; the code has moved, so re-measure every anchor against the current tree.
- Census scripts from the brief: `.planning/dandan-5169/brief/scripts/` (`matches.py`, `count_reads.py`, `count_reads_lines.py`, `names.txt`). Run as `python3 <abs path>/count_reads.py HEAD '<pattern>' -v`.

## Paths
- Work tree W = `/home/user/phase` (cloud clone, branch `feat/dandan-format`). All relative paths in this file are relative to W. Use absolute paths.
- Skills: read them by absolute path under `/home/user/phase/.claude/skills/<name>/SKILL.md` (the Skill tool also loads this same copy).
- Run root (gitignored): `W/.planning/dandan-5169/`. Charter: `phase-charter`. Phase plans: `phase-<k>/plan.md`. Reviews: `phase-<k>/...`.

## Build and test (no Tilt; cloud box: 4 cores, 15 GB RAM, ~29 GB disk)
- No Tilt. Ignore Tilt instructions; run cargo directly from `/home/user/phase`, e.g. `cd /home/user/phase && cargo nextest run -p engine <filter>`.
- ONE heavy build at a time; the box is small. Never run two cargo commands concurrently. Prefer targeted `cargo nextest run -p <pkg> <filter>` / `cargo check -p <pkg>`. New engine tests go in `crates/engine/tests/integration/` with a `mod` line in `tests/integration/main.rs`. Redirect long output to a file under `/home/user/phase/.planning/dandan-5169/` and grep it.
- Never `cargo clean`, never delete `target/`.
- CR text: `docs/MagicCompRules.txt`. Grep every CR number before writing it.
- Card Oracle text: `jq '.data["<Card Name>"][0].text' data/mtgjson/AtomicCards.json` once card data is generated, or the Scryfall API (`curl -s 'https://api.scryfall.com/cards/named?exact=<name>' | jq .oracle_text`). Check `client/public/card-data.json` exists before relying on it.

## Git
- Never `git stash`, `git checkout <file>`, `git restore`, `git reset`, `git commit`, `git push`, or any branch switch. Planners and reviewers are read-only on tracked files. Executors edit only their authorized paths and never commit (the orchestrator commits).
- Read history with `git log`, `git show`, `git grep`, `git diff` only.

## Binding rules (lead's brief, on top of CLAUDE.md / AGENTS.md)
- Idiomatic Rust, CR fidelity (every CR number grepped), build for the class, the engine owns the logic, display-only frontend, nom combinators on the first pass, one authority per concept.
- Measured claims: every claim about the code rests on a command you ran. Planners and reviewers probe dynamically (run the code), not just read it.
- Discriminating tests: every new test fails when the fix is reverted (shown red, then green) and has a positive reach-guard. Tests assert; they do not narrate. Never pin a file:line in a test or comment.
- Real cards only, in real game flows; read each card's Oracle text before deciding what it does.
- Comments: one sentence saying what the code cannot (a reason, an invariant, a CR rule, a trap). No history, no evidence, no restating the code.
- Findings are fixed in this change or dropped with a verdict. No follow-up rows, no TODOs.
- Protocol: new format, mulligan and draw-frame wire surfaces bump the protocol/lobby/wire versions per `scripts/check-protocol-version.mjs`; committed generated bindings are regenerated (CI byte-compares them).
- Workers run on Sonnet; be economical with tokens: read what a decision needs, not whole subsystems.
