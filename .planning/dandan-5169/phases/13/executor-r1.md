# Phase 13 executor report r1 (FreeReveal mulligan)

Mode: implementation/fix (phase mode). BASE_SHA = START_SHA = 9ca36e7c8335bebec987beb468e7e0693c2cd082. IMPLEMENTATION_WORKTREE = /home/lgray/vibe-coding/dandan-run/wt-dandan.
Worktree: clean at start, HEAD == START_SHA; at end HEAD unchanged, nothing staged, 26 unstaged/untracked paths, all in scope.nul (`comm` against scope.nul: zero outside; the fixture `integration_cards.json.gz` untouched, not needed). All evidence below is PREPARATORY, not completion evidence.

## 1. Diff summary
- engine types: `actions.rs` `MulliganChoice::FreeReveal`; `game_state.rs` `MulliganDeclarationKind {Regular, FreeReveal}` + `MulliganDeclaration.kind` (`#[serde(default)]`) + serde unit test.
- engine rules: `mulligan.rs` `free_reveal_offered` (pub(crate), computed from live hand + `free_reveal_mulligan()` axis, never stored), `hand_land_split`, `reveal_hand` (names only, empty `card_ids`), `redraw_after_free_reveal` (Immediate timing), handler arm (Immediate redraws; Simultaneous holds a `FreeReveal` declaration), `close_declare_round` (reveals before any return; per-kind count: FreeReveal keeps count, no bottom). `interaction.rs` surface value "freeReveal". `candidates.rs` emits the candidate when `free_reveal_offered`.
- phase-ai `search.rs`: `deterministic_choice` Declare: keep > engine-issued FreeReveal > Powder > Mulligan; `fallback_action` Declare: issued FreeReveal else Powder else Keep.
- client: `types.ts` (`FreeReveal`, `MulliganDeclarationKind`, `declared[].kind`), `GamePage.tsx` (button + fallback ChoiceModal option + handler id `freeReveal`, shown iff the engine issued the action), 8 locale `game.json` (`freeReveal`, `freeRevealDescription`).
- protocol: full-game 110->111, wire 92->93, lobby 16 unchanged, MIN_SUPPORTED = 110: `lobby-broker/protocol.rs` (const, history entry, two asserts), `server-core/protocol.rs` (test renamed `protocol_version_is_111_for_free_reveal_mulligan`, doc), `ws-adapter.ts`, `network/protocol.ts`, `check-protocol-version.mjs` (+40 / +39 and header), `protocol.test.ts` (title v93 + toBe(93)), `p2p-adapter-multiplayer.test.ts` (title v92->v93, frames 92/93).
- tests: `dandan_free_reveal.rs` (+ mod line), unit tests in `mulligan.rs`, `candidates.rs`, `search.rs`, `game_state.rs`, `GamePage.freeRevealMulligan.test.tsx`.

## 3. PREPARATORY verification (all against the final tree, fmt applied to exact RUST_PATHS first)
- `cargo clippy --workspace --all-targets -- D warnings`: rc 0 (p13-clippy.log).
- `cargo nextest run -p phase-engine -p lobby-broker -p server-core -p phase-ai`: 36503 run, 36503 passed, 33 skipped (p13-full.log). Default fixture mode.
- `node scripts/check-protocol-version.mjs`: rc 1 after bumping only lobby-broker `PROTOCOL_VERSION` ("Rust=111, client=110"), rc 0 after all pins.
- `scripts/check-interaction-bindings.sh --check`: rc 0, diff-free.
- client: `tsc -b --noEmit --force` rc 0; vitest GamePage.freeRevealMulligan (4) + `src/i18n` (localeParity, namespaceRegistration, resources) + protocol.test.ts: 292 passed; p2p wire-gate block: 6 passed; eslint on touched TS: 0 errors (2 pre-existing warnings).
- `data/engine-inventory.json` absent (not generated); no FreeReveal/kind pre-existed per `git grep`.

## 4. Parser preparatory gate
No file under `crates/engine/src/parser/` touched: n/a.

## 5. Discriminating-test coverage map
Mutations were run in place from backups and restored (`grep P13_MUT` = 0 in all crates; files touched). Env-gated mutants in one build, each run against a no-env control (17/17 green).

| claim / seam | entry | test | assertion that flips on revert | sibling / paired guard |
|---|---|---|---|---|
| predicate: lands < 2 OR nonlands < 2 (CR 205.2a) | `apply(FreeReveal)` + `candidate_actions` | `dandan_free_reveal::v1_the_hand_predicate...`, `v1_lands_are_counted_by_card_type...` | M1 (`<=`) red; M2 (`&&`) red (13 tests); M3 (Island subtype) red | (2,5),(5,2),(3,4) refused with state unchanged; twin one card away accepted; Mystic Sanctuary counts as a land |
| count gate (no regular mulligan yet) | same | `v2_a_regular_mulligan_ends_the_free_reveal` | M4 (clause removed) red (v2 + candidates test) | count-0 twin offered |
| format axis | handler/candidates | `v5_the_format_axis_gates...`, unit `free_reveal_is_refused_where_the_format_offers_none` | M8 (Unavailable arm = predicate) red | same state, only `format_config` differs |
| held declaration, close, count, shuffle, reveal | `apply` on the 80-card pile | `v3_a_free_reveal_is_held...` (declarer P1/non-canonical, P0/canonical holder, P1 starting), `v4_free_reveal_and_regular_mulligans_close_together...` | M5 (kind as Regular) red; M6 (no shuffle for FreeReveal) red; M7 (reveal omitted) red | v4 mixes kinds, arrival opposite to deal order, both starting seats; P0 owes a bottom, P1 none; `v3_a_regular_close_discloses_nothing_either` |
| repeat while qualifying | `apply` | `v6_...may_repeat` | M5/M7 | - |
| no id disclosure (N3) | `public_revealed_cards`, `viewer_knows_card_identity` over every object after the closing `apply` | `assert_nothing_disclosed` in `v3_*` | MLEAK (reveal_hand emits the hand's ids): `public_revealed_cards.is_empty()` fires, with the equality on the event relaxed so this assertion is the one reached (live-instrument control) | Regular close also disclosed-nothing |
| `Immediate` timing helper | unit | `free_reveal_redraw_reveals_then_reshuffles_only_the_declarers_hand` | not mutated (helper has no count/ledger to mutate) | other seat untouched, no `ShuffledLibrary` event |
| serde | unit | `mulligan_declaration_kind_round_trips_and_defaults_to_regular` | M9 (`serde(default)` removed) red | Regular round-trips |
| candidate emission | `candidates.rs` | `free_reveal_is_issued_only_to_the_seat...`, `..._not_issued_after_a_regular_mulligan_or_while_bottoming` | M14 (unconditional) red | other seat keeps Keep+Mulligan; BottomCards phase still has candidates |
| AI selection | `deterministic_choice`, `fallback_action`, real `apply` | `mulligan_selects_the_engine_issued...`, `mulligan_keeps_a_hand...`, `mulligan_prefers_a_free_reveal_to_a_serum_powder`, `fallback_takes...`, `ai_pair_leaves_the_dandan_mulligan...` | M11 (never selects) red x3; M13/M13F (Powder first) red; M11F (fallback never selects) red | registry `!keep` reach guard; count-1 twin; candidate withheld => Mulligan (AI never re-derives); registry keep => Keep; 30 seeded AI-vs-AI Dandan boots all leave the mulligan with `free_reveals > 0` |
| client button | `GamePage` | `GamePage.freeRevealMulligan.test.tsx` | button gated `false &&`: 2 positive tests red | no action => no button (Keep present); declared viewer => no prompt; fallback ChoiceModal reached (viewer seat absent) |
Not run: M12 (AI derives predicate; the withheld-candidate leg is its detector) and M10 (n/a). All production seams have a production-path test; none shape-only.

## 6. Maintainer-simulation matrix
| seam | entry / first branch | authority | bound value, when | mode | storage | consumers | invalidation | hostile fixtures | serde |
|---|---|---|---|---|---|---|---|---|---|
| FreeReveal legality | `handle_mulligan_decision` FreeReveal arm; `free_reveal_offered` axis match | `entry.player`'s own hand, count, phase | read at the declaring call | live predicate (never cached) | none | handler, `candidates.rs`, AI via issued set, client via `legalActions` | hand changes between calls are re-read; BottomCards phase refuses | V1/V2/V5 | no stored value, so no visibility leak in `WaitingFor` |
| held declaration | Simultaneous arm: `pending.remove`, push `MulliganDeclaration{kind}` | declaring seat (authorised via `pending`) | declaration | snapshotted kind + count | `WaitingFor::MulliganDecision.declared` | `close_declare_round` | elimination drops it (`prune_mulligan_pending` keeps whole struct, unchanged); hand cannot change while held (seat not in `pending`) | V3 (non-active, non-canonical declarer; canonical declarer), V4 | `kind` default Regular; protocol 111/93 |
| reveal | `close_declare_round` first loop over `redrawers` (seat-walk from active) | declarer's hand at the close | event at close, before any return | by name only, `card_ids` empty | `GameEvent::CardsRevealed` | log (names), `remember_public_reveals` (iterates no ids) | n/a | disclosure probe (MLEAK control) | event shape unchanged |
| per-kind count | `close_declare_round` entry rebuild (single exhaustive match) | `MulliganDeclaration.kind` | close | latched kind | rebuilt `MulliganDecisionEntry` | cap check, Keep owed-bottom math | count stays 0 => never reaches cap | V4 | - |
No row incomplete; no row DEFERRED.

## 7. CR-annotation diff gate
`UNVERIFIED:` count 0. Numbers added: 103.5, 205.2a, 701.20a, 701.20b (grepped in `docs/MagicCompRules.txt`; 205.2a lists land as a card type, 701.20a/b reveal does not move the card, 103.5 mulligan procedure). The Dandan free reveal itself is a format modification, annotated as "CR 103.5 as modified by the Dandan free-reveal rule".

New-field threading (`MulliganDeclaration.kind`): constructors `mulligan.rs` Regular (Mulligan arm) and FreeReveal (new arm) thread it; test literals set it; `elimination.rs::prune_mulligan_pending` passes the struct through (preserves it); client mirror `declared[].kind`. `MulliganChoice::FreeReveal`: exhaustive matches are `mulligan.rs` and `interaction.rs` only (compiler-confirmed; `manabrew-compat`, `server-core/session.rs`, `phase-ai/policies/mulligan/mod.rs`, `planner/mod.rs` not edited, no compiler error).

## 8. Judgement calls
- Plan step-0 premises replaced: (a) close ordering uses `seat_walk_from_active` (landed Phase 12), not `apnap_order_from`; (b) `MulliganDeclaration` is not `Copy`/`Default` at base, kept as is; (c) protocol numerals re-derived (111/93, MIN 110); (d) plan's V3 "control leg via RevealHand effect" replaced by the MLEAK mutation (`remember_card_identities` is pub(crate)); (e) V5 control: same booted Dandan state with `format_config` swapped to Standard rather than a separate Standard state of untyped cards (real cards only).
- `reveal_hand` takes `&GameState` (read-only).
- V8e liveness probe became `ai_pair_leaves_the_dandan_mulligan_taking_free_reveals` (30 seeds, 200-step bound).
- Fallback Declare: FreeReveal-first changes the old Powder-first order, as the plan states.

## 9. Stop-and-return items
None.

## 11. Deviations
None beyond section 8 (all inside scope).

## 12. Risks
- Computed (not stored) availability follows the plan's reasoned charter divergence (3.2); reviewer should confirm the divergence is accepted.
- `CardsRevealed` with empty `card_ids`: engine side verified (log, remember hook); the client reveal-fan path with 7 names and 0 ids was not exercised (no vitest for `revealFanCards` run).
- `fallback_action` now FreeReveal-first with no registry check: repeats while the hand qualifies (13.8% per hand is the plan's computed figure, not re-measured; the 30-seed `ai_pair` loop covers `deterministic_choice` only).
- Heavy shared-box load made the 36k-test run take 28 min; no flakiness observed.
