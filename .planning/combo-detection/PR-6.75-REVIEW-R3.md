# PR-6.75 plan review — ROUND 3 (fresh fable xhigh, confirmation, @bceec86e3, 2026-07-02)

VERDICT: GAPS — 1 MAJOR, 1 minor, 4 nits. All R2 closures GENUINE; both new proofs VERIFIED
(7-row category-(1) table stands); all 3 measurement deviations reproduced EXACT (incl. CR 800.4a
correction — R2 report was wrong, plan right); 5/6 novel dispositions sound; disjointness rule
sound at walker-guard bar; counts internally consistent; zero stale @47adf7fc1.

## F1 — MAJOR — §2 rule 1 (ParentTarget chain-root) undefined on the measured PARENTLESS-root shape;
Cactuar disposition false as written. Cactuar's trigger = `Bounce{target: ParentTarget}` as ROOT
effect, sub_ability null — no parent exists. Riders of the Mark same shape (root Bounce{ParentTarget}
→ sub Token{count: Toughness{Source}}). Rule 1's boundary "root unresolvable ⇒ external, census
unknown ⇒ overlap assumed" applied honestly ⇒ Cactuar = reads_src{SetMembership} (its ObjectCount
{And[SelfRef,EnteredThisTurn]} intervening-if) × sibling writes_external{SetMembership} ⇒ feed ⇒
CONFLICT ⇒ printed auto→prompt flip. Card is order-INDEPENDENT in truth (member-private predicates).
Cannot be a category-(1) row. Resolver semantics: targeting.rs:922-952 resolves parentless
ParentTarget per event shape (ZoneChanged ⇒ the ENTERING/EVENT object; Phase ⇒ None); filter.rs:
3061-3070 documents the untargeted-referent route. Riders is decision-neutral despite mislabel
(cleared by KIND non-feed, not chain root).
CLOSURE: (i) extend rule 1 w/ explicit parentless/root-position clause pinned to the RESOLVER's
referent — ZoneChanged ⇒ event-object write (routes into rule 2's disjointness machinery);
no-event-object (Phase) ⇒ pin resolver behavior (None/no-op vs source fallback) at impl, classify
to match, FAIL-CLOSED if unpinnable; (ii) re-disposition Cactuar under the extended clause w/ own
soundness sentence; (iii) relabel Riders of the Mark in K-11 as kind-non-feed-cleared; (iv) N-E
pairing for the parentless-root shape (revert-fail: classifying it external flips it). Also state
rule 1's root-walk skips player/no-object intermediate effects (Wedding Announcement: topmost
OBJECT referent reading).

## F2 — minor — §5.2 floor "38 cards" is a unit slip: 38 TRIGGERS = 37 unique cards (Keeper of the
Accord has 2 qualifying triggers; 57 triggers = 56 cards). State floor in triggers (≥38) or as 37 cards.

## F3 — nit — Doom row leads w/ a gate mechanism absent from the measured AST (no highest-life
selector node; tie-break misparses as ChooseFromZone{Exile}). Row survives on the stated fallback
(PutCounter{Any} external × CountersOn{Source} read ⇒ conflict) — make the fallback PRIMARY.

## F4 — nit — Complex Automaton observability step unstated: add one sentence constructing the
asymmetry (one copy tapped/countered ⇒ battlefield outcome observably differs).

## F5 — nit — line 6 "working tree matches tip" stale: HEAD now 2e7ad800c (2 comment-only CR-format
commits past bceec86e3 in ability_scan.rs — ZERO anchor drift verified, 3394 lines). Fix sentence
(§7 content-signature step already covers). Related: §5.2's designed-mechanism list omits the two
R2 rules + §1.3.1-K.

## Confirmed (do NOT churn)
R2 closures all 10 genuine (B-NEW-1 a-closure w/ Firebrand parse + 12-card L surface + priced
ledger; M-NEW-1 K re-run exact incl. histogram + 18 dispositions; M-NEW-2 57/59 FPs = poppet
stitcher + sandsteppe war riders confirmed FilterProp non-writes, 19 exact, floor 38; M-NEW-3;
m-1..m-6 w/ ~25 anchors spot-verified). Proofs: Doom VERIFIED (CR 904.3 two-copy + CR 205.4h/704.6e
face-up; arithmetic (7,5)/(3,1) ⇒ (4,4) vs (3,5)); Complex Automaton VERIFIED (both fire at 7,
CR 603.4 re-check flips for second ⇒ order picks which bounces). Dispositions: dust stalker +
thopter assembly SOUND (mutual Another+ColorCount census ⇒ both fire-time conditions false);
scute swarm SOUND (creation-only writes, censuses disjoint from Land read); hunger tide SOUND
(CounterAdded valid_card:SelfRef ⇒ event matches ≤1 source); chain-root 11 sound EXCEPT Riders
(relabel per F1); non-feeding 5 ALL SOUND. Disjointness rule SOUND (Another proves X ≠ every member
source; self-entry ⇒ singleton; token-copy event object never a member source; fail-closed w/o
provable exclusion; N-E (vi) discriminates both directions).
