# MSH-F plan review — round 2 (mshf-plan-reviewer-r2)

VERDICT A (Cosmic Cube): CLEAN. VERDICT B (Hawkeye): CLEAN.
All round-1 findings RESOLVED (verified against actual code in wt-msh-f). 4 NEW findings, all LOW/cosmetic — to be handled by the executor in passing, no further plan-review round needed.

## Round-1 findings — all RESOLVED
- [HIGH A] aggregate delegation: `parse_object_property_aggregate_ref` (oracle_nom/quantity.rs:959) genuinely returns the remainder. Traced full input → `Ok((" without paying its mana cost", Aggregate{Max,Power, attacking creatures you control}))`. "without paying" NOT mis-consumed (`parse_without_keyword_suffix` requires a keyword; "paying" isn't one). `parse_cast_snapshot_suffix` Errs → falls back to remainder. Routing via top-level `parse_quantity_ref`@560 reaches aggregate arm@564. `nom_quantity` imported mod.rs:73.
- [MED A] prefix-comparator: Comparator GT/LT/GE/LE/EQ/NE@~5219 all exist; Beseech is post-value suffix, Cosmic Cube pre-value prefix — two-sub-combinator split sound, Beseech untouched.
- [MED A] finalize-seam: `cast_permission_constraint_allows_cast`@1577 dynamic branch 1592-1598 resolves via `resolve_quantity(state,value,obj.controller,obj.id)`, permissive when resulting_mv None; offer@1670 None; finalize@1695/1763 re-checks Some; rejection casting_costs.rs:5350-5377 → `Err(ActionNotAllowed(...))`. Row A2 correct.
- [LOW A] anchors: FilterProp::Attacking@2591, ObjectProperty::Power@4606, ManaValue@2185 (value already QuantityExpr), Aggregate@4132; LingeringPermission path cast_from_zone.rs:491→556/568; CR 601.2e@2466/601.2f@2468 grep-clean.
- [MED B] non-goal + grep: QuantityModification@15810 is a separate enum from DamageModification@15775; `grep -rln DamageModification::Plus` = exactly 4 files; QuantityModification::Plus disjoint 12-file set. Categorical rationale sound. (BUT see NEW-1 CR mis-cite.)
- [LOW B] foot-guns: parse_cda_quantity strips trailing '.' (oracle_quantity.rs:667→ctx, `trim_end_matches('.')`) + returns Option→map_opt; new arm must precede `value(Plus{0}, tag("plus x"))`; 4 regression assertions @11783/11810/14026/14036 exist as u32, migrate to Fixed{}.

## Fresh adversarial pass — all clear
- `map_opt(preceded(tag(...), rest), …)` safe: `scan_at_word_boundaries`@primitives.rs:856 discards remainder on first success.
- Resolver borrow compiles: `damage_modification_for_rid`@873 returns owned clone (886-891); SetToSourcePower arm@956-968 proves the immutable-read-in-match pattern.
- Serde load-test real: `#[serde(tag="type")]`; bare `2`→QuantityExpr custom Deserialize@5013-5030→Fixed{2}; QuantityExpr derives Eq@4898.
- No first-scan interception of Hawkeye (`parse_damage_modification_phrase`@4956-4975 only matches double/triple/equal-to forms).
- No remaining new-variant need.

## NEW findings (all LOW — executor handles in passing)
- **NEW-1 [LOW]** CR mis-citation in Sub-Plan B non-goal PROSE (plan lines 24/134): says "CR 121 counter-placement"; counters are **CR 122.1@1178** (CR 121.1@1142 = drawing a card). CR 111.1@645 tokens ✓. Categorical conclusion unchanged; NO code annotation inherits this (resolver/parser annots 614.1a/120/107.1b/107.3a all correct). Fix prose to CR 122 so it can't leak into a comment.
- **NEW-2 [LOW]** resolver pseudocode: `resolve_quantity(state, value, …)` should be `&value` (takes `&QuantityExpr`, quantity.rs:68). Compiler-caught.
- **NEW-3 [LOW]** Sub-Plan A step-1 pseudocode "`take_until` for the first of `alt((...))`" isn't literally nom-valid (take_until takes one literal). Achieve via many_till/peek or repo scan helpers; Cosmic Cube only needs "with mana value ". Implementation note.
- **NEW-4 [LOW]** `rest` greediness: new arm's `rest` consumes to end-of-fragment; a hypothetical multi-clause trailing fragment could make parse_cda_quantity over-consume → None → freeze arm wins (silent Plus{0}). Hawkeye is single-clause so unaffected; guard only if a sibling surfaces.
