//! CR 603.3b: the PR-6.75 read/write **conflict** profiler — a second
//! compiler-exhaustive, wildcard-free walk of a resolved ability's typed AST,
//! sibling to `ability_scan.rs`. Where `ability_scan` answers three 1-bit
//! read-axis questions, this module answers the richer question the legacy
//! trigger-ordering paths need (§1.2 of the PR-6.75 plan, PR-6.25 §3 C0(ii)):
//! *which kinds of game state does an ability READ, which does it WRITE, and at
//! what scope* — so `group_is_order_independent` can auto-order a same-event or
//! co-departure group of normalized-identical siblings only when their
//! resolution functions provably commute (CR 603.3b), and prompt otherwise.
//!
//! The gate predicate is fail-closed and **kind/scope-aware**: two identical
//! siblings conflict iff one member's WRITE lands in a location class the other
//! member's LIVE read observes (a read/write *feed*). Members are normalized-
//! identical, so a sibling's write-set equals my own.
//!
//! # Soundness scope (documented residual)
//!
//! Commutation is proven **modulo the source-actor residual** (§1.2 side
//! condition): per-source granted state (lifelink CR 702.15 / deathtouch
//! CR 702.2) and the CR 800.4a player-loss object-removal cascade modulate
//! resolution without appearing in the normalized AST or any profiled read.
//! Both channels were auto-ordered UNCONDITIONALLY by the pre-C1 short-circuit
//! and are inherited unchanged (zero ordering-decision change). Damage kinds are
//! therefore RECIPIENT-classified, not source-bound (CR 704.5a / CR 800.4a).
//!
//! A SECOND inherited fail-open: `reads_event_live` is consulted ONLY on the
//! batch path (`profiles_conflict`: the `all_same_source` fast path and the
//! freeze-invalidation row, both guarded `!same_event`) — never in same-event
//! feed analysis. So a same-event group that WRITES the triggering object and
//! then READS that now-modified object's characteristic ("put a +1/+1 counter on
//! it, then transform ~ if that creature's power ≥ 6" ×2 — order-observable,
//! `source_independent` false so the T1 fast path is skipped, yet the
//! event-object read×write feed is uncaught ⇒ auto) is not gated. This is
//! DISTINCT from the PR-6.25 Case A board-write × source-read feed (which IS
//! caught). Like the source-actor residual it is inherited from the pre-C1
//! always-auto short-circuit (NOT a regression) and is left OPEN deliberately:
//! closing it would ADD a prompt = a D3 widening needing its own proof.
//!
//! # M3 binding mandate (review-blocking)
//!
//! Every NON-fully-conservative arm binds ALL payload fields of its variant;
//! `{ .. }` field elision is permitted ONLY on arms whose RHS is
//! maximal-conservative (`RwProfile::conservative()`). A precise arm that elides
//! a field would classify whatever that field carries as nothing — fail-OPEN
//! (the inc1 5-hole class). Same discipline as `ability_scan.rs`.
//!
//! # Traversal closure & fail-closed defaults
//!
//! Closed under payload reachability across the same type set as `ability_scan`
//! (`Effect`, `QuantityRef`, `QuantityExpr`, `AbilityCondition`,
//! `TriggerCondition`, `TargetFilter`, `ObjectScope`, `PlayerFilter`,
//! `PlayerScope`, `ControllerRef`, `CountScope`, `StaticCondition`, `Duration`),
//! plus the choice/RNG `AbilityDefinition` sub-bodies (§2 choice-wrapper / RNG
//! union descent). A future variant must fail to compile until classified. An
//! effect KIND absent from the plan's §1.3.1-D group-reachable histogram (zero
//! printed presence, nothing to flip) may take `RwProfile::conservative()`.
//! Sub-enums the walk does not descend (`FilterProp` interiors,
//! `PermissionGrantee`, `CombineSource`, `DamageSource`) are handled like
//! `ability_scan`'s conservative subtrees; the §5.2 parity sweep (commit 2) is
//! the arbiter for any D5 tag that hides only there.
//!
//! CR annotations: CR 603.3b (gate), CR 603.10a + CR 400.7 (frozen-read kinds +
//! freeze-invalidation row), CR 603.4 (condition inclusion), CR 603.5
//! (resolution-time-choice exclusions + Mana×unless-pay guard), CR 603.7
//! (deferred-body arms), CR 707.10 / CR 707.10c (CopySpell), CR 707.2
//! (CopyTokenOf template read), CR 702.15 / CR 702.2 / CR 704.5a / CR 800.4a
//! (source-actor residual).

// Consumers landed in commit 2: `game::triggers::group_is_order_independent`
// calls `ability_rw_profile` / `trigger_condition_rw_profile` / `profiles_conflict`
// on the legacy same-event and departure-batch ordering paths (CR 603.3b).

use crate::types::ability::FilterProp;
use crate::types::ability::{
    AbilityCondition, AbilityDefinition, ControllerRef, Duration, Effect, ModalChoice,
    MultiTargetSpec, ObjectScope, PlayerFilter, PlayerScope, QuantityExpr, QuantityRef,
    RepeatContinuation, ResolvedAbility, StaticCondition, TargetFilter, TriggerCondition,
    TypeFilter, TypedFilter,
};
use crate::types::game_state::TargetSelectionConstraint;
use crate::types::zones::Zone;
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// State-kind lattice (§ D-profile).
// ---------------------------------------------------------------------------

/// A class of mutable game state, for CR 603.3b sibling-conflict analysis.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StateKind {
    /// CR 208/209: object power/toughness (live, after layers).
    ObjectPt,
    /// CR 122.1: counters on an object.
    ObjectCounters,
    /// CR 400: zone membership / control (which objects are where / whose).
    SetMembership,
    /// CR 119: player life (and energy/poison/player counters CR 122.1, folded
    /// here as monotone player resources).
    PlayerLife,
    /// CR 401/402: hand + library contents/order.
    HandLibrary,
    /// CR 119.3: per-turn life-change journal (fed by `PlayerLife` writes).
    JournalLife,
    /// CR 120.6: per-turn draw/discard journal (fed by `HandLibrary` writes).
    JournalCards,
    /// CR 601: per-turn/per-game cast journal (no in-resolution write feeds it).
    JournalCast,
    /// CR 405: the stack's shape (copies pushed, spells countered).
    StackShape,
    /// CR 301.5/302.6: tap state.
    TapState,
    /// Unclassifiable — conflicts with everything (fail-closed).
    Other,
}

/// Explicit-bool set over `StateKind` — mirrors `Axes`' style (no bitmagic).
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct KindSet {
    object_pt: bool,
    object_counters: bool,
    set_membership: bool,
    player_life: bool,
    hand_library: bool,
    journal_life: bool,
    journal_cards: bool,
    journal_cast: bool,
    stack_shape: bool,
    tap_state: bool,
    other: bool,
}

impl KindSet {
    const EMPTY: KindSet = KindSet {
        object_pt: false,
        object_counters: false,
        set_membership: false,
        player_life: false,
        hand_library: false,
        journal_life: false,
        journal_cards: false,
        journal_cast: false,
        stack_shape: false,
        tap_state: false,
        other: false,
    };
    const ALL: KindSet = KindSet {
        object_pt: true,
        object_counters: true,
        set_membership: true,
        player_life: true,
        hand_library: true,
        journal_life: true,
        journal_cards: true,
        journal_cast: true,
        stack_shape: true,
        tap_state: true,
        other: true,
    };

    fn one(k: StateKind) -> KindSet {
        let mut s = KindSet::EMPTY;
        s.set(k);
        s
    }
    fn set(&mut self, k: StateKind) {
        match k {
            StateKind::ObjectPt => self.object_pt = true,
            StateKind::ObjectCounters => self.object_counters = true,
            StateKind::SetMembership => self.set_membership = true,
            StateKind::PlayerLife => self.player_life = true,
            StateKind::HandLibrary => self.hand_library = true,
            StateKind::JournalLife => self.journal_life = true,
            StateKind::JournalCards => self.journal_cards = true,
            StateKind::JournalCast => self.journal_cast = true,
            StateKind::StackShape => self.stack_shape = true,
            StateKind::TapState => self.tap_state = true,
            StateKind::Other => self.other = true,
        }
    }
    fn union(self, o: KindSet) -> KindSet {
        KindSet {
            object_pt: self.object_pt || o.object_pt,
            object_counters: self.object_counters || o.object_counters,
            set_membership: self.set_membership || o.set_membership,
            player_life: self.player_life || o.player_life,
            hand_library: self.hand_library || o.hand_library,
            journal_life: self.journal_life || o.journal_life,
            journal_cards: self.journal_cards || o.journal_cards,
            journal_cast: self.journal_cast || o.journal_cast,
            stack_shape: self.stack_shape || o.stack_shape,
            tap_state: self.tap_state || o.tap_state,
            other: self.other || o.other,
        }
    }
    fn minus(self, o: KindSet) -> KindSet {
        KindSet {
            object_pt: self.object_pt && !o.object_pt,
            object_counters: self.object_counters && !o.object_counters,
            set_membership: self.set_membership && !o.set_membership,
            player_life: self.player_life && !o.player_life,
            hand_library: self.hand_library && !o.hand_library,
            journal_life: self.journal_life && !o.journal_life,
            journal_cards: self.journal_cards && !o.journal_cards,
            journal_cast: self.journal_cast && !o.journal_cast,
            stack_shape: self.stack_shape && !o.stack_shape,
            tap_state: self.tap_state && !o.tap_state,
            other: self.other && !o.other,
        }
    }
    fn is_empty(self) -> bool {
        self == KindSet::EMPTY
    }
    fn any(self) -> bool {
        !self.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Census (§2 census-overlap refinement of the SetMembership same-kind row).
// ---------------------------------------------------------------------------

/// Extractable type-tag requirements (core types + subtypes + token-ness),
/// lowercased into a common tag space.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Census {
    /// No object moved (absent/no-op write) — overlaps nothing.
    None,
    /// Unextractable / unbounded — assume overlap (fail-closed).
    Any,
    /// A concrete positive tag set.
    Tags(BTreeSet<String>),
}

impl Census {
    fn merge(&mut self, o: Census) {
        let taken = std::mem::replace(self, Census::None);
        *self = match (taken, o) {
            (Census::None, x) | (x, Census::None) => x,
            (Census::Any, _) | (_, Census::Any) => Census::Any,
            (Census::Tags(mut a), Census::Tags(b)) => {
                a.extend(b);
                Census::Tags(a)
            }
        };
    }
}

/// CR 205 tag-set overlap: two censuses can name a common object iff they share
/// a tag; `None` overlaps nothing, `Any` overlaps every non-`None`.
fn census_overlap(a: &Census, b: &Census) -> bool {
    match (a, b) {
        (Census::None, _) | (_, Census::None) => false,
        (Census::Any, _) | (_, Census::Any) => true,
        (Census::Tags(x), Census::Tags(y)) => x.intersection(y).next().is_some(),
    }
}

/// The group's live source objects' type census. Read once at the
/// `begin_trigger_ordering` chokepoint. A missing source ⇒ `None` ⇒ overlap
/// assumed (fail-closed).
#[derive(Clone, Debug, Default)]
pub(crate) struct SourceCensus {
    tags: Option<BTreeSet<String>>,
}

impl SourceCensus {
    pub(crate) fn from_tags<I: IntoIterator<Item = String>>(tags: I) -> Self {
        SourceCensus {
            tags: Some(tags.into_iter().map(|t| t.to_lowercase()).collect()),
        }
    }
    pub(crate) fn unknown() -> Self {
        SourceCensus { tags: None }
    }
    fn as_census(&self) -> Census {
        match &self.tags {
            None => Census::Any,
            Some(t) => Census::Tags(t.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// ZoneSpan (§2 census-overlap refinement — the ZONE axis of the SetMembership
// same-kind row; CR 400.1 a zone is where objects live).
// ---------------------------------------------------------------------------

/// CR 400.1: the zone(s) a `SetMembership` read observes / a membership write
/// touches. A whole-zone or `InZone`-free read is `Any` (fail-closed); a
/// creation write touches only its DESTINATION (battlefield for a token); a move
/// is recorded fail-closed `Any` (both endpoints matter but are not tracked
/// precisely). The membership feed row (§2) requires zone overlap IN ADDITION to
/// type-census overlap, so a battlefield-destination token creation cannot feed
/// a graveyard-count read (Tombstone Stairwell). `merge` is fail-closed — `Any`
/// swallows any precise set — so a mix of a precise write and an unrefined `Any`
/// write yields `Any` (never fewer conflicts than a single unrefined write).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum ZoneSpan {
    /// No membership read/write on this side — overlaps nothing.
    None,
    /// Unextractable / untracked — assume overlap (fail-closed).
    Any,
    /// A concrete zone set.
    Zones(std::collections::HashSet<Zone>),
}

impl ZoneSpan {
    fn one(z: Zone) -> ZoneSpan {
        ZoneSpan::Zones(std::iter::once(z).collect())
    }
    fn merge(&mut self, o: ZoneSpan) {
        let taken = std::mem::replace(self, ZoneSpan::None);
        *self = match (taken, o) {
            (ZoneSpan::None, x) | (x, ZoneSpan::None) => x,
            (ZoneSpan::Any, _) | (_, ZoneSpan::Any) => ZoneSpan::Any,
            (ZoneSpan::Zones(mut a), ZoneSpan::Zones(b)) => {
                a.extend(b);
                ZoneSpan::Zones(a)
            }
        };
    }
}

/// CR 400.1 zone overlap: two spans can name a common zone iff they share one;
/// `None` overlaps nothing, `Any` overlaps every non-`None` (mirrors
/// `census_overlap`).
fn zone_overlap(a: &ZoneSpan, b: &ZoneSpan) -> bool {
    match (a, b) {
        (ZoneSpan::None, _) | (_, ZoneSpan::None) => false,
        (ZoneSpan::Any, _) | (_, ZoneSpan::Any) => true,
        (ZoneSpan::Zones(x), ZoneSpan::Zones(y)) => x.intersection(y).next().is_some(),
    }
}

/// CR 400.1: the zones a read filter observes — its explicit `InZone`/`InAnyZone`
/// constraints (`TargetFilter::extract_zones`), or `Any` when it declares none (a
/// bare board read defaults to the battlefield, but we stay fail-closed rather
/// than assume it, so an `InZone`-free read still conflicts with every membership
/// write as before; only a filter with an EXPLICIT zone gets precise treatment).
fn zones_of_filter(f: &TargetFilter) -> ZoneSpan {
    let zones = f.extract_zones();
    if zones.is_empty() {
        ZoneSpan::Any
    } else {
        ZoneSpan::Zones(zones.into_iter().collect())
    }
}

// ---------------------------------------------------------------------------
// RwProfile (§2 D-profile).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct RwProfile {
    /// Source-scoped reads ONLY (live unless the structure freezes them, CR
    /// 603.10a). Recipient reads are NOT recorded — a Recipient read is the
    /// write's own modify-input (read-modify-write); identical members' composed
    /// per-object totals are symmetric (Canopy Gargantuan proof §1.3.1-G).
    reads_src: KindSet,
    /// Board-/graveyard-/stack-top-scoped mutable reads. Aggregates carry
    /// `SetMembership` alongside their value kind, so membership writes feed them.
    reads_board: KindSet,
    /// Life totals / hand size / journals — sibling-mutable player state.
    reads_player: KindSet,
    /// CR 603.10a look-back class: `HadCounters`, source cast-time facts. Never
    /// conflict WHILE THE FREEZE IS VALID (see `writes_reentry_hazard`).
    reads_frozen: KindSet,
    /// Event reads not proven frozen (`EventSource`/`EventTarget`/…). Consulted
    /// by T1's fast path and the freeze-invalidation row; the batch structure
    /// treats them as frozen.
    reads_event_live: bool,
    /// D5: one of the 12 retained-prompt event-context refs is present.
    /// Consulted ONLY by the batch branch (commit 2).
    legacy_batch_prompt: bool,
    /// Writes scoped to the member's own Source/Recipient object.
    writes_self: KindSet,
    /// Board-/player-/stack-scoped writes (INCLUDES creation, event-object, and
    /// LastCreated writes — everything not self). Board reads see all of these.
    writes_external: KindSet,
    /// Sub-portion of `writes_external` targeting the EVENT object
    /// (`TriggeringSource`-class + parentless `ParentTarget`, §2 rule 1/2).
    writes_event_object: KindSet,
    /// Sub-portion of `writes_external` from fresh-id creation / `LastCreated`
    /// (§2 rule 1). Fresh ObjectIds cannot be a sibling's source.
    writes_created: KindSet,
    /// CR 603.10a/B1: an EXTERNAL move of EXISTING objects whose destination is
    /// the battlefield or whose origin is exile — re-enters/overwrites a
    /// departed member's LKI. Feeds the freeze-invalidation row.
    writes_reentry_hazard: bool,
    /// A `SetMembership` write whose census IS the source's typeline, resolved
    /// against `source_census` at conflict time: either a self-scoped move
    /// (`ChangeZone{SelfRef}`) or a `CopyTokenOf{SelfRef}` created copy (CR 707.2,
    /// §1.3.1-F). `membership_census_of` merges `source_census` when this is set.
    writes_membership_self: bool,
    /// Census of external/creation/event-object `SetMembership` writes.
    writes_membership_external_census: Census,
    /// CR 400.1: zones the external/creation/event-object `SetMembership` writes
    /// touch (ZONE axis of the membership feed row — a battlefield creation vs a
    /// graveyard read is zone-disjoint, Tombstone Stairwell).
    writes_membership_external_zones: ZoneSpan,
    /// Census requirements of all `SetMembership` reads.
    reads_membership_census: Census,
    /// CR 400.1: zones all `SetMembership` reads observe (from their filter's
    /// `InZone`; `Any` when unextractable — fail-closed).
    reads_membership_zones: ZoneSpan,
    /// CR 122.1: census of EXTERNAL `ObjectCounters` writes' target filters —
    /// object-scope disjointness for the source-scoped counter read (§2; a quest
    /// read on an enchantment source × a +1/+1 write on creatures is
    /// object-disjoint, Earthbender Ascension). `Any` when unrefined (fail-closed).
    writes_external_counter_census: Census,
    /// A resolution-time payment (`unless_pay` / `PayCost`) is present (CR
    /// 603.5). With `writes_pool`, trips the Mana×unless-pay guard.
    has_pay_or_unless: bool,
    /// The ability writes the mana pool (`Effect::Mana`).
    writes_pool: bool,
}

impl RwProfile {
    fn empty() -> RwProfile {
        RwProfile {
            reads_src: KindSet::EMPTY,
            reads_board: KindSet::EMPTY,
            reads_player: KindSet::EMPTY,
            reads_frozen: KindSet::EMPTY,
            reads_event_live: false,
            legacy_batch_prompt: false,
            writes_self: KindSet::EMPTY,
            writes_external: KindSet::EMPTY,
            writes_event_object: KindSet::EMPTY,
            writes_created: KindSet::EMPTY,
            writes_reentry_hazard: false,
            writes_membership_self: false,
            writes_membership_external_census: Census::None,
            writes_membership_external_zones: ZoneSpan::None,
            reads_membership_census: Census::None,
            reads_membership_zones: ZoneSpan::None,
            writes_external_counter_census: Census::None,
            has_pay_or_unless: false,
            writes_pool: false,
        }
    }

    /// Fail-closed maximal profile for untraversed / unclassified subtrees.
    fn conservative() -> RwProfile {
        let mut p = RwProfile::empty();
        p.reads_board = KindSet::ALL;
        p.writes_self = KindSet::ALL;
        p.writes_external = KindSet::ALL;
        p.writes_membership_external_census = Census::Any;
        p.writes_membership_external_zones = ZoneSpan::Any;
        p.writes_membership_self = true;
        p.reads_membership_census = Census::Any;
        p.reads_membership_zones = ZoneSpan::Any;
        p.writes_external_counter_census = Census::Any;
        p
    }

    pub(crate) fn merge(&mut self, o: RwProfile) {
        self.reads_src = self.reads_src.union(o.reads_src);
        self.reads_board = self.reads_board.union(o.reads_board);
        self.reads_player = self.reads_player.union(o.reads_player);
        self.reads_frozen = self.reads_frozen.union(o.reads_frozen);
        self.reads_event_live |= o.reads_event_live;
        self.legacy_batch_prompt |= o.legacy_batch_prompt;
        self.writes_self = self.writes_self.union(o.writes_self);
        self.writes_external = self.writes_external.union(o.writes_external);
        self.writes_event_object = self.writes_event_object.union(o.writes_event_object);
        self.writes_created = self.writes_created.union(o.writes_created);
        self.writes_reentry_hazard |= o.writes_reentry_hazard;
        self.writes_membership_self |= o.writes_membership_self;
        self.writes_membership_external_census
            .merge(o.writes_membership_external_census);
        self.writes_membership_external_zones
            .merge(o.writes_membership_external_zones);
        self.reads_membership_census
            .merge(o.reads_membership_census);
        self.reads_membership_zones.merge(o.reads_membership_zones);
        self.writes_external_counter_census
            .merge(o.writes_external_counter_census);
        self.has_pay_or_unless |= o.has_pay_or_unless;
        self.writes_pool |= o.writes_pool;
    }

    /// CR 603.3b T1 (§1.2): the resolution function never consults the source
    /// binding — no source read, no self write, no source-referential frozen
    /// read. Fail-closed (the walk routes source predicates into `reads_src` /
    /// `reads_frozen`).
    pub(crate) fn source_independent(&self) -> bool {
        self.reads_src.is_empty() && self.writes_self.is_empty() && self.reads_frozen.is_empty()
    }

    /// D5 (CR 603.10a): true iff one of the 12 retained-prompt event-context refs
    /// is present. Consulted ONLY by the batch branch (`batch_conflict`) to keep
    /// the legacy departure-batch prompting parity (D3 zero widening).
    pub(crate) fn legacy_batch_prompt(&self) -> bool {
        self.legacy_batch_prompt
    }

    /// Drop all writes (deferred-body descent, CR 603.7: writes happen
    /// post-window, so reads descend but writes are not counted).
    fn drop_writes(&mut self) {
        self.writes_self = KindSet::EMPTY;
        self.writes_external = KindSet::EMPTY;
        self.writes_event_object = KindSet::EMPTY;
        self.writes_created = KindSet::EMPTY;
        self.writes_reentry_hazard = false;
        self.writes_membership_self = false;
        self.writes_membership_external_census = Census::None;
        self.writes_membership_external_zones = ZoneSpan::None;
        self.writes_external_counter_census = Census::None;
        self.writes_pool = false;
    }
}

// ---------------------------------------------------------------------------
// GroupStructure.
// ---------------------------------------------------------------------------

pub(crate) struct GroupStructure {
    /// All members fired on ONE trigger event.
    pub(crate) same_event: bool,
    /// All members share one `source_id`.
    pub(crate) all_same_source: bool,
    /// Every member's trigger is a ZoneChanged-from-battlefield whose object is
    /// that member's own source (departure batch — CR 603.10a frozen reads).
    pub(crate) all_sources_self_departed: bool,
    /// The shared `valid_card` filter provably excludes every member's source
    /// (extractable `Another`/not-self, §2 rule 2).
    pub(crate) event_object_excludes_sources: bool,
    /// The group's trigger event carries an event object (ZoneChanged / a source
    /// event). FALSE for `Phase` triggers ⇒ a `TriggeringSource` / parentless
    /// `ParentTarget` write resolves to None and is a no-op (targeting.rs:951
    /// `_ => None`; bounce.rs empty-target no-op — §2 rule 1 parentless clause).
    /// Beyond the plan's five listed fields; required to mechanize the
    /// resolver-referent pin the context-free profile cannot express.
    pub(crate) event_object_present: bool,
    /// The live source census, for the membership census-overlap row (§2).
    pub(crate) source_census: SourceCensus,
}

// ---------------------------------------------------------------------------
// feeds matrix (§2).
// ---------------------------------------------------------------------------

/// CR 603.3b feed matrix: same-kind rows + the cross rows (`ObjectCounters →
/// ObjectPt`, `PlayerLife → JournalLife`, `HandLibrary → JournalCards`), with the
/// `SetMembership` same-kind row refined by census overlap.
/// `SetMembership → aggregate {ObjectPt,ObjectCounters}` needs no explicit row —
/// aggregate reads already carry a `SetMembership` tag. `Other` conflicts with
/// everything. `PlayerLife → SetMembership` is deliberately ABSENT (the CR 800.4a
/// player-loss cascade is the documented source-actor residual, §1.2).
fn feeds(
    reads: KindSet,
    writes: KindSet,
    read_census: &Census,
    write_census: &Census,
    read_zones: &ZoneSpan,
    write_zones: &ZoneSpan,
) -> bool {
    if (writes.other && reads.any()) || (reads.other && writes.any()) {
        return true;
    }
    if (reads.object_pt && writes.object_pt)
        || (reads.object_counters && writes.object_counters)
        || (reads.player_life && writes.player_life)
        || (reads.hand_library && writes.hand_library)
        || (reads.journal_life && writes.journal_life)
        || (reads.journal_cards && writes.journal_cards)
        || (reads.journal_cast && writes.journal_cast)
        || (reads.stack_shape && writes.stack_shape)
        || (reads.tap_state && writes.tap_state)
    {
        return true;
    }
    // CR 122.1 + CR 613.4: counters change P/T.
    if reads.object_pt && writes.object_counters {
        return true;
    }
    // CR 119.3: a life write feeds a life-change journal read.
    if reads.journal_life && writes.player_life {
        return true;
    }
    // CR 120.6: a hand/library write feeds a draw/discard journal read.
    if reads.journal_cards && writes.hand_library {
        return true;
    }
    // SetMembership same-kind, census- AND zone-refined (§2; CR 205 type tags +
    // CR 400.1 zones). A battlefield token creation cannot feed a graveyard read
    // even though both name "creature" — their zones are disjoint (Tombstone).
    if reads.set_membership
        && writes.set_membership
        && census_overlap(read_census, write_census)
        && zone_overlap(read_zones, write_zones)
    {
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// The conflict predicate (§1.2 pseudocode, implemented exactly).
// ---------------------------------------------------------------------------

/// CR 603.3b: do two normalized-identical siblings' resolution functions FAIL to
/// commute under the given group structure? Sound modulo the source-actor
/// residual (module doc). Fail-closed.
pub(crate) fn profiles_conflict(p: &RwProfile, s: &GroupStructure) -> bool {
    // Mana×unless-pay guard (D4-R1, CR 603.5): pool write + unless-pay/pay-cost
    // breaks the identical-choice-set symmetry. 0 printed co-occurrences.
    if p.writes_pool && p.has_pay_or_unless {
        return true;
    }
    // T1 identical-function fast paths (§1.2), sound modulo the source-actor
    // residual.
    if s.same_event && (s.all_same_source || p.source_independent()) {
        return false;
    }
    if s.all_same_source && !p.reads_event_live {
        return false;
    }

    let freeze_valid = !p.writes_reentry_hazard;
    let frozen_src = s.all_sources_self_departed && freeze_valid;
    let live_src_reads = if frozen_src {
        KindSet::EMPTY
    } else {
        p.reads_src
    };

    // Event-object writes resolve to None (no-op) when the trigger carries no
    // event object (§2 rule 1 parentless clause; targeting.rs:951).
    let effective_external = if s.event_object_present {
        p.writes_external
    } else {
        p.writes_external.minus(p.writes_event_object)
    };

    // Freeze-invalidation row (B1) — a DIRECT conjunct on the batch path.
    if !s.same_event
        && !freeze_valid
        && (p.reads_frozen.any() || p.reads_src.any() || p.reads_event_live)
    {
        return true;
    }

    // SRC-read sibling writes: external existing objects, plus self only when
    // all members share one source; event-object writes excluded under
    // object-disjointness; created/LastCreated writes always excluded.
    let mut src_writes = effective_external.minus(p.writes_created);
    if s.same_event && s.event_object_excludes_sources && s.event_object_present {
        src_writes = src_writes.minus(p.writes_event_object);
    }
    // CR 122.1 object-scope disjointness (§2 Earthbender): an EXTERNAL counter
    // write feeds a SOURCE-scoped counter read only if the write filter can match
    // the source (census overlap; fail-closed — an unrefined counter write is
    // `Any`, and `None` here means no external counter write). A quest read on an
    // enchantment source × a +1/+1 write on creatures is object-disjoint ⇒ drop
    // `ObjectCounters` from the source-read feed. The same-source SELF counter
    // write is added AFTER this gate (a self write on the shared source DOES feed).
    if p.reads_src.object_counters && src_writes.object_counters {
        let ext_counter_census = if matches!(p.writes_external_counter_census, Census::None) {
            Census::Any // membership present but census unrecorded ⇒ fail-closed
        } else {
            p.writes_external_counter_census.clone()
        };
        if !census_overlap(&s.source_census.as_census(), &ext_counter_census) {
            src_writes = src_writes.minus(KindSet::one(StateKind::ObjectCounters));
        }
    }
    let src_self_membership = s.all_same_source && p.writes_membership_self;
    let src_write_census = membership_census_of(
        src_writes,
        &p.writes_membership_external_census,
        src_self_membership,
        s,
    );
    let src_write_zones = membership_zones_of(
        src_writes,
        &p.writes_membership_external_zones,
        src_self_membership,
    );
    if s.all_same_source {
        src_writes = src_writes.union(p.writes_self);
    }
    if feeds(
        live_src_reads,
        src_writes,
        &p.reads_membership_census,
        &src_write_census,
        &p.reads_membership_zones,
        &src_write_zones,
    ) {
        return true;
    }

    // BOARD/PLAYER-read sibling writes: everything (incl. self).
    let board_writes = effective_external.union(p.writes_self);
    let board_write_census = membership_census_of(
        board_writes,
        &p.writes_membership_external_census,
        p.writes_membership_self,
        s,
    );
    let board_write_zones = membership_zones_of(
        board_writes,
        &p.writes_membership_external_zones,
        p.writes_membership_self,
    );
    let board_reads = p.reads_board.union(p.reads_player);
    if feeds(
        board_reads,
        board_writes,
        &p.reads_membership_census,
        &board_write_census,
        &p.reads_membership_zones,
        &board_write_zones,
    ) {
        return true;
    }

    false
}

/// The membership-write census applying to an effective write set: external
/// census if any external membership write is present, unioned with the source
/// census if a self membership write is in scope.
fn membership_census_of(
    write_kinds: KindSet,
    external_census: &Census,
    self_in_scope: bool,
    s: &GroupStructure,
) -> Census {
    let mut c = Census::None;
    if write_kinds.set_membership {
        c.merge(external_census.clone());
    }
    if self_in_scope {
        c.merge(s.source_census.as_census());
    }
    c
}

/// CR 400.1: the membership-write ZONE span for an effective write set — the
/// external/creation zones if any external membership write is present, plus
/// `Any` (fail-closed) when a self membership move is in scope (its endpoints are
/// untracked). Mirrors `membership_census_of`.
fn membership_zones_of(
    write_kinds: KindSet,
    external_zones: &ZoneSpan,
    self_in_scope: bool,
) -> ZoneSpan {
    let mut z = ZoneSpan::None;
    if write_kinds.set_membership {
        z.merge(external_zones.clone());
    }
    if self_in_scope {
        z.merge(ZoneSpan::Any);
    }
    z
}

// ---------------------------------------------------------------------------
// Public entry points.
// ---------------------------------------------------------------------------

/// Profile a resolved ability's reads/writes (CR 603.3b). A top-level
/// `ParentTarget` is parentless (§2 rule 1) ⇒ chain-root context starts empty.
pub(crate) fn ability_rw_profile(a: &ResolvedAbility) -> RwProfile {
    let mut p = RwProfile::empty();
    walk_ability(a, None, &mut p);
    p
}

/// Profile a bare trigger-level `condition` (CR 603.4 intervening-if — re-checked
/// at resolution, so its reads are order-relevant).
pub(crate) fn trigger_condition_rw_profile(c: &TriggerCondition) -> RwProfile {
    rw_trigger_condition(c)
}

// ---------------------------------------------------------------------------
// Chain-root scope for anaphoric `ParentTarget` writes (§2 rule 1).
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WriteScope {
    /// The member's own source (SelfRef / SourceOrPaired).
    SelfSource,
    /// An existing external object / player / board.
    External,
    /// The event object (`TriggeringSource`-class + parentless `ParentTarget`).
    EventObject,
    /// A fresh-id creation / LastCreated object.
    Created,
}

/// CR 603.4 + CR 608.2c: the write scope of an effect target. `ParentTarget`
/// resolves to the CHAIN ROOT (`nearest object-referent ancestor`,
/// filter.rs:3063-3085); a PARENTLESS `ParentTarget` resolves to the EVENT object
/// on a ZoneChanged trigger (targeting.rs:946-950) — represented as `EventObject`,
/// which `profiles_conflict` drops when the trigger carries no event object.
/// Exhaustive & wildcard-free: a future `TargetFilter` variant must be classified.
fn scope_of(target: &TargetFilter, chain_root: Option<WriteScope>) -> WriteScope {
    match target {
        TargetFilter::SelfRef | TargetFilter::SourceOrPaired => WriteScope::SelfSource,
        TargetFilter::TriggeringSource => WriteScope::EventObject,
        TargetFilter::ParentTarget | TargetFilter::ParentTargetSlot { .. } => {
            chain_root.unwrap_or(WriteScope::EventObject)
        }
        TargetFilter::LastCreated => WriteScope::Created,
        TargetFilter::None
        | TargetFilter::Any
        | TargetFilter::Player
        | TargetFilter::Controller
        | TargetFilter::Typed(..)
        | TargetFilter::Not { .. }
        | TargetFilter::Or { .. }
        | TargetFilter::And { .. }
        | TargetFilter::StackAbility { .. }
        | TargetFilter::StackSpell
        | TargetFilter::SpecificObject { .. }
        | TargetFilter::SpecificPlayer { .. }
        | TargetFilter::Neighbor { .. }
        | TargetFilter::ScopedPlayer
        | TargetFilter::AttachedTo
        | TargetFilter::LastRevealed
        | TargetFilter::CostPaidObject
        | TargetFilter::ChosenCard
        | TargetFilter::TrackedSet { .. }
        | TargetFilter::TrackedSetFiltered { .. }
        | TargetFilter::ExiledBySource
        | TargetFilter::ExiledCardByIndex { .. }
        | TargetFilter::TriggeringSpellController
        | TargetFilter::TriggeringSpellOwner
        | TargetFilter::TriggeringPlayer
        | TargetFilter::EventTarget
        | TargetFilter::TriggeringSourceController
        | TargetFilter::ParentTargetController
        | TargetFilter::ParentTargetOwner
        | TargetFilter::SourceChosenPlayer
        | TargetFilter::OriginalController
        | TargetFilter::PostReplacementSourceController
        | TargetFilter::PostReplacementDamageTarget
        | TargetFilter::PostReplacementDamageTargetOwner
        | TargetFilter::DefendingPlayer
        | TargetFilter::HasChosenName
        | TargetFilter::ChosenDamageSource
        | TargetFilter::Named { .. }
        | TargetFilter::Owner
        | TargetFilter::AllPlayers => WriteScope::External,
    }
}

/// Place a non-membership object write (`ObjectPt`/`ObjectCounters`/`TapState`/
/// `Other`) by scope.
fn place_object_write(p: &mut RwProfile, kind: StateKind, sc: WriteScope) {
    match sc {
        WriteScope::SelfSource => p.writes_self.set(kind),
        WriteScope::External => p.writes_external.set(kind),
        WriteScope::EventObject => {
            p.writes_external.set(kind);
            p.writes_event_object.set(kind);
        }
        WriteScope::Created => {
            p.writes_external.set(kind);
            p.writes_created.set(kind);
        }
    }
}

/// Place a `SetMembership` (zone/control) write with its census + reentry-hazard
/// (CR 603.10a/B1) + hand/library endpoint tagging.
fn place_membership_write(
    p: &mut RwProfile,
    sc: WriteScope,
    census: Census,
    origin: Option<Zone>,
    dest: Zone,
) {
    let hazard = matches!(sc, WriteScope::External)
        && (dest == Zone::Battlefield || origin == Some(Zone::Exile))
        && origin != Some(Zone::Library);
    // CR 400.1: a MOVE touches both endpoints (origin removal + dest addition), so
    // a graveyard-count read IS fed by a graveyard→battlefield return.
    let mut move_zones = ZoneSpan::one(dest);
    if let Some(o) = origin {
        move_zones.merge(ZoneSpan::one(o));
    }
    match sc {
        WriteScope::SelfSource => {
            p.writes_self.set(StateKind::SetMembership);
            p.writes_membership_self = true;
        }
        WriteScope::External => {
            p.writes_external.set(StateKind::SetMembership);
            p.writes_membership_external_census.merge(census);
            p.writes_membership_external_zones.merge(move_zones);
            p.writes_reentry_hazard |= hazard;
        }
        WriteScope::EventObject => {
            p.writes_external.set(StateKind::SetMembership);
            p.writes_event_object.set(StateKind::SetMembership);
            // Event object identity is unknown at profile time ⇒ census + zone Any
            // (the precise `move_zones` is used only by the External/Created arms).
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.writes_reentry_hazard |= hazard;
        }
        WriteScope::Created => {
            p.writes_external.set(StateKind::SetMembership);
            p.writes_created.set(StateKind::SetMembership);
            p.writes_membership_external_census.merge(census);
            p.writes_membership_external_zones.merge(move_zones);
        }
    }
    if is_hand_or_library(dest) || origin.is_some_and(is_hand_or_library) {
        p.writes_external.set(StateKind::HandLibrary);
    }
}

fn is_hand_or_library(z: Zone) -> bool {
    matches!(z, Zone::Hand | Zone::Library)
}

/// CR 205: extract a type-tag census from a filter used as a read/write selector.
/// `Typed` yields its core-type + subtype + token-ness tags; `Not`/broad filters
/// ⇒ `Any` (fail-closed); `And`/`Or` union their components.
fn census_of_filter(f: &TargetFilter) -> Census {
    match f {
        TargetFilter::Typed(tf) => census_of_typed(tf),
        TargetFilter::And { filters } | TargetFilter::Or { filters } => {
            let mut c = Census::None;
            for x in filters {
                c.merge(census_of_filter(x));
            }
            if matches!(c, Census::None) {
                Census::Any
            } else {
                c
            }
        }
        _ => Census::Any,
    }
}

fn census_of_typed(tf: &TypedFilter) -> Census {
    let mut tags: BTreeSet<String> = BTreeSet::new();
    for t in &tf.type_filters {
        match t {
            TypeFilter::Creature => tags.insert("creature".into()),
            TypeFilter::Land => tags.insert("land".into()),
            TypeFilter::Artifact => tags.insert("artifact".into()),
            TypeFilter::Enchantment => tags.insert("enchantment".into()),
            TypeFilter::Instant => tags.insert("instant".into()),
            TypeFilter::Sorcery => tags.insert("sorcery".into()),
            TypeFilter::Planeswalker => tags.insert("planeswalker".into()),
            TypeFilter::Battle => tags.insert("battle".into()),
            TypeFilter::Kindred => tags.insert("kindred".into()),
            TypeFilter::Subtype(s) => tags.insert(s.to_lowercase()),
            // Broad / non-positive / disjunctive type constraints ⇒ unextractable.
            TypeFilter::Permanent | TypeFilter::Card | TypeFilter::Any => return Census::Any,
            TypeFilter::Non(_) | TypeFilter::AnyOf(_) => return Census::Any,
        };
    }
    for prop in &tf.properties {
        match prop {
            FilterProp::Token => {
                tags.insert("token".into());
            }
            FilterProp::NonToken => {
                tags.insert("nontoken".into());
            }
            _ => {}
        }
    }
    if tags.is_empty() {
        Census::Any
    } else {
        Census::Tags(tags)
    }
}

/// A read filter that provably counts only the member's own source (a `SelfRef`
/// conjunct) — routes to per-member-private `reads_src` (§2 read-carrier closure).
fn filter_is_self_scoped(f: &TargetFilter) -> bool {
    match f {
        TargetFilter::SelfRef => true,
        TargetFilter::And { filters } => filters.iter().any(filter_is_self_scoped),
        _ => false,
    }
}

/// CR 111.1: the filter's `valid_card` provably excludes every group member's
/// source (an `Another` component) — the object-disjointness signal (§2 rule 2).
/// Consumed by the parity sweep's STATIC event-object-disjointness model
/// (`triggers_ordering_parity_tests`). The production chokepoint
/// (`group_is_order_independent`) instead uses the DYNAMIC id-disjointness check
/// (the event object's id vs the members' `source_id`s) because `valid_card` is
/// not carried on `PendingTrigger`; the dynamic check is a sound, at-least-as-
/// precise witness of the same relation. `#[allow(dead_code)]` covers the
/// non-test lib build where only the sweep (a `#[cfg(test)]` consumer) calls it.
#[allow(dead_code)]
pub(crate) fn filter_excludes_source(f: &TargetFilter) -> bool {
    match f {
        TargetFilter::Typed(tf) => tf
            .properties
            .iter()
            .any(|p| matches!(p, FilterProp::Another)),
        TargetFilter::And { filters } => filters.iter().any(filter_excludes_source),
        _ => false,
    }
}

/// CR 205: does a printed source's type census overlap a filter's type census
/// (fail-closed — either side unextractable ⇒ `Any` ⇒ overlap)? Used by the
/// parity sweep's condition-based reachability guard (§1.3.1-F): a same-event
/// 2-copy group is unreachable when the source itself matches an
/// `Another`-self-exclusion count the intervening-if requires to be zero
/// (Thopter Assembly). `#[allow(dead_code)]` covers the non-test lib build where
/// only the sweep (a `#[cfg(test)]` consumer) calls it.
#[allow(dead_code)]
pub(crate) fn source_census_overlaps_filter(s: &SourceCensus, f: &TargetFilter) -> bool {
    census_overlap(&s.as_census(), &census_of_filter(f))
}

/// D5 (CR 603.10a): the 9 `TargetFilter` carriers of the 12 retained-prompt
/// event-context refs (the other 3 — `EventContextAmount`,
/// `EventContextSourceCostX`, `ManaSpentToCast` — are `QuantityRef`s, handled by
/// the read path). The frozen serde oracle
/// (`value_contains_trigger_event_context_ref`) matched these tags ANYWHERE in
/// the serialized ability — read OR write position — so a tag as an effect WRITE
/// TARGET must also set `legacy_batch_prompt` for the batch branch to retain its
/// prompt (D3 zero-widening). Each of the 9 is a unit variant serializing to a
/// bare string, exactly what the oracle matched; the struct-variant
/// `ParentTargetSlot` is deliberately EXCLUDED (it serializes as an object key,
/// which the oracle's value-walk never matches). Composite filters are descended
/// so a nested tag is still caught (position-agnostic like the oracle).
fn target_is_legacy_ref(f: &TargetFilter) -> bool {
    match f {
        TargetFilter::TriggeringSpellController
        | TargetFilter::TriggeringSpellOwner
        | TargetFilter::TriggeringPlayer
        | TargetFilter::TriggeringSource
        | TargetFilter::ParentTarget
        | TargetFilter::ParentTargetController
        | TargetFilter::ParentTargetOwner
        | TargetFilter::StackSpell
        | TargetFilter::CostPaidObject => true,
        TargetFilter::Not { filter } => target_is_legacy_ref(filter),
        TargetFilter::And { filters } | TargetFilter::Or { filters } => {
            filters.iter().any(target_is_legacy_ref)
        }
        _ => false,
    }
}

/// Set `legacy_batch_prompt` when an effect write target carries a D5 event-
/// context ref (§D5, CR 603.10a) — mirrors the read-carrier path
/// (`rw_target_filter` → `legacy_ref`) for write position. Position-agnostic:
/// applied at every write-target-bearing arm so the tag anywhere ⇒ prompt.
fn flag_legacy_write_target(p: &mut RwProfile, target: &TargetFilter) {
    if target_is_legacy_ref(target) {
        p.legacy_batch_prompt = true;
    }
}

// ---------------------------------------------------------------------------
// Read builders.
// ---------------------------------------------------------------------------

fn reads_board_of(k: StateKind) -> RwProfile {
    let mut p = RwProfile::empty();
    p.reads_board = KindSet::one(k);
    p
}
/// CR 400.1 + CR 205: a whole-zone / unfiltered `SetMembership` board read
/// (GraveyardSize, Devotion, …) over a zone's contents; the census is a
/// card-type tag-set (CR 205). The filter is fully unextractable, so its census is `Census::Any`
/// (§2: unextractable ⇒ overlap assumed, fail-CLOSED). Must NOT use
/// `reads_board_of(SetMembership)`, which leaves `Census::None` (the write-side
/// "no object moved" sentinel) and would make the membership feed row never fire.
fn reads_zone_membership() -> RwProfile {
    let mut p = reads_board_of(StateKind::SetMembership);
    p.reads_membership_census = Census::Any;
    // CR 400.1: a whole-zone read's zone is not extractable here (GraveyardSize
    // = graveyard, Devotion = battlefield, … — one helper, many zones) ⇒ `Any`,
    // fail-closed (conflicts with every membership write, as before).
    p.reads_membership_zones = ZoneSpan::Any;
    p
}
fn reads_player_of(k: StateKind) -> RwProfile {
    let mut p = RwProfile::empty();
    p.reads_player = KindSet::one(k);
    p
}
fn reads_src_of(k: StateKind) -> RwProfile {
    let mut p = RwProfile::empty();
    p.reads_src = KindSet::one(k);
    p
}
/// CR 603.10a: a source-referential look-back / cast-time fact — frozen, never
/// sibling-fed, but marks source-dependence (`source_independent` false).
fn frozen_source_read() -> RwProfile {
    let mut p = RwProfile::empty();
    p.reads_frozen = KindSet::one(StateKind::SetMembership);
    p
}
fn reads_frozen_of(k: StateKind) -> RwProfile {
    let mut p = RwProfile::empty();
    p.reads_frozen = KindSet::one(k);
    p
}
fn reads_event_live() -> RwProfile {
    let mut p = RwProfile::empty();
    p.reads_event_live = true;
    p
}
fn legacy_ref() -> RwProfile {
    let mut p = reads_event_live();
    p.legacy_batch_prompt = true;
    p
}
fn writes_pool_profile() -> RwProfile {
    let mut p = RwProfile::empty();
    p.writes_pool = true;
    p
}
fn ext_write(k: StateKind) -> RwProfile {
    let mut p = RwProfile::empty();
    p.writes_external.set(k);
    p
}
fn self_write(k: StateKind) -> RwProfile {
    let mut p = RwProfile::empty();
    p.writes_self.set(k);
    p
}

/// A board aggregate read over `filter`: `reads_board{SetMembership}` (with
/// census) unless the filter is provably self-scoped ⇒ per-member-private
/// `reads_src` (§2).
fn board_membership_read(filter: &TargetFilter) -> RwProfile {
    let mut p = RwProfile::empty();
    if filter_is_self_scoped(filter) {
        p.reads_src = KindSet::one(StateKind::SetMembership);
    } else {
        p.reads_board = KindSet::one(StateKind::SetMembership);
    }
    p.reads_membership_census = census_of_filter(filter);
    // CR 400.1: the zone(s) the read counts, from the filter's explicit `InZone`
    // (Tombstone Stairwell's Zombie count reads the GRAVEYARD, not the battlefield
    // its own tokens enter). No `InZone` ⇒ `Any` (fail-closed, unchanged behavior).
    p.reads_membership_zones = zones_of_filter(filter);
    p
}

/// A board VALUE aggregate (power/counter aggregate) over `filter`: records the
/// value kind AND `SetMembership` (a membership write changes the aggregate, §2).
fn board_value_aggregate_read(filter: &TargetFilter, value: StateKind) -> RwProfile {
    let mut p = board_membership_read(filter);
    if filter_is_self_scoped(filter) {
        p.reads_src.set(value);
    } else {
        p.reads_board.set(value);
    }
    p
}

/// Read an object characteristic at a given scope (§2 read-carrier closure):
/// Source ⇒ `reads_src`; Recipient ⇒ nothing (read-modify-write); event objects
/// ⇒ `reads_event_live`; other object scopes ⇒ `reads_board`.
fn read_object_scope(scope: &ObjectScope, kind: StateKind) -> RwProfile {
    match scope {
        ObjectScope::Source => reads_src_of(kind),
        ObjectScope::Recipient => RwProfile::empty(),
        ObjectScope::Target | ObjectScope::Anaphoric | ObjectScope::Demonstrative => {
            reads_board_of(kind)
        }
        ObjectScope::EventSource | ObjectScope::EventTarget => reads_event_live(),
        // D5 carrier: `CostPaidObject` is one of the 12 retained refs.
        ObjectScope::CostPaidObject => legacy_ref(),
    }
}

/// (player_recipient, object_recipient) for a damage/target filter — recipient
/// classification (CR 704.5a / CR 800.4a source-actor residual documented in the
/// module doc; damage kinds are recipient-classified, not source-bound).
fn target_recipient(f: &TargetFilter) -> (bool, bool) {
    match f {
        TargetFilter::Player
        | TargetFilter::Controller
        | TargetFilter::Owner
        | TargetFilter::AllPlayers
        | TargetFilter::DefendingPlayer
        | TargetFilter::SpecificPlayer { .. }
        | TargetFilter::ScopedPlayer
        | TargetFilter::SourceChosenPlayer
        | TargetFilter::OriginalController
        | TargetFilter::TriggeringPlayer
        | TargetFilter::TriggeringSpellController
        | TargetFilter::TriggeringSpellOwner
        | TargetFilter::TriggeringSourceController
        | TargetFilter::ParentTargetController
        | TargetFilter::ParentTargetOwner
        | TargetFilter::PostReplacementSourceController
        | TargetFilter::PostReplacementDamageTargetOwner => (true, false),
        // "any target" and player-or-object filters ⇒ both recipients.
        TargetFilter::Any | TargetFilter::Or { .. } => (true, true),
        // Everything else is object-scoped for damage purposes.
        _ => (false, true),
    }
}

// ---------------------------------------------------------------------------
// Ability walk (mirrors `resolved_ability_axes`, with chain-root threading).
// ---------------------------------------------------------------------------

fn walk_ability(a: &ResolvedAbility, chain_root: Option<WriteScope>, acc: &mut RwProfile) {
    let ResolvedAbility {
        effect,
        sub_ability,
        else_ability,
        condition,
        duration,
        player_scope,
        starting_with,
        repeat_for,
        multi_target,
        target_constraints,
        unless_pay,
        target_chooser,
        repeat_until,
        modal,
        mode_abilities,
        targets: _,
        source_id: _,
        source_incarnation: _,
        controller: _,
        original_controller: _,
        scoped_player: _,
        kind: _,
        context: _,
        optional_targeting: _,
        optional: _,
        optional_for: _,
        target_choice_timing: _,
        description: _,
        min_x_value: _,
        cant_be_copied: _,
        copy_count_status: _,
        forward_result: _,
        distribution: _,
        chosen_x: _,
        cost_paid_object: _,
        effect_context_object: _,
        ability_index: _,
        may_trigger_origin: _,
        target_selection_mode: _,
        chosen_players: _,
        sub_link: _,
        dig_found_nothing_for_parent_target: _,
    } = a;

    let (eff, own_scope) = rw_effect(effect, chain_root);
    acc.merge(eff);
    let child_root = own_scope.or(chain_root);

    if let Some(sub) = sub_ability {
        walk_ability(sub, child_root, acc);
    }
    if let Some(els) = else_ability {
        walk_ability(els, chain_root, acc);
    }
    if let Some(c) = condition {
        acc.merge(rw_ability_condition(c));
    }
    if let Some(d) = duration {
        acc.merge(rw_duration(d));
    }
    if let Some(ps) = player_scope {
        acc.merge(rw_player_filter(ps));
    }
    if let Some(sw) = starting_with {
        acc.merge(rw_controller_ref(sw));
    }
    if let Some(rf) = repeat_for {
        acc.merge(rw_quantity_expr(rf));
    }
    if let Some(MultiTargetSpec { min, max }) = multi_target {
        acc.merge(rw_quantity_expr(min));
        if let Some(max) = max {
            acc.merge(rw_quantity_expr(max));
        }
    }
    for c in target_constraints {
        acc.merge(rw_target_constraint(c));
    }
    // CR 603.5: unless-pay is a resolution-time choice — no read-kind, but arms
    // the Mana×unless-pay guard.
    if unless_pay.is_some() {
        acc.has_pay_or_unless = true;
    }
    if let Some(tc) = target_chooser {
        acc.merge(rw_target_filter(tc));
    }
    if let Some(ru) = repeat_until {
        acc.merge(rw_repeat_continuation(ru));
    }
    if let Some(m) = modal {
        acc.merge(rw_modal_choice(m));
    }
    // CR 700.2b: reflexive-modal per-mode defs are not descended — conservative.
    if !mode_abilities.is_empty() {
        acc.merge(RwProfile::conservative());
    }
}

/// Descend a choice/RNG sub-body (`AbilityDefinition`). `..`-free so a future
/// field forces a decision (§2 choice-wrapper / RNG union descent).
fn walk_definition(a: &AbilityDefinition, chain_root: Option<WriteScope>, acc: &mut RwProfile) {
    let AbilityDefinition {
        effect,
        sub_ability,
        else_ability,
        condition,
        duration,
        player_scope,
        starting_with,
        repeat_for,
        multi_target,
        target_constraints,
        unless_pay,
        modal,
        mode_abilities,
        target_chooser,
        repeat_until,
        kind: _,
        cost: _,
        description: _,
        target_prompt: _,
        activation_restrictions: _,
        activator_filter: _,
        activation_zone: _,
        ability_tag: _,
        optional_targeting: _,
        optional: _,
        optional_for: _,
        target_choice_timing: _,
        distribute: _,
        min_x_value: _,
        cant_be_copied: _,
        cost_reduction: _,
        forward_result: _,
        target_selection_mode: _,
        sub_link: _,
        iteration_kind_binding: _,
    } = a;

    let (eff, own_scope) = rw_effect(effect, chain_root);
    acc.merge(eff);
    let child_root = own_scope.or(chain_root);

    if let Some(sub) = sub_ability {
        walk_definition(sub, child_root, acc);
    }
    if let Some(els) = else_ability {
        walk_definition(els, chain_root, acc);
    }
    if let Some(c) = condition {
        acc.merge(rw_ability_condition(c));
    }
    if let Some(d) = duration {
        acc.merge(rw_duration(d));
    }
    if let Some(ps) = player_scope {
        acc.merge(rw_player_filter(ps));
    }
    if let Some(sw) = starting_with {
        acc.merge(rw_controller_ref(sw));
    }
    if let Some(rf) = repeat_for {
        acc.merge(rw_quantity_expr(rf));
    }
    if let Some(MultiTargetSpec { min, max }) = multi_target {
        acc.merge(rw_quantity_expr(min));
        if let Some(max) = max {
            acc.merge(rw_quantity_expr(max));
        }
    }
    for c in target_constraints {
        acc.merge(rw_target_constraint(c));
    }
    if unless_pay.is_some() {
        acc.has_pay_or_unless = true;
    }
    if let Some(tc) = target_chooser {
        acc.merge(rw_target_filter(tc));
    }
    if let Some(ru) = repeat_until {
        acc.merge(rw_repeat_continuation(ru));
    }
    if let Some(m) = modal {
        acc.merge(rw_modal_choice(m));
    }
    if !mode_abilities.is_empty() {
        acc.merge(RwProfile::conservative());
    }
}

fn rw_modal_choice(m: &ModalChoice) -> RwProfile {
    let ModalChoice {
        dynamic_max_choices,
        chooser,
        min_choices: _,
        max_choices: _,
        mode_count: _,
        mode_descriptions: _,
        allow_repeat_modes: _,
        constraints: _,
        mode_costs: _,
        mode_pawprints: _,
        entwine_cost: _,
        selection: _,
    } = m;
    let mut p = rw_player_filter(chooser);
    if let Some(q) = dynamic_max_choices {
        p.merge(rw_quantity_expr(q));
    }
    p
}

fn rw_repeat_continuation(r: &RepeatContinuation) -> RwProfile {
    match r {
        RepeatContinuation::ControllerChoice => RwProfile::empty(),
        RepeatContinuation::UntilStopConditions {
            stop_on_put_to_hand: _,
            stop_on_duplicate_exiled_names: _,
        } => RwProfile::empty(),
        RepeatContinuation::WhileCondition {
            condition,
            max_iterations: _,
        } => rw_ability_condition(condition),
    }
}

fn rw_target_constraint(c: &TargetSelectionConstraint) -> RwProfile {
    match c {
        TargetSelectionConstraint::DifferentTargetPlayers => RwProfile::empty(),
        TargetSelectionConstraint::DifferentObjectControllers => RwProfile::empty(),
        TargetSelectionConstraint::TotalManaValue {
            value,
            comparator: _,
        } => rw_quantity_expr(value),
    }
}

fn rw_duration(x: &Duration) -> RwProfile {
    match x {
        Duration::UntilEndOfTurn
        | Duration::UntilEndOfCombat
        | Duration::UntilHostLeavesPlay
        | Duration::Permanent => RwProfile::empty(),
        Duration::UntilNextTurnOf { player, .. }
        | Duration::UntilEndOfNextTurnOf { player, .. }
        | Duration::UntilNextStepOf { player, .. } => rw_player_scope(player),
        Duration::ForAsLongAs { condition } => rw_static_condition(condition),
    }
}

/// CR 119.3: a player-life write plus its life-change journal (CR 119.3).
fn life_writes() -> RwProfile {
    let mut p = RwProfile::empty();
    p.writes_external.set(StateKind::PlayerLife);
    p.writes_external.set(StateKind::JournalLife);
    p
}

/// CR 120 damage: recipient-classified writes (source-actor residual documented
/// in the module doc — CR 702.15 / CR 702.2 / CR 704.5a / CR 800.4a).
fn damage_writes(target: &TargetFilter) -> RwProfile {
    let (player, object) = target_recipient(target);
    let mut p = RwProfile::empty();
    if player {
        p.merge(life_writes());
    }
    if object {
        p.writes_external.set(StateKind::SetMembership);
        p.writes_membership_external_census
            .merge(census_of_filter(target));
        // CR 120.3e + CR 704.5g: lethal damage moves a creature battlefield →
        // graveyard as an SBA ⇒ both zones (fail-closed Any).
        p.writes_membership_external_zones.merge(ZoneSpan::Any);
    }
    flag_legacy_write_target(&mut p, target);
    p
}

// ---------------------------------------------------------------------------
// Effect classification (mirrors `scan_effect`; write-kind per §2 categories).
// Returns (profile, primary object-write scope for chain-root propagation).
// ---------------------------------------------------------------------------

fn rw_effect(x: &Effect, chain_root: Option<WriteScope>) -> (RwProfile, Option<WriteScope>) {
    // Object write of `kind` targeting `target`, placed by scope.
    let obj = |kind: StateKind, target: &TargetFilter| -> (RwProfile, Option<WriteScope>) {
        let sc = scope_of(target, chain_root);
        let mut p = RwProfile::empty();
        place_object_write(&mut p, kind, sc);
        // CR 122.1 object-scope disjointness (§2): record the census of an EXTERNAL
        // counter write's target filter, so a source-scoped counter read only
        // conflicts when the write filter can match the source (Earthbender: a
        // `+1/+1` write on creatures can't reach an enchantment source's quest
        // counter). Self/created writes are handled by their own scoping.
        if kind == StateKind::ObjectCounters
            && matches!(sc, WriteScope::External | WriteScope::EventObject)
        {
            p.writes_external_counter_census
                .merge(census_of_filter(target));
        }
        flag_legacy_write_target(&mut p, target);
        (p, Some(sc))
    };
    // Membership move targeting `target` with the given zone endpoints.
    let mem = |target: &TargetFilter,
               origin: Option<Zone>,
               dest: Zone|
     -> (RwProfile, Option<WriteScope>) {
        let sc = scope_of(target, chain_root);
        let mut p = RwProfile::empty();
        place_membership_write(&mut p, sc, census_of_filter(target), origin, dest);
        flag_legacy_write_target(&mut p, target);
        (p, Some(sc))
    };
    // Deferred body (CR 603.7): descend reads, drop writes.
    let deferred = |def: &AbilityDefinition| -> RwProfile {
        let mut p = RwProfile::empty();
        walk_definition(def, None, &mut p);
        p.drop_writes();
        p
    };

    match x {
        // ---- Damage (recipient-classified, CR 120) ----
        Effect::DealDamage {
            amount,
            target,
            damage_source: _,
            excess,
        } => {
            let mut p = damage_writes(target);
            p.merge(rw_quantity_expr(amount));
            // CR 120.4a: the excess-redirect rider deals overkill to the damaged
            // permanent's controller — a player-life write not captured by
            // object-recipient damage_writes.
            if excess.is_some() {
                p.merge(life_writes());
            }
            (p, None)
        }
        Effect::DamageAll {
            amount,
            target,
            player_filter,
            damage_source: _,
        } => {
            let mut p = RwProfile::empty();
            p.writes_external.set(StateKind::SetMembership);
            p.writes_membership_external_census
                .merge(census_of_filter(target));
            // CR 704.5g: SBA deaths move battlefield → graveyard (fail-closed Any).
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            flag_legacy_write_target(&mut p, target);
            if player_filter.is_some() {
                p.merge(life_writes());
            }
            p.merge(rw_quantity_expr(amount));
            (p, None)
        }
        Effect::DamageEachPlayer {
            amount,
            player_filter: _,
        } => {
            let mut p = life_writes();
            p.merge(rw_quantity_expr(amount));
            (p, None)
        }
        Effect::Fight { target, subject } => {
            let mut p = damage_writes(target);
            p.merge(damage_writes(subject));
            (p, None)
        }

        // ---- Hand / library ----
        Effect::Draw { count, target: _ } => {
            let mut p = ext_write(StateKind::HandLibrary);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::Discard {
            count,
            target: _,
            unless_filter: _,
            filter: _,
            selection: _,
        } => {
            let mut p = ext_write(StateKind::HandLibrary);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::DiscardCard {
            target: _,
            count: _,
        } => (ext_write(StateKind::HandLibrary), None),
        Effect::Scry { count, target: _ } => {
            let mut p = ext_write(StateKind::HandLibrary);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::Surveil { count, target: _ } => {
            let mut p = ext_write(StateKind::HandLibrary);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::Shuffle { target: _ } => (ext_write(StateKind::HandLibrary), None),
        Effect::PutAtLibraryPosition {
            target: _,
            count,
            position: _,
        } => {
            let mut p = ext_write(StateKind::HandLibrary);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::Learn => (ext_write(StateKind::HandLibrary), None),
        Effect::DraftFromSpellbook {
            destination: _,
            tapped: _,
        } => (ext_write(StateKind::HandLibrary), None),
        Effect::Clash => (ext_write(StateKind::HandLibrary), None),
        Effect::ChooseDrawnThisTurnPayOrTopdeck {
            count,
            life_payment,
            player: _,
        } => {
            let mut p = ext_write(StateKind::HandLibrary);
            p.merge(life_writes());
            p.merge(rw_quantity_expr(count));
            p.merge(rw_quantity_expr(life_payment));
            (p, None)
        }

        // ---- Life ----
        Effect::GainLife { amount, player } => {
            let mut p = life_writes();
            flag_legacy_write_target(&mut p, player);
            p.merge(rw_quantity_expr(amount));
            (p, None)
        }
        Effect::LoseLife { amount, target } => {
            let mut p = life_writes();
            if let Some(t) = target {
                flag_legacy_write_target(&mut p, t);
            }
            p.merge(rw_quantity_expr(amount));
            (p, None)
        }
        Effect::SetLifeTotal { target, amount } => {
            let mut p = life_writes();
            flag_legacy_write_target(&mut p, target);
            p.merge(rw_quantity_expr(amount));
            (p, None)
        }
        Effect::GainEnergy { amount } => {
            let mut p = ext_write(StateKind::PlayerLife);
            p.merge(rw_quantity_expr(amount));
            (p, None)
        }
        Effect::GivePlayerCounter {
            count,
            target,
            counter_kind: _,
        } => {
            let mut p = ext_write(StateKind::PlayerLife);
            flag_legacy_write_target(&mut p, target);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }

        // ---- Counters (ObjectCounters) ----
        Effect::PutCounter {
            count,
            target,
            counter_type: _,
        } => {
            let (mut p, sc) = obj(StateKind::ObjectCounters, target);
            p.merge(rw_quantity_expr(count));
            (p, sc)
        }
        Effect::PutCounterAll {
            count,
            target,
            counter_type: _,
        } => {
            let (mut p, sc) = obj(StateKind::ObjectCounters, target);
            p.merge(rw_quantity_expr(count));
            (p, sc)
        }
        Effect::RemoveCounter {
            counter_type: _,
            count,
            target,
        } => {
            let (mut p, sc) = obj(StateKind::ObjectCounters, target);
            p.merge(rw_quantity_expr(count));
            (p, sc)
        }
        Effect::MultiplyCounter {
            target,
            counter_type: _,
            multiplier: _,
        } => obj(StateKind::ObjectCounters, target),
        Effect::MoveCounters {
            source,
            count,
            target,
            counter_type: _,
            mode: _,
            selection: _,
        } => {
            let (mut p, sc) = obj(StateKind::ObjectCounters, target);
            let source_sc = scope_of(source, chain_root);
            place_object_write(&mut p, StateKind::ObjectCounters, source_sc);
            // CR 122.1 object-scope (§2): the donor is also an external counter write.
            if matches!(source_sc, WriteScope::External | WriteScope::EventObject) {
                p.writes_external_counter_census
                    .merge(census_of_filter(source));
            }
            flag_legacy_write_target(&mut p, source);
            if let Some(c) = count {
                p.merge(rw_quantity_expr(c));
            }
            (p, sc)
        }
        Effect::Bolster { count } => {
            let mut p = ext_write(StateKind::ObjectCounters);
            // Untargeted external counter write ⇒ census Any (fail-closed, §2).
            p.writes_external_counter_census.merge(Census::Any);
            p.merge(rw_quantity_expr(count));
            (p, Some(WriteScope::External))
        }
        Effect::Endure { amount, subject } => {
            let (mut p, sc) = obj(StateKind::ObjectCounters, subject);
            p.writes_external.set(StateKind::SetMembership);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(amount));
            (p, sc)
        }
        Effect::AddPendingETBCounters {
            count,
            counter_type: _,
        } => {
            let mut p = self_write(StateKind::ObjectCounters);
            p.merge(rw_quantity_expr(count));
            (p, Some(WriteScope::SelfSource))
        }
        Effect::Proliferate => {
            let mut p = ext_write(StateKind::ObjectCounters);
            p.writes_external.set(StateKind::PlayerLife);
            // Any counter on any permanent/player with a counter ⇒ census Any.
            p.writes_external_counter_census.merge(Census::Any);
            (p, Some(WriteScope::External))
        }
        Effect::Amass { count, subtype: _ } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::ObjectCounters);
            p.writes_external_counter_census.merge(Census::Any);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(count));
            (p, Some(WriteScope::External))
        }
        Effect::Intensify { amount, scope: _ } => {
            let mut p = ext_write(StateKind::ObjectCounters);
            p.writes_external_counter_census.merge(Census::Any);
            p.merge(rw_quantity_expr(amount));
            (p, Some(WriteScope::External))
        }
        Effect::Double {
            target,
            target_kind: _,
        } => {
            let (mut p, sc) = obj(StateKind::ObjectCounters, target);
            p.writes_external.set(StateKind::PlayerLife);
            (p, sc)
        }

        // ---- Zone moves (SetMembership) ----
        Effect::Destroy {
            target,
            cant_regenerate: _,
        } => mem(target, Some(Zone::Battlefield), Zone::Graveyard),
        Effect::DestroyAll {
            target,
            cant_regenerate: _,
        } => mem(target, Some(Zone::Battlefield), Zone::Graveyard),
        Effect::Sacrifice {
            target,
            count,
            min_count: _,
        } => {
            let (mut p, sc) = mem(target, Some(Zone::Battlefield), Zone::Graveyard);
            p.merge(rw_quantity_expr(count));
            (p, sc)
        }
        Effect::Bounce {
            target,
            destination,
            selection: _,
        } => mem(
            target,
            Some(Zone::Battlefield),
            destination.unwrap_or(Zone::Hand),
        ),
        Effect::BounceAll {
            target,
            count,
            destination,
        } => {
            let (mut p, sc) = mem(
                target,
                Some(Zone::Battlefield),
                destination.unwrap_or(Zone::Hand),
            );
            if let Some(c) = count {
                p.merge(rw_quantity_expr(c));
            }
            (p, sc)
        }
        Effect::ChangeZone {
            origin,
            destination,
            target,
            owner_library: _,
            enter_transformed: _,
            enters_under: _,
            enter_tapped: _,
            enters_attacking: _,
            up_to: _,
            enter_with_counters,
            conditional_enter_with_counters,
            face_down_profile: _,
            enters_modified_if: _,
        } => {
            let (mut p, sc) = mem(target, *origin, *destination);
            for (_ct, q) in enter_with_counters {
                p.merge(rw_quantity_expr(q));
            }
            for (_f, _ct, q) in conditional_enter_with_counters {
                p.merge(rw_quantity_expr(q));
            }
            (p, sc)
        }
        Effect::ChangeZoneAll {
            origin,
            destination,
            target,
            enter_with_counters,
            enters_under: _,
            enter_tapped: _,
            face_down_profile: _,
            library_position: _,
            random_order: _,
        } => {
            let (mut p, sc) = mem(target, *origin, *destination);
            for (_ct, q) in enter_with_counters {
                p.merge(rw_quantity_expr(q));
            }
            (p, sc)
        }
        Effect::Mill {
            count,
            target: _,
            destination: _,
        } => {
            let mut p = RwProfile::empty();
            place_membership_write(
                &mut p,
                WriteScope::External,
                Census::Any,
                Some(Zone::Library),
                Zone::Graveyard,
            );
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::ExileTop {
            player: _,
            count,
            face_down: _,
        } => {
            let mut p = RwProfile::empty();
            place_membership_write(
                &mut p,
                WriteScope::External,
                Census::Any,
                Some(Zone::Library),
                Zone::Exile,
            );
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::ExileFromTopUntil {
            player: _,
            until: _,
        } => {
            let mut p = RwProfile::empty();
            place_membership_write(
                &mut p,
                WriteScope::External,
                Census::Any,
                Some(Zone::Library),
                Zone::Exile,
            );
            (p, None)
        }
        Effect::Dig {
            player: _,
            count,
            filter: _,
            destination: _,
            keep_count: _,
            up_to: _,
            rest_destination: _,
            reveal: _,
            enter_tapped: _,
            source: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::Seek {
            filter: _,
            count,
            from_top: _,
            destination: _,
            enter_tapped: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::SearchLibrary {
            source_zones: _,
            filter: _,
            count,
            reveal: _,
            target_player: _,
            selection_constraint: _,
            split: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::ChooseFromZone {
            filter: _,
            count: _,
            zone: _,
            additional_zones: _,
            zone_owner: _,
            chooser: _,
            up_to: _,
            selection: _,
            constraint: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            (p, None)
        }
        Effect::Explore => {
            let mut p = self_write(StateKind::ObjectCounters);
            p.writes_external.set(StateKind::HandLibrary);
            (p, None)
        }
        Effect::Forage => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            (p, None)
        }
        Effect::Connive { target, count } => {
            let (mut p, sc) = obj(StateKind::ObjectCounters, target);
            p.writes_external.set(StateKind::HandLibrary);
            p.merge(rw_quantity_expr(count));
            (p, sc)
        }
        Effect::CollectEvidence { amount: _ } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            (p, None)
        }
        Effect::Discover {
            mana_value_limit,
            player: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_external.set(StateKind::StackShape);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(mana_value_limit));
            (p, None)
        }
        Effect::RevealUntil {
            player: _,
            filter: _,
            count,
            enters_under: _,
            matched_disposition: _,
            kept_destination: _,
            rest_destination: _,
            enter_tapped: _,
            enters_attacking: _,
            kept_optional_to: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::Manifest {
            target: _,
            count,
            enters_under: _,
            profile: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::ManifestDread => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            (p, None)
        }
        Effect::Cloak { target: _, count } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::Meld {
            source: _,
            partner: _,
            result: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            (p, None)
        }
        Effect::PhaseOut { target } => obj_membership_scope(target, chain_root),
        Effect::MadnessCast { cost: _ } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_external.set(StateKind::StackShape);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            (p, None)
        }

        // ---- Creation (SetMembership, fresh ids) ----
        Effect::Token {
            name: _,
            power: _,
            toughness: _,
            types,
            colors: _,
            keywords: _,
            tapped: _,
            count,
            owner: _,
            attach_to: _,
            enters_attacking: _,
            supertypes: _,
            static_abilities: _,
            enter_with_counters,
        } => {
            let mut p = RwProfile::empty();
            p.writes_external.set(StateKind::SetMembership);
            p.writes_created.set(StateKind::SetMembership);
            p.writes_membership_external_census
                .merge(census_of_types(types));
            // CR 111.1 + CR 400.1: a token is CREATED on the battlefield — it
            // touches ONLY that zone (no origin), so it cannot feed a graveyard /
            // hand / library read (Tombstone Stairwell: battlefield Zombie tokens
            // vs a graveyard-creature count are zone-disjoint).
            p.writes_membership_external_zones
                .merge(ZoneSpan::one(Zone::Battlefield));
            for (_ct, q) in enter_with_counters {
                p.merge(rw_quantity_expr(q));
            }
            p.merge(rw_quantity_expr(count));
            (p, Some(WriteScope::Created))
        }
        Effect::CopyTokenOf {
            target,
            owner: _,
            source_filter,
            enters_attacking: _,
            tapped: _,
            count,
            extra_keywords: _,
            additional_modifications: _,
        } => {
            let mut p = RwProfile::empty();
            p.writes_external.set(StateKind::SetMembership);
            p.writes_created.set(StateKind::SetMembership);
            // CR 707.2: the created token acquires the copy SOURCE's copiable
            // values (card type / subtypes), so its membership census is the
            // source's typeline — NOT `Census::Any`.
            if matches!(target, TargetFilter::SelfRef) {
                // A SelfRef copy's census is the group's live source census,
                // resolved at `profiles_conflict` time — reproduce the SelfRef
                // membership-move representation (`writes_membership_self`), which
                // `membership_census_of` resolves against `source_census`. This
                // is what clears Scute Swarm (creature token ≠ its Lands read,
                // census-disjoint — §1.3.1-F) instead of over-conflicting on Any.
                p.writes_membership_self = true;
                // Copiable-values read (fail-closed source dependence).
                p.reads_src.set(StateKind::ObjectPt);
            } else {
                // Non-SelfRef: census from the copy-source filter where
                // extractable (`source_filter` for the "for each" variant, else
                // the targeted `target`), else `Census::Any` (fail-closed).
                let copy_source = source_filter.as_ref().unwrap_or(target);
                p.writes_membership_external_census
                    .merge(census_of_filter(copy_source));
                // CR 111.1: the copy is created on the battlefield.
                p.writes_membership_external_zones
                    .merge(ZoneSpan::one(Zone::Battlefield));
            }
            // D5: an event-context write target (e.g. `CopyTokenOf{TriggeringSource}`)
            // retains the batch prompt (CR 603.10a).
            flag_legacy_write_target(&mut p, target);
            p.merge(rw_quantity_expr(count));
            (p, Some(WriteScope::Created))
        }
        Effect::Conjure {
            cards: _,
            destination,
            tapped: _,
        } => {
            let mut p = RwProfile::empty();
            p.writes_external.set(StateKind::SetMembership);
            p.writes_created.set(StateKind::SetMembership);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            if is_hand_or_library(*destination) {
                p.writes_external.set(StateKind::HandLibrary);
            }
            (p, Some(WriteScope::Created))
        }
        Effect::Incubate { count } => {
            let mut p = RwProfile::empty();
            p.writes_external.set(StateKind::SetMembership);
            p.writes_created.set(StateKind::SetMembership);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            p.merge(rw_quantity_expr(count));
            (p, Some(WriteScope::Created))
        }
        Effect::Populate => {
            let mut p = RwProfile::empty();
            p.writes_external.set(StateKind::SetMembership);
            p.writes_created.set(StateKind::SetMembership);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            (p, Some(WriteScope::Created))
        }
        Effect::Investigate => {
            let mut p = RwProfile::empty();
            p.writes_external.set(StateKind::SetMembership);
            p.writes_created.set(StateKind::SetMembership);
            p.writes_membership_external_census
                .merge(Census::Tags(BTreeSet::from([
                    "clue".into(),
                    "artifact".into(),
                ])));
            // CR 111.1: the Clue token is created on the battlefield.
            p.writes_membership_external_zones
                .merge(ZoneSpan::one(Zone::Battlefield));
            (p, Some(WriteScope::Created))
        }

        // ---- P/T & type (ObjectPt) ----
        Effect::Pump {
            power: _,
            toughness: _,
            target,
        } => obj(StateKind::ObjectPt, target),
        Effect::PumpAll {
            power: _,
            toughness: _,
            target,
        } => obj(StateKind::ObjectPt, target),
        Effect::DoublePT {
            target,
            mode: _,
            factor: _,
        } => obj(StateKind::ObjectPt, target),
        Effect::DoublePTAll {
            target,
            mode: _,
            factor: _,
        } => obj(StateKind::ObjectPt, target),
        Effect::SwitchPT { target } => obj(StateKind::ObjectPt, target),
        Effect::Transform { target } => obj(StateKind::ObjectPt, target),
        Effect::BecomeCopy {
            target,
            duration: _,
            mana_value_limit: _,
            additional_modifications: _,
        } => {
            let (mut p, sc) = obj(StateKind::ObjectPt, target);
            place_object_write(
                &mut p,
                StateKind::SetMembership,
                scope_of(target, chain_root),
            );
            (p, sc)
        }
        Effect::Animate {
            power: _,
            toughness: _,
            types: _,
            remove_types: _,
            target,
            keywords: _,
        } => {
            let (mut p, sc) = obj(StateKind::ObjectPt, target);
            place_object_write(
                &mut p,
                StateKind::SetMembership,
                scope_of(target, chain_root),
            );
            (p, sc)
        }
        Effect::TurnFaceUp { target } => {
            let (mut p, sc) = obj(StateKind::ObjectPt, target);
            place_object_write(
                &mut p,
                StateKind::SetMembership,
                scope_of(target, chain_root),
            );
            (p, sc)
        }
        Effect::TurnFaceDown { target, profile: _ } => obj(StateKind::ObjectPt, target),
        Effect::GenericEffect {
            static_abilities: _,
            duration,
            target,
        } => {
            let tf = target.clone().unwrap_or(TargetFilter::SelfRef);
            let (mut p, sc) = obj(StateKind::ObjectPt, &tf);
            place_object_write(&mut p, StateKind::SetMembership, scope_of(&tf, chain_root));
            if let Some(d) = duration {
                p.merge(rw_duration(d));
            }
            (p, sc)
        }

        // ---- Control (SetMembership external) ----
        Effect::GainControl { target: _ }
        | Effect::GainControlAll { target: _ }
        | Effect::GiveControl {
            target: _,
            recipient: _,
        }
        | Effect::ExchangeControl {
            target_a: _,
            target_b: _,
        } => {
            let mut p = ext_write(StateKind::SetMembership);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            (p, Some(WriteScope::External))
        }

        // ---- Stack (StackShape) ----
        Effect::CopySpell {
            target,
            retarget: _,
            copier: _,
            additional_modifications: _,
            starting_loyalty_from_casualty_sacrifice: _,
        } => {
            // CR 707.10/707.10c: SelfRef / explicit-target copies read the
            // original by id (clean); the untargeted top-of-stack fallback reads
            // the mutable stack top (D4).
            let mut p = ext_write(StateKind::StackShape);
            if !matches!(
                target,
                TargetFilter::SelfRef
                    | TargetFilter::StackSpell
                    | TargetFilter::SpecificObject { .. }
            ) {
                p.reads_board.set(StateKind::StackShape);
            }
            (p, None)
        }
        Effect::Counter {
            target: _,
            source_rider: _,
            countered_spell_zone: _,
        } => (ext_write(StateKind::StackShape), None),
        Effect::CastCopyOfCard {
            target: _,
            count,
            cost: _,
        } => {
            let mut p = ext_write(StateKind::StackShape);
            p.writes_external.set(StateKind::SetMembership);
            p.writes_external.set(StateKind::HandLibrary);
            p.writes_membership_external_census.merge(Census::Any);
            p.writes_membership_external_zones.merge(ZoneSpan::Any);
            if let Some(c) = count {
                p.merge(rw_quantity_expr(c));
            }
            (p, None)
        }
        Effect::CastFromZone {
            target: _,
            without_paying_mana_cost: _,
            mode: _,
            cast_transformed: _,
            alt_ability_cost: _,
            constraint: _,
            duration: _,
            driver: _,
            mana_spend_permission: _,
        } => {
            let mut p = ext_write(StateKind::HandLibrary);
            p.writes_external.set(StateKind::StackShape);
            (p, None)
        }
        Effect::ExileResolvingSpellInsteadOfGraveyard => (ext_write(StateKind::StackShape), None),

        // ---- Pool ----
        Effect::Mana {
            produced: _,
            restrictions: _,
            grants: _,
            expiry: _,
            target: _,
        } => (writes_pool_profile(), None),

        // ---- Tap ----
        Effect::SetTapState {
            target,
            scope: _,
            state: _,
        } => obj(StateKind::TapState, target),

        // ---- Deferred bodies (CR 603.7): reads descended, writes NOT counted ----
        Effect::CreateDelayedTrigger {
            condition: _,
            effect,
            uses_tracked_set: _,
        } => (deferred(effect), None),
        Effect::CreateDrawReplacement { replacement_effect } => {
            let (mut b, _) = rw_effect(replacement_effect, None);
            b.drop_writes();
            (b, None)
        }
        Effect::PreventDamage {
            amount_dynamic,
            target: _,
            damage_source_filter: _,
            prevention_duration: _,
            amount: _,
            scope: _,
        } => {
            let mut p = RwProfile::empty();
            if let Some(q) = amount_dynamic {
                p.merge(rw_quantity_expr(q));
            }
            (p, None)
        }
        Effect::PayCost {
            cost: _,
            scale,
            payer: _,
        } => {
            let mut p = RwProfile::empty();
            p.has_pay_or_unless = true;
            if let Some(q) = scale {
                p.merge(rw_quantity_expr(q));
            }
            (p, None)
        }
        Effect::AddTargetReplacement { .. }
        | Effect::AddRestriction { .. }
        | Effect::ReduceNextSpellCost { .. }
        | Effect::GrantNextSpellAbility { .. }
        | Effect::CreateEmblem { .. }
        | Effect::CreateDamageReplacement { .. }
        | Effect::Regenerate { .. }
        | Effect::GrantCastingPermission { .. } => (RwProfile::empty(), None),

        // ---- Choice wrappers (union descent, §2) ----
        Effect::ChooseOneOf { chooser, branches } => {
            let mut p = rw_player_filter(chooser);
            for b in branches {
                walk_definition(b, chain_root, &mut p);
            }
            (p, None)
        }
        Effect::Vote {
            choices: _,
            per_choice_effect,
            starting_with,
            voter_scope: _,
            tally_mode: _,
            // CR 701.38a/b: visibility is reveal-timing only (inert); subject's
            // Objects case is an unmodeled residual of this incomplete arm (which
            // also drops outcome_template) — not introduced by the rebase.
            subject: _,
            visibility: _,
        } => {
            let mut p = rw_controller_ref(starting_with);
            for b in per_choice_effect {
                walk_definition(b, chain_root, &mut p);
            }
            (p, None)
        }

        // ---- RNG (no read-kind; descend sub-effects, §2/D4) ----
        Effect::RollDie {
            count,
            sides: _,
            results,
            modifier: _,
        } => {
            let mut p = rw_quantity_expr(count);
            for r in results {
                walk_definition(&r.effect, chain_root, &mut p);
            }
            (p, None)
        }
        Effect::FlipCoin {
            win_effect,
            lose_effect,
            flipper: _,
        } => {
            let mut p = RwProfile::empty();
            if let Some(w) = win_effect {
                walk_definition(w, chain_root, &mut p);
            }
            if let Some(l) = lose_effect {
                walk_definition(l, chain_root, &mut p);
            }
            (p, None)
        }
        Effect::FlipCoins {
            count,
            win_effect,
            lose_effect,
            flipper: _,
        } => {
            let mut p = rw_quantity_expr(count);
            if let Some(w) = win_effect {
                walk_definition(w, chain_root, &mut p);
            }
            if let Some(l) = lose_effect {
                walk_definition(l, chain_root, &mut p);
            }
            (p, None)
        }
        Effect::FlipCoinUntilLose { win_effect } => {
            let mut p = RwProfile::empty();
            walk_definition(win_effect, chain_root, &mut p);
            (p, None)
        }

        // ---- Status / designation tail (fail-closed writes_external{Other}).
        // M3: all fields bound (no `..` on a non-conservative RHS) so a future
        // read/write-bearing field forces reclassification. Count/min/max leaves
        // here are fixed numbers (ability_scan does not descend them). ----
        // CR 702.95 soulbond / CR 701 attach: an attachment/pairing DESIGNATION.
        // It mutates only pairing/attachment state, which EVERY reader consults
        // through a FROZEN source condition (SourceIsPaired / SourceAttachedTo-
        // Creature / SourceIsEquipped ⇒ `frozen_source_read`, never fed), so no
        // LIVE read observes it ⇒ NO observable RW kind. The plan's status-tail
        // `Other` default is parity-safe only where no co-occurring read exists,
        // but Deadeye Navigator's soulbond trigger reads `SourceMatchesFilter`
        // ObjectPt alongside the `PairWith` write, so `Other` (which conflicts with
        // any read) falsely prompts. Order-independence proof: two identical
        // soulbond triggers off ONE creature-enters event each pair their own
        // source with the event object; pairing is a symmetric designation and the
        // ObjectPt read sees only the frozen pre-write source ⇒ identical board in
        // either order (no feed — attachment/pairing state is read only frozen).
        // The write target still flags D5 batch parity (CR 603.10a).
        Effect::PairWith { target }
        | Effect::Attach {
            attachment: _,
            target,
        } => {
            let mut p = RwProfile::empty();
            flag_legacy_write_target(&mut p, target);
            (p, None)
        }
        // Target-bearing status effects: `target` is a write recipient, so a D5
        // event-context ref there retains the batch prompt (CR 603.10a).
        Effect::Goad { target }
        | Effect::GoadAll { target }
        | Effect::ExtraTurn { target }
        | Effect::Suspect { target, scope: _ }
        | Effect::Unsuspect { target, scope: _ }
        | Effect::BecomePrepared { target }
        | Effect::ApplyPerpetual {
            target,
            modification: _,
        } => {
            let mut p = ext_write(StateKind::Other);
            flag_legacy_write_target(&mut p, target);
            (p, None)
        }
        Effect::OpenAttractions { count: _ }
        | Effect::RegisterBending { kind: _ }
        | Effect::BlightEffect {
            player: _,
            count: _,
        }
        | Effect::ChooseObjectsIntoTrackedSet {
            chooser: _,
            filter: _,
            min: _,
            max: _,
        }
        | Effect::BecomeMonarch
        | Effect::RingTemptsYou
        | Effect::TimeTravel
        | Effect::Planeswalk
        | Effect::VentureIntoDungeon
        | Effect::SolveCase => (ext_write(StateKind::Other), None),
        Effect::ForceAttack {
            target,
            required_player: _,
            duration,
        } => {
            let mut p = ext_write(StateKind::Other);
            flag_legacy_write_target(&mut p, target);
            p.merge(rw_duration(duration));
            (p, None)
        }
        Effect::AdditionalPhase {
            target: _,
            count,
            phase: _,
            after: _,
            followed_by: _,
            attacker_restriction: _,
        } => {
            let mut p = ext_write(StateKind::Other);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::AssembleContraptions { count } => {
            let mut p = ext_write(StateKind::Other);
            p.merge(rw_quantity_expr(count));
            (p, None)
        }
        Effect::PutSticker {
            target: _,
            count,
            max_ticket_cost,
            kind: _,
            ticket_cost_payment: _,
        } => {
            let mut p = ext_write(StateKind::Other);
            p.merge(rw_quantity_expr(count));
            if let Some(q) = max_ticket_cost {
                p.merge(rw_quantity_expr(q));
            }
            (p, None)
        }

        // ---- No observable WRITE kind (info / terminal / plumbing). M3: bind
        // all fields. `RevealHand.count` is a dynamic `Option<QuantityExpr>` read
        // (ability_scan descends it) ⇒ surfaced below; all other leaves here are
        // concrete/announced values. ----
        Effect::RevealHand {
            count,
            target: _,
            card_filter: _,
            selection: _,
            choice_optional: _,
            reveal: _,
        } => {
            let mut p = RwProfile::empty();
            if let Some(q) = count {
                p.merge(rw_quantity_expr(q));
            }
            (p, None)
        }
        Effect::ApplyPostReplacementDamage {
            context: _,
            target: _,
            amount: _,
            is_combat: _,
        }
        | Effect::Cleanup {
            clear_remembered: _,
            clear_chosen_player: _,
            clear_chosen_color: _,
            clear_chosen_type: _,
            clear_chosen_card: _,
            clear_imprinted: _,
            clear_triggers: _,
            clear_coin_flips: _,
        }
        | Effect::RuntimeHandled { handler: _ }
        | Effect::Reveal { target: _ }
        | Effect::RevealTop {
            player: _,
            count: _,
        }
        | Effect::TargetOnly { target: _ }
        | Effect::Choose {
            choice_type: _,
            persist: _,
            selection: _,
        }
        | Effect::LoseTheGame { target: _ }
        | Effect::WinTheGame { target: _ }
        | Effect::RemoveAllDamage { target: _ }
        | Effect::Unimplemented {
            name: _,
            description: _,
        }
        | Effect::NoOp
        | Effect::EndTheTurn
        | Effect::EndCombatPhase => (RwProfile::empty(), None),

        // ---- Histogram-absent ⇒ fail-closed conservative ----
        Effect::StartYourEngines { .. }
        | Effect::ChangeSpeed { .. }
        | Effect::EachDealsDamageEqualToPower { .. }
        | Effect::CounterAll { .. }
        | Effect::SearchOutsideGame { .. }
        | Effect::RevealFromHand { .. }
        | Effect::ChooseDamageSource { .. }
        | Effect::PhaseIn { .. }
        | Effect::ForceBlock { .. }
        | Effect::BecomeUnprepared { .. }
        | Effect::BecomeSaddled { .. }
        | Effect::SetClassLevel { .. }
        | Effect::FreeCastFromZones { .. }
        | Effect::CreateTokenCopyFromPool { .. }
        | Effect::Myriad
        | Effect::Encore
        | Effect::CombineHost { .. }
        | Effect::ChooseAugmentAndCombineWithHost { .. }
        | Effect::ExileHaunting { .. }
        | Effect::HideawayConceal { .. }
        | Effect::CopyTokenBlockingAttacker { .. }
        | Effect::GainActivatedAbilitiesOfTarget { .. }
        | Effect::ChooseCard { .. }
        | Effect::ReturnAsAura { .. }
        | Effect::EpicCopy { .. }
        | Effect::SeparateIntoPiles { .. }
        | Effect::ControlNextTurn { .. }
        | Effect::UnattachAll { .. }
        | Effect::ExploreAll { .. }
        | Effect::Tribute { .. }
        | Effect::ProliferateTarget { .. }
        | Effect::Exploit { .. }
        | Effect::LoseAllPlayerCounters { .. }
        | Effect::Heist { .. }
        | Effect::HeistExile
        | Effect::Cascade
        | Effect::Ripple { .. }
        | Effect::MiracleCast { .. }
        | Effect::PutOnTopOrBottom { .. }
        | Effect::GiftDelivery { .. }
        | Effect::Detain { .. }
        | Effect::SetRoomDoorLock { .. }
        | Effect::ChangeTargets { .. }
        | Effect::GrantExtraLoyaltyActivations { .. }
        | Effect::SkipNextTurn { .. }
        | Effect::SkipNextStep { .. }
        | Effect::RemoveFromCombat { .. }
        | Effect::ExchangeLifeWithStat { .. }
        | Effect::ExchangeLifeTotals { .. }
        | Effect::SetDayNight { .. }
        | Effect::Monstrosity { .. }
        | Effect::Specialize
        | Effect::Renown { .. }
        | Effect::Adapt { .. }
        | Effect::Harness
        | Effect::ChooseAndSacrificeRest { .. }
        | Effect::RememberCard { .. }
        | Effect::ForEachCategoryExile { .. }
        | Effect::VentureInto { .. }
        | Effect::TakeTheInitiative
        | Effect::RollToVisitAttractions
        | Effect::AssembleContraptionsFromRollDifference
        | Effect::CrankContraptions { .. }
        | Effect::ReassembleContraption { .. }
        | Effect::AssembleContraptionOnSprocket { .. }
        | Effect::ReassembleContraptionOnSprocket { .. }
        | Effect::ApplySticker { .. }
        | Effect::ProcessRadCounters => (RwProfile::conservative(), None),
    }
}

/// A membership write scoped by target (used where the scope matters for chain
/// propagation, e.g. `PhaseOut`).
fn obj_membership_scope(
    target: &TargetFilter,
    chain_root: Option<WriteScope>,
) -> (RwProfile, Option<WriteScope>) {
    let sc = scope_of(target, chain_root);
    let mut p = RwProfile::empty();
    place_membership_write(
        &mut p,
        sc,
        census_of_filter(target),
        Some(Zone::Battlefield),
        Zone::Exile,
    );
    flag_legacy_write_target(&mut p, target);
    (p, Some(sc))
}

/// CR 205: extract a census from a token's type strings.
fn census_of_types(types: &[String]) -> Census {
    if types.is_empty() {
        return Census::Any;
    }
    Census::Tags(types.iter().map(|t| t.to_lowercase()).collect())
}

// ---------------------------------------------------------------------------
// Quantity reads (mirror `scan_quantity_*`).
// ---------------------------------------------------------------------------

fn rw_quantity_expr(x: &QuantityExpr) -> RwProfile {
    match x {
        QuantityExpr::Ref { qty } => rw_quantity_ref(qty),
        QuantityExpr::Fixed { value: _ } => RwProfile::empty(),
        QuantityExpr::DivideRounded {
            inner,
            divisor: _,
            rounding: _,
        }
        | QuantityExpr::Offset { inner, offset: _ }
        | QuantityExpr::ClampMin { inner, minimum: _ }
        | QuantityExpr::Multiply { inner, factor: _ }
        | QuantityExpr::UpTo { max: inner } => rw_quantity_expr(inner),
        QuantityExpr::Power { exponent, base: _ } => rw_quantity_expr(exponent),
        QuantityExpr::Difference { left, right } => {
            let mut p = rw_quantity_expr(left);
            p.merge(rw_quantity_expr(right));
            p
        }
        QuantityExpr::Sum { exprs } | QuantityExpr::Max { exprs } => {
            let mut p = RwProfile::empty();
            for e in exprs {
                p.merge(rw_quantity_expr(e));
            }
            p
        }
    }
}

fn rw_quantity_ref(x: &QuantityRef) -> RwProfile {
    match x {
        QuantityRef::HandSize { .. } => reads_player_of(StateKind::HandLibrary),
        QuantityRef::LifeTotal { player: _ } | QuantityRef::LifeAboveStarting => {
            reads_player_of(StateKind::PlayerLife)
        }
        QuantityRef::StartingLifeTotal => RwProfile::empty(),
        QuantityRef::GraveyardSize { .. } => reads_zone_membership(),
        QuantityRef::ObjectCount { filter }
        | QuantityRef::ObjectCountDistinct { filter, .. }
        | QuantityRef::ObjectCountBySharedQuality { filter, .. }
        | QuantityRef::ControlledByEachPlayer { filter, .. }
        | QuantityRef::DistinctColorsAmongPermanents { filter } => board_membership_read(filter),
        QuantityRef::CountersOnObjects {
            filter,
            counter_type: _,
        }
        | QuantityRef::DistinctCounterKindsAmong { filter } => {
            board_value_aggregate_read(filter, StateKind::ObjectCounters)
        }
        QuantityRef::Aggregate {
            filter,
            function: _,
            property: _,
        } => board_value_aggregate_read(filter, StateKind::ObjectPt),
        QuantityRef::PlayerCount { filter: _ } => RwProfile::empty(),
        QuantityRef::CountersOn { scope, .. } | QuantityRef::Intensity { scope, .. } => {
            read_object_scope(scope, StateKind::ObjectCounters)
        }
        QuantityRef::Power { scope, .. }
        | QuantityRef::Toughness { scope, .. }
        | QuantityRef::ObjectManaValue { scope, .. }
        | QuantityRef::ObjectColorCount { scope, .. }
        | QuantityRef::ObjectNameWordCount { scope, .. }
        | QuantityRef::ObjectTypelineComponentCount { scope, .. }
        | QuantityRef::ManaSymbolsInManaCost { scope, .. } => {
            read_object_scope(scope, StateKind::ObjectPt)
        }
        QuantityRef::TargetObjectManaValue { filter: _ } => reads_board_of(StateKind::ObjectPt),
        QuantityRef::PlayerCounter { scope: _, kind: _ } => reads_player_of(StateKind::PlayerLife),
        // CR 122.1f: the target's controller's player-counter total (poison ==
        // "poisoned"). A target-relative player-mutable read — same proxy as the
        // `PlayerCounter` sibling above (StateKind has no dedicated player-counter
        // kind; player counters ride the `PlayerLife` sibling-mutable player-state
        // row, which `Effect::GivePlayerCounter` writes for the matching feed).
        QuantityRef::TargetControllerCounter { kind: _ } => reads_player_of(StateKind::PlayerLife),
        QuantityRef::Variable { name: _ } | QuantityRef::SelfManaValue => RwProfile::empty(),
        QuantityRef::TargetZoneCardCount { zone: _ } => reads_zone_membership(),
        QuantityRef::Devotion { .. }
        | QuantityRef::DistinctCardTypes { .. }
        | QuantityRef::BasicLandTypeCount { .. }
        | QuantityRef::PartySize { .. } => reads_zone_membership(),
        QuantityRef::CardsExiledBySource
        | QuantityRef::ExiledCardPower { .. }
        | QuantityRef::TrackedSetSize
        | QuantityRef::FilteredTrackedSetSize { .. }
        | QuantityRef::TrackedSetAggregate { .. }
        | QuantityRef::ExiledFromHandThisResolution
        | QuantityRef::PreviousEffectAmount
        | QuantityRef::TurnsTaken
        | QuantityRef::CrimesCommittedThisTurn
        | QuantityRef::ChosenNumber
        | QuantityRef::AttackedThisTurn { .. }
        | QuantityRef::DescendedThisTurn
        // CR 701.65b/701.66b/701.67c: controller-scoped per-turn accumulator; no
        // per-source binding, member-invariant under uniformity (Avatar Aang).
        | QuantityRef::BendTypesThisTurn
        | QuantityRef::LandsPlayedThisTurn { .. }
        | QuantityRef::DungeonsCompleted
        | QuantityRef::CostXPaid
        | QuantityRef::KickerCount
        | QuantityRef::AdditionalCostPaymentCount
        | QuantityRef::AdditionalCostPaymentCountFor { .. }
        | QuantityRef::ConvokedCreatureCount
        | QuantityRef::ColorsInCommandersColorIdentity
        | QuantityRef::CommanderCastFromCommandZoneCount
        | QuantityRef::CommanderManaValue { .. }
        | QuantityRef::Speed { .. }
        | QuantityRef::VoteCount { .. } => RwProfile::empty(),
        QuantityRef::ZoneCardCount { filter, .. } => match filter {
            Some(f) => board_membership_read(f),
            None => reads_zone_membership(),
        },
        // Object-population "this turn" journals ⇒ membership-fed board reads.
        QuantityRef::EnteredThisTurn { filter }
        | QuantityRef::SacrificedThisTurn { filter, .. }
        | QuantityRef::BattlefieldEntriesThisTurn { filter, .. }
        | QuantityRef::ZoneChangeCountThisTurn { filter, .. }
        | QuantityRef::ZoneChangeAggregateThisTurn { filter, .. }
        | QuantityRef::TokensCreatedThisTurn { filter, .. } => board_membership_read(filter),
        QuantityRef::CounterAddedThisTurn { .. } => reads_board_of(StateKind::ObjectCounters),
        // Player-resource journals.
        QuantityRef::LifeLostThisTurn { player: _ }
        | QuantityRef::LifeGainedThisTurn { player: _ }
        | QuantityRef::DamageDealtThisTurn { .. } => reads_player_of(StateKind::JournalLife),
        QuantityRef::CardsDrawnThisTurn { player: _ }
        | QuantityRef::CardsDiscardedThisTurn { .. } => reads_player_of(StateKind::JournalCards),
        QuantityRef::SpellsCastThisTurn { .. }
        | QuantityRef::SpellsCastLastTurn
        | QuantityRef::SpellsCastThisGame { .. }
        | QuantityRef::LoyaltyAbilitiesActivatedThisTurn { .. }
        | QuantityRef::PlayerActionsThisTurn { .. } => reads_player_of(StateKind::JournalCast),
        QuantityRef::UnspentMana { color: _ } => reads_player_of(StateKind::PlayerLife),
        QuantityRef::AttachmentsOnLeavingObject { .. } => reads_event_live(),
        QuantityRef::TimesCostPaidThisResolution => reads_event_live(),
        // D5 carriers.
        QuantityRef::EventContextAmount
        | QuantityRef::EventContextSourceCostX
        | QuantityRef::ManaSpentToCast { .. } => legacy_ref(),
    }
}

// ---------------------------------------------------------------------------
// Condition reads.
// ---------------------------------------------------------------------------

fn rw_ability_condition(x: &AbilityCondition) -> RwProfile {
    match x {
        AbilityCondition::QuantityCheck {
            lhs,
            rhs,
            comparator: _,
        } => {
            let mut p = rw_quantity_expr(lhs);
            p.merge(rw_quantity_expr(rhs));
            p
        }
        AbilityCondition::PreviousEffectAmount {
            rhs,
            comparator: _,
            channel: _,
        } => rw_quantity_expr(rhs),
        AbilityCondition::ObjectsShareQuality {
            subject: _,
            reference: _,
            quality: _,
        } => reads_board_of(StateKind::ObjectPt),
        AbilityCondition::TargetMatchesFilter {
            filter: _,
            use_lki: _,
            subject_slot: _,
        } => reads_board_of(StateKind::ObjectPt),
        AbilityCondition::SourceMatchesFilter { filter: _ } => reads_src_of(StateKind::ObjectPt),
        AbilityCondition::SourceIsTapped => reads_src_of(StateKind::TapState),
        AbilityCondition::ControllerControlsMatching { filter } => board_membership_read(filter),
        AbilityCondition::ScopedPlayerMatches { filter } => rw_player_filter(filter),
        AbilityCondition::TriggeringSpellTargetsFilter { filter: _ }
        | AbilityCondition::ZoneChangeObjectMatchesFilter { .. }
        | AbilityCondition::ZoneChangedThisWay { filter: _ }
        | AbilityCondition::CostPaidObjectMatchesFilter { filter: _ } => reads_event_live(),
        AbilityCondition::EventOutcomeWon => reads_event_live(),
        AbilityCondition::SpellCastWithVariantThisTurn { variant: _ }
        | AbilityCondition::NthResolutionThisTurn { n: _ } => {
            reads_player_of(StateKind::JournalCast)
        }
        // CR 701.20 + CR 603.3b: "if a card revealed THIS WAY has card type T" —
        // a read of the card the member's OWN parent reveal surfaced (a per-
        // resolution local, like an `ObjectScope::Recipient` read-modify-write:
        // §2 read-carrier closure). No sibling write can change the TYPE of the
        // card MY reveal surfaces; a sibling that reorders/moves the library only
        // changes WHICH card each identical member reveals, and identical
        // top-consuming functions compose order-independently (CR 603.3b T1:
        // f∘f = f∘f). Order-independence proof: two Delvers of Secrets off one
        // upkeep each look at the top card (a non-mutating Dig) and Transform{Self}
        // iff it is instant/sorcery — both read the SAME frozen top card and each
        // transforms its OWN source ⇒ identical board in either order; two Lurking
        // Predators off one spell cast each reveal-and-route the then-current top,
        // so the top-N cards are each routed by their own type regardless of which
        // copy processed which ⇒ identical library/battlefield in either order (no
        // feed — the read is the write's own reveal output). So `conservative()`
        // (which the coarse fallback assigned) falsely conflicts.
        AbilityCondition::RevealedHasCardType { .. } => RwProfile::empty(),
        AbilityCondition::SourceEnteredThisTurn
        | AbilityCondition::AdditionalCostPaid { .. }
        | AbilityCondition::CastVariantPaid { .. }
        | AbilityCondition::SourceAttachedToCreature
        | AbilityCondition::ControllerControlledMatchingAsCast { .. }
        | AbilityCondition::SourceLacksKeyword { .. }
        | AbilityCondition::WasStartingPlayer { .. } => frozen_source_read(),
        AbilityCondition::ConditionInstead { inner }
        | AbilityCondition::Not { condition: inner } => rw_ability_condition(inner),
        AbilityCondition::And { conditions } | AbilityCondition::Or { conditions } => {
            let mut p = RwProfile::empty();
            for c in conditions {
                p.merge(rw_ability_condition(c));
            }
            p
        }
        AbilityCondition::AdditionalCostPaidInstead
        | AbilityCondition::AlternativeManaCostPaid
        | AbilityCondition::EffectOutcome { .. }
        | AbilityCondition::WhenYouDo
        | AbilityCondition::CastFromZone { .. }
        | AbilityCondition::CastDuringPhase { .. }
        | AbilityCondition::CurrentPhaseIs { .. }
        | AbilityCondition::CastTimingPermission { .. }
        | AbilityCondition::ManaColorSpent { .. }
        | AbilityCondition::TargetSharesNameWithOtherExiledThisWay { .. }
        | AbilityCondition::CastVariantPaidInstead { .. }
        | AbilityCondition::HasMaxSpeed
        | AbilityCondition::IsMonarch
        | AbilityCondition::IsInitiative
        | AbilityCondition::HasCityBlessing
        | AbilityCondition::IsRingBearer
        | AbilityCondition::TargetHasKeywordInstead { .. }
        | AbilityCondition::HasObjectTarget
        | AbilityCondition::IsYourTurn
        | AbilityCondition::FirstCombatPhaseOfTurn
        | AbilityCondition::FirstEndStepOfTurn
        | AbilityCondition::DayNightIsNeither
        | AbilityCondition::DayNightIs { .. } => RwProfile::empty(),
    }
}

fn rw_trigger_condition(x: &TriggerCondition) -> RwProfile {
    match x {
        TriggerCondition::GainedLife { minimum: _ }
        | TriggerCondition::LostLife
        | TriggerCondition::LostLifeLastTurn => reads_player_of(StateKind::JournalLife),
        // CR 120.3e + CR 603.3b: "dealt damage this turn" is combat/marked-damage
        // history (CR 120), NOT a life-total change (CR 119) — a frozen per-turn
        // fact about the damaged object, settled at damage time. A sibling
        // GainLife/LoseLife (a `PlayerLife`/`JournalLife` write) cannot alter it, so
        // it must NOT ride the life-journal row (the coarse conflation that flipped
        // Abattoir Ghoul). Order-independence proof: two Abattoir Ghouls off ONE
        // creature's death both read the SAME frozen "dealt damage this turn" flag
        // and gain that creature's (LKI-frozen) toughness ⇒ identical life in either
        // order (no feed: a life write doesn't change a damage-history fact).
        // `frozen_source_read` never feeds while the freeze is valid (marks
        // source/history dependence; fail-closed on a reentry hazard).
        TriggerCondition::DealtDamageBySourceThisTurn
        | TriggerCondition::DealtDamageThisTurnBySource { source: _ } => frozen_source_read(),
        TriggerCondition::LifeTotalGE { minimum: _ } => reads_player_of(StateKind::PlayerLife),
        TriggerCondition::ControlsType { filter }
        | TriggerCondition::ControlCount { filter, .. }
        | TriggerCondition::ControlsNone { filter }
        | TriggerCondition::DefendingPlayerControlsNone { filter } => board_membership_read(filter),
        TriggerCondition::QuantityComparison {
            lhs,
            rhs,
            comparator: _,
        } => {
            let mut p = rw_quantity_expr(lhs);
            p.merge(rw_quantity_expr(rhs));
            p
        }
        TriggerCondition::HadCounters { .. } => reads_frozen_of(StateKind::ObjectCounters),
        TriggerCondition::HasCounters { .. } => reads_src_of(StateKind::ObjectCounters),
        TriggerCondition::CounterAddedThisTurn => reads_board_of(StateKind::ObjectCounters),
        TriggerCondition::SourceIsTapped => reads_src_of(StateKind::TapState),
        TriggerCondition::SourceMatchesFilter { filter: _ } => reads_src_of(StateKind::ObjectPt),
        TriggerCondition::NoSpellsCastLastTurn
        | TriggerCondition::TwoOrMoreSpellsCastLastTurn
        | TriggerCondition::CastSpellThisTurn { .. }
        | TriggerCondition::SpellCastWithVariantThisTurn { .. } => {
            reads_player_of(StateKind::JournalCast)
        }
        TriggerCondition::DuringPlayersTurn { player } => rw_player_filter(player),
        TriggerCondition::SourceEnteredThisTurn
        | TriggerCondition::SourceIsHarnessed
        | TriggerCondition::SourceIsAttacking
        | TriggerCondition::SourceIsTransformed
        | TriggerCondition::SourceIsFaceUp
        | TriggerCondition::SourceIsFaceDown
        | TriggerCondition::SourceInZone { .. }
        | TriggerCondition::IsRenowned { .. }
        | TriggerCondition::WasStartingPlayer { .. } => frozen_source_read(),
        TriggerCondition::ZoneChangeObjectMatchesFilter { .. }
        | TriggerCondition::ZoneChangeObjectIsTapped
        | TriggerCondition::EventDamageSourceMatchesFilter { .. }
        | TriggerCondition::DamagedPlayerIsEventSourceOwner
        | TriggerCondition::TriggeringSpellTargetsFilter { .. } => reads_event_live(),
        TriggerCondition::ManaColorSpent { .. } | TriggerCondition::ManaSpentCondition { .. } => {
            reads_player_of(StateKind::JournalCast)
        }
        TriggerCondition::And { conditions } | TriggerCondition::Or { conditions } => {
            let mut p = RwProfile::empty();
            for c in conditions {
                p.merge(rw_trigger_condition(c));
            }
            p
        }
        TriggerCondition::Not { condition } => rw_trigger_condition(condition),
        TriggerCondition::AttackersDeclaredCount { .. } => RwProfile::empty(),
        TriggerCondition::Descended
        | TriggerCondition::EchoDue
        | TriggerCondition::MinCoAttackers { .. }
        | TriggerCondition::SolveConditionMet
        | TriggerCondition::ClassLevelGE { .. }
        | TriggerCondition::AttractionVisitRoll { .. }
        | TriggerCondition::WasCast { .. }
        | TriggerCondition::WasPlayed
        | TriggerCondition::AdditionalCostPaid { .. }
        | TriggerCondition::CastVariantPaid { .. }
        | TriggerCondition::CastVariantPaidPersistent { .. }
        | TriggerCondition::ActivatedAbilityIsNonMana
        | TriggerCondition::FirstTimeObjectTappedThisTurn
        | TriggerCondition::WasType { .. }
        | TriggerCondition::AttackedThisTurn
        | TriggerCondition::FirstCombatPhaseOfTurn
        | TriggerCondition::HasMaxSpeed
        | TriggerCondition::IsMonarch
        | TriggerCondition::IsInitiative
        | TriggerCondition::NoMonarch
        | TriggerCondition::HasCityBlessing
        | TriggerCondition::CompletedDungeon { .. }
        | TriggerCondition::TributeNotPaid
        | TriggerCondition::CastDuringPhase { .. }
        | TriggerCondition::CastTimingPermission { .. }
        | TriggerCondition::ControlsCommander { .. }
        | TriggerCondition::ChosenLabelIs { .. }
        | TriggerCondition::ExceptFirstDrawInDrawStep
        | TriggerCondition::PlacedByAbilitySource => RwProfile::empty(),
    }
}

fn rw_static_condition(x: &StaticCondition) -> RwProfile {
    match x {
        StaticCondition::DevotionGE { .. }
        | StaticCondition::SharesColorWithMostCommonColorAmongPermanents => reads_zone_membership(),
        StaticCondition::IsPresent { filter } => match filter {
            Some(f) => board_membership_read(f),
            None => reads_zone_membership(),
        },
        StaticCondition::DefendingPlayerControls { filter } => board_membership_read(filter),
        StaticCondition::QuantityComparison {
            lhs,
            rhs,
            comparator: _,
        } => {
            let mut p = rw_quantity_expr(lhs);
            p.merge(rw_quantity_expr(rhs));
            p
        }
        StaticCondition::HasCounters { .. } => reads_src_of(StateKind::ObjectCounters),
        StaticCondition::IsTapped { scope, .. } => read_object_scope(scope, StateKind::TapState),
        StaticCondition::SourceIsTapped => reads_src_of(StateKind::TapState),
        StaticCondition::OpponentPoisonAtLeast { count: _ } => {
            reads_player_of(StateKind::PlayerLife)
        }
        StaticCondition::SpellCastWithVariantThisTurn { .. } => {
            reads_player_of(StateKind::JournalCast)
        }
        StaticCondition::SourceMatchesFilter { filter: _ } => reads_src_of(StateKind::ObjectPt),
        StaticCondition::And { conditions } | StaticCondition::Or { conditions } => {
            let mut p = RwProfile::empty();
            for c in conditions {
                p.merge(rw_static_condition(c));
            }
            p
        }
        StaticCondition::Not { condition } => rw_static_condition(condition),
        StaticCondition::UnlessPay { .. } => RwProfile::conservative(),
        StaticCondition::SourceAttackingAlone
        | StaticCondition::SourceIsAttacking
        | StaticCondition::SourceIsBlocking
        | StaticCondition::SourceIsBlocked
        | StaticCondition::SourceEnteredThisTurn
        | StaticCondition::SourceHasDealtDamage
        | StaticCondition::SourceIsSaddled
        | StaticCondition::SourceIsEquipped
        | StaticCondition::SourceIsEnchanted
        | StaticCondition::SourceIsMonstrous
        | StaticCondition::SourceIsHarnessed
        | StaticCondition::SourceAttachedToCreature
        | StaticCondition::SourceIsPaired
        | StaticCondition::SourceInZone { .. }
        | StaticCondition::WasStartingPlayer { .. } => frozen_source_read(),
        StaticCondition::RecipientHasCounters { .. }
        | StaticCondition::RecipientMatchesFilter { .. }
        | StaticCondition::RecipientAttackingOwnerTarget { .. } => RwProfile::empty(),
        StaticCondition::ChosenColorIs { .. }
        | StaticCondition::ChosenLabelIs { .. }
        | StaticCondition::HasMaxSpeed
        | StaticCondition::SpeedGE { .. }
        | StaticCondition::DayNightIs { .. }
        | StaticCondition::CastVariantPaid { .. }
        | StaticCondition::ClassLevelGE { .. }
        | StaticCondition::IsMonarch
        | StaticCondition::IsInitiative
        | StaticCondition::NoMonarch
        | StaticCondition::HasCityBlessing
        | StaticCondition::CompletedADungeon
        | StaticCondition::Unrecognized { .. }
        | StaticCondition::DuringYourTurn
        | StaticCondition::WasCast { .. }
        | StaticCondition::IsRingBearer
        | StaticCondition::RingLevelAtLeast { .. }
        | StaticCondition::ControlsCommander { .. }
        | StaticCondition::SourceControllerEquals { .. }
        | StaticCondition::EnchantedIsFaceDown
        | StaticCondition::AdditionalCostPaid
        | StaticCondition::CastingAsVariant { .. }
        | StaticCondition::None => RwProfile::empty(),
    }
}

// ---------------------------------------------------------------------------
// Filter / player / scope reads.
// ---------------------------------------------------------------------------

/// A filter used as a READ carrier (target_chooser, nested filters). Selectors
/// are read-free; event-context refs contribute event reads (and D5 flags for
/// the 12 tags). Composite filters descend to catch nested event refs.
fn rw_target_filter(x: &TargetFilter) -> RwProfile {
    match x {
        // D5 carriers (9 TargetFilter tags of the 12).
        TargetFilter::TriggeringSpellController
        | TargetFilter::TriggeringSpellOwner
        | TargetFilter::TriggeringPlayer
        | TargetFilter::TriggeringSource
        | TargetFilter::ParentTarget
        | TargetFilter::ParentTargetController
        | TargetFilter::ParentTargetOwner
        | TargetFilter::StackSpell
        | TargetFilter::CostPaidObject => legacy_ref(),
        // Non-D5 event refs. `ParentTargetSlot` is NOT one of the 12 retained tags
        // (it serializes as an object key the frozen serde oracle never matched —
        // the write path `target_is_legacy_ref` excludes it too), so it must NOT
        // set `legacy_batch_prompt`; it is a live event read like the others here.
        TargetFilter::ParentTargetSlot { .. }
        | TargetFilter::EventTarget
        | TargetFilter::TriggeringSourceController
        | TargetFilter::PostReplacementSourceController
        | TargetFilter::PostReplacementDamageTarget
        | TargetFilter::PostReplacementDamageTargetOwner
        | TargetFilter::ChosenDamageSource => reads_event_live(),
        TargetFilter::Not { filter } | TargetFilter::TrackedSetFiltered { filter, .. } => {
            rw_target_filter(filter)
        }
        TargetFilter::And { filters } | TargetFilter::Or { filters } => {
            let mut p = RwProfile::empty();
            for f in filters {
                p.merge(rw_target_filter(f));
            }
            p
        }
        TargetFilter::None
        | TargetFilter::Any
        | TargetFilter::Player
        | TargetFilter::Controller
        | TargetFilter::SelfRef
        | TargetFilter::SourceOrPaired
        | TargetFilter::Typed(..)
        | TargetFilter::StackAbility { .. }
        | TargetFilter::SpecificObject { .. }
        | TargetFilter::SpecificPlayer { .. }
        | TargetFilter::Neighbor { .. }
        | TargetFilter::ScopedPlayer
        | TargetFilter::AttachedTo
        | TargetFilter::LastCreated
        | TargetFilter::LastRevealed
        | TargetFilter::ChosenCard
        | TargetFilter::TrackedSet { .. }
        | TargetFilter::ExiledBySource
        | TargetFilter::ExiledCardByIndex { .. }
        | TargetFilter::SourceChosenPlayer
        | TargetFilter::OriginalController
        | TargetFilter::DefendingPlayer
        | TargetFilter::HasChosenName
        | TargetFilter::Named { .. }
        | TargetFilter::Owner
        | TargetFilter::AllPlayers => RwProfile::empty(),
    }
}

fn rw_player_filter(x: &PlayerFilter) -> RwProfile {
    match x {
        PlayerFilter::OpponentLostLife | PlayerFilter::OpponentGainedLife => {
            reads_player_of(StateKind::JournalLife)
        }
        PlayerFilter::OpponentDealtCombatDamage { source: _ } => {
            reads_player_of(StateKind::JournalLife)
        }
        // D5 carrier.
        PlayerFilter::TriggeringPlayer => legacy_ref(),
        PlayerFilter::OpponentOtherThanTriggering
        | PlayerFilter::OpponentOfTriggeringPlayer
        | PlayerFilter::OpponentOfTriggeringPlayerNotAttacked
        | PlayerFilter::ParentObjectTargetController
        | PlayerFilter::ParentObjectTargetOwner => reads_event_live(),
        PlayerFilter::ControlsCount {
            filter,
            count,
            relation: _,
            comparator: _,
        } => {
            let mut p = board_membership_read(filter);
            p.merge(rw_quantity_expr(count));
            p
        }
        PlayerFilter::PlayerAttribute {
            attr,
            value,
            relation: _,
            comparator: _,
        } => {
            let mut p = rw_quantity_ref(attr);
            p.merge(rw_quantity_expr(value));
            p
        }
        PlayerFilter::AllExcept { exclude } => rw_player_filter(exclude),
        PlayerFilter::Controller
        | PlayerFilter::Opponent
        | PlayerFilter::DefendingPlayer
        | PlayerFilter::HasLostTheGame
        | PlayerFilter::OpponentAttacked { .. }
        | PlayerFilter::All
        | PlayerFilter::HighestSpeed
        | PlayerFilter::ZoneChangedThisWay
        | PlayerFilter::PerformedActionThisWay { .. }
        | PlayerFilter::OwnersOfCardsExiledBySource
        | PlayerFilter::VotedFor { .. }
        | PlayerFilter::ChosenPlayer { .. } => RwProfile::empty(),
    }
}

fn rw_player_scope(x: &PlayerScope) -> RwProfile {
    match x {
        PlayerScope::ParentObjectTargetController => reads_event_live(),
        PlayerScope::AllPlayers { exclude, .. } => match exclude {
            Some(e) => rw_player_scope(e),
            None => RwProfile::empty(),
        },
        PlayerScope::Controller
        | PlayerScope::ScopedPlayer
        | PlayerScope::Target
        | PlayerScope::Opponent { .. }
        | PlayerScope::RecipientController
        | PlayerScope::DefendingPlayer
        | PlayerScope::SourceChosenPlayer => RwProfile::empty(),
    }
}

fn rw_controller_ref(x: &ControllerRef) -> RwProfile {
    match x {
        // D5 carriers.
        ControllerRef::ParentTargetController
        | ControllerRef::ParentTargetOwner
        | ControllerRef::TriggeringPlayer => legacy_ref(),
        ControllerRef::You
        | ControllerRef::Opponent
        | ControllerRef::ScopedPlayer
        | ControllerRef::TargetPlayer
        | ControllerRef::DefendingPlayer
        | ControllerRef::ChosenPlayer { .. }
        | ControllerRef::SourceChosenPlayer
        | ControllerRef::EnchantedPlayer => RwProfile::empty(),
    }
}

// ---------------------------------------------------------------------------
// N-E unit pairings (§5.4). Build ASTs directly; assert profile+conflict.
// Each pairing is discriminating: the paired assertions bracket exactly one
// classification decision (see the revert-fail table in the impl report).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ability::{AbilityKind, Comparator, PtValue};
    use crate::types::counter::CounterType;
    use crate::types::identifiers::ObjectId;
    use crate::types::player::PlayerId;

    // ---- builders ----
    fn ra(effect: Effect) -> ResolvedAbility {
        ResolvedAbility::new(effect, vec![], ObjectId(1), PlayerId(0))
    }
    fn cond(mut a: ResolvedAbility, c: AbilityCondition) -> ResolvedAbility {
        a.condition = Some(c);
        a
    }
    fn qfix(v: i32) -> QuantityExpr {
        QuantityExpr::Fixed { value: v }
    }
    fn qref(r: QuantityRef) -> QuantityExpr {
        QuantityExpr::Ref { qty: r }
    }
    fn creature() -> TargetFilter {
        TargetFilter::Typed(TypedFilter::creature())
    }
    fn sub(t: &str) -> TargetFilter {
        TargetFilter::Typed(TypedFilter::creature().subtype(t.to_string()))
    }
    fn power_src() -> QuantityRef {
        QuantityRef::Power {
            scope: ObjectScope::Source,
        }
    }
    fn tough_recip() -> QuantityRef {
        QuantityRef::Toughness {
            scope: ObjectScope::Recipient,
        }
    }
    fn counters_src() -> QuantityRef {
        QuantityRef::CountersOn {
            scope: ObjectScope::Source,
            counter_type: None,
        }
    }
    fn obj_count(f: TargetFilter) -> QuantityRef {
        QuantityRef::ObjectCount { filter: f }
    }
    fn put_counter_all(count: QuantityExpr, target: TargetFilter) -> Effect {
        Effect::PutCounterAll {
            count,
            target,
            counter_type: CounterType::Plus1Plus1,
        }
    }
    fn put_counter(count: QuantityExpr, target: TargetFilter) -> Effect {
        Effect::PutCounter {
            count,
            target,
            counter_type: CounterType::Plus1Plus1,
        }
    }
    fn remove_counter(target: TargetFilter) -> Effect {
        Effect::RemoveCounter {
            counter_type: None,
            count: qfix(1),
            target,
        }
    }
    fn gain_life(amount: QuantityExpr) -> Effect {
        Effect::GainLife {
            amount,
            player: TargetFilter::Controller,
        }
    }
    fn sacrifice_self() -> Effect {
        Effect::Sacrifice {
            target: TargetFilter::SelfRef,
            count: qfix(1),
            min_count: 0,
        }
    }
    fn token(types: &[&str], count: QuantityExpr) -> Effect {
        Effect::Token {
            name: "t".into(),
            power: PtValue::Fixed(1),
            toughness: PtValue::Fixed(1),
            types: types.iter().map(|s| s.to_string()).collect(),
            colors: vec![],
            keywords: vec![],
            tapped: false,
            count,
            owner: TargetFilter::Controller,
            attach_to: None,
            enters_attacking: false,
            supertypes: vec![],
            static_abilities: vec![],
            enter_with_counters: vec![],
        }
    }
    fn change_zone(origin: Option<Zone>, dest: Zone, target: TargetFilter) -> Effect {
        Effect::ChangeZone {
            origin,
            destination: dest,
            target,
            owner_library: false,
            enter_transformed: false,
            enters_under: None,
            enter_tapped: crate::types::zones::EtbTapState::default(),
            enters_attacking: false,
            up_to: false,
            enter_with_counters: vec![],
            conditional_enter_with_counters: vec![],
            face_down_profile: None,
            enters_modified_if: None,
        }
    }
    fn copy_spell(target: TargetFilter) -> Effect {
        Effect::CopySpell {
            target,
            retarget: crate::types::ability::CopyRetargetPermission::KeepOriginalTargets,
            copier: None,
            additional_modifications: vec![],
            starting_loyalty_from_casualty_sacrifice: false,
        }
    }
    fn qcheck(lhs: QuantityRef, rhs: i32) -> AbilityCondition {
        AbilityCondition::QuantityCheck {
            lhs: qref(lhs),
            rhs: qfix(rhs),
            comparator: Comparator::GE,
        }
    }

    // ---- group structures ----
    fn se() -> GroupStructure {
        gs(true, false, false, false, true, SourceCensus::unknown())
    }
    fn se_phase() -> GroupStructure {
        gs(true, false, false, false, false, SourceCensus::unknown())
    }
    fn se_disjoint() -> GroupStructure {
        gs(true, false, false, true, true, SourceCensus::unknown())
    }
    fn batch() -> GroupStructure {
        gs(false, false, true, false, true, SourceCensus::unknown())
    }
    fn gs(
        same_event: bool,
        all_same_source: bool,
        self_departed: bool,
        excludes: bool,
        present: bool,
        source_census: SourceCensus,
    ) -> GroupStructure {
        GroupStructure {
            same_event,
            all_same_source,
            all_sources_self_departed: self_departed,
            event_object_excludes_sources: excludes,
            event_object_present: present,
            source_census,
        }
    }

    fn conflicts(a: &ResolvedAbility, s: &GroupStructure) -> bool {
        profiles_conflict(&ability_rw_profile(a), s)
    }

    // ===================== base shapes =====================

    #[test]
    fn base_chaotic_goo_flipcoin_self_counters_clean() {
        // FlipCoin{win: PutCounter(SelfRef), lose: RemoveCounter(SelfRef)} — no
        // reads ⇒ clean even though the self-writes disable source-independence.
        let e = Effect::FlipCoin {
            win_effect: Some(Box::new(def(put_counter(qfix(1), TargetFilter::SelfRef)))),
            lose_effect: Some(Box::new(def(remove_counter(TargetFilter::SelfRef)))),
            flipper: TargetFilter::Controller,
        };
        let p = ability_rw_profile(&ra(e));
        assert!(
            p.writes_self.object_counters,
            "FlipCoin body descends to self-counter write"
        );
        assert!(!profiles_conflict(&p, &se_phase()));
    }

    #[test]
    fn base_gutter_grime_live_src_counter_read_vs_token_membership_clean() {
        // Observer alive: CountersOn{Source} LIVE read × token membership write —
        // counters vs membership don't feed (T3).
        let e = token(&["Creature", "Ooze"], qref(counters_src()));
        assert!(!conflicts(&ra(e), &batch()));
    }

    #[test]
    fn base_mana_crypt_flipcoin_player_damage_clean() {
        let e = Effect::FlipCoin {
            win_effect: None,
            lose_effect: Some(Box::new(def(Effect::DealDamage {
                amount: qfix(3),
                target: TargetFilter::Controller,
                damage_source: None,
                excess: None,
            }))),
            flipper: TargetFilter::Controller,
        };
        assert!(!conflicts(&ra(e), &se_phase()));
    }

    #[test]
    fn base_fruit_src_pt_read_vs_life_write_clean() {
        // Toughness{Source} read × life write — no feed (ObjectPt vs PlayerLife).
        let e = gain_life(qref(QuantityRef::Toughness {
            scope: ObjectScope::Source,
        }));
        assert!(!conflicts(&ra(e), &se()));
    }

    #[test]
    fn base_quirion_dryad_write_only_self_counter_clean() {
        assert!(!conflicts(
            &ra(put_counter(qfix(1), TargetFilter::SelfRef)),
            &se()
        ));
    }

    #[test]
    fn base_copyspell_selfref_clean_vs_topofstack_conflict() {
        // Walk-classification discriminator (D4, CR 707.10/707.10c): SelfRef/
        // explicit reads the original by id (no board read); the untargeted
        // fallback reads the MUTABLE stack top.
        let self_p = ability_rw_profile(&ra(copy_spell(TargetFilter::SelfRef)));
        let fallback_p = ability_rw_profile(&ra(copy_spell(TargetFilter::Any)));
        assert!(!self_p.reads_board.stack_shape, "SelfRef reads by id");
        assert!(
            fallback_p.reads_board.stack_shape,
            "fallback reads the mutable stack top"
        );
        // Under same-event the two identical source-independent copies commute
        // (f∘f) ⇒ both auto. The fallback's mutable-read hazard is order-relevant
        // on the distinct-event path, where the fallback conflicts and SelfRef
        // stays clean — the classification distinction made observable.
        assert!(conflicts(&ra(copy_spell(TargetFilter::Any)), &batch()));
        assert!(!conflicts(&ra(copy_spell(TargetFilter::SelfRef)), &batch()));
    }

    #[test]
    fn base_case_a_live_power_read_vs_board_counter_write_conflict() {
        // "put +1/+1 on each creature; draw if power>=6" — live Power{Source}
        // read × PutCounterAll board write; counters feed P/T.
        let a = cond(
            ra(put_counter_all(qfix(1), creature())),
            qcheck(power_src(), 6),
        );
        assert!(conflicts(&a, &se()));
    }

    #[test]
    fn base_graveyard_return_board_membership_conflict() {
        // board creature-count read × return-to-battlefield membership write,
        // census overlap (creature).
        let a = cond(
            ra(change_zone(
                Some(Zone::Graveyard),
                Zone::Battlefield,
                creature(),
            )),
            qcheck(obj_count(creature()), 1),
        );
        assert!(conflicts(&a, &batch()));
    }

    // ===================== (i) Mana × unless-pay guard =====================

    #[test]
    fn ne_i_mana_unless_pay_guard() {
        // echo: unless-pay + self-sac, NO pool write ⇒ clean.
        let mut echo = ability_rw_profile(&ra(sacrifice_self()));
        echo.has_pay_or_unless = true;
        assert!(!echo.writes_pool);
        assert!(!profiles_conflict(&echo, &se()));
        // synthetic: pool write + unless-pay ⇒ combination guard ⇒ conflict.
        let mut synth = RwProfile::empty();
        synth.writes_pool = true;
        synth.has_pay_or_unless = true;
        assert!(profiles_conflict(&synth, &se()));
    }

    // ===================== (ii) Recipient vs Source =====================

    #[test]
    fn ne_ii_recipient_vs_source() {
        // Canopy Gargantuan: Toughness{Recipient} ⇒ read-modify-write, no
        // sibling-read record ⇒ clean.
        let gargantuan = ra(put_counter_all(qref(tough_recip()), creature()));
        assert!(!conflicts(&gargantuan, &se()));
        // Ouroboroid: Power{Source} live read × PutCounterAll external write ⇒
        // counters feed P/T ⇒ conflict.
        let ouroboroid = ra(put_counter_all(qref(power_src()), creature()));
        assert!(conflicts(&ouroboroid, &se()));
    }

    // ===================== (iii) T1 completion =====================

    #[test]
    fn ne_iii_t1_source_independence() {
        // Endless Ranks: board count read × token membership write, no self
        // write ⇒ source-independent ⇒ same-event fast path ⇒ clean.
        let endless = ra(token(
            &["Creature", "Zombie"],
            qref(obj_count(sub("Zombie"))),
        ));
        assert!(ability_rw_profile(&endless).source_independent());
        assert!(!conflicts(&endless, &se()));
        // + a self-counter rider ⇒ source-DEPENDENT ⇒ falls to the board row ⇒
        // census overlap (zombie) ⇒ conflict.
        let mut dependent = endless.clone();
        dependent = dependent.sub_ability(ra(put_counter(qfix(1), TargetFilter::SelfRef)));
        assert!(!ability_rw_profile(&dependent).source_independent());
        assert!(conflicts(&dependent, &se()));
    }

    // ===================== (iv) census overlap =====================

    #[test]
    fn ne_iv_census_overlap() {
        // Pestilence: creature-count read × self-sac of an ENCHANTMENT source ⇒
        // census-disjoint ⇒ clean.
        let pestilence = cond(ra(sacrifice_self()), qcheck(obj_count(creature()), 1));
        let s = gs(
            true,
            false,
            false,
            false,
            false,
            SourceCensus::from_tags(["Enchantment".to_string()]),
        );
        assert!(!profiles_conflict(&ability_rw_profile(&pestilence), &s));
        // Docent: Wizard-count read × Wizard-token write + self-transform ⇒
        // overlap ⇒ conflict.
        let docent = cond(
            ra(token(&["Creature", "Wizard"], qfix(1))).sub_ability(ra(Effect::Transform {
                target: TargetFilter::SelfRef,
            })),
            qcheck(obj_count(sub("Wizard")), 1),
        );
        assert!(conflicts(&docent, &se()));
    }

    #[test]
    fn major1_unfiltered_zone_membership_read_conflicts() {
        // MAJOR-1: a whole-zone `GraveyardSize` read carries census `Any`
        // (unextractable ⇒ overlap assumed). "return all creature cards from
        // your graveyard to the battlefield; draw if graveyard has >=3 cards" —
        // board GraveyardSize read × sibling return-to-battlefield membership
        // write ⇒ census-overlap conflict on the departure-batch path (where the
        // same-event f∘f short-circuit does not apply).
        let a = cond(
            ra(change_zone(
                Some(Zone::Graveyard),
                Zone::Battlefield,
                creature(),
            )),
            qcheck(
                QuantityRef::GraveyardSize {
                    player: PlayerScope::Controller,
                },
                3,
            ),
        );
        assert!(conflicts(&a, &batch()));
    }

    #[test]
    fn zone_census_battlefield_write_vs_graveyard_read_discriminates() {
        // Tombstone Stairwell: a battlefield Zombie-token creation (Token{creature}
        // ⇒ SetMembership dest = Battlefield) whose COUNT reads the GRAVEYARD
        // creature count. The write and the read overlap on TYPE (creature) but
        // their ZONES are disjoint (CR 400.1: a fresh token touches only the
        // battlefield; the count reads the graveyard) ⇒ no feed ⇒ clean. The
        // frozen source condition (`SourceEnteredThisTurn`) only disables the T1
        // source-independent fast path so the feed rows are reached — exactly what
        // Tombstone's `SourceInZone{Battlefield}` intervening-if does.
        let in_zone = |z: Zone| {
            let mut tf = TypedFilter::creature();
            tf.properties.push(FilterProp::InZone { zone: z });
            TargetFilter::Typed(tf)
        };
        let disjoint = cond(
            ra(token(
                &["Creature"],
                qref(obj_count(in_zone(Zone::Graveyard))),
            )),
            AbilityCondition::SourceEnteredThisTurn,
        );
        assert!(
            !conflicts(&disjoint, &se()),
            "battlefield token write × GRAVEYARD creature-count read ⇒ zone-disjoint ⇒ clean"
        );

        // The SAME read/write with the count scoped to the BATTLEFIELD (matching
        // zones) ⇒ census AND zone overlap ⇒ conflict. This is the discriminating
        // witness: a zone-BLIND census would report the disjoint pairing as a
        // conflict too, so dropping the zone check flips the first assertion.
        let same_zone = cond(
            ra(token(
                &["Creature"],
                qref(obj_count(in_zone(Zone::Battlefield))),
            )),
            AbilityCondition::SourceEnteredThisTurn,
        );
        assert!(
            conflicts(&same_zone, &se()),
            "battlefield token write × BATTLEFIELD creature-count read ⇒ same zone ⇒ conflict"
        );
    }

    // ===================== (v) chain-root =====================

    #[test]
    fn ne_v_chain_root() {
        // Smoldering Egg: PutCounter{SelfRef} → RemoveCounter{ParentTarget} +
        // CountersOn{Source} read ⇒ chain root SelfRef ⇒ self-write ⇒ clean.
        let egg = cond(
            ra(put_counter(qfix(1), TargetFilter::SelfRef))
                .sub_ability(ra(remove_counter(TargetFilter::ParentTarget))),
            qcheck(counters_src(), 1),
        );
        assert!(!conflicts(&egg, &se()));
        // Re-rooted at a Typed filter (root NOT SelfRef) ⇒ external counter write
        // × live src-counter read ⇒ conflict.
        let rerooted = cond(
            ra(put_counter(qfix(1), creature()))
                .sub_ability(ra(remove_counter(TargetFilter::ParentTarget))),
            qcheck(counters_src(), 1),
        );
        assert!(conflicts(&rerooted, &se()));
    }

    // ===================== (vi) event-object disjointness =====================

    #[test]
    fn ne_vi_event_object_disjointness() {
        // Railway Brawler: PutCounter{TriggeringSource, count: Power{Source}} with
        // a source-excluding valid_card ⇒ event-object write excluded from
        // src-read scoping ⇒ clean.
        let brawler = ra(put_counter(
            qref(power_src()),
            TargetFilter::TriggeringSource,
        ));
        assert!(!conflicts(&brawler, &se_disjoint()));
        // Without source-exclusion the event object can be a sibling's source ⇒
        // external counter write feeds the live Power{Source} read ⇒ conflict.
        assert!(conflicts(&brawler, &se()));
    }

    // ===================== (vii) parentless-root =====================

    #[test]
    fn ne_vii_parentless_root() {
        // Root Bounce{ParentTarget} (parentless) + a SelfRef-scoped membership
        // read. On a ZoneChanged trigger the referent is the EVENT object.
        let ast = cond(
            ra(Effect::Bounce {
                target: TargetFilter::ParentTarget,
                destination: None,
                selection: crate::types::ability::BounceSelection::Targeted,
            }),
            qcheck(
                obj_count(TargetFilter::And {
                    filters: vec![TargetFilter::SelfRef],
                }),
                1,
            ),
        );
        // (a) source-excluding valid_card ⇒ rule 2 clears ⇒ clean.
        assert!(!conflicts(&ast, &se_disjoint()));
        // (b) no exclusion ⇒ event object can be a sibling source ⇒ conflict.
        assert!(conflicts(&ast, &se()));
        // (c) Phase trigger (no event object) ⇒ None ⇒ no write ⇒ clean.
        assert!(!conflicts(&ast, &se_phase()));
    }

    // ---- test-local helper ----
    fn def(effect: Effect) -> AbilityDefinition {
        AbilityDefinition::new(AbilityKind::Spell, effect)
    }
}
