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
/// and later split. No schema string can require escaping or exceed34 bytes.
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

/// Closed dispatch: ID34, tick20, ordinal5, quantity10, resource8, role13,
/// agreement kind15, route kind10, blocked tag17, null4. IDs use their existing
/// fixed32-byte temporary String serializer. No compound record is a leaf.
#[derive(Clone, Copy)]
enum FactsJsonLeaf {
    Entity(LivingEntityIdV2),
    Event(LivingEventIdV2),
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
