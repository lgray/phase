//! CR 111.1 + CR 111.10 + CR 111.4: Debug-only catalog of pre-defined token
//! presets. Sourced from `crates/engine/data/known-tokens.toml` (committed
//! phase-native source generated from MTGJSON set token data by the
//! `tokens-gen` bin), converted to JSON at build time by `build.rs`, and
//! embedded via `include_bytes!` — the debug-mode toml_edit parse of the
//! ~5.9MB catalog cost ~1s per test process under nextest's
//! process-per-test model.
//!
//! The catalog is a fixed engine resource — versioned with code. Runtime
//! token-art resolution, named-token parsing, token ability materialization,
//! and the debug-create UI all consume this single engine-typed list of
//! bodies.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::types::card::TokenImageRef;
use crate::types::game_state::GameState;
use crate::types::identifiers::ObjectId;
use crate::types::proposed_event::TokenCharacteristics;

/// CR 111.10: Stable identifier for predefined-ability artifact tokens. Each
/// variant maps to one arm of `effects::token::predefined_token_abilities`,
/// keyed by subtype string. The cross-reference is asserted in tests so a
/// preset's `category` cannot drift from the runtime ability registry.
///
/// Eldrazi Spawn (also keyed by `predefined_token_abilities`) is *not*
/// listed here — Spawn is a Creature subtype, not an artifact token, so
/// `TokenCategory::Creature` covers it. The engine still attaches the
/// spawn ability at create-time via the same subtype-keyed dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredefinedTokenKind {
    Treasure,
    Food,
    Gold,
    Clue,
    Blood,
    Powerstone,
    Map,
    Lander,
}

impl PredefinedTokenKind {
    /// The subtype string consulted by
    /// `effects::token::predefined_token_abilities` at create-token time.
    pub fn subtype_str(&self) -> &'static str {
        match self {
            Self::Treasure => "Treasure",
            Self::Food => "Food",
            Self::Gold => "Gold",
            Self::Clue => "Clue",
            Self::Blood => "Blood",
            Self::Powerstone => "Powerstone",
            Self::Map => "Map",
            Self::Lander => "Lander",
        }
    }
}

/// CR 110.4 dispatch for debug grouping. Exhaustive over the shapes the
/// `tokens-gen` converter produces; the converter errors out on any entry
/// that cannot be classified, forcing this enum to grow deliberately rather
/// than via an `Other` catch-all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenCategory {
    /// CR 111.10: Predefined artifact tokens whose abilities are attached at
    /// runtime by `predefined_token_abilities`.
    PredefinedArtifact { kind: PredefinedTokenKind },
    /// CR 302.1: Any token with the Creature core type.
    Creature,
    /// CR 303.1 + CR 303.4: Aura enchantment token (Roles, Curses, etc.).
    Aura,
    /// CR 301.1 + CR 301.5: Equipment artifact token.
    Equipment,
    /// CR 311.1: Vehicle artifact token.
    Vehicle,
    /// CR 303.1: Non-Aura enchantment token.
    Enchantment,
    /// CR 305.1: Land token (manlands, etc.).
    Land,
    /// CR 301.1: Plain artifact token that isn't Equipment, Vehicle, or a
    /// predefined-ability subtype (Book artifacts, custom curiosities, etc.).
    Artifact,
}

/// How completely this preset's body represents its source data.
/// `Full` means a vanilla body + simple keywords + (for predefined-ability
/// subtypes) the engine-attached abilities cover the printed rules text.
/// `PartialMissingAbilities` flags presets where the source entry has
/// Trigger/Activated/PermanentLayerEffect/Equip rule trees that phase.rs
/// cannot yet model — debug spawn produces the body without those rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetFidelity {
    Full,
    PartialMissingAbilities,
}

/// Catalog-only provenance for token P/T values. Runtime token creation still
/// uses concrete `TokenCharacteristics`; this field records when MTGJSON's
/// token entry used source-defined or dynamic P/T text that cannot be widened
/// into a fixed body without inventing rules text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TokenPtProvenance {
    #[default]
    FixedOrAbsent,
    SourceDefinedOrDynamic {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        power: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        toughness: Option<String>,
    },
}

impl TokenPtProvenance {
    pub fn is_fixed_or_absent(&self) -> bool {
        matches!(self, Self::FixedOrAbsent)
    }

    fn is_source_defined_or_dynamic(&self) -> bool {
        matches!(self, Self::SourceDefinedOrDynamic { .. })
    }
}

/// A single debug-spawnable preset. `body` is shared with `TokenSpec`'s
/// characteristics — single source of truth on the body shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPreset {
    pub id: String,
    pub category: TokenCategory,
    pub fidelity: PresetFidelity,
    #[serde(default, skip_serializing_if = "TokenPtProvenance::is_fixed_or_absent")]
    pub pt_provenance: TokenPtProvenance,
    pub body: TokenCharacteristics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_card_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_card_refs: Vec<TokenSourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_image_ref: Option<TokenImageRef>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub set_code: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub set_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collector_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub type_line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenSourceRef {
    pub card_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scryfall_oracle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scryfall_id: Option<String>,
}

#[derive(Deserialize)]
struct CatalogFile {
    token: Vec<TokenPreset>,
}

/// What `tokens-gen` did with one `known-tokens.overlay.toml` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayRowOutcome {
    /// Nothing in the generated catalog claims this token; the row is in.
    Applied,
    /// The row is in, but a generated preset shares its set and token name
    /// without sharing a source card. Either MTGJSON published this token
    /// without linking it back to the card that creates it — in which case the
    /// row is now stale — or the name simply collides. A human decides which.
    AppliedShadowed { by_id: String },
    /// MTGJSON now ships this token; the generated preset wins and the row is
    /// dropped. Delete it from the overlay.
    Superseded { by_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayRowReport {
    pub overlay_id: String,
    pub set_code: String,
    pub display_name: String,
    pub outcome: OverlayRowOutcome,
}

/// The overlay row's `source_card_refs` entries that name a Scryfall oracle id.
fn row_source_oracle_ids(row: &TokenPreset) -> impl Iterator<Item = &str> {
    row.source_card_refs
        .iter()
        .filter_map(|source_ref| source_ref.scryfall_oracle_id.as_deref())
}

/// Same set and same token name, case-insensitively — the same case folding
/// `known_token_body_by_name_for_source` already applies to display names.
fn names_same_token(a: &TokenPreset, b: &TokenPreset) -> bool {
    a.set_code == b.set_code
        && a.body
            .display_name
            .eq_ignore_ascii_case(&b.body.display_name)
}

/// Merge hand-authored presets into a generated catalog.
///
/// `known-tokens.toml` is rewritten whole on every regen, so a row MTGJSON has
/// no source for cannot survive in it; `known-tokens.overlay.toml` is the
/// authored input that does. A row is superseded when the catalog already holds
/// a preset with the same `id` — the identity `PRESETS` requires to be unique —
/// or with the same token *identity*: same set, same display name (case
/// insensitively) and a shared `source_card_refs` `scryfall_oracle_id`. An
/// overlay row's own `id` is a placeholder the real MTGJSON entry will not
/// share, so the id leg alone cannot supersede anything; the creating card is
/// what makes the name leg an identity rather than a label.
///
/// The scan runs over the growing output, so overlay rows deduplicate against
/// each other exactly as they do against generated data.
///
/// Returns the merged catalog in ascending-`id` order — the order `tokens-gen`
/// serializes and `catalog_is_ordered_and_self_consistent` asserts — with one
/// report per overlay row.
pub fn merge_overlay(
    generated: Vec<TokenPreset>,
    overlay: Vec<TokenPreset>,
) -> Result<(Vec<TokenPreset>, Vec<OverlayRowReport>), String> {
    let mut presets = generated;
    let mut reports = Vec::with_capacity(overlay.len());

    for row in overlay {
        // A row with no source-card oracle id can never be superseded, so it
        // would outlive the data it stands in for. Refuse it at the boundary.
        if row_source_oracle_ids(&row).next().is_none() {
            return Err(format!(
                "overlay row `{}` ({} / {}) carries no `source_card_refs` entry with a \
                 `scryfall_oracle_id`; a hand-authored row must name the card it stands in \
                 for, since that reference is what lets MTGJSON's entry supersede the row",
                row.id, row.set_code, row.body.display_name
            ));
        }

        let mut superseded_by: Option<String> = None;
        let mut shadowed_by: Option<String> = None;
        for preset in &presets {
            let same_name = names_same_token(preset, &row);
            if preset.id == row.id
                || (same_name
                    && row_source_oracle_ids(&row)
                        .any(|oracle_id| token_preset_has_source_ref(preset, oracle_id, None)))
            {
                superseded_by = Some(preset.id.clone());
                break;
            }
            if same_name && shadowed_by.is_none() {
                shadowed_by = Some(preset.id.clone());
            }
        }

        let overlay_id = row.id.clone();
        let set_code = row.set_code.clone();
        let display_name = row.body.display_name.clone();
        let outcome = match superseded_by {
            Some(by_id) => OverlayRowOutcome::Superseded { by_id },
            None => {
                let outcome = match shadowed_by {
                    Some(by_id) => OverlayRowOutcome::AppliedShadowed { by_id },
                    None => OverlayRowOutcome::Applied,
                };
                presets.push(row);
                outcome
            }
        };
        reports.push(OverlayRowReport {
            overlay_id,
            set_code,
            display_name,
            outcome,
        });
    }

    presets.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((presets, reports))
}

/// Embedded catalog data: `data/known-tokens.toml` converted to JSON in
/// `OUT_DIR` by `build.rs` (structural conversion — same serde shape).
static PRESETS: LazyLock<Vec<TokenPreset>> = LazyLock::new(|| {
    let raw = include_bytes!(concat!(env!("OUT_DIR"), "/known-tokens.json"));
    let parsed: CatalogFile =
        serde_json::from_slice(raw).expect("build.rs-converted known-tokens.json well-formed");
    // Duplicate-id assertion: every preset must be addressable by a unique
    // stable id (used by the FE for selection state and React keys).
    let mut seen = std::collections::HashSet::new();
    for p in &parsed.token {
        assert!(
            seen.insert(p.id.clone()),
            "known-tokens.toml: duplicate preset id `{}`",
            p.id
        );
    }
    parsed.token
});

/// Returns the full set of debug-spawnable token presets, in catalog order:
/// ascending id, which is what `tokens-gen` emits so that a regen produces a
/// minimal diff (asserted by `catalog_is_ordered_and_self_consistent`). Ids are
/// MTGJSON uuids, so this is not a display order — consumers impose their own
/// (the debug UI groups by category and sorts by power/toughness/name).
pub fn known_token_presets() -> &'static [TokenPreset] {
    &PRESETS
}

pub fn known_token_preset_by_id(id: &str) -> Option<&'static TokenPreset> {
    known_token_presets().iter().find(|preset| preset.id == id)
}

/// CR 111.4: A token's name and subtype(s) are set by the effect that creates
/// it; for named tokens (Vibranium, Mutavault, …) those characteristics live in
/// the predefined catalog. Resolve the full token body by display name so the
/// Oracle parser can lower `"create a [Name] token"` to a complete
/// `Effect::Token` for the *entire class* of registry-defined named tokens,
/// rather than a hardcoded allowlist. Case-insensitive to match Oracle text
/// casing variance. Returns `None` when a display name maps to multiple distinct
/// bodies (common subtype names like Bear / Angel) and no source context
/// disambiguates the intended token.
pub fn known_token_body_by_name(name: &str) -> Option<&'static TokenCharacteristics> {
    known_token_body_by_name_for_source(name, None)
}

/// Source-scoped variant for Oracle parsing: when a display name is ambiguous,
/// prefer the preset linked to the card currently being parsed. Fall back to a
/// global match only if every matching body is identical.
pub fn known_token_body_by_name_for_source(
    name: &str,
    source_name: Option<&str>,
) -> Option<&'static TokenCharacteristics> {
    let name = name.trim();
    if let Some(source_name) = source_name.map(str::trim).filter(|name| !name.is_empty()) {
        let mut source_matches = known_token_presets().iter().filter(|preset| {
            preset.body.display_name.eq_ignore_ascii_case(name)
                && preset
                    .source_card_names
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(source_name))
        });
        if let Some(first) = source_matches.next() {
            let first_body = &first.body;
            return source_matches
                .all(|preset| &preset.body == first_body)
                .then_some(first_body);
        }
    }

    unique_token_body_by_name(name)
}

fn unique_token_body_by_name(name: &str) -> Option<&'static TokenCharacteristics> {
    let mut matches = known_token_presets()
        .iter()
        .filter(|preset| preset.body.display_name.eq_ignore_ascii_case(name));
    let first = matches.next()?;
    let first_body = &first.body;
    matches
        .all(|preset| &preset.body == first_body)
        .then_some(first_body)
}

pub fn find_exact_token_ref(
    state: &GameState,
    source_id: ObjectId,
    body: &TokenCharacteristics,
) -> Option<TokenImageRef> {
    find_token_ref_with_mode(state, source_id, body, TokenRefMatchMode::Exact)
}

/// CR 111.10 + CR 702.175a: Resolve a card-linked token preset for a copy
/// token when the copied body exactly matches a catalog entry. Unlike
/// [`find_exact_token_ref`], never skips body matching for a sole
/// `source_related_token_ids` link — Twinflame-style copies keep source P/T and
/// must not route through an offspring 1/1 preset.
pub fn find_card_linked_copy_token_ref(
    state: &GameState,
    source_id: ObjectId,
    body: &TokenCharacteristics,
) -> Option<TokenImageRef> {
    find_token_ref_with_mode(state, source_id, body, TokenRefMatchMode::CardLinkedCopy)
}

#[derive(Clone, Copy)]
enum TokenRefMatchMode {
    /// `CreateToken` path: a unique `source_related_token_ids` link may resolve
    /// only with exact body matching unless the source identity is confirmed.
    Exact,
    /// `CreateToken` path after source oracle/face identity has narrowed the
    /// preset. Source-defined P/T presets may then match concrete runtime P/T.
    SourceLinkedExact,
    /// `CopyTokenOf` path: always require an exact body match so copies that
    /// keep source P/T (Twinflame, Populate) do not inherit offspring presets.
    CardLinkedCopy,
}

fn find_token_ref_with_mode(
    state: &GameState,
    source_id: ObjectId,
    body: &TokenCharacteristics,
    mode: TokenRefMatchMode,
) -> Option<TokenImageRef> {
    let source = state.objects.get(&source_id);
    let related_ids = source
        .map(|obj| obj.source_related_token_ids.as_slice())
        .unwrap_or(&[]);
    let source_oracle =
        source.and_then(|obj| obj.printed_ref.as_ref().map(|r| r.oracle_id.as_str()));
    let name_only_offspring_copy = source.is_some_and(|obj| {
        obj.keywords
            .iter()
            .chain(obj.base_keywords.iter())
            .any(|keyword| matches!(keyword, crate::types::keywords::Keyword::Offspring(_)))
    });
    if matches!(mode, TokenRefMatchMode::CardLinkedCopy)
        && related_ids.is_empty()
        && source_oracle.is_none()
        && !name_only_offspring_copy
    {
        return None;
    }
    let source_face = source.and_then(|obj| obj.printed_ref.as_ref().map(|r| r.face_name.as_str()));
    let source_name = source
        .map(|obj| obj.name.as_str())
        .or_else(|| state.lki_cache.get(&source_id).map(|lki| lki.name.as_str()));

    if related_ids.is_empty() && source_oracle.is_none() && source_name.is_none() {
        return None;
    }

    if !related_ids.is_empty() {
        let related_presets: Vec<_> = related_ids
            .iter()
            .filter_map(|id| known_token_preset_by_id(id))
            .collect();

        if matches!(mode, TokenRefMatchMode::Exact) {
            if let [preset] = related_presets.as_slice() {
                let source_matches = source_oracle.is_some_and(|oracle_id| {
                    token_preset_has_source_ref(preset, oracle_id, source_face)
                });
                let match_mode = if source_matches {
                    TokenRefMatchMode::SourceLinkedExact
                } else {
                    TokenRefMatchMode::Exact
                };
                if (source_matches || source_oracle.is_none())
                    && token_preset_body_matches(preset, body, match_mode)
                {
                    return preset.token_image_ref.clone();
                }
                return None;
            }
        }

        let matches: Vec<_> = if let Some(oracle_id) = source_oracle {
            let match_mode = if matches!(mode, TokenRefMatchMode::Exact) {
                TokenRefMatchMode::SourceLinkedExact
            } else {
                mode
            };
            related_presets
                .into_iter()
                .filter(|preset| token_preset_has_source_ref(preset, oracle_id, source_face))
                .filter(|preset| token_preset_body_matches(preset, body, match_mode))
                .collect()
        } else {
            related_presets
                .into_iter()
                .filter(|preset| token_body_matches(&preset.body, body))
                .collect()
        };
        let first = matches.first()?;
        if !matches
            .iter()
            .skip(1)
            .all(|preset| token_preset_semantics_match(first, preset))
        {
            return None;
        }
        return first.token_image_ref.clone();
    }

    let source_gated: Vec<&TokenPreset> = known_token_presets()
        .iter()
        .filter(|preset| {
            if !token_body_matches(&preset.body, body) {
                return false;
            }
            if let Some(oracle_id) = source_oracle {
                return token_preset_has_source_ref(preset, oracle_id, source_face);
            }
            if let Some(name) = source_name {
                return preset
                    .source_card_names
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(name));
            }
            false
        })
        .collect();
    if !source_gated.is_empty() {
        // Same CR 111.10 dedup the related-ids path applies: multiple presets
        // that are semantically identical differ only in printing — the pick
        // is presentation-only, so take the first deterministically instead of
        // silently resolving nothing (a Role exists on two flip sheets, so a
        // source listed on both used to land here).
        return semantically_unique_ref(&source_gated);
    }

    // No preset lists this source. When EVERY body-matching preset is
    // semantically identical, the token's identity is fully determined by its
    // body alone (the live face: a Role token — CR 111.10 fixes all seven
    // kinds by name, and `role_normalized_display_name` already reconciled the
    // engine's "<Role> Role" naming). The source gate would add nothing but a
    // silent `None`, which strands the display on a name search no printing
    // can satisfy (#7552). Bodies with semantically DIFFERENT presets (art
    // variants with different abilities) still resolve nothing here — the
    // gate's ambiguity protection is preserved.
    let body_only: Vec<&TokenPreset> = known_token_presets()
        .iter()
        .filter(|preset| token_body_matches(&preset.body, body))
        .collect();
    semantically_unique_ref(&body_only)
}

/// The shared "all remaining candidates agree" reduction: `Some` ref iff every
/// preset in `matches` is semantically identical to the first — the choice is
/// then presentation-only and deterministic. Empty or disagreeing sets resolve
/// nothing.
fn semantically_unique_ref(matches: &[&TokenPreset]) -> Option<TokenImageRef> {
    let first = matches.first()?;
    if !matches
        .iter()
        .skip(1)
        .all(|preset| token_preset_semantics_match(first, preset))
    {
        return None;
    }
    first.token_image_ref.clone()
}

/// CR 111.10: The engine names a Role token "<Role> Role" (e.g. "Monster Role",
/// the parser/token-creation convention), but catalog presets — generated from
/// MTGJSON — name the same token by its bare role word ("Monster"). Reconcile the
/// trailing " Role" so a "Monster Role" token body matches its "Monster" face
/// preset. Without this a DFC Role token ("Monster // Sorcerer"), whose source
/// card links to BOTH face presets and so skips the single-preset fast path,
/// never resolves an image ref and renders with no art. Non-Role names have no
/// " Role" suffix and are unaffected; the accompanying subtype/type comparison
/// still prevents a Role body from matching a non-Role preset.
fn role_normalized_display_name(name: &str) -> &str {
    name.strip_suffix(" Role").unwrap_or(name)
}

fn token_body_matches(a: &TokenCharacteristics, b: &TokenCharacteristics) -> bool {
    token_body_identity_matches(a, b) && a.power == b.power && a.toughness == b.toughness
}

fn token_body_identity_matches(a: &TokenCharacteristics, b: &TokenCharacteristics) -> bool {
    role_normalized_display_name(&a.display_name) == role_normalized_display_name(&b.display_name)
        && sorted_debug(&a.core_types) == sorted_debug(&b.core_types)
        && sorted_strings(&a.subtypes) == sorted_strings(&b.subtypes)
        && sorted_debug(&a.supertypes) == sorted_debug(&b.supertypes)
        && sorted_debug(&a.colors) == sorted_debug(&b.colors)
        && sorted_debug(&a.keywords) == sorted_debug(&b.keywords)
}

fn token_preset_body_matches(
    preset: &TokenPreset,
    body: &TokenCharacteristics,
    mode: TokenRefMatchMode,
) -> bool {
    token_body_identity_matches(&preset.body, body)
        && ((preset.body.power == body.power && preset.body.toughness == body.toughness)
            || (matches!(mode, TokenRefMatchMode::SourceLinkedExact)
                && preset.pt_provenance.is_source_defined_or_dynamic()))
}

fn token_preset_semantics_match(a: &TokenPreset, b: &TokenPreset) -> bool {
    // Used only to deduplicate source-related presets that already matched the
    // emitted runtime body/rules semantics. P/T provenance is catalog metadata,
    // not a runtime semantic difference after body matching has selected both
    // candidates.
    a.category == b.category
        && a.fidelity == b.fidelity
        && token_body_matches(&a.body, &b.body)
        && a.rules_text == b.rules_text
}

fn token_preset_has_source_ref(
    preset: &TokenPreset,
    oracle_id: &str,
    source_face: Option<&str>,
) -> bool {
    preset.source_card_refs.iter().any(|source_ref| {
        source_ref.scryfall_oracle_id.as_deref() == Some(oracle_id)
            && source_face.is_none_or(|face| {
                source_ref
                    .face_name
                    .as_deref()
                    .is_none_or(|candidate| candidate == face)
            })
    })
}

fn sorted_strings(values: &[String]) -> Vec<&str> {
    let mut out: Vec<&str> = values.iter().map(String::as_str).collect();
    out.sort_unstable();
    out
}

fn sorted_debug<T: std::fmt::Debug>(values: &[T]) -> Vec<String> {
    let mut out: Vec<String> = values.iter().map(|value| format!("{value:?}")).collect();
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::zones::create_object;
    use crate::types::card::PrintedCardRef;
    use crate::types::card_type::CoreType;
    use crate::types::identifiers::CardId;
    use crate::types::keywords::Keyword;
    use crate::types::mana::{ManaColor, ManaCost};
    use crate::types::player::PlayerId;
    use crate::types::zones::Zone;

    const FIXED_OOZE_PRESET_ID: &str = "25b62fd5-b036-5c64-88fd-8f50d0675e4d";
    const SOURCE_DEFINED_OOZE_PRESET_ID: &str = "1545ee29-d9c1-57ff-acae-431cfd6d60cf";
    const ROT_LIKE_ORACLE_ID: &str = "8f47c236-46b6-47cb-9ea9-7adfef8fd8ce";
    const SLIME_MOLDING_ORACLE_ID: &str = "e01c8122-9159-4f28-ac6c-338bd889650e";
    const FANATIC_OF_RHONAS_PRESET_ID: &str = "001dd45c-851b-5eb3-9a53-fc9fb2c0e322";
    const IRIDESCENT_VINELASHER_PRESET_ID: &str = "c39bbf40-9bf9-5400-8be3-0fb961f0a643";

    fn green_ooze_body(power: Option<i32>, toughness: Option<i32>) -> TokenCharacteristics {
        TokenCharacteristics {
            display_name: "Ooze".to_string(),
            power,
            toughness,
            core_types: vec![CoreType::Creature],
            subtypes: vec!["Ooze".to_string()],
            supertypes: Vec::new(),
            colors: vec![ManaColor::Green],
            keywords: Vec::new(),
        }
    }

    fn fanatic_of_rhonas_body() -> TokenCharacteristics {
        TokenCharacteristics {
            display_name: "Fanatic of Rhonas".to_string(),
            power: Some(4),
            toughness: Some(4),
            core_types: vec![CoreType::Creature],
            subtypes: vec![
                "Zombie".to_string(),
                "Snake".to_string(),
                "Druid".to_string(),
            ],
            supertypes: Vec::new(),
            colors: vec![ManaColor::Black],
            keywords: Vec::new(),
        }
    }

    fn iridescent_vinelasher_offspring_body() -> TokenCharacteristics {
        TokenCharacteristics {
            display_name: "Iridescent Vinelasher".to_string(),
            power: Some(1),
            toughness: Some(1),
            core_types: vec![CoreType::Creature],
            subtypes: vec!["Lizard".to_string(), "Assassin".to_string()],
            supertypes: Vec::new(),
            colors: vec![ManaColor::Black],
            keywords: Vec::new(),
        }
    }

    fn state_with_source(
        source_name: &str,
        oracle_id: Option<&str>,
        related_ids: &[&str],
    ) -> (GameState, ObjectId) {
        let mut state = GameState::new_two_player(42);
        let source = create_object(
            &mut state,
            CardId(99),
            PlayerId(0),
            source_name.to_string(),
            Zone::Battlefield,
        );
        let obj = state.objects.get_mut(&source).unwrap();
        if let Some(oracle_id) = oracle_id {
            obj.printed_ref = Some(PrintedCardRef {
                oracle_id: oracle_id.to_string(),
                face_name: source_name.to_string(),
            });
        }
        obj.source_related_token_ids
            .extend(related_ids.iter().map(|id| (*id).to_string()));
        (state, source)
    }

    /// Forces `LazyLock` evaluation in `cargo test -p phase-engine` so an unknown
    /// `Keyword`/`CoreType`/`ManaColor` variant or a duplicate id panics in
    /// CI rather than at first production access. (Malformed TOML fails
    /// earlier, in `build.rs`; the structural conversion there cannot catch
    /// these typed-schema violations.)
    #[test]
    fn catalog_loads_and_validates() {
        let presets = known_token_presets();
        assert!(!presets.is_empty(), "catalog must contain entries");
    }

    /// Structural properties of `tokens-gen` output, checked against the
    /// committed catalog. Both are per-entry properties with no pinned totals,
    /// so a weekly refresh that only adds tokens stays green.
    ///
    /// Scope, stated precisely because it is narrow: these catch entries
    /// *reordered* relative to generator output, and an image ref pointing at a
    /// different preset. They do NOT catch a body-only rewrite (nothing here
    /// ties `body`/`fidelity`/`source_card_refs` to an identity) nor a deleted
    /// entry (removing from the middle keeps the sequence ascending) — a
    /// deletion is the count floors' job, in
    /// `analyze_token_coverage_treats_source_defined_pt_as_represented`.
    ///
    /// And none of it establishes provenance — that the catalog is tokens-gen's
    /// output for its declared vintage — which needs the gitignored generator
    /// input, absent at test time. See the comment on that coverage test for
    /// the by-hand `tokens-gen`/`cmp` recipe.
    #[test]
    fn catalog_is_ordered_and_self_consistent() {
        let presets = known_token_presets();

        // `tokens_gen.rs` emits `presets.sort_by(|a, b| a.id.cmp(&b.id))`, so a
        // strictly ascending id sequence is a property of every generated
        // catalog. Strict (not `<=`) also re-states the LazyLock's duplicate-id
        // guard at the ordering level.
        for pair in presets.windows(2) {
            assert!(
                pair[0].id < pair[1].id,
                "known-tokens.toml is not strictly id-ascending: `{}` precedes `{}`",
                pair[0].id,
                pair[1].id
            );
        }

        // The generator builds each preset's image ref from the same MTGJSON
        // uuid it uses for `TokenPreset::id`, so a ref pointing at a different
        // preset means the entry was assembled by something other than
        // tokens-gen. Consistency is asserted only where a ref exists: a token
        // with no Scryfall image is legal upstream data, and requiring one
        // would re-introduce a weekly false-red.
        for preset in presets {
            let Some(image_ref) = &preset.token_image_ref else {
                continue;
            };
            assert_eq!(
                image_ref.preset_id, preset.id,
                "preset `{}` carries an image ref for `{}`",
                preset.id, image_ref.preset_id
            );
        }
    }

    /// Every `PredefinedArtifact { kind }` preset must carry the matching
    /// subtype string, and the engine's `predefined_token_abilities` must
    /// have a non-empty ability list for that subtype. This invariant binds
    /// the catalog to the runtime ability registry so a kind cannot drift
    /// from its subtype or from its ability factory.
    #[test]
    fn predefined_artifact_subtypes_match_registry() {
        for preset in known_token_presets() {
            if let TokenCategory::PredefinedArtifact { kind } = &preset.category {
                let expected_subtype = kind.subtype_str();
                assert!(
                    preset.body.subtypes.iter().any(|s| s == expected_subtype),
                    "preset {} category PredefinedArtifact {{ {:?} }} but subtypes are {:?}",
                    preset.id,
                    kind,
                    preset.body.subtypes
                );
                assert!(
                    !crate::game::effects::token::predefined_token_abilities(expected_subtype)
                        .is_empty(),
                    "predefined_token_abilities has no arm for {}",
                    expected_subtype
                );
            }
        }
    }

    #[test]
    fn fixed_source_related_token_requires_exact_body_match() {
        let (state, source) = state_with_source(
            "Rot Like the Scum You Are",
            Some(ROT_LIKE_ORACLE_ID),
            &[FIXED_OOZE_PRESET_ID],
        );

        assert!(find_exact_token_ref(&state, source, &green_ooze_body(None, None)).is_none());
        assert!(find_exact_token_ref(&state, source, &green_ooze_body(Some(3), Some(3))).is_none());
    }

    #[test]
    fn fixed_source_related_token_matches_exact_body() {
        let (state, source) = state_with_source(
            "Rot Like the Scum You Are",
            Some(ROT_LIKE_ORACLE_ID),
            &[FIXED_OOZE_PRESET_ID],
        );

        let image = find_exact_token_ref(&state, source, &green_ooze_body(Some(2), Some(2)))
            .expect("fixed Ooze body should match linked preset image");

        assert_eq!(image.preset_id, FIXED_OOZE_PRESET_ID);
    }

    #[test]
    fn source_defined_source_related_token_may_ignore_runtime_pt_for_create_token() {
        let (state, source) = state_with_source(
            "Slime Molding",
            Some(SLIME_MOLDING_ORACLE_ID),
            &[SOURCE_DEFINED_OOZE_PRESET_ID],
        );

        let image = find_exact_token_ref(&state, source, &green_ooze_body(Some(7), Some(7)))
            .expect("source-defined Ooze should match after source identity is known");

        assert_eq!(image.preset_id, SOURCE_DEFINED_OOZE_PRESET_ID);
    }

    #[test]
    fn source_defined_pt_mismatch_is_not_global_or_copy_match() {
        let body = green_ooze_body(Some(7), Some(7));
        let (global_state, global_source) =
            state_with_source("Slime Molding", Some(SLIME_MOLDING_ORACLE_ID), &[]);
        assert!(find_exact_token_ref(&global_state, global_source, &body).is_none());

        let (ambiguous_state, ambiguous_source) =
            state_with_source("Slime Molding", None, &[SOURCE_DEFINED_OOZE_PRESET_ID]);
        assert!(find_exact_token_ref(&ambiguous_state, ambiguous_source, &body).is_none());

        let (copy_state, copy_source) = state_with_source(
            "Slime Molding",
            Some(SLIME_MOLDING_ORACLE_ID),
            &[SOURCE_DEFINED_OOZE_PRESET_ID],
        );
        assert!(find_card_linked_copy_token_ref(&copy_state, copy_source, &body).is_none());
    }

    #[test]
    fn card_linked_copy_does_not_use_name_only_source_fallback() {
        let body = fanatic_of_rhonas_body();
        let (state, source) = state_with_source("Fanatic of Rhonas", None, &[]);

        assert!(find_card_linked_copy_token_ref(&state, source, &body).is_none());
    }

    #[test]
    fn offspring_card_linked_copy_keeps_name_only_source_fallback() {
        let body = iridescent_vinelasher_offspring_body();
        let (mut state, source) = state_with_source("Iridescent Vinelasher", None, &[]);
        state
            .objects
            .get_mut(&source)
            .unwrap()
            .keywords
            .push(Keyword::Offspring(ManaCost::generic(2)));

        let image = find_card_linked_copy_token_ref(&state, source, &body)
            .expect("offspring copy token may use name-only source context");

        assert_eq!(image.preset_id, IRIDESCENT_VINELASHER_PRESET_ID);
    }

    #[test]
    fn card_linked_copy_matches_related_token_body_without_printed_ref() {
        let body = fanatic_of_rhonas_body();
        let (state, source) =
            state_with_source("Fanatic of Rhonas", None, &[FANATIC_OF_RHONAS_PRESET_ID]);

        let image = find_card_linked_copy_token_ref(&state, source, &body)
            .expect("related Fanatic copy token body should match linked preset");

        assert_eq!(image.preset_id, FANATIC_OF_RHONAS_PRESET_ID);
    }

    #[test]
    fn exact_create_token_keeps_name_only_source_fallback() {
        let body = fanatic_of_rhonas_body();
        let (state, source) = state_with_source("Fanatic of Rhonas", None, &[]);

        let image = find_exact_token_ref(&state, source, &body)
            .expect("exact create-token lookup may use name-only source context");

        assert_eq!(image.preset_id, FANATIC_OF_RHONAS_PRESET_ID);
    }

    /// Builds a merge-level preset. Nothing here names a card: the merge's
    /// population is every row the overlay will ever hold.
    fn merge_preset(
        id: &str,
        set: &str,
        name: &str,
        source_oracle_id: Option<&str>,
    ) -> TokenPreset {
        TokenPreset {
            id: id.to_string(),
            category: TokenCategory::Creature,
            fidelity: PresetFidelity::Full,
            pt_provenance: TokenPtProvenance::FixedOrAbsent,
            body: TokenCharacteristics {
                display_name: name.to_string(),
                power: Some(1),
                toughness: Some(1),
                core_types: vec![CoreType::Creature],
                subtypes: vec![name.to_string()],
                supertypes: Vec::new(),
                colors: vec![ManaColor::Green],
                keywords: Vec::new(),
            },
            source_card_names: Vec::new(),
            source_card_refs: source_oracle_id
                .map(|oracle_id| {
                    vec![TokenSourceRef {
                        card_name: format!("{name} Maker"),
                        face_name: None,
                        scryfall_oracle_id: Some(oracle_id.to_string()),
                        scryfall_id: None,
                    }]
                })
                .unwrap_or_default(),
            token_image_ref: None,
            set_code: set.to_string(),
            set_name: String::new(),
            collector_number: None,
            released_at: None,
            type_line: String::new(),
            rules_text: None,
        }
    }

    const MAKER_ORACLE_ID: &str = "11111111-1111-1111-1111-111111111111";
    const OTHER_MAKER_ORACLE_ID: &str = "22222222-2222-2222-2222-222222222222";

    #[test]
    fn overlay_row_matching_nothing_is_merged_in_id_order() {
        let generated = vec![
            merge_preset("id-a", "AAA", "Beast", Some(MAKER_ORACLE_ID)),
            merge_preset("id-c", "CCC", "Horror", Some(MAKER_ORACLE_ID)),
        ];
        let row = merge_preset("id-b", "BBB", "Wurm", Some(MAKER_ORACLE_ID));

        let (presets, reports) = merge_overlay(generated, vec![row]).expect("row is valid");

        // Present and positioned, not merely "not dropped".
        let ids: Vec<&str> = presets.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["id-a", "id-b", "id-c"]);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].outcome, OverlayRowOutcome::Applied);
        assert_eq!(reports[0].overlay_id, "id-b");
    }

    #[test]
    fn mtgjson_supersedes_an_overlay_row_when_it_publishes_that_token() {
        // MTGJSON publishes the token under its own uuid and links it back to
        // the same creating card the overlay row names.
        let generated = vec![merge_preset(
            "mtgjson-id",
            "FRA",
            "Wurm",
            Some(MAKER_ORACLE_ID),
        )];
        let row = merge_preset("placeholder-id", "FRA", "Wurm", Some(MAKER_ORACLE_ID));

        let (presets, reports) = merge_overlay(generated, vec![row]).expect("row is valid");

        let wurms: Vec<&TokenPreset> = presets
            .iter()
            .filter(|p| p.body.display_name == "Wurm")
            .collect();
        assert_eq!(wurms.len(), 1, "an id-only key would leave two");
        // The surviving id is the generated one, so "everything is superseded"
        // still fails this.
        assert_eq!(wurms[0].id, "mtgjson-id");
        assert_eq!(
            reports[0].outcome,
            OverlayRowOutcome::Superseded {
                by_id: "mtgjson-id".to_string()
            }
        );
    }

    #[test]
    fn a_generated_preset_sharing_the_overlay_row_id_supersedes_it() {
        // Same id, different set *and* name: only the id leg can fire, and it
        // must — a duplicate id panics the `PRESETS` `LazyLock` for every
        // consumer.
        let generated = vec![merge_preset(
            "shared-id",
            "AAA",
            "Beast",
            Some(MAKER_ORACLE_ID),
        )];
        let row = merge_preset("shared-id", "BBB", "Wurm", Some(MAKER_ORACLE_ID));

        let (presets, reports) = merge_overlay(generated, vec![row]).expect("row is valid");

        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].body.display_name, "Beast");
        assert_eq!(
            reports[0].outcome,
            OverlayRowOutcome::Superseded {
                by_id: "shared-id".to_string()
            }
        );
    }

    #[test]
    fn a_same_named_token_from_a_different_card_does_not_supersede() {
        // The admitted member the key must refuse: a same-named token published
        // in the same set by a *different* card. The loose `(set, name)` key
        // drops the hand-authored row here.
        let mut generated = merge_preset("mtgjson-id", "FRA", "Wurm", Some(OTHER_MAKER_ORACLE_ID));
        generated.body.power = Some(9);
        generated.body.toughness = Some(9);
        let row = merge_preset("placeholder-id", "FRA", "Wurm", Some(MAKER_ORACLE_ID));

        let (presets, reports) = merge_overlay(vec![generated], vec![row]).expect("row is valid");

        // Positive reach-guard for the negative claim: the row is *present*.
        assert_eq!(presets.len(), 2);
        assert!(presets.iter().any(|p| p.id == "placeholder-id"));
        assert!(presets.iter().any(|p| p.id == "mtgjson-id"));
        assert_eq!(
            reports[0].outcome,
            OverlayRowOutcome::AppliedShadowed {
                by_id: "mtgjson-id".to_string()
            }
        );
    }

    #[test]
    fn neither_a_different_set_nor_a_different_name_supersedes() {
        // Both near-misses share the source card, so only `set_code` and
        // `display_name` keep them from superseding.
        let generated = vec![
            merge_preset("other-set", "XXX", "Wurm", Some(MAKER_ORACLE_ID)),
            merge_preset("other-name", "FRA", "Beast", Some(MAKER_ORACLE_ID)),
        ];
        let row = merge_preset("placeholder-id", "FRA", "Wurm", Some(MAKER_ORACLE_ID));

        let (presets, reports) = merge_overlay(generated, vec![row]).expect("row is valid");

        assert_eq!(presets.len(), 3);
        // Not even shadowed: `names_same_token` needs both legs.
        assert_eq!(reports[0].outcome, OverlayRowOutcome::Applied);
    }

    #[test]
    fn overlay_rows_deduplicate_against_each_other_on_both_legs() {
        // The merge scans its own growing output, so the overlay cannot emit a
        // duplicate id on its own. One member at *each* key leg.
        let id_leg = vec![
            merge_preset("dup-id", "AAA", "Beast", Some(MAKER_ORACLE_ID)),
            merge_preset("dup-id", "BBB", "Wurm", Some(MAKER_ORACLE_ID)),
        ];
        let (presets, reports) = merge_overlay(Vec::new(), id_leg).expect("rows are valid");
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].body.display_name, "Beast");
        assert_eq!(
            reports[1].outcome,
            OverlayRowOutcome::Superseded {
                by_id: "dup-id".to_string()
            }
        );

        let name_leg = vec![
            merge_preset("first-id", "FRA", "Wurm", Some(MAKER_ORACLE_ID)),
            merge_preset("second-id", "FRA", "Wurm", Some(MAKER_ORACLE_ID)),
        ];
        let (presets, reports) = merge_overlay(Vec::new(), name_leg).expect("rows are valid");
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].id, "first-id");
        assert_eq!(
            reports[1].outcome,
            OverlayRowOutcome::Superseded {
                by_id: "first-id".to_string()
            }
        );
    }

    #[test]
    fn an_overlay_row_without_a_source_card_reference_is_rejected() {
        // Two members of the refused class: no `source_card_refs` at all, and a
        // ref that names a card but carries no `scryfall_oracle_id`.
        let no_refs = merge_preset("no-refs-id", "FRA", "Wurm", None);
        let err = merge_overlay(Vec::new(), vec![no_refs]).expect_err("must be refused");
        assert!(err.contains("no-refs-id"), "error must name the row: {err}");

        let mut no_oracle_id = merge_preset("no-oracle-id", "FRA", "Wurm", None);
        no_oracle_id.source_card_refs = vec![TokenSourceRef {
            card_name: "Wurm Maker".to_string(),
            face_name: None,
            scryfall_oracle_id: None,
            scryfall_id: Some("not-an-oracle-id".to_string()),
        }];
        let err = merge_overlay(Vec::new(), vec![no_oracle_id]).expect_err("must be refused");
        assert!(
            err.contains("no-oracle-id"),
            "error must name the row: {err}"
        );

        // Positive control: the identical list *with* the reference is accepted,
        // so the `Err` legs above are the guard firing and not a merge that
        // refuses everything.
        let valid = merge_preset("valid-id", "FRA", "Wurm", Some(MAKER_ORACLE_ID));
        let (presets, _) = merge_overlay(Vec::new(), vec![valid]).expect("valid row is accepted");
        assert_eq!(presets.len(), 1);
    }

    #[test]
    fn committed_overlay_rows_are_present_in_the_catalog() {
        let overlay: CatalogFile =
            toml::from_str(include_str!("../../data/known-tokens.overlay.toml"))
                .expect("known-tokens.overlay.toml parses as a catalog file");

        // Live-instrument control: a membership loop over an empty list confirms
        // nothing, so an emptied overlay must red here rather than pass vacuously.
        assert!(
            !overlay.token.is_empty(),
            "known-tokens.overlay.toml holds no rows: either a row was lost, or the \
             file has outlived its purpose and it plus this test should be retired"
        );

        let mut seen = std::collections::HashSet::new();
        for row in &overlay.token {
            assert!(
                seen.insert(row.id.as_str()),
                "known-tokens.overlay.toml: duplicate row id `{}`",
                row.id
            );
            assert_eq!(
                known_token_preset_by_id(&row.id),
                Some(row),
                "overlay row `{}` ({} / {}) is missing from or differs in known-tokens.toml. \
                 Two repairs: re-run `tokens-gen` if the catalog lost the row, or delete the \
                 row from the overlay if `tokens-gen` reported it superseded.",
                row.id,
                row.set_code,
                row.body.display_name
            );
        }
    }
}
