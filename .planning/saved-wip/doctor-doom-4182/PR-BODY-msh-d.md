:robot: _AI text below_ :robot:

## Building blocks: `Plan` card type + elided-verb disjunctive control condition

**Doctor Doom** was resolver-flagged (`gap_count=0` but `supported=false`): every ability parsed, but its static "As long as you control an artifact creature or a Plan, Doctor Doom has indestructible" lowered to `StaticCondition::Unrecognized`. The layer system evaluates an unrecognized condition as `true`, so the indestructible grant was **always-on** instead of gated on board state. Two root causes, both fixed.

### 1. New `CoreType::Plan` (+ `TypeFilter::Plan`)

"Plan" is a card type from Marvel's Spider-Man. It was absent from `CoreType`, so `CoreType::from_str("Plan")` errored and the card-data importer (`from_str().ok()` + `filter_map`) **silently dropped** it. Added `CoreType::Plan` modeled structurally on `CoreType::Plane` — a nontraditional, **non-permanent** card type (excluded from `is_permanent_type` / `PERMANENT_TYPES`), with `TypeFilter::Plan` mirroring `TypeFilter::Battle` for runtime matching.

CR note: "Plan" is **not** in the CR snapshot (CR 205.2a, dated 2026-04-17, lists artifact..vanguard — Marvel's Spider-Man postdates it), so the variant carries a `needs-manual-verification` annotation rather than a fabricated CR number. All exhaustive matches were updated by a compiler-driven sweep (no wildcards), including explicit `CoreType ↔ TypeFilter` Plan bridge arms in `conditions.rs` so Plan no longer falls through to `Subtype("Plan")`.

### 2. Elided-verb disjunctive control condition

"you control `<type A>` or `<type B>`" shares one "you control" verb across two type filters. The second filter ("a Plan") is not a standalone condition, so the top-level condition disjunction can't split it, and `parse_type_phrase` only consumed the first type — leaving the whole condition `Unrecognized`. `parse_you_control_a` now folds an article-led `" or <article> <type>"` continuation into a disjunction:

```
IsPresent { Or[ Typed{[Artifact,Creature], You, Battlefield},
                Typed{[Plan],              You, Battlefield} ] }
```

via the existing `inject_controller` `Or`-distribution. This is scoped to the **control-condition layer** (not `parse_type_phrase`, which would have regressed conjunctive "and" and recipient "it's X or Y" shapes — measured: that approach fails 4 existing tests), and covers the whole "you control A or B" class. **No new `StaticCondition` variant** — the runtime `IsPresent`/`Or` evaluator already gates the grant (CR 611.3a, continuous re-evaluation), so no runtime evaluator change was needed.

### CR references (grep-verified)

- **CR 109.5** — "you"/"your" = controller.
- **CR 205.2a** — card types list (confirms "Plan" is absent → `needs-manual-verification`).
- **CR 604.1** — handling static abilities.
- **CR 608.2c** — read the whole text / disjunction.
- **CR 611.3a** — a static ability's continuous effect isn't locked in; re-evaluated each moment (why the conditional grant toggles).
- **CR 702.12b** — indestructible.

### Tests (non-vacuous + discriminating; reviewer-run)

- Runtime toggle (Anger pattern): Doctor Doom HAS indestructible while you control an artifact creature; does NOT when you control neither an artifact creature nor a Plan (the **revert-probe** — pre-fix `Unrecognized => true` made the grant always-on, so this arm fails before the fix); does NOT when only an opponent controls an artifact creature (proves `ControllerRef::You` scope).
- Condition parse asserts typed `IsPresent{Or[...]}`, not `Unrecognized`; single-type "you control an artifact creature" still parses (regression).
- `CoreType::Plan`: `from_str`/`Display`/serde round-trip; non-permanent classification matches `Plane`; `protection_quality_str == None`.
- `TypeFilter::Plan` runtime match (Plan object matches, creature does not); the `CoreType ↔ TypeFilter` Plan bridge round-trips (not `Subtype`).

Full engine test suite green (13346 passed); clippy `-D warnings` clean (engine + draft-core); parser-combinator gate exit 0; zero non-exhaustive-match errors. `mtgish/` untouched. New serde variants round-trip-tested; 0 Plan cards in the current corpus so no migration.
