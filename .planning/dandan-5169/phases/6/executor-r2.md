# Phase 6 executor round 2
Mode: implementation/fix. BASE_SHA: see run; START_SHA 8eecaec29b; IMPLEMENTATION_WORKTREE /home/lgray/vibe-coding/dandan-run/wt-dandan (start clean, HEAD==START_SHA, nothing staged; end: HEAD unchanged, nothing staged, delta = the two paths below). PREPARATORY only.

Closed [HIGH] v1 default-mode failure.
- dandan_shared_pile_storage.rs (v1_non_shared_formats_keep_per_seat_libraries): five Snow-Covered basics named as literals; asserts the set of momir_fixed_deck_names() equals them (so the generator selects them and a drifted Momir list fails).
- integration_cards.json.gz regenerated (python3 scripts/gen-test-fixture.py): HEAD fixture had only snow-covered island, now all five. `--check`: ok, 4939 cards.
- Sweep: file's other card names are all literals (expected_decklist, MIXED_PILE, add_real_card calls); --check clean is the instrument; whole file in default mode (FORGE_TEST_FULL_DB unset): 20 tests run, 20 passed (phases/6/r2-default.log).
- Discrimination: fixture lacking the four basics gave 12 of 60 (reviewer's measurement); the new literals are what put them in the fixture.

Trap hit: first default-mode run showed 3 failures (v3, v5/v6, v9), also failing under FORGE_TEST_FULL_DB=1: stale mutant-build artifacts from r1 mutation runs (sources identical to HEAD). After `touch` of the 10 mutated src files and a rebuild, 20/20 pass. Orchestrator's own build/verification should rebuild from touched sources; no source content changed.

CR annotations: none added. Stop-and-return: none.
