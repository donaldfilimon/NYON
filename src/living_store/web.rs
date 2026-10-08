//! Actual separate Living IndexedDB adapter with incremental archive replay.
use super::{manifest::*, *};
use nyon_workshop_core::living::{LivingArchiveReplayStatusV2, begin_decode_living_archive_v2};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
#[wasm_bindgen(inline_js = r#"
async function livingDb(){return await new Promise((resolve,reject)=>{const r=indexedDB.open('nyon.living.v2',1);r.onupgradeneeded=()=>{if(!r.result.objectStoreNames.contains('living'))r.result.createObjectStore('living');};r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);r.onblocked=()=>reject(new Error('Living database blocked'));});}
export async function livingSnapshot(){const db=await livingDb();try{return await new Promise((resolve,reject)=>{const t=db.transaction('living','readonly'),s=t.objectStore('living'),out=[null,null,null];['generation-a','generation-b','head'].forEach((k,i)=>{const r=s.get(k);r.onsuccess=()=>out[i]=r.result??null;});t.oncomplete=()=>resolve(JSON.stringify(out));t.onerror=()=>reject(t.error);t.onabort=()=>reject(t.error);});}finally{db.close();}}
export async function livingPayload(key){const db=await livingDb();try{return await new Promise((resolve,reject)=>{const t=db.transaction('living','readonly'),r=t.objectStore('living').get(key);let out;r.onsuccess=()=>out=r.result;t.oncomplete=()=>resolve(out);t.onerror=()=>reject(t.error);t.onabort=()=>reject(t.error);});}finally{db.close();}}
export async function livingCommit(raw,side,manifest,head,payloads,garbage) {
  const expected=JSON.parse(raw),writes=JSON.parse(payloads),deletes=JSON.parse(garbage);
  const db=await livingDb();
  try { return await new Promise((resolve,reject)=>{
    const t=db.transaction('living','readwrite'),s=t.objectStore('living'),out=[null,null,null];
    let count=0,stale=false,failed=false,failure,settled=false;
    const finish=(error,value)=>{if(settled)return;settled=true;if(error)reject(failure);else resolve(value);};
    const abortFailure=(error)=>{
      failed=true;failure=error;stale=false;
      try{t.abort();}catch{finish(true);}
    };
    t.oncomplete=()=>finish(failed,true);
    t.onerror=()=>{if(!failed){failed=true;failure=t.error;}finish(true);};
    t.onabort=()=>{if(stale)finish(false,false);else{if(!failed){failed=true;failure=t.error;}finish(true);}};
    ['generation-a','generation-b','head'].forEach((key,i)=>{const r=s.get(key);r.onsuccess=()=>{
      if(failed||settled)return;
      try{
        out[i]=r.result??null;
        if(++count===3){
          if(out.some((v,j)=>v!==expected[j])){
            stale=true;
            try{t.abort();}catch(error){failed=true;failure=error;stale=false;finish(true);}
            return;
          }
          for(const [key,bytes] of writes)s.put(new Uint8Array(bytes),key);
          for(const key of deletes)s.delete(key);
          s.put(manifest,side);s.put(head,'head');
        }
      }catch(error){abortFailure(error);}
    };});
  }); } finally {db.close();}
}
export async function livingExists(key){const db=await livingDb();try{return await new Promise((resolve,reject)=>{const t=db.transaction('living','readonly'),r=t.objectStore('living').getKey(key);let exists=false;r.onsuccess=()=>exists=r.result!==undefined;t.oncomplete=()=>resolve(exists);t.onerror=()=>reject(t.error);t.onabort=()=>reject(t.error);});}finally{db.close();}}
export async function livingYield(){await new Promise(resolve=>setTimeout(resolve,0));}
"#)]
extern "C" {
    #[wasm_bindgen(js_name=livingSnapshot)]
    fn snapshot_js() -> js_sys::Promise;
    #[wasm_bindgen(js_name=livingPayload)]
    fn payload_js(key: &str) -> js_sys::Promise;
    #[wasm_bindgen(js_name=livingCommit)]
    fn commit_js(
        raw: &str,
        side: &str,
        manifest: &str,
        head: &str,
        payloads: &str,
        garbage: &str,
    ) -> js_sys::Promise;
    #[wasm_bindgen(js_name=livingExists)]
    fn exists_js(key: &str) -> js_sys::Promise;
    #[wasm_bindgen(js_name=livingYield)]
    fn yield_js() -> js_sys::Promise;
}
fn storage(value: JsValue) -> LivingStoreErrorV2 {
    LivingStoreErrorV2::Storage(format!("{value:?}"))
}
async fn payload(reference: &Reference, limit: usize) -> Result<Vec<u8>, LivingStoreErrorV2> {
    validate_ref(reference, limit)?;
    let value = JsFuture::from(payload_js(&reference.file))
        .await
        .map_err(storage)?;
    if !value.is_instance_of::<js_sys::Uint8Array>() {
        return Err(LivingStoreErrorV2::CorruptRecovery);
    }
    let array = js_sys::Uint8Array::new(&value);
    if array.length() as usize > limit || u64::from(array.length()) != reference.len {
        return Err(LivingStoreErrorV2::TooLarge);
    }
    let bytes = array.to_vec();
    if hash(&bytes) != reference.sha256 {
        return Err(LivingStoreErrorV2::Integrity);
    }
    Ok(bytes)
}
fn recover(
    values: &[Option<String>; 3],
) -> Result<(u64, DiskState, &'static str), LivingStoreErrorV2> {
    if values.iter().all(Option::is_none) {
        return Ok((0, DiskState::default(), "generation-a"));
    }
    let head = values[2].as_ref().and_then(|v| v.parse::<u64>().ok());
    let mut valid: Vec<_> = [(0, "generation-b"), (1, "generation-a")]
        .into_iter()
        .filter_map(|(i, side)| {
            values[i]
                .as_ref()
                .filter(|v| v.len() <= MANIFEST_LIMIT)
                .and_then(|v| Manifest::decode(v.as_bytes()).ok())
                .map(|g| (g, side))
        })
        .collect();
    valid.retain(|(g, _)| g.format_version == Some(2) || head == Some(g.generation));
    valid.sort_by_key(|(g, _)| g.generation);
    if let Some(head) = head {
        if let Some(i) = valid.iter().position(|(g, _)| g.generation == head) {
            let (g, side) = valid.remove(i);
            return Ok((g.generation, g.payload, side));
        }
        valid.retain(|(g, _)| g.generation < head);
    }
    valid
        .pop()
        .map(|(g, side)| (g.generation, g.payload, side))
        .ok_or(LivingStoreErrorV2::CorruptRecovery)
}
async fn validate_archive(
    state: &State,
    bytes: &[u8],
    cancel: &Cell<u8>,
    budget: &Cell<usize>,
) -> Result<(LivingCatalogHashV2, LivingGalaxyAuthorityV2), LivingStoreErrorV2> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| LivingStoreErrorV2::NonCanonical)?;
    let h: LivingCatalogHashV2 = serde_json::from_value(
        value
            .get("catalog_hash")
            .cloned()
            .ok_or(LivingStoreErrorV2::NonCanonical)?,
    )
    .map_err(|_| LivingStoreErrorV2::NonCanonical)?;
    let pack = state
        .packs
        .iter()
        .find(|p| p.descriptor.hash == h)
        .ok_or(LivingStoreErrorV2::MissingCatalog)?;
    let catalog =
        decode_living_catalog_pack_v2(&pack.bytes).map_err(|_| LivingStoreErrorV2::Integrity)?;
    let mut replay = begin_decode_living_archive_v2(bytes, &catalog)
        .map_err(|_| LivingStoreErrorV2::Integrity)?;
    loop {
        if cancel.get() == 2 {
            replay.cancel();
            return Err(LivingStoreErrorV2::Cancelled);
        }
        let allowance = loop {
            if cancel.get() == 2 {
                replay.cancel();
                return Err(LivingStoreErrorV2::Cancelled);
            }
            let available = budget.replace(0);
            if available > 0 {
                break available.min(1024);
            }
            JsFuture::from(yield_js()).await.map_err(storage)?;
        };
        match replay
            .poll(allowance)
            .map_err(|_| LivingStoreErrorV2::Integrity)?
        {
            LivingArchiveReplayStatusV2::Ready => break,
            LivingArchiveReplayStatusV2::Pending => {
                JsFuture::from(yield_js()).await.map_err(storage)?;
            }
        }
    }
    Ok((
        h,
        replay
            .finish()
            .map_err(|_| LivingStoreErrorV2::Integrity)?
            .into_authority(),
    ))
}
async fn transact(
    request: LivingStoreRequestV2,
    cancel: Rc<Cell<u8>>,
    budget: Rc<Cell<usize>>,
) -> Result<LivingStoreResultV2, LivingStoreErrorV2> {
    let raw = JsFuture::from(snapshot_js())
        .await
        .map_err(storage)?
        .as_string()
        .ok_or(LivingStoreErrorV2::CorruptRecovery)?;
    let values: [Option<String>; 3] =
        serde_json::from_str(&raw).map_err(|_| LivingStoreErrorV2::CorruptRecovery)?;
    let (generation, disk, side) = recover(&values)?;
    let mut state = State {
        next_slot: disk.next_slot,
        pack_generation_high_water: disk.pack_generation_high_water,
        slots: disk
            .slots
            .iter()
            .map(|s| Slot {
                descriptor: s.descriptor.clone(),
                archives: s.archives.iter().map(|(g, _)| (*g, Vec::new())).collect(),
                retained_catalogs: s.archives.iter().map(|(_, r)| r.catalog).collect(),
                notebooks: s.notebooks.iter().map(|(g, _)| (*g, Vec::new())).collect(),
            })
            .collect(),
        packs: disk
            .packs
            .iter()
            .map(|p| Pack {
                descriptor: p.descriptor.clone(),
                bytes: Vec::new(),
            })
            .collect(),
    };
    let slot_id = match &request {
        LivingStoreRequestV2::LoadSlot { id }
        | LivingStoreRequestV2::ReadArchive { id, .. }
        | LivingStoreRequestV2::ReadNotebook { id, .. }
        | LivingStoreRequestV2::WriteNotebook { id, .. }
        | LivingStoreRequestV2::SaveSlot { id, .. }
        | LivingStoreRequestV2::RenameSlot { id, .. }
        | LivingStoreRequestV2::SetArchived { id, .. } => Some(*id),
        _ => None,
    };
    let mut needed = BTreeSet::new();
    if let Some(id) = slot_id
        && let Some((i, slot)) = disk
            .slots
            .iter()
            .enumerate()
            .find(|(_, s)| s.descriptor.id == id)
    {
        for (j, (_, r)) in slot
            .archives
            .iter()
            .enumerate()
            .filter(|(_, (g, _))| match request {
                LivingStoreRequestV2::ReadArchive { generation, .. }
                | LivingStoreRequestV2::ReadNotebook { generation, .. } => g.0 == generation.0,
                _ => true,
            })
        {
            match payload(r, 32 * 1024 * 1024).await {
                Ok(bytes) => state.slots[i].archives[j].1 = bytes,
                Err(_)
                    if !matches!(
                        request,
                        LivingStoreRequestV2::ReadArchive { .. }
                            | LivingStoreRequestV2::ReadNotebook { .. }
                    ) => {}
                Err(error) => return Err(error),
            }
            needed.insert(r.catalog);
        }
        for (j, (_, r)) in slot
            .notebooks
            .iter()
            .enumerate()
            .filter(|(_, (g, _))| match request {
                LivingStoreRequestV2::ReadNotebook { generation, .. } => g.0 == generation.0,
                LivingStoreRequestV2::RenameSlot { .. }
                | LivingStoreRequestV2::SetArchived { .. } => *g == slot.descriptor.generation,
                _ => false,
            })
        {
            state.slots[i].notebooks[j].1 = payload(r, 1024 * 1024).await?;
        }
    }
    let incoming = match &request {
        LivingStoreRequestV2::CreateSlot { archive, .. }
        | LivingStoreRequestV2::SaveSlot { archive, .. } => Some(archive),
        _ => None,
    };
    if let Some(bytes) = incoming {
        let v: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| LivingStoreErrorV2::NonCanonical)?;
        let h = serde_json::from_value(
            v.get("catalog_hash")
                .cloned()
                .ok_or(LivingStoreErrorV2::NonCanonical)?,
        )
        .map_err(|_| LivingStoreErrorV2::NonCanonical)?;
        needed.insert(h);
    }
    if let LivingStoreRequestV2::ReadPack { hash } = request {
        needed.insert(hash);
    }
    for (i, pack) in disk.packs.iter().enumerate() {
        if needed.contains(&pack.descriptor.hash) {
            match payload(&pack.reference, 2 * 1024 * 1024).await {
                Ok(bytes) => state.packs[i].bytes = bytes,
                Err(_) if slot_id.is_some() => {}
                Err(error) => return Err(error),
            }
        }
    }
    if matches!(request, LivingStoreRequestV2::ListSlots) {
        for (entry, reference) in state.slots.iter_mut().zip(&disk.slots) {
            if let Some((_, reference)) = reference.archives.last()
                && JsFuture::from(exists_js(&reference.file))
                    .await
                    .map_err(storage)?
                    .as_bool()
                    != Some(true)
            {
                entry.descriptor.diagnostic = Some(LivingStoreErrorV2::NotFound);
            }
        }
    }
    if matches!(request, LivingStoreRequestV2::ListPacks) {
        for (entry, reference) in state.packs.iter_mut().zip(&disk.packs) {
            if JsFuture::from(exists_js(&reference.reference.file))
                .await
                .map_err(storage)?
                .as_bool()
                != Some(true)
            {
                entry.descriptor.diagnostic = Some(LivingStoreErrorV2::NotFound);
            }
        }
    }
    let mut validated = None;
    if let Some(bytes) = incoming {
        validated = Some(validate_archive(&state, bytes, &cancel, &budget).await?);
    }
    if let Some(id) = slot_id
        && !matches!(
            request,
            LivingStoreRequestV2::ReadArchive { .. } | LivingStoreRequestV2::ReadNotebook { .. }
        )
        && let Some(i) = state.slots.iter().position(|s| s.descriptor.id == id)
    {
        let mut recovery = None;
        for (j, (g, bytes)) in state.slots[i].archives.iter().enumerate().rev() {
            if let Ok((h, authority)) = validate_archive(&state, bytes, &cancel, &budget).await {
                recovery = Some((j, *g, h, authority));
                break;
            }
        }
        let (j, g, h, authority) = recovery.ok_or(LivingStoreErrorV2::CorruptRecovery)?;
        state.slots[i].archives.truncate(j + 1);
        state.slots[i].retained_catalogs.truncate(j + 1);
        let generations = state.slots[i]
            .archives
            .iter()
            .map(|(g, _)| *g)
            .collect::<Vec<_>>();
        state.slots[i]
            .notebooks
            .retain(|(g, _)| generations.contains(g));
        state.slots[i].descriptor.generation = g;
        state.slots[i].descriptor.catalog_hash = h;
        if matches!(request, LivingStoreRequestV2::LoadSlot { .. }) {
            validated = Some((h, authority));
        }
    }
    if cancel.get() == 2 {
        return Err(LivingStoreErrorV2::Cancelled);
    }
    let read_only = matches!(
        request,
        LivingStoreRequestV2::ListSlots
            | LivingStoreRequestV2::ListPacks
            | LivingStoreRequestV2::ListPackReferences { .. }
            | LivingStoreRequestV2::LoadSlot { .. }
            | LivingStoreRequestV2::ReadPack { .. }
            | LivingStoreRequestV2::ReadArchive { .. }
            | LivingStoreRequestV2::ReadNotebook { .. }
    );
    let mut notebook_validated = false;
    if let LivingStoreRequestV2::WriteNotebook {
        id,
        expected_generation,
        canonical_bytes,
    } = &request
    {
        let slot = state
            .slots
            .iter()
            .find(|s| s.descriptor.id == *id)
            .ok_or(LivingStoreErrorV2::NotFound)?;
        if slot.descriptor.generation != *expected_generation {
            return Err(LivingStoreErrorV2::StaleGeneration);
        }
        let pack = state
            .packs
            .iter()
            .find(|p| p.descriptor.hash == slot.descriptor.catalog_hash)
            .ok_or(LivingStoreErrorV2::MissingCatalog)?;
        let catalog = decode_living_catalog_pack_v2(&pack.bytes)
            .map_err(|_| LivingStoreErrorV2::Integrity)?;
        let bytes = &slot
            .archives
            .last()
            .ok_or(LivingStoreErrorV2::CorruptRecovery)?
            .1;
        let mut archive_replay = begin_decode_living_archive_v2(bytes, &catalog)
            .map_err(|_| LivingStoreErrorV2::Integrity)?;
        loop {
            let allowance = notebook_allowance(&cancel, &budget).await?;
            if archive_replay
                .poll(allowance)
                .map_err(|_| LivingStoreErrorV2::Integrity)?
                == LivingArchiveReplayStatusV2::Ready
            {
                break;
            }
            JsFuture::from(yield_js()).await.map_err(storage)?;
        }
        let archive = archive_replay
            .finish()
            .map_err(|_| LivingStoreErrorV2::Integrity)?;
        let mut replay = begin_decode_living_notebook_v2(canonical_bytes, archive)
            .map_err(|_| LivingStoreErrorV2::Integrity)?;
        loop {
            let allowance = notebook_allowance(&cancel, &budget).await?;
            if replay
                .poll(allowance.min(128))
                .map_err(|_| LivingStoreErrorV2::Integrity)?
                == LivingArchiveReplayStatusV2::Ready
            {
                break;
            }
            JsFuture::from(yield_js()).await.map_err(storage)?;
        }
        replay.finish().map_err(|_| LivingStoreErrorV2::Integrity)?;
        notebook_validated = true;
    }
    let result = state.execute_ready(request, validated, notebook_validated)?;
    if !read_only {
        let mut writes = Vec::new();
        let mut next = DiskState {
            next_slot: state.next_slot,
            pack_generation_high_water: state.pack_generation_high_water,
            ..DiskState::default()
        };
        for pack in state.packs {
            let reference = if pack.bytes.is_empty() {
                disk.packs
                    .iter()
                    .find(|p| p.descriptor.hash == pack.descriptor.hash)
                    .ok_or(LivingStoreErrorV2::CorruptRecovery)?
                    .reference
                    .clone()
            } else {
                reference(&pack.bytes, pack.descriptor.hash, &mut writes)
            };
            next.packs.push(DiskPack {
                descriptor: pack.descriptor,
                reference,
            });
        }
        for slot in state.slots {
            let mut archives = Vec::new();
            for (i, (g, bytes)) in slot.archives.into_iter().enumerate() {
                let r = if bytes.is_empty() {
                    disk.slots
                        .iter()
                        .find(|s| s.descriptor.id == slot.descriptor.id)
                        .and_then(|s| s.archives.iter().find(|(old, _)| *old == g))
                        .ok_or(LivingStoreErrorV2::CorruptRecovery)?
                        .1
                        .clone()
                } else {
                    reference(
                        &bytes,
                        *slot
                            .retained_catalogs
                            .get(i)
                            .ok_or(LivingStoreErrorV2::CorruptRecovery)?,
                        &mut writes,
                    )
                };
                archives.push((g, r));
            }
            let mut notebooks = Vec::new();
            for (g, bytes) in slot.notebooks {
                let r = if bytes.is_empty() {
                    disk.slots
                        .iter()
                        .find(|s| s.descriptor.id == slot.descriptor.id)
                        .and_then(|s| s.notebooks.iter().find(|(old, _)| *old == g))
                        .ok_or(LivingStoreErrorV2::CorruptRecovery)?
                        .1
                        .clone()
                } else {
                    reference(&bytes, slot.descriptor.catalog_hash, &mut writes)
                };
                notebooks.push((g, r));
            }
            next.slots.push(DiskSlot {
                descriptor: slot.descriptor,
                archives,
                notebooks,
            });
        }
        next.validate()?;
        let next_generation = generation.checked_add(1).ok_or(LivingStoreErrorV2::Limit)?;
        let retiring = values[if side == "generation-a" { 0 } else { 1 }]
            .as_ref()
            .filter(|bytes| bytes.len() <= MANIFEST_LIMIT)
            .and_then(|bytes| Manifest::decode(bytes.as_bytes()).ok());
        let garbage = retiring
            .map(|retiring| retired_payloads(&retiring.payload, &next, &disk))
            .unwrap_or_default();
        let garbage = serde_json::to_string(&garbage).map_err(|_| LivingStoreErrorV2::Integrity)?;
        let manifest = serde_json::to_string(&Manifest::new(next_generation, next)?)
            .map_err(|_| LivingStoreErrorV2::Integrity)?;
        if manifest.len() > MANIFEST_LIMIT {
            return Err(LivingStoreErrorV2::TooLarge);
        }
        if cancel.get() == 2 {
            return Err(LivingStoreErrorV2::Cancelled);
        }
        cancel.set(1);
        let writes = serde_json::to_string(&writes).map_err(|_| LivingStoreErrorV2::Integrity)?;
        let committed = JsFuture::from(commit_js(
            &raw,
            side,
            &manifest,
            &next_generation.to_string(),
            &writes,
            &garbage,
        ))
        .await
        .map_err(storage)?;
        if committed.as_bool() != Some(true) {
            return Err(LivingStoreErrorV2::StaleGeneration);
        }
    }
    Ok(result)
}
fn reference(
    bytes: &[u8],
    catalog: LivingCatalogHashV2,
    writes: &mut Vec<(String, Vec<u8>)>,
) -> Reference {
    let sha256 = hash(bytes);
    let file = sha256
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    writes.push((file.clone(), bytes.to_vec()));
    Reference {
        file,
        len: bytes.len() as u64,
        sha256,
        catalog,
    }
}
struct Job {
    owner: u64,
    request: Option<LivingStoreRequestV2>,
    cancel: Rc<Cell<u8>>,
    budget: Rc<Cell<usize>>,
    result: Rc<RefCell<Option<LivingStorePollV2>>>,
}
#[derive(Default)]
pub struct IndexedDbLivingStoreV2 {
    jobs: BTreeMap<LivingStoreJobIdV2, Job>,
}
impl LivingStoreV2 for IndexedDbLivingStoreV2 {
    fn begin(
        &mut self,
        owner_epoch: u64,
        request: LivingStoreRequestV2,
    ) -> Result<LivingStoreJobIdV2, LivingStoreErrorV2> {
        preflight(&request)?;
        if self.jobs.len() >= 16 {
            return Err(LivingStoreErrorV2::Limit);
        }
        let id = LivingStoreJobIdV2(super::allocate_job_id()?);
        self.jobs.insert(
            id,
            Job {
                owner: owner_epoch,
                request: Some(request),
                cancel: Rc::new(Cell::new(0)),
                budget: Rc::new(Cell::new(0)),
                result: Rc::new(RefCell::new(None)),
            },
        );
        Ok(id)
    }
    fn poll(
        &mut self,
        owner_epoch: u64,
        id: LivingStoreJobIdV2,
        work_budget: usize,
    ) -> LivingStorePollV2 {
        let Some(job) = self.jobs.get_mut(&id) else {
            return LivingStorePollV2::Failed(LivingStoreErrorV2::NotFound);
        };
        if job.owner != owner_epoch {
            return LivingStorePollV2::Failed(LivingStoreErrorV2::WrongOwner);
        }
        if work_budget == 0 {
            return LivingStorePollV2::Pending { work_done: 0 };
        }
        job.budget.set(
            job.budget
                .get()
                .saturating_add(work_budget.min(1024))
                .min(1024),
        );
        if let Some(request) = job.request.take() {
            let result = Rc::clone(&job.result);
            let cancel = Rc::clone(&job.cancel);
            let budget = Rc::clone(&job.budget);
            wasm_bindgen_futures::spawn_local(async move {
                let outcome = match transact(request, cancel, budget).await {
                    Ok(value) => LivingStorePollV2::Ready(value),
                    Err(error) => LivingStorePollV2::Failed(error),
                };
                *result.borrow_mut() = Some(outcome);
            });
        }
        job.result
            .borrow()
            .clone()
            .unwrap_or(LivingStorePollV2::Pending { work_done: 0 })
    }
    fn cancel(
        &mut self,
        owner_epoch: u64,
        id: LivingStoreJobIdV2,
    ) -> Result<LivingStoreCancelV2, LivingStoreErrorV2> {
        let Some(job) = self.jobs.get(&id) else {
            return Err(LivingStoreErrorV2::NotFound);
        };
        if job.owner != owner_epoch {
            return Err(LivingStoreErrorV2::WrongOwner);
        }
        let result = job.result.borrow().clone();
        if let Some(LivingStorePollV2::Ready(value)) = result {
            return Ok(LivingStoreCancelV2::CommitAlreadyWon(value));
        }
        if job.cancel.get() == 1 {
            return Ok(LivingStoreCancelV2::CancelPending);
        }
        job.cancel.set(2);
        self.jobs.remove(&id);
        Ok(LivingStoreCancelV2::Cancelled)
    }
    fn release(&mut self, owner_epoch: u64, id: LivingStoreJobIdV2) -> bool {
        if self
            .jobs
            .get(&id)
            .is_some_and(|job| job.owner == owner_epoch && job.result.borrow().is_some())
        {
            self.jobs.remove(&id);
            true
        } else {
            false
        }
    }
}
impl Drop for IndexedDbLivingStoreV2 {
    fn drop(&mut self) {
        for job in self.jobs.values() {
            if job.cancel.get() == 0 {
                job.cancel.set(2);
            }
        }
    }
}

async fn notebook_allowance(
    cancel: &Cell<u8>,
    budget: &Cell<usize>,
) -> Result<usize, LivingStoreErrorV2> {
    loop {
        if cancel.get() == 2 {
            return Err(LivingStoreErrorV2::Cancelled);
        }
        let available = budget.replace(0);
        if available > 0 {
            return Ok(available.min(1024));
        }
        JsFuture::from(yield_js()).await.map_err(storage)?;
    }
}
