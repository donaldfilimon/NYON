//! Additive, separately sealed facts emitted by ordinary-event producers.
//! Ordinary receipt payloads and identities remain unchanged. Missing facts
//! mean unknown; consumers never reconstruct them from a later state.
use super::{
    ids::*,
    model::{LivingAgreementV2, LivingBlockedReasonV2, LivingResourceV2, LivingRouteKindV2},
    receipt::{LivingEventKindV2, LivingPendingEventsV2, LivingTickReceiptV2},
    wire::{LivingWireErrorV2, decode_canonical_v2, encode_canonical_v2},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
/// Independent explanation domain.
pub const LIVING_EVENT_FACTS_DOMAIN_V2: &[u8] = b"NYON-LIVING-EVENT-FACTS-V2\0";
/// Independent explanation byte bound.
pub const LIVING_MAX_EVENT_FACTS_BYTES_V2: usize = 2 * 1024 * 1024;
/// Ephemeral replay-regenerated route restriction; excluded from canonical bytes.
/// This observes a restriction, not the cause of a later industry's shortage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingCorridorRestrictionV2 {
    pub(crate) tick: LivingTickV2,
    pub(crate) receipt_digest: LivingReceiptDigestV2,
    pub(crate) route: LivingEntityIdV2,
    pub(crate) source_world: LivingEntityIdV2,
    pub(crate) destination_world: LivingEntityIdV2,
    pub(crate) source_system: LivingEntityIdV2,
    pub(crate) destination_system: LivingEntityIdV2,
    pub(crate) sender: LivingEntityIdV2,
    pub(crate) receiver: LivingEntityIdV2,
    pub(crate) resource: LivingResourceV2,
    pub(crate) requested_batch: u32,
    pub(crate) eligible_batch: u32,
    pub(crate) restricted_batch: u32,
    pub(crate) actual_dispatched_units: u32,
    pub(crate) hazard: super::model::LivingHazardV2,
}
impl LivingCorridorRestrictionV2 {
    pub const fn tick(&self) -> LivingTickV2 {
        self.tick
    }
    pub const fn receipt_digest(&self) -> LivingReceiptDigestV2 {
        self.receipt_digest
    }
    pub const fn route(&self) -> LivingEntityIdV2 {
        self.route
    }
    pub const fn source_world(&self) -> LivingEntityIdV2 {
        self.source_world
    }
    pub const fn destination_world(&self) -> LivingEntityIdV2 {
        self.destination_world
    }
    pub const fn source_system(&self) -> LivingEntityIdV2 {
        self.source_system
    }
    pub const fn destination_system(&self) -> LivingEntityIdV2 {
        self.destination_system
    }
    pub const fn sender(&self) -> LivingEntityIdV2 {
        self.sender
    }
    pub const fn receiver(&self) -> LivingEntityIdV2 {
        self.receiver
    }
    pub const fn resource(&self) -> LivingResourceV2 {
        self.resource
    }
    pub const fn requested_batch(&self) -> u32 {
        self.requested_batch
    }
    pub const fn eligible_batch(&self) -> u32 {
        self.eligible_batch
    }
    pub const fn restricted_batch(&self) -> u32 {
        self.restricted_batch
    }
    /// Accepted shipment units; zero also represents a legitimate restriction to zero.
    pub const fn actual_dispatched_units(&self) -> u32 {
        self.actual_dispatched_units
    }
    pub fn hazard(&self) -> &super::model::LivingHazardV2 {
        &self.hazard
    }
}
/// What an actual resource quantity describes; order is canonical wire order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingEventResourceRoleV2 {
    Discarded,
    Input,
    Output,
    Paid,
    Refund,
    Transferred,
}
/// A measured quantity, never a forecast or inferred inventory delta.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventResourceFactV2 {
    pub role: LivingEventResourceRoleV2,
    pub resource: LivingResourceV2,
    pub quantity: u32,
}
/// Successful aggregate deposit consumption; no individual deposit identity is inferred.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingDepositInputFactV2 {
    pub world: LivingEntityIdV2,
    pub units: u32,
}
/// Actual accepted destination inventory transfer, recorded at delivery.
/// Sender/endpoints belong to the immutable manifest; systems describe the
/// available world records at this boundary, never the historical departure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingDeliveryFactV2 {
    pub source_world: LivingEntityIdV2,
    pub destination_world: LivingEntityIdV2,
    pub dispatch_owner: LivingEntityIdV2,
    pub intended_receiver: LivingEntityIdV2,
    pub recipient: LivingEntityIdV2,
    pub resource: LivingResourceV2,
    pub units: u32,
    /// None retains the honest unknown classification of legacy manifests.
    pub dispatch_kind: Option<LivingRouteKindV2>,
    /// The source world's current system, when the world still exists.
    pub source_system: Option<LivingEntityIdV2>,
    /// The destination world's system at the delivery boundary, when known.
    pub destination_system: Option<LivingEntityIdV2>,
}
/// Producer-supplied facts, with bounded sorted lists and no freeform motives.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventFactsV2 {
    pub actor: Option<LivingEntityIdV2>,
    pub entity: Option<LivingEntityIdV2>,
    pub resources: Vec<LivingEventResourceFactV2>,
    pub blocked_reasons: Vec<LivingBlockedReasonV2>,
    pub affected: Vec<LivingEntityIdV2>,
    /// Sparse additive success witness, omitted when absent to preserve old bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deposit_input: Option<LivingDepositInputFactV2>,
    /// The exact bilateral agreement accepted at this formation boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreement: Option<LivingAgreementV2>,
    /// The actual full inventory transfer, never an attempted delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<LivingDeliveryFactV2>,
}
impl LivingEventFactsV2 {
    /// Validate before appending the ordinary event, preserving rejected collectors.
    pub fn validate(&self, kind: LivingEventKindV2) -> Result<(), LivingEventFactsErrorV2> {
        let mut cursor = LivingEventFactsValidationCursorV2::new(kind);
        while !cursor.ready() {
            cursor.visit(self)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FactsValidationStage {
    Agreement,
    Delivery,
    Deposit,
    Lengths,
    Quantity,
    ResourceOrder,
    AffectedOrder,
    BlockedPresence,
    BlockedOrder,
    BlockedNumeric,
    Ready,
}

fn blocked_key(reason: &LivingBlockedReasonV2) -> (u8, Option<LivingResourceV2>) {
    match reason {
        LivingBlockedReasonV2::Disabled => (0, None),
        LivingBlockedReasonV2::MissingDeposit { .. } => (1, None),
        LivingBlockedReasonV2::MissingInput { resource, .. } => (2, Some(*resource)),
        LivingBlockedReasonV2::OutputFull { resource } => (3, Some(*resource)),
    }
}

/// One policy for both drivers. Pin the same immutable facts and kind until
/// terminal; the cursor holds no borrowed or owning facts/collection field.
pub(crate) struct LivingEventFactsValidationCursorV2 {
    kind: LivingEventKindV2,
    stage: FactsValidationStage,
    index: usize,
    fault: Option<LivingEventFactsErrorV2>,
    last_visit_visits: usize,
}
impl LivingEventFactsValidationCursorV2 {
    pub(crate) fn assert_non_owning(&self) {
        let Self {
            kind: _,
            stage: _,
            index: _,
            fault: _,
            last_visit_visits: _,
        } = self;
    }
    pub(crate) fn new(kind: LivingEventKindV2) -> Self {
        Self {
            kind,
            stage: FactsValidationStage::Agreement,
            index: 0,
            fault: None,
            last_visit_visits: 0,
        }
    }
    pub(crate) fn ready(&self) -> bool {
        self.stage == FactsValidationStage::Ready && self.fault.is_none()
    }
    pub(crate) fn last_visit_visits(&self) -> usize {
        self.last_visit_visits
    }
    pub(crate) fn visit(
        &mut self,
        facts: &LivingEventFactsV2,
    ) -> Result<bool, LivingEventFactsErrorV2> {
        self.last_visit_visits = 0;
        if let Some(error) = self.fault {
            return Err(error);
        }
        use FactsValidationStage as S;
        // Only the fixed stage graph can transition without a row visit.
        let invalid = loop {
            match self.stage {
                S::Agreement => {
                    self.stage = S::Delivery;
                    break facts.agreement.as_ref().is_some_and(|a| {
                        self.kind != LivingEventKindV2::AgreementFormed
                            || a.participant_a >= a.participant_b
                            || a.start_tick >= a.end_tick
                    });
                }
                S::Delivery => {
                    self.stage = S::Deposit;
                    break facts.delivery.as_ref().is_some_and(|d| {
                        self.kind != LivingEventKindV2::ShipmentDelivered
                            || d.recipient != d.intended_receiver
                            || d.units == 0
                            || d.units > 10_000
                            || (d.dispatch_kind == Some(LivingRouteKindV2::Internal)
                                && d.dispatch_owner != d.intended_receiver)
                    });
                }
                S::Deposit => {
                    self.stage = S::Lengths;
                    break facts.deposit_input.as_ref().is_some_and(|d| {
                        self.kind != LivingEventKindV2::FacilityProduced
                            || d.units == 0
                            || d.units > 10_000
                    });
                }
                S::Lengths => {
                    self.stage = S::Quantity;
                    break facts.resources.len() > 18
                        || facts.blocked_reasons.len() > 8
                        || facts.affected.len() > 64;
                }
                S::Quantity => {
                    if self.index == facts.resources.len() {
                        self.stage = S::ResourceOrder;
                        self.index = 1;
                        continue;
                    }
                    let invalid = facts.resources[self.index].quantity > 10_000;
                    self.index += 1;
                    break invalid;
                }
                S::ResourceOrder => {
                    if self.index >= facts.resources.len() {
                        self.stage = S::AffectedOrder;
                        self.index = 1;
                        continue;
                    }
                    let a = &facts.resources[self.index - 1];
                    let b = &facts.resources[self.index];
                    self.index += 1;
                    break (a.role, a.resource) >= (b.role, b.resource);
                }
                S::AffectedOrder => {
                    if self.index >= facts.affected.len() {
                        self.stage = S::BlockedPresence;
                        continue;
                    }
                    let invalid = facts.affected[self.index - 1] >= facts.affected[self.index];
                    self.index += 1;
                    break invalid;
                }
                S::BlockedPresence => {
                    self.stage = S::BlockedOrder;
                    self.index = 1;
                    break (self.kind == LivingEventKindV2::FacilityBlocked)
                        == facts.blocked_reasons.is_empty();
                }
                S::BlockedOrder => {
                    if self.index >= facts.blocked_reasons.len() {
                        self.stage = S::BlockedNumeric;
                        self.index = 0;
                        continue;
                    }
                    let invalid = blocked_key(&facts.blocked_reasons[self.index - 1])
                        >= blocked_key(&facts.blocked_reasons[self.index]);
                    self.index += 1;
                    break invalid;
                }
                S::BlockedNumeric => {
                    if self.index == facts.blocked_reasons.len() {
                        self.stage = S::Ready;
                        continue;
                    }
                    let invalid = match &facts.blocked_reasons[self.index] {
                        LivingBlockedReasonV2::MissingDeposit {
                            required,
                            available,
                        }
                        | LivingBlockedReasonV2::MissingInput {
                            required,
                            available,
                            ..
                        } => *required > 10_000 || *available >= *required,
                        LivingBlockedReasonV2::Disabled
                        | LivingBlockedReasonV2::OutputFull { .. } => false,
                    };
                    self.index += 1;
                    break invalid;
                }
                S::Ready => return Ok(false),
            }
        };
        self.last_visit_visits = 1;
        if invalid {
            self.fault = Some(LivingEventFactsErrorV2::Facts);
            Err(LivingEventFactsErrorV2::Facts)
        } else {
            Ok(true)
        }
    }
}
/// One exact ordinary emission ordinal and its derived identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingExplainedEventV2 {
    pub ordinal: u16,
    pub event_id: LivingEventIdV2,
    pub facts: LivingEventFactsV2,
}

/// A token is bounded before construction, not encoded into a growing Vec
/// and later split. Closed IDs/digests use at most66 quoted bytes.
pub(crate) struct CanonicalFragment {
    bytes: [u8; 256],
    len: usize,
}
impl CanonicalFragment {
    fn new() -> Self {
        Self {
            bytes: [0; 256],
            len: 0,
        }
    }
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}
impl std::io::Write for CanonicalFragment {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let end = self
            .len
            .checked_add(bytes.len())
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| std::io::Error::other("fixed facts token overflow"))?;
        self.bytes[self.len..end].copy_from_slice(bytes);
        self.len = end;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Closed dispatch: ID34, digest66, tick20, ordinal5, quantity10, resource8, role13,
/// agreement kind15, route kind10, blocked tag17, null4. IDs use their existing
/// fixed32-byte temporary String serializer. No compound record is a leaf.
#[derive(Clone, Copy)]
enum FactsJsonLeaf {
    Entity(LivingEntityIdV2),
    Event(LivingEventIdV2),
    LivingStateDigest(LivingStateDigestV2),
    LivingReceiptDigest(LivingReceiptDigestV2),
    Ordinal(u16),
    Quantity(u32),
    Tick(LivingTickV2),
    Resource(LivingResourceV2),
    Role(LivingEventResourceRoleV2),
    AgreementKind(super::model::LivingAgreementKindV2),
    RouteKind(LivingRouteKindV2),
    BlockedTag(LivingBlockedReasonV2),
    Null,
}
impl FactsJsonLeaf {
    fn encode(self, output: &mut CanonicalFragment) -> Result<(), LivingWireErrorV2> {
        // This private helper is reachable only through the proven scalar cases.
        fn scalar<T: Serialize>(
            output: &mut CanonicalFragment,
            value: &T,
        ) -> Result<(), LivingWireErrorV2> {
            serde_json::to_writer(output, value).map_err(|_| LivingWireErrorV2::Json)
        }
        match self {
            Self::Entity(value) => scalar(output, &value),
            Self::Event(value) => scalar(output, &value),
            Self::LivingStateDigest(value) => scalar(output, &value),
            Self::LivingReceiptDigest(value) => scalar(output, &value),
            Self::Ordinal(value) => scalar(output, &value),
            Self::Quantity(value) => scalar(output, &value),
            Self::Tick(value) => scalar(output, &value),
            Self::Resource(value) => scalar(output, &value),
            Self::Role(value) => scalar(output, &value),
            Self::AgreementKind(value) => scalar(output, &value),
            Self::RouteKind(value) => scalar(output, &value),
            Self::BlockedTag(value) => scalar(output, &value.as_wire_str()),
            Self::Null => scalar(output, &()),
        }
    }
}
enum FactsJsonToken {
    Literal(&'static [u8]),
    Leaf(FactsJsonLeaf),
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum FactsEncodingFocus {
    Entry,
    Facts,
    Resources,
    ResourceRow,
    Blocked,
    BlockedRow,
    Affected,
    Deposit,
    Agreement,
    Delivery,
    Done,
}
/// Fixed object positions: open, key, colon, value, comma, close, done.
#[derive(Clone, Copy, Default)]
struct FactsObjectPosition {
    token: u8,
    field: usize,
}
impl FactsObjectPosition {
    fn assert_non_owning(&self) {
        let Self { token: _, field: _ } = self;
    }
}
enum FactsEncodingValue {
    Leaf(FactsJsonLeaf),
    Child(FactsEncodingFocus),
}

/// Pin the same immutable facts until terminal. Four schema-specific inline
/// positions suffice: entry, facts, one array and one row/witness. There is no
/// general recursive stack, prior-array replay or dynamic token storage.
pub(crate) struct LivingExplainedEntryEncodingCursorV2 {
    ordinal: u16,
    event_id: LivingEventIdV2,
    focus: FactsEncodingFocus,
    entry: FactsObjectPosition,
    facts: FactsObjectPosition,
    row: FactsObjectPosition,
    array_token: u8,
    array_index: usize,
    fault: bool,
    last_visit_visits: usize,
    #[cfg(test)]
    reject_next: bool,
}
impl LivingExplainedEntryEncodingCursorV2 {
    pub(crate) fn assert_non_owning(&self) {
        let Self {
            ordinal: _,
            event_id: _,
            focus: _,
            entry,
            facts,
            row,
            array_token: _,
            array_index: _,
            fault: _,
            last_visit_visits: _,
            #[cfg(test)]
                reject_next: _,
        } = self;
        entry.assert_non_owning();
        facts.assert_non_owning();
        row.assert_non_owning();
    }
    #[cfg(test)]
    pub(crate) fn reject_next_fragment(&mut self) {
        self.reject_next = true;
    }
    pub(crate) fn new(ordinal: u16, event_id: LivingEventIdV2) -> Self {
        Self {
            ordinal,
            event_id,
            focus: FactsEncodingFocus::Entry,
            entry: FactsObjectPosition::default(),
            facts: FactsObjectPosition::default(),
            row: FactsObjectPosition::default(),
            array_token: 0,
            array_index: 0,
            fault: false,
            last_visit_visits: 0,
            #[cfg(test)]
            reject_next: false,
        }
    }
    pub(crate) fn last_visit_visits(&self) -> usize {
        self.last_visit_visits
    }
    pub(crate) fn next_fragment(
        &mut self,
        facts: &LivingEventFactsV2,
    ) -> Result<Option<CanonicalFragment>, LivingWireErrorV2> {
        self.last_visit_visits = 0;
        if self.fault {
            return Err(LivingWireErrorV2::Json);
        }
        #[cfg(test)]
        if self.reject_next {
            self.fault = true;
            self.last_visit_visits = 1;
            return Err(LivingWireErrorV2::Json);
        }
        let Some(token) = self.next_token(facts) else {
            return Ok(None);
        };
        self.last_visit_visits = 1;
        let mut output = CanonicalFragment::new();
        let encoded = match token {
            FactsJsonToken::Literal(bytes) => {
                use std::io::Write;
                output.write_all(bytes).map_err(|_| LivingWireErrorV2::Json)
            }
            FactsJsonToken::Leaf(value) => value.encode(&mut output),
        };
        if encoded.is_err() {
            self.fault = true;
        }
        encoded?;
        debug_assert!(!output.as_bytes().is_empty());
        Ok(Some(output))
    }
    fn position(&self) -> FactsObjectPosition {
        use FactsEncodingFocus as F;
        match self.focus {
            F::Entry => self.entry,
            F::Facts => self.facts,
            _ => self.row,
        }
    }
    fn position_mut(&mut self) -> &mut FactsObjectPosition {
        use FactsEncodingFocus as F;
        match self.focus {
            F::Entry => &mut self.entry,
            F::Facts => &mut self.facts,
            _ => &mut self.row,
        }
    }
    fn fields(&self, facts: &LivingEventFactsV2) -> usize {
        use FactsEncodingFocus as F;
        match self.focus {
            F::Entry => 3,
            F::Facts => {
                5 + usize::from(facts.deposit_input.is_some())
                    + usize::from(facts.agreement.is_some())
                    + usize::from(facts.delivery.is_some())
            }
            F::ResourceRow => 3,
            F::BlockedRow => match facts.blocked_reasons[self.array_index] {
                LivingBlockedReasonV2::Disabled => 1,
                LivingBlockedReasonV2::MissingDeposit { .. } => 3,
                LivingBlockedReasonV2::MissingInput { .. } => 4,
                LivingBlockedReasonV2::OutputFull { .. } => 2,
            },
            F::Deposit => 2,
            F::Agreement => 5,
            F::Delivery => 10,
            _ => unreachable!("fixed object focus"),
        }
    }
    fn facts_field(facts: &LivingEventFactsV2, mut index: usize) -> usize {
        if index < 5 {
            return index;
        }
        index -= 5;
        // Exactly three sparse fields, independent of array sizes.
        for (field, present) in [
            (5, facts.deposit_input.is_some()),
            (6, facts.agreement.is_some()),
            (7, facts.delivery.is_some()),
        ] {
            if present {
                if index == 0 {
                    return field;
                }
                index -= 1;
            }
        }
        unreachable!("present sparse field")
    }
    fn key(&self, facts: &LivingEventFactsV2, field: usize) -> &'static [u8] {
        use FactsEncodingFocus as F;
        match self.focus {
            F::Entry => [b"\"ordinal\"".as_slice(), b"\"event_id\"", b"\"facts\""][field],
            F::Facts => [
                b"\"actor\"".as_slice(),
                b"\"entity\"",
                b"\"resources\"",
                b"\"blocked_reasons\"",
                b"\"affected\"",
                b"\"deposit_input\"",
                b"\"agreement\"",
                b"\"delivery\"",
            ][Self::facts_field(facts, field)],
            F::ResourceRow => [b"\"role\"".as_slice(), b"\"resource\"", b"\"quantity\""][field],
            F::BlockedRow => match facts.blocked_reasons[self.array_index] {
                LivingBlockedReasonV2::Disabled => b"\"type\"",
                LivingBlockedReasonV2::MissingDeposit { .. } => {
                    [b"\"type\"".as_slice(), b"\"required\"", b"\"available\""][field]
                }
                LivingBlockedReasonV2::MissingInput { .. } => [
                    b"\"type\"".as_slice(),
                    b"\"resource\"",
                    b"\"required\"",
                    b"\"available\"",
                ][field],
                LivingBlockedReasonV2::OutputFull { .. } => {
                    [b"\"type\"".as_slice(), b"\"resource\""][field]
                }
            },
            F::Deposit => [b"\"world\"".as_slice(), b"\"units\""][field],
            F::Agreement => [
                b"\"kind\"".as_slice(),
                b"\"participant_a\"",
                b"\"participant_b\"",
                b"\"start_tick\"",
                b"\"end_tick\"",
            ][field],
            F::Delivery => [
                b"\"source_world\"".as_slice(),
                b"\"destination_world\"",
                b"\"dispatch_owner\"",
                b"\"intended_receiver\"",
                b"\"recipient\"",
                b"\"resource\"",
                b"\"units\"",
                b"\"dispatch_kind\"",
                b"\"source_system\"",
                b"\"destination_system\"",
            ][field],
            _ => unreachable!("fixed object key"),
        }
    }
    fn value(&self, facts: &LivingEventFactsV2, field: usize) -> FactsEncodingValue {
        use FactsEncodingFocus as F;
        use FactsEncodingValue::{Child, Leaf};
        use FactsJsonLeaf as L;
        fn entity(value: Option<LivingEntityIdV2>) -> FactsJsonLeaf {
            value.map_or(FactsJsonLeaf::Null, FactsJsonLeaf::Entity)
        }
        match self.focus {
            F::Entry => match field {
                0 => Leaf(L::Ordinal(self.ordinal)),
                1 => Leaf(L::Event(self.event_id)),
                _ => Child(F::Facts),
            },
            F::Facts => match Self::facts_field(facts, field) {
                0 => Leaf(entity(facts.actor)),
                1 => Leaf(entity(facts.entity)),
                2 => Child(F::Resources),
                3 => Child(F::Blocked),
                4 => Child(F::Affected),
                5 => Child(F::Deposit),
                6 => Child(F::Agreement),
                _ => Child(F::Delivery),
            },
            F::ResourceRow => {
                let r = &facts.resources[self.array_index];
                Leaf(match field {
                    0 => L::Role(r.role),
                    1 => L::Resource(r.resource),
                    _ => L::Quantity(r.quantity),
                })
            }
            F::BlockedRow => {
                let reason = facts.blocked_reasons[self.array_index];
                if field == 0 {
                    return Leaf(L::BlockedTag(reason));
                }
                Leaf(match reason {
                    LivingBlockedReasonV2::MissingDeposit {
                        required,
                        available,
                    } => L::Quantity(if field == 1 { required } else { available }),
                    LivingBlockedReasonV2::MissingInput {
                        resource,
                        required,
                        available,
                    } => match field {
                        1 => L::Resource(resource),
                        2 => L::Quantity(required),
                        _ => L::Quantity(available),
                    },
                    LivingBlockedReasonV2::OutputFull { resource } => L::Resource(resource),
                    LivingBlockedReasonV2::Disabled => unreachable!("unit tag only"),
                })
            }
            F::Deposit => {
                let d = facts.deposit_input.as_ref().expect("present deposit");
                Leaf(if field == 0 {
                    L::Entity(d.world)
                } else {
                    L::Quantity(d.units)
                })
            }
            F::Agreement => {
                let a = facts.agreement.as_ref().expect("present agreement");
                Leaf(match field {
                    0 => L::AgreementKind(a.kind),
                    1 => L::Entity(a.participant_a),
                    2 => L::Entity(a.participant_b),
                    3 => L::Tick(a.start_tick),
                    _ => L::Tick(a.end_tick),
                })
            }
            F::Delivery => {
                let d = facts.delivery.as_ref().expect("present delivery");
                Leaf(match field {
                    0 => L::Entity(d.source_world),
                    1 => L::Entity(d.destination_world),
                    2 => L::Entity(d.dispatch_owner),
                    3 => L::Entity(d.intended_receiver),
                    4 => L::Entity(d.recipient),
                    5 => L::Resource(d.resource),
                    6 => L::Quantity(d.units),
                    7 => d.dispatch_kind.map_or(L::Null, L::RouteKind),
                    8 => entity(d.source_system),
                    _ => entity(d.destination_system),
                })
            }
            _ => unreachable!("fixed object value"),
        }
    }
    fn next_token(&mut self, facts: &LivingEventFactsV2) -> Option<FactsJsonToken> {
        use FactsEncodingFocus as F;
        use FactsJsonToken::{Leaf, Literal};
        loop {
            if self.focus == F::Done {
                return None;
            }
            if matches!(self.focus, F::Resources | F::Blocked | F::Affected) {
                let len = match self.focus {
                    F::Resources => facts.resources.len(),
                    F::Blocked => facts.blocked_reasons.len(),
                    _ => facts.affected.len(),
                };
                match self.array_token {
                    0 => {
                        self.array_token = if len == 0 { 3 } else { 1 };
                        return Some(Literal(b"["));
                    }
                    1 => {
                        self.array_token = if self.array_index + 1 < len { 2 } else { 3 };
                        match self.focus {
                            F::Resources => self.focus = F::ResourceRow,
                            F::Blocked => self.focus = F::BlockedRow,
                            _ => {
                                return Some(Leaf(FactsJsonLeaf::Entity(
                                    facts.affected[self.array_index],
                                )));
                            }
                        }
                        self.row = FactsObjectPosition::default();
                    }
                    2 => {
                        self.array_index += 1;
                        self.array_token = 1;
                        return Some(Literal(b","));
                    }
                    3 => {
                        self.array_token = 4;
                        return Some(Literal(b"]"));
                    }
                    _ => self.focus = F::Facts,
                }
                continue;
            }
            let position = self.position();
            match position.token {
                0 => {
                    self.position_mut().token = 1;
                    return Some(Literal(b"{"));
                }
                1 => {
                    self.position_mut().token = 2;
                    return Some(Literal(self.key(facts, position.field)));
                }
                2 => {
                    self.position_mut().token = 3;
                    return Some(Literal(b":"));
                }
                3 => {
                    self.position_mut().token = if position.field + 1 < self.fields(facts) {
                        4
                    } else {
                        5
                    };
                    match self.value(facts, position.field) {
                        FactsEncodingValue::Leaf(value) => return Some(Leaf(value)),
                        FactsEncodingValue::Child(focus) => {
                            self.focus = focus;
                            match focus {
                                F::Facts => self.facts = FactsObjectPosition::default(),
                                F::Resources | F::Blocked | F::Affected => {
                                    self.array_token = 0;
                                    self.array_index = 0;
                                }
                                _ => self.row = FactsObjectPosition::default(),
                            }
                        }
                    }
                }
                4 => {
                    let p = self.position_mut();
                    p.field += 1;
                    p.token = 1;
                    return Some(Literal(b","));
                }
                5 => {
                    self.position_mut().token = 6;
                    return Some(Literal(b"}"));
                }
                _ => {
                    self.focus = match self.focus {
                        F::Entry => F::Done,
                        F::Facts => F::Entry,
                        F::ResourceRow => F::Resources,
                        F::BlockedRow => F::Blocked,
                        _ => F::Facts,
                    }
                }
            }
        }
    }
}
/// Why requested producer explanations are unavailable.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingEventFactsOmissionReasonV2 {
    Capacity,
}
/// Inclusive ordinary-event ordinal ranges whose requested facts did not fit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventFactsOmittedV2 {
    pub reason: LivingEventFactsOmissionReasonV2,
    pub ranges: Vec<[u16; 2]>,
}
/// Separately authenticated facts bound to one ordinary receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventFactsReceiptV2 {
    pub tick: LivingTickV2,
    pub state_digest: LivingStateDigestV2,
    pub receipt_digest: LivingReceiptDigestV2,
    pub events: Vec<LivingExplainedEventV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omitted: Option<LivingEventFactsOmittedV2>,
    pub integrity_sha256: LivingReceiptDigestV2,
}
#[derive(Serialize)]
struct Payload<'a> {
    tick: LivingTickV2,
    state_digest: LivingStateDigestV2,
    receipt_digest: LivingReceiptDigestV2,
    events: &'a [LivingExplainedEventV2],
    #[serde(skip_serializing_if = "Option::is_none")]
    omitted: &'a Option<LivingEventFactsOmittedV2>,
}
/// Typed collector and binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingEventFactsErrorV2 {
    #[error("invalid event facts wire: {0}")]
    Wire(#[from] LivingWireErrorV2),
    #[error("ordinary event collector rejected: {0}")]
    Receipt(#[from] super::receipt::LivingReceiptErrorV2),
    #[error("event facts capacity exceeded")]
    Capacity,
    #[error("invalid bounded event facts")]
    Facts,
    #[error("event facts do not bind this ordinary receipt")]
    Binding,
    #[error("event facts integrity mismatch")]
    Integrity,
}
impl LivingEventFactsReceiptV2 {
    fn calculated_digest(&self) -> Result<LivingReceiptDigestV2, LivingEventFactsErrorV2> {
        let bytes = encode_canonical_v2(
            &Payload {
                tick: self.tick,
                state_digest: self.state_digest,
                receipt_digest: self.receipt_digest,
                events: &self.events,
                omitted: &self.omitted,
            },
            LIVING_MAX_EVENT_FACTS_BYTES_V2,
        )?;
        let mut h = Sha256::new();
        h.update(LIVING_EVENT_FACTS_DOMAIN_V2);
        h.update(bytes);
        Ok(LivingReceiptDigestV2(h.finalize().into()))
    }
    /// Validate the entire sidecar and ordinary receipt before publishing any facts.
    pub fn verify_receipt(
        &self,
        receipt: &LivingTickReceiptV2,
    ) -> Result<(), LivingEventFactsErrorV2> {
        self.canonical_bytes()?;
        receipt.verify()?;
        if self.tick != receipt.tick
            || self.state_digest != receipt.state_digest
            || self.receipt_digest != receipt.digest
            || !self.events.windows(2).all(|p| p[0].ordinal < p[1].ordinal)
        {
            return Err(LivingEventFactsErrorV2::Binding);
        }
        for entry in &self.events {
            let e = receipt
                .events
                .get(usize::from(entry.ordinal))
                .ok_or(LivingEventFactsErrorV2::Binding)?;
            if e.ordinal != entry.ordinal || e.id != entry.event_id {
                return Err(LivingEventFactsErrorV2::Binding);
            }
            entry.facts.validate(e.kind)?;
            if entry
                .facts
                .agreement
                .as_ref()
                .is_some_and(|agreement| agreement.start_tick != receipt.tick)
            {
                return Err(LivingEventFactsErrorV2::Binding);
            }
        }
        if let Some(omitted) = &self.omitted {
            if omitted.ranges.is_empty()
                || self
                    .events
                    .last()
                    .is_some_and(|event| omitted.ranges[0][0] <= event.ordinal)
                || omitted
                    .ranges
                    .iter()
                    .any(|[start, end]| start > end || usize::from(*end) >= receipt.events.len())
                || !omitted
                    .ranges
                    .windows(2)
                    .all(|pair| u32::from(pair[0][1]) + 1 < u32::from(pair[1][0]))
            {
                return Err(LivingEventFactsErrorV2::Binding);
            }
            let mut ranges = omitted.ranges.iter().peekable();
            for event in &self.events {
                while ranges.peek().is_some_and(|range| range[1] < event.ordinal) {
                    ranges.next();
                }
                if ranges.peek().is_some_and(|range| range[0] <= event.ordinal) {
                    return Err(LivingEventFactsErrorV2::Binding);
                }
            }
        }
        if self.integrity_sha256 != self.calculated_digest()? {
            return Err(LivingEventFactsErrorV2::Integrity);
        }
        Ok(())
    }
    /// Canonical bytes, bounded independently of ordinary receipts.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LivingEventFactsErrorV2> {
        Ok(encode_canonical_v2(self, LIVING_MAX_EVENT_FACTS_BYTES_V2)?)
    }
    /// Strict decoding plus complete pair authentication.
    pub fn decode(
        bytes: &[u8],
        receipt: &LivingTickReceiptV2,
    ) -> Result<Self, LivingEventFactsErrorV2> {
        let result: Self = decode_canonical_v2(bytes, LIVING_MAX_EVENT_FACTS_BYTES_V2)?;
        result.verify_receipt(receipt)?;
        Ok(result)
    }
}
/// Seal actual collector facts after ordinary identities have been derived.
pub fn seal_living_event_facts_receipt_v2(
    receipt: &LivingTickReceiptV2,
    pending: &LivingPendingEventsV2,
) -> Result<LivingEventFactsReceiptV2, LivingEventFactsErrorV2> {
    receipt.verify()?;
    if !pending.explanations_enabled() {
        return Err(LivingEventFactsErrorV2::Binding);
    }
    if pending.payloads().len() != receipt.events.len()
        || pending
            .payloads()
            .iter()
            .zip(&receipt.events)
            .any(|(p, e)| {
                p.ordinal != e.ordinal || p.kind != e.kind || p.provenance != e.provenance
            })
    {
        return Err(LivingEventFactsErrorV2::Binding);
    }
    let candidates = pending
        .explained
        .iter()
        .map(|(ordinal, facts)| {
            let e = receipt
                .events
                .get(usize::from(*ordinal))
                .ok_or(LivingEventFactsErrorV2::Binding)?;
            Ok(LivingExplainedEventV2 {
                ordinal: *ordinal,
                event_id: e.id,
                facts: facts.clone(),
            })
        })
        .collect::<Result<Vec<_>, LivingEventFactsErrorV2>>()?;
    let build = |retained: usize| {
        let mut ranges: Vec<[u16; 2]> = Vec::new();
        for event in &candidates[retained..] {
            if let Some(last) = ranges.last_mut()
                && u32::from(last[1]) + 1 == u32::from(event.ordinal)
            {
                last[1] = event.ordinal;
            } else {
                ranges.push([event.ordinal, event.ordinal]);
            }
        }
        LivingEventFactsReceiptV2 {
            tick: receipt.tick,
            state_digest: receipt.state_digest,
            receipt_digest: receipt.digest,
            events: candidates[..retained].to_vec(),
            omitted: (!ranges.is_empty()).then_some(LivingEventFactsOmittedV2 {
                reason: LivingEventFactsOmissionReasonV2::Capacity,
                ranges,
            }),
            integrity_sha256: LivingReceiptDigestV2([0; 32]),
        }
    };
    // Every detailed entry costs more than its corresponding range boundary;
    // therefore serialized size is monotone in the retained ordinal prefix.
    // Include the complete envelope, fixed-width integrity and omission marker.
    let mut low = 0;
    let mut high = candidates.len();
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if build(middle).canonical_bytes().is_ok() {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let mut r = build(low);
    r.canonical_bytes()?;
    r.integrity_sha256 = r.calculated_digest()?;
    r.verify_receipt(receipt)?;
    Ok(r)
}

/// A view is supplied for each visit, never stored beside its owning job.
#[derive(Clone, Copy)]
enum FactsEncodingView<'a> {
    Prefix {
        pending: &'a LivingPendingEventsV2,
        receipt: &'a LivingTickReceiptV2,
        retained: usize,
    },
    Sealed(&'a LivingEventFactsReceiptV2),
}
impl<'a> FactsEncodingView<'a> {
    fn tick(self) -> LivingTickV2 {
        match self {
            Self::Prefix { receipt, .. } => receipt.tick,
            Self::Sealed(r) => r.tick,
        }
    }
    fn state_digest(self) -> LivingStateDigestV2 {
        match self {
            Self::Prefix { receipt, .. } => receipt.state_digest,
            Self::Sealed(r) => r.state_digest,
        }
    }
    fn receipt_digest(self) -> LivingReceiptDigestV2 {
        match self {
            Self::Prefix { receipt, .. } => receipt.digest,
            Self::Sealed(r) => r.receipt_digest,
        }
    }
    fn integrity(self) -> LivingReceiptDigestV2 {
        match self {
            Self::Prefix { .. } => LivingReceiptDigestV2([0; 32]),
            Self::Sealed(r) => r.integrity_sha256,
        }
    }
    fn count(self) -> usize {
        match self {
            Self::Prefix { retained, .. } => retained,
            Self::Sealed(r) => r.events.len(),
        }
    }
    fn omitted(self) -> bool {
        match self {
            Self::Prefix {
                pending, retained, ..
            } => retained < pending.explained.len(),
            Self::Sealed(r) => r.omitted.is_some(),
        }
    }
    fn entry(self, index: usize) -> (u16, LivingEventIdV2, &'a LivingEventFactsV2) {
        match self {
            Self::Prefix {
                pending, receipt, ..
            } => {
                let (ordinal, facts) = &pending.explained[index];
                (*ordinal, receipt.events[usize::from(*ordinal)].id, facts)
            }
            Self::Sealed(r) => {
                let e = &r.events[index];
                (e.ordinal, e.event_id, &e.facts)
            }
        }
    }
}
// The largest variant owns one fixed stack fragment (<=256 payload bytes).
// Boxing this per token would introduce allocation into every encoding visit.
#[allow(clippy::large_enum_variant)]
enum FactsEncodingStep {
    Fragment(CanonicalFragment),
    Scan,
    Done,
}
/// Closed envelope schema. Range coalescing scans at most one source row per
/// visit. Scan/entry transitions are work even when they emit no bytes.
struct FactsEnvelopeCursor {
    stage: u8,
    index: usize,
    range_index: usize,
    range: Option<[u16; 2]>,
    next_ordinal: Option<u16>,
    entry: Option<LivingExplainedEntryEncodingCursorV2>,
    envelope: bool,
    bytes: usize,
    hasher: Sha256,
    #[cfg(test)]
    tokens: usize,
    #[cfg(test)]
    max_fragment: usize,
}
impl FactsEnvelopeCursor {
    fn new(envelope: bool) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(LIVING_EVENT_FACTS_DOMAIN_V2);
        Self {
            stage: 0,
            index: 0,
            range_index: 0,
            range: None,
            next_ordinal: None,
            entry: None,
            envelope,
            bytes: 0,
            hasher,
            #[cfg(test)]
            tokens: 0,
            #[cfg(test)]
            max_fragment: 0,
        }
    }
    fn assert_non_owning(&self) {
        let Self {
            stage: _,
            index: _,
            range_index: _,
            range: _,
            next_ordinal: _,
            entry,
            envelope: _,
            bytes: _,
            hasher: _,
            #[cfg(test)]
                tokens: _,
            #[cfg(test)]
                max_fragment: _,
        } = self;
        if let Some(entry) = entry {
            entry.assert_non_owning();
        }
    }
    fn capacity(&self) -> Result<(), LivingEventFactsErrorV2> {
        if self.bytes > LIVING_MAX_EVENT_FACTS_BYTES_V2 {
            Err(LivingWireErrorV2::TooLarge {
                actual: self.bytes,
                limit: LIVING_MAX_EVENT_FACTS_BYTES_V2,
            }
            .into())
        } else {
            Ok(())
        }
    }
    fn digest(&self) -> LivingReceiptDigestV2 {
        LivingReceiptDigestV2(self.hasher.clone().finalize().into())
    }
    fn visit(
        &mut self,
        view: FactsEncodingView<'_>,
    ) -> Result<FactsEncodingStep, LivingEventFactsErrorV2> {
        let step = self.next(view)?;
        if let FactsEncodingStep::Fragment(fragment) = &step {
            self.bytes += fragment.len;
            #[cfg(test)]
            {
                self.tokens += 1;
                self.max_fragment = self.max_fragment.max(fragment.len);
            }
            if self.bytes <= LIVING_MAX_EVENT_FACTS_BYTES_V2 {
                self.hasher.update(fragment.as_bytes());
            }
        }
        Ok(step)
    }
    fn next(
        &mut self,
        view: FactsEncodingView<'_>,
    ) -> Result<FactsEncodingStep, LivingEventFactsErrorV2> {
        use FactsJsonToken::{Leaf, Literal};
        let token = match self.stage {
            0 => Literal(b"{"),
            1 => Literal(b"\"tick\""),
            2 => Literal(b":"),
            3 => Leaf(FactsJsonLeaf::Tick(view.tick())),
            4 => Literal(b","),
            5 => Literal(b"\"state_digest\""),
            6 => Literal(b":"),
            7 => Leaf(FactsJsonLeaf::LivingStateDigest(view.state_digest())),
            8 => Literal(b","),
            9 => Literal(b"\"receipt_digest\""),
            10 => Literal(b":"),
            11 => Leaf(FactsJsonLeaf::LivingReceiptDigest(view.receipt_digest())),
            12 => Literal(b","),
            13 => Literal(b"\"events\""),
            14 => Literal(b":"),
            15 => Literal(b"["),
            16 => {
                if self.index < view.count() {
                    let (ordinal, id, _) = view.entry(self.index);
                    self.entry = Some(LivingExplainedEntryEncodingCursorV2::new(ordinal, id));
                    self.stage = 17;
                    return Ok(FactsEncodingStep::Scan);
                }
                self.stage = 19;
                Literal(b"]")
            }
            17 => {
                if let Some(fragment) = self
                    .entry
                    .as_mut()
                    .unwrap()
                    .next_fragment(view.entry(self.index).2)?
                {
                    return Ok(FactsEncodingStep::Fragment(fragment));
                }
                self.entry = None;
                self.index += 1;
                self.stage = 18;
                return Ok(FactsEncodingStep::Scan);
            }
            18 => {
                self.stage = 16;
                if self.index < view.count() {
                    return Self::literal(b",");
                }
                return Ok(FactsEncodingStep::Scan);
            }
            20 => {
                if !view.omitted() {
                    self.stage = 40;
                    return Ok(FactsEncodingStep::Scan);
                }
                self.range_index = match view {
                    FactsEncodingView::Prefix { retained, .. } => retained,
                    _ => 0,
                };
                Literal(b",")
            }
            21 => Literal(b"\"omitted\""),
            22 => Literal(b":"),
            23 => Literal(b"{"),
            24 => Literal(b"\"reason\""),
            25 => Literal(b":"),
            26 => Literal(b"\"capacity\""),
            27 => Literal(b","),
            28 => Literal(b"\"ranges\""),
            29 => Literal(b":"),
            30 => Literal(b"["),
            31 => {
                match view {
                    FactsEncodingView::Prefix { pending, .. } => {
                        if let Some((ordinal, _)) = pending.explained.get(self.range_index) {
                            self.range_index += 1;
                            match self.range.as_mut() {
                                None => self.range = Some([*ordinal, *ordinal]),
                                Some(range) if u32::from(range[1]) + 1 == u32::from(*ordinal) => {
                                    range[1] = *ordinal
                                }
                                Some(_) => {
                                    self.next_ordinal = Some(*ordinal);
                                    self.stage = 32;
                                }
                            }
                        } else if self.range.is_some() {
                            self.stage = 32;
                        } else {
                            self.stage = 38;
                        }
                    }
                    FactsEncodingView::Sealed(r) => {
                        if let Some(range) =
                            r.omitted.as_ref().unwrap().ranges.get(self.range_index)
                        {
                            self.range = Some(*range);
                            self.range_index += 1;
                            self.stage = 32;
                        } else {
                            self.stage = 38;
                        }
                    }
                }
                return Ok(FactsEncodingStep::Scan);
            }
            32 => Literal(b"["),
            33 => Leaf(FactsJsonLeaf::Ordinal(self.range.unwrap()[0])),
            34 => Literal(b","),
            35 => Leaf(FactsJsonLeaf::Ordinal(self.range.unwrap()[1])),
            36 => Literal(b"]"),
            37 => {
                self.range = self.next_ordinal.take().map(|ordinal| [ordinal, ordinal]);
                let more = match view {
                    FactsEncodingView::Prefix { .. } => self.range.is_some(),
                    FactsEncodingView::Sealed(r) => {
                        self.range_index < r.omitted.as_ref().unwrap().ranges.len()
                    }
                };
                self.stage = if more { 31 } else { 38 };
                if more {
                    return Self::literal(b",");
                }
                return Ok(FactsEncodingStep::Scan);
            }
            38 => Literal(b"]"),
            39 => Literal(b"}"),
            40 => {
                if !self.envelope {
                    self.stage = 45;
                    return Ok(FactsEncodingStep::Scan);
                }
                Literal(b",")
            }
            41 => Literal(b"\"integrity_sha256\""),
            42 => Literal(b":"),
            43 => {
                self.stage = 44;
                Leaf(FactsJsonLeaf::LivingReceiptDigest(view.integrity()))
            }
            45 => Literal(b"}"),
            46 => return Ok(FactsEncodingStep::Done),
            _ => unreachable!("closed envelope token stage"),
        };
        self.stage += 1;
        let mut fragment = CanonicalFragment::new();
        match token {
            Literal(bytes) => {
                use std::io::Write;
                fragment
                    .write_all(bytes)
                    .map_err(|_| LivingWireErrorV2::Json)?;
            }
            Leaf(leaf) => leaf.encode(&mut fragment)?,
        }
        Ok(FactsEncodingStep::Fragment(fragment))
    }
    fn literal(bytes: &'static [u8]) -> Result<FactsEncodingStep, LivingEventFactsErrorV2> {
        use std::io::Write;
        let mut fragment = CanonicalFragment::new();
        fragment
            .write_all(bytes)
            .map_err(|_| LivingWireErrorV2::Json)?;
        Ok(FactsEncodingStep::Fragment(fragment))
    }
}

/// Final verification preserves the public verifier's grouped fault order.
struct FactsPairVerificationCursor {
    stage: u8,
    index: usize,
    range_index: usize,
    encoding: FactsEnvelopeCursor,
    ordinary: super::receipt::LivingOrdinaryReceiptVerificationCursorV2,
    semantic: Option<LivingEventFactsValidationCursorV2>,
    fault: Option<LivingEventFactsErrorV2>,
}
impl FactsPairVerificationCursor {
    fn new() -> Self {
        Self {
            stage: 0,
            index: 0,
            range_index: 0,
            encoding: FactsEnvelopeCursor::new(true),
            ordinary: super::receipt::LivingOrdinaryReceiptVerificationCursorV2::new(),
            semantic: None,
            fault: None,
        }
    }
    fn assert_non_owning(&self) {
        let Self {
            stage: _,
            index: _,
            range_index: _,
            encoding,
            ordinary,
            semantic,
            fault: _,
        } = self;
        encoding.assert_non_owning();
        ordinary.assert_non_owning();
        if let Some(semantic) = semantic {
            semantic.assert_non_owning();
        }
    }
    fn ready(&self) -> bool {
        self.stage == 12 && self.fault.is_none()
    }
    fn visit(
        &mut self,
        facts: &LivingEventFactsReceiptV2,
        receipt: &LivingTickReceiptV2,
    ) -> Result<bool, LivingEventFactsErrorV2> {
        if let Some(error) = self.fault {
            return Err(error);
        }
        if self.ready() {
            return Ok(false);
        }
        let result = self.visit_inner(facts, receipt);
        if let Err(error) = result {
            self.fault = Some(error);
        }
        result.map(|()| true)
    }
    fn visit_inner(
        &mut self,
        facts: &LivingEventFactsReceiptV2,
        receipt: &LivingTickReceiptV2,
    ) -> Result<(), LivingEventFactsErrorV2> {
        use LivingEventFactsErrorV2::Binding;
        match self.stage {
            0 => {
                if matches!(
                    self.encoding.visit(FactsEncodingView::Sealed(facts))?,
                    FactsEncodingStep::Done
                ) {
                    self.encoding.capacity()?;
                    self.stage = 1;
                }
            }
            1 => {
                if self.ordinary.ready() {
                    self.stage = 2;
                } else {
                    self.ordinary.visit(receipt)?;
                }
            }
            2 => {
                if facts.tick != receipt.tick
                    || facts.state_digest != receipt.state_digest
                    || facts.receipt_digest != receipt.digest
                {
                    return Err(Binding);
                }
                self.index = 1;
                self.stage = 3;
            }
            3 => {
                if self.index < facts.events.len() {
                    if facts.events[self.index - 1].ordinal >= facts.events[self.index].ordinal {
                        return Err(Binding);
                    }
                    self.index += 1;
                } else {
                    self.index = 0;
                    self.stage = 4;
                }
            }
            4 => {
                if let Some(entry) = facts.events.get(self.index) {
                    let ordinary = receipt
                        .events
                        .get(usize::from(entry.ordinal))
                        .ok_or(Binding)?;
                    if ordinary.ordinal != entry.ordinal || ordinary.id != entry.event_id {
                        return Err(Binding);
                    }
                    self.semantic = Some(LivingEventFactsValidationCursorV2::new(ordinary.kind));
                    self.stage = 5;
                } else {
                    self.stage = 7;
                }
            }
            5 => {
                let semantic = self.semantic.as_mut().unwrap();
                if semantic.ready() {
                    self.semantic = None;
                    self.stage = 6;
                } else {
                    semantic.visit(&facts.events[self.index].facts)?;
                }
            }
            6 => {
                if facts.events[self.index]
                    .facts
                    .agreement
                    .as_ref()
                    .is_some_and(|a| a.start_tick != receipt.tick)
                {
                    return Err(Binding);
                }
                self.index += 1;
                self.stage = 4;
            }
            7 => {
                if let Some(omitted) = &facts.omitted {
                    if omitted.ranges.is_empty()
                        || facts
                            .events
                            .last()
                            .is_some_and(|e| omitted.ranges[0][0] <= e.ordinal)
                    {
                        return Err(Binding);
                    }
                    self.index = 0;
                    self.stage = 8;
                } else {
                    self.encoding = FactsEnvelopeCursor::new(false);
                    self.stage = 11;
                }
            }
            8 => {
                let ranges = &facts.omitted.as_ref().unwrap().ranges;
                if let Some([start, end]) = ranges.get(self.index) {
                    if start > end || usize::from(*end) >= receipt.events.len() {
                        return Err(Binding);
                    }
                    self.index += 1;
                } else {
                    self.index = 1;
                    self.stage = 9;
                }
            }
            9 => {
                let ranges = &facts.omitted.as_ref().unwrap().ranges;
                if self.index < ranges.len() {
                    if u32::from(ranges[self.index - 1][1]) + 1 >= u32::from(ranges[self.index][0])
                    {
                        return Err(Binding);
                    }
                    self.index += 1;
                } else {
                    self.index = 0;
                    self.range_index = 0;
                    self.stage = 10;
                }
            }
            10 => {
                let ranges = &facts.omitted.as_ref().unwrap().ranges;
                if let Some(event) = facts.events.get(self.index) {
                    if let Some(range) = ranges.get(self.range_index) {
                        if range[1] < event.ordinal {
                            self.range_index += 1;
                        } else if range[0] <= event.ordinal {
                            return Err(Binding);
                        } else {
                            self.index += 1;
                        }
                    } else {
                        self.index += 1;
                    }
                } else {
                    self.encoding = FactsEnvelopeCursor::new(false);
                    self.stage = 11;
                }
            }
            11 => {
                if matches!(
                    self.encoding.visit(FactsEncodingView::Sealed(facts))?,
                    FactsEncodingStep::Done
                ) {
                    self.encoding.capacity()?;
                    if self.encoding.digest() != facts.integrity_sha256 {
                        return Err(LivingEventFactsErrorV2::Integrity);
                    }
                    self.stage = 12;
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

/// Original nested facts move through one retained row and prepared storage.
/// All cursors borrow these owners; every other field is fixed scalar state.
pub(crate) struct LivingEventFactsSealJobV2 {
    pending: Option<LivingPendingEventsV2>,
    receipt: Option<LivingTickReceiptV2>,
    source: Option<std::vec::IntoIter<(u16, LivingEventFactsV2)>>,
    held: Option<LivingExplainedEventV2>,
    events: Vec<LivingExplainedEventV2>,
    ranges: Vec<[u16; 2]>,
    events_storage: Option<super::step_job::LivingVecStorageJobV2<'static, LivingExplainedEventV2>>,
    ranges_storage: Option<super::step_job::LivingVecStorageJobV2<'static, [u16; 2]>>,
    output: Option<LivingEventFactsReceiptV2>,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    ordinary: super::receipt::LivingOrdinaryReceiptVerificationCursorV2,
    encoding: FactsEnvelopeCursor,
    verification: FactsPairVerificationCursor,
    stage: u8,
    index: usize,
    low: usize,
    high: usize,
    middle: usize,
    fault: Option<LivingEventFactsErrorV2>,
    capacity: bool,
    delivered: bool,
    last_visit_visits: usize,
}
impl LivingEventFactsSealJobV2 {
    #[cfg(test)]
    pub(crate) fn stage_for_test(&self) -> u8 {
        self.stage
    }
    pub(crate) fn begin(pending: LivingPendingEventsV2, receipt: LivingTickReceiptV2) -> Self {
        Self {
            pending: Some(pending),
            receipt: Some(receipt),
            source: None,
            held: None,
            events: Vec::new(),
            ranges: Vec::new(),
            events_storage: None,
            ranges_storage: None,
            output: None,
            retirement: Default::default(),
            ordinary: super::receipt::LivingOrdinaryReceiptVerificationCursorV2::new(),
            encoding: FactsEnvelopeCursor::new(true),
            verification: FactsPairVerificationCursor::new(),
            stage: 0,
            index: 0,
            low: 0,
            high: 0,
            middle: 0,
            fault: None,
            capacity: false,
            delivered: false,
            last_visit_visits: 0,
        }
    }
    pub(crate) fn ready(&self) -> bool {
        self.stage == 19
            && self.retirement.is_empty()
            && self.fault.is_none()
            && !self.capacity
            && !self.delivered
    }
    pub(crate) fn visit(&mut self) -> Result<bool, super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as E;
        self.last_visit_visits = 0;
        if let Some(fault) = self.fault {
            return Err(E::Fault(fault.into()));
        }
        if self.capacity {
            return Err(E::PreparationCapacity);
        }
        if self.stage == 19 || self.delivered {
            return Ok(false);
        }
        self.last_visit_visits = 1;
        if self.retirement.visit() {
            return Ok(true);
        }
        match self.visit_inner() {
            Ok(()) => Ok(true),
            Err(FactsSealError::Facts(error)) => {
                self.fault = Some(error);
                Err(E::Fault(error.into()))
            }
            Err(FactsSealError::Storage) => {
                self.capacity = true;
                Err(E::PreparationCapacity)
            }
        }
    }
    fn visit_inner(&mut self) -> Result<(), FactsSealError> {
        use super::step_job::LivingVecStorageJobV2 as Storage;
        match self.stage {
            0 => {
                if self.ordinary.ready() {
                    self.stage = 1;
                } else {
                    self.ordinary
                        .visit(self.receipt.as_ref().unwrap())
                        .map_err(LivingEventFactsErrorV2::from)?;
                }
            }
            1 => {
                let pending = self.pending.as_ref().unwrap();
                if !pending.explanations_enabled()
                    || pending.payloads().len() != self.receipt.as_ref().unwrap().events.len()
                {
                    return Err(LivingEventFactsErrorV2::Binding.into());
                }
                self.stage = 2;
            }
            2 => {
                let pending = self.pending.as_ref().unwrap();
                if let Some(payload) = pending.payloads().get(self.index) {
                    let event = &self.receipt.as_ref().unwrap().events[self.index];
                    if payload.ordinal != event.ordinal
                        || payload.kind != event.kind
                        || payload.provenance != event.provenance
                    {
                        return Err(LivingEventFactsErrorV2::Binding.into());
                    }
                    self.index += 1;
                } else {
                    self.index = 0;
                    self.stage = 3;
                }
            }
            3 => {
                if let Some((ordinal, _)) = self.pending.as_ref().unwrap().explained.get(self.index)
                {
                    if self
                        .receipt
                        .as_ref()
                        .unwrap()
                        .events
                        .get(usize::from(*ordinal))
                        .is_none()
                    {
                        return Err(LivingEventFactsErrorV2::Binding.into());
                    }
                    self.index += 1;
                } else {
                    self.high = self.index;
                    self.index = 0;
                    self.stage = 4;
                }
            }
            4 => {
                self.encoding = FactsEnvelopeCursor::new(true);
                if self.low < self.high {
                    self.middle = self.low + (self.high - self.low).div_ceil(2);
                    self.stage = 5;
                } else {
                    self.middle = self.low;
                    self.stage = 6;
                }
            }
            5 | 6 => {
                // Each trial borrows only its retained prefix; a suffix's semantic
                // or encoding fault must not be surfaced by eager preparation.
                let view = FactsEncodingView::Prefix {
                    pending: self.pending.as_ref().unwrap(),
                    receipt: self.receipt.as_ref().unwrap(),
                    retained: self.middle,
                };
                let result = self.encoding.visit(view);
                match result {
                    Ok(FactsEncodingStep::Done) => {
                        let fit = self.encoding.capacity();
                        if self.stage == 5 {
                            if fit.is_ok() {
                                self.low = self.middle;
                            } else {
                                self.high = self.middle - 1;
                            }
                            self.stage = 4;
                        } else {
                            fit?;
                            self.stage = 7;
                        }
                    }
                    Err(_) if self.stage == 5 => {
                        self.high = self.middle - 1;
                        self.stage = 4;
                    }
                    Err(error) => return Err(error.into()),
                    _ => {}
                }
            }
            7 => {
                self.events_storage = Some(Storage::begin_empty(self.low));
                self.stage = 8;
            }
            8 => {
                let storage = self.events_storage.as_mut().unwrap();
                if storage.ready() {
                    self.events = storage
                        .take_ready_result()
                        .map_err(|_| FactsSealError::Storage)?;
                    self.retirement.start(self.events_storage.take());
                    self.stage = 9;
                } else {
                    storage.poll(1).map_err(|_| FactsSealError::Storage)?;
                }
            }
            9 => {
                self.ranges_storage = Some(Storage::begin_empty(
                    self.pending.as_ref().unwrap().explained.len() - self.low,
                ));
                self.stage = 10;
            }
            10 => {
                let storage = self.ranges_storage.as_mut().unwrap();
                if storage.ready() {
                    self.ranges = storage
                        .take_ready_result()
                        .map_err(|_| FactsSealError::Storage)?;
                    self.retirement.start(self.ranges_storage.take());
                    self.index = self.low;
                    self.stage = 11;
                } else {
                    storage.poll(1).map_err(|_| FactsSealError::Storage)?;
                }
            }
            11 => {
                if let Some((ordinal, _)) = self.pending.as_ref().unwrap().explained.get(self.index)
                {
                    if let Some(last) = self.ranges.last_mut()
                        && u32::from(last[1]) + 1 == u32::from(*ordinal)
                    {
                        last[1] = *ordinal;
                    } else {
                        Storage::commit_one(&mut self.ranges, &mut Some([*ordinal, *ordinal]))
                            .map_err(|_| FactsSealError::Storage)?;
                    }
                    self.index += 1;
                } else {
                    self.stage = 12;
                }
            }
            12 => {
                self.source = Some(
                    self.pending
                        .as_mut()
                        .unwrap()
                        .take_explained_for_sealing()
                        .into_iter(),
                );
                self.index = 0;
                self.stage = 13;
            }
            13 => {
                if self.index < self.low {
                    let (ordinal, facts) = self.source.as_mut().unwrap().next().unwrap();
                    self.held = Some(LivingExplainedEventV2 {
                        ordinal,
                        event_id: self.receipt.as_ref().unwrap().events[usize::from(ordinal)].id,
                        facts,
                    });
                    self.stage = 14;
                } else {
                    // Raw iterator, including its original allocation and omitted
                    // nested owners. No slice clone, split_off or truncate.
                    self.retirement.start(self.source.take().unwrap());
                    self.stage = 15;
                }
            }
            14 => {
                Storage::commit_one(&mut self.events, &mut self.held)
                    .map_err(|_| FactsSealError::Storage)?;
                self.index += 1;
                self.stage = 13;
            }
            15 => {
                let receipt = self.receipt.as_ref().unwrap();
                self.output = Some(LivingEventFactsReceiptV2 {
                    tick: receipt.tick,
                    state_digest: receipt.state_digest,
                    receipt_digest: receipt.digest,
                    events: std::mem::take(&mut self.events),
                    omitted: if self.ranges.is_empty() {
                        None
                    } else {
                        Some(LivingEventFactsOmittedV2 {
                            reason: LivingEventFactsOmissionReasonV2::Capacity,
                            ranges: std::mem::take(&mut self.ranges),
                        })
                    },
                    integrity_sha256: LivingReceiptDigestV2([0; 32]),
                });
                self.encoding = FactsEnvelopeCursor::new(false);
                self.stage = 16;
            }
            16 => {
                if matches!(
                    self.encoding
                        .visit(FactsEncodingView::Sealed(self.output.as_ref().unwrap()))?,
                    FactsEncodingStep::Done
                ) {
                    self.encoding.capacity()?;
                    self.stage = 17;
                }
            }
            17 => {
                self.output.as_mut().unwrap().integrity_sha256 = self.encoding.digest();
                self.stage = 18;
            }
            18 => {
                if self.verification.ready() {
                    self.stage = 19;
                } else {
                    self.verification.visit(
                        self.output.as_ref().unwrap(),
                        self.receipt.as_ref().unwrap(),
                    )?;
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
    pub(crate) fn take_ready_result(
        &mut self,
    ) -> Option<(
        LivingPendingEventsV2,
        LivingTickReceiptV2,
        LivingEventFactsReceiptV2,
    )> {
        if !self.ready() {
            return None;
        }
        self.delivered = true;
        Some((
            self.pending.take().unwrap(),
            self.receipt.take().unwrap(),
            self.output.take().unwrap(),
        ))
    }
}
enum FactsSealError {
    Facts(LivingEventFactsErrorV2),
    Storage,
}
impl From<LivingEventFactsErrorV2> for FactsSealError {
    fn from(error: LivingEventFactsErrorV2) -> Self {
        Self::Facts(error)
    }
}
impl<'a> super::step_job::RetirementFields<'a> for LivingEventFactsSealJobV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.pending.creator_allocations(out);
        self.receipt.creator_allocations(out);
        self.source.creator_allocations(out);
        self.held.creator_allocations(out);
        self.events.creator_allocations(out);
        self.ranges.creator_allocations(out);
        self.events_storage.creator_allocations(out);
        self.ranges_storage.creator_allocations(out);
        self.output.creator_allocations(out);
        self.retirement.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: super::step_job::LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn super::step_job::RetirementPoll + 'a>> {
        use super::step_job::retirement_node;
        let Self {
            pending,
            receipt,
            source,
            held,
            events,
            ranges,
            events_storage,
            ranges_storage,
            output,
            retirement,
            ordinary,
            encoding,
            verification,
            stage: _,
            index: _,
            low: _,
            high: _,
            middle: _,
            fault: _,
            capacity: _,
            delivered: _,
            last_visit_visits: _,
        } = self;
        ordinary.assert_non_owning();
        encoding.assert_non_owning();
        verification.assert_non_owning();
        match index {
            0 => Some(retirement_node(pending.take(), policy)),
            1 => Some(retirement_node(receipt.take(), policy)),
            2 => Some(retirement_node(source.take(), policy)),
            3 => Some(retirement_node(held.take(), policy)),
            4 => Some(retirement_node(std::mem::take(events), policy)),
            5 => Some(retirement_node(std::mem::take(ranges), policy)),
            6 => Some(retirement_node(events_storage.take(), policy)),
            7 => Some(retirement_node(ranges_storage.take(), policy)),
            8 => Some(retirement_node(output.take(), policy)),
            9 => Some(retirement_node(std::mem::take(retirement), policy)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod bounded_facts_tests {
    use super::super::model::LivingAgreementKindV2;
    use super::*;
    fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn resources(n: usize) -> Vec<LivingEventResourceFactV2> {
        let roles = [
            LivingEventResourceRoleV2::Discarded,
            LivingEventResourceRoleV2::Input,
            LivingEventResourceRoleV2::Output,
            LivingEventResourceRoleV2::Paid,
            LivingEventResourceRoleV2::Refund,
            LivingEventResourceRoleV2::Transferred,
        ];
        (0..n)
            .map(|i| LivingEventResourceFactV2 {
                role: roles[(i / 3).min(5)],
                resource: [
                    LivingResourceV2::Alloy,
                    LivingResourceV2::Energy,
                    LivingResourceV2::Ore,
                ][i % 3],
                quantity: 10_000,
            })
            .collect()
    }
    fn blocked() -> Vec<LivingBlockedReasonV2> {
        let mut rows = vec![
            LivingBlockedReasonV2::Disabled,
            LivingBlockedReasonV2::MissingDeposit {
                required: 10_000,
                available: 9999,
            },
        ];
        for resource in [
            LivingResourceV2::Alloy,
            LivingResourceV2::Energy,
            LivingResourceV2::Ore,
        ] {
            rows.push(LivingBlockedReasonV2::MissingInput {
                resource,
                required: 1,
                available: 0,
            });
        }
        for resource in [
            LivingResourceV2::Alloy,
            LivingResourceV2::Energy,
            LivingResourceV2::Ore,
        ] {
            rows.push(LivingBlockedReasonV2::OutputFull { resource });
        }
        rows
    }
    fn agreement() -> LivingAgreementV2 {
        LivingAgreementV2 {
            kind: LivingAgreementKindV2::Trade,
            participant_a: id(1),
            participant_b: id(2),
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(1),
        }
    }
    fn delivery() -> LivingDeliveryFactV2 {
        LivingDeliveryFactV2 {
            source_world: id(0),
            destination_world: id(255),
            dispatch_owner: id(1),
            intended_receiver: id(2),
            recipient: id(2),
            resource: LivingResourceV2::Energy,
            units: 10_000,
            dispatch_kind: None,
            source_system: None,
            destination_system: None,
        }
    }
    // Independent compatibility policy copied from the pre-release implementation.
    // It is deliberately test-only; production has only the incremental policy.
    fn reference(f: &LivingEventFactsV2, kind: LivingEventKindV2) -> bool {
        if f.agreement.as_ref().is_some_and(|a| {
            kind != LivingEventKindV2::AgreementFormed
                || a.participant_a >= a.participant_b
                || a.start_tick >= a.end_tick
        }) || f.delivery.as_ref().is_some_and(|d| {
            kind != LivingEventKindV2::ShipmentDelivered
                || d.recipient != d.intended_receiver
                || d.units == 0
                || d.units > 10_000
                || (d.dispatch_kind == Some(LivingRouteKindV2::Internal)
                    && d.dispatch_owner != d.intended_receiver)
        }) {
            return false;
        }
        if f.deposit_input.as_ref().is_some_and(|d| {
            kind != LivingEventKindV2::FacilityProduced || d.units == 0 || d.units > 10_000
        }) {
            return false;
        }
        if f.resources.len() > 18
            || f.blocked_reasons.len() > 8
            || f.affected.len() > 64
            || f.resources.iter().any(|r| r.quantity > 10_000)
            || !f
                .resources
                .windows(2)
                .all(|p| (p[0].role, p[0].resource) < (p[1].role, p[1].resource))
            || !f.affected.windows(2).all(|p| p[0] < p[1])
        {
            return false;
        }
        if (kind == LivingEventKindV2::FacilityBlocked) == f.blocked_reasons.is_empty() {
            return false;
        }
        let key = |r: &LivingBlockedReasonV2| match r {
            LivingBlockedReasonV2::Disabled => (0, None),
            LivingBlockedReasonV2::MissingDeposit { .. } => (1, None),
            LivingBlockedReasonV2::MissingInput { resource, .. } => (2, Some(*resource)),
            LivingBlockedReasonV2::OutputFull { resource } => (3, Some(*resource)),
        };
        if !f
            .blocked_reasons
            .windows(2)
            .all(|p| key(&p[0]) < key(&p[1]))
        {
            return false;
        }
        !f.blocked_reasons.iter().any(|r| match r {
            LivingBlockedReasonV2::MissingDeposit {
                required,
                available,
            }
            | LivingBlockedReasonV2::MissingInput {
                required,
                available,
                ..
            } => *required > 10_000 || *available >= *required,
            _ => false,
        })
    }
    fn check(f: &LivingEventFactsV2, kind: LivingEventKindV2, visits: usize, valid: bool) {
        assert_eq!(reference(f, kind), valid);
        assert_eq!(f.validate(kind).is_ok(), valid);
        let mut c = LivingEventFactsValidationCursorV2::new(kind);
        let mut total = 0;
        loop {
            let result = c.visit(f);
            total += c.last_visit_visits();
            if result.is_err() {
                assert!(!valid);
                assert_eq!(result, Err(LivingEventFactsErrorV2::Facts));
                assert_eq!(c.visit(f), Err(LivingEventFactsErrorV2::Facts));
                assert_eq!(c.last_visit_visits(), 0);
                break;
            }
            if c.ready() {
                assert!(valid);
                assert_eq!(c.visit(f), Ok(false));
                assert_eq!(c.last_visit_visits(), 0);
                break;
            }
        }
        assert_eq!(total, visits);
    }
    #[test]
    fn validation_formula_and_all_valid_limits() {
        for r in [0_usize, 1, 18] {
            for a in [0_usize, 1, 64] {
                for b in [0_usize, 1, 8] {
                    let f = LivingEventFactsV2 {
                        resources: resources(r),
                        affected: (0..a).map(|i| id(i as u8)).collect(),
                        blocked_reasons: blocked().into_iter().take(b).collect(),
                        ..Default::default()
                    };
                    let kind = if b == 0 {
                        LivingEventKindV2::FacilityProduced
                    } else {
                        LivingEventKindV2::FacilityBlocked
                    };
                    check(
                        &f,
                        kind,
                        5 + r + r.saturating_sub(1) + a.saturating_sub(1) + b.saturating_sub(1) + b,
                        true,
                    );
                }
            }
        }
        let mut f = LivingEventFactsV2 {
            agreement: Some(agreement()),
            ..Default::default()
        };
        check(&f, LivingEventKindV2::AgreementFormed, 5, true);
        f.agreement = None;
        f.delivery = Some(delivery());
        check(&f, LivingEventKindV2::ShipmentDelivered, 5, true);
        let d = f.delivery.as_mut().unwrap();
        d.dispatch_kind = Some(LivingRouteKindV2::Internal);
        d.dispatch_owner = d.intended_receiver;
        check(&f, LivingEventKindV2::ShipmentDelivered, 5, true);
        f.delivery = None;
        f.deposit_input = Some(LivingDepositInputFactV2 {
            world: id(1),
            units: 1,
        });
        check(&f, LivingEventKindV2::FacilityProduced, 5, true);
        f.deposit_input.as_mut().unwrap().units = 10_000;
        check(&f, LivingEventKindV2::FacilityProduced, 5, true);
        f.resources = resources(1);
        f.resources[0].quantity = 0;
        check(&f, LivingEventKindV2::FacilityProduced, 6, true);
    }
    #[test]
    fn validator_constraints_and_fault_prefix_precedence() {
        for malformed in 0..5 {
            let mut a = agreement();
            let kind = if malformed == 0 {
                LivingEventKindV2::FacilityProduced
            } else {
                LivingEventKindV2::AgreementFormed
            };
            match malformed {
                1 => a.participant_b = a.participant_a,
                2 => a.participant_a = id(3),
                3 => a.end_tick = a.start_tick,
                4 => a.start_tick = LivingTickV2(2),
                _ => (),
            }
            let f = LivingEventFactsV2 {
                agreement: Some(a),
                delivery: Some(delivery()),
                resources: resources(19),
                ..Default::default()
            };
            check(&f, kind, 1, false);
        }
        for malformed in 0..6 {
            let mut d = delivery();
            let kind = if malformed == 0 {
                LivingEventKindV2::FacilityProduced
            } else {
                LivingEventKindV2::ShipmentDelivered
            };
            match malformed {
                1 => d.recipient = id(3),
                2 => d.units = 0,
                3 => d.units = 10_001,
                4 => d.dispatch_kind = Some(LivingRouteKindV2::Internal),
                5 => {
                    d.dispatch_kind = Some(LivingRouteKindV2::Internal);
                    d.dispatch_owner = id(0);
                }
                _ => (),
            }
            check(
                &LivingEventFactsV2 {
                    delivery: Some(d),
                    resources: resources(19),
                    ..Default::default()
                },
                kind,
                2,
                false,
            );
        }
        for units in [0, 10_001, 1] {
            let kind = if units == 1 {
                LivingEventKindV2::ShipmentDelivered
            } else {
                LivingEventKindV2::FacilityProduced
            };
            check(
                &LivingEventFactsV2 {
                    deposit_input: Some(LivingDepositInputFactV2 {
                        world: id(1),
                        units,
                    }),
                    resources: resources(19),
                    ..Default::default()
                },
                kind,
                3,
                false,
            );
        }
        for (r, b, a) in [(19, 0, 0), (0, 9, 0), (0, 0, 65)] {
            check(
                &LivingEventFactsV2 {
                    resources: resources(r),
                    blocked_reasons: vec![LivingBlockedReasonV2::Disabled; b],
                    affected: vec![id(0); a],
                    ..Default::default()
                },
                LivingEventKindV2::FacilityProduced,
                4,
                false,
            );
        }
        let mut f = LivingEventFactsV2 {
            resources: resources(2),
            affected: vec![id(1), id(1)],
            ..Default::default()
        };
        f.resources[1].quantity = 10_001;
        check(&f, LivingEventKindV2::FacilityProduced, 6, false);
        f.resources[1].quantity = 1;
        f.resources[1] = f.resources[0].clone();
        check(&f, LivingEventKindV2::FacilityProduced, 7, false);
        f.resources.swap(0, 1);
        f.resources[0].resource = LivingResourceV2::Ore;
        check(&f, LivingEventKindV2::FacilityProduced, 7, false);
        f.resources.clear();
        check(&f, LivingEventKindV2::FacilityProduced, 5, false);
        f.affected = vec![id(2), id(1)];
        check(&f, LivingEventKindV2::FacilityProduced, 5, false);
        check(&f, LivingEventKindV2::FacilityBlocked, 5, false);
        f.affected.clear();
        check(&f, LivingEventKindV2::FacilityBlocked, 5, false);
        f.blocked_reasons = vec![LivingBlockedReasonV2::Disabled];
        check(&f, LivingEventKindV2::FacilityProduced, 5, false);
        f.blocked_reasons.push(LivingBlockedReasonV2::Disabled);
        check(&f, LivingEventKindV2::FacilityBlocked, 6, false);
        f.blocked_reasons = vec![
            LivingBlockedReasonV2::MissingDeposit {
                required: 0,
                available: 0
            };
            2
        ];
        check(&f, LivingEventKindV2::FacilityBlocked, 6, false);
        for input in [false, true] {
            for (required, available) in [(0, 0), (10_001, 0), (1, 1), (1, 2)] {
                let bad = if input {
                    LivingBlockedReasonV2::MissingInput {
                        resource: LivingResourceV2::Alloy,
                        required,
                        available,
                    }
                } else {
                    LivingBlockedReasonV2::MissingDeposit {
                        required,
                        available,
                    }
                };
                f.blocked_reasons = vec![LivingBlockedReasonV2::Disabled, bad];
                check(&f, LivingEventKindV2::FacilityBlocked, 8, false);
            }
        }
    }
    fn expected_tokens(f: &LivingEventFactsV2) -> usize {
        fn array(costs: impl Iterator<Item = usize>) -> usize {
            let rows: Vec<_> = costs.collect();
            if rows.is_empty() {
                2
            } else {
                rows.len() + 1 + rows.iter().sum::<usize>()
            }
        }
        let q = usize::from(f.deposit_input.is_some())
            + usize::from(f.agreement.is_some())
            + usize::from(f.delivery.is_some());
        12 + 18
            + 3 * q
            + array(f.resources.iter().map(|_| 13))
            + array(f.blocked_reasons.iter().map(|r| match r {
                LivingBlockedReasonV2::Disabled => 5,
                LivingBlockedReasonV2::MissingDeposit { .. } => 13,
                LivingBlockedReasonV2::MissingInput { .. } => 17,
                LivingBlockedReasonV2::OutputFull { .. } => 9,
            }))
            + array(f.affected.iter().map(|_| 1))
            + 9 * usize::from(f.deposit_input.is_some())
            + 21 * usize::from(f.agreement.is_some())
            + 41 * usize::from(f.delivery.is_some())
    }
    fn encode_reference(f: &LivingEventFactsV2, ordinal: u16, event_id: LivingEventIdV2) {
        let reference = serde_json::to_vec(&LivingExplainedEventV2 {
            ordinal,
            event_id,
            facts: f.clone(),
        })
        .unwrap();
        let mut c = LivingExplainedEntryEncodingCursorV2::new(ordinal, event_id);
        let mut bytes = Vec::new();
        let mut tokens = 0;
        while let Some(token) = c.next_fragment(f).unwrap() {
            assert!(!token.as_bytes().is_empty());
            assert!(token.as_bytes().len() <= 256);
            assert_eq!(c.last_visit_visits(), 1);
            tokens += 1;
            bytes.extend_from_slice(token.as_bytes());
        }
        assert_eq!(bytes, reference);
        assert_eq!(tokens, expected_tokens(f));
        assert_eq!(c.last_visit_visits(), 0);
        assert!(c.next_fragment(f).unwrap().is_none());
        assert_eq!(c.last_visit_visits(), 0);
        // Compose each existing schema fixture through the complete envelope.
        // Entry bytes above are independently Serde-produced; framing and token
        // arithmetic here are handwritten and never obtained from the cursor.
        let receipt = LivingEventFactsReceiptV2 {
            tick: LivingTickV2(1),
            state_digest: LivingStateDigestV2([2; 32]),
            receipt_digest: LivingReceiptDigestV2([3; 32]),
            events: vec![LivingExplainedEventV2 {
                ordinal,
                event_id,
                facts: f.clone(),
            }],
            omitted: None,
            integrity_sha256: LivingReceiptDigestV2([0; 32]),
        };
        for envelope in [false, true] {
            let mut expected = format!(
                "{{\"tick\":1,\"state_digest\":\"{}\",\"receipt_digest\":\"{}\",\"events\":[",
                "02".repeat(32),
                "03".repeat(32)
            )
            .into_bytes();
            expected.extend_from_slice(&reference);
            expected.extend_from_slice(b"]");
            if envelope {
                expected.extend_from_slice(
                    format!(",\"integrity_sha256\":\"{}\"", "00".repeat(32)).as_bytes(),
                );
            }
            expected.push(b'}');
            let mut cursor = FactsEnvelopeCursor::new(envelope);
            let mut actual = Vec::new();
            loop {
                match cursor.visit(FactsEncodingView::Sealed(&receipt)).unwrap() {
                    FactsEncodingStep::Fragment(fragment) => {
                        actual.extend_from_slice(fragment.as_bytes())
                    }
                    FactsEncodingStep::Scan => {}
                    FactsEncodingStep::Done => break,
                }
            }
            assert_eq!(actual, expected);
            assert_eq!(
                cursor.tokens,
                expected_tokens(f) + if envelope { 22 } else { 18 }
            );
            assert!(cursor.max_fragment <= 256);
            let mut hash = Sha256::new();
            hash.update(LIVING_EVENT_FACTS_DOMAIN_V2);
            hash.update(&expected);
            assert_eq!(
                cursor.digest(),
                LivingReceiptDigestV2(hash.finalize().into())
            );
        }
    }
    #[test]
    fn fixed_encoder_serde_bytes_all_schema_scalars_and_sparse_combinations() {
        for ordinal in [0, u16::MAX] {
            for event_id in [LivingEventIdV2([0; 16]), LivingEventIdV2([255; 16])] {
                encode_reference(&LivingEventFactsV2::default(), ordinal, event_id);
                for reason in blocked() {
                    encode_reference(
                        &LivingEventFactsV2 {
                            blocked_reasons: vec![reason],
                            ..Default::default()
                        },
                        ordinal,
                        event_id,
                    );
                }
                for mask in 0..8 {
                    for actors in 0..4 {
                        let mut f = LivingEventFactsV2 {
                            resources: resources(18),
                            blocked_reasons: blocked(),
                            affected: vec![id(0), id(255)],
                            actor: (actors & 1 != 0).then_some(id(0)),
                            entity: (actors & 2 != 0).then_some(id(255)),
                            deposit_input: (mask & 1 != 0).then_some(LivingDepositInputFactV2 {
                                world: id(255),
                                units: u32::MAX,
                            }),
                            agreement: (mask & 2 != 0).then_some(LivingAgreementV2 {
                                start_tick: LivingTickV2(0),
                                end_tick: LivingTickV2(u64::MAX),
                                ..agreement()
                            }),
                            delivery: (mask & 4 != 0).then_some(delivery()),
                        };
                        for (i, r) in f.resources.iter_mut().enumerate() {
                            r.quantity = if i % 2 == 0 { 0 } else { u32::MAX };
                        }
                        encode_reference(&f, ordinal, event_id);
                    }
                }
            }
        }
        for kind in [
            LivingAgreementKindV2::Nonaggression,
            LivingAgreementKindV2::Trade,
            LivingAgreementKindV2::Truce,
        ] {
            encode_reference(
                &LivingEventFactsV2 {
                    agreement: Some(LivingAgreementV2 {
                        kind,
                        ..agreement()
                    }),
                    ..Default::default()
                },
                0,
                LivingEventIdV2([0; 16]),
            );
        }
        for kind in [
            None,
            Some(LivingRouteKindV2::Aid),
            Some(LivingRouteKindV2::Internal),
            Some(LivingRouteKindV2::Trade),
        ] {
            for mask in 0..4 {
                encode_reference(
                    &LivingEventFactsV2 {
                        delivery: Some(LivingDeliveryFactV2 {
                            units: u32::MAX,
                            dispatch_kind: kind,
                            source_system: (mask & 1 != 0).then_some(id(255)),
                            destination_system: (mask & 2 != 0).then_some(id(0)),
                            ..delivery()
                        }),
                        ..Default::default()
                    },
                    u16::MAX,
                    LivingEventIdV2([255; 16]),
                );
            }
        }
        for reason in [
            LivingBlockedReasonV2::MissingDeposit {
                required: 0,
                available: u32::MAX,
            },
            LivingBlockedReasonV2::MissingInput {
                resource: LivingResourceV2::Ore,
                required: u32::MAX,
                available: 0,
            },
        ] {
            encode_reference(
                &LivingEventFactsV2 {
                    blocked_reasons: vec![reason],
                    ..Default::default()
                },
                0,
                LivingEventIdV2([0; 16]),
            );
        }
        for units in [0, u32::MAX] {
            encode_reference(
                &LivingEventFactsV2 {
                    deposit_input: Some(LivingDepositInputFactV2 {
                        world: id(0),
                        units,
                    }),
                    delivery: Some(LivingDeliveryFactV2 {
                        units,
                        ..delivery()
                    }),
                    ..Default::default()
                },
                0,
                LivingEventIdV2([0; 16]),
            );
        }
    }
    #[test]
    fn encoder_retained_failure_and_initial_fixed_buffer() {
        let f = LivingEventFactsV2::default();
        let mut c = LivingExplainedEntryEncodingCursorV2::new(0, LivingEventIdV2([0; 16]));
        c.reject_next_fragment();
        assert!(matches!(c.next_fragment(&f), Err(LivingWireErrorV2::Json)));
        assert_eq!(c.last_visit_visits(), 1);
        assert!(matches!(c.next_fragment(&f), Err(LivingWireErrorV2::Json)));
        assert_eq!(c.last_visit_visits(), 0);
        let mut fragment = CanonicalFragment::new();
        use std::io::Write;
        assert!(fragment.write_all(&[0; 257]).is_err());
        assert_eq!(fragment.len, 0);
        fragment.write_all(&[0; 256]).unwrap();
        assert!(fragment.write_all(&[0]).is_err());
        assert_eq!(fragment.len, 256);
    }
}

#[cfg(test)]
mod facts_sealing_tests {
    use super::super::model::LivingAgreementKindV2;
    use super::super::receipt::{LivingEventProvenanceV2, seal_living_tick_receipt_v2};
    use super::super::step_job::{
        LivingLifecycleJobErrorV2, LivingLifecycleProgressV2, LivingPhaseRetirementJobV2,
        LivingStateRetirementPolicyV2, RetirementFields,
    };
    use super::*;

    fn pair(n: usize, tick: u64, actors: usize) -> (LivingPendingEventsV2, LivingTickReceiptV2) {
        let mut pending = LivingPendingEventsV2::new();
        for i in 0..n {
            pending
                .record_explained(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::RunProduction,
                    },
                    LivingEventKindV2::FacilityProduced,
                    LivingEventFactsV2 {
                        actor: (i < actors).then_some(LivingEntityIdV2([1; 16])),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        let receipt = seal_living_tick_receipt_v2(
            LivingTickV2(tick),
            vec![],
            &pending,
            LivingStateDigestV2([2; 32]),
        )
        .unwrap();
        (pending, receipt)
    }
    fn run(job: &mut LivingEventFactsSealJobV2) -> Result<(), LivingLifecycleJobErrorV2> {
        for _ in 0..100_000_000 {
            if job.ready() {
                return Ok(());
            }
            job.visit()?;
        }
        panic!("facts visit bound");
    }
    fn encode(view: FactsEncodingView<'_>, envelope: bool) -> (Vec<u8>, FactsEnvelopeCursor) {
        let mut cursor = FactsEnvelopeCursor::new(envelope);
        let mut bytes = Vec::new();
        loop {
            match cursor.visit(view).unwrap() {
                FactsEncodingStep::Fragment(fragment) => {
                    assert!((1..=256).contains(&fragment.len));
                    bytes.extend_from_slice(fragment.as_bytes());
                }
                FactsEncodingStep::Scan => {}
                FactsEncodingStep::Done => break,
            }
        }
        assert_eq!(bytes.len(), cursor.bytes);
        (bytes, cursor)
    }
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
    // Handwritten schema, deliberately independent of Serde and all cursors.
    fn literal_default(
        r: &LivingTickReceiptV2,
        retained: usize,
        actors: usize,
        omitted: Option<[u16; 2]>,
        integrity: Option<LivingReceiptDigestV2>,
    ) -> Vec<u8> {
        let mut text = format!(
            "{{\"tick\":{},\"state_digest\":\"{}\",\"receipt_digest\":\"{}\",\"events\":[",
            r.tick.0,
            hex(&r.state_digest.0),
            hex(&r.digest.0)
        );
        for (i, event) in r.events.iter().take(retained).enumerate() {
            if i != 0 {
                text.push(',');
            }
            let actor = if i < actors {
                format!("\"{}\"", "01".repeat(16))
            } else {
                "null".to_owned()
            };
            text.push_str(&format!("{{\"ordinal\":{i},\"event_id\":\"{}\",\"facts\":{{\"actor\":{actor},\"entity\":null,\"resources\":[],\"blocked_reasons\":[],\"affected\":[]}}}}", hex(&event.id.0)));
        }
        text.push(']');
        if let Some([start, end]) = omitted {
            text.push_str(&format!(
                ",\"omitted\":{{\"reason\":\"capacity\",\"ranges\":[[{start},{end}]]}}"
            ));
        }
        if let Some(digest) = integrity {
            text.push_str(&format!(",\"integrity_sha256\":\"{}\"", hex(&digest.0)));
        }
        text.push('}');
        text.into_bytes()
    }
    #[test]
    fn facts_tokens_match_literal_full_envelope_and_original_reference() {
        for n in [0, 1, 2, 10, 257] {
            let (pending, receipt) = pair(n, 1, 0);
            let original = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
            let (bytes, cursor) = encode(FactsEncodingView::Sealed(&original), true);
            assert_eq!(
                bytes,
                literal_default(&receipt, n, 0, None, Some(original.integrity_sha256))
            );
            assert_eq!(bytes, original.canonical_bytes().unwrap());
            assert_eq!(cursor.tokens, if n == 0 { 22 } else { 21 + 37 * n });
            let (payload, payload_cursor) = encode(FactsEncodingView::Sealed(&original), false);
            assert_eq!(payload, literal_default(&receipt, n, 0, None, None));
            assert_eq!(payload_cursor.digest(), original.integrity_sha256);
            if n == 0 {
                assert_eq!(bytes.len(), 274);
                assert_eq!(payload.len(), 188);
            }
            assert!(cursor.max_fragment <= 256);
        }
        let (pending, receipt) = pair(2, 1, 0);
        // Only ordinal1 is explained; its omission is exactly [1,1].
        let mut pending = pending;
        pending.explained.remove(0);
        let (bytes, cursor) = encode(
            FactsEncodingView::Prefix {
                pending: &pending,
                receipt: &receipt,
                retained: 0,
            },
            true,
        );
        assert_eq!(
            bytes,
            literal_default(
                &receipt,
                0,
                0,
                Some([1, 1]),
                Some(LivingReceiptDigestV2([0; 32]))
            )
        );
        assert_eq!(bytes.len(), 323);
        assert_eq!(cursor.tokens, 40);
    }
    #[test]
    fn facts_prefix_uses_full_integrity_envelope_and_upper_mid_search() {
        const N: usize = 13_959;
        for (tick, actors, all_bytes, retained) in [
            (1, 5, 2_097_122, N),
            (1, 6, 2_097_152, N),
            (10, 6, 2_097_153, N - 1),
        ] {
            let (pending, receipt) = pair(N, tick, actors);
            let independent = literal_default(
                &receipt,
                N,
                actors,
                None,
                Some(LivingReceiptDigestV2([0; 32])),
            );
            assert_eq!(independent.len(), all_bytes);
            let (full, _) = encode(
                FactsEncodingView::Prefix {
                    pending: &pending,
                    receipt: &receipt,
                    retained: N,
                },
                true,
            );
            assert_eq!(full, independent);
            let expected = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
            assert_eq!(expected.events.len(), retained);
            let mut expected_mids = Vec::new();
            let (mut low, mut high) = (0, N);
            while low < high {
                let middle = low + (high - low).div_ceil(2);
                expected_mids.push(middle);
                let omitted = (middle < N).then_some([middle as u16, (N - 1) as u16]);
                if literal_default(
                    &receipt,
                    middle,
                    actors,
                    omitted,
                    Some(LivingReceiptDigestV2([0; 32])),
                )
                .len()
                    <= 2_097_152
                {
                    low = middle;
                } else {
                    high = middle - 1;
                }
            }
            let mut job = LivingEventFactsSealJobV2::begin(pending, receipt);
            let mut mids = Vec::new();
            while !job.ready() {
                if job.stage == 5 && job.encoding.stage == 0 {
                    mids.push(job.middle);
                }
                job.visit().unwrap();
            }
            assert_eq!(mids, expected_mids);
            let (_, receipt, actual) = job.take_ready_result().unwrap();
            assert_eq!(actual, expected);
            assert_eq!(
                actual.canonical_bytes().unwrap(),
                literal_default(
                    &receipt,
                    retained,
                    actors,
                    (retained < N).then_some([13958, 13958]),
                    Some(actual.integrity_sha256)
                )
            );
            assert_eq!(
                actual.canonical_bytes().unwrap().len(),
                if retained == N {
                    all_bytes
                } else {
                    2_097_152 - 93
                }
            );
        }
    }
    fn compare(
        pending: LivingPendingEventsV2,
        receipt: LivingTickReceiptV2,
    ) -> Result<LivingEventFactsReceiptV2, LivingEventFactsErrorV2> {
        let expected = seal_living_event_facts_receipt_v2(&receipt, &pending);
        let mut job = LivingEventFactsSealJobV2::begin(pending, receipt);
        let actual = match run(&mut job) {
            Ok(()) => Ok(job.take_ready_result().unwrap().2),
            Err(LivingLifecycleJobErrorV2::Fault(
                super::super::simulation::LivingDeterministicFaultV2::EventFacts(error),
            )) => {
                assert_eq!(job.last_visit_visits, 1);
                assert!(job.visit().is_err());
                assert_eq!(job.last_visit_visits, 0);
                Err(error)
            }
            other => panic!("unexpected result {other:?}"),
        };
        assert_eq!(actual, expected);
        actual
    }
    #[test]
    fn facts_omitted_payload_is_not_semantically_validated() {
        let (mut pending, receipt) = pair(2, 1, 0);
        // Injected malformed child input: too many affected IDs. Public append
        // correctly rejects it; sealing must still omit its oversized suffix.
        pending.explained[1].1.affected = vec![LivingEntityIdV2([1; 16]); 65_536];
        let actual = compare(pending.clone(), receipt.clone()).unwrap();
        assert_eq!(actual.events.len(), 1);
        assert_eq!(actual.omitted.unwrap().ranges, [[1, 1]]);
        pending.explained[1].1.affected.truncate(65);
        assert_eq!(
            compare(pending.clone(), receipt.clone()),
            Err(LivingEventFactsErrorV2::Facts)
        );
        pending.explained[1].0 = 9;
        assert_eq!(
            compare(pending, receipt),
            Err(LivingEventFactsErrorV2::Binding)
        );
    }
    #[test]
    fn facts_ordinary_verifier_preserves_wrapped_fault_precedence() {
        use super::super::receipt::LivingReceiptErrorV2 as R;
        let (pending, receipt) = pair(2, 1, 0);
        let mut bad = receipt.clone();
        bad.events[0].ordinal = 9;
        assert_eq!(
            compare(pending.clone(), bad.clone()),
            Err(LivingEventFactsErrorV2::Receipt(R::DigestMismatch))
        );
        bad.digest = bad.payload().digest().unwrap();
        assert_eq!(
            compare(pending.clone(), bad.clone()),
            Err(LivingEventFactsErrorV2::Receipt(R::EventIdentityMismatch))
        );
        bad.applied_revisions = vec![LivingRevisionIdV2([1; 32]); 2];
        assert_eq!(
            compare(pending.clone(), bad),
            Err(LivingEventFactsErrorV2::Receipt(R::UnsortedRevisions))
        );
        let mut changed = pending.clone();
        changed.explained.reverse();
        assert_eq!(
            compare(changed, receipt.clone()),
            Err(LivingEventFactsErrorV2::Binding)
        );
        let mut changed = pending;
        changed.explained[1].0 = 0;
        assert_eq!(
            compare(changed, receipt),
            Err(LivingEventFactsErrorV2::Binding)
        );
    }
    #[test]
    fn facts_final_verification_preserves_capacity_binding_and_integrity_order() {
        let (pending, receipt) = pair(3, 1, 0);
        let original = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
        for variant in 0..9 {
            let mut facts = original.clone();
            let mut ordinary = receipt.clone();
            match variant {
                0 => {
                    facts.events[0].facts.affected = vec![LivingEntityIdV2([1; 16]); 65_536];
                    ordinary.digest.0[0] ^= 1;
                }
                1 => {
                    ordinary.digest.0[0] ^= 1;
                    facts.tick.0 += 1;
                }
                2 => {
                    facts.events[0].facts.affected = vec![LivingEntityIdV2([1; 16]); 65];
                    facts.events.swap(1, 2);
                }
                3 => {
                    facts.events[0].event_id.0[0] ^= 1;
                    facts.events[0].facts.affected = vec![LivingEntityIdV2([1; 16]); 65];
                }
                4 => {
                    facts.events[0].facts.affected = vec![LivingEntityIdV2([1; 16]); 65];
                }
                5 => {
                    facts.omitted = Some(LivingEventFactsOmittedV2 {
                        reason: LivingEventFactsOmissionReasonV2::Capacity,
                        ranges: vec![],
                    });
                }
                6 => {
                    facts.events.clear();
                    facts.omitted = Some(LivingEventFactsOmittedV2 {
                        reason: LivingEventFactsOmissionReasonV2::Capacity,
                        ranges: vec![[0, 0], [1, 9]],
                    });
                }
                7 => {
                    facts.events.clear();
                    facts.omitted = Some(LivingEventFactsOmittedV2 {
                        reason: LivingEventFactsOmissionReasonV2::Capacity,
                        ranges: vec![[0, 0], [1, 1]],
                    });
                }
                _ => {
                    facts.integrity_sha256.0[0] ^= 1;
                }
            }
            let expected = facts.verify_receipt(&ordinary);
            let mut cursor = FactsPairVerificationCursor::new();
            let actual = loop {
                match cursor.visit(&facts, &ordinary) {
                    Err(error) => break Err(error),
                    Ok(false) => break Ok(()),
                    _ => {}
                }
            };
            assert_eq!(actual, expected, "variant{variant}");
        }
    }
    fn retire(job: LivingEventFactsSealJobV2, budget: usize) -> u64 {
        retire_schedule(job, &[budget])
    }
    fn retire_schedule(job: LivingEventFactsSealJobV2, schedule: &[usize]) -> u64 {
        let mut retirement =
            LivingPhaseRetirementJobV2::begin(job, LivingStateRetirementPolicyV2::ReleaseHandle);
        let mut poll = 0;
        loop {
            let budget = schedule[poll % schedule.len()];
            poll += 1;
            let status = retirement.poll(budget).unwrap();
            assert!(retirement.last_poll_visits() <= budget);
            if status == LivingLifecycleProgressV2::Ready {
                break;
            }
        }
        let total = retirement.total_visits();
        retirement.poll(1).unwrap();
        assert_eq!(retirement.last_poll_visits(), 0);
        total
    }
    // Independent graph arithmetic from facts-sealing-fix1-owner-worksheet.md.
    // Read-only partition lengths instantiate formulas; measured retirement
    // counters are never used to choose an expected value.
    fn opt<T>(owner: &Option<T>, cost: impl FnOnce(&T) -> u64) -> u64 {
        owner.as_ref().map_or(1, |owner| 3 + cost(owner))
    }
    fn fact_cost(facts: &LivingEventFactsV2) -> u64 {
        10 + 3 * (facts.resources.len() + facts.blocked_reasons.len() + facts.affected.len()) as u64
    }
    fn source_cost(source: &std::vec::IntoIter<(u16, LivingEventFactsV2)>) -> u64 {
        1 + source
            .as_slice()
            .iter()
            .map(|(_, f)| 14 + fact_cost(f))
            .sum::<u64>()
    }
    fn storage_cost<T: RetirementFields<'static>>(
        storage: &super::super::step_job::LivingVecStorageJobV2<'static, T>,
    ) -> u64 {
        let partitions = storage.production_partitions();
        assert!(
            partitions.original.is_none()
                && partitions.source.is_none()
                && partitions.pending.is_none()
        );
        if let Some(destination) = partitions.destination {
            assert!(destination.is_empty());
            25
        } else {
            22
        }
    }
    fn job_cost(job: &LivingEventFactsSealJobV2, lane_remaining: Option<u64>) -> u64 {
        assert_eq!(job.retirement.is_empty(), lane_remaining.is_none());
        21 + opt(&job.pending, |pending| {
            assert!(
                pending.decisions.is_empty()
                    && pending.creator_recoveries.is_empty()
                    && pending.corridor_restrictions.is_empty()
            );
            16 + 3 * pending.payloads().len() as u64
                + pending
                    .explained
                    .iter()
                    .map(|(_, f)| 14 + fact_cost(f))
                    .sum::<u64>()
        }) + opt(&job.receipt, |r| {
            7 + 3 * (r.applied_revisions.len() + r.events.len()) as u64
        }) + opt(&job.source, source_cost)
            + opt(&job.held, |e| fact_cost(&e.facts))
            + 1
            + job
                .events
                .iter()
                .map(|e| 2 + fact_cost(&e.facts))
                .sum::<u64>()
            + 1
            + 3 * job.ranges.len() as u64
            + opt(&job.events_storage, storage_cost)
            + opt(&job.ranges_storage, storage_cost)
            + opt(&job.output, |r| {
                let rows = r
                    .events
                    .iter()
                    .map(|e| 2 + fact_cost(&e.facts))
                    .sum::<u64>();
                r.omitted
                    .as_ref()
                    .map_or(7 + rows, |o| 13 + rows + 3 * o.ranges.len() as u64)
            })
            + lane_remaining.map_or(7, |remaining| 6 + remaining)
    }
    fn advance_counted(job: &mut LivingEventFactsSealJobV2, lane_remaining: &mut Option<u64>) {
        let active = !job.retirement.is_empty();
        let admitted = if active {
            None
        } else if (job.stage == 8 && job.events_storage.as_ref().unwrap().ready())
            || (job.stage == 10 && job.ranges_storage.as_ref().unwrap().ready())
        {
            Some(25)
        } else if job.stage == 13 && job.index == job.low {
            Some(source_cost(job.source.as_ref().unwrap()))
        } else {
            None
        };
        assert!(job.visit().unwrap());
        assert_eq!(job.last_visit_visits, 1);
        if active {
            *lane_remaining = match lane_remaining.unwrap() {
                0 => None,
                n => Some(n - 1),
            };
        } else if admitted.is_some() {
            *lane_remaining = admitted;
        }
        assert_eq!(job.retirement.is_empty(), lane_remaining.is_none());
    }
    #[test]
    fn facts_original_owner_graph_has_predeclared_literal_counts() {
        for budget in [1, 64, 1024] {
            let (pending, receipt) = pair(0, 1, 0);
            assert_eq!(
                retire(LivingEventFactsSealJobV2::begin(pending, receipt), budget),
                64
            );
            let (mut pending, receipt) = pair(0, 1, 0);
            pending.explained.push((
                0,
                LivingEventFactsV2 {
                    blocked_reasons: vec![LivingBlockedReasonV2::Disabled; 3],
                    affected: vec![LivingEntityIdV2([1; 16]); 2],
                    ..Default::default()
                },
            ));
            assert_eq!(
                retire(LivingEventFactsSealJobV2::begin(pending, receipt), budget),
                103
            );
            let (pending, receipt) = pair(0, 1, 0);
            let mut job = LivingEventFactsSealJobV2::begin(pending, receipt);
            run(&mut job).unwrap();
            assert!(!job.visit().unwrap());
            assert_eq!(job.last_visit_visits, 0);
            let _output = job.take_ready_result().unwrap();
            assert!(job.take_ready_result().is_none());
            assert_eq!(retire(job, budget), 37);
        }
    }
    #[test]
    fn facts_original_nested_owners_survive_every_reached_cut() {
        let (mut pending, _) = pair(2, 1, 0);
        for (ordinal, facts) in &mut pending.explained {
            facts.affected = vec![LivingEntityIdV2([*ordinal as u8 + 1; 16])];
        }
        let receipt = seal_living_tick_receipt_v2(
            LivingTickV2(1),
            vec![],
            &pending,
            LivingStateDigestV2([2; 32]),
        )
        .unwrap();
        let expected = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
        let mut probe = LivingEventFactsSealJobV2::begin(pending.clone(), receipt.clone());
        let mut total = 0;
        while !probe.ready() {
            probe.visit().unwrap();
            total += 1;
        }
        assert!(total < 10_000);
        for cut in 0..=total {
            let source = pending.clone();
            let pointers: Vec<_> = source
                .explained
                .iter()
                .map(|(_, f)| f.affected.as_ptr() as usize)
                .collect();
            let ordinary_ids = receipt.clone();
            let event_pointer = ordinary_ids.events.as_ptr() as usize;
            let mut job = LivingEventFactsSealJobV2::begin(source, ordinary_ids);
            let mut lane_remaining = None;
            for _ in 0..cut {
                advance_counted(&mut job, &mut lane_remaining);
            }
            let mut allocations = Vec::new();
            job.creator_allocations(&mut allocations);
            for pointer in &pointers {
                assert_eq!(
                    allocations
                        .iter()
                        .filter(|row| row.0 == *pointer && row.1 == 1)
                        .count(),
                    1,
                    "cut{cut}"
                );
            }
            assert_eq!(
                allocations
                    .iter()
                    .filter(|row| row.0 == event_pointer && row.1 == 2)
                    .count(),
                1
            );
            if job.ready() {
                let (_, _, actual) = job.take_ready_result().unwrap();
                assert_eq!(actual, expected);
                for (i, entry) in actual.events.iter().enumerate() {
                    assert_eq!(entry.facts.affected.as_ptr() as usize, pointers[i]);
                }
            } else {
                assert!(job.take_ready_result().is_none());
            }
            let expected_remaining = job_cost(&job, lane_remaining);
            let schedules: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 64, 2, 1024, 7]];
            assert_eq!(
                retire_schedule(job, schedules[cut % schedules.len()]),
                expected_remaining,
                "cut {cut}"
            );
        }
    }
    #[test]
    fn facts_storage_capacity_is_local_sticky_and_retains_originals() {
        let (pending, receipt) = pair(1, 1, 0);
        let pointer = pending.explained.as_ptr();
        let mut job = LivingEventFactsSealJobV2::begin(pending, receipt);
        while job.stage != 8 {
            job.visit().unwrap();
        }
        job.events_storage = Some(super::super::step_job::LivingVecStorageJobV2::begin_empty(
            usize::MAX,
        ));
        assert_eq!(
            job.visit(),
            Err(LivingLifecycleJobErrorV2::PreparationCapacity)
        );
        assert_eq!(job.last_visit_visits, 1);
        assert!(job.fault.is_none());
        assert_eq!(job.pending.as_ref().unwrap().explained.as_ptr(), pointer);
        assert_eq!(
            job.visit(),
            Err(LivingLifecycleJobErrorV2::PreparationCapacity)
        );
        assert_eq!(job.last_visit_visits, 0);
        assert!(job.take_ready_result().is_none());
        retire(job, 64);
    }
    fn literal_sparse(r: &LivingTickReceiptV2, ordinals: &[u16], ranges: &[[u16; 2]]) -> Vec<u8> {
        let mut text = format!(
            "{{\"tick\":{},\"state_digest\":\"{}\",\"receipt_digest\":\"{}\",\"events\":[",
            r.tick.0,
            hex(&r.state_digest.0),
            hex(&r.digest.0)
        );
        for (index, ordinal) in ordinals.iter().enumerate() {
            if index != 0 {
                text.push(',');
            }
            text.push_str(&format!("{{\"ordinal\":{ordinal},\"event_id\":\"{}\",\"facts\":{{\"actor\":null,\"entity\":null,\"resources\":[],\"blocked_reasons\":[],\"affected\":[]}}}}",hex(&r.events[usize::from(*ordinal)].id.0)));
        }
        text.push(']');
        if !ranges.is_empty() {
            text.push_str(",\"omitted\":{\"reason\":\"capacity\",\"ranges\":[");
            for (index, [start, end]) in ranges.iter().enumerate() {
                if index != 0 {
                    text.push(',');
                }
                text.push_str(&format!("[{start},{end}]"));
            }
            text.push_str("]}");
        }
        text.push_str(&format!(",\"integrity_sha256\":\"{}\"}}", "00".repeat(32)));
        text.into_bytes()
    }
    #[test]
    fn facts_sparse_suffix_search_preserves_exact_ranges_and_zero_integrity() {
        type SuffixRangeCase<'a> = (&'a [u16], usize, &'a [[u16; 2]], bool);
        let cases: &[SuffixRangeCase<'_>] = &[
            (&[0, 1, 2, 5, 7, 8], 1, &[[1, 2], [5, 5], [7, 8]], true),
            (&[9, 10, 11], 0, &[[9, 11]], true),
            (&[65534, 65535], 0, &[[65534, 65535]], true),
            (&[1, 1, 2], 0, &[[1, 1], [1, 2]], false),
            (&[5, 3, 4], 0, &[[5, 5], [3, 4]], false),
            (&[0, 1, 3, 2], 1, &[[1, 1], [3, 3], [2, 2]], false),
        ];
        for &(ordinals, retained, ranges, valid) in cases {
            let (mut pending, receipt) =
                pair(usize::from(*ordinals.iter().max().unwrap()) + 1, 1, 0);
            pending.explained = ordinals
                .iter()
                .map(|ordinal| (*ordinal, LivingEventFactsV2::default()))
                .collect();
            // A nonempty bounded-child source also directly exercises a zero-prefix view.
            let expected_zero_ranges = {
                let mut groups: Vec<[u16; 2]> = Vec::new();
                for &ordinal in ordinals {
                    if let Some(last) = groups.last_mut()
                        && u32::from(last[1]) + 1 == u32::from(ordinal)
                    {
                        last[1] = ordinal;
                    } else {
                        groups.push([ordinal, ordinal]);
                    }
                }
                groups
            };
            let (zero, _) = encode(
                FactsEncodingView::Prefix {
                    pending: &pending,
                    receipt: &receipt,
                    retained: 0,
                },
                true,
            );
            assert_eq!(zero, literal_sparse(&receipt, &[], &expected_zero_ranges));
            // A semantically valid bounded entry always fits alone. To naturally
            // select zero/one at the real cap, inject an explicitly malformed
            // oversized child; omitted semantics must remain unvisited.
            pending.explained[retained].1.affected = vec![LivingEntityIdV2([1; 16]); 65536];
            let expected = seal_living_event_facts_receipt_v2(&receipt, &pending);
            let mut expected_mids = Vec::new();
            let (mut low, mut high) = (0, ordinals.len());
            while low < high {
                let middle = low + (high - low).div_ceil(2);
                expected_mids.push(middle);
                // All prefixes through the giant exceed 2 MiB: one affected
                // array alone contributes 35*65536-1=2,293,759 bytes. Earlier
                // prefixes are <2 KiB by the literal framing above.
                if middle <= retained {
                    low = middle;
                } else {
                    high = middle - 1;
                }
            }
            assert_eq!(low, retained);
            let selected = literal_sparse(&receipt, &ordinals[..retained], ranges);
            let mut job = LivingEventFactsSealJobV2::begin(pending, receipt.clone());
            let mut mids = Vec::new();
            while job.stage != 7 {
                if job.stage == 5 && job.encoding.stage == 0 {
                    mids.push(job.middle);
                }
                job.visit().unwrap();
            }
            assert_eq!(mids, expected_mids);
            assert_eq!(job.low, retained);
            assert_eq!(job.encoding.bytes, selected.len());
            let (actual, _) = encode(
                FactsEncodingView::Prefix {
                    pending: job.pending.as_ref().unwrap(),
                    receipt: &receipt,
                    retained,
                },
                true,
            );
            assert_eq!(actual, selected);
            let result = run(&mut job);
            if valid {
                result.unwrap();
                let (_, _, actual) = job.take_ready_result().unwrap();
                assert_eq!(actual, expected.unwrap());
                assert_eq!(actual.omitted.as_ref().unwrap().ranges, ranges);
                assert_eq!(actual.events.len(), retained);
            } else {
                assert_eq!(expected, Err(LivingEventFactsErrorV2::Binding));
                assert_eq!(job.fault, Some(LivingEventFactsErrorV2::Binding));
                assert!(result.is_err());
                assert!(job.take_ready_result().is_none());
                assert!(job.visit().is_err());
                assert_eq!(job.last_visit_visits, 0);
            }
        }
    }
    #[test]
    fn facts_agreement_semantics_precede_start_tick_binding() {
        let agreement = LivingAgreementV2 {
            kind: LivingAgreementKindV2::Trade,
            participant_a: LivingEntityIdV2([1; 16]),
            participant_b: LivingEntityIdV2([2; 16]),
            start_tick: LivingTickV2(1),
            end_tick: LivingTickV2(2),
        };
        let mut pending = LivingPendingEventsV2::new();
        pending
            .record_explained(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::RunProduction,
                },
                LivingEventKindV2::AgreementFormed,
                LivingEventFactsV2 {
                    agreement: Some(agreement),
                    ..Default::default()
                },
            )
            .unwrap();
        let receipt = seal_living_tick_receipt_v2(
            LivingTickV2(1),
            vec![],
            &pending,
            LivingStateDigestV2([2; 32]),
        )
        .unwrap();
        let original = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
        for (end, stage, expected) in [
            (0, 5, LivingEventFactsErrorV2::Facts),
            (2, 6, LivingEventFactsErrorV2::Binding),
        ] {
            let mut facts = original.clone();
            let agreement = facts.events[0].facts.agreement.as_mut().unwrap();
            agreement.start_tick = LivingTickV2(0);
            agreement.end_tick = LivingTickV2(end);
            facts.integrity_sha256 = LivingReceiptDigestV2([0; 32]);
            assert_eq!(facts.verify_receipt(&receipt), Err(expected));
            let mut cursor = FactsPairVerificationCursor::new();
            loop {
                if let Err(error) = cursor.visit(&facts, &receipt) {
                    assert_eq!(error, expected);
                    break;
                }
            }
            assert_eq!((cursor.stage, cursor.index), (stage, 0));
            assert_eq!(cursor.visit(&facts, &receipt), Err(expected));
            assert_eq!(cursor.stage, stage);
        }
    }
    #[test]
    fn facts_final_groups_are_ordered_before_integrity_and_measure_full_capacity() {
        let (pending, receipt) = pair(8, 1, 0);
        let original = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
        for (ranges, expected_stage, expected_index) in [
            (vec![[0, 0], [7, 6]], 7, 1),
            (vec![[1, 1], [2, 2], [7, 6]], 8, 2),
            (vec![[1, 1], [2, 2], [4, 4]], 9, 1),
            (vec![[1, 2], [4, 4]], 11, 1),
        ] {
            let mut facts = original.clone();
            facts.events.truncate(1);
            facts.omitted = Some(LivingEventFactsOmittedV2 {
                reason: LivingEventFactsOmissionReasonV2::Capacity,
                ranges,
            });
            facts.integrity_sha256 = LivingReceiptDigestV2([0; 32]);
            let expected = facts.verify_receipt(&receipt).unwrap_err();
            let mut c = FactsPairVerificationCursor::new();
            let mut observed = 0u16;
            let actual = loop {
                observed |= 1 << c.stage;
                if let Err(e) = c.visit(&facts, &receipt) {
                    break e;
                }
            };
            assert_eq!(actual, expected);
            assert_eq!((c.stage, c.index), (expected_stage, expected_index));
            if expected_stage == 11 {
                assert_ne!(observed & (1 << 10), 0);
                assert_eq!(actual, LivingEventFactsErrorV2::Integrity);
            }
            assert_eq!(c.visit(&facts, &receipt), Err(actual));
            assert_eq!(c.stage, expected_stage);
        }
        // All endpoints individually fit; the full envelope is oversized and
        // must precede the deliberately invalid adjacent/duplicate ranges.
        let mut facts = original;
        facts.events.clear();
        facts.omitted = Some(LivingEventFactsOmittedV2 {
            reason: LivingEventFactsOmissionReasonV2::Capacity,
            ranges: vec![[0, 0]; 350_000],
        });
        let expected = LivingEventFactsErrorV2::Wire(LivingWireErrorV2::TooLarge {
            actual: 317 + 6 * 350_000,
            limit: 2_097_152,
        });
        assert_eq!(facts.verify_receipt(&receipt), Err(expected));
        let mut c = FactsPairVerificationCursor::new();
        loop {
            if let Err(error) = c.visit(&facts, &receipt) {
                assert_eq!(error, expected);
                break;
            }
        }
        assert_eq!(c.stage, 0);
        assert_eq!(c.encoding.bytes, 2_100_317);
    }
    fn omitted_owner_fixture() -> (
        LivingEventFactsSealJobV2,
        Vec<(usize, usize, usize)>,
        [usize; 3],
    ) {
        let mut pending = LivingPendingEventsV2::new();
        for _ in 0..2 {
            pending
                .record_explained(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::RunProduction,
                    },
                    LivingEventKindV2::FacilityBlocked,
                    LivingEventFactsV2 {
                        resources: vec![LivingEventResourceFactV2 {
                            role: LivingEventResourceRoleV2::Input,
                            resource: LivingResourceV2::Energy,
                            quantity: 1,
                        }],
                        blocked_reasons: vec![LivingBlockedReasonV2::Disabled],
                        affected: vec![LivingEntityIdV2([1; 16])],
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        pending
            .record_explained(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::RunProduction,
                },
                LivingEventKindV2::FacilityProduced,
                LivingEventFactsV2::default(),
            )
            .unwrap();
        // Deliberately malformed oversized OMITTED child, as in the worksheet.
        pending.explained[1].1.affected = vec![LivingEntityIdV2([2; 16]); 65536];
        pending.explained[2].1.resources = Vec::with_capacity(3);
        pending.explained[2].1.blocked_reasons = Vec::with_capacity(4);
        pending.explained[2].1.affected = Vec::with_capacity(5);
        let mut pointers = Vec::new();
        for (_, f) in &pending.explained {
            pointers.push((
                f.resources.as_ptr() as usize,
                f.resources.len(),
                f.resources.capacity(),
            ));
            pointers.push((
                f.blocked_reasons.as_ptr() as usize,
                f.blocked_reasons.len(),
                f.blocked_reasons.capacity(),
            ));
            pointers.push((
                f.affected.as_ptr() as usize,
                f.affected.len(),
                f.affected.capacity(),
            ));
        }
        let receipt = seal_living_tick_receipt_v2(
            LivingTickV2(1),
            vec![LivingRevisionIdV2([1; 32])],
            &pending,
            LivingStateDigestV2([2; 32]),
        )
        .unwrap();
        let fixed = [
            receipt.applied_revisions.as_ptr() as usize,
            receipt.events.as_ptr() as usize,
            pending.explained.as_ptr().wrapping_add(3) as usize,
        ];
        (
            LivingEventFactsSealJobV2::begin(pending, receipt),
            pointers,
            fixed,
        )
    }
    #[test]
    fn facts_omitted_original_owners_and_partial_lanes_have_exact_remaining_work() {
        let (mut probe, pointers, fixed) = omitted_owner_fixture();
        let mut lane = None;
        let mut cuts = std::collections::BTreeSet::new();
        let mut last_stage = None;
        let mut last_verification_stage = None;
        let mut last_signature = None;
        let mut observed: Vec<_> = pointers.iter().map(|(_, len, _)| Some(*len)).collect();
        let mut saw_exhausted = false;
        let mut saw_partial = false;
        let mut saw_held = false;
        let mut saw_output = false;
        let mut cut = 0usize;
        loop {
            let stage = probe.stage;
            let at_owner_cut = last_stage != Some(stage)
                || matches!(stage, 8 | 10 | 13 | 14)
                || (stage == 5 && probe.encoding.stage == 0)
                || (stage == 18 && last_verification_stage != Some(probe.verification.stage))
                || (stage == 15
                    && lane.is_some_and(|n| {
                        !(120..=196_608).contains(&n) || (98_300..=98_310).contains(&n)
                    }));
            if at_owner_cut || probe.ready() {
                let mut allocations = Vec::new();
                probe.creator_allocations(&mut allocations);
                for (i, (pointer, initial, capacity)) in pointers.iter().enumerate() {
                    let found: Vec<_> = allocations
                        .iter()
                        .filter(|a| a.0 == *pointer && a.2 == Some(*capacity))
                        .collect();
                    assert!(found.len() <= 1, "original buffer duplicated at {cut}/{i}");
                    if i < 3 {
                        assert_eq!(found.len(), 1);
                        assert_eq!(found[0].1, *initial);
                    }
                    let now = found.first().map(|a| a.1);
                    match (observed[i], now) {
                        (Some(before), Some(after)) => assert!(after <= before),
                        (Some(before), None) => {
                            assert_eq!(before, 0, "original dropped before empty at {cut}/{i}")
                        }
                        (None, Some(_)) => panic!("original buffer resurrected"),
                        (None, None) => {}
                    }
                    if i == 5 && now.is_some_and(|n| n > 0 && n < 65536) {
                        saw_partial = true;
                    }
                    observed[i] = now;
                }
                for (pointer, len) in [(fixed[0], 1), (fixed[1], 3)] {
                    assert_eq!(
                        allocations
                            .iter()
                            .filter(|a| a.0 == pointer && a.1 == len)
                            .count(),
                        1
                    );
                }
                saw_exhausted |= allocations
                    .iter()
                    .any(|a| a.0 == fixed[2] && a.1 == 0 && a.2.is_none());
                saw_held |= probe.held.is_some();
                saw_output |= probe.output.is_some();
                // Ignore identical long leaf-run lengths when selecting costly
                // reconstruction cuts; preserve empty/one/interior/root states.
                let signature = (
                    stage,
                    probe.verification.stage,
                    probe.held.is_some(),
                    probe.output.is_some(),
                    lane.is_some(),
                    allocations
                        .iter()
                        .filter(|a| a.2 != Some(0))
                        .map(|a| (a.0, if a.1 > 1 { 2 } else { a.1 }, a.2))
                        .collect::<Vec<_>>(),
                );
                if last_signature.as_ref() != Some(&signature) {
                    cuts.insert(cut);
                    last_signature = Some(signature);
                }
            }
            if lane == Some(98_305) {
                cuts.insert(cut);
            }
            last_stage = Some(stage);
            last_verification_stage = Some(probe.verification.stage);
            if probe.ready() {
                break;
            }
            advance_counted(&mut probe, &mut lane);
            cut += 1;
            assert!(cut < 2_000_000);
        }
        assert_eq!(probe.low, 1);
        assert_eq!(
            probe
                .output
                .as_ref()
                .unwrap()
                .omitted
                .as_ref()
                .unwrap()
                .ranges,
            [[1, 2]]
        );
        assert!(saw_exhausted && saw_partial && saw_held && saw_output);
        assert!(observed[3..].iter().all(Option::is_none));
        let schedules: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 64, 2, 1024, 7]];
        for (index, cut) in cuts.into_iter().enumerate() {
            let (mut job, _, _) = omitted_owner_fixture();
            let mut lane = None;
            for _ in 0..cut {
                advance_counted(&mut job, &mut lane);
            }
            let expected = job_cost(&job, lane);
            assert_eq!(
                retire_schedule(job, schedules[index % schedules.len()]),
                expected,
                "cut {cut}"
            );
        }
    }
}
