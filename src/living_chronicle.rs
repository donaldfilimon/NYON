//! Bounded presentation cache of verified Living receipts. No authority mutation.
use nyon_workshop_core::living::{
    LivingDecisionOutcomeV2, LivingDecisionReceiptV2, LivingEntityIdV2, LivingEventFactsReceiptV2,
    LivingEventFactsV2, LivingEventIdV2, LivingEventKindV2, LivingEventProvenanceV2,
    LivingIntentDecisionV2, LivingPhaseV2, LivingReceiptDigestV2, LivingTickReceiptV2,
};
use std::collections::{BTreeSet, VecDeque};
pub const LIVING_CHRONICLE_LIMIT: usize = 500;
pub const LIVING_CHRONICLE_LABEL_LIMIT: usize = 2048;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ChronicleCategory {
    Production,
    Freight,
    Settlement,
    Diplomacy,
    War,
    Occupation,
    Civilization,
    Creator,
    Decision,
    Other,
}
impl ChronicleCategory {
    pub const ALL: [Self; 10] = [
        Self::Production,
        Self::Freight,
        Self::Settlement,
        Self::Diplomacy,
        Self::War,
        Self::Occupation,
        Self::Civilization,
        Self::Creator,
        Self::Decision,
        Self::Other,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Production => "Production",
            Self::Freight => "Freight",
            Self::Settlement => "Settlement",
            Self::Diplomacy => "Diplomacy",
            Self::War => "War",
            Self::Occupation => "Occupation",
            Self::Civilization => "Civilization",
            Self::Creator => "Creator",
            Self::Decision => "Decisions",
            Self::Other => "Other",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LivingChronicleFilter {
    pub category: Option<ChronicleCategory>,
    pub actor: Option<LivingEntityIdV2>,
    pub entity: Option<LivingEntityIdV2>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingChronicleRow {
    pub label: String,
    pub category: ChronicleCategory,
    pub tick_first: u64,
    pub tick_last: u64,
    /// Last contributing receipt; first/last event IDs remain individually derived references.
    pub receipt_digest: LivingReceiptDigestV2,
    pub first_receipt_digest: LivingReceiptDigestV2,
    pub decision_ordinal: Option<u16>,
    pub decision_phase: Option<LivingPhaseV2>,
    pub event_ordinal_first: Option<u16>,
    pub event_ordinal_last: Option<u16>,
    pub event_id_first: Option<LivingEventIdV2>,
    pub event_id_last: Option<LivingEventIdV2>,
    pub actor: Option<LivingEntityIdV2>,
    pub entity: Option<LivingEntityIdV2>,
    pub count: u64,
    pub provenance: Option<LivingEventProvenanceV2>,
    pub score: Option<i64>,
    pub outcome: Option<LivingDecisionOutcomeV2>,
    pub event_kind: Option<LivingEventKindV2>,
    /// Verified producer facts; None remains unknown rather than inferred.
    pub facts: Option<LivingEventFactsV2>,
    /// The producer requested details that exceeded the authenticated envelope.
    pub details_omitted_for_capacity: bool,
}
#[derive(Clone, Debug, Default)]
pub struct LivingChronicle {
    entries: VecDeque<LivingChronicleRow>,
    seen: VecDeque<LivingReceiptDigestV2>,
    last_tick: Option<u64>,
    pause: BTreeSet<ChronicleCategory>,
}
impl LivingChronicle {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.seen.clear();
        self.last_tick = None;
    }
    pub fn set_pause_categories(
        &mut self,
        categories: impl IntoIterator<Item = ChronicleCategory>,
    ) {
        self.pause = categories.into_iter().collect();
    }
    pub fn rows(&self, filter: &LivingChronicleFilter) -> Vec<&LivingChronicleRow> {
        self.entries
            .iter()
            .rev()
            .filter(|row| {
                filter.category.is_none_or(|v| row.category == v)
                    && filter.actor.is_none_or(|v| row.actor == Some(v))
                    && filter.entity.is_none_or(|v| row.entity == Some(v))
            })
            .collect()
    }
    /// Validates the complete pair before publishing any cache changes. Windows
    /// are [0,99], [100,199], ...; tick zero edits remain individually receipted.
    /// Caller clears on backward navigation or switching histories.
    pub fn record(
        &mut self,
        receipt: &LivingTickReceiptV2,
        decision: Option<&LivingDecisionReceiptV2>,
    ) -> Result<Option<ChronicleCategory>, String> {
        self.record_with_facts(receipt, decision, None)
    }
    /// Bind the entire ordinary/decision/facts set before publishing any row.
    pub fn record_with_facts(
        &mut self,
        receipt: &LivingTickReceiptV2,
        decision: Option<&LivingDecisionReceiptV2>,
        facts: Option<&LivingEventFactsReceiptV2>,
    ) -> Result<Option<ChronicleCategory>, String> {
        receipt.verify().map_err(|e| e.to_string())?;
        if let Some(facts) = facts {
            facts.verify_receipt(receipt).map_err(|e| e.to_string())?;
        }
        if let Some(decision) = decision {
            decision
                .verify_receipt(receipt)
                .map_err(|e| e.to_string())?;
        }
        if self.seen.contains(&receipt.digest) {
            return Ok(None);
        }
        if self.last_tick.is_some_and(|tick| receipt.tick.0 < tick) {
            return Err("Chronicle requires clear before backward navigation".into());
        }
        let mut next = self.clone();
        let mut pause = None;
        for event in &receipt.events {
            let details_omitted_for_capacity = facts
                .and_then(|facts| facts.omitted.as_ref())
                .is_some_and(|omitted| {
                    let index = omitted
                        .ranges
                        .partition_point(|range| range[1] < event.ordinal);
                    omitted
                        .ranges
                        .get(index)
                        .is_some_and(|range| range[0] <= event.ordinal)
                });
            let explained = facts.and_then(|facts| {
                facts
                    .events
                    .binary_search_by_key(&event.ordinal, |entry| entry.ordinal)
                    .ok()
                    .map(|index| &facts.events[index].facts)
            });
            let creator = matches!(event.provenance, LivingEventProvenanceV2::Creator { .. });
            let category = if creator {
                ChronicleCategory::Creator
            } else {
                category(event.kind)
            };
            if pause.is_none() && next.pause.contains(&category) {
                pause = Some(category);
            }
            let aggregate = !creator
                && matches!(
                    event.kind,
                    LivingEventKindV2::FacilityBlocked
                        | LivingEventKindV2::ShipmentDispatched
                        | LivingEventKindV2::ShipmentDelivered
                        | LivingEventKindV2::ShipmentReturned
                        | LivingEventKindV2::ShipmentWaiting
                );
            let prior = aggregate
                .then(|| {
                    next.entries.iter().position(|r| {
                        r.event_kind == Some(event.kind)
                            && r.category == category
                            && r.tick_first / 100 == receipt.tick.0 / 100
                            && r.facts.as_ref() == explained
                            && r.details_omitted_for_capacity == details_omitted_for_capacity
                    })
                })
                .flatten();
            let mut row = if let Some(index) = prior {
                next.entries.remove(index).expect("known bounded row")
            } else {
                LivingChronicleRow {
                    label: String::new(),
                    category,
                    tick_first: receipt.tick.0,
                    tick_last: receipt.tick.0,
                    receipt_digest: receipt.digest,
                    first_receipt_digest: receipt.digest,
                    decision_ordinal: None,
                    decision_phase: None,
                    event_ordinal_first: Some(event.ordinal),
                    event_ordinal_last: Some(event.ordinal),
                    event_id_first: Some(event.id),
                    event_id_last: Some(event.id),
                    actor: explained.and_then(|f| f.actor),
                    entity: explained.and_then(|f| f.entity),
                    count: 0,
                    provenance: Some(event.provenance),
                    score: None,
                    outcome: None,
                    event_kind: Some(event.kind),
                    facts: explained.cloned(),
                    details_omitted_for_capacity,
                }
            };
            row.count = row.count.checked_add(1).ok_or("Chronicle count overflow")?;
            row.tick_last = receipt.tick.0;
            row.receipt_digest = receipt.digest;
            row.event_ordinal_last = Some(event.ordinal);
            row.event_id_last = Some(event.id);
            // Aggregates span autonomous phases; no single phase is attributed.
            if aggregate && row.count > 1 {
                row.provenance = None;
            }
            row.label = if creator {
                format!("Creator: {}", event.kind.as_wire_str())
            } else if aggregate {
                format!(
                    "{}: {} recorded events",
                    event.kind.as_wire_str(),
                    row.count
                )
            } else {
                event.kind.as_wire_str().to_owned()
            };
            if let Some(facts) = explained {
                row.label = event_facts_label(&row.label, facts);
            } else if details_omitted_for_capacity {
                row.label
                    .push_str("; details unavailable: explanation capacity reached");
            }
            next.push(row);
        }
        if let Some(decision) = decision {
            for intent in &decision.intents {
                if pause.is_none() && next.pause.contains(&ChronicleCategory::Decision) {
                    pause = Some(ChronicleCategory::Decision);
                }
                next.push(LivingChronicleRow {
                    label: decision_label(intent),
                    category: ChronicleCategory::Decision,
                    tick_first: receipt.tick.0,
                    tick_last: receipt.tick.0,
                    receipt_digest: receipt.digest,
                    first_receipt_digest: receipt.digest,
                    decision_ordinal: Some(intent.ordinal),
                    decision_phase: Some(intent.phase),
                    event_ordinal_first: None,
                    event_ordinal_last: None,
                    event_id_first: None,
                    event_id_last: None,
                    actor: Some(intent.actor),
                    entity: intent.target,
                    count: 1,
                    provenance: None,
                    score: Some(intent.score.0),
                    outcome: Some(intent.outcome),
                    event_kind: None,
                    facts: None,
                    details_omitted_for_capacity: false,
                });
            }
        }
        next.last_tick = Some(receipt.tick.0);
        next.seen.push_back(receipt.digest);
        if next.seen.len() > LIVING_CHRONICLE_LIMIT {
            next.seen.pop_front();
        }
        *self = next;
        Ok(pause)
    }
    fn push(&mut self, row: LivingChronicleRow) {
        self.entries.push_back(row);
        if self.entries.len() > LIVING_CHRONICLE_LIMIT {
            self.entries.pop_front();
        }
    }
}
fn category(kind: LivingEventKindV2) -> ChronicleCategory {
    use LivingEventKindV2::*;
    match kind {
        FacilityBlocked
        | FacilityProduced
        | ConstructionCancelled
        | ConstructionCompleted
        | HullCompleted
        | RefundDiscarded => ChronicleCategory::Production,
        ShipmentCaptured | ShipmentDelivered | ShipmentDispatched | ShipmentReturned
        | ShipmentWaiting => ChronicleCategory::Freight,
        SettlementClaimOpened | SettlementClaimResolved => ChronicleCategory::Settlement,
        AgreementExpired | AgreementFormed => ChronicleCategory::Diplomacy,
        CombatRound | HullDestroyed | WarDeclared | WarEnded => ChronicleCategory::War,
        ColonyCaptured => ChronicleCategory::Occupation,
        CivilizationDormant | CivilizationReactivated => ChronicleCategory::Civilization,
        CreatorIntervention => ChronicleCategory::Creator,
        _ => ChronicleCategory::Other,
    }
}

const OMITTED: &str = " ... [summary omitted; inspect receipt]";
struct BoundedLabel {
    text: String,
    omitted: bool,
}
impl std::fmt::Write for BoundedLabel {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        for character in value.chars() {
            if self.text.len() == LIVING_CHRONICLE_LABEL_LIMIT - OMITTED.len() {
                self.omitted = true;
                return Err(std::fmt::Error);
            }
            self.text
                .push(if character.is_ascii() && !character.is_ascii_control() {
                    character
                } else {
                    '?'
                });
        }
        Ok(())
    }
}
fn decision_label(intent: &LivingIntentDecisionV2) -> String {
    use std::fmt::Write;
    let mut label = BoundedLabel {
        text: String::with_capacity(LIVING_CHRONICLE_LABEL_LIMIT),
        omitted: false,
    };
    let action = [
        "Construct",
        "Build hull",
        "Establish route",
        "Explore",
        "Move",
        "Settle",
        "Defend",
        "Attack",
        "Return",
        "Propose agreement",
        "Propose war",
    ][usize::from(intent.action.ordinal())];
    if write!(
        &mut label,
        "{action}: {:?}; score {}; tier {:?}; reasons ",
        intent.outcome, intent.score.0, intent.tier
    )
    .is_ok()
    {
        for (index, reason) in intent.reasons.iter().enumerate() {
            if (index > 0 && label.write_str("; ").is_err())
                || write!(&mut label, "{reason:?}").is_err()
            {
                break;
            }
        }
    }
    if label.omitted {
        label.text.push_str(OMITTED);
    }
    label.text
}

fn event_facts_label(base: &str, facts: &LivingEventFactsV2) -> String {
    use std::fmt::Write;
    let mut label = BoundedLabel {
        text: String::with_capacity(LIVING_CHRONICLE_LABEL_LIMIT),
        omitted: false,
    };
    let _ = label.write_str(base);
    if let Some(actor) = facts.actor {
        let _ = write!(&mut label, "; actor {}", hex_entity(actor));
    }
    if let Some(entity) = facts.entity {
        let _ = write!(&mut label, "; entity {}", hex_entity(entity));
    }
    if let Some(agreement) = &facts.agreement {
        let _ = write!(
            &mut label,
            "; {:?} agreement between {} and {}; ticks {} through {} (exclusive)",
            agreement.kind,
            hex_entity(agreement.participant_a),
            hex_entity(agreement.participant_b),
            agreement.start_tick.0,
            agreement.end_tick.0,
        );
    }
    if let Some(delivery) = &facts.delivery {
        let _ = write!(
            &mut label,
            "; per event delivered {} {} from world {} to world {}; sender {}; recipient {}; dispatch ",
            delivery.units,
            delivery.resource.as_wire_str(),
            hex_entity(delivery.source_world),
            hex_entity(delivery.destination_world),
            hex_entity(delivery.dispatch_owner),
            hex_entity(delivery.recipient),
        );
        match delivery.dispatch_kind {
            Some(kind) => {
                let _ = write!(&mut label, "{kind:?}");
            }
            None => {
                let _ = label.write_str("unknown");
            }
        }
    }
    for amount in &facts.resources {
        if write!(
            &mut label,
            "; per event {:?} {} {}",
            amount.role,
            amount.quantity,
            amount.resource.as_wire_str()
        )
        .is_err()
        {
            break;
        }
    }
    for reason in &facts.blocked_reasons {
        if write!(&mut label, "; {reason:?}").is_err() {
            break;
        }
    }
    if let Some(draw) = &facts.deposit_input {
        let _ = write!(
            &mut label,
            "; deposit draw {} units from world {}",
            draw.units,
            hex_entity(draw.world)
        );
    }
    if !facts.affected.is_empty() {
        let _ = write!(
            &mut label,
            "; affected {} identified entities (inspect receipt)",
            facts.affected.len()
        );
    }
    if label.omitted {
        label.text.push_str(OMITTED);
    }
    label.text
}

fn hex_entity(id: LivingEntityIdV2) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(32);
    for byte in id.0 {
        let _ = write!(&mut text, "{byte:02x}");
    }
    text
}
