# Implementation Plan — Memory Vessel full parser lowering (PARSER-ONLY)

Scope: complete the parser so Memory Vessel's oracle text lowers fully (no `Effect::Unimplemented`). All engine primitives (`ProhibitPlayFromZone` variant + enforcement gates + resolver + untap-prune) already exist, compile clean, and are proven by 3 hand-built tests. This is PARSER-ONLY.

Barred files (s07-frozen): `game/effects/mod.rs`, `game/effects/delayed_trigger.rs`, `game/filter.rs`. `game/effects/add_restriction.rs` is READ-only confirmation, not edited.

## Ground truth (probe-measured)

`cargo test -p engine --test probe_mv_tmp` (exit 0): MV parses to **1 ability** — `ExileTop{ScopedPlayer, Fixed(7)}`, `player_scope: All`, cost `{Tap, Exile SelfRef}`, `activation_restrictions:[AsSorcery]` (all correct). Its lone `sub_ability` (kind `Spell`, `sub_link: SequentialSibling`, `duration: UntilNextTurnOf{Controller}`) is one collapsed `Effect::Unimplemented{ name:"players", description:"players may play cards they exiled this way, and they can't play cards from their hand" }`. `parse_warnings: []`.

## Design (a) — WHY the collapse (exact dispatch miss)

2nd sentence chunk `"Until your next turn, players may play cards they exiled this way, and they can't play cards from their hand"` (split at `". "` by `parse_effect_chain_ir`):
1. `parse_effect_clause` (mod.rs:5028) → `clause_shell::peel_clause` returns empty (shell only peels TRAILING duration via `strip_trailing_duration` clause_shell.rs:195, a leading condition, optional prefixes — a LEADING duration is not a shell slot; `"players may"` is not a peelable `you may`). → `parse_effect_clause_inner(full chunk)`.
2. Restriction block: `try_parse_cast_only_from_zones_restriction` (mod.rs:6192) needs `"can't cast spells from anywhere other than"` → None; `try_parse_cant_cast_spells_effect` (mod.rs:6198) parses `"until your next turn, "` + subject `"players"`→AllPlayers, then requires `" can't cast spells"` but sees `" may play…"` → None.
3. `strip_leading_duration` (mod.rs:6279, def lower.rs:4302 `terminated(parse_duration, tag(", "))`) strips `"Until your next turn, "`→`UntilNextTurnOf{Controller}` and RECURSES `with_clause_duration(parse_effect_clause("players may play cards they exiled this way, and they can't play cards from their hand"), UntilNextTurnOf)`.
4. In recursion, `try_parse_play_from_exile` (mod.rs:6427 → `try_parse_per_grantee_play_grant` mod.rs:9109) has ObjectOwner arms only for `"each player may play the card they exiled this way"` (mod.rs:9124-9127) — NO arm for `"players may play cards they exiled this way"`, and no handling of the `", and they can't…"` tail. → `Effect::Unimplemented`.
5. `with_clause_duration` (ast.rs:1514) `_ => {}` arm for `Unimplemented` sets only `clause.duration = UntilNextTurnOf`.

**Two independent misses:** (i) no per-owner grant arm for the `"players … cards"` surface; (ii) the `", and <restriction>"` conjunction is never split (`starts_bare_and_clause_lower` sequence.rs:1983 has no `"they can't play"` arm; `parse_effect_clause_inner` does not re-split).

## Design (b) — split grant + restriction

**Reject pure-split.** Splitting at `", and "` yields sibling chunks, but the shared leading `"Until your next turn,"` sits only on the grant chunk; there is NO general leading-duration distribution — the only re-stamp mechanism (`leading_host_lifetime_split` mod.rs:20501→23425) is gated to `Duration::UntilHostLeavesPlay` (mod.rs:20495-20500). The restriction chunk would lose `UntilNextTurnOf` → `expiry` fills to `EndOfTurn` instead of `UntilPlayerNextTurn{activator}` — wrong. `with_clause_duration` (ast.rs:1521-1550) does not descend into sub_abilities.

**Chosen — compound recognizer that OWNS the shared leading duration and composes two general building blocks**, mirroring how `try_parse_cast_only_from_zones_restriction` (mod.rs:2652-2672) and `try_parse_cant_cast_spells_effect` (mod.rs:2783-2807) each self-parse their leading duration + build a `sub_ability` tail. Runs in the restriction block BEFORE the mod.rs:6279 leading-strip (sees the leading duration intact), delegates grant + restriction halves to reusable recognizers.

## Design (c) — TrackedSet binding (unchanged, correct)

`try_parse_per_grantee_play_grant` emits `target: TargetFilter::TrackedSet{ id: TrackedSetId(0) }` (mod.rs:9199); `grant_permission::resolve` normalizes the sentinel to the most-recently-published set (the `ExileTop` cards) + rebinds `granted_to`→owner. Rocco pipeline snapshot (`oracle_pipeline_snapshot_tests.rs:229-287`) proves ObjectOwner + TrackedSet(0) end-to-end.

## Design (d) — duration→expiry plumbing (confirmed)

`add_restriction::fill_runtime_fields` (add_restriction.rs:90): source=ability.source_id (:98); `ability.duration == Some(UntilNextTurnOf{Controller})` → `expiry = UntilPlayerNextTurn{ player: ability.controller }` (:171-177); `affected_players: AllPlayers` passes through (:143-145). Test 3 exercises this via hand-built `.duration(UntilNextTurnOf{Controller})` (memory_vessel_std_s25.rs:328-346). **Parser's only job: emit `def.duration = Some(UntilNextTurnOf{Controller})` on the restriction sub_ability + `affected_players: AllPlayers`.**

## Pattern Coverage

- Grant arm (per-owner `"[each player|players] may [play|cast] [the] card[s] they exiled this way"`): class = {Memory Vessel, Rocco Street Chef} (card-data.json grep: 2 cards, both covered).
- Restriction recognizer (`"[scope] can't play [cards|lands or cast spells] from [zone] [this turn]"` → `ProhibitPlayFromZone{zone}`): effect-form members = Memory Vessel (hand) + Shaman's Trance (`"Other players can't play lands or cast spells from their graveyards this turn"` → graveyard, `OpponentsOfSourceController`, `EndOfTurn`). (Experimental Frenzy `"You can't play cards from your hand"` is a printed STATIC — different layer, out of scope, confirms class >1.)

## Step-by-step (all in `crates/engine/src/parser/oracle_effect/mod.rs` + test file)

**Step 1 — grant-phrase combinator + widen ObjectOwner arm (near mod.rs:9109).**
```rust
// CR 611.2a + CR 108.3 + CR 400.7i: per-owner "[each player|players] may
// [play|cast] [the] card[s] they exiled this way" — owner-binding exile-play
// grant, composed by dimension (subject × verb × object noun).
fn parse_per_owner_exiled_this_way(i: &str) -> OracleResult<'_, ()> {
    let (i, _) = alt((tag("each player "), tag("players "))).parse(i)?;
    let (i, _) = alt((tag("may play "), tag("may cast "))).parse(i)?;
    let (i, _) = opt(tag("the ")).parse(i)?;
    let (i, _) = alt((tag("cards"), tag("card"))).parse(i)?;   // longest-first
    tag(" they exiled this way").parse(i).map(|(i, _)| (i, ()))
}
```
In `try_parse_per_grantee_play_grant`'s ObjectOwner `alt` (9113-9147), REPLACE the 4 flat tags with one arm. **B1 (reviewer, must-fix): the existing `alt` tuple arms are all `tag(...)` → Output `&str`; a `value((), ...)` arm returning `()` will NOT typecheck.** Use `nom::combinator::recognize(parse_per_owner_exiled_this_way)` (yields the consumed `&str`, matching the tuple). Preserves Rocco (`each player`/`the card`/`the cards`), adds MV (`players`/bare `cards`). Surgical Edit.

**Step 2 — general `try_parse_cant_play_from_zone` recognizer (new fn near mod.rs:2782).**
Mirror `try_parse_cant_cast_spells_effect`: optional leading-duration strip (reuse the alt at 2783-2807), then `nom_on_lower` over: scope alt (`they`/`players`/`each player`→`AllPlayers`; `other players`/`your opponents`/`opponents`/`each opponent`→`OpponentsOfSourceController`; `target player`→`TargetedPlayer`), `alt((tag(" can't "),tag(" cannot ")))`, play-phrase `alt((tag("play cards"),tag("play a card"),tag("play lands or cast spells"),tag("play lands and cast spells")))`, `tag(" from ")`, zone alt (`their hand[s]`/`your hand`/`hand`→`Zone::Hand`; `their graveyard[s]`/`graveyard`→`Zone::Graveyard`; `exile`→`Zone::Exile`), `opt(tag(" this turn"))`. Build `ParsedEffectClause{ effect: Effect::AddRestriction{ restriction: GameRestriction::ProhibitActivity{ source: ObjectId(0), affected_players, expiry: RestrictionExpiry::EndOfTurn /*placeholder, overridden at resolution*/, activity: ProhibitedActivity::ProhibitPlayFromZone{ zone } } }, duration, .. }`. CR: `// CR 116.2a + CR 305.1 + CR 601.2a: prohibit playing (cast or land) from <zone>. // CR 611.2a + CR 514.2: duration→expiry at resolution.`

**Step 3 — compound recognizer `try_parse_exile_play_grant_with_play_prohibition` (new fn).**
```rust
// CR 611.2a: one leading duration scopes BOTH a per-owner exile-play grant and
// a play-from-zone prohibition ("Until your next turn, players may play cards
// they exiled this way, and they can't play cards from their hand" — Memory Vessel).
fn try_parse_exile_play_grant_with_play_prohibition(
    tp: TextPair<'_>, ctx: &ParseContext,
) -> Option<ParsedEffectClause> {
    let (dur, body) = strip_leading_duration(tp.original)?;                 // require shared duration
    // local conjunction split — grant=before ", and ", restr=after:
    let (grant_text, restr_text) = split_at_conjunction_and(body)?;         // terminated(take_until(", and "), tag(", and "))
    let g_lower = grant_text.to_lowercase();
    let mut grant = try_parse_play_from_exile(TextPair::new(grant_text, &g_lower), ctx)?;
    if !matches!(&grant.effect,
        Effect::GrantCastingPermission{ permission: CastingPermission::PlayFromExile{..}, .. }) {
        return None;                                                        // guard: must be PlayFromExile grant
    }
    let mut restr_def = parse_effect_chain(restr_text, AbilityKind::Spell); // routes to Step-2 recognizer
    if !matches!(&*restr_def.effect,
        Effect::AddRestriction{ restriction: GameRestriction::ProhibitActivity{
            activity: ProhibitedActivity::ProhibitPlayFromZone{..}, .. } }) {
        return None;                                                        // fail closed → honest Unimplemented
    }
    grant = with_clause_duration(grant, dur.clone());                       // patches PlayFromExile.duration + grant.duration
    restr_def.duration = Some(dur);                                         // → UntilPlayerNextTurn{activator} at resolution
    restr_def.sub_link = SubAbilityLink::SequentialSibling;
    grant.sub_ability = Some(Box::new(restr_def));
    Some(grant)
}
```
NOTE: exact `nom_on_lower`/remainder bookkeeping (the `split_at_conjunction_and` helper) must match the codebase idiom (`&lower[lower.len()-rest.len()..]`) and return original-case slices. The grant-before / restr-after mapping is the load-bearing part — implementer must verify with a probe.

**Step 4 — dispatch wiring (after mod.rs:6200, BEFORE the leading-strip at 6279).**
```rust
// CR 611.2a: shared-duration exile-play grant + play-from-zone prohibition (Memory Vessel).
if let Some(clause) = try_parse_exile_play_grant_with_play_prohibition(tp, ctx) { return clause; }
// CR 116.2a + CR 305.1: "[players] can't play cards from [zone]" prohibition (Shaman's Trance).
if let Some(clause) = try_parse_cant_play_from_zone(tp) { return clause; }
```
Order: compound first (needs grant + `", and"` + prohibition; standalone prohibition returns None on MV's chunk). Both precede mod.rs:6279 so the compound sees the leading duration.

Result chain: `ExileTop → grant{PlayFromExile{UntilNextTurnOf}, ObjectOwner, TrackedSet(0), duration:UntilNextTurnOf} → restriction{AddRestriction{ProhibitPlayFromZone{Hand}, AllPlayers}, duration:UntilNextTurnOf, SequentialSibling}`.

## Verification Matrix

| Claim | Test / revert-failing assertion | Negative / reach-guard |
|---|---|---|
| MV lowers fully | New card-level test: no `Effect::Unimplemented` in chain (today IS Unimplemented → revert-fails) | Rocco standalone grant still `PlayFromExile{ObjectOwner, UntilNextStepOf}` (refactor didn't regress) |
| Grant shape | one chain effect = `GrantCastingPermission{PlayFromExile{UntilNextTurnOf{Controller}}, ObjectOwner, TrackedSet(0)}` | NOT `ParentTargetController`/`Any` grantee |
| Restriction shape+duration | one chain effect = `AddRestriction{ProhibitPlayFromZone{Zone::Hand}, AllPlayers}` with owning def `.duration == Some(UntilNextTurnOf{Controller})` (revert Step-3 duration set → None → fails) | positive shapes present → negative non-vacuous |
| Sorcery timing | `activation_restrictions` contains `AsSorcery` | — |
| Runtime resolution | 3 existing hand-built tests STAY (per-owner scoping, activator-keyed expiry, hand-play block cast+land) | test 1 P2≠P1 set; test 2 survive-P1-untap/expire-P0-untap |

**Card-level test** (add to memory_vessel_std_s25.rs, verbatim Oracle text): `parse_oracle_text("{T}, Exile this artifact: Each player exiles the top seven cards of their library. Until your next turn, players may play cards they exiled this way, and they can't play cards from their hand. Activate only as a sorcery.", "Memory Vessel", &[], &["Artifact".into()], &[])` — walk `abilities[0]` + sub_ability chain into `Vec<&Effect>`; assert `ExileTop` + `GrantCastingPermission{PlayFromExile,ObjectOwner,TrackedSet(0),UntilNextTurnOf}` + `AddRestriction{ProhibitPlayFromZone{Hand},AllPlayers}` (owning def duration `UntilNextTurnOf{Controller}`) + `AsSorcery`; assert ZERO `Effect::Unimplemented`. Discriminating (today chain has the Unimplemented sub_ability).

## Coverage probe (CI-only regression — MANDATORY, rider #1)

Steps 1 (widens shared grant alt) + 2-4 (shared restriction dispatch) → run BEFORE/AFTER coverage + report every non-MV delta:
- MV must flip supported; net Unimplemented must not rise.
- Rocco Street Chef — grant-arm refactor must be BYTE-IDENTICAL grant.
- Shaman's Trance — expected NEW `ProhibitPlayFromZone{Graveyard}` clause: verify rules-correct (`OpponentsOfSourceController`, `EndOfTurn`) AND enforcement covers cast-from-graveyard + play-land-from-graveyard (zone-general gate).
- Experimental Frenzy — static layer, must be UNCHANGED.
- grep `"can't play"` / `"may play … exiled this way"` (tiny set: ~2 grant + ~3 prohibit). Any card newly losing a clause or misparsing = blocker.

## Cleanup / cadence
- Delete throwaway `crates/engine/tests/probe_mv_tmp.rs` at commit time.
- Also fix trivial `mut` warning memory_vessel_std_s25.rs:136 (`let mut runner` → `let runner`; -D warnings promotes it to error).
- `cargo fmt --all`; parser combinator gate (`scripts/check-parser-combinators.sh`); then `cargo check --workspace` + `cargo clippy --workspace --exclude phase-tauri --all-targets -- -D warnings` (CI surface, rider #4) + `cargo test -p engine`. Coverage probe run directly.
- NON-NEGOTIABLE: nom combinators on first write (rider #2). NO new engine variants. NO s07-frozen files.

## REVIEW VERDICT (abf896cd30cb9d6c6): APPROVE-WITH-CONDITIONS — executor MUST satisfy
Every material claim verified true (root-cause, pure-split rejection, duration→expiry add_restriction.rs:171-177, TrackedSet, zone-general enforcement casting.rs:538, all 7 CRs, scope guards, discriminating test). Card classes confirmed: grant class = exactly {MV, Rocco} (no other card carries `" they exiled this way"` suffix → Step-1 widen can't swallow another clause); prohibition class checked all 15 `can't play` candidates → NO false positives (Experimental Frenzy/Aggressive Mining/Rock Jockey are `you`-scoped statics; the `play lands`-bare / no-`from` cards rejected by the mandatory `" from " + zone` guard). Shaman's Trance flip rules-correct + enforced.

CONDITIONS:
1. **B1 (compile fix):** Step-1 arm = `recognize(parse_per_owner_exiled_this_way)` not `value((), ...)` (alt tuple Output = `&str`). Folded into Step 1 above.
2. **Coverage probe with EVIDENCE:** (a) Rocco snapshot `pipeline_rocco_street_chef_emits_three_triggers` (src/parser/oracle_pipeline_snapshot_tests.rs:229, INLINE module not tests/) stays green — byte-identical `ObjectOwner`/`TrackedSet(0)`/`UntilNextStepOf`; (b) Shaman's Trance → `ProhibitPlayFromZone{Graveyard}` with `OpponentsOfSourceController` + probe the RESOLVED expiry lands EndOfTurn (note: trailing "this turn" may be consumed by peel_clause's TRAILING-duration strip [strip_trailing_duration lives in lower.rs:4333, called from clause_shell.rs:198] rather than Step-2's `opt(tag(" this turn"))` — EITHER path must land EndOfTurn); (c) Experimental Frenzy stays a static (CantPlayLand), unchanged; (d) net Unimplemented does not rise.
3. **mut fix** (memory_vessel_std_s25.rs:136): confirmed safe (runner used only via `state(&self)`); keep contingent on `-D warnings`.

NITS (non-blocking): N1 citation paths (clause_shell.rs → crates/engine/src/parser/clause_shell.rs; ast.rs → crates/engine/src/parser/oracle_ir/ast.rs; strip_trailing_duration in lower.rs:4333 not clause_shell). N3: `split_at_conjunction_and` is a 1-line nom expr (`terminated(take_until(", and "), tag(", and "))`) — inline unless reused (ponytail).
