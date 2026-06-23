# MSH-B: Hulkling intervening-if (entering-creature P/T vs source) — investigation log

Worktree: /private/tmp/wt-msh-intervening-if, branch card/msh-intervening-if, off origin/main 0875f389d.
Tilt DOWN → direct cargo with nightly proxy (`export PATH="$HOME/.cargo/bin:$PATH"`).
card-data.json/coverage-data.json gitignored; query from MAIN checkout /Users/lgray/vibe-coding/phase-rs-workdir/phase/data/.

## The gap (verified via coverage parse_details + card-data)
Hulkling, Burgeoning Bruiser: "Vigilance / Whenever another creature you control enters, if it has
greater power or toughness than Hulkling, put a +1/+1 counter on Hulkling."
- supported=false, gap_count=1.
- The trigger + effect parse FINE: ChangesZone (another creature you control → Battlefield) →
  PutCounter P1P1 on self. The keyword Vigilance parses.
- The ONLY gap: trigger `condition` = null. The intervening-if "if it has greater power or toughness
  than ~" is DROPPED. ("it" = the entering creature = the ETB event object; "Hulkling"/~ = the source.)

## Binding context (CR 603.4)
This is a trigger intervening-if whose subject is the ENTERING object, not the source permanent. The
existing TriggerCondition for exactly this is:
- `TriggerCondition::ZoneChangeObjectMatchesFilter { origin, destination, filter }`
  (types/ability.rs, doc: "Intervening-if condition whose subject is the object from the triggering
  zone-change event rather than the permanent that owns the ability. ETB conditions check the live
  object in its destination zone." CR 603.4 + 603.6 + 603.10). This is the RIGHT TriggerCondition —
  evaluate a TargetFilter against the entering creature.

So the condition shape is:
  ZoneChangeObjectMatchesFilter { destination: Some(Battlefield), filter: <entering vs source P/T> }
where the filter expresses "(power > source.power) OR (toughness > source.toughness)".

## The architectural crux — source-relative P/T comparison (add-engine-variant gate)
"greater power than {source}" is a SOURCE-RELATIVE comparison (compare the matched object's stat to
the SOURCE object's same stat). The existing surface:
- `FilterProp::PowerGTSource` (UNIT variant) — filter.rs:3383: `obj.power > source.power`. Hard-coded
  to (stat=power, comparator=GT, ref=source). CR 509.1b (blocking restriction "creatures with greater
  power"). EXISTS for the POWER half.
- NO toughness twin. `ToughnessGTPower` is creature-self (toughness > its OWN power), not vs source.
- `FilterProp::PtComparison { stat, scope, comparator, value }` — compares against a FIXED i32
  `value`, NOT source-relative. Cannot express "> source.power". (Its doc explicitly notes it
  REPLACED the PowerLE/GE/ToughnessLE/GE sibling cluster with a parameterized form — the precedent for
  parameterizing P/T comparisons.)

SIBLING-CLUSTER SMELL: `PowerGTSource` is a source-relative P/T comparison hard-coded to
(power, GT). Adding `ToughnessGTSource` as a second unit sibling would start the same cross-product
the `PtComparison` refactor already eliminated for the fixed-value case. The gate should decide
between:
  (a) Parameterize the source-relative comparison: a new prop like
      `PtComparisonToSource { stat: PtStat, scope: PtValueScope, comparator: Comparator }`
      (compares obj's stat to SOURCE's same stat). PowerGTSource becomes
      PtComparisonToSource{ Power, Current, GT }. This mirrors PtComparison's axes minus the literal
      `value` (the value IS the source's stat). Covers power|toughness|total × any comparator. Hulkling =
      AnyOf([PtComparisonToSource{Power,Current,GT}, PtComparisonToSource{Toughness,Current,GT}]).
  (b) Add a single `ToughnessGTSource` unit sibling (cheap now, compounds later) — gate likely REJECTS
      this as the smell.
  Categorical-boundary: this is all CR 208 (power/toughness) within FilterProp — single rule section,
  so parameterization is allowed at FilterProp. NOT crossing into life (CR 119) etc.
RUN THE GATE to decide (a) vs (b) and whether refactoring PowerGTSource's call sites (blocking
restriction at filter.rs:3383 + the combat path) is in-scope. Prefer (a) if the refactor is bounded —
PowerGTSource has limited call sites (filter.rs:180/385/2670/3383/3810).

## Parser side
"if it has greater power or toughness than ~" must be recognized by parse_inner_condition (the single
condition authority) and produce the ZoneChangeObjectMatchesFilter (or, if parse_inner_condition only
emits StaticCondition, a StaticCondition that bridges via static_condition_to_trigger_condition's
ZoneChangeObjectMatchesFilter arm — check how existing "if it has/is <X>" entering-object conditions
bridge). The "it" here is the entering creature (event object), distinct from the source ~. Must
delegate to parse_inner_condition per CLAUDE.md; do NOT bespoke-match the string. Check whether an
existing "greater power than ~" / "power greater than ~" combinator exists for the blocking-restriction
PowerGTSource and reuse/extend it for the trigger condition + add the toughness/OR dimension.

## Verification plan (non-vacuous, discriminating)
- parse_inner_condition (or the trigger condition extractor) on "it has greater power or toughness than
  ~" → ZoneChangeObjectMatchesFilter{ filter: AnyOf([<power vs source GT>, <toughness vs source GT>]) }.
  Fail-before: condition null.
- Discriminating: "greater power than ~" (power only) → just the power prop, not the OR; a non-source
  fixed comparison ("power 4 or greater") must still → PtComparison (not the source-relative prop).
- Runtime: ZoneChangeObjectMatchesFilter gates correctly — entering 3/3 vs Hulkling 2/2 (greater power)
  → counter; entering 1/1 → no counter; entering 1/3 vs 2/2 (greater toughness only) → counter (proves
  OR + toughness half). Use game/triggers.rs test patterns.
- Card-level: Hulkling trigger condition = Some(ZoneChangeObjectMatchesFilter{...AnyOf...}); effect +
  PutCounter unchanged; gap_count 1→0.

## add-engine-variant GATE VERDICT: EXISTS_AS_PARAMETER — NO NEW VARIANT
Stage 1 = EXISTS_AS_PARAMETER. The source-relative P/T comparison is fully expressible with existing types:
- PtComparison { stat, scope, comparator, value } already takes `value: QuantityExpr` (NOT a fixed i32 — types/ability.rs:2446-2452).
- QuantityRef::Power { scope: ObjectScope } and QuantityRef::Toughness { scope } exist; ObjectScope::Source = "the source object of the resolving ability (~/this creature)" (engine-inventory). So:
    "entering creature has greater power than ~"  = PtComparison{ Power,     Current, GT, Ref(Power{Source}) }
    "...greater toughness than ~"                 = PtComparison{ Toughness, Current, GT, Ref(Toughness{Source}) }
  Hulkling OR-form = AnyOf([those two]).
- RUNTIME VERIFIED correct end-to-end (no new handler needed):
    triggers.rs:4851 ZoneChangeObjectMatchesFilter → matches_zone_change_event_object_filter(..., filter, FilterContext::from_source(state, source_id)).
    filter.rs:matches_zone_change_event_object_filter → for destination=Battlefield: matches_target_filter(state, ENTERING_object_id, filter, ctx) where ctx.source = ABILITY source (Hulkling).
    filter.rs:3367 PtComparison → comparator.evaluate(object_pt_value(obj=entering, stat, scope), resolve_filter_threshold(state, value, source=Hulkling)).
  ⇒ comparator.evaluate(entering.power, hulkling.power) with GT. Exactly "entering has greater power than Hulkling".
- DO NOT add PtComparisonToSource and DO NOT refactor PowerGTSource (out of scope — it's a separate
  combat blocking-restriction leaf at oracle_target.rs:3484 / oracle_nom/filter.rs:182; leave it).
  This keeps the change parser-only with zero engine-type changes.

## SCOPE (parser-only, no engine variant)
1. Add a combinator recognizing "it has greater power or toughness than ~" (and the single-stat
   "greater power than ~" / "greater toughness than ~") in the entering-object intervening-if dispatch
   (parse_zone_change_object_filter_condition, oracle_trigger.rs:3439, alongside
   parse_zone_change_object_token_predicate). Produce ZoneChangeObjectMatchesFilter{ destination:
   Battlefield, filter: Typed(creature, [AnyOf([PtComparison{Power..GT..Ref(Power{Source})},
   PtComparison{Toughness..GT..Ref(Toughness{Source})}])]) }. The OR is the existing FilterProp::AnyOf
   (mirror parse_combat_alone_props at oracle_trigger.rs:3501). Single-stat forms emit one PtComparison.
   "it" = entering event object (the ZoneChange subject); "~" = source (resolved via ObjectScope::Source).
2. Delegate as much as possible to parse_inner_condition / the shared P/T-comparison combinator. Check
   whether parse_inner_condition can already emit a PtComparison with Ref(Power{Source}) for "greater
   power than ~" — if a shared combinator exists (nom_filter::parse_pt_comparison handles disjunctive
   "power or toughness"), reuse it and only supply the source-relative threshold + the entering-object
   binding. Verify the exact reuse boundary in the plan; do NOT bespoke-match the verbatim string.
Verify ObjectScope variant name (Source) + QuantityRef::Power/Toughness field name (scope) at impl.
