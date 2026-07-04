# S25 P3 Wave 1 #2 — Sandman, Shifting Scoundrel — PLAN (3-stage, leaf recognizer)

Planner a50c089b57a5c64d1 (opus/xhigh). Base HEAD 712f93144. **CADENCE: 3-stage (leaf recognizer)** — new tightly-gated parser recognizer reusing proven infra (Coastal Wizard two-chained idiom + Zombify gy-target ChangeZone + Bloodsoaked activation_zone). NO new variant, NO shared-core/frozen edit, NO change to parse_target/resolved_targets/try_split_targeted_compound. Blast-radius coverage-regression MANDATORY (recognizer sits in a shared dispatch fn).

## Card + gap
"{3}{G}{G}: Return this card and target land card from your graveyard to the battlefield tapped." (static P/T=lands + can't-be-blocked-by-power≤2 already parse.)

## ROOT CAUSE (measured — card is INERT today, 3 coupled failures)
Routes through `try_split_targeted_compound` (mod.rs:12690). " and " = shared-destination compound, breaks 3 ways:
1. parse_target stops at "~" (oracle_target.rs:774-786) → `add_inferred_origin_constraints_to_target(SelfRef,Graveyard)` (mod.rs:12544/def 25591) wraps SelfRef into And[SelfRef, Typed{[],InZone GY,Owned You}]. `resolved_targets` (targeting.rs:630) has NO And arm → PRIMARY resolves empty → Sandman never moves.
2. "to the battlefield tapped" stripped by strip_return_destination → 2nd conjunct "target land card from your graveyard" is verbless. Verb carry-forward (mod.rs:12785) calls extract_effect_verb(primary) (mod.rs:25879) which does NOT map ChangeZone{Battlefield,Graveyard}→verb → None → sub stays Unimplemented → land never moves.
3. activation_zone_from_self_effect (oracle.rs:4408) only stamps activation_zone=Graveyard for a BARE ChangeZone{SelfRef}. The And[SelfRef,gy] primary isn't recognized → activation_zone=null → candidates.rs:3174 gate (requires activation_zone==Some(Graveyard)) → ability NOT offered from gy. Card fully inert.

Precedents (confirmed): Zombify → ChangeZone{Typed[Creature],You,[InZone GY]} (atomic "target X from your graveyard" parses). Coastal Wizard/Lady Sun "Return this creature and another target creature to their owners' hands" → primary Bounce{SelfRef} + sub Bounce{Typed[Creature,Another]} (two-chained idiom). Bloodsoaked Champion → ChangeZone{SelfRef,Graveyard} gets activation_zone=Graveyard.

## CLASS (exactly 2, build for both)
- Sandman: "target land card" (mandatory single, tapped).
- Slimefoot and Squee: "up to one other target creature card" (up-to-one, optional, "other", NOT tapped).

## DESIGN (two chained ChangeZone effects; leaf recognizer BEFORE the generic splitter)
New fn `try_parse_reanimate_self_and_target(text, ctx)` in oracle_effect/mod.rs (near the compound splitters ~:13787; NOT frozen). Call in lower_imperative_clause IMMEDIATELY BEFORE try_split_targeted_compound (mod.rs:11972). Nom combinators from line 1:
1. `tag("return ")` prefix.
2. parse_target first conjunct; require == TargetFilter::SelfRef. Then `tag(" and ")`.
3. REST after " and " = "target land card from your graveyard to the battlefield tapped". sub_text = "return " + REST → parse_imperative_effect(sub_text, ctx) — reuses full return-to-BF path: parse_target→Typed{[Land],You,[InZone GY]}, strip_return_destination→dest=Battlefield+enter_tapped, infer_origin→Graveyard. Slimefoot "up to one other" → multi_target=up_to(1)+Another.
4. GATE: only fire if sub.effect == ChangeZone{ destination:Battlefield, origin:Some(z!=Battlefield) }. Else None (fall through). Self-validating.
5. PRIMARY = clone sub's ChangeZone with target=TargetFilter::SelfRef (BARE, not And), up_to=false → Bloodsoaked shape → activation_zone auto-stamps Graveyard. Both enter tapped/from-GY/to-BF (primary copies sub's origin/dest/enter_tapped/counters/enters_under).
6. Return ParsedEffectClause{ effect: primary ChangeZone{SelfRef}, sub_ability: Some(AbilityDefinition wrapping sub), … }. **Set sub_ability.multi_target = sub_clause.multi_target AND .optional** so Slimefoot's up-to-one survives (the existing try_split wrapper at mod.rs:13764 DROPS multi_target — do NOT copy that omission).

TargetFilter "target land card from your graveyard" = `Typed{ type_filters:[Land], controller:You, properties:[InZone{Graveyard}] }` (Zombify form). Real chosen target (CR 601.2c/115.1) at activation on the sub; SelfRef primary untargeted.

Rejected: And-single-ChangeZone (needs shared-core resolved_targets And arm = blast radius on every And ChangeZone). Fixing try_split_targeted_compound (extract_effect_verb ChangeZone→"return" + re-append destination = shared carry-forward, perturbs every "return/exile X and target Y") → would push to escalation. Leaf recognizer before the splitter = ZERO engine change, safest.

## FILES
ONLY `crates/engine/src/parser/oracle_effect/mod.rs` (new fn + one call :11972). Relied-on-UNMODIFIED: oracle_target.rs, oracle.rs:4408, targeting.rs:630, candidates.rs:3174, imperative.rs return-lowering. Frozen effects/mod.rs/filter.rs/delayed_trigger.rs untouched. Tests: oracle_effect/tests.rs + runtime test.

## CR (grep-verified)
608.2c (follow instructions in order — two chained moves); 601.2c + 115.1 (land is chosen target at stack placement; SelfRef not a target); 602.2 (activating from graveyard); 400.7 (SelfRef = source incarnation only).

## TESTS
Parser-shape (verbatim, tests.rs): Sandman → primary ChangeZone{Graveyard→Battlefield, SelfRef, tapped} + sub ChangeZone{Graveyard→Battlefield, Typed{[Land],You,[InZone GY]}, tapped}; 0 Unimplemented; full-card activation_zone==Some(Graveyard). Slimefoot → primary ChangeZone{SelfRef} + sub ChangeZone{Typed[Creature]+Another,[InZone GY]}, sub multi_target==up_to(1), NOT tapped; 0 Unimplemented (positive reach-guard).
Runtime (card-test, ACTIVATE not parse-shape): Sandman + Forest(land) + Grizzly Bears(nonland) in P1 graveyard; {3}{G}{G} available. Activate from graveyard, target Forest, resolve → assert BOTH Sandman AND Forest on battlefield, BOTH TAPPED. Discriminating-target: Grizzly Bears (nonland) NOT eligible target. Revert-to-red: restoring the misparse fails twice (activation_zone null → not offered; And empty + Unimplemented sub → nothing moves).
MANDATORY coverage-regression (recognizer in shared dispatch): regen with recognizer DISABLED (comment :11972 call) → baseline; ENABLED → current; coverage-regression-check baseline current. Assert Sandman + Slimefoot GAINED (Unimplemented ↓≥2), 0 REGRESSED(engine), no sibling "return A and B" card changed.

## RISKS
1. multi_target preservation on sub (Slimefoot up-to-one) — carry sub_clause.multi_target/optional onto the wrapped AbilityDefinition; verify chained-sub targeting reads it (Combo Attack/Omo precedent). If not honored, Slimefoot degrades to exactly-one → flag.
2. Simultaneity: two chained sub-events vs one CR 608.2 event — no observable diff for Sandman+land (matches shipped Coastal Wizard/Lady Sun). Revisit only for intra-set ETB-simultaneity.
3. Gating breadth — fires only on "return "+SelfRef-first+sub validating non-BF-origin→BF ChangeZone. Coverage-diff is the safety net.
