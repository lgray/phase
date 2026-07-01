# PR-4b implementation log (executor)

Worktree: /home/lgray/vibe-coding/wt-combo-pr4 (branch feat/combo-detect-pr4, HEAD f3cbf07e6 = PR-4a). cargo-direct (no Tilt).

## Verified facts (read in worktree)
- ability_graph.rs: effect_projection (:360), trigger_axis (:663), From<&ResourceAxis> for AxisKey (:141), fold_cost (:995). All 4 exhaustive no-wildcard.
- Proj builder :206-254 (add_mana/add_counter/add_damage/mark/finish). count_seed :260. default_object_class :270.
- build_nodes call sites :1227 trigger_axis(&trig.mode), :1243 trigger_axis(&trigger.mode).
- ResourceVector fields confirmed (resource.rs:160-219): life/damage_dealt/library_delta maps; tokens_created/cards_drawn/casts_this_step/landfall_triggers/combat_phases/extra_turns/death_triggers/etb_triggers/ltb_triggers/sac_triggers scalars.
- Zone enum (zones.rs:5): Library/Hand/Battlefield/Graveyard/Stack/Exile/Command.
- Phase::is_combat() (phase.rs:49) — reuse for AdditionalPhase.
- TriggerDefinition (ability.rs:15489): mode/origin:Option<Zone>/destination:Option<Zone>.
- SacrificeCost.target: TargetFilter (ability.rs:6340).
- Effect variant fields: Draw{count,target}; Mill{count,target,destination}; GainLife{amount,player}; LoseLife{amount,target:Option}; Token{count,..}; CopyTokenOf{count,..}; CreateTokenCopyFromPool{count,..}; Investigate(unit); ChangeZone{origin:Option,destination:Zone,..}; ChangeZoneAll{origin,destination,..}; Bounce{destination:Option,..}; BounceAll{destination:Option,..}; Sacrifice{target,count,..}; Destroy{target,..}; DestroyAll{target,..}; ExtraTurn{target}; AdditionalPhase{phase,count,..}; SearchLibrary{target_player:Option,count,..}; Seek{count,..}; Conjure{destination:Zone,..}.

## CR numbers grep-verified against docs/MagicCompRules.txt
- 701.8a (destroy=move to graveyard) :3321 — M1 fix
- 118.12 (effect performed as cost) :1031 — L1 fix (was 118.3 = resource availability :972)
- 119.3 (gain/lose life) :1065
- 119.4 (pay life subtracted) :1067
- 121.1 (draw) :1142
- 701.17a (mill) :3408
- 111.1 (token) :645
- 603.6a (ETB) :2599; 603.6c (LTB) :2604; 700.4 (dies) :3234
- 701.21a (sacrifice) :3451
- 500.7 (extra turn) :2127; 500.8 (extra phase) :2129
- 701.23a (search) :3465; 401.1 (library) :1994
- 104.3c (deck-out) :344; 704.5b :5494

## Decisions / judgement calls
- sac_produces_death: returns false ONLY when filter Typed with explicit Non(Creature). Positive Land/Artifact/Enchantment NOT treated as creature-excluding (creatures can share those types — Dryad Arbor, artifact/enchantment creatures) → recall-safe (plan risk table: never false for undeterminable).
- LifeLostAll left None (plan §3.4 table lists LifeLost/LifeChanged/PayLife only). MilledAll IS flipped (plan lists it). Documented asymmetry.
- Bounce/BounceAll routed through project_zone_change(Some(Battlefield), dest) — origin always battlefield ⇒ always LTB ⇒ M2 else-branch unreachable but present for uniformity.

## Status: COMPLETE (uncommitted in worktree)
- cargo build -p engine: OK 20s (exhaustiveness proven — no missing-variant errors).
- cargo clippy -p engine --lib --tests -D warnings: OK.
- cargo test -p engine --lib analysis::ability_graph: 36 passed (17 new).
- cargo test -p engine --lib analysis: 132 passed.
- cargo test -p engine --lib (full): 13875 passed, 0 failed, 6 ignored.
- CR diff gate: 0 UNVERIFIED. Fixed one mis-cite: 205.4b→205.2b (card-type multiplicity), and 118.3→118.12, +701.8a for Destroy.
- Revert probes all RUN + restored byte-identical (git diff = ability_graph.rs only, no probe residue):
  - C1-1 (dies trigger→None): node_a.requires(Death) flips FAIL.
  - C1-2 (Effect::Sacrifice→Unmodeled): node_b.produces(Death) flips FAIL.
  - C2-1 (LifeGained→None): node_h.requires(Life) flips FAIL.
  - C2-2 (Effect::GainLife→Unmodeled, PayLife stays neg): node_f.produces(Life) flips FAIL — symmetric-pair necessity proven.
  - M2 (drop ChangeZone Unmodeled else-branch): GY→hand matches!(Unmodeled) flips FAIL.
- One stale 4a test updated: unmodeled_effect_projects_nothing used Draw as "still-deferred" example; Draw is now modeled → switched to Scry (stays Unmodeled per §3.2).
