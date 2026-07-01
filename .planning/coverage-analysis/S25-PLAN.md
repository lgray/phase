# S25-effect-verb-bespoke — implementation plan (40 Standard-legal cards)

**Snapshot:** 2026-07-01 @ `acd2f5e6b` · card-data + coverage-data regenerated (inputs stamp `2026-07-01T20:05Z`).
**Backing artifact:** `.planning/coverage-analysis/out/standard/cluster-assignment.tsv` (rows `S25-effect-verb-bespoke`, 40 cards, 0 unclustered).
**Skill:** produced via `/engine-planner`. Each family below names its execution skill (`/add-engine-effect`, `/oracle-parser`, `/add-replacement-effect`, `/add-trigger`, `/review-engine-plan`).

> **Hard rule (inherited from `std-rollout/PLAN.md`): all 40 cards MUST be implemented — no deferrals, no "out-of-scope," no dropping. Groups A/B/C order cards by *implementation cost*, not by whether they ship. "Group C" = harder cards that are *gated* by `/review-engine-plan` (reviewed before code) and then built — every one ships. `/review-engine-plan` gating is the mechanism for shipping hard cards correctly; it is not a synonym for deferral.**

---

## The dominant insight (drives the whole dispatch order)

S25 is the coverage tool's **"parser lowered everything except one effect verb"** bucket (`gap_count > 0`, handler `Effect:<verb>`). The critical measured fact:

> **Roughly half of S25 — 19 of 40 (all of Group A) — is a PARSER-LOWERING gap for an `Effect` variant that ALREADY EXISTS**, with more reuse inside Group B. The engine can already *resolve* Cloak, Animate, Dig, RepeatContinuation, LoseLife, SearchLibrary, RemoveKeyword, SetColor, put/remove-counters, and the S01 excess-damage trigger — the parser just can't lower the specific verb phrasing into them.

Confirmed existing variants (`crates/engine/src/types/ability.rs`):
`Effect::Cloak` (10668, CR 701.58) · `Effect::Animate` (9291) · `Effect::Dig` (8675) · `Effect::SearchLibrary` (9435) · `Effect::LoseLife` (8426) · `Effect::Investigate` (8807) · `Effect::BecomeCopy` (9164) · `Effect::GainActivatedAbilitiesOfTarget` (9194) · `RepeatContinuation` (effects/mod.rs) · `ContinuousModification::{RemoveKeyword (17260), SetColor (17411), AddColor (17414)}` · `AbilityCost::Behold` + `BeholdCostAction` (6483, CR 701.4).

**Consequence:** the plan splits into three tiers by cost, not by card. **Group A (parser-only, reuse existing effect)** is the bulk and the cheapest — batch it first. **Group B (small new composition/effect)** is medium. **Group C (new subsystem / genuinely bespoke)** is gated by `/review-engine-plan`, and one card (Secret of Bloodbending) needs the Controlling-Another-Player subsystem (CR 723) that does not exist at all.

---

## Analogous trace (hard gate)

**Traced feature: Cloak** (the exemplar of "effect exists, parser gap" — Group A's whole shape).
Full path, parser → resolver:
- Parser dispatch: `crates/engine/src/parser/oracle_effect/imperative.rs:8190` (`"cloak" | "cloaks"` arm) → builds `ImperativeFamilyAst::Cloak { target, count }`, lowered at `imperative.rs:10007` → `Effect::Cloak { target, count }`. Keyword-action registration: `parser/oracle_util.rs:1466` (`KEYWORD_ACTIONS`).
- Type: `types/ability.rs:10668` `Effect::Cloak` (CR 701.58a annotation present).
- Resolver: `game/effects/cloak.rs`, dispatched from `game/effects/mod.rs`; face-down/printed-characteristics interplay in `game/printed_cards.rs`; coverage registration `game/coverage.rs`.
- Tests: `parser/oracle_effect/tests.rs:8499`.

Every Group A family follows this exact shape: extend the parser dispatch/lowering only; the `Effect` variant, resolver, and coverage registration already exist. The Cloak parser today handles `"cloak the top N"` (imperative.rs:8195) but **not** the article/pronoun phrasings Vannifar (`"Cloak a card from your hand"`) and Expose the Culprit (`"cloak them"`) use — that gap generalizes to the whole group.

---

## GROUP A — Parser-only, reuse an existing `Effect` variant (batch first; cheapest ROI)

Skill: **`/oracle-parser`** (authoritative parser reference) for every family here. No new `Effect` variant, no resolver change. Each is a nom-combinator extension in the relevant `oracle_effect/` or `oracle_static/` dispatcher. **Nom compliance:** extend the existing `alt()`/`tag()` dispatch in the named parser file — never `contains()`. The parser IS the detector.

| A# | family / building block (existing) | cards | swallowed clause → target lowering |
|----|-----------------------------------|-------|-----------------------------------|
| **A1** | **Cloak** → `Effect::Cloak` (parser: `oracle_effect/imperative.rs` cloak arm) | Vannifar, Evolved Enigma · Expose the Culprit | `"Cloak a card from your hand"` / `"shuffle that pile, then cloak them"` — extend the cloak arm to accept `a/one/them` + zone-qualified source; Expose composes `Effect::Shuffle` + `Effect::Cloak` (sequence). |
| **A2** | **Animate / "it's a P/T … creature" / "is a <type> in addition"** → `Effect::Animate` (+ `ContinuousModification::AddType`) | Brilliance Unleashed · The Tomb of Aclazotz · Vraska, the Silencer | `"it's a 3/3 Robot artifact creature with flying"` / `"is a Vampire in addition to its other types"` / `"It's a Treasure artifact with '…', lose all other card types"`. Brilliance = plain Animate; Tomb adds `enters with a finality counter` (compose enters-with-counter) + add-type; **Vraska is the heavy tail** (Animate + granted activated ability + *lose all other card types*) → move to Group C if the granted-ability lowering isn't already a building block. |
| **A3** | **Color-set** → `ContinuousModification::SetColor` (17411) | Foraging Wickermaw | `"become that color"` — lower to `SetColor` bound to the event-context color. |
| **A4** | **Dig + LoseLife** → `Effect::Dig` (8675) + `Effect::LoseLife` (8426) | Stargaze | `"Look at twice X … put X into hand, rest into graveyard. You lose X life."` — compose Dig(keep_count=X, look=2X) + LoseLife(X). Both variants exist; parser sequence only. |
| **A5** | **LoseLife dynamic-qty** → `Effect::LoseLife` + `QuantityExpr` (revealed-card MV) | Parker Luck | `"lose life equal to the mana value of the card revealed"` — needs a `QuantityRef` for "MV of the revealed card"; check `game/quantity.rs` for an existing revealed-object MV ref before adding one. |
| **A6** | **RemoveKeyword continuous** → `ContinuousModification::RemoveKeyword` (17260) | Quick Draw | `"Creatures target opponent controls lose first strike and double strike"` — lower "lose <keyword> and <keyword>" to two RemoveKeyword mods (compose, don't enumerate). |
| **A7** | **Mana-spend permission/restriction** → `mana_spend_permission` field (exists; `database/synthesis.rs`) | Outrageous Robbery · Tin Street Gossip · Overgrown Zealot | `"spend mana as though it were mana of any type"` / `"spend this mana only to cast face-down spells or turn creatures face up"` / `"…only to turn permanents face up"`. One `ManaSpendPermission` typed axis (spend-as-any-type vs spend-restricted-to-purpose). Verify the field's enum already spans both directions; extend the parser that fills it. |
| **A8** | **Counters (put/remove, dynamic X)** → existing counter effects + `QuantityExpr::ObjectCount` | Crowd-Control Warden · Rhys, the Evermore · Esper Terra · Prishe's Wanderings | Warden `"put X +1/+1 counters, X = other creatures you control"` (enters/turned-face-up + dynamic X — S08-style ObjectCount); Rhys `"Remove any number of counters"` (variable-count removal); Esper Terra `"put up to three lore counters"` + `"Add {W}{W},{U}{U},…"` (counters + mana add); Prishe reflexive `"when you search your library this way, put a +1/+1 counter"` (delayed/reflexive trigger + counter → may belong to `/add-trigger`). |
| **A9** | **Behold as an effect** → reuse `BeholdCostAction` infra (6483) at effect layer | Sarkhan, Dragon Ascendant | `"behold a Dragon"` (ETB effect, not a cost) — check whether an `Effect`-side behold exists; if only the *cost* form exists, this is a small Group-B effect wrapping the same reveal/from-battlefield choice. CR 701.4. |
| **A10** | **RepeatContinuation** → `RepeatContinuation` (exists; `game/effects/mod.rs`) | Another Round | `"repeat this process X more times"` — lower to the existing repeat-continuation wrapper with count `X`. Pure parser; RepeatContinuation shipped #4030. |
| **A11** | **Excess-damage reflexive trigger** → S01 `DamageChannel::Excess` block + targeted `Effect::Destroy` | Rhino's Rampage | `"When excess damage is dealt to the creature an opponent controls this way, destroy up to one target noncreature artifact with mana value 3 or less"` — reuse the excess-damage reflexive-trigger building block shipped in S01 (Torch the Witness / Orbital Plunge) + a targeted Destroy. Parser + reflexive-trigger wiring only. CR 120.10 (triggers checking excess damage). |

**Group A subtotal: 19 cards** (if Vraska (A2) shifts to Group C, A = 18 / C = 14). Batch-1 dispatch: one `/engine-implementer` run per 1–2 families, sequential (shared file `oracle_effect/imperative.rs` / `sequence.rs` = collision point → do not parallelize A1/A2/A8/A10).

---

## GROUP B — Small new composition or effect (medium; `/add-engine-effect`)

| B# | family / new building block | cards | notes / CR |
|----|-----------------------------|-------|-----------|
| **B1** | **Multi-zone search-by-name + exile** (new: `SearchLibrary` exists but not gy+hand+library at once) | Deadly Cover-Up · The End | `"search its owner's graveyard, hand, and library for any number of cards with that name and exile them"`. One reusable "search these zones for name-matches, exile" effect (the *Bell, Book and Candle* / *Conspiracy* pattern). CR 701.23 (Search) + CR 201.2a/201.3a (same-name matching) + CR 701.13 (Exile). 2 cards, 1 block. |
| **B2** | **Delayed sacrifice at a future end step** (`DelayedTrigger` infra exists — `game/effects/`, `database/`) | Kav Landseeker · Choreographed Sparks | `"At the beginning of the end step on your next turn, sacrifice that token"` / copy gains `"At the beginning of the end step, sacrifice ~"`. Compose the existing delayed-trigger builder; Choreographed also grants haste + the delayed sac to a **copy** (compose with the copy effect). CR 603.7 (delayed triggered abilities). |
| **B3** | **Gain ALL abilities of another object** (broaden `GainActivatedAbilitiesOfTarget` → all abilities) | Symbiote Spider-Man | `"gain this card's other abilities"` — either parameterize the existing variant (activated → all) or a sibling; run `/add-engine-variant` gate. |
| **B4** | **CDA token w/ dynamic base P/T** (token-create + characteristic-defining P/T) | The Skullspore Nexus | `"create a … token with base power and toughness each equal to the total power of those creatures"` — reuse token-create + a `QuantityExpr` for "total power of a set"; check `game/quantity.rs::TargetPower`-style aggregate. |
| **B5** | **"first <spell type> you've cast this turn" trigger** (`/add-trigger`) | Alania, Divergent Storm | spell-type-count-per-turn tracking → conditional trigger. Sibling of S19 new-trigger matchers; verify the cast-count watcher exists. |
| **B6** | **Control-loss trigger + unattach** (`/add-trigger`) | Stolen Uniform | `"When you lose control of that Equipment this turn, … unattach it"` — lose-control trigger matcher + unattach effect. CR 603.6/702.6 (equip). |

**Group B subtotal: 8 cards.** Each is one small engine effect or trigger matcher; `/add-engine-effect` or `/add-trigger`, then compose in the parser. B3/B4 hit `/add-engine-variant`.

---

## GROUP C — New subsystem / genuinely bespoke (`/review-engine-plan` BEFORE code)

| C# | card | why heavy / new subsystem |
|----|------|---------------------------|
| **C1 — HEAVY** | Secret of Bloodbending · Bumi, Unleashed | **Controlling Another Player (CR 723) — does not exist at all.** "You control target opponent during their next combat phase / next turn." Bumi pairs a combat restriction ("only land creatures can attack that combat") onto a controlled combat. This is a whole turn/priority-ownership subsystem → **built behind a dedicated `/review-engine-plan` gate (reviewed before code, then shipped).** The single hardest S25 card; it is implemented, not dropped. |
| **C2** | Moonlit Meditation | Token-creation **replacement** ("first time you would create tokens each turn, instead create copies of enchanted permanent") → `/add-replacement-effect` + the once-per-turn latch. |
| **C3** | Niko, Light of Hope | "Shards you control become copies of it until the next end step" — `BecomeCopy` exists but the mass/duration-bound form over a dynamic set needs verification. |
| **C4** | Memory Vessel | Play-permission from an exile set + "can't play from hand" — a play-permission/-restriction pair (CR 118, 601). |
| **C5** | No Witnesses | "Each player who controls the most creatures investigates. Then destroy all creatures." — most-of aggregate over players (S16-adjacent per-player enumeration) + wrath. |
| **C6** | Graceful Takedown | Multi-target fan-out: "any number of target enchanted creatures … and up to one other target … each deal damage equal to their power to target creature you don't control." Heavy targeting state-machine (flagged Tier-3 heavy in std-rollout PLAN). |
| **C7** | Glen Elendra's Answer | "all abilities your opponents control" (counter/negate all abilities) — needs the exact oracle context; likely a mass ability-removal. |
| **C8** | Vincent's Limit Break | Three creature-token faces `{0}/{1}/{3}` (Galian Beast / Death Gigas / Hellmasker) — modal/level token-face subsystem, bespoke. |
| **C9** | The Dominion Bracelet | "This ability costs {X} less to activate, where X is ~'s power" — dynamic cost reduction keyed to source power (cost-reduction-by-quantity). |
| **C10** | Vanille, Cheerful l'Cie | "Fearless l'Cie, you may pay {3}{B}{G}" — alt-cost keyword (S23/S26-adjacent); route through the alt-cost parser once that lands. |
| **C11** | Sandman, Shifting Scoundrel | "target land card from your graveyard" — target-in-graveyard extension (S17-adjacent `parse_target` zone extension). Small, but a targeting-layer change. |
| **C12** | Bre of Clan Stoutarm | "Otherwise" else-branch (modal / conditional-else parse) — S12-adjacent; small but structural. |

**Group C subtotal: 13 cards** (14 if Vraska moves here from A2). C1 is the only true new-subsystem heavyweight; several (C11, C12, C9) are small once their adjacent cluster (S17/S12/S23) building blocks exist — sequence them *after* those clusters so they inherit the block.

---

## Mandatory architectural sections

**Pattern Coverage.** The cluster covers 40 Standard cards; the *families* cover classes far beyond these 40 — e.g. A1 Cloak generalizes to every article/pronoun cloak phrasing (Bloomburrow + future), A6 RemoveKeyword to every "loses <kw> and <kw>", A7 to every mana-spend permission, B1 to the whole graveyard+hand+library name-exile class, B2 to every delayed end-step sacrifice. No family is a single-card special case; C8/C1 are the only ones near 1–2 cards and are explicitly gated, not shortcut.

**Building Blocks.** Reused (measured to exist): `Effect::{Cloak, Animate, Dig, SearchLibrary, LoseLife, Investigate, BecomeCopy, GainActivatedAbilitiesOfTarget}`, `RepeatContinuation`, `ContinuousModification::{RemoveKeyword, SetColor, AddColor}`, `AbilityCost::Behold`/`BeholdCostAction`, `QuantityExpr::ObjectCount`/`TargetPower` (`game/quantity.rs`), `DelayedTrigger` builder, `parse_for_each_clause`, the `oracle_effect/imperative.rs` + `sequence.rs` dispatchers, `parse_target`. New (justified): B1 multi-zone name-search-exile; C1 Controlling-Another-Player (CR 723); C2 token-create replacement latch. Every new piece serves a *class*, and each new variant runs the `/add-engine-variant` gate against `cargo engine-inventory`.

**Logic Placement.** Group A = parser only (`crates/engine/src/parser/…`); the resolver/effect/coverage layers are untouched because the `Effect` variant already exists. Group B = new `Effect`/trigger in `types/ability.rs` + `game/effects/…` + `game/effects/mod.rs` dispatch + coverage registration, then parser lowering. Group C = same as B plus a subsystem (turn/priority for C1, replacement layer for C2). Frontend: none needed except where a new `WaitingFor` choice is introduced (none in Group A; verify per Group-B/C card).

**Rust Idioms.** Typed enums throughout — A7 uses a `ManaSpendPermission` enum (spend-as-any vs restricted-purpose), never a bool; A5/B4 use `QuantityExpr`/`QuantityRef`, not inline ints; A6 composes two `RemoveKeyword` mods rather than a bespoke "lose-first-and-double-strike" variant; exhaustive `match` on effect kinds (no wildcard) so the compiler flags every new variant's unhandled arm.

**Nom Compliance.** Every Group A/B detection extends an existing `alt()`/`tag()` dispatch in the named parser module (Cloak arm at `imperative.rs:8190`, keyword-action table at `oracle_util.rs:1466`, sequence composition at `oracle_effect/sequence.rs`). No `contains()`/`starts_with()`/`find()` for dispatch. Detection = `parse_*(text).is_some()`, not substring scan. Compose per-axis (`alt` of source-article × `alt` of zone × count) rather than enumerating full-string tags.

**Extension vs Creation.** Group A + B2/B3/B5/B6 = *extension* of existing patterns (new parser arms, parameterize existing variants, reuse delayed-trigger/trigger-matcher infra). Genuine *creation*: B1 (multi-zone search-exile effect), C1 (CR 723 subsystem), C2 (token-create replacement). Each creation is justified by a class ≥2 cards or an entire missing rules area.

**Verification (approach; per-card matrix built at `/engine-implementer` dispatch).** For each family: (1) a parser round-trip test asserting the new phrasing lowers to the exact existing `Effect` variant (e.g. Vannifar → `Effect::Cloak{count:1,…}`); (2) a runtime test that the card now resolves (cast → measured board delta), not just parses; (3) coverage flips the card `supported=false → true` in `out/standard/` on re-scan; (4) a hostile/negative sibling per seam (e.g. A6: a card that *gains* a keyword must NOT hit the lose-arm; A7: spend-as-any vs spend-restricted must not collapse); (5) revert-failing assertion (remove the new parser arm → the round-trip test fails). No card is counted cleared until its semantics are fully implemented. If a card's Oracle text is parsed before its subsystem exists *mid-development*, it MUST stay red via `Effect::unimplemented(name, fragment)` (never a silent free-resolve) so coverage stays honest — this is a transient in-progress guard, **not** a shipping state; the card ships only when it resolves correctly (see [[unimplemented-to-supported-exposes-unwired-path]]).

**Variant Discoverability.** Before any new variant (B1/B3/B4, C1/C2), run `cargo engine-inventory` and the `/add-engine-variant` checklist; grep the inventory for sibling-cluster smells (e.g. a `Search*` family that B1 should parameterize rather than sit beside).

---

## Dispatch order (sequential; shared parser files are collision points)

1. **Group A parser batch** (13 cards) — biggest ROI, zero engine risk. Order to avoid `imperative.rs`/`sequence.rs` collisions: A1 Cloak → A4/A5 Dig+LoseLife → A6 RemoveKeyword → A3 SetColor → A7 mana-spend → A8 counters → A2 Animate → A9 Behold-effect. One `/engine-implementer` run per 1–2 families, re-scan `coverage-breakdown.sh --format standard` after each to confirm the card flips supported.
2. **Group B** (7 cards) — B1 multi-zone search (2 cards, one block) → B2 delayed-sac → B3/B4 (variant gate) → B5/B6 triggers.
3. **Adjacent-cluster inheritors** — do C11 (S17 target-in-gy), C12 (S12 else), C10 (S23/S26 alt-cost), C9 (cost-reduction-by-qty) *after* their sibling clusters land so they reuse the block.
4. **Group C heavies — each `/review-engine-plan`-gated before code, then built (none deferred)** — C1 Controlling-Another-Player (CR 723) is sequenced last only because it is the largest new subsystem; it is implemented, not dropped. C2 replacement; C6 multi-target fan-out (already a std-rollout Tier-3 flag); C3–C12 as listed.
5. Re-run `cluster-assign.sh standard` after each batch; S25 must shrink monotonically toward **0** and no cleared card may reappear in `unsupported.tsv`.

**Clearance target: all 40 cards ship — zero deferrals** (per the std-rollout hard rule; S25 → 0). Group A (19) + Group B (8) = 27 land on existing engine primitives; Group C (13) follows via `/review-engine-plan`, with exactly one genuinely new subsystem (C1, CR 723) and the rest small adjacent-cluster inheritors — every one implemented, none dropped or out-of-scope. **19 + 8 + 13 = 40.** Full per-card verification matrices are generated at `/engine-implementer` dispatch time, per batch.

## Review addenda (`/review-engine-plan`, 2026-07-01 — binding on dispatch)

1. **CR fixes applied (were wrong):** B1 originally cited `701.19 (search)` = **Regenerate** and `701.13 (name reference)` = **Exile** — both wrong. Corrected to **CR 701.23 (Search) + CR 201.2a/201.3a (same-name) + CR 701.13 (Exile)**, all grep-verified in `docs/MagicCompRules.txt`. Every remaining CR (701.58, 701.4, 723, 120.10, 603.7, 702.6) re-verified clean.
2. **A9 (Sarkhan) reclassified A → B.** Grep confirms `Behold` exists only as `AbilityCost::Behold`/`BeholdCostAction` (`types/ability.rs:6483,6974`); there is **no `Effect::Behold`**. Sarkhan's "behold a Dragon" is an ETB *effect*, so it needs an effect-side wrapper = engine work, not parser-only. **Effective counts: Group A = 18, Group B = 9, Group C = 13 = 40** (the A9 row stays in the A table for continuity but dispatches as a Group-B item).
3. **Identity/Provenance contracts are MANDATORY per-card at dispatch** (`/engine-planner` check #10). These S25 cards carry "this way / that X / copies of it / that color" provenance that must be pinned (source phrase → authority id/value → binding time → live-vs-latched → storage → consumption → invalidation → multi-authority hostile fixture): **Rhino's Rampage** ("excess damage dealt *this way*" — which damage event), **Prishe's Wanderings** ("search *this way*"), **Deadly Cover-Up / The End** ("cards with *that name*" — latched at resolution from the exiled/chosen card, CR 201.3a), **Foraging Wickermaw** ("become *that color*"), **Niko** ("copies of *it*" — Niko's characteristics + until-end-step duration), **Kav Landseeker** ("sacrifice *that token*"), **Stolen Uniform** ("lose control of *that Equipment*"), **Secret of Bloodbending** ("*that player*"). No family ships without its provenance contract.
4. **Nom-compliance binding:** this strategy plan names the parser file + the dispatch to extend per family, but each `/engine-implementer` sub-plan MUST specify the **exact combinator** for every new detection (extend the existing `alt()`/`tag()`; `parse_*(text).is_some()`, never `contains()`/`starts_with()`). This is a hard gate at sub-plan review, not deferred away.
5. **Verification binding:** each family's per-card matrix (built at dispatch) must include a **runtime** test — cast → measured board delta — for the "engine already resolves this" claim, plus the coverage `false→true` flip; a parser round-trip shape test alone does NOT satisfy the support claim. Cards lowered before their subsystem exists stay red via `Effect::unimplemented`.
6. **Scope matrix required for the heavies:** C1 (control another player — controller/turn/priority scope, CR 723), C6 (multi-target fan-out — target-slot scope), and A6 (RemoveKeyword on *opponent-controlled* creatures — controller scope) must enumerate reachable scope variants + negative sibling tests at dispatch.
7. **Class-vs-card note:** C8 (Vincent's Limit Break, three token faces) is the weakest "class" claim (~1 card). It still ships (no-defer rule); at dispatch, either identify the reusable modal/level-token-face block or record it as a justified single-card consumer.

**Review verdict:** no *architectural* blockers — building-block reuse is sound and grounded, the trace is complete, card-accounting balances (18+9+13=40), and no-deferral is clean. The one hard defect (B1 CR errors) is fixed above. Items 3–7 are binding requirements carried into each `/engine-implementer` dispatch, not open design questions.

## Open items to resolve at dispatch (do not guess now)
- A9/B: is there an `Effect`-side Behold, or only `AbilityCost::Behold`? (grep `Effect::Behold`.)
- A2: is Vraska's "granted activated ability on an animated token + lose all card types" already a lowering block? If not → C.
- A5/B4: does `game/quantity.rs` already have a revealed-object-MV ref and a "total power of a set" aggregate? Reuse before adding.
- B3: parameterize `GainActivatedAbilitiesOfTarget` (activated→all) vs new sibling — decide via `/add-engine-variant`.
