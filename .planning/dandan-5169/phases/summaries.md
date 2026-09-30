# Accepted phase summaries (for later planners; no debates)

## Phase 1 — card-bot /lfg format autocomplete
- `scripts/card-bot/lfgInteractions.ts` exports `LFG_FORMAT_OPTION` (autocomplete, no static choices) and `suggestFormats(query, registry = FORMATS)` (prefix before contains, lowercased label+key, capped at 25). `register.ts` uses `LFG_FORMAT_OPTION`. `formats.test.ts` asserts no static choice list; mirror-equality test unchanged (Phase 2 adds the Dandan entry to `scripts/card-bot/formats.ts`).

## Phase 2 — S1 format registration
- `GameFormat::Dandan` (after FreeformCommander, before Custom); `FormatConfig::dandan()` (20 life, 2 players, exactly 80, supplies_fixed_deck, no command zone); registry entry after Momir.
- Six exhaustive axis methods on `GameFormat` in `crates/engine/src/types/format.rs`: shared zones, draw deal order, free-reveal mulligan, hand-entry ownership, opening-hand equivalence, best-of-three ceiling. Census test `format_axis_census.rs` covers them.
- Deck validation: Dandan joins the Freeform constructed arm group (no card pool; axes read). `custom_format.rs` refuses Dandan as a Custom source. manabrew-compat refuses shared-zone snapshots (`upstream.shared-zone-ownership-missing`).
- Protocol: full-game 93, lobby 15 (`MIN_LOBBY_PROTOCOL_FOR_DANDAN = 15` frozen in ws-adapter.ts), wire 75. card-bot `formats.ts` has the Dandan entry.
- Known interim: `supplies_fixed_deck` true but deck loading keys on Momir until Phase 6.

## Phase 3 — AI force-keep re-gate
- `FixedDeckKeepMulligan` (phase-ai `policies/mulligan/fixed_deck_keepables.rs`) force-keeps only when `format.opening_hand_equivalence()` is Equivalent (Momir); Dandan abstains. Trace fact `opening_hand_equivalent`. ai-gate owed at run level after Phase 17.

## Phase 4 — PREREQ-0 swallow check
- `parser/swallow_check.rs::effect_is_replacement_carrier` accepts `Effect::Counter { countered_spell_zone: Some(_), .. }`. Memory Lapse, Lapse of Certainty, Remand, Spell Crumple now supported; Hinder stays flagged.
