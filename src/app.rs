//! Shared application state and native/browser winit lifecycle.

mod core;
mod input_router;
pub mod onboarding;
pub mod settings;

pub use core::*;

use std::{sync::Arc, time::Duration};

#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    collections::BTreeMap,
    sync::mpsc::{self, Receiver, Sender},
};

use glam::Vec2;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use winit::dpi::LogicalSize;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

use crate::{
    advisory::gpu::GpuAdvisory,
    engine::{
        gpu::{GpuContext, GpuError},
        input::{Action, InputState, NavigationAction},
        primitives::PrimitiveBatch,
        render::Renderer,
        render_frame::RenderFrame,
    },
    game::view::{
        GameLayout, build_command_deck_underlay, draw_advisory, draw_recoverable_message,
    },
    preferences::store::{MemoryPreferencesStore, PreferencesStore},
    presentation::{
        InteractionController, PointerButton, PointerSource,
        ui::{append_ui_primitives, build_ui_batch},
    },
    scenario::store::ScenarioStore,
    ui::{AtlasMetrics, UiBatch},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceState {
    Success,
    Suboptimal,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceDecision {
    Render,
    RenderThenReconfigure,
    Skip,
    Reconfigure,
    Recreate,
    Fatal,
}

pub const fn surface_decision(state: SurfaceState) -> SurfaceDecision {
    match state {
        SurfaceState::Success => SurfaceDecision::Render,
        SurfaceState::Suboptimal => SurfaceDecision::RenderThenReconfigure,
        SurfaceState::Timeout | SurfaceState::Occluded => SurfaceDecision::Skip,
        SurfaceState::Outdated => SurfaceDecision::Reconfigure,
        SurfaceState::Lost => SurfaceDecision::Recreate,
        SurfaceState::Validation => SurfaceDecision::Fatal,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    GpuInitFinished { generation: u64 },
}

#[cfg(not(target_arch = "wasm32"))]
type NativeGpuInitResult = Result<GpuContext, GpuError>;
#[cfg(target_arch = "wasm32")]
type WebGpuInitResult = Result<GpuContext, GpuError>;

#[cfg(not(target_arch = "wasm32"))]
struct CompletionInbox<T> {
    receiver: Receiver<(u64, T)>,
    pending: BTreeMap<u64, T>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> CompletionInbox<T> {
    fn new(receiver: Receiver<(u64, T)>) -> Self {
        Self {
            receiver,
            pending: BTreeMap::new(),
        }
    }

    fn take(&mut self, generation: u64) -> Option<T> {
        self.pending.extend(self.receiver.try_iter());
        self.pending.remove(&generation)
    }

    fn clear(&mut self) {
        self.pending.extend(self.receiver.try_iter());
        self.pending.clear();
    }
}

const fn next_gpu_generation(generation: u64) -> u64 {
    generation.wrapping_add(1)
}

const fn should_start_gpu_initialization(gpu_ready: bool, task_pending: bool) -> bool {
    !gpu_ready && !task_pending
}

/// Shared native/browser lifecycle owner. Platform entry points decide how the
/// asynchronous GPU future is driven; campaign and editor transitions remain in
/// `AppCore` and therefore stay testable without a window or adapter.
pub struct App<S: ScenarioStore, P: PreferencesStore = MemoryPreferencesStore> {
    core: AppCore<S, P>,
    input: InputState,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    renderer: Option<Renderer>,
    gpu_advisory: Option<GpuAdvisory>,
    underlay_batch: PrimitiveBatch,
    overlay_batch: PrimitiveBatch,
    ui_batch: UiBatch,
    ui_metrics: AtlasMetrics,
    interaction: InteractionController,
    modifiers: ModifiersState,
    last_frame: Option<Instant>,
    device_epoch: u64,
    gpu_generation: u64,
    fatal_message: Option<String>,
    first_frame_presented: bool,
    event_proxy: EventLoopProxy<AppEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    gpu_init_sender: Sender<(u64, NativeGpuInitResult)>,
    #[cfg(not(target_arch = "wasm32"))]
    gpu_init_inbox: CompletionInbox<NativeGpuInitResult>,
    #[cfg(not(target_arch = "wasm32"))]
    gpu_init_task: Option<tokio::task::JoinHandle<()>>,
    #[cfg(target_arch = "wasm32")]
    gpu_init_mailbox: Rc<RefCell<Option<WebGpuInitResult>>>,
    #[cfg(target_arch = "wasm32")]
    gpu_init_pending: bool,
}

impl<S: ScenarioStore, P: PreferencesStore> App<S, P> {
    pub fn new(core: AppCore<S, P>, event_proxy: EventLoopProxy<AppEvent>) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (gpu_init_sender, gpu_init_receiver) = mpsc::channel();

        Self {
            core,
            input: InputState::default(),
            window: None,
            gpu: None,
            renderer: None,
            gpu_advisory: None,
            underlay_batch: PrimitiveBatch::default(),
            overlay_batch: PrimitiveBatch::default(),
            ui_batch: UiBatch::default(),
            ui_metrics: AtlasMetrics::embedded().expect("validated embedded UI atlas metrics"),
            interaction: InteractionController::default(),
            modifiers: ModifiersState::empty(),
            last_frame: None,
            device_epoch: 0,
            gpu_generation: 0,
            fatal_message: None,
            first_frame_presented: false,
            event_proxy,
            #[cfg(not(target_arch = "wasm32"))]
            gpu_init_sender,
            #[cfg(not(target_arch = "wasm32"))]
            gpu_init_inbox: CompletionInbox::new(gpu_init_receiver),
            #[cfg(not(target_arch = "wasm32"))]
            gpu_init_task: None,
            #[cfg(target_arch = "wasm32")]
            gpu_init_mailbox: Rc::new(RefCell::new(None)),
            #[cfg(target_arch = "wasm32")]
            gpu_init_pending: false,
        }
    }

    pub fn core(&self) -> &AppCore<S, P> {
        &self.core
    }

    pub fn take_fatal_message(&mut self) -> Option<String> {
        self.fatal_message.take()
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: impl Into<String>) {
        if self.fatal_message.is_none() {
            let message = message.into();
            #[cfg(target_arch = "wasm32")]
            log::error!("{message}");
            self.fatal_message = Some(message);
        }
        event_loop.exit();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn initialize_gpu(&mut self, event_loop: &ActiveEventLoop) {
        if !should_start_gpu_initialization(self.gpu.is_some(), self.gpu_init_task.is_some()) {
            return;
        }
        let Some(window) = self.window.clone() else {
            return;
        };
        let prepared = match GpuContext::prepare(window) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.fail(event_loop, format!("GPU startup failed: {error}"));
                return;
            }
        };
        let runtime = match tokio::runtime::Handle::try_current() {
            Ok(runtime) => runtime,
            Err(error) => {
                self.fail(
                    event_loop,
                    format!("Tokio runtime is unavailable during GPU startup: {error}"),
                );
                return;
            }
        };
        self.gpu_generation = next_gpu_generation(self.gpu_generation);
        let generation = self.gpu_generation;
        let sender = self.gpu_init_sender.clone();
        let event_proxy = self.event_proxy.clone();
        log::info!("GPU initialization scheduled, generation {generation}");
        self.gpu_init_task = Some(runtime.spawn(async move {
            let result = prepared.initialize().await;
            if sender.send((generation, result)).is_ok() {
                let _ = event_proxy.send_event(AppEvent::GpuInitFinished { generation });
            }
        }));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn complete_gpu_initialization(&mut self, event_loop: &ActiveEventLoop, generation: u64) {
        let result = self.gpu_init_inbox.take(generation);
        if generation != self.gpu_generation {
            drop(result);
            log::debug!(
                "discarded stale GPU initialization generation {generation}; current generation is {}",
                self.gpu_generation
            );
            return;
        }
        let Some(result) = result else {
            self.fail(
                event_loop,
                format!("GPU initialization generation {generation} completed without a result"),
            );
            return;
        };
        self.gpu_init_task = None;
        self.install_gpu_initialization(event_loop, result);
    }

    #[cfg(target_arch = "wasm32")]
    fn initialize_gpu(&mut self) {
        if !should_start_gpu_initialization(self.gpu.is_some(), self.gpu_init_pending) {
            return;
        }
        let Some(window) = self.window.clone() else {
            return;
        };
        self.gpu_generation = next_gpu_generation(self.gpu_generation);
        let generation = self.gpu_generation;
        let mailbox = Rc::new(RefCell::new(None));
        self.gpu_init_mailbox = Rc::clone(&mailbox);
        self.gpu_init_pending = true;
        let event_proxy = self.event_proxy.clone();
        log::info!("browser GPU initialization scheduled, generation {generation}");
        wasm_bindgen_futures::spawn_local(async move {
            let result = GpuContext::new(window).await;
            *mailbox.borrow_mut() = Some(result);
            let _ = event_proxy.send_event(AppEvent::GpuInitFinished { generation });
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn complete_gpu_initialization(&mut self, event_loop: &ActiveEventLoop, generation: u64) {
        if generation != self.gpu_generation {
            log::debug!(
                "discarded stale browser GPU initialization generation {generation}; current generation is {}",
                self.gpu_generation
            );
            return;
        }
        let result = self.gpu_init_mailbox.borrow_mut().take();
        self.gpu_init_pending = false;
        let Some(result) = result else {
            self.fail(
                event_loop,
                format!(
                    "browser GPU initialization generation {generation} completed without a result"
                ),
            );
            return;
        };
        self.install_gpu_initialization(event_loop, result);
    }

    fn install_gpu_initialization(
        &mut self,
        event_loop: &ActiveEventLoop,
        result: Result<GpuContext, GpuError>,
    ) {
        let mut gpu = match result {
            Ok(gpu) => gpu,
            Err(error) => {
                self.fail(event_loop, format!("GPU startup failed: {error}"));
                return;
            }
        };
        let Some(window) = self.window.as_ref() else {
            self.fail(event_loop, "GPU initialization completed without a window");
            return;
        };
        if let Err(error) = gpu.resize(window.inner_size()) {
            self.fail(event_loop, format!("surface startup failed: {error}"));
            return;
        }
        let renderer = if gpu.surface_format().is_some() {
            match Renderer::new(&gpu) {
                Ok(renderer) => Some(renderer),
                Err(error) => {
                    self.fail(event_loop, format!("renderer startup failed: {error}"));
                    return;
                }
            }
        } else {
            None
        };
        let gpu_advisory = match GpuAdvisory::new(&gpu.adapter, &gpu.device, &gpu.queue) {
            Ok(advisory) => advisory,
            Err(error) => {
                log::warn!("GPU advisory unavailable: {error}");
                None
            }
        };
        self.device_epoch = self.device_epoch.wrapping_add(1);
        self.core
            .set_gpu_epoch(self.device_epoch, gpu_advisory.is_some());
        let info = gpu.adapter.get_info();
        log::info!(
            "GPU adapter '{}' backend {:?} device {:?}, epoch {}",
            info.name,
            info.backend,
            info.device_type,
            self.device_epoch
        );
        self.renderer = renderer;
        self.gpu_advisory = gpu_advisory;
        self.gpu = Some(gpu);
        self.submit_pending_advisory();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn cancel_gpu_initialization(&mut self) {
        self.gpu_generation = next_gpu_generation(self.gpu_generation);
        if let Some(task) = self.gpu_init_task.take() {
            task.abort();
        }
        self.gpu_init_inbox.clear();
    }

    #[cfg(target_arch = "wasm32")]
    fn cancel_gpu_initialization(&mut self) {
        self.gpu_generation = next_gpu_generation(self.gpu_generation);
        self.gpu_init_pending = false;
        self.gpu_init_mailbox = Rc::new(RefCell::new(None));
    }

    fn update_viewport(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let logical = window.inner_size().to_logical::<f32>(window.scale_factor());
        self.core
            .set_viewport(Vec2::new(logical.width, logical.height));
    }

    fn resize_surface(&mut self, event_loop: &ActiveEventLoop, size: PhysicalSize<u32>) {
        self.update_viewport();
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        match gpu.resize(size) {
            Ok(format_changed) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize([size.width, size.height]);
                }
                if gpu.surface_format().is_some() && (format_changed || self.renderer.is_none()) {
                    match Renderer::new(gpu) {
                        Ok(renderer) => self.renderer = Some(renderer),
                        Err(error) => {
                            self.fail(event_loop, format!("renderer resize failed: {error}"));
                        }
                    }
                }
            }
            Err(error) => self.fail(event_loop, format!("surface resize failed: {error}")),
        }
    }

    fn recreate_surface(&mut self, event_loop: &ActiveEventLoop) {
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        match gpu.recreate_surface() {
            Ok(_format_changed) => {
                if gpu.surface_format().is_some() {
                    match Renderer::new(gpu) {
                        Ok(renderer) => self.renderer = Some(renderer),
                        Err(error) => {
                            self.fail(event_loop, format!("renderer recovery failed: {error}"));
                        }
                    }
                }
            }
            Err(error) => self.fail(event_loop, format!("surface recovery failed: {error}")),
        }
    }

    fn build_frame(&mut self) {
        match self.core.mode() {
            AppMode::Playing => {
                let layout = GameLayout::with_user_scale(
                    self.core.viewport(),
                    self.core.preferences().ui_scale.factor(),
                );
                build_command_deck_underlay(
                    self.core.simulation().state(),
                    self.core.view_state(),
                    &layout,
                    &mut self.underlay_batch,
                );
                draw_advisory(
                    self.core.simulation().state(),
                    self.core.advisory_snapshot(),
                    self.core.advisory_updating(),
                    &layout,
                    &mut self.underlay_batch,
                );
                self.overlay_batch.clear();
                if let Some(message) = self.core.recoverable_message() {
                    draw_recoverable_message(message, &layout, &mut self.overlay_batch);
                }
                let ui_frame = self.core.ui_frame();
                append_ui_primitives(&ui_frame, &mut self.overlay_batch);
                if let Err(error) = build_ui_batch(&ui_frame, &self.ui_metrics, &mut self.ui_batch)
                {
                    self.ui_batch.clear();
                    self.core
                        .set_recoverable_message(format!("UI batch failed: {error}"));
                }
            }
            AppMode::EditingScenario => {
                self.ui_batch.clear();
                self.underlay_batch.clear();
                self.overlay_batch.clear();
                if let Some(editor) = self.core.editor() {
                    crate::editor::build_frame(editor, &mut self.underlay_batch);
                }
                if let Some(message) = self.core.recoverable_message() {
                    draw_recoverable_message(
                        message,
                        &GameLayout::new(self.core.viewport()),
                        &mut self.overlay_batch,
                    );
                }
            }
            AppMode::Settings => {
                self.underlay_batch.clear();
                self.overlay_batch.clear();
                let ui_frame = self.core.ui_frame();
                append_ui_primitives(&ui_frame, &mut self.overlay_batch);
                if let Err(error) = build_ui_batch(&ui_frame, &self.ui_metrics, &mut self.ui_batch)
                {
                    self.ui_batch.clear();
                    self.core
                        .set_recoverable_message(format!("UI batch failed: {error}"));
                }
                if let Some(message) = self.core.recoverable_message() {
                    draw_recoverable_message(
                        message,
                        &GameLayout::with_user_scale(
                            self.core.viewport(),
                            self.core.preferences().ui_scale.factor(),
                        ),
                        &mut self.overlay_batch,
                    );
                }
            }
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let delta = self
            .last_frame
            .replace(now)
            .map_or(Duration::ZERO, |previous| {
                now.saturating_duration_since(previous)
            });
        self.core.advance(delta);
        self.poll_advisory();
        self.submit_pending_advisory();
        self.build_frame();

        let acquired = self.gpu.as_ref().and_then(GpuContext::acquire);
        match acquired {
            Some(wgpu::CurrentSurfaceTexture::Success(frame)) => {
                self.render_frame(event_loop, frame, false)
            }
            Some(wgpu::CurrentSurfaceTexture::Suboptimal(frame)) => {
                self.render_frame(event_loop, frame, true)
            }
            Some(wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded)
            | None => {}
            Some(wgpu::CurrentSurfaceTexture::Outdated) => {
                if let Some(window) = &self.window {
                    self.resize_surface(event_loop, window.inner_size());
                }
            }
            Some(wgpu::CurrentSurfaceTexture::Lost) => self.recreate_surface(event_loop),
            Some(wgpu::CurrentSurfaceTexture::Validation) => {
                self.fail(event_loop, "surface acquisition validation failure")
            }
        }
        self.input.finish_frame();
    }

    fn render_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        frame: wgpu::SurfaceTexture,
        reconfigure_after_present: bool,
    ) {
        let Some(gpu) = &self.gpu else {
            return;
        };
        let scene = (self.core.mode() == AppMode::Playing).then(|| self.core.scene_frame());
        let physical_size = gpu.physical_size();
        let render_frame = RenderFrame {
            logical_viewport: self.core.viewport().to_array(),
            physical_target: [physical_size.width, physical_size.height],
            quality: self.core.presentation_preferences().graphics_quality,
            scene: scene.as_ref(),
            ui: (!self.ui_batch.glyphs().is_empty() || !self.ui_batch.panels().is_empty())
                .then_some(&self.ui_batch),
            primitive_underlay: &self.underlay_batch,
            primitive_overlay: &self.overlay_batch,
        };
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        if let Err(error) = renderer.render(gpu, frame, &render_frame) {
            self.fail(event_loop, format!("frame rendering failed: {error}"));
            return;
        }
        if !self.first_frame_presented {
            self.first_frame_presented = true;
            log::info!("first frame presented");
        }
        if reconfigure_after_present && let Some(window) = &self.window {
            self.resize_surface(event_loop, window.inner_size());
        }
    }

    fn submit_pending_advisory(&mut self) {
        let Some(request) = self.core.take_gpu_request() else {
            return;
        };
        let Some(advisory) = &mut self.gpu_advisory else {
            self.core
                .fail_device_epoch("GPU advisory backend is unavailable");
            return;
        };
        if let Err(error) = advisory.submit(&request) {
            self.core.fail_device_epoch(error.to_string());
            self.gpu_advisory = None;
        }
    }

    fn poll_advisory(&mut self) {
        let Some(advisory) = &mut self.gpu_advisory else {
            return;
        };
        match advisory.poll() {
            Ok(Some(completion)) => {
                self.core
                    .complete_gpu(completion.metadata, completion.result);
            }
            Ok(None) => {}
            Err(error) => {
                self.core.fail_device_epoch(error.to_string());
                self.gpu_advisory = None;
            }
        }
    }
}

impl<S: ScenarioStore + 'static, P: PreferencesStore + 'static> ApplicationHandler<AppEvent>
    for App<S, P>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.core.on_resume();
        self.last_frame = None;
        if self.window.is_none() {
            let attributes =
                Window::default_attributes().with_title("Intergalactic Warfare // Sector Command");
            #[cfg(not(target_arch = "wasm32"))]
            let attributes = attributes
                .with_inner_size(LogicalSize::new(1440.0, 900.0))
                .with_min_inner_size(LogicalSize::new(960.0, 600.0));
            #[cfg(target_arch = "wasm32")]
            let attributes = attributes
                .with_append(true)
                .with_prevent_default(true)
                .with_focusable(true);
            match event_loop.create_window(attributes) {
                Ok(window) => {
                    window.set_ime_allowed(true);
                    self.window = Some(Arc::new(window));
                    self.update_viewport();
                }
                Err(error) => {
                    self.fail(event_loop, format!("window creation failed: {error}"));
                    return;
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.initialize_gpu(event_loop);
        #[cfg(target_arch = "wasm32")]
        self.initialize_gpu();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.core.on_suspend();
        self.input.clear_all();
        self.last_frame = None;
        #[cfg(not(target_arch = "wasm32"))]
        self.cancel_gpu_initialization();
        #[cfg(target_arch = "wasm32")]
        self.cancel_gpu_initialization();
        self.renderer = None;
        self.gpu_advisory = None;
        self.gpu = None;
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::GpuInitFinished { generation } => {
                self.complete_gpu_initialization(event_loop, generation)
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize_surface(event_loop, size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    self.resize_surface(event_loop, window.inner_size());
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => self.handle_key(&event),
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                if let Some(editor) = self.core.editor_mut() {
                    editor.set_ime_preedit(text);
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if let Some(editor) = self.core.editor_mut() {
                    editor.commit_ime(&text);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let logical = physical_to_logical(position, self.window_scale_factor());
                self.input.cursor_logical = logical;
                let mut pointer_active = false;
                for id in 0..=2 {
                    if self.interaction.has_contact(id) {
                        pointer_active = true;
                        self.update_pointer(id, logical);
                    }
                }
                if pointer_active {
                    self.core.clear_hovered_world();
                } else {
                    let hud_consumed = self.core.ui_frame().consumes_pointer(logical);
                    self.core.set_scene_cursor(logical, hud_consumed);
                }
            }
            WindowEvent::CursorLeft { .. } => self.core.clear_hovered_world(),
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = wheel_logical_delta(delta, self.window_scale_factor());
                self.input.add_wheel_delta(delta);
                if let Some(editor) = self.core.editor_mut() {
                    editor.scroll_by(-delta.y);
                } else if self.core.mode() == AppMode::Playing
                    && !self
                        .core
                        .ui_frame()
                        .consumes_pointer(self.input.cursor_logical)
                {
                    self.core.zoom_camera(delta.y);
                }
            }
            WindowEvent::Touch(touch) => {
                let logical = physical_to_logical(touch.location, self.window_scale_factor());
                self.input.cursor_logical = logical;
                match touch.phase {
                    TouchPhase::Started => self.begin_pointer(
                        touch.id,
                        logical,
                        PointerSource::Touch,
                        PointerButton::Primary,
                    ),
                    TouchPhase::Moved => self.update_pointer(touch.id, logical),
                    TouchPhase::Ended => self.end_pointer(touch.id, logical),
                    TouchPhase::Cancelled => self.interaction.cancel(),
                }
            }
            WindowEvent::Focused(false) => {
                self.input.clear_all();
                self.interaction.cancel();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl<S: ScenarioStore, P: PreferencesStore> App<S, P> {
    fn window_scale_factor(&self) -> f64 {
        self.window
            .as_ref()
            .map_or(1.0, |window| window.scale_factor())
    }
}

fn mouse_pointer(button: MouseButton) -> Option<(u64, PointerButton)> {
    match button {
        MouseButton::Left => Some((0, PointerButton::Primary)),
        MouseButton::Right => Some((1, PointerButton::Secondary)),
        MouseButton::Middle => Some((2, PointerButton::Middle)),
        MouseButton::Back | MouseButton::Forward | MouseButton::Other(_) => None,
    }
}

fn physical_to_logical(position: PhysicalPosition<f64>, scale_factor: f64) -> Vec2 {
    let logical = position.to_logical::<f32>(scale_factor);
    Vec2::new(logical.x, logical.y)
}

fn wheel_logical_delta(delta: MouseScrollDelta, scale_factor: f64) -> Vec2 {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y) * 44.0,
        MouseScrollDelta::PixelDelta(position) => physical_to_logical(position, scale_factor),
    }
}

fn action_for_key(key: &Key) -> Option<Action> {
    match key {
        Key::Character(value) => match value.to_ascii_lowercase().as_str() {
            "1" => Some(Action::Field1),
            "2" => Some(Action::Field2),
            "3" => Some(Action::Field3),
            "q" => Some(Action::Decrease),
            "e" => Some(Action::Increase),
            "p" => Some(Action::Pause),
            "[" => Some(Action::SpeedDown),
            "]" => Some(Action::SpeedUp),
            "c" => Some(Action::CameraReset),
            "s" => Some(Action::Settings),
            "r" => Some(Action::Restart),
            "h" => Some(Action::Help),
            _ => None,
        },
        Key::Named(NamedKey::Space) => Some(Action::Launch),
        Key::Named(NamedKey::Escape) => Some(Action::Cancel),
        Key::Named(NamedKey::F4) => Some(Action::Scenario),
        _ => None,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_gpu_startup_tests {
    use super::*;

    fn inbox<T>() -> (Sender<(u64, T)>, CompletionInbox<T>) {
        let (sender, receiver) = mpsc::channel();
        (sender, CompletionInbox::new(receiver))
    }

    #[test]
    fn matching_generation_accepts_its_result() {
        let (sender, mut inbox) = inbox();
        sender.send((7, "current")).unwrap();

        assert_eq!(inbox.take(7), Some("current"));
    }

    #[test]
    fn stale_generation_is_discarded_without_replacing_current() {
        let (sender, mut inbox) = inbox();
        sender.send((4, "stale")).unwrap();
        sender.send((6, "current")).unwrap();

        let _ = inbox.take(4);
        assert_eq!(inbox.take(6), Some("current"));
    }

    #[test]
    fn out_of_order_result_remains_pending_until_its_marker() {
        let (sender, mut inbox) = inbox();
        sender.send((12, "newer")).unwrap();
        sender.send((11, "older")).unwrap();

        assert_eq!(inbox.take(11), Some("older"));
        assert_eq!(inbox.take(12), Some("newer"));
    }

    #[test]
    fn suspend_invalidation_makes_old_completion_stale() {
        let active = 22;
        let invalidated = next_gpu_generation(active);
        let (sender, mut inbox) = inbox();
        sender.send((active, "completed after suspend")).unwrap();

        assert_ne!(active, invalidated);
        let _ = inbox.take(active);
        assert_eq!(inbox.take(invalidated), None);
    }

    #[test]
    fn current_initialization_error_is_preserved_for_fatal_boundary() {
        let (sender, mut inbox) = inbox::<Result<(), GpuError>>();
        sender
            .send((31, Err(GpuError::UnsupportedSurface)))
            .unwrap();

        let error = inbox.take(31).unwrap().unwrap_err();
        assert_eq!(
            error.to_string(),
            "the graphics adapter does not support the window surface"
        );
    }

    #[test]
    fn redundant_resume_does_not_start_a_second_initialization() {
        assert!(should_start_gpu_initialization(false, false));
        assert!(!should_start_gpu_initialization(true, false));
        assert!(!should_start_gpu_initialization(false, true));
        assert!(!should_start_gpu_initialization(true, true));
    }
}

fn navigation_for_key(key: &Key, modifiers: ModifiersState) -> Option<NavigationAction> {
    match key {
        Key::Named(NamedKey::Tab) if modifiers.shift_key() => Some(NavigationAction::TabBackward),
        Key::Named(NamedKey::Tab) => Some(NavigationAction::TabForward),
        Key::Named(NamedKey::ArrowUp) => Some(NavigationAction::Up),
        Key::Named(NamedKey::ArrowDown) => Some(NavigationAction::Down),
        Key::Named(NamedKey::ArrowLeft) => Some(NavigationAction::Left),
        Key::Named(NamedKey::ArrowRight) => Some(NavigationAction::Right),
        Key::Named(NamedKey::PageUp) => Some(NavigationAction::PageUp),
        Key::Named(NamedKey::PageDown) => Some(NavigationAction::PageDown),
        Key::Named(NamedKey::Enter | NamedKey::Space) => Some(NavigationAction::Activate),
        Key::Named(NamedKey::Escape) => Some(NavigationAction::Escape),
        Key::Named(NamedKey::Backspace) => Some(NavigationAction::Backspace),
        Key::Named(NamedKey::Delete) => Some(NavigationAction::Delete),
        Key::Named(NamedKey::Home) => Some(NavigationAction::Home),
        Key::Named(NamedKey::End) => Some(NavigationAction::End),
        _ => None,
    }
}
