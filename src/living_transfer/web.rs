//! User-gesture browser file input/download. IndexedDB durability is separate.
use super::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Event, HtmlElement, HtmlInputElement};

#[derive(Default)]
pub struct BrowserLivingTransfer {
    result: Rc<RefCell<Option<Result<LivingTransferOutcome, LivingTransferError>>>>,
    active: bool,
    input: Option<HtmlInputElement>,
    listeners: Vec<Closure<dyn FnMut(Event)>>,
}

impl BrowserLivingTransfer {
    fn clean(&mut self) {
        if let Some(input) = self.input.take() {
            input.remove();
        }
        self.listeners.clear();
        self.active = false;
    }
}
impl Drop for BrowserLivingTransfer {
    fn drop(&mut self) {
        self.clean();
    }
}

impl LivingTransfer for BrowserLivingTransfer {
    fn begin(&mut self, request: LivingTransferRequest) -> Result<(), LivingTransferError> {
        if self.active {
            return Err(LivingTransferError::Busy);
        }
        request.validate()?;
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or(LivingTransferError::Unavailable)?;
        let body = document.body().ok_or(LivingTransferError::Unavailable)?;
        // A fresh mailbox prevents an abandoned import completion affecting its successor.
        self.result = Rc::new(RefCell::new(None));
        match request {
            LivingTransferRequest::Import(kind) => {
                let input: HtmlInputElement = document
                    .create_element("input")
                    .map_err(|_| LivingTransferError::Platform)?
                    .dyn_into()
                    .map_err(|_| LivingTransferError::Platform)?;
                input.set_type("file");
                input.set_accept(".json,application/json");
                input
                    .set_attribute("style", "display:none")
                    .map_err(|_| LivingTransferError::Platform)?;
                body.append_child(&input)
                    .map_err(|_| LivingTransferError::Platform)?;
                let file_input = input.clone();
                let mailbox = Rc::clone(&self.result);
                let change = Closure::<dyn FnMut(Event)>::new(move |_| {
                    let Some(file) = file_input.files().and_then(|files| files.get(0)) else {
                        *mailbox.borrow_mut() = Some(Ok(LivingTransferOutcome::Cancelled));
                        return;
                    };
                    if file.size() > kind.max_bytes() as f64 {
                        *mailbox.borrow_mut() = Some(Err(LivingTransferError::TooLarge));
                        return;
                    }
                    let mailbox = Rc::clone(&mailbox);
                    wasm_bindgen_futures::spawn_local(async move {
                        let result = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                            .await
                            .map_err(|_| LivingTransferError::Platform)
                            .and_then(|buffer| {
                                let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                                if bytes.is_empty() || bytes.len() > kind.max_bytes() {
                                    return Err(LivingTransferError::TooLarge);
                                }
                                Ok(LivingTransferOutcome::Imported { kind, bytes })
                            });
                        *mailbox.borrow_mut() = Some(result);
                    });
                });
                input
                    .add_event_listener_with_callback("change", change.as_ref().unchecked_ref())
                    .map_err(|_| LivingTransferError::Platform)?;
                let mailbox = Rc::clone(&self.result);
                let cancel = Closure::<dyn FnMut(Event)>::new(move |_| {
                    *mailbox.borrow_mut() = Some(Ok(LivingTransferOutcome::Cancelled));
                });
                input
                    .add_event_listener_with_callback("cancel", cancel.as_ref().unchecked_ref())
                    .map_err(|_| LivingTransferError::Platform)?;
                self.listeners = vec![change, cancel];
                self.input = Some(input.clone());
                self.active = true;
                input.click();
            }
            LivingTransferRequest::Export {
                suggested_name,
                bytes,
                ..
            } => {
                let parts = js_sys::Array::new();
                parts.push(&js_sys::Uint8Array::from(bytes.as_slice()));
                let options = web_sys::BlobPropertyBag::new();
                options.set_type("application/json");
                let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
                    .map_err(|_| LivingTransferError::Platform)?;
                let url = web_sys::Url::create_object_url_with_blob(&blob)
                    .map_err(|_| LivingTransferError::Platform)?;
                let result = (|| {
                    let link: HtmlElement = document
                        .create_element("a")
                        .map_err(|_| LivingTransferError::Platform)?
                        .dyn_into()
                        .map_err(|_| LivingTransferError::Platform)?;
                    link.set_attribute("href", &url)
                        .map_err(|_| LivingTransferError::Platform)?;
                    link.set_attribute("download", &suggested_name)
                        .map_err(|_| LivingTransferError::Platform)?;
                    body.append_child(&link)
                        .map_err(|_| LivingTransferError::Platform)?;
                    link.click();
                    link.remove();
                    Ok(LivingTransferOutcome::Exported(
                        LivingExportOutcome::DownloadStarted,
                    ))
                })();
                // Keep the URL alive until the browser has processed the
                // synthetic link activation. Revocation is presentation-only.
                let revoke_url = url.clone();
                let revoke = Closure::once_into_js(move || {
                    let _ = web_sys::Url::revoke_object_url(&revoke_url);
                });
                if web_sys::window().is_none_or(|window| {
                    window
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            revoke.unchecked_ref(),
                            1_000,
                        )
                        .is_err()
                }) {
                    let _ = web_sys::Url::revoke_object_url(&url);
                }
                *self.result.borrow_mut() = Some(result);
                self.active = true;
            }
        }
        Ok(())
    }
    fn poll(&mut self) -> LivingTransferStatus {
        if !self.active {
            return LivingTransferStatus::Idle;
        }
        let result = self.result.borrow_mut().take();
        if let Some(result) = result {
            self.clean();
            LivingTransferStatus::Ready(result)
        } else {
            LivingTransferStatus::Pending
        }
    }
    fn cancel(&mut self) {
        self.clean();
        self.result = Rc::new(RefCell::new(None));
    }
}
