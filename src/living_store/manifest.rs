use super::*;
use std::collections::BTreeSet;
pub(crate) const MANIFEST_LIMIT: usize = 256 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reference {
    pub(crate) file: String,
    pub(crate) len: u64,
    pub(crate) sha256: [u8; 32],
    pub(crate) catalog: LivingCatalogHashV2,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiskSlot {
    pub(crate) descriptor: LivingSlotDescriptorV2,
    pub(crate) archives: Vec<(LivingGenerationV2, Reference)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) notebooks: Vec<(LivingGenerationV2, Reference)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiskPack {
    pub(crate) descriptor: LivingPackDescriptorV2,
    pub(crate) reference: Reference,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiskState {
    pub(crate) next_slot: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pack_generation_high_water: Option<u64>,
    pub(crate) slots: Vec<DiskSlot>,
    pub(crate) packs: Vec<DiskPack>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub(crate) kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) format_version: Option<u32>,
    pub(crate) generation: u64,
    pub(crate) payload: DiskState,
    pub(crate) integrity: [u8; 32],
}
impl Manifest {
    pub(crate) fn new(generation: u64, payload: DiskState) -> Result<Self, LivingStoreErrorV2> {
        let integrity = manifest_integrity(generation, &payload)?;
        Ok(Self {
            kind: "NYON_LIVING_STORE_V2".into(),
            format_version: Some(2),
            generation,
            payload,
            integrity,
        })
    }
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, LivingStoreErrorV2> {
        let value: Self =
            serde_json::from_slice(bytes).map_err(|_| LivingStoreErrorV2::CorruptRecovery)?;
        if value.kind != "NYON_LIVING_STORE_V2"
            || value.generation == 0
            || value.integrity
                != match value.format_version {
                    Some(2) => manifest_integrity(value.generation, &value.payload)?,
                    None => hash(
                        &serde_json::to_vec(&value.payload)
                            .map_err(|_| LivingStoreErrorV2::Integrity)?,
                    ),
                    _ => return Err(LivingStoreErrorV2::CorruptRecovery),
                }
        {
            return Err(LivingStoreErrorV2::CorruptRecovery);
        }
        value.payload.validate()?;
        Ok(value)
    }
}
fn manifest_integrity(
    generation: u64,
    payload: &DiskState,
) -> Result<[u8; 32], LivingStoreErrorV2> {
    let mut bytes = b"NYON-LIVING-STORE-MANIFEST-V2\0".to_vec();
    bytes.extend_from_slice(&generation.to_le_bytes());
    bytes.extend_from_slice(
        &serde_json::to_vec(payload).map_err(|_| LivingStoreErrorV2::Integrity)?,
    );
    Ok(hash(&bytes))
}
impl DiskState {
    pub(crate) fn validate(&self) -> Result<(), LivingStoreErrorV2> {
        if self.slots.len() > 64 || self.packs.len() > 256 {
            return Err(LivingStoreErrorV2::CorruptRecovery);
        }
        if self.pack_generation_high_water.is_some_and(|high| {
            self.packs
                .iter()
                .any(|pack| pack.descriptor.generation.0 > high)
        }) {
            return Err(LivingStoreErrorV2::Integrity);
        }
        let mut ids = BTreeSet::new();
        let mut packs = BTreeSet::new();
        for pack in &self.packs {
            if !packs.insert(pack.descriptor.hash)
                || pack.descriptor.generation.0 == 0
                || pack.reference.catalog != pack.descriptor.hash
            {
                return Err(LivingStoreErrorV2::CorruptRecovery);
            }
            validate_ref(&pack.reference, 2 * 1024 * 1024)?;
        }
        for slot in &self.slots {
            name(&slot.descriptor.name).map_err(|_| LivingStoreErrorV2::CorruptRecovery)?;
            if !ids.insert(slot.descriptor.id)
                || slot.descriptor.id.0 == 0
                || slot.descriptor.id.0 > self.next_slot
                || slot.archives.is_empty()
                || slot.archives.len() > 2
            {
                return Err(LivingStoreErrorV2::CorruptRecovery);
            }
            let mut notebook_generations = BTreeSet::new();
            for (g, r) in &slot.notebooks {
                if !notebook_generations.insert(g.0)
                    || !slot
                        .archives
                        .iter()
                        .any(|(a, archive)| a == g && archive.catalog == r.catalog)
                {
                    return Err(LivingStoreErrorV2::CorruptRecovery);
                }
                validate_ref(r, 1024 * 1024)?;
            }
            let mut previous = 0;
            for (g, r) in &slot.archives {
                if g.0 <= previous || !packs.contains(&r.catalog) {
                    return Err(LivingStoreErrorV2::CorruptRecovery);
                }
                previous = g.0;
                validate_ref(r, 32 * 1024 * 1024)?;
            }
            if previous != slot.descriptor.generation.0
                || slot
                    .archives
                    .last()
                    .is_none_or(|(_, r)| r.catalog != slot.descriptor.catalog_hash)
            {
                return Err(LivingStoreErrorV2::CorruptRecovery);
            }
        }
        Ok(())
    }
}
pub(crate) fn validate_ref(r: &Reference, limit: usize) -> Result<(), LivingStoreErrorV2> {
    let expected_file = r
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if r.file != expected_file
        || r.len == 0
        || r.len > limit as u64
        || r.file.len() != 64
        || !r
            .file
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(LivingStoreErrorV2::CorruptRecovery);
    }
    Ok(())
}

pub(crate) fn retired_payloads(
    retiring: &DiskState,
    current: &DiskState,
    prior: &DiskState,
) -> Vec<String> {
    fn files(state: &DiskState) -> BTreeSet<&str> {
        state
            .packs
            .iter()
            .map(|pack| pack.reference.file.as_str())
            .chain(state.slots.iter().flat_map(|slot| {
                slot.archives
                    .iter()
                    .chain(slot.notebooks.iter())
                    .map(|(_, reference)| reference.file.as_str())
            }))
            .collect()
    }
    let keep = files(current)
        .into_iter()
        .chain(files(prior))
        .collect::<BTreeSet<_>>();
    files(retiring)
        .into_iter()
        .filter(|file| !keep.contains(file))
        .map(str::to_owned)
        .collect()
}
