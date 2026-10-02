//! Additive authoritative decision explanations. Existing tick receipt bytes and
//! event identities are unchanged; this separately sealed record binds them.
use super::{
    catalog::LivingSlugV2,
    ids::*,
    model::{LivingAgreementKindV2, LivingGalaxyStateV2, LivingResourceV2},
    receipt::LivingTickReceiptV2,
    wire::{LivingWireErrorV2, decode_canonical_v2, encode_canonical_v2},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Canonical decision sidecar kind.
pub const LIVING_DECISION_KIND_V2: &str = "NYON_LIVING_DECISION_RECEIPT";
/// Bounded explanatory bytes per boundary, independent from retained history.
pub const LIVING_MAX_DECISION_BYTES_V2: usize = 2 * 1024 * 1024;
/// Domain distinct from ordinary receipt and state identities.
pub const LIVING_DECISION_DOMAIN_V2: &[u8] = b"NYON-LIVING-DECISION-RECEIPT-V2\0";

/// A canonical signed decimal scalar; integers are strings on this new wire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LivingDecisionScoreV2(pub i64);
impl Serialize for LivingDecisionScoreV2 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for LivingDecisionScoreV2 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        let v = s.parse::<i64>().map_err(serde::de::Error::custom)?;
        if v.to_string() != s {
            return Err(serde::de::Error::custom("noncanonical signed decimal"));
        }
        Ok(Self(v))
    }
}
/// Dedicated decision integrity identity; underlying scalar retains hex encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LivingDecisionDigestV2(pub LivingReceiptDigestV2);

/// Candidate action payload; it identifies inputs without allocating entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingIntentActionV2 {
    /// Construct a completed industry through the ordinary job path.
    Construction {
        world: LivingEntityIdV2,
        definition: LivingSlugV2,
        /// Exact idle duplicate producer removed atomically with this replacement.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scrap: Option<LivingEntityIdV2>,
    },
    /// Build one hull.
    Hull {
        world: LivingEntityIdV2,
        definition: LivingSlugV2,
    },
    /// Establish or adjust freight permission and endpoints.
    Route {
        source: LivingEntityIdV2,
        destination: LivingEntityIdV2,
        resource: LivingResourceV2,
        /// Exact existing route adjusted while retaining its scheduled clock.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        existing_route: Option<LivingEntityIdV2>,
    },
    /// Scout a world.
    Explore {
        fleet: LivingEntityIdV2,
        destination: LivingEntityIdV2,
    },
    /// Move to a world.
    Move {
        fleet: LivingEntityIdV2,
        destination: LivingEntityIdV2,
    },
    /// Settle an eligible world.
    Settle {
        fleet: LivingEntityIdV2,
        destination: LivingEntityIdV2,
    },
    /// Defend an owned world.
    Defend {
        fleet: LivingEntityIdV2,
        world: LivingEntityIdV2,
    },
    /// Attack a hostile world.
    Attack {
        fleet: LivingEntityIdV2,
        world: LivingEntityIdV2,
    },
    /// Return to a world.
    Return {
        fleet: LivingEntityIdV2,
        world: LivingEntityIdV2,
    },
    /// Bilateral agreement proposal.
    Agreement {
        peer: LivingEntityIdV2,
        agreement_kind: LivingAgreementKindV2,
    },
    /// War proposal.
    War { peer: LivingEntityIdV2 },
}
impl LivingIntentActionV2 {
    /// Additive fixed registry, independent of old entity/phase registries.
    pub const fn ordinal(&self) -> u16 {
        match self {
            Self::Construction { .. } => 0,
            Self::Hull { .. } => 1,
            Self::Route { .. } => 2,
            Self::Explore { .. } => 3,
            Self::Move { .. } => 4,
            Self::Settle { .. } => 5,
            Self::Defend { .. } => 6,
            Self::Attack { .. } => 7,
            Self::Return { .. } => 8,
            Self::Agreement { .. } => 9,
            Self::War { .. } => 10,
        }
    }
    /// Canonical bytes provide the final tie breaker, including definition slug.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LivingWireErrorV2> {
        encode_canonical_v2(self, LIVING_MAX_DECISION_BYTES_V2)
    }
}
/// Rules priority tiers. Emergency priority applies only to actionable choices.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingDecisionTierV2 {
    /// Restore missing power.
    PowerEmergency,
    /// Restore missing ore.
    OreEmergency,
    /// Restore missing alloy.
    AlloyEmergency,
    /// Ordinary scored choice.
    Ordinary,
    /// Existing journey or dormant ark recovery.
    Recovery,
}
/// Accepted/rejected refers to this candidate, not a scheduled action completing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingDecisionOutcomeV2 {
    /// Centrally reserved and admitted.
    Accepted,
    /// No resources spent by this candidate.
    Rejected,
}
/// Structured, replayable rules evidence, never a fabricated narrative string.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingDecisionReasonV2 {
    /// Input or preserved reserve prevents admission.
    InsufficientResource {
        world: LivingEntityIdV2,
        resource: LivingResourceV2,
        required: u32,
        available: u32,
        reserve: u32,
    },
    /// Output would exceed capacity.
    OutputCapacity {
        world: LivingEntityIdV2,
        resource: LivingResourceV2,
        amount: u32,
        available_capacity: u32,
    },
    /// Queue or global reservation unavailable.
    QueueCapacity {
        world: LivingEntityIdV2,
        queue: LivingSlugV2,
        limit: u32,
        reserved: u32,
    },
    /// A competing actor holds the target.
    TargetReserved {
        target: LivingEntityIdV2,
        winner: LivingEntityIdV2,
    },
    /// Another legal candidate won the permitted slot.
    LowerRank {
        winning_actor: LivingEntityIdV2,
        winning_ordinal: u16,
    },
    /// Required target intelligence is absent.
    MissingIntelligence { world: LivingEntityIdV2 },
    /// Intelligence is too old.
    StaleObservation {
        world: LivingEntityIdV2,
        observed_tick: LivingTickV2,
        maximum_age: LivingTickV2,
    },
    /// Named structural rule blocks the target.
    InvalidTarget {
        target: LivingEntityIdV2,
        rule: LivingSlugV2,
    },
    /// Dormant actor cannot originate this action.
    DormantRestriction { actor: LivingEntityIdV2 },
    /// Both directions must satisfy a threshold.
    BilateralThreshold {
        peer: LivingEntityIdV2,
        forward: i32,
        reverse: i32,
        required: i32,
    },
    /// A truce remains active.
    TruceActive {
        peer: LivingEntityIdV2,
        end_tick: LivingTickV2,
    },
    /// Exact ranking contribution.
    PolicyScore {
        policy: LivingSlugV2,
        component: LivingSlugV2,
        value: LivingDecisionScoreV2,
    },
    /// Forecast calculation witness.
    Forecast {
        world: LivingEntityIdV2,
        horizon: LivingTickV2,
        resource: LivingResourceV2,
        predicted: LivingDecisionScoreV2,
        demand: LivingDecisionScoreV2,
    },
    /// Named rule admits the action.
    Accepted { rule: LivingSlugV2 },
}
/// One candidate and its actual ranking/reservation outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LivingIntentDecisionV2 {
    /// Only diplomacy (7) and intent generation (8) emit these records.
    pub phase: LivingPhaseV2,
    /// Civilization whose visible information supplied the candidate.
    pub actor: LivingEntityIdV2,
    /// Dense candidate ordinal within actor/phase.
    pub ordinal: u16,
    /// Candidate input.
    pub action: LivingIntentActionV2,
    /// Its principal target, or null.
    pub target: Option<LivingEntityIdV2>,
    /// Actual integer authority rank.
    pub score: LivingDecisionScoreV2,
    /// Accepted priority tier.
    pub tier: LivingDecisionTierV2,
    /// Applied boundary T after normalizing snapshot state.tick to T.
    pub snapshot_tick: LivingTickV2,
    /// Digest of the corresponding normalized immutable phase snapshot.
    pub snapshot_digest: LivingStateDigestV2,
    /// Target observation age witness; distinct from snapshot clock.
    pub observed_tick: Option<LivingTickV2>,
    /// Admission result.
    pub outcome: LivingDecisionOutcomeV2,
    /// Ordered rules evidence.
    pub reasons: Vec<LivingDecisionReasonV2>,
}
/// Committed sidecar. Integrity is checked independently of ordinary receipts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LivingDecisionReceiptV2 {
    /// Exact kind.
    pub kind: String,
    /// Two.
    pub format_version: u32,
    /// Two.
    pub rules_version: u32,
    /// Exact catalog identity.
    pub catalog_hash: LivingCatalogHashV2,
    /// Branch on which this boundary committed.
    pub branch: LivingBranchIdV2,
    /// Committed boundary T.
    pub tick: LivingTickV2,
    /// Published completed T-1 state digest.
    pub committed_state_before: LivingStateDigestV2,
    /// Published completed T state digest.
    pub state_after: LivingStateDigestV2,
    /// Bound ordinary receipt digest, with unchanged ordinary bytes.
    pub receipt_digest: LivingReceiptDigestV2,
    /// Phase/actor/ordinal sorted records.
    pub intents: Vec<LivingIntentDecisionV2>,
    /// Domain-separated integrity over payload excluding this field.
    pub integrity_sha256: LivingDecisionDigestV2,
}
#[derive(Serialize)]
struct Payload<'a> {
    kind: &'a str,
    format_version: u32,
    rules_version: u32,
    catalog_hash: LivingCatalogHashV2,
    branch: LivingBranchIdV2,
    tick: LivingTickV2,
    committed_state_before: LivingStateDigestV2,
    state_after: LivingStateDigestV2,
    receipt_digest: LivingReceiptDigestV2,
    intents: &'a [LivingIntentDecisionV2],
}
/// Typed validation errors, preserving pending authority until sealing succeeds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingDecisionErrorV2 {
    /// Wire encoding or strict decoding failed.
    #[error("invalid decision wire: {0}")]
    Wire(#[from] LivingWireErrorV2),
    /// Wrong kind or version.
    #[error("wrong decision kind or version")]
    KindOrVersion,
    /// Invalid phase, clocks, reason bounds, order or dense ordinal.
    #[error("invalid decision candidate")]
    Candidate,
    /// Bounded sidecar capacity exceeded.
    #[error("decision capacity exceeded")]
    Capacity,
    /// Stored integrity is wrong.
    #[error("decision integrity mismatch")]
    Integrity,
    /// Ordinary receipt binding is wrong.
    #[error("decision receipt binding mismatch")]
    ReceiptBinding,
    /// Snapshot cannot be validated/hash normalized.
    #[error("invalid decision phase snapshot")]
    Snapshot,
}
impl LivingDecisionReceiptV2 {
    fn payload(&self) -> Payload<'_> {
        Payload {
            kind: &self.kind,
            format_version: self.format_version,
            rules_version: self.rules_version,
            catalog_hash: self.catalog_hash,
            branch: self.branch,
            tick: self.tick,
            committed_state_before: self.committed_state_before,
            state_after: self.state_after,
            receipt_digest: self.receipt_digest,
            intents: &self.intents,
        }
    }
    fn digest(&self) -> Result<LivingDecisionDigestV2, LivingDecisionErrorV2> {
        let bytes = encode_canonical_v2(&self.payload(), LIVING_MAX_DECISION_BYTES_V2)?;
        let mut sha = Sha256::new();
        sha.update(LIVING_DECISION_DOMAIN_V2);
        sha.update(bytes);
        Ok(LivingDecisionDigestV2(LivingReceiptDigestV2(
            sha.finalize().into(),
        )))
    }
    /// Strict structural/integrity verification. Replay verifies candidate truth.
    pub fn verify(&self) -> Result<(), LivingDecisionErrorV2> {
        if self.kind != LIVING_DECISION_KIND_V2
            || self.format_version != 2
            || self.rules_version != 2
        {
            return Err(LivingDecisionErrorV2::KindOrVersion);
        }
        if self.tick.0 == 0 {
            return Err(LivingDecisionErrorV2::Candidate);
        }
        let mut previous = None;
        for intent in &self.intents {
            if !matches!(
                intent.phase,
                LivingPhaseV2::UpdateDiplomacy | LivingPhaseV2::GenerateIntents
            ) || intent.snapshot_tick != self.tick
                || intent.observed_tick.is_some_and(|tick| tick > self.tick)
                || intent.reasons.is_empty()
                || intent.reasons.len() > 64
            {
                return Err(LivingDecisionErrorV2::Candidate);
            }
            let diplomatic = matches!(
                intent.action,
                LivingIntentActionV2::Agreement { .. } | LivingIntentActionV2::War { .. }
            );
            if diplomatic != (intent.phase == LivingPhaseV2::UpdateDiplomacy) {
                return Err(LivingDecisionErrorV2::Candidate);
            }
            let key = (intent.phase.ordinal(), intent.actor, intent.ordinal);
            match previous {
                Some((phase, actor, ordinal)) if (phase, actor) == (key.0, key.1) => {
                    if ordinal == u16::MAX || intent.ordinal != ordinal + 1 {
                        return Err(LivingDecisionErrorV2::Candidate);
                    }
                }
                Some(old) if old >= key => return Err(LivingDecisionErrorV2::Candidate),
                _ => {
                    if intent.ordinal != 0 {
                        return Err(LivingDecisionErrorV2::Candidate);
                    }
                }
            }
            previous = Some(key);
            for reason in &intent.reasons {
                match reason {
                    LivingDecisionReasonV2::InsufficientResource {
                        required,
                        available,
                        reserve,
                        ..
                    } if [*required, *available, *reserve].iter().any(|v| *v > 10000) => {
                        return Err(LivingDecisionErrorV2::Candidate);
                    }
                    LivingDecisionReasonV2::OutputCapacity {
                        amount,
                        available_capacity,
                        ..
                    } if *amount > 10000 || *available_capacity > 10000 => {
                        return Err(LivingDecisionErrorV2::Candidate);
                    }
                    LivingDecisionReasonV2::BilateralThreshold {
                        forward,
                        reverse,
                        required,
                        ..
                    } if [*forward, *reverse, *required]
                        .iter()
                        .any(|v| !(-100..=100).contains(v)) =>
                    {
                        return Err(LivingDecisionErrorV2::Candidate);
                    }
                    LivingDecisionReasonV2::StaleObservation { observed_tick, .. }
                        if *observed_tick > self.tick =>
                    {
                        return Err(LivingDecisionErrorV2::Candidate);
                    }
                    _ => {}
                }
            }
        }
        if self.digest()? != self.integrity_sha256 {
            return Err(LivingDecisionErrorV2::Integrity);
        }
        Ok(())
    }
    /// Canonical public bytes after integrity validation.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LivingDecisionErrorV2> {
        self.verify()?;
        Ok(encode_canonical_v2(self, LIVING_MAX_DECISION_BYTES_V2)?)
    }
    /// Bind to the unchanged ordinary committed receipt.
    pub fn verify_receipt(
        &self,
        receipt: &LivingTickReceiptV2,
    ) -> Result<(), LivingDecisionErrorV2> {
        self.verify()?;
        receipt
            .verify()
            .map_err(|_| LivingDecisionErrorV2::ReceiptBinding)?;
        if self.tick != receipt.tick
            || self.state_after != receipt.state_digest
            || self.receipt_digest != receipt.digest
        {
            return Err(LivingDecisionErrorV2::ReceiptBinding);
        }
        Ok(())
    }
}
/// Seal a sorted boundary after the ordinary receipt commits. It never mutates state.
pub fn seal_living_decision_receipt_v2(
    catalog_hash: LivingCatalogHashV2,
    branch: LivingBranchIdV2,
    committed_state_before: LivingStateDigestV2,
    receipt: &LivingTickReceiptV2,
    intents: Vec<LivingIntentDecisionV2>,
) -> Result<LivingDecisionReceiptV2, LivingDecisionErrorV2> {
    receipt
        .verify()
        .map_err(|_| LivingDecisionErrorV2::ReceiptBinding)?;
    let mut record = LivingDecisionReceiptV2 {
        kind: LIVING_DECISION_KIND_V2.into(),
        format_version: 2,
        rules_version: 2,
        catalog_hash,
        branch,
        tick: receipt.tick,
        committed_state_before,
        state_after: receipt.state_digest,
        receipt_digest: receipt.digest,
        intents,
        integrity_sha256: LivingDecisionDigestV2(LivingReceiptDigestV2([0; 32])),
    };
    record.integrity_sha256 = record.digest()?;
    record.verify()?;
    Ok(record)
}
/// Strict bounded decoder; successful integrity is not a simulation replay proof.
pub fn decode_living_decision_receipt_v2(
    bytes: &[u8],
) -> Result<LivingDecisionReceiptV2, LivingDecisionErrorV2> {
    let record: LivingDecisionReceiptV2 = decode_canonical_v2(bytes, LIVING_MAX_DECISION_BYTES_V2)?;
    record.verify()?;
    Ok(record)
}
/// Hash the exact private phase snapshot with clock normalized to application T.
pub fn living_decision_snapshot_digest_v2(
    snapshot: &LivingGalaxyStateV2,
    tick: LivingTickV2,
) -> Result<LivingStateDigestV2, LivingDecisionErrorV2> {
    if tick.0 == 0 || snapshot.tick.0.checked_add(1) != Some(tick.0) {
        return Err(LivingDecisionErrorV2::Snapshot);
    }
    let mut normalized = snapshot.clone();
    normalized.tick = tick;
    normalized
        .digest()
        .map_err(|_| LivingDecisionErrorV2::Snapshot)
}
