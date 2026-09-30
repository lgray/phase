# Accepted phase summaries (for later planners; no debates)

## Phase 1 — card-bot /lfg format autocomplete
- `scripts/card-bot/lfgInteractions.ts` exports `LFG_FORMAT_OPTION` (autocomplete, no static choices) and `suggestFormats(query, registry = FORMATS)` (prefix before contains, lowercased label+key, capped at 25). `register.ts` uses `LFG_FORMAT_OPTION`. `formats.test.ts` asserts no static choice list; mirror-equality test unchanged (Phase 2 adds the Dandan entry to `scripts/card-bot/formats.ts`).
