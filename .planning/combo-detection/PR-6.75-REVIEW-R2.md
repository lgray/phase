# PR-6.75 plan review — ROUND 2 (fresh fable xhigh, 2026-07-02)

Verified against pr65-wt tip bceec86e3 (atop upstream/main 6cefafb21). All R1 findings CONFIRMED
CLOSED. All 5 printed-card proofs VERIFIED (none refuted ⇒ literal zero-change unsatisfiable;
scoped mission stands). 42 CONSERVATIVE arms byte-identical at tip (lines moved). No composition
hazard with bceec86e3 loop-(iv)/granted_keyword_triggers_in_zone (disjoint surface). 17 CR
citations + CR 702.15 all grep-verified.

VERDICT: GAPS — 1 BLOCKER, 3 MAJOR, 6 minor (all NEW, in the revision's own extensions).

## BLOCKER
B-NEW-1 — T1/T3 "identical functions" premise FALSE for source-actor effects modulated by non-AST
per-source state; read-based conflict formula cannot see it. CR 702.15 lifelink / CR 702.2
deathtouch modulate damage resolution through source-granted state invisible to RwProfile (aura/
counter on one copy doesn't change the normalized AST ⇒ group forms). Printed counterexample:
Firebrand Archer (DamageEachPlayer{Fixed 1, Opponent}, SpellCast, no-input, zero reads ⇒
source_independent() true ⇒ T1 auto; feeds(∅)=false ⇒ auto through main formula too). Two Archers,
one lifelink-granted, MP, one opponent at 1 life: order changes controller lifegain (+3 vs +2 —
first member's damage kills the opponent CR 704.5a, shrinking the second's recipient set CR 800.4c).
Object-recipient analogue: DamageAll + deathtouch vs Stuffy Doll-class. Second channel same family:
player-loss cascade — PlayerLife write kills player ⇒ CR 800.4c removes their objects ⇒ feeds
sibling SetMembership reads, NO feed row (11 printed player-dmg × board-read co-occurrences
measured; none order-dependent through own reads ⇒ parity holds, but proof doesn't cover channel).
IMPACT: zero parity diff (auto today via shipped short-circuit) — but T1 says "trivially sound" and
§4's shipped comment would claim "provably commute" — false as stated. Grade-identical to R1-B1.
CLOSURE OPTIONS: (a) honest narrowing — side condition on T1/T3 ("modulo source-attributed
resolution modulators — lifelink/deathtouch-class granted state and player-loss object-removal
cascades — documented residual inherited from the shipped same-event short-circuit, zero decision
change"), correct shipped-comment text, follow-up-ledger row; parity-free, cheap. (b) constructive —
source-actor damage kinds (DealDamage/DamageAll/DamageEachPlayer/Fight) source-bound in
source_independent() + PlayerLife→SetMembership loss-cascade row; FLIPS printed cards (Court of
Embereth class in E-59) + needs per-card proofs — expensive, EXPANDS the visible surface beyond the
locked adjudication. Plan currently does neither.

## MAJOR
M-NEW-1 — Event-anaphoric write targets flip 3 printed cards never measured; §1.3.1-G "only …
co-occurrences" false as measured. Railway Brawler (PutCounter{target:TriggeringSource} ×
Power{Source} read; Counters-row scope rule doesn't cover TriggeringSource ⇒ fail-closed external ⇒
feed ⇒ PROMPT; auto today; NOT order-dependent — event object is "another" creature, never a group
source ⇒ provably disjoint). Smoldering Egg / Replicating Ring (RemoveCounter{target:ParentTarget}
"remove all of them" × CountersOn{Source}; ParentTarget chains to parent's SelfRef = semantically
self-write; no chain-root resolution rule ⇒ fail-closed external ⇒ PROMPT; auto today; provably
order-independent). CLOSURE: (i) re-run G including anaphoric/event-object target shapes
(TriggeringSource/ParentTarget/EventTarget/…) across ALL write kinds; (ii) two designed rules —
ParentTarget writes classify by CHAIN-ROOT scope (self when root is SelfRef); event-object-targeted
writes on same-event path get object-disjointness clearing vs reads_src ("another"-filtered event
object provably not a group source); (iii) N-E pairings + revert-fails for both.
M-NEW-2 — §5.2 positive floor self-contradictory: asserts "E-59 classifies auto" but §1.3.1-E splits
59 = 14 source-dependent + 45 source-independent, and 3 of the 14 (Docent/Sidequest/Promise) are
category-(1) PROMPTS. §1.2-T1 repeats the mislabel. Also pin which def is normative (direct
type-matching = 59 vs effect-keyed = 57). CLOSURE: floor = the 45 source-independent subset (or
59-minus-named-rows); fix T1 sentence; state the def.
M-NEW-3 — Mission-statement rewrite incomplete + user-override caveat ABSENT. Header line 3 still
"zero printed-card ordering-decision change in either direction" — contradicts §0.1/§5.2's 5-card
visible surface. Gated-C1 pivot caveat appears nowhere. CLOSURE: header line 3 → "zero
ordering-decision change outside the proven-CR-603.3b-unsound set; visible surface = exactly the
proven-order-dependent rows of §5.2, each with an in-plan proof"; add gated-C1 pivot caveat to §0/§7.

## minor
m-1 — §1.3 same_event total 5,557 not reproducible (measured 4,881 non-legendary / 6,308 all); pin
def or correct (other floors A/E/G/H/I reproduced exactly).
m-2 — Anchor re-pins from rebase (MOVED only, content byte-identical): allowlist :3358→:3395, leaf
:3416→:3453, batch :3391→:3428, group :3510→:3547, begin :3562→:3599, no_ordering_input :3467→:3504,
stamps :4042/:4081→:4079/:4118, Demonstrate seam :2608→:2630, Replicate :2547→:2560, broken tests
19036→19168 / cc 15092→15609, pr625 fixtures →20584/20629/20706; ability_scan 3102→3394 lines, arms
Pump 324→332 … UnlessControlsOtherLeq 2638→2927; quantity pin :9722→:9715; sba comment :198→:220 +
NEW 4th mark_simultaneous_departures call :1223 (premise-neutral). ALSO: 6 axis-3 #[allow(dead_code)]
now REMOVED at tip (inc2b landed) — §5.5(c)/§7 wording stale.
m-3 — log.rs mod tests :1239 not :1240.
m-4 — Your Inescapable Doom (scheme; PutCounter{Any} + CountersOn{Source} + highest-life damage) =
genuine category-(1) member → add proof row; §0 "exactly these [5]" → "at least these 5 +
sweep-surfaced rows, each with proof".
m-5 — Line 101 keeps a "sweep arbitrates" (Heaven Sent saga-proxy) vs §2-tail "ONLY place";
reconcile wording (it's a measurement proxy, not a classification hedge).
m-6 — Census timing: source_census read once at begin_trigger_ordering; third-party type changes
possible in priority windows before members resolve (types mutable, token-ness not). Member
self-writes covered; third-party channel = inherited atomic-window assumption shared with shipped
allowlist path — document on the census row.

## Confirmations (for the reviser: do NOT churn these)
R1 closures: B1 (mechanism verified at tip; N-B2 + parity-I reproduced 6/0), M1 (die_result;
Some-stampers bypass begin_trigger_ordering via :4064), M2 (Goo/Crypt/Arcbound reproduced; floors
41/40 + 9/8), M3, M4-as-specified, m1-m5 — all genuinely closed.
5-proof table: Ouroboroid / Docent / Sidequest / Promise (strongest) / Spawn of Mayhem — ALL
VERIFIED with reconstructed proofs.
DealDamage reversal: 100-trigger class reproduced; per-payload classification parity-sound (proof
residual folded into B-NEW-1). Recipient RMW commutation proof VERIFIED (incl. set-type +
self-exclusion; Canopy checked). Census-overlap sound modulo m-6 doc. 45-card parity NECESSITY of
the T1 disjunct confirmed (it must stay; it needs the B-NEW-1 side condition).
Scorecard: items 1/4/6 satisfied; 2 partial (B-NEW-1 batch side); 3 NOT (B-NEW-1+M-NEW-1); 5 gated
(M-NEW-1/2); adjudication (1) satisfied, (2) NOT (M-NEW-3), (3a) verified, (3b) breached at T1.
