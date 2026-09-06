use std::{collections::BTreeSet, fmt};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use sha2::{Digest, Sha256};

use crate::ids::{CatalogHash, CatalogId, is_catalog_id, is_description, is_name, is_pack_id};

pub const MAX_PACK_BYTES: usize = 1_048_576;
const MAX_JSON_DEPTH: usize = 32;
const MAX_QUANTITY: u64 = 1_000_000_000;
const MAX_DURATION_TICKS: u64 = 36_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogDefinitionKind {
    Solar,
    Extractor,
    Processor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityV1 {
    pub resource_id: CatalogId,
    pub quantity: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum IndustryDefinitionV1 {
    Solar {
        id: CatalogId,
        name: String,
        description: String,
        duration_ticks: u64,
        outputs: Vec<QuantityV1>,
    },
    Extractor {
        id: CatalogId,
        name: String,
        description: String,
        duration_ticks: u64,
        energy_input: QuantityV1,
        output: QuantityV1,
    },
    Processor {
        id: CatalogId,
        name: String,
        description: String,
        duration_ticks: u64,
        inputs: Vec<QuantityV1>,
        outputs: Vec<QuantityV1>,
    },
}

impl IndustryDefinitionV1 {
    pub fn id(&self) -> &CatalogId {
        match self {
            Self::Solar { id, .. } | Self::Extractor { id, .. } | Self::Processor { id, .. } => id,
        }
    }

    pub const fn kind(&self) -> CatalogDefinitionKind {
        match self {
            Self::Solar { .. } => CatalogDefinitionKind::Solar,
            Self::Extractor { .. } => CatalogDefinitionKind::Extractor,
            Self::Processor { .. } => CatalogDefinitionKind::Processor,
        }
    }

    pub fn inputs(&self) -> &[QuantityV1] {
        match self {
            Self::Solar { .. } => &[],
            Self::Extractor { energy_input, .. } => std::slice::from_ref(energy_input),
            Self::Processor { inputs, .. } => inputs,
        }
    }

    pub fn outputs(&self) -> &[QuantityV1] {
        match self {
            Self::Solar { outputs, .. } | Self::Processor { outputs, .. } => outputs,
            Self::Extractor { output, .. } => std::slice::from_ref(output),
        }
    }

    pub fn extractor_resource(&self) -> Option<&CatalogId> {
        match self {
            Self::Extractor { output, .. } => Some(&output.resource_id),
            Self::Solar { .. } | Self::Processor { .. } => None,
        }
    }

    fn name_and_description(&self) -> (&str, &str) {
        match self {
            Self::Solar {
                name, description, ..
            }
            | Self::Extractor {
                name, description, ..
            }
            | Self::Processor {
                name, description, ..
            } => (name, description),
        }
    }

    pub const fn duration_ticks(&self) -> u64 {
        match self {
            Self::Solar { duration_ticks, .. }
            | Self::Extractor { duration_ticks, .. }
            | Self::Processor { duration_ticks, .. } => *duration_ticks,
        }
    }

    /// Validated, presentation-safe display name supplied by this catalog.
    pub fn name(&self) -> &str {
        self.name_and_description().0
    }

    /// Validated, presentation-safe detail supplied by this catalog.
    pub fn description(&self) -> &str {
        self.name_and_description().1
    }

    fn canonicalize_quantities(&mut self) {
        match self {
            Self::Solar { outputs, .. } => {
                outputs.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
            }
            Self::Extractor { .. } => {}
            Self::Processor {
                inputs, outputs, ..
            } => {
                inputs.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
                outputs.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SimpleDefinitionV1 {
    id: CatalogId,
    name: String,
    description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HazardDefinitionV1 {
    id: CatalogId,
    name: String,
    description: String,
    duration_ticks: u64,
    route_capacity_numerator: u64,
    route_capacity_denominator: u64,
}

impl HazardDefinitionV1 {
    pub fn id(&self) -> &CatalogId {
        &self.id
    }

    pub const fn duration_ticks(&self) -> u64 {
        self.duration_ticks
    }

    pub const fn route_capacity_ratio(&self) -> (u64, u64) {
        (
            self.route_capacity_numerator,
            self.route_capacity_denominator,
        )
    }

    /// Validated, presentation-safe display name supplied by this catalog.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Validated, presentation-safe detail supplied by this catalog.
    pub fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CatalogPackWireV1 {
    kind: String,
    format_version: u32,
    pack_id: String,
    pack_version: u32,
    title: String,
    star_archetypes: Vec<SimpleDefinitionV1>,
    world_archetypes: Vec<SimpleDefinitionV1>,
    resources: Vec<SimpleDefinitionV1>,
    industry_definitions: Vec<IndustryDefinitionV1>,
    hazard_definitions: Vec<HazardDefinitionV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedCatalogPackV1 {
    wire: CatalogPackWireV1,
    canonical: Vec<u8>,
    catalog_hash: CatalogHash,
}

impl ValidatedCatalogPackV1 {
    pub fn pack_id(&self) -> &str {
        &self.wire.pack_id
    }

    pub const fn pack_version(&self) -> u32 {
        self.wire.pack_version
    }

    pub fn title(&self) -> &str {
        &self.wire.title
    }

    pub const fn catalog_hash(&self) -> CatalogHash {
        self.catalog_hash
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }

    pub fn has_resource(&self, id: &CatalogId) -> bool {
        self.wire.resources.iter().any(|item| item.id == *id)
    }

    pub fn has_star_archetype(&self, id: &CatalogId) -> bool {
        self.wire.star_archetypes.iter().any(|item| item.id == *id)
    }

    pub fn has_world_archetype(&self, id: &CatalogId) -> bool {
        self.wire.world_archetypes.iter().any(|item| item.id == *id)
    }

    pub fn industry(&self, id: &CatalogId) -> Option<&IndustryDefinitionV1> {
        self.wire
            .industry_definitions
            .iter()
            .find(|item| item.id() == id)
    }

    pub fn hazard(&self, id: &CatalogId) -> Option<&HazardDefinitionV1> {
        self.wire
            .hazard_definitions
            .iter()
            .find(|item| item.id() == id)
    }

    pub fn resource_ids(&self) -> impl ExactSizeIterator<Item = &CatalogId> {
        self.wire.resources.iter().map(|item| &item.id)
    }

    pub fn star_archetype_metadata(&self, id: &CatalogId) -> Option<(&str, &str)> {
        simple_metadata(&self.wire.star_archetypes, id)
    }

    pub fn world_archetype_metadata(&self, id: &CatalogId) -> Option<(&str, &str)> {
        simple_metadata(&self.wire.world_archetypes, id)
    }

    pub fn resource_metadata(&self, id: &CatalogId) -> Option<(&str, &str)> {
        simple_metadata(&self.wire.resources, id)
    }
}

fn simple_metadata<'a>(
    definitions: &'a [SimpleDefinitionV1],
    id: &CatalogId,
) -> Option<(&'a str, &'a str)> {
    definitions
        .iter()
        .find(|item| item.id == *id)
        .map(|item| (item.name.as_str(), item.description.as_str()))
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackValidationError {
    #[error("catalog pack has {actual} bytes; the limit is {limit}")]
    PackTooLarge { actual: usize, limit: usize },
    #[error("catalog pack must not begin with a UTF-8 BOM")]
    Bom,
    #[error("catalog pack is not valid UTF-8")]
    InvalidUtf8,
    #[error("catalog pack JSON nesting exceeds depth {limit}")]
    DepthExceeded { limit: usize },
    #[error("catalog pack contains a duplicate JSON key")]
    DuplicateKey,
    #[error("catalog pack must use integer JSON numbers")]
    Float,
    #[error("catalog pack is not one complete valid JSON value")]
    Json,
    #[error("catalog pack has an unsupported kind or format version")]
    Format,
    #[error("catalog pack field is invalid: {field}")]
    InvalidField { field: &'static str },
    #[error("catalog pack exceeds the {kind} definition limit")]
    DefinitionLimit { kind: &'static str },
    #[error("catalog definition identifier is not globally unique")]
    DuplicateCatalogId,
    #[error("catalog definition references an unknown resource")]
    UnknownResource,
    #[error("catalog recipe is invalid")]
    InvalidRecipe,
}

pub fn decode_catalog_pack(bytes: &[u8]) -> Result<ValidatedCatalogPackV1, PackValidationError> {
    strict_preflight(bytes, MAX_PACK_BYTES)?;
    let value = parse_strict_value(bytes)?;
    let json = value.into_json();
    let mut wire: CatalogPackWireV1 =
        serde_json::from_value(json).map_err(|_| PackValidationError::Json)?;
    wire.star_archetypes
        .sort_by(|left, right| left.id.cmp(&right.id));
    wire.world_archetypes
        .sort_by(|left, right| left.id.cmp(&right.id));
    wire.resources.sort_by(|left, right| left.id.cmp(&right.id));
    for industry in &mut wire.industry_definitions {
        industry.canonicalize_quantities();
    }
    wire.industry_definitions
        .sort_by(|left, right| left.id().cmp(right.id()));
    wire.hazard_definitions
        .sort_by(|left, right| left.id.cmp(&right.id));
    validate_wire(&wire)?;
    let canonical = serde_json::to_vec(&wire).map_err(|_| PackValidationError::Json)?;
    let mut hash = Sha256::new();
    hash.update(b"NYON-WORKSHOP-PACK-V1\0");
    hash.update(&canonical);
    Ok(ValidatedCatalogPackV1 {
        wire,
        canonical,
        catalog_hash: CatalogHash(hash.finalize().into()),
    })
}

pub fn encode_catalog_pack(pack: &ValidatedCatalogPackV1) -> Result<Vec<u8>, PackValidationError> {
    if pack.canonical.len() > MAX_PACK_BYTES {
        return Err(PackValidationError::PackTooLarge {
            actual: pack.canonical.len(),
            limit: MAX_PACK_BYTES,
        });
    }
    Ok(pack.canonical.clone())
}

fn validate_wire(wire: &CatalogPackWireV1) -> Result<(), PackValidationError> {
    if wire.kind != "NYON_WORKSHOP_DATA" || wire.format_version != 1 {
        return Err(PackValidationError::Format);
    }
    if !is_pack_id(&wire.pack_id) {
        return Err(PackValidationError::InvalidField { field: "pack_id" });
    }
    if wire.pack_version == 0 {
        return Err(PackValidationError::InvalidField {
            field: "pack_version",
        });
    }
    if !is_name(&wire.title) {
        return Err(PackValidationError::InvalidField { field: "title" });
    }
    for (actual, limit, kind) in [
        (wire.star_archetypes.len(), 32, "star"),
        (wire.world_archetypes.len(), 64, "world"),
        (wire.resources.len(), 32, "resource"),
        (wire.industry_definitions.len(), 64, "industry"),
        (wire.hazard_definitions.len(), 32, "hazard"),
    ] {
        if actual > limit {
            return Err(PackValidationError::DefinitionLimit { kind });
        }
    }

    let mut ids = BTreeSet::new();
    for definition in wire
        .star_archetypes
        .iter()
        .chain(&wire.world_archetypes)
        .chain(&wire.resources)
    {
        validate_simple(definition)?;
        if !ids.insert(definition.id.clone()) {
            return Err(PackValidationError::DuplicateCatalogId);
        }
    }

    let resources: BTreeSet<CatalogId> =
        wire.resources.iter().map(|item| item.id.clone()).collect();
    for industry in &wire.industry_definitions {
        let (name, description) = industry.name_and_description();
        validate_identity(industry.id(), name, description)?;
        if !ids.insert(industry.id().clone()) {
            return Err(PackValidationError::DuplicateCatalogId);
        }
        if !(1..=MAX_DURATION_TICKS).contains(&industry.duration_ticks()) {
            return Err(PackValidationError::InvalidField {
                field: "duration_ticks",
            });
        }
        match industry {
            IndustryDefinitionV1::Solar { outputs, .. } => {
                validate_quantities(outputs, 1, 4, &resources)?;
            }
            IndustryDefinitionV1::Extractor {
                energy_input,
                output,
                ..
            } => {
                validate_quantity(energy_input, &resources)?;
                validate_quantity(output, &resources)?;
                if energy_input.resource_id == output.resource_id {
                    return Err(PackValidationError::InvalidRecipe);
                }
            }
            IndustryDefinitionV1::Processor {
                inputs, outputs, ..
            } => {
                validate_quantities(inputs, 1, 8, &resources)?;
                validate_quantities(outputs, 1, 4, &resources)?;
            }
        }
    }
    for hazard in &wire.hazard_definitions {
        validate_identity(&hazard.id, &hazard.name, &hazard.description)?;
        if !ids.insert(hazard.id.clone()) {
            return Err(PackValidationError::DuplicateCatalogId);
        }
        if !(1..=MAX_DURATION_TICKS).contains(&hazard.duration_ticks)
            || !(1..=MAX_QUANTITY).contains(&hazard.route_capacity_numerator)
            || !(1..=MAX_QUANTITY).contains(&hazard.route_capacity_denominator)
            || hazard.route_capacity_numerator > hazard.route_capacity_denominator
        {
            return Err(PackValidationError::InvalidField { field: "hazard" });
        }
    }
    Ok(())
}

fn validate_simple(definition: &SimpleDefinitionV1) -> Result<(), PackValidationError> {
    validate_identity(&definition.id, &definition.name, &definition.description)
}

fn validate_identity(
    id: &CatalogId,
    name: &str,
    description: &str,
) -> Result<(), PackValidationError> {
    if !is_catalog_id(id.as_str()) {
        return Err(PackValidationError::InvalidField { field: "id" });
    }
    if !is_name(name) {
        return Err(PackValidationError::InvalidField { field: "name" });
    }
    if !is_description(description) {
        return Err(PackValidationError::InvalidField {
            field: "description",
        });
    }
    Ok(())
}

fn validate_quantities(
    quantities: &[QuantityV1],
    minimum: usize,
    maximum: usize,
    resources: &BTreeSet<CatalogId>,
) -> Result<(), PackValidationError> {
    if !(minimum..=maximum).contains(&quantities.len()) {
        return Err(PackValidationError::InvalidRecipe);
    }
    let mut unique = BTreeSet::new();
    for quantity in quantities {
        validate_quantity(quantity, resources)?;
        if !unique.insert(&quantity.resource_id) {
            return Err(PackValidationError::InvalidRecipe);
        }
    }
    Ok(())
}

fn validate_quantity(
    quantity: &QuantityV1,
    resources: &BTreeSet<CatalogId>,
) -> Result<(), PackValidationError> {
    if !(1..=MAX_QUANTITY).contains(&quantity.quantity) {
        return Err(PackValidationError::InvalidField { field: "quantity" });
    }
    if !resources.contains(&quantity.resource_id) {
        return Err(PackValidationError::UnknownResource);
    }
    Ok(())
}

pub(crate) fn strict_preflight(bytes: &[u8], limit: usize) -> Result<(), PackValidationError> {
    if bytes.len() > limit {
        return Err(PackValidationError::PackTooLarge {
            actual: bytes.len(),
            limit,
        });
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(PackValidationError::Bom);
    }
    std::str::from_utf8(bytes).map_err(|_| PackValidationError::InvalidUtf8)?;
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_JSON_DEPTH {
                    return Err(PackValidationError::DepthExceeded {
                        limit: MAX_JSON_DEPTH,
                    });
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
enum StrictValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    String(String),
    Array(Vec<Self>),
    Object(serde_json::Map<String, serde_json::Value>),
}

impl StrictValue {
    fn into_json(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(value),
            Self::I64(value) => value.into(),
            Self::U64(value) => value.into(),
            Self::String(value) => value.into(),
            Self::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(Self::into_json).collect())
            }
            Self::Object(value) => serde_json::Value::Object(value),
        }
    }
}

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without floats or duplicate keys")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue::I64(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue::U64(value))
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Err(E::custom("float"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(StrictValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<StrictValue>()? {
            values.push(value);
        }
        Ok(StrictValue::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate key"));
            }
            let value = map.next_value::<StrictValue>()?.into_json();
            values.insert(key, value);
        }
        Ok(StrictValue::Object(values))
    }
}

fn parse_strict_value(bytes: &[u8]) -> Result<StrictValue, PackValidationError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictValue::deserialize(&mut deserializer).map_err(|error| {
        let text = error.to_string();
        if text.contains("duplicate key") {
            PackValidationError::DuplicateKey
        } else if text.contains("float") {
            PackValidationError::Float
        } else {
            PackValidationError::Json
        }
    })?;
    deserializer.end().map_err(|_| PackValidationError::Json)?;
    Ok(value)
}
