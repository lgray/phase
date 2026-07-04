# PR-6.75 plan review — ROUND 1 (fable xhigh, 2026-07-02)

VERDICT: GAPS — 1 BLOCKER, 4 MAJOR, 5 minor.

Anchor reconciliation: at 47adf7fc1 the plan/recon anchors are CORRECT (Power:1336, Source:2088,
EventSource:2091, verified via git show). The independent :1520/:2339/:2342 refs measured the
DRIFTED working tree (3287 lines, +185 — inc2b landing). 42 arms confirmed byte-verified.

## BLOCKER
B1 — LKI-freeze lemma (§1.1/T2) FALSE without a battlefield-re-entry side condition. Verified:
freeze at exit zones.rs:147-181 (insert :178), zone-guarded read quantity.rs:3728-3743
(live-battlefield-FIRST, LKI fallback), sole production lki_cache writer zones.rs:178, eviction
turns.rs:429. BUT zone moves mutate obj.zone in place (zones.rs:690) — ObjectId STABLE across moves
⇒ a sibling's graveyard→battlefield membership write RE-ENTERS a departed member's source under the
same id ⇒ the "frozen" read goes LIVE again (also re-death overwrites the LKI entry). Representable
counterexample: two co-departing "when this dies, create X tokens where X = its power, then return
all creature cards from your graveyard to the battlefield" (Token{Ref(Power{Source})} + ChangeZoneAll)
with asymmetric death-time powers — [S1,S2] = 1+1 tokens, [S2,S1] = 3+1. Gate admits it. Zero parity
impact (allowlist auto-orders it today too) + zero printed cards — but mission item 2 demands PROVEN,
and §4's shipped comment would be false. CLOSURE (cheap): frozen-read admission additionally requires
group write-set contains NO toward-battlefield membership write (or feed row
SetMembership(battlefield-entry) → frozen reads), + one synthetic hostile alongside N-B. Alternative:
narrow T2 honestly + document residual as inherited-from-allowlist. Plan currently does neither.

## MAJOR
M1 — §1.2 identity basis omits `die_result` (PendingTrigger.die_result triggers.rs:98, stamped into
StackEntryKind::TriggeredAbility :4042/:4081, read at resolution; group_is_order_independent
:3510-3547 never compares it). T1 "literally the same state transformation" incomplete. CLOSURE: add
die_result equality to group conjuncts (one line, strictly conservative) or prove unconstructible.
M2 — Measured-claims errors: (a) Chaotic Goo has NO CountersOn{Source} read — AST is
FlipCoin{win: PutCounter(Fixed 1, SelfRef), lose: RemoveCounter(Fixed 1, SelfRef)}; 1-bit flip risk is
solely the FlipCoin CONSERVATIVE arm. (b) Mana Crypt likewise read-free (FlipCoin{lose:
DealDamage(Fixed 3, Controller)}). (c) Arcbound Worker's modular trigger CARRIES A TARGET
(Typed[Creature,Artifact]) ⇒ ordering input ⇒ never gate-reachable; the 80-card src-read floor and
§8's "modular keeps auto-ordering by DESIGN" mis-calibrated. D1 conclusion SURVIVES on verified
evidence (Gutter Grime ✓ CountersOn{Source}; Fruit ✓ Toughness{Source}; Shambler ✓; Hangarback ✓) —
but §1.3/§1.4/§8/§5.2 floors must be corrected + re-measured over the no-ordering-input subset.
M3 — ability_rw.rs discipline under-specified: has wildcard-free mandate (§4 item 1, D5) +
add-engine-variant hook (D6); MISSING explicit "every non-fully-conservative arm binds ALL payload
fields — no `{ .. }` elision on precise arms" (the 5-hole inc1 class). One sentence in §4 item 1.
M4 — D-profile conservative floor predictably violates the zero-diff gate; §3 hedges ("or the card
prompts justified") contradict §5.2 ("no other category exists"). Unclassified-arm printed triggers
flip auto→prompt: GenericEffect (462+64), Animate (7), Choose (3), Scry (6, ABSENT from floor).
CLOSURE: (i) pre-classify GenericEffect by payload descent + add Scry + histogram-tail kinds, delete
hedges; or (ii) explicitly scope the expected sweep-forced iteration + keep zero-diff absolute.
Open kinds: DealDamage (write-Other default ✓ fail-closed), CreateDelayedTrigger (✓ fail-closed),
Mana = WEAK (pool-write "no sibling kind" is fail-open; guard is unpinned corpus claim — see m3).

## minor
m1 — log.rs:1325 lki writer citation is in mod tests (:1240); sole production writer zones.rs:178
(+turns.rs:429 clear). Fix citation (conclusion strengthens).
m2 — §5.2 "three group structures" then enumerates five.
m3 — D4 unless-pay/Mana pool-feed exclusion has no pinned hostile; add one N-E pairing (echo-style
unless-pay pair + pool-read shape asserting conservative) or document on the arm.
m4 — §5.1 "PR-6.25-era predicate tests all green" measured against batch-path tightening; C1 changes
the same-event branch — name the re-measurement (grep same-event group fixtures).
m5 — zero-diff proof is a one-shot FORGE_TEST_FULL_DB=1 implementer run; list the PR-description
paste as a MERGE PRECONDITION, not a note.

## Verified clean
All 14 CR citations ✓. 12-tag→typed-variant anchors ✓ (types/ability.rs spot-verified).
legacy_uses_trigger_event consumers fully accounted (:3419/:3441/:3523/:3535 + doc :3499).
Demonstrate/Replicate same source_id ✓; copy_spell by-id/never-mutates-original ✓ (:468+);
HadCounters reads lki_cache directly (triggers.rs:6195) ✓; groups per-controller ✓; T1 multiset
argument sound for genuinely identical members (incl RNG/may-choices/CR 603.4 re-checks) — only
holes are M1 + B1. Shambler Source-not-EventSource ✓. zone_changes retained ✓. Axis-3 strongest form ✓.

## Scorecard
Item 1 satisfied (gated on M4); item 2 NOT satisfied (B1); item 3 satisfied minus M1; item 4
satisfied; item 5 gated on M2/M4 + m5; item 6 satisfied.
