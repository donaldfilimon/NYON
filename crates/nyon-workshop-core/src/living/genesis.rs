//! The Living Galaxy V2 genesis manifest and its validated form.
//!
//! Section 10 makes the manifest the source of replay truth: "`genesis_manifest`
//! is the validated source of replay truth. Generators are never rerun when
//! decoding." So this module carries no generation of any kind. It decodes a
//! declared tick-zero galaxy, validates it, and derives the one digest the root
//! branch is built from.
//!
//! `LivingGenesisGeneratorV2` is **provenance only**: section 10 says
//! "`genesis_generator` records an exact generator/starter ID and version for
//! provenance". It is an identifier and a version, never an executable, and
//! nothing here consults it.
//!
//! # Materialization
//!
//! Section 10: "`genesis_manifest_digest_32` is the Living state digest of the
//! validated tick-zero state materialized from that manifest." The manifest
//! declares that state directly, so materialization is the identity projection
//! of its `state` field. That is deliberate rather than lazy. A manifest that
//! required an expansion algorithm would make that algorithm part of replay
//! truth, which is exactly what "generators are never rerun when decoding"
//! forbids; a declared state can only be checked, never recomputed.
//!
//! # The wire envelope is not published upstream
//!
//! Section 10 publishes the top-level field order of a catalog pack and of an
//! archive, and the authoritative state schema, but no field list for the
//! genesis manifest itself. The envelope below — `kind`, `format_version`,
//! `rules_version`, `state` — is this implementation's, chosen to mirror the
//! pack envelope exactly. Reordering a field is a format break, and this
//! envelope needs promoting into the rules specification the way the state
//! schema was promoted, so that the frozen vectors have upstream text to be
//! derived from.
//!
//! # What this module deliberately does not re-implement
//!
//! Referential integrity, capacities, and integer ranges are
//! [`LivingGalaxyStateV2::validate`]'s job, and this module calls it rather
//! than restating any part of it. What is added here is only what is specific
//! to genesis: the envelope, the tick-zero conditions, and section 2's clock
//! initialization rule.

use serde::{Deserialize, Serialize};

use super::LIVING_RULES_VERSION;
use super::catalog::{LivingIndustryEffectV2, LivingSlugV2, ValidatedLivingCatalogPackV2};
use super::ids::{LivingBranchIdV2, LivingCatalogHashV2, LivingStateDigestV2, root_branch_id_v2};
use super::model::{LivingGalaxyStateV2, LivingValidationErrorV2};
use super::wire::{
    LIVING_MAX_ARCHIVE_BYTES_V2, LivingWireErrorV2, decode_canonical_v2, encode_canonical_v2,
};

/// The only document kind this module accepts.
pub const LIVING_GENESIS_KIND_V2: &str = "NYON_LIVING_GALAXY_GENESIS";
/// Wire format version of a Living V2 genesis manifest.
pub const LIVING_GENESIS_FORMAT_VERSION_V2: u32 = 2;

/// Section 3, hub baseline: "Each hub produces 2 energy every 10 ticks".
///
/// The three hub periods are fixed rules rather than catalog data, and no
/// module declared them before this one. They are exported through
/// `living::` so the module that eventually runs a boundary imports rather
/// than restates them.
pub const LIVING_HUB_ENERGY_PERIOD_TICKS_V2: u64 = 10;
/// Section 3, hub baseline: "and 1 ore every 100 ticks".
pub const LIVING_HUB_ORE_PERIOD_TICKS_V2: u64 = 100;
/// Section 3, hub baseline: the fallback fabricator runs "every 100 ticks".
pub const LIVING_HUB_FALLBACK_PERIOD_TICKS_V2: u64 = 100;

/// Every way a Living V2 genesis manifest can fail validation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingGenesisErrorV2 {
    /// The canonical wire contract rejected the bytes.
    #[error("Living V2 genesis wire error: {0}")]
    Wire(#[from] LivingWireErrorV2),
    /// The document is not a Living V2 genesis manifest.
    #[error("Living V2 genesis manifests declare kind {LIVING_GENESIS_KIND_V2}")]
    Kind,
    /// The wire format version is not the one this module implements.
    #[error("Living V2 genesis format version {found} is not {LIVING_GENESIS_FORMAT_VERSION_V2}")]
    FormatVersion {
        /// Version found in the document.
        found: u32,
    },
    /// The rules version is not Living V2's.
    #[error("Living V2 genesis rules version {found} is not 2")]
    RulesVersion {
        /// Version found in the document.
        found: u32,
    },
    /// The declared state is not at the genesis boundary.
    #[error("Living V2 genesis declares tick {found}; the genesis boundary is 0")]
    Tick {
        /// Boundary found in the document.
        found: u64,
    },
    /// A history sequence is already advanced at genesis.
    #[error("Living V2 genesis field {field} is {found}; nothing has been accepted yet")]
    Sequence {
        /// The offending field.
        field: &'static str,
        /// Value found in the document.
        found: u64,
    },
    /// A clock does not hold the boundary section 2 initializes it to.
    #[error(
        "Living V2 genesis clock {clock} is {found:?}; section 2 initializes it to {expected:?}"
    )]
    Clock {
        /// Which clock disagreed.
        clock: &'static str,
        /// Boundary found in the document, or `None` for an absent clock.
        found: Option<u64>,
        /// Boundary section 2 requires, or `None` where no recipe recurs.
        expected: Option<u64>,
    },
    /// The declared state failed the authority state contract.
    #[error("Living V2 genesis state error: {0}")]
    State(#[from] LivingValidationErrorV2),
}

/// Provenance for the generator or curated starter that produced a manifest.
///
/// Section 10: "`genesis_generator` records an exact generator/starter ID and
/// version for provenance". It is a record, not a procedure. Nothing reruns
/// what it names, and no field of a validated genesis is derived from it, so it
/// travels beside a manifest in an archive rather than inside one.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingGenesisGeneratorV2 {
    /// The exact generator or curated starter identifier.
    pub generator_id: LivingSlugV2,
    /// That generator's own content version.
    pub generator_version: u32,
}

/// A declared genesis galaxy, before validation.
///
/// The four fields are the wire order. Reordering one is a format break.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingGenesisManifestV2 {
    /// Always [`LIVING_GENESIS_KIND_V2`].
    pub kind: String,
    /// Always [`LIVING_GENESIS_FORMAT_VERSION_V2`].
    pub format_version: u32,
    /// Always Living V2's rules version.
    pub rules_version: u32,
    /// The declared tick-zero galaxy this manifest materializes to.
    pub state: LivingGalaxyStateV2,
}

/// A genesis manifest that passed every check, with its derived digest.
///
/// The document is private and reachable only through the accessors below, so
/// a rejected manifest can never yield a partly built value: every constructor
/// validates first and builds second.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedLivingGenesisV2 {
    document: LivingGenesisManifestV2,
    canonical_bytes: Vec<u8>,
    state_canonical_bytes: Vec<u8>,
    manifest_digest: LivingStateDigestV2,
    catalog_hash: LivingCatalogHashV2,
}

/// Decode and validate one Living V2 genesis manifest against its catalog.
///
/// The bound is the archive bound rather than the pack bound: a manifest
/// embeds a whole galaxy, and a large curated starter is legitimate.
pub fn decode_living_genesis_manifest_v2(
    bytes: &[u8],
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<ValidatedLivingGenesisV2, LivingGenesisErrorV2> {
    let document: LivingGenesisManifestV2 =
        decode_canonical_v2(bytes, LIVING_MAX_ARCHIVE_BYTES_V2)?;
    build(document, Some(bytes.to_vec()), catalog)
}

/// Validate one in-memory genesis manifest against its catalog.
///
/// This is the path a generator or curated starter uses once it has built a
/// manifest: it never bypasses a check that [`decode_living_genesis_manifest_v2`]
/// performs, it only skips the decode that produced the document.
pub fn validate_living_genesis_manifest_v2(
    manifest: LivingGenesisManifestV2,
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<ValidatedLivingGenesisV2, LivingGenesisErrorV2> {
    build(manifest, None, catalog)
}

fn build(
    document: LivingGenesisManifestV2,
    bytes: Option<Vec<u8>>,
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<ValidatedLivingGenesisV2, LivingGenesisErrorV2> {
    check_envelope(&document)?;
    check_genesis_boundary(&document.state)?;
    // The authority contract runs before the clock rule on purpose. The clock
    // rule resolves each facility's catalog definition, and an unresolvable
    // definition is a dangling reference rather than a clock defect.
    document.state.validate(catalog)?;
    check_clocks(&document.state, catalog)?;

    let canonical_bytes = match bytes {
        Some(bytes) => bytes,
        None => encode_canonical_v2(&document, LIVING_MAX_ARCHIVE_BYTES_V2)?,
    };
    let state_canonical_bytes = document.state.canonical_bytes()?;
    let manifest_digest = document.state.digest()?;

    Ok(ValidatedLivingGenesisV2 {
        document,
        canonical_bytes,
        state_canonical_bytes,
        manifest_digest,
        catalog_hash: catalog.catalog_hash(),
    })
}

fn check_envelope(document: &LivingGenesisManifestV2) -> Result<(), LivingGenesisErrorV2> {
    if document.kind != LIVING_GENESIS_KIND_V2 {
        return Err(LivingGenesisErrorV2::Kind);
    }
    if document.format_version != LIVING_GENESIS_FORMAT_VERSION_V2 {
        return Err(LivingGenesisErrorV2::FormatVersion {
            found: document.format_version,
        });
    }
    if document.rules_version != LIVING_RULES_VERSION {
        return Err(LivingGenesisErrorV2::RulesVersion {
            found: document.rules_version,
        });
    }
    Ok(())
}

/// Section 10 calls the materialized state the "tick-zero state".
///
/// The two sequence conditions are an inference, recorded as one rather than
/// asserted as quoted text. The design specification places the root branch "at
/// that manifest, before creator revision zero", and section 10 says the root
/// branch does not consume the fork sequence, so at genesis neither historical
/// sequence can have advanced. If a later ruling admits a resumed sequence at
/// genesis, this is the check that moves.
fn check_genesis_boundary(state: &LivingGalaxyStateV2) -> Result<(), LivingGenesisErrorV2> {
    if state.tick.0 != 0 {
        return Err(LivingGenesisErrorV2::Tick {
            found: state.tick.0,
        });
    }
    if state.accepted_sequence != 0 {
        return Err(LivingGenesisErrorV2::Sequence {
            field: "accepted_sequence",
            found: state.accepted_sequence,
        });
    }
    if state.branch_sequence != 0 {
        return Err(LivingGenesisErrorV2::Sequence {
            field: "branch_sequence",
            found: state.branch_sequence,
        });
    }
    Ok(())
}

/// Section 2: "Genesis at boundary G initializes each clock to G plus its
/// period: hub energy, solar, and extractor at G+10; foundry at G+20; hub
/// ore/fallback at G+100."
///
/// The three named facilities are read as the general rule they illustrate —
/// each recurring recipe is due one of its own catalog periods after G — rather
/// than as three hard-coded slugs, since the same section states the rule in
/// exactly those general terms first. A facility whose effect never recurs has
/// no due boundary at all.
///
/// Only clocks are checked here. `next_repair`, hit points, blocked reasons and
/// queued jobs carry no stated genesis rule, and inventing one would be this
/// module deciding a rule instead of enforcing one.
fn check_clocks(
    state: &LivingGalaxyStateV2,
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<(), LivingGenesisErrorV2> {
    let boundary = state.tick.0;

    for colony in state.colonies.as_slice() {
        let hub = &colony.hub;
        check_clock(
            "hub energy_next_due",
            Some(hub.energy_next_due.0),
            Some(boundary + LIVING_HUB_ENERGY_PERIOD_TICKS_V2),
        )?;
        check_clock(
            "hub ore_next_due",
            Some(hub.ore_next_due.0),
            Some(boundary + LIVING_HUB_ORE_PERIOD_TICKS_V2),
        )?;
        check_clock(
            "hub fallback_next_due",
            Some(hub.fallback_next_due.0),
            Some(boundary + LIVING_HUB_FALLBACK_PERIOD_TICKS_V2),
        )?;
    }

    for facility in state.facilities.as_slice() {
        // `validate` already proved this definition resolves.
        let definition = catalog.industry(facility.definition.as_str()).ok_or(
            LivingValidationErrorV2::DanglingReference {
                reference: "facility definition",
            },
        )?;
        let expected = match definition.effect {
            LivingIndustryEffectV2::Recipe { period_ticks, .. } => {
                Some(boundary + u64::from(period_ticks))
            }
            LivingIndustryEffectV2::HullAssembly { .. }
            | LivingIndustryEffectV2::Defense { .. } => None,
        };
        check_clock(
            "facility next_due",
            facility.next_due.map(|tick| tick.0),
            expected,
        )?;
    }

    Ok(())
}

fn check_clock(
    clock: &'static str,
    found: Option<u64>,
    expected: Option<u64>,
) -> Result<(), LivingGenesisErrorV2> {
    if found == expected {
        return Ok(());
    }
    Err(LivingGenesisErrorV2::Clock {
        clock,
        found,
        expected,
    })
}

impl ValidatedLivingGenesisV2 {
    /// The exact canonical bytes of the manifest document.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// The canonical bytes of the materialized tick-zero state, which are the
    /// bytes [`Self::manifest_digest`] hashes.
    pub fn state_canonical_bytes(&self) -> &[u8] {
        &self.state_canonical_bytes
    }

    /// The wire format version, always [`LIVING_GENESIS_FORMAT_VERSION_V2`].
    pub const fn format_version(&self) -> u32 {
        self.document.format_version
    }

    /// The rules version, always Living V2's.
    pub const fn rules_version(&self) -> u32 {
        self.document.rules_version
    }

    /// The materialized tick-zero state.
    pub const fn state(&self) -> &LivingGalaxyStateV2 {
        &self.document.state
    }

    /// The catalog this manifest was validated against.
    pub const fn catalog_hash(&self) -> LivingCatalogHashV2 {
        self.catalog_hash
    }

    /// Section 10's `genesis_manifest_digest_32`: the Living state digest of
    /// the validated tick-zero state materialized from this manifest.
    pub const fn manifest_digest(&self) -> LivingStateDigestV2 {
        self.manifest_digest
    }

    /// The root branch this genesis and seed begin.
    ///
    /// The catalog hash is the one this manifest validated against, so no
    /// caller can pair a genesis with a catalog it was never checked under.
    pub fn root_branch_id(&self, genesis_seed: [u8; 32]) -> LivingBranchIdV2 {
        root_branch_id_v2(self.catalog_hash, genesis_seed, self.manifest_digest)
    }
}
