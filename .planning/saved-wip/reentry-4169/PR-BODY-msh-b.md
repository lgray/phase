:robot: _AI text below_ :robot:

## Building block: entering-object source-relative P/T intervening-if

This PR fixes **Hulkling, Burgeoning Bruiser** — "Vigilance / Whenever another creature you control enters, **if it has greater power or toughness than Hulkling**, put a +1/+1 counter on Hulkling." The ETB intervening-if comparing the entering creature's power/toughness to the source was being dropped (the trigger `condition` was `null`; `supported: false`, `gap_count: 1`).

**No new engine variant.** The condition is expressed entirely with existing building blocks (the `add-engine-variant` gate returned EXISTS_AS_PARAMETER):

- `TriggerCondition::ZoneChangeObjectMatchesFilter { origin, destination, filter }` — the existing entering-object intervening-if condition (its subject is the zone-change event object, i.e. the entering creature, not the source permanent).
- `FilterProp::PtComparison { stat, scope, comparator, value }` — its `value` is already a `QuantityExpr`, so a *source-relative* threshold is expressible.
- `QuantityRef::Power { scope }` / `Toughness { scope }` with `ObjectScope::Source` — resolves to the ability source's (Hulkling's) corresponding stat.
- `FilterProp::AnyOf` — composes the "power **or** toughness" disjunction.

So "greater power or toughness than ~" lowers to:

```
ZoneChangeObjectMatchesFilter {
  origin: None,
  destination: Battlefield,
  filter: Typed(creature, [AnyOf([
    PtComparison { stat: Power,     scope: Current, comparator: GT, value: Ref(Power{scope: Source}) },
    PtComparison { stat: Toughness, scope: Current, comparator: GT, value: Ref(Toughness{scope: Source}) },
  ])]),
}
```

The runtime already evaluates this end-to-end: `ZoneChangeObjectMatchesFilter` matches the filter against the entering object with the `FilterContext` source = the ability source, so `PtComparison`'s threshold resolves to the source's stat. No new runtime handler.

### What changed

- New nom combinator `parse_entering_pt_vs_source_condition` in `oracle_trigger.rs`, recognizing "if it has greater power [or toughness] than ~" and the single-stat forms ("greater power than ~" / "greater toughness than ~" → one `PtComparison`, no `AnyOf`). Wired as the **first** arm of `parse_zone_change_object_filter_condition`, before the `"if it "`-predicate fallback (which hardcodes `destination: Graveyard` — the ETB case needs `Battlefield`). Purely additive: the fallback predicate rejects "has greater …", so nothing is shadowed.
- Does **not** touch `FilterProp::PowerGTSource` (the CR 509.1b combat blocking-restriction leaf) or `nom_filter::parse_pt_comparison` (the fixed-threshold grammar).

### Class this generalizes

The combinator handles the *category* of ETB intervening-ifs comparing the entering creature's power and/or toughness to the source's same stat — Hulkling is the driver, and the single-stat arms cover any future "Whenever a creature enters, if it has greater power/toughness than ~" payoff with no new code.

### CR references (verified against the Comprehensive Rules)

- **CR 603.4** — intervening-"if" clause (checked at trigger and rechecked on resolution).
- **CR 603.6a** — ETB triggers; the subject "it" is the entering newcomer.
- **CR 113.7** — the source of an ability ("~"), bound via `ObjectScope::Source`.
- **CR 208.1** — power/toughness.
- (CR 603.10a's look-back list does **not** include ETB, so `PtValueScope::Current` reads the live post-layer P/T — correct, not LKI.)

### Tests (non-vacuous + discriminating)

- Card-synthesis: full Hulkling text → the `AnyOf` of two source-relative `PtComparison`s; `PutCounter P1P1` on self intact. Fail-before: condition was `null`.
- Single-stat discriminator: "greater power than ~" → one `PtComparison`, not `AnyOf`.
- Regression: an existing fixed-threshold P/T parse stays a fixed-value `PtComparison` (the new source-relative arm doesn't shadow it).
- Runtime gating: entering 3/3 vs Hulkling 2/2 → true; 2/2 → false (strict GT, not GE); 1/3 vs 2/2 → true (toughness-only OR — proves the threshold resolves to Hulkling, not the entering object); 1/2 vs 2/2 → false.

No new serialized surface (reuses existing `TriggerCondition`/`FilterProp`/`QuantityRef`). `mtgish/` untouched.
