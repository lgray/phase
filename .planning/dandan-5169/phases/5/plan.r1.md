# Phase 5 plan — CR 612 word-substitution primitive (Magical Hack, Crystal Spray, and the color/land-type class)

Mode: engine-planner, phase-plan mode, phase k=5, round 0. Planned against HEAD `06554d0e` (Phase 4 edits only `parser/swallow_check.rs`; this plan touches nothing there). Nothing in this plan was compiled or run (a cargo build by another agent was in flight and the charter says read-first); every assertion is labelled MEASURED (command named) or UNESTABLISHED. Claims ledger at the end.

## 0. Premise verification (Step 0)

Oracle text read verbatim from `client/public/card-data.json` (jq) and, for the two named cards, Scryfall (`api.scryfall.com/cards/named?exact=`), identical in both:

- Magical Hack (Instant, {U}): "Change the text of target spell or permanent by replacing all instances of one basic land type with another. (For example, you may change "swampwalk" to "plainswalk." This effect lasts indefinitely.)"
- Crystal Spray (Instant, {2}{U}): "Change the text of target spell or permanent by replacing all instances of one color word with another or one basic land type with another until end of turn. / Draw a card."

Class measured with `jq` over every card whose `oracle_text` matches `change the text|replacing all instances` (17 hits). Card-data parse today for each is `Effect::Unimplemented` (`unrecognized_clause_head`) except March of Progress (Overload, already parsed) and Exchange of Words (CR 612.5, no ability list).

| Card | Clause (verbatim core) | Domain | Duration | This phase |
|---|---|---|---|---|
| Magical Hack | target spell or permanent, one basic land type | Land | indefinite | IN |
| Crystal Spray | target spell or permanent, one color word … or one basic land type …, until end of turn; "Draw a card." | Color or Land (player picks) | EOT | IN |
| Sleight of Mind | target spell or permanent, one color word | Color | indefinite | IN |
| Alter Reality | same as Sleight; Flashback {1}{U} | Color | indefinite | IN (Flashback already a parsed keyword) |
| Glamerdye | same; Retrace | Color | indefinite | IN (Retrace already a parsed keyword) |
| Mind Bend | target permanent, color or land | Color or Land | indefinite | IN |
| Spectral Shift | modal: land mode / color mode; Entwine {2} | Land, Color | indefinite | IN (Entwine already parsed) |
| Trait Doctoring | target permanent, color or land, until end of turn; Cipher | Color or Land | EOT | IN (Cipher already parsed) |
| Whim of Volrath | target permanent, color or land, until end of turn; Buyback {2} | Color or Land | EOT | IN (Buyback already parsed) |
| Balduvian Shaman | `{T}:` target white enchantment you control that doesn't have cumulative upkeep, one color word; "That enchantment gains "Cumulative upkeep {1}."" | Color | indefinite | IN ONLY IF the target phrase and the cumulative-upkeep grant parse with existing building blocks (UNESTABLISHED, measured by coverage); otherwise the whole card stays `Effect::unimplemented` on the strict-failure path. No new grant machinery is built for it. |
| Artificial Evolution | one creature type, "The new creature type can't be Wall." | Creature type | indefinite | OUT — see §1.3 |
| New Blood | gain control + "replacing all instances of one creature type with Vampire" | Creature type | indefinite | OUT — §1.3 |
| Magical Hacker | replace "+" with "-" and vice versa | mana-ability symbols | EOT | OUT — §1.3 |
| Deceptive Divination | replace "sorcery" with "instant" on cards not on the stack | card-type word | — | OUT — §1.3 |
| Exchange of Words | exchange text boxes (CR 612.5) | whole text box | — | OUT — different primitive |
| March of Progress | Overload "target"→"each" | — | — | already supported via Overload; untouched |

Class size in scope: 9 cards certain, 10 with Balduvian Shaman (MEASURED by the jq above). Crusade-style probing: `Bad Moon` ("Black creatures get +1/+1.", one static with `HasColor`) exists in card-data (MEASURED) and is the static-ability fixture.

## 1. CR 612 reading (greps of `docs/MagicCompRules.txt`, all MEASURED)

- 612.1: text-changing effects change words on the object, generally rules text and type line. 613.1c: Layer 3.
- 612.2: only words "used in the correct way" change; a color word used as a color word, a land type used as a land type. A color-word/subtype change can't change a card name even if the name contains the word.
- 612.2a: token-creating text's creature types double as names and ARE changed (creature-type domain only; not reached here).
- 612.3: granted abilities are not text, so they are not changed (Layer 6 runs after Layer 3).
- 612.4: a token's subtypes and rules text are defined by its creating spell/ability and are changeable.
- 608.2b: target legality is rechecked on resolution, and "an effect may have changed the text of the spell" is named there.
- 608.2d: choices an effect offers are made as it resolves (the words are chosen on resolution, not at cast).
- 611.2a: no stated duration means until end of game. 611.2c: the affected set is fixed when the effect begins.
- 613.7b: resolution-generated effects take a timestamp when created; 613.7: timestamp order within a layer.
- 305.6/305.7: basic land types; a basic land type grants an intrinsic mana ability. 205.3i: land types.
- 105.1: five colors. 702.14a: landwalk "[type]walk".
- 400.7a: effects from spells that change characteristics of a permanent spell continue onto the permanent. The engine has a documented pre-existing deviation for ALL grants (comment in `game/effects/effect.rs`, "KNOWN DEVIATION (task #129)"). This phase does not close it and makes no claim about a text change surviving spell→permanent.

### 1.1 What "word" means (the contract the whole design rests on)
A value is a text word iff the Oracle phrase that produced it renders that value as a color word ("black") or a land-type word ("Forest", "forestwalk"). A mana symbol ({B}), a card name, an option label, a mana-cost shard are NOT words (CR 612.2). Consequence: `ManaColor` and land-type strings are both words in some carriers and not words in others, so the substitution is **carrier-directed**, never a blind value replace.

### 1.2 Domain
`{ColorWord, BasicLandType}` — the two words the cards in scope name. Five colors (CR 105.1) and five basic land types (CR 305.6) are closed sets, so the choice is a finite enumerable prompt.

### 1.3 Why creature types and the other members stay OUT (CR 612 reasoning)
CR 612 treats creature type words identically, so the primitive is built to take them as one more leaf, but the members cannot ride this phase honestly: (a) the charter fixes the domain at `{BasicLandType, ColorWord}` and the brief calls creature type "the class's next member"; (b) the creature-type domain is an open string set (CR 205.3m registry, `ChoiceType::CreatureType` already exists) with Artificial Evolution's "can't be Wall" restriction and New Blood's fixed target word and gain-control, each needing its own restriction/parse path; (c) CR 612.2a makes token names follow creature-type changes, a carrier rule that does not exist for the two domains here and would be untested. Magical Hacker swaps mana-ability symbols ("+", "-"), which are not one of the typed word domains. Deceptive Divination replaces a card-type word on objects not on the stack (a plane, different population and domain). Exchange of Words is CR 612.5, a text-box exchange, a different primitive. All five stay `Effect::unimplemented` by construction: the new parser recognizer fails closed on any domain phrase other than "color word" / "basic land type" (a discriminating parser test, §9, pairs each with a positive sibling).

## 2. Skills and checklists applied

- `add-engine-variant` gate (run on `ContinuousModification::SubstituteTextWord`, below): APPROVED.
- `add-static-ability` (layer placement, exhaustive arms, layer dirty tracking) — applied; `add-engine-effect` — **no new `Effect` variant**: the handler seam is the existing `Effect::GenericEffect` plus the existing `Effect::Choose` (Labeled); `add-interactive-effect` — **no new `WaitingFor`/`GameAction`**: the words are chosen through the existing `WaitingFor::NamedChoice` round trip, so the AI legal-action, multiplayer-routing and frontend checklist rows are satisfied by reuse (rows stated in §10 as reuse).
- `oracle-parser` (nom mandate, ParsedEffectClause, CR annotation) and `card-test` (cast-pipeline recipe, six foot-guns) — applied in §9/§10.

### 2.1 `add-engine-variant` gate
Stage 1 existence (MEASURED: `data/engine-inventory.json` `ContinuousModification.variants` printed; `grep -rn "TextWord\|TextSubstitution\|ColorWord" crates --include=*.rs` returns nothing). Closest slots and why each is not the concept:
- `SetTextName`/`SetChosenName`/`SetName`: set the NAME (CR 612.8 / 707.9b). The new concept replaces words inside rules text and type line (CR 612.1/612.2) and must NOT touch the name (612.2). Different object of the operation.
- `SetBasicLandType`/`AddSubtype`/`AddChosenSubtype`: Layer-4 type-setting (CR 305.7/205.1) — they overwrite the land's subtypes; they do not rewrite a keyword, a filter, or a condition. Magical Hack on "swampwalk" on a non-land creature is impossible to express with them.
- `AddChosenColor`/`SetColor`: Layer-5 color (CR 105.3), not text.
- `Overload` transform / `Cleave` bracket strip: parse/cast-time structural rewrites of a fixed keyword's effect (traced, §4); they substitute no player-chosen word on another object.
Verdict: DOES_NOT_EXIST.
Stage 2 parameterization: the only `Name`-rooted cluster is `SetName`/`SetTextName`/`SetChosenName` (three variants sharing the `Name` root). The new variant is not on that axis, so it is not a fourth sibling; leaving that cluster untouched is deliberate (absorbing it would rewrite three serialized shapes for no card). Inside the new variant the axes are parameterized, not proliferated: one variant carrying a typed `TextSubstitutionSpec` (Fixed | Chosen) over a typed `TextSubstitution` (Color | BasicLandType) — no `SubstituteColor`/`SubstituteLandType`/`SubstituteChosen…` siblings. Verdict EXTEND_OK.
Stage 3 categorical boundary: the whole axis lies in CR 612 (612.1, 612.2). Color and basic land type are both *words* inside one text-changing rule, not two rule sections. WITHIN_SECTION.
Post-gate items: CR annotation (greps above), exhaustive arms with no wildcard (`cargo check` lists them; expected list in §8), the `ability_scan.rs`/`ability_rw.rs` classifier arms (§8), runtime status (real handler, no stub), serialized-surface audit (§6.6: `ContinuousModification` is inside `TransientContinuousEffect` inside `GameState`, so protocol bump).

## 3. Design

### 3.1 Types (`types/ability.rs`, beside `BasicLandType`)
```
/// CR 612.2: the word classes a text-changing effect can name.
enum TextWordDomain { ColorWord, BasicLandType }
/// CR 612.2: one concrete from→to word replacement; both words are of one class.
enum TextSubstitution {
    Color { from: ManaColor, to: ManaColor },
    BasicLandType { from: BasicLandType, to: BasicLandType },
}
/// Fixed = latched words (what the layer applies). Chosen = parse-time form,
/// words picked on resolution (CR 608.2d), latched to Fixed when the effect installs.
enum TextSubstitutionSpec { Fixed(TextSubstitution), Chosen { domains: Vec<TextWordDomain> } }
ContinuousModification::SubstituteTextWord { substitution: TextSubstitutionSpec }   // layer(): Layer::Text
```
Methods on `TextSubstitution` (single authority for label round trip; no duplicated option lists): `options(domains: &[TextWordDomain]) -> Vec<String>` (every ordered pair with `from != to`, domain order then WUBRG×WUBRG / Plains…Forest order), `label(&self) -> String` (`"Black -> Blue"`, `"Forest -> Island"`), `from_label(label, domains) -> Option<Self>` (rejects `from == to` and any label outside `domains`). "It can't change a word to the same word" (ruling) holds by construction; the rules let the player name words absent from the object (ruling "can target a card with no appropriate words"), so options are NOT narrowed to words present. The choice is one atomic prompt because both words are chosen at the same resolution point (CR 608.2d); a pair prompt also makes "from ≠ to" structural, and Crystal Spray's domain pick is folded into the same pair list (20 color pairs + 20 land pairs, unambiguous labels).

Why `Fixed|Chosen` inside one variant and not `SubstituteTextWord` + `SubstituteChosenTextWord`: the codebase's Dynamic/Fixed pairs (`AddDynamicPower` → `AddPower`) are siblings of the same name root; parameterizing avoids a third cluster. A `Chosen` modification reaching the layer is inert (never applied); `snapshot_transient_modifications` (the existing latch seam, `game/effects/effect.rs`) converts Chosen → Fixed.

### 3.2 Emitted ability shape (parser output, all existing variants)
Precedent (MEASURED): Chaoslace card-data = `GenericEffect { static_abilities: [continuous, affected: ParentTarget, [SetColor]], duration: Permanent, target: Or[StackSpell, Typed Permanent] }`; `try_parse_become_choice` emits `Choose → sub_ability GenericEffect`.
Output for "Change the text of <target> by replacing all instances of one <domain> with another [or one <domain> with another] [until end of turn]":
```
Choose { choice_type: Labeled { options: TextSubstitution::options(domains) }, persist: false, selection: Chosen }
  └ sub_ability: GenericEffect {
        static_abilities: [ continuous().affected(ParentTarget)
                            .modifications([SubstituteTextWord{ Chosen{domains} }]).description(clause text) ],
        duration: Some(Permanent | UntilEndOfTurn),
        target: Some(<parsed target filter>), end_cost: None }
        └ sub_ability: (Crystal Spray only) Draw 1 — produced by the normal chain assembler from "Draw a card."
```
Duration: no stated duration ⇒ `Duration::Permanent` (CR 611.2a), set explicitly by the recognizer because `effect.rs` defaults `None` to EOT (the documented known deviation). "until end of turn" ⇒ `UntilEndOfTurn`.

### 3.3 Where the substitution is applied (consumption seams)
A text change has to reach every reader of the object's rules text. Readers are of two kinds:

(a) **Permanents (battlefield).** New module `game/text_substitution.rs` owns `apply_to_permanent_text(obj, TextSubstitution)`. Called from a **Layer-3 pre-gather pass** in `evaluate_layers` (`game/layers.rs`), placed after Layer 1/1b/stickers and its static-index refresh and before `gather_active_continuous_effects` (Step 3). Reason: the main loop gathers every Layer 4–7 effect once from `static_definitions` before any layer applies, so a text change applied in the Text bucket would be too late for a changed static ability ("Black creatures get +1/+1" on Bad Moon) — the same reason Layer 1 has its own fixed-point pass (CR 613.2). The pass collects the transient effects whose modification is `SubstituteTextWord{Fixed}` via `gather_transient_continuous_effects` (cheap; no whole-board static scan), orders them with `order_active_continuous_effects(Layer::Text, …)` (timestamp, CR 613.7/613.7b), matches recipients through the existing recipient/affected-set machinery (CR 611.2c set fixed at start), applies, then those effects are removed from the `Layer::Text` bucket of the main gather so nothing applies twice. `ContinuousModification::layer()` still returns `Layer::Text` (charter: "evaluated at the existing text layer"); `apply_continuous_effect`'s arm for the variant is an explicit documented no-op (applied in the Text pre-pass above; a static-definition-hosted instance is not produced by any parser path). Layer 6 runs later, so granted abilities are untouched (CR 612.3).

What `apply_to_permanent_text` rewrites, all of it the object's rules text or type line (CR 612.1): `obj.abilities`, `obj.trigger_definitions`, `obj.static_definitions`, `obj.replacement_definitions` — printed entries only; entries for which `ReplacementDefinition::is_resolution_installed()` are continuous effects a resolution created, not text, and are skipped (CR 612.3 spirit; `reseed_replacements_carrying_resolution_effects` shows the printed/installed split) — `obj.keywords`, and the subtype list of `obj.card_types` (land-type word on the type line; result de-duplicated, since a subtype set has no repeats). The object's name, mana cost, color indicator and P/T are never touched (CR 612.2; mana symbols and cost shards are not words). The basic-land intrinsic mana ability is derived after the Type layer by `apply_intrinsic_basic_land_mana_abilities` (MEASURED in `evaluate_layers`), so a Forest whose subtype became Island taps for {U} with no extra code (CR 305.6/305.7).

(b) **Spells on the stack.** Stack objects are not reset by the layer pass (MEASURED: the stack-seed loop in `evaluate_layers` resets only `keywords` and `controller`, with a comment to extend the reset authority before a static modifies another stack characteristic), and a resolving spell reads the `ResolvedAbility` stored in its stack entry, not `obj.abilities`. So for `obj.zone != Zone::Battlefield` the layer does NOT mutate the object (a mutation would be sticky). Instead `text_substitution::restamp_resolving_spell_text(state, object_id, &mut ResolvedAbility)` is called in `resolve_top` (`game/stack.rs`) directly beside the existing "CR 608.2c re-stamp the resolving spell's baked controller" block, before `bind_resolving_ability_referents` and before the CR 608.2b legality recheck, for `is_spell` entries only (an activated/triggered ability on the stack is not a spell or permanent; a text change cannot reach it). It collects the applicable `SubstituteTextWord{Fixed}` effects with a collector modelled on `collect_applicable_off_zone_keyword_effects` (`layer == Layer::Text`, same recipient/condition filter, ordered by `order_active_continuous_effects(Layer::Text, …)`), then rewrites the ability chain's `effect` and `condition` fields (recursing `sub_ability`, `else_ability`, `mode_abilities`) — a typed, field-scoped rewrite, never a whole-`ResolvedAbility` round trip, so targets, paid-cost state and controller are untouched.

### 3.4 The rewrite engine (`game/text_substitution.rs`)
`TextSubstitution::rewrite<T: Serialize + DeserializeOwned>(&self, value: &T) -> Option<T>`: serialize to `serde_json::Value`, walk, rewrite at carrier positions only, deserialize back; `None` when nothing changed or the round trip fails (fail-closed: text unchanged). Why a value walk and not a hand-written exhaustive mutable visitor: the engine has no mutable typed walker over `Effect`/filters/conditions (MEASURED: `ability_visit.rs` is an immutable `&Effect` visitor that descends abilities only; `each_target_filter_mut` in `parser/oracle_effect/mod.rs` is explicitly partial with a `_ => {}` arm), and the color/land words are spread over ~55 measured carrier positions (census below). A new exhaustive mutable walk of the AST would be a separate multi-thousand-line project; the value walk touches each carrier once.

Completeness is enforced by measurement instead of by a hand-kept list being "probably right": **carrier classification table** `WORD_CARRIERS` (one `const` table in this module, each row `(container tag, field, WordClass)` with `WordClass = Word(TextWordDomain) | NotAWord(reason)`), plus a **census test** (§9 row C1) that walks every card face in the shared card DB (`test_support::shared_card_db()`: committed fixture by default, full export under `FORGE_TEST_FULL_DB=1`), serializes `abilities/triggers/static_abilities/replacements/keywords`, and fails on any `ManaColor` or basic-land-type string whose `(container, field)` is in neither class. A new card shape that introduces a new position turns the test red and forces a Word/NotAWord decision. Round-trip identity (`from_value(to_value(x)) == x`) is asserted over the same corpus (row C2). MEASURED support: `types/ability.rs` contains no `#[serde(skip)]` field (grep), and card-data itself is the serialized form the engine loads.

Initial classification, from a census I ran (python over `client/public/card-data.json`, all `abilities/triggers/static_abilities/replacements/keywords`; signature = (nearest `type` tag, key)). The executor reruns it with the test and completes the table; the rule is §1.1 (decide by reading the listed example card's Oracle text):
- Color, Word: `HasColor.color`, `NotColor.color`, `SetColor.colors`, `AddColor.color`, `{"Color": c}` inside `AddKeyword/RemoveKeyword/WithKeyword/WithoutKeyword` and top-level keywords ("protection from white"), hexproof-from-color `{"type":"Color","data":c}`, `Token.colors` (CR 612.4, "create a 1/1 white Spirit"), `ManaColorSpent.color` ("if black mana was spent"), `CastManaSpentMetric::OfColor.color`, `DevotionGE.colors` / `DevotionColors::Fixed`, `SourceIsColor.color`, `Choose.excluded` ("a color other than white").
- Color, NotAWord (symbols/identity): `Cost.shards`, mana production (`Fixed.colors`, `AnyOneColor.color_options`, `AnyCombination`, `Mixed`, `ChoiceAmongCombinations.options`, `ChosenColor.fixed_alternative`, `ReplaceWith.mana_type`), `ManaSymbolCount`, `ManaSymbolsInManaCost`, `Choose.options` (prompt labels), `Fixed.value` mana lists. Examples to read: Llanowar Elves ("{T}: Add {G}."), Cadaverous Bloom.
- Land, Word: `Typed.Subtype` (filter subtype), `{"Landwalk": t}` in keywords and grants/removals and `Token`, `AddSubtype.subtype`, `SetBasicLandType.land_type`, `UnlessControlsSubtype.subtypes`, `Token.types` entries equal to a land type (CR 612.4), keyword-cost carriers with a `subtype` field (typecycling), `ChangeZone.subtypes`, `qualifier`/`Subtype` keyword carriers to be classified from their example cards (crevasse, dross golem).
- Land, NotAWord: `Token.name`, `Conjure.name` (card/token names, CR 612.2: "Forest Dryad" stays "Forest Dryad"), `Choose.options`.

Two rewrite guards beyond the table: a rewrite only fires when the carrier's current value equals `from`; `from == to` is a no-op.

### 3.5 Choice latch (`game/effects/effect.rs`, `snapshot_transient_modifications`)
One new arm: `SubstituteTextWord { Chosen { domains } }` → read `state.last_named_choice`; if it is `ChoiceValue::Label(l)` and `TextSubstitution::from_label(l, &domains)` succeeds, emit `SubstituteTextWord { Fixed(sub) }`; otherwise leave the `Chosen` form (inert, nothing applies — fail-closed). This is exactly how `AddDynamicPower` is latched at install time (CR 608.2h: information fixed when the effect is applied; CR 611.2c). The prompt is answered through the existing `WaitingFor::NamedChoice` / `GameAction::ChooseOption`; `bind_named_choice` with `persist: false` stores `last_named_choice = Label(..)` (MEASURED: `ChoiceValue::from_choice` maps `Labeled` to `Label`; `effects/choose.rs` sets `last_named_choice`). The player answering is the spell's controller (CR 608.2d).

### 3.6 Write-classification for incremental layer flush (`game/layers.rs`)
`modification_characteristic_writes_at(SubstituteTextWord)` returns `CharacteristicKinds::ALL` (comment: rewrites arbitrary rules text, which no kind set bounds; over-approximation only over-escalates). Pinned by the layer-parity test (row L1).

## 4. Analogous trace (hard gate)
Traced features and paths followed:
- **Chaoslace** (card-data `chaoslace.abilities[0]`; `parser/oracle_effect/subject.rs` "becomes" clause → `ParsedEffectClause` → `Effect::GenericEffect` with `affected: ParentTarget`, `duration: Permanent`, target `Or[StackSpell, Typed Permanent]` → `game/effects/effect.rs::resolve` → `game/layers.rs` TCE apply). Supplies the target filter shape, `ParentTarget` binding, Permanent duration.
- **try_parse_become_choice** (`parser/oracle_effect/subject.rs`): `Choose → sub_ability GenericEffect` chain returned from one clause; **AddChosenColor** (`types/ability.rs` → `types/layers.rs` → `game/layers.rs`) for the chosen-read layer pattern; **`snapshot_transient_modifications`** (`game/effects/effect.rs`) for the latch seam; **SetTextName/SetChosenName** (`types/ability.rs:SetTextName` → `types/layers.rs` Layer::Text → `game/layers.rs` arm → the ~18 exhaustive-arm files).
- **Cleave** (`database/synthesis.rs`: `prepare_cleave_oracle_text`, `parse_oracle_with_cleave_brackets`): a build-time second parse of the card's own printed text with bracketed spans removed, stashed as `CleaveVariant`. **Overload** (`game/effects/overload.rs::transform_ability_def`): a cast-time typed rewrite of one fixed keyword's effect tree (`target`→`each`), partial by design. Neither substitutes words the resolving player names, on another object, at resolution, so neither shares the seam; **neither is edited** (the charter's "genuine shared seam" test fails: Cleave's output is parse-time data, Overload's rewrite is a closed effect-shape map with no word domain).
- **Off-zone keyword collector** (`game/off_zone_characteristics.rs::collect_applicable_off_zone_keyword_effects`): template for the stack-spell effect collector.
- **Controller re-stamp on a resolving spell** (`game/stack.rs::resolve_top`, `set_controller_recursive`): template for the resolution-time rewrite seam.

## 5. Pattern Coverage
Cards: 9 certain + Balduvian Shaman conditional (§0). Class: "a continuous effect that replaces one color word or one basic land type word with another on a spell or permanent" — every class member's effect is one `SubstituteTextWord`; the domain list, target filter and duration are parameters, so Mind Bend, Whim of Volrath, Trait Doctoring, Sleight, Alter Reality, Glamerdye and Spectral Shift need no code beyond the two charter cards. Adding creature types later is one `TextSubstitution` leaf plus its carrier rows.

## 6. Plan sections required by the skill

### 6.1 Sizing
Units (one unit = one coherent mechanic by one checklist pass regardless of lockstep layers): **1 unit** — "CR 612 word substitution on the color/land-type domain". It is one mechanic whose pieces are lockstep layers of one pass (types → latch → layer application → stack consumption → parser), none of which is shippable or testable alone: the variant without the parser is unreachable, the layer arm without the stack seam makes every spell target wrong, the parser without the latch applies nothing. Independently-tested behaviors, stated so the sizing is consistent with the body: permanent text change; static-ability text change; spell-on-stack text change (CR 608.2b); timestamp order; word-class guard (612.2); duration (EOT expiry vs indefinite); word choice round trip; parser fail-closed for out-of-domain text. These are assertions of the one primitive, not separate mechanics.
Dependency edges: types → (latch, layers, stack seam, parser) → tests; no inter-unit edges.
Scope-path count (phase-fit counting rule; fixtures and regenerated data excluded; directory entries expanded): **30 paths**, scope matrix §8. T2 fires (≥13); T1 does not (1 unit); T1∧T2 does not fire, matching the charter.

### 6.2 Building Blocks
`BasicLandType` and `ManaColor` (types), `Duration::{Permanent, UntilEndOfTurn}`, `Effect::{Choose, GenericEffect}`, `ChoiceType::Labeled`/`ChoiceValue::Label`, `StaticDefinition::continuous()`, `TargetFilter::{Or, StackSpell, ParentTarget}` via `oracle_target::parse_target`, `oracle_nom::duration::parse_duration`, `oracle_nom::bridge::nom_on_lower`, `ParsedEffectClause`, `gather_transient_continuous_effects`, `order_active_continuous_effects`, `snapshot_transient_modifications`, `collect_applicable_off_zone_keyword_effects` (pattern), `apply_intrinsic_basic_land_mana_abilities`, `set_controller_recursive` (pattern), `test_support::shared_card_db`, `GameScenario`/`GameRunner::cast`. New helpers and why: `TextSubstitution::{options,label,from_label,rewrite}` and the carrier table (no existing mutable word walker), the stack-spell collector (the off-zone collector is keyword-only and `Layer::Ability`-only), `try_parse_text_change_clause` (no existing recognizer).

### 6.3 Logic Placement
Types and label round trip in `types/ability.rs` (engine types). Word carriers and rewrite in `game/text_substitution.rs` (engine game logic; the frontend computes nothing: labels are engine strings). Layer pass in `game/layers.rs`. Latch in `game/effects/effect.rs`. Resolution seam in `game/stack.rs`. Parser recognizer in `parser/oracle_effect/text_change.rs`, hooked in `parser/oracle_effect/mod.rs`. No adapter or client logic.

### 6.4 Rust Idioms
Typed enums throughout (no bools; `Fixed|Chosen` instead of a `chosen: bool`). Exhaustive `match` on `ContinuousModification`/`TextSubstitution` with no wildcard. Constructors reject `from == to` (`Option`). The carrier table rows are typed (`WordClass`), not strings-of-meaning; only JSON tag/key names are strings because that is what the serialized carriers are. `rewrite` returns `Option`, never panics.

### 6.5 Nom Compliance (files under `parser/` change)
`try_parse_text_change_clause(tp: TextPair) -> Option<ParsedEffectClause>` runs inside `nom_on_lower`. Combinators: `tag("change the text of ")`, then `oracle_target::parse_target` for the target phrase (existing authority; its remainder feeds the next tag), `tag(" by replacing all instances of one ")`, `alt((value(ColorWord, tag("color word")), value(BasicLandType, tag("basic land type"))))`, `tag(" with another")`, `opt(preceded(tag(" or one "), domain-alt, tag(" with another")))` (second domain, must differ from the first), `opt(preceded(tag(" "), parse_duration))`, `opt(tag("."))`, `eof`. No `contains`/`starts_with`/`find` dispatch. The recognizer is placed at the top of `parse_effect_clause` beside `try_parse_temporary_cant_become_tapped` (which documents that a recognizer owning its duration must run before the clause shell peels it), so `until end of turn` is consumed by `parse_duration` here and is not double-peeled. Any other domain phrase ("creature type", "+"…) fails `alt`, returns `None`, and falls to today's unrecognized path.

### 6.6 Extension vs Creation
Extends: new `ContinuousModification` variant in the existing Text layer; existing `GenericEffect`+`Choose` chain; existing latch seam; existing NamedChoice round trip. Creates: a new module (`text_substitution.rs`) and one recognizer file because no equivalent exists (measured §2.1); justified by the word-carrier table being the single authority for CR 612.2 "used in the correct way".
Serialized surface: `ContinuousModification` is serialized inside transient effects in `GameState` and in card-data, so the full-game protocol moves 93→94 and the P2P wire 75→76 in lockstep (lobby protocol unchanged: no lobby frame carries the shape; measured that `scripts/check-protocol-version.mjs` derives both from literal constants and the Phase 2 commit `b96d6ef2` touched the same file set). Edits: `crates/lobby-broker/src/protocol.rs` (`PROTOCOL_VERSION` and its history entry), `crates/server-core/src/protocol.rs`, `client/src/adapter/ws-adapter.ts`, `client/src/network/protocol.ts`, `client/src/network/__tests__/protocol.test.ts`, `client/src/adapter/__tests__/p2p-adapter-multiplayer.test.ts`, `scripts/check-protocol-version.mjs` (`EXPECTED_PROTOCOL_VERSION` +23 from upstream base, `EXPECTED_WIRE_PROTOCOL_VERSION` +22). Not in the charter's scope list: flagged to the orchestrator for scope-freeze (§8).

### 6.7 Variant Discoverability
`data/engine-inventory.json` consulted (present, dated just before HEAD); `ContinuousModification` variant names printed; no substitution variant; gate in §2.1.

## 7. Identity / Provenance Contract
- Phrase: "replacing all instances of one <word class> with another" on "target spell or permanent".
- Authority: the pair (from, to) the spell's controller selects from the NamedChoice prompt; type `TextSubstitution`; value fixed at answer time.
- Binding time: at resolution (CR 608.2d), latched when the `GenericEffect` installs its transient effect (`snapshot_transient_modifications`), after the prompt answer; the affected object is the target bound at install (CR 611.2c; `ParentTarget` → `SpecificObject`).
- Storage: `TransientContinuousEffect.modifications[SubstituteTextWord{Fixed}]` with the effect's `duration` and timestamp (CR 613.7b). Not stored on the spell object, which leaves the stack.
- Consumers: Layer-3 pre-pass (battlefield) and `restamp_resolving_spell_text` (stack spell). Both read the same transient list through one collector.
- Live vs snapshot: words snapshotted; the recipient set snapshotted; the text is re-derived from printed text every layer pass (battlefield), so expiry reverts with no undo code.
- Invalidation: `Duration::UntilEndOfTurn` by `prune_end_of_turn_effects`; `Permanent` by `prune_affected_object_left_effects` (recipient leaves) — existing machinery.
- Multi-authority hostile fixtures: two text changes on one permanent in both timestamp orders (§9 P4); two different permanents each hit by a separate substitution (no crosstalk, P6); the same words chosen against a spell vs a permanent (S1 vs P1).

## 8. Scope matrix (literal paths; `SCOPE_PATHS` candidates)
Authored (28) plus test module pair (2) = 30. Sites marked (arm) are compiler-forced exhaustive arms from `git grep -l 'SetTextName' -- crates` filtered to non-test; the parser sites in that list that only *construct* `SetTextName` (`parser/oracle_effect/subject.rs`, `parser/oracle_static/type_change.rs`, tests) need no edit and are not in scope.

1. `crates/engine/src/types/ability.rs` — types, variant, label methods, CR doc.
2. `crates/engine/src/types/ability_visit.rs` (arm).
3. `crates/engine/src/types/layers.rs` — `layer()` arm + layer test.
4. `crates/engine/src/game/layers.rs` — write-class arm, apply arm (documented no-op), Text pre-gather pass, gather-bucket filter, other forced arms.
5. `crates/engine/src/game/text_substitution.rs` (new).
6. `crates/engine/src/game/mod.rs` — `mod` line.
7. `crates/engine/src/game/effects/effect.rs` — latch arm.
8. `crates/engine/src/game/stack.rs` — `resolve_top` seam. **Not in charter scope rule; plan-discovered site (the stack-spell consumption seam); needs scope-freeze approval.**
9. `crates/engine/src/game/off_zone_characteristics.rs` (arm).
10. `crates/engine/src/game/quantity.rs` (arm; charter seam note).
11. `crates/engine/src/game/ability_rw.rs` (arm; classify in the read/write profiler, conservative).
12. `crates/engine/src/game/ability_scan.rs` (arm; classify per axis).
13. `crates/engine/src/game/coverage.rs` (description arm).
14. `crates/engine/src/game/effects/become_copy.rs` (arm).
15. `crates/engine/src/parser/oracle.rs` (3 arms).
16. `crates/engine/src/parser/oracle_effect/lower.rs` (2 arms).
17. `crates/engine/src/parser/oracle_static/shared.rs` (arm).
18. `crates/engine/src/parser/oracle_effect/mod.rs` — hook + `mod text_change;`.
19. `crates/engine/src/parser/oracle_effect/text_change.rs` (new) — recognizer + parser tests.
20. `crates/phase-ai/src/features/devotion.rs` (arm).
21. `crates/phase-ai/src/policies/x_reference.rs` (arm).
22. `crates/lobby-broker/src/protocol.rs` — protocol bump (not in charter; plan-discovered, required by repo rule).
23. `crates/server-core/src/protocol.rs` — same.
24. `client/src/adapter/ws-adapter.ts` — same.
25. `client/src/network/protocol.ts` — same.
26. `client/src/network/__tests__/protocol.test.ts` — same.
27. `client/src/adapter/__tests__/p2p-adapter-multiplayer.test.ts` — same.
28. `scripts/check-protocol-version.mjs` — same.
29. `crates/engine/tests/integration/text_substitution_cr612.rs` (new; card tests, census, parity).
30. `crates/engine/tests/integration/main.rs` — `mod text_substitution_cr612;`.
Excluded from counting: `crates/engine/tests/fixtures/integration_cards.json.gz` (regenerate with `python3 scripts/gen-test-fixture.py`, no `--check`, because new test cards enter the fixture and the parse of fixture-resident cards changes), `client/public/card-data.json` / coverage data.
Executor note: the `cargo check` exhaustive-arm list is authoritative; if it names a file not above, stop-and-return with evidence (engine-implementer stop rule). The phase-ai crate is a second workspace member that compiles against the engine enum, hence rows 20–21.

## 9. Verification Matrix
Rows use the `card-test` recipe (`GameScenario` + `GameRunner::cast(..).resolve()` or `runner.act(..)` for the word prompt, verbatim Oracle text via `add_real_card` from the regenerated fixture, asserts on `CastOutcome` and semantic accessors, every negative paired with a positive reach-guard). Every row must be shown red with the change reverted (revert-fails column) and green with it. Test-file structure: one integration module; parser SHAPE tests live in `text_change.rs` `#[cfg(test)]`.

| # | Claim | Seam / production entry | Runtime test (verbatim cards) | Revert-failing assertion | Negative siblings / hostile + paired positive guard | Coverage impact |
|---|---|---|---|---|---|---|
| P1 | Magical Hack changes a land-type word on a permanent indefinitely (CR 612.1, 613.1c) | `SubstituteTextWord` pre-pass → `obj.keywords` | Bog Wraith ("Swampwalk"); cast Magical Hack, answer `"Swamp -> Plains"`; assert `Landwalk("Plains")` present and `Landwalk("Swamp")` absent; advance through cleanup of the turn and into the next: still Plains (indefinite, CR 611.2a). Combat consequence: Bog Wraith cannot be blocked while defender controls a Plains and can while defender controls only a Swamp. | without the layer pass keywords stay Swamp | guard: the combat branch on an untouched Bog Wraith shows the Swamp-block legality (proves the evasion query reads the keyword) | Magical Hack → supported |
| P2 | CR 612.2 name carve-out | `rewrite` carrier table (`Token.name`/object name untouched) | Mountain Goat ("Mountainwalk"), Magical Hack `"Mountain -> Island"`: name still "Mountain Goat"; keyword `Landwalk("Island")` (positive) | table row flipped to Word → name assertion red | positive guard is the same card's keyword change in the same test | — |
| P3 | Type-line land word + derived mana (CR 305.6/305.7) | pre-pass subtype rewrite → `apply_intrinsic_basic_land_mana_abilities` | Forest on the battlefield, Magical Hack `"Forest -> Island"`; subtypes `["Island"]`, name "Forest"; tapping it yields {U} not {G}; with an Island already present subtypes contain Island once | subtype stays Forest | Swamp untouched (guard: a second basic keeps its own mana) | — |
| P4 | Timestamp order (CR 613.7b) | pre-pass order | Bog Wraith: Hack `"Swamp -> Plains"` then Hack `"Plains -> Forest"` ⇒ Forestwalk; fresh Bog Wraith, reverse casting order ⇒ Plainswalk | either ordering bug flips one of the two | both orders in one test; each order's end state is a positive assertion | — |
| P5 | Color word in a keyword (CR 612.2, 702.16) and symbols untouched | carrier rows `{"Color":c}` (Word) vs mana production (NotAWord) | Black Knight ("Protection from white"): Sleight of Mind `"White -> Blue"` ⇒ protection from blue; second Black Knight, Sleight `"Black -> Red"` ⇒ nothing changes and name "Black Knight" stays. Llanowar Elves, Sleight `"Green -> Blue"` ⇒ still taps for {G} | flipping the mana-production row to Word turns the Elves assertion red | the Knight "White -> Blue" case is the positive guard of the "Black -> Red" no-op | Sleight of Mind → supported |
| ST1 | A text-changed **static ability** applies in the same pass | Text pre-gather pass before Step 3 gather | Bad Moon + one black and one white creature; Sleight of Mind `"Black -> White"` on Bad Moon ⇒ the white creature gets +1/+1, the black one does not; after the effect's source is irrelevant (indefinite) still true next turn | move the pass after the gather ⇒ P/T unchanged ⇒ red | baseline: before casting, black creature has +1/+1 (guard: anthem live) | — |
| S1 | A text change on a **spell on the stack** is what resolves (CR 608.2b) | `restamp_resolving_spell_text` in `resolve_top` | P1 casts Terror on a red creature; P0 responds with Sleight of Mind `"Black -> Red"` targeting Terror: Terror now reads nonred ⇒ target illegal at resolution ⇒ creature survives. Second run: Acid Rain ("Destroy all Forests.") with Magical Hack `"Forest -> Island"` on it ⇒ an Island dies, a Forest lives | remove the seam ⇒ Terror still kills / Forest still dies | baselines without the Hack/Sleight (Terror kills the red creature; Acid Rain kills the Forest) in the same test prove the spells are live | Sleight/Hack spell-target half |
| S2 | Permanent target vs spell target both work through one latched effect | `Or[StackSpell, Typed Permanent]` target at cast (CR 601.2c) | both targets offered by the driver; cast onto a permanent (P1) and onto a spell (S1) | — | declining/illegal target: Hack with no legal target cannot be cast | — |
| C1 | Choice prompt round trip and domain | `Choose{Labeled}` + NamedChoice | Crystal Spray: prompt lists both color pairs and land pairs (40 options, no `X -> X`); answer a color pair on Bad Moon and, in a second test, a land pair on Mountain Goat; both resolve and **Draw a card.** resolves after (`assert_hand_drawn(P0, 1)`) | latch arm removed ⇒ `Chosen` stays inert ⇒ no change | hostile: answer `"Forest -> Island"` to Sleight of Mind's prompt ⇒ rejected (label outside domains); answering `"Black -> Black"` ⇒ rejected. Guard: prompt for Sleight lists exactly the 20 color pairs | Crystal Spray → supported |
| E1 | Until-end-of-turn expiry (CR 514.2) | `prune_end_of_turn_effects` + per-pass reset | Crystal Spray on Mountain Goat `"Mountain -> Forest"`; Forestwalk during the turn; after cleanup Mountainwalk again | duration set to Permanent ⇒ stays Forest ⇒ red | guard: Forestwalk present at end step before cleanup | — |
| L1 | Incremental flush equals full evaluation with a live substitution | `CharacteristicKinds::ALL` write class | parity test modelled on `assert_pt_identical`: board with a live substitution, compare the incremental-flush result with a forced `LayersDirty::Full` result | write class `EMPTY` ⇒ divergence | guard: board without the effect is identical under both (control) | — |
| PR1 | Parser SHAPE: the four text forms | `try_parse_text_change_clause` | Magical Hack text ⇒ `Choose(Labeled 20 land pairs) → GenericEffect{Permanent, Or[StackSpell,Permanent], Chosen{[BasicLandType]}}`; Crystal Spray ⇒ duration EOT, domains `[ColorWord, BasicLandType]`, chain tail Draw; Mind Bend ⇒ target `Typed Permanent` only; Sleight ⇒ `[ColorWord]` | recognizer removed ⇒ `Unimplemented` | fail-closed: Artificial Evolution, New Blood, Magical Hacker, Deceptive Divination still contain `Effect::Unimplemented`; guard: Sleight of Mind in the same test parses with zero Unimplemented | all IN-scope cards |
| PR2 | Modal and keyword-bearing class members parse with no Unimplemented | modal parser + recognizer | Spectral Shift (2 modes, Entwine kept), Alter Reality (Flashback kept), Glamerdye (Retrace), Trait Doctoring (Cipher), Whim of Volrath (Buyback) | recognizer removed ⇒ Unimplemented | each card's keyword still parsed (guard) | → supported |
| CEN1 | Carrier census: every color/land-type string position in the corpus is classified | `WORD_CARRIERS` | test over `shared_card_db()`; run once under `FORGE_TEST_FULL_DB=1` by the executor and report the count of distinct positions | delete a table row ⇒ red | positive guard: the census visits a nonzero number of `HasColor` and `Landwalk` occurrences | — |
| CEN2 | Serde round-trip identity of rewritten types over the corpus | `rewrite` | same corpus: `from_value(to_value(x)) == x` for abilities, triggers, statics, replacements, keywords | — | guard: nonzero faces checked | — |
| COV | Coverage flips | `cargo coverage` + brief jq (`names.txt` style filter over `client/public/coverage-data.json`) | the IN-scope cards report supported; OUT cards remain unsupported | — | — | verified at the end, reported with the jq output |

Whether any Oracle text is accepted while semantics stay deferred: **no** — the recognizer returns a full typed chain or `None`; there is no accept-and-stub path. Coverage honesty: the out-of-domain phrases fall to the existing `Unimplemented` tags, pinned by PR1's fail-closed rows.
Whole-card effect of Crystal Spray's two sentences is checked by C1 (draw after the change).

## 10. Reference Readings
None. No row copies a sibling route's result or asserts parity/preservation; each expected value is derived from the card's verbatim text and the CR above (P1 from Magical Hack's example "swampwalk → plainswalk" and 612.1; S1 from 608.2b's explicit mention of spell text changes; ST1 from Bad Moon's verbatim line; E1 from Crystal Spray's "until end of turn"). The only same-model comparison, L1, is between the incremental and full layer paths and its expected value is derived independently (the board after a forced full evaluation, which P1/ST1 already derive from Oracle text).

## 11. Step-by-step implementation plan (checklist order)

1. **Verify CR numbers** used in annotations by `grep -n "^612.2\|^612.3\|^612.4\|^613.1c\|^613.7\|^613.7b\|^608.2b\|^608.2d\|^611.2a\|^611.2c\|^305.6\|^305.7\|^105.1\|^702.14a\|^702.16\|^400.7a\|^205.3i" docs/MagicCompRules.txt` (all found while planning).
2. `types/ability.rs`: add `TextWordDomain`, `TextSubstitution` (+ `options`, `label`, `from_label`, constructor rejecting `from == to`), `TextSubstitutionSpec`, and `ContinuousModification::SubstituteTextWord`. Serde: internally tagged like neighbours; `Chosen` serializes with its `domains`. Doc comment cites CR 612.1, 612.2, 608.2d. No `RUNTIME: TODO` (real handler lands in this phase).
3. `types/layers.rs`: `SubstituteTextWord => Layer::Text`; extend the Layer-3 test list beside `SetChosenName`.
4. `game/text_substitution.rs` + `game/mod.rs`: carrier table, `rewrite`, `apply_to_permanent_text`, stack collector, `restamp_resolving_spell_text`; unit tests for rewrite in the same file (table-level: one per carrier class plus name carve-out).
5. `game/layers.rs`: write-class arm (`ALL`); no-op apply arm; Text pre-gather pass and bucket filter; forced-arm additions elsewhere in the file. Probe P-A first (below).
6. `game/effects/effect.rs`: latch arm in `snapshot_transient_modifications`.
7. `game/stack.rs`: seam in `resolve_top`, `is_spell` only, ahead of `bind_resolving_ability_referents`.
8. Remaining forced arms (`ability_visit.rs`, `off_zone_characteristics.rs`, `quantity.rs`, `ability_rw.rs`, `ability_scan.rs`, `coverage.rs`, `become_copy.rs`, `parser/oracle.rs`, `lower.rs`, `oracle_static/shared.rs`, `phase-ai` ×2): leaf/conservative classification per the `add-engine-variant` post-gate text (ability_scan and ability_rw classifications: the modification reads no game state at resolution and writes the recipient's own text: classify as a self-object write, `reads_member_bound` false, `reads_event_live` false, `writes_event_object` false; if the compiler forces more precision, take the conservative form).
9. Parser: `text_change.rs` recognizer, hook in `parse_effect_clause`, tests PR1/PR2.
10. Protocol bump files (§6.6) and run `node scripts/check-protocol-version.mjs`.
11. Card tests in `tests/integration/text_substitution_cr612.rs` + `mod` line; regenerate `integration_cards.json.gz`.
12. Verification: before any cargo command `until ! pgrep -x cargo >/dev/null && ! pgrep -x rustc >/dev/null; do sleep 20; done`, prefix `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0`; `cargo fmt --all`; `cargo check -p phase-engine -p phase-ai`; `cargo nextest run -p phase-engine text_substitution` and the layers/stack/parser suites touched; census under `FORGE_TEST_FULL_DB=1`; `cargo coverage` + jq.

### Probe gates for the executor (stop-and-return if falsified, with output)
- **P-A (layers):** a throwaway test confirming the Text pre-gather pass lands before `gather_active_continuous_effects` and that an anthem's static is gathered from the rewritten definitions (ST1's Bad Moon assertion is the probe). If `apply_continuous_effect` cannot be reused for the pre-pass, apply via `text_substitution::apply_to_permanent_text` with the recipient set from `effect_candidate_ids`.
- **P-B (chain assembly):** Crystal Spray lowers to `Choose → GenericEffect → Draw` with the target slot assigned at cast and the draw after the change (C1).
- **P-C (target phrase):** `parse_target("target spell or permanent by replacing …")` yields `Or[StackSpell, Typed Permanent]` with the right remainder (Chaoslace's card-data shows the filter for a different tail).
- **P-D (round trip):** CEN2.
- **P-E (stack seam order):** the rewrite precedes the CR 608.2b legality recheck (S1's Terror assertion).

## 12. Class coverage summary
Covered by this phase: Magical Hack, Crystal Spray, Sleight of Mind, Alter Reality, Glamerdye, Mind Bend, Spectral Shift, Trait Doctoring, Whim of Volrath; Balduvian Shaman conditionally. Remain `Effect::unimplemented`: Artificial Evolution and New Blood (creature-type domain, §1.3), Magical Hacker (symbol swap), Deceptive Divination (card-type word, plane), Exchange of Words (CR 612.5, other primitive). March of Progress is already supported (Overload).

## 13. Claims ledger
MEASURED (commands run while planning): `Layer::Text` exists and `SetTextName`/`SetChosenName` map to it (`types/layers.rs`); no substitution variant (inventory, grep); card texts and class list (jq, Scryfall for two); Chaoslace target/duration shape; carrier census (python over card-data); `Bad Moon`, `Terror`, `Acid Rain`, `Black Knight`, `Mountain Goat`, `Bog Wraith`, `Llanowar Elves` exist with the texts quoted; layer structure (single gather after layer 1, stack objects not reset, intrinsic basic mana derived after Type); `snapshot_transient_modifications` as the latch seam; no `serde(skip)` in `types/ability.rs`; exhaustive-arm file list (`git grep -l SetTextName`); protocol file set (`b96d6ef2` stat).
UNESTABLISHED (no compile or run was possible): P-A..P-E above; that the serialize→walk→deserialize round trip is identity for every corpus shape (CEN2 is the gate); that Balduvian Shaman's target phrase and cumulative-upkeep grant parse with existing blocks; that `ability_scan.rs`/`ability_rw.rs` accept the conservative classification without further precision; that incremental flush parity holds (L1); text-change carryover from spell to permanent (CR 400.7a) — not addressed and not claimed.
