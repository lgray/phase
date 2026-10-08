# Phase 21 plan review, round 1 (phase-plan mode, phase-fit context declared)

Reviewer model: claude-opus-5-5. Plan base a10c2316dd. Skill: review-engine-plan at 1ecbc840cf.

**Verdict: no `behavior` or `machinery` findings. Three `text` findings, each with replacement text below.** The plan can be closed by applying them, with no further design round.

## Probes (scratch trees under dandan-run/scratch, deleted after)

- **Base** (`p21rev`, a copy of W at a10c2316dd): an integration probe driving `load_and_hydrate_decks`, `validate_name_deck_for_format_full`, `evaluate_deck_format_gate` and `reconstruct_initial_state` against `integration_cards.json.gz`.
- **Prototype** (`p21rev-proto`): the same tree with three minimal edits:
  - Decision 2 (the pile seat's non-empty `main_deck` is loaded alone, otherwise the default list);
  - Decision 3 (an all-slots-empty submission is `Ok` when `tag().supplies_fixed_deck()`);
  - Decision 4's skip removal.

  Driven rows: the base probe again, a `validate_deck_list_seats` inline probe, an `ai_seat_setups` probe, and full-socket `CreateGameWithSettings` probes.

Measured results:
- **Charter premise (a1) overturned, confirmed at base.** Dandan accepts:
  - a Jund-style 80: `Ok`;
  - 80 x Lightning Bolt: `Ok`;
  - an 80 with 4 Black Lotus: `Ok`.

  Dandan refuses:
  - an Amulet of Quoz in the 80: `Err("Can't be in a deck or sideboard unless the game is played for ante: Amulet of Quoz")`;
  - 79 and 81 cards: "exactly 80 (found N)";
  - a sideboard;
  - a commander slot ("Dandân decks do not use a commander slot");
  - an empty submission: `Err(... found 0)`.

  Gate on an empty submission: Dandan `false`, Momir `false`. Standard empty: `Err`.
- **Loader red at base, confirmed.** A Jund 80 on seat 0 with 80 Islands on seat 1 loads the 23-name default; pools = `[(P0, 23 entries, 80 cards)]`.

  With the prototype: the library is the Jund multiset, pools = `[(P0, 6, 80)]`, and `deck_pool_of(P1).player == P0`. Every object after `start_game` has a Jund name. An empty submission still loads 80 cards with no empty seat.
- **Replay needs no header field, confirmed.** With the prototype, `reconstruct_initial_state` from a header whose `deck_data` is the Jund `DeckList` gives the same library sequence as the live game (66 = 80 - 14).
  - wasm `initialize_game_impl` records the submitted `DeckList` as `ReplayHeader.deck_data` (`recorded_deck_list`), so the pile is in the header.
- **Server legs need no production change, confirmed (driven, prototype).** Full-socket create results:
  - Dandan, empty host deck: `GameCreated`;
  - Dandan, empty host deck plus an AI seat with an empty `DeckChoice::DeckList`: `GameCreated` → `SessionAttached` → `GameStarted`;
  - Momir, empty: `GameCreated`;
  - Standard, empty: still `DeckRejected`.

  `ai_seat_setups`:
  - Dandan and Momir with an empty `DeckList`: `Ok(1)`;
  - Standard: `Err`;
  - the `deck: None` shape: still `Err` (random starter).

  The server join path (`join_game_with_password_full`) only `resolve_deck`s, with no format check, so Phase 22 guests submitting nothing need no server change either.
- **The `deck: None` drop verdict holds.**
  - Both `ws-adapter.ts` setup frames (`nativeAiSetupFrame`, `nativePregameSetupFrame`) send `deck: {type: "DeckList", ...}`.
  - The server-lobby host (`startHosting`) only sends `ai_seats` when some opponent is human. Dandan and Momir have `max_players: 2`, so an AI opponent routes to the local path (`allOpponentsAreAi`).
  - The P2P `aiSeatDeckChoice` fallback resolves client-side and never reaches `ai_seat_setups`.
- **wasm seat loop, prototype.**
  - Dandan, pile 79 on each seat in turn: `"Player deck: … (found 79)"`, `"AI opponent deck: …"`, `"AI player 2 deck: …"`.
  - Dandan: pile 80 plus empty seats → `None`; all seats empty → `None`.
  - Momir: 60 Plains → `Some("Player deck: … may only contain the five snow basic lands …")`; all empty → `None`.
  - Standard: all empty → `Some(...)`.
  - At base the function returns `None` for every supplying-format input: the only call is inside `if !supplies_fixed_deck()`.
- **(b) Every start path, by command.**
  - `git grep -n 'load_and_hydrate_decks' -- crates` finds these production callers: engine-wasm `initialize_game_impl`, server-core `session.rs::start_game`, and `replay::reconstruct_initial_state`.
  - The remaining callers are phase-ai bins and `duel_suite`, which only build Commander and two-player default states (no Dandan; `git grep -i dandan` finds no hit there), plus tests.
  - `match_flow` reloads pools that are already synthesized.
  - `client/src-tauri/src` has no deck code: the `deck` grep is empty, with a nonzero `fn ` count as control.
- **(d) Momir.** Accepting the empty submission is the charter's stated consequence of the single axis rule (Phase 21 seam note). Validating non-empty Momir seats at wasm follows from charter Decision 4. It is a regression for one shipped path; see T2.
- **(e)** `DeckSupply` is a closed three-value enum on one axis, with no shared name root and no `bool`. `supplies_fixed_deck` is re-derived from it and keeps its value for every format.
- **(g) CR citations.**
  - 407.3 (ante) and 103.3 ("The players' decks become their libraries") are verified in `docs/MagicCompRules.txt`.
  - The 103.3 subject matches the loader comment's "becomes the shared library".
  - No CR is cited for the no-ban / no-copy-limit rule, which is correct.
- **Sizing.**
  - T1 = 1 unit: axis + loader + validator rule + boundary export make one mechanic through one `add-engine-variant` pass.
  - T2 = 9 paths, which I recounted against the scope rule; the d.ts groups with `lib.rs`.
  - The conjunction cannot fire, and the Sizing section is consistent with the body.

## Findings

### T1 (`text`): the d.ts diff check cannot pass as written

The precedent export commit be100055ae added two things for the new export: the `export function bestOfThreeCeilingForFormat` declaration with its doc comment, and a `readonly bestOfThreeCeilingForFormat: (a: any) => any;` member in `InitOutput`. It also carried unrelated doc-comment drift. Check it with `git show be100055ae -- client/src/wasm/engine_wasm.d.ts`.

Old text:
- Step 4: "`git diff` shows one added declaration."
- Decision 4: "the diff must be one added declaration."
- E6: "d.ts diff = one declaration"

Replace each with: "the regenerated d.ts adds the `deckSupplyForFormat` declaration with its doc comment and its `InitOutput` member. Any other hunk is regeneration drift, committed as produced, never hand-edited."

### T2 (`text`): the driver notes claim no sender reaches a non-empty Momir/Dandan seat at init — false for GameProvider's fresh `p2p-host` branch

Coordinates: claim 8 "a non-80 or illegal one never reached init (lobby blocks it)", and the driver note "no in-tree sender reaches it: the lobby hint already refused such decks".

`GameProvider.tsx` handles `mode === "p2p-host"`. When `takeActiveP2PHost(gameId)` is null and nothing is saved to resume, it starts a fresh WASM host. That host's deck is `buildPlayerOnlyDeckList(parsedDeck ?? EMPTY_PARSED_DECK, …)`, built from the active deck, which no format check has seen. The branch is reached by a refresh or URL, and `formatConfig` comes from the saved active-game meta or the `format` param.
- **At base:** the wasm skip ignores that deck, so Momir and Dandan start on the fixed or default pile.
- **After Phase 21:**
  - a non-legal active deck is refused at init, e.g. `"Player deck: Momir's Madness decks may only contain the five snow basic lands …"` (prototype probe, 60 Plains);
  - a legal Dandan 80 becomes the pile.

No Phase 21 line changes because of this: charter Decision 4 freezes the wasm change, and Phase 22 owns `GameProvider.tsx`.

Replacement for the driver note and PR-body line: "wasm now validates non-empty Momir and Dandan seats. A host route that submits an unvalidated active deck is refused at init until Phase 22 routes every pile-seat construction through its helper; the routes are the GameProvider fresh `p2p-host` branch, and the lobby host once Phase 22 stops requiring a legal active deck."

Replacement for claim 8's sentence: "… a non-80 or illegal one is blocked by the lobby hint on the MultiplayerPage route, and is refused at wasm init on the GameProvider fresh `p2p-host` route."

Addendum line for `addenda/phase-22`, touching the decision "every empty-seat construction takes the pile from one helper": "the helper must submit an empty pile-seat deck for `EngineFixed` (Momir) as well as the `HostPile` default on every route, including the GameProvider fresh `p2p-host` branch and the MultiplayerPage server and P2P host. Since Phase 21, wasm init validates any non-empty Momir or Dandan seat, so the active deck that 'today's flow' submits is refused when it is not legal for the format."

### T3 (`text`): E4b names the wrong assertion as the one that is red at base

Measured at base with the Jund header: replay == live is `true`, because both load the default. The equality is therefore green at base and under M1. Only the reach guard ("differs from the default header's") is red.

Old E4b text in the revert-failing column: "yields the same shared-library name sequence as `load_and_hydrate_decks` + `start_game` with the same seed (red at base: default list)".

Replacement: "the replayed shared-library name multiset == the Jund pile (red at base and under M1: default list); and the replayed sequence == the live sequence for the same header and seed (preservation; green at base)."

## Constraints for the executor (not findings)

- E4b needs an `Arc<CardDatabase>`. `support::shared_card_db()` returns `&'static`, and `load_fixture` is private, so reload the fixture through `CardDatabase::from_export_reader(GzDecoder…)`, as the probe did. Do not add a support helper for one row.
- E5a puts `"deck_supply"` on the no-client-key list for `types.ts` and `formatRegistry.ts`. Phase 22 must keep that substring out of both files; the wrapper lives in `engineRuntime.ts`. One addendum line for phase-22 is enough, alongside the T2 line.

## Pre-existing

None reported. The `deck: None` AI shape being refused for supplying formats reproduces at base. The plan already dispositions it with a verdict (no sender reaches it), and the probes above confirm that verdict.
