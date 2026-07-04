# S07 Token-Splitter Class Fix — "create A, a B, and a C token" 3+-item middle-drop

**VERDICT (planner, Opus/xhigh): SOUND — clean-peek discriminator. Design the iterating splitter.**
The `parse_token_noun_start` peek is a clean (a)-vs-(b) discriminator, AND the disjunctive trap + quoted-comma edge are already handled by two pre-existing patterns in the same file. No shared-parser rewrite. The fix broadens the binary left/right split into an N-way split reusing mechanisms that already exist.

**Verification note:** Tilt does NOT watch worktree `s07-impl-wt`. All build/test verification is DIRECT cargo; clippy + `test -p engine` EXCEED the ~5-min background-job wall-clock cap → run FOREGROUND with a 10-min timeout. `cargo coverage` guts `known-tokens.toml` → `git checkout crates/engine/data/known-tokens.toml` after.

---

## §1 — Baseline (verified against code at HEAD d22ac9083; line numbers accurate, no drift)

- `try_parse_create_token_sequence` — `mod.rs:3522`
- `split_create_token_sequence` — `mod.rs:3550` (returns a **binary** `(left_orig, left_lower, right_orig, right_lower)`)
- `parse_token_sequence_conjunction` — `mod.rs:3574` = `preceded(tag("and "), peek(parse_token_noun_start))`
- `parse_token_noun_start` — `mod.rs:3578` = `alt(("a ", "an ", "one ", "x ", "that many ", number+" "))`
- `parse_choice_list_separator` — `mod.rs:3649`; `split_choice_list_items` (quote-swallowing) — `mod.rs:3680`
- Dispatch order: `try_parse_create_token_sequence` @ `mod.rs:5831` runs **before** `try_parse_create_token_choice` @ `mod.rs:5838`.

**Root-cause mechanism (verified):** `split_create_token_sequence` (3550-3572) scans for the single `"and "`-conjunction, produces a **binary** split, guard `token::parse_token_description(left).is_some()` (3563). `parse_token_description` (`token.rs:327`) parses from the front, is NOT `all_consuming`, and its literal-name boundary loop stops at `","` (`token.rs:304`). So Bestial Menace `"a 1/1 … Snake …, a 2/2 … Wolf …, and a 3/3 … Elephant …"` → scan finds the lone `" and "` before the LAST item → `left = "a 1/1 … Snake …, a 2/2 … Wolf …,"`, `right = "a 3/3 … Elephant …"`. `parse_token_description(left)` returns `Some` parsing only Snake, dropping `", a 2/2 … Wolf …,"`. `try_parse_token(left_lower,…)` (3534) likewise parses only Snake → `[Snake, Elephant]`, **Wolf silently dropped, no diagnostic**. Reproduces measured `[Treasure, Clue]` / `[Human, Goblin]` (first+last, middle dropped). Comma-only 2-item string has no `"and "` → scan None → declines → single non-Token effect (matches measured).

`split_clause_sequence` (`sequence.rs:593`) NOT involved — untouched.

## §2 — Fix design (nom-first, N-way, reuses three existing mechanisms)

Change `split_create_token_sequence` from **binary** to **N-way** returning `Option<Vec<(&str /*orig*/, &str /*lower*/)>>`, and rewrite `try_parse_create_token_sequence` to chain all N. All combinators already imported at `mod.rs:62-68` — zero new deps.

1. **Conjunctive gate (disjunctive rejector) — KEEP the existing check.** Before splitting, require `nom_primitives::scan_preceded(after_create.lower, parse_token_sequence_conjunction).is_some()`. A create-ALL list is always terminated by an `"and"` coordinator; disjunctive `"A, B, or C"` has no `"and "`+noun → gate declines → falls through to `try_parse_create_token_choice` (3470) building the modal `ChooseOneOf`. This gate IS the conjunctive-vs-disjunctive discriminator; do not remove it.
2. **Conjunctive-only separator + noun-start peek** (new tiny combinator, mirrors `parse_choice_list_separator` minus `or`, plus peek):
   ```
   value((), (
       alt((tag(", and "), tag(", "), tag(" and "))),   // longest-first; NO " or "/", or "
       peek(parse_token_noun_start),
   ))
   ```
3. **Quote-swallowing item** — reuse the exact unit from `split_choice_list_items` (`mod.rs:3681-3685`):
   ```
   let unit = alt((
       recognize((tag("\""), take_until("\""), tag("\""))),          // opaque quoted span
       recognize(preceded(not(<sep from step 2>), anychar)),
   ));
   let item = recognize(many1(unit));
   let (_, lower_items) = all_consuming(separated_list1(<sep>, item)).parse(after_create.lower).ok()?;
   ```
   Map each `lower_items[i]` back to `original` by byte offset (ASCII offset parity; existing 3557-3561 already relies on it). Require `len() >= 2`.

**Caller (`try_parse_create_token_sequence`)** — parse each item via `token::try_parse_token(lower, orig.trim(), ctx)`, require EVERY item is `Effect::Token`, fold into a sub_ability chain (build from the back). **[REVIEW A3 — ownership]** `AbilityDefinition::new` / `parsed_clause` take `Effect` BY VALUE (verified `mod.rs:3542/3545`), so consume OWNED effects (`into_iter()`, not `.iter()`); the pseudocode below is shape-only:
```
let mut chain: Option<Box<AbilityDefinition>> = None;
for (orig, effect) in owned_items_tail.into_iter().rev() {   // owned, not &ref
    let mut def = AbilityDefinition::new(AbilityKind::Spell, effect);
    def.description = Some(format!("create {}", orig.trim()));
    def.sub_ability = chain.take();
    chain = Some(Box::new(def));
}
let mut clause = parsed_clause(head_effect);   // items[0], owned
clause.sub_ability = chain;
```
This reproduces the EXACT current 2-item builder shape (verified `mod.rs:3542-3547`: `AbilityDefinition::new(Spell, right_effect)` + `.description=Some(format!("create {}", right_orig.trim()))`, head=`parsed_clause(left_effect)` w/ `clause.sub_ability=Some(Box::new(right_def))`) — N is the exact generalization, preserving written order item0→…→itemN-1. The runtime chain driver (`game/effects/mod.rs`, walks `sub_ability` at `2179-2188`/`2454`) resolves each plain `Effect::Token` in written order (CR 608.2c) — proven today by The Companion (Food→sub→Royal Role) and Bestial Menace (Snake→sub→Elephant), both currently-supported.

**[REVIEW A1 — decline safety]** ⚠️ Correcting the plan's earlier false claim: if the `"and "`-gate fired (conjunctive list) AND `items.len()>=2` BUT not-every-item is `Effect::Token`, a plain `None` return does NOT surface a diagnostic — it falls through generic dispatch to a SILENT single-leading-token (the same silent-drop class this fix eliminates). ZERO corpus cards trigger this today (all witnesses are pure-token, incl. The Companion — see §4). Still, to defend the no-silent-drop invariant against future cards, emit `Effect::unimplemented(name, fragment)` for the whole create-clause in that branch instead of falling through. ~4 lines; converts a future silent failure into an honest gap.

### Discrimination proof — (a) list separator vs (b) intra-item comma
- **(b1) keyword continuation** — `"with first strike, vigilance, and trample"` / `"with flying and lifelink"`: continuation words (`vigilance`, `trample`, `lifelink`, `first strike`, `menace`, `haste`) never match `parse_token_noun_start` (`mod.rs:3578` — only `a `/`an `/`one `/`x `/`that many `/number+space) → candidate separators inside a keyword list FAIL the peek → not split points. CLEAN.
- **(a) list items** — verified across ALL 9 conjunctive witnesses: every item begins `"a "`/`"an "` → peek PASSES. CLEAN.
- **(b2) quoted-ability comma** — The Companion of the Wilds `with "This creature can't block,"`: comma-then-quote (`,"`) matches no separator; only the ` and ` after the closing quote splits → correct. Reef-Worm `"When this dies, create a 6/6 …"` → `", "` precedes `"create"` (not an article) → peek fails. Quote-swallow unit (step 3) consumes any double-quoted span opaquely → even a hypothetical quoted `", a 2/2 …"` cannot over-split. CLEAN.
- **Disjunctive shared-comma trap (Scoping #1)** — `", "` between first two items is identical in `"A, B, and C"` and `"A, B, or C"`; the peek does NOT distinguish. Retain step-1 `"and "`-gate: declines pure-disjunctive lists → modal parser. RESOLVED without folding `" or "` into the splitter.

All three protections (peek, `"and "`-gate, quote-swallow) already exist in `mod.rs`. → **SOUND.**

## §3 — Discriminating tests (each non-vacuous; revert drops middle / over-splits)

**Parse-level** (`oracle_effect/tests.rs`, `parse_effect_chain`, assert MIDDLE spec, walk `def.effect` + `sub_ability` + `sub_ability.sub_ability`):
- **Bestial Menace** (3 simple): `[Snake 1/1, Wolf 2/2, Elephant 3/3]` — assert node[1] name=`Wolf`, P/T 2/2. (Revert → Wolf missing.)
- **Fae Offering** (3 predefined artifacts): `[Clue, Food, Treasure]` — assert node[1] name=`Food`.
- **Triplicate Titan** (3 items WITH trailing keyword clauses): assert node[1] keywords=`[Vigilance]` AND exactly 3 nodes (intra-item `"with flying"` didn't over-split; `", and "` split correctly).
- **Trostani's Summoner** (mixed): node[1]=`Centaur 3/3` (no keyword), node[0]=`Knight` w/ Vigilance, node[2]=`Rhino` w/ Trample.
- **[REVIEW B1] The Companion of the Wilds** (3 items, quoted intra-item comma, corrects a real current misparse — THE best discriminating witness): `"create a Food token, a 1/1 black Rat creature token with \"This creature can't block,\", and a Royal role token attached to a creature you control."` → 3 chained `Effect::Token` `[Food, Rat, Royal Role]`. Assert node[1]=`Rat` 1/1 black AND the `CantBlock` static lands on **node[1] (Rat)**, NOT node[0] (Food). (Current binary parse WRONGLY attaches can't-block to Food + drops the Rat entirely; the role token already parses as `Effect::Token{types:[Enchantment,Aura,Role]}`. NOT a token.rs gap — the plan's earlier "role token isn't a Token" exception was FALSE.)
- **[REVIEW A2] NON-REGRESSION that ACTUALLY exercises the new step-2 peek** — note the all-keyword `"Zombie … with first strike, vigilance, and trample"` case is declined by the UNCHANGED `"and "`-gate (`trample` fails `peek`) BEFORE step-2 runs, so it does NOT test the new peek (vacuous for it). Add a gate-PASSING witness with an intra-item keyword comma: `"Create a 2/2 Zombie token with menace, vigilance, and a 1/1 Bird token with flying."` → expect EXACTLY 2 tokens `[Zombie(menace,vigilance), Bird(flying)]`. (Revert the step-2 peek → `", "` after `menace` splits off a bare `"vigilance"` item → not a Token → decline → test fails. This is the ONLY test that gates the new peek.) Also keep the existing all-keyword `does_not_split` test (`tests.rs:9988`, actual line) green as a gate-level guard.
- Keep existing passing 2-item tests (`tests.rs:9944`, `9969`) green.

**Cast-level** (`crates/engine/tests/integration/`, GameScenario + `GameRunner::cast(...).resolve()`):
- **Bestial Menace** full-resolve → battlefield has all 3 (Snake+Wolf+Elephant), token delta = 3.
- **Fae Offering** resolve → Clue+Food+Treasure all present. (Revert → 2 tokens; fails.)

## §4 — Empirical corpus gate (before/after; FOREGROUND direct cargo)
- Parse-dump the 10 conjunctive witnesses — A Killer Among Us (Human/**Merfolk**/Goblin), Bestial Menace, Fae Offering, Overencumbered (Clue/**Food**/Junk), Somberwald Beastmaster, Liberated Livestock, Mascot Exhibition, Triplicate Titan, Trostani's Summoner, The Companion of the Wilds — assert each now yields its **middle** token (3 chained `Effect::Token`). **[REVIEW B1]** ALL 10 are pure-token including The Companion (its Royal Role parses as `Effect::Token`) — NO "not-yet-a-Token" exception. The Companion additionally CORRECTS a current misparse (can't-block moves off Food onto the Rat) — see §3.
- `cargo coverage` full DB: **REGRESSED(engine) = 0**. Then `git checkout crates/engine/data/known-tokens.toml`.

## §5 — CR annotations (grep-verified against docs/MagicCompRules.txt)
- **CR 111.1** (645) — tokens are markers. **CR 111.2** (647) — creator is owner/controller. **CR 608.2c** (2793) — "follows its instructions in the order written … apply the rules of English" — load-bearing: comma+`and` list is do-ALL in written order; the peek is the "rules of English" discriminator. **CR 608.2d** (2795) — choices while applying — cite on the *disjunctive* `try_parse_create_token_choice` path only.
- Reuse the existing `// CR 111.2 + CR 608.2c` header on `try_parse_create_token_sequence` (`mod.rs:3518`); do NOT introduce 122.1.

## §6 — STOP-AND-RETURN trigger (explicit)
STOP fires iff `parse_token_noun_start` could NOT discriminate (a) from (b) without a broad token-parser rewrite — e.g. genuine list items beginning with bare keyword words, or quoted abilities routinely containing `", a <article>"` the quote-swallow couldn't isolate. **Measured: none hold** (§2). Disjunctive shared-comma does not trigger STOP (the `"and "`-gate resolves it). → **No STOP. Proceed.**

## §7 — Files the executor touches (narrow)
- `crates/engine/src/parser/oracle_effect/mod.rs` — rewrite `split_create_token_sequence` (3550) → N-way `Vec`; add the conjunctive separator combinator near 3574 (keep `parse_token_sequence_conjunction` as the gate); rewrite chain-build in `try_parse_create_token_sequence` (3522).
- `crates/engine/src/parser/oracle_effect/tests.rs` — parse-level + non-regression tests (§3).
- `crates/engine/tests/integration/<new>.rs` — Bestial Menace + Fae Offering cast-level (§3).
- `crates/engine/src/parser/oracle_effect/token.rs` — **[REVIEW B1] NOT expected** (all 10 witnesses' items already parse as `Effect::Token`, incl. The Companion's Royal Role). Touch ONLY if a measured item genuinely fails to parse as `Effect::Token` (none known); if so, journal as tracked debt, do NOT expand scope.
- **[REVIEW A4 — named-token ceiling, ponytail note only]** No corpus card is a named-token-FIRST conjunctive sequence (verified: "create Boo, a legendary … Hamster token" has no `"and "`+article → gate declines → unchanged single-token dispatch, no regression). Known ceiling: a hypothetical `"create Boo, a legendary 1/1 Hamster token, and a Treasure token"` would over-split `"Boo"` (`", "`+`a legendary` peek) → `"Boo"` not a `Token` → decline. ZERO such cards today — add a `// ponytail:` comment at the splitter noting this ceiling, no code for it.

---

## Disjunctive classification (Rider 1 — probe, don't fix)
All three route to `try_parse_create_token_choice` (`mod.rs:3470`), strip `opt("your choice of ")` (3483), split `" or "`/`", or "` via `split_choice_list_items` into a CORRECT `ChooseOneOf`:
- **Chicago Loop** — 3 modal branches (Dinosaur Skeleton/trample, Bear/haste, Bird/flying). **Already-correct modal.**
- **The Third Doctor** — `ChooseOneOf` Clue/Food/Treasure. **Already-correct modal.**
- **Transmutation Font** — `ChooseOneOf` Blood/Clue/Food. **Already-correct modal.**

Classification: **not (a), not (b) — a third, better state: correctly-parsed modal (create-ONE).** NO false-supported hazard. The conjunctive fix does NOT touch them (excludes `" or "`; `"and "`-gate declines them → choice parser at 5838). CHEAP CONFIRM for executor: one-line `parse_effect_chain` dump each → expect `Effect::ChooseOneOf { branches:[3] }`, not `Unimplemented`. No modal machinery this increment.

---

## Fix-mechanism summary
Binary → N-way `split_create_token_sequence`: keep the `"and "`-conjunction gate (rejects disjunctive), split the whole create-list with `separated_list1` over conjunctive-only separators `{", and ", ", ", " and "}` each guarded by `peek(parse_token_noun_start)` + a quoted-span-swallowing item unit — both lifted verbatim from sibling `split_choice_list_items`. Chain all N `Effect::Token` via `sub_ability`. Zero new imports, ~one small combinator, three protections all pre-existing. Middle survives; keyword-list single tokens don't over-split; disjunctive `"or"` stays modal.
