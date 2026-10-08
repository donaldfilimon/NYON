//! Actual IndexedDB coordinator. Each CAS updates both recovery records and
//! head in one transaction, comparing the exact snapshot used by Rust recovery.
use super::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(inline_js = r#"
async function selectionDb() {
  return await new Promise((resolve,reject) => {
    const r=indexedDB.open('nyon.library.v1',1);
    r.onupgradeneeded=()=>{if(!r.result.objectStoreNames.contains('selection'))r.result.createObjectStore('selection');};
    r.onsuccess=()=>resolve(r.result); r.onerror=()=>reject(r.error); r.onblocked=()=>reject(new Error('coordinator blocked'));
  });
}
export async function nyonSelectionRead() {
  const db=await selectionDb();
  try { return await new Promise((resolve,reject)=>{
    const t=db.transaction('selection','readonly'), s=t.objectStore('selection'), out=[null,null,null];
    ['generation-a','generation-b','head'].forEach((key,i)=>{ const r=s.get(key);r.onsuccess=()=>{out[i]=r.result??null;}; });
    t.oncomplete=()=>resolve(JSON.stringify(out));t.onerror=()=>reject(t.error);t.onabort=()=>reject(t.error);
  }); } finally {db.close();}
}
export async function nyonSelectionCas(snapshot,next,side,head) {
  const expected=JSON.parse(snapshot);
  const db=await selectionDb();
  try { return await new Promise((resolve,reject)=>{
    const t=db.transaction('selection','readwrite'),s=t.objectStore('selection'),out=[null,null,null];
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
          s.put(next,side);s.put(head,'head');
        }
      }catch(error){abortFailure(error);}
    };});
  }); } finally {db.close();}
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = nyonSelectionRead)]
    fn read_records() -> js_sys::Promise;
    #[wasm_bindgen(js_name = nyonSelectionCas)]
    fn cas_records(snapshot: &str, next: &str, side: &str, head: &str) -> js_sys::Promise;
}
fn storage(error: JsValue) -> SelectionError {
    SelectionError::Storage(format!("{error:?}"))
}
async fn snapshot() -> Result<(String, [Option<String>; 3]), SelectionError> {
    let text = JsFuture::from(read_records())
        .await
        .map_err(storage)?
        .as_string()
        .ok_or(SelectionError::CorruptRecovery)?;
    let values = serde_json::from_str(&text).map_err(|_| SelectionError::CorruptRecovery)?;
    Ok((text, values))
}
fn recovered(values: &[Option<String>; 3]) -> Result<Option<LibrarySelection>, SelectionError> {
    let head = values[2].as_ref().and_then(|text| text.parse().ok());
    if values[0].is_none() && values[1].is_none() && values[2].is_some() {
        return Err(SelectionError::CorruptRecovery);
    }
    recover_selection(
        values[0].as_ref().map(|text| text.as_bytes()),
        values[1].as_ref().map(|text| text.as_bytes()),
        head,
    )
}
pub async fn read() -> Result<Option<LibrarySelection>, SelectionError> {
    recovered(&snapshot().await?.1)
}
pub async fn compare_and_swap(
    expected: Option<u64>,
    target: Option<SelectionTarget>,
) -> Result<LibrarySelection, SelectionError> {
    compare_and_swap_guarded(expected, target, None).await
}
async fn compare_and_swap_guarded(
    expected: Option<u64>,
    target: Option<SelectionTarget>,
    cancel: Option<&Cell<u8>>,
) -> Result<LibrarySelection, SelectionError> {
    let (raw, values) = snapshot().await?;
    if recovered(&values)?.map(|record| record.generation) != expected {
        return Err(SelectionError::StaleGeneration);
    }
    let record = LibrarySelection::new(
        expected
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(SelectionError::Exhausted)?,
        target,
    )?;
    let a = values[0]
        .as_ref()
        .and_then(|text| LibrarySelection::decode(text.as_bytes()).ok());
    let side = if expected.is_some() && a.map(|record| record.generation) == expected {
        "generation-b"
    } else {
        "generation-a"
    };
    let next = String::from_utf8(record.encode()?).map_err(|_| SelectionError::CorruptRecovery)?;
    if let Some(cancel) = cancel {
        if cancel.get() == 2 {
            return Err(SelectionError::Cancelled);
        }
        cancel.set(1);
    }
    let result = JsFuture::from(cas_records(
        &raw,
        &next,
        side,
        &record.generation.to_string(),
    ))
    .await
    .map_err(storage)?;
    if result.as_bool() != Some(true) {
        return Err(SelectionError::StaleGeneration);
    }
    Ok(record)
}

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};
struct WebJob {
    cancel: Rc<Cell<u8>>,
    owner: u64,
    request: Option<SelectionRequest>,
    result: Rc<RefCell<Option<SelectionPoll>>>,
}
#[derive(Default)]
pub struct IndexedDbLibraryCoordinator {
    jobs: BTreeMap<SelectionJobId, WebJob>,
}
impl LibrarySelectionCoordinator for IndexedDbLibraryCoordinator {
    fn begin(
        &mut self,
        owner_epoch: u64,
        request: SelectionRequest,
    ) -> Result<SelectionJobId, SelectionError> {
        if self.jobs.len() >= 16 {
            return Err(SelectionError::Storage("coordinator job capacity".into()));
        }
        let id = SelectionJobId(super::allocate_job_id()?);
        self.jobs.insert(
            id,
            WebJob {
                cancel: Rc::new(Cell::new(0)),
                owner: owner_epoch,
                request: Some(request),
                result: Rc::new(RefCell::new(None)),
            },
        );
        Ok(id)
    }
    fn poll(&mut self, owner_epoch: u64, id: SelectionJobId, work_budget: usize) -> SelectionPoll {
        let Some(job) = self.jobs.get_mut(&id) else {
            return SelectionPoll::Failed(SelectionError::InvalidTarget);
        };
        if job.owner != owner_epoch {
            return SelectionPoll::Failed(SelectionError::InvalidTarget);
        };
        if work_budget == 0 {
            return SelectionPoll::Pending;
        }
        if let Some(request) = job.request.take() {
            let result = Rc::clone(&job.result);
            let cancel = Rc::clone(&job.cancel);
            wasm_bindgen_futures::spawn_local(async move {
                let outcome = match request {
                    SelectionRequest::Read => read().await,
                    SelectionRequest::CompareAndSwap { expected, target } => {
                        compare_and_swap_guarded(expected, target, Some(&cancel))
                            .await
                            .map(Some)
                    }
                };
                *result.borrow_mut() = Some(match outcome {
                    Ok(record) => SelectionPoll::Ready(record),
                    Err(SelectionError::Cancelled) => SelectionPoll::Cancelled,
                    Err(error) => SelectionPoll::Failed(error),
                });
            });
        }
        job.result
            .borrow()
            .clone()
            .unwrap_or(SelectionPoll::Pending)
    }
    fn cancel(
        &mut self,
        owner_epoch: u64,
        id: SelectionJobId,
    ) -> Result<SelectionCancel, SelectionError> {
        let Some(job) = self.jobs.get(&id) else {
            return Err(SelectionError::InvalidTarget);
        };
        if job.owner != owner_epoch {
            return Err(SelectionError::InvalidTarget);
        };
        let outcome = job.result.borrow().clone();
        match outcome {
            Some(SelectionPoll::Ready(Some(record))) => {
                Ok(SelectionCancel::CommitAlreadyWon(record))
            }
            Some(SelectionPoll::Failed(_)) => Ok(SelectionCancel::CancelPending),
            Some(_) => {
                self.jobs.remove(&id);
                Ok(SelectionCancel::Cancelled)
            }
            None if job.cancel.get() == 0 => {
                job.cancel.set(2);
                self.jobs.remove(&id);
                Ok(SelectionCancel::Cancelled)
            }
            None => Ok(SelectionCancel::CancelPending),
        }
    }
    fn release(&mut self, owner_epoch: u64, id: SelectionJobId) -> bool {
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

impl Drop for IndexedDbLibraryCoordinator {
    fn drop(&mut self) {
        for job in self.jobs.values() {
            if job.cancel.get() == 0 {
                job.cancel.set(2);
            }
        }
    }
}
