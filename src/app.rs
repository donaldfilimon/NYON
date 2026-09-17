//! Shared application state and native/browser winit lifecycle.

pub mod client_runtime;
mod core;
mod input_router;
pub mod onboarding;
pub mod settings;
pub mod transfer;

pub use core::*;

use std::{sync::Arc, time::Duration};

#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    collections::BTreeMap,
    sync::mpsc::{self, Receiver, Sender},
};

#[cfg(not(target_arch = "wasm32"))]
use accesskit::{Action as AccessibilityAction, ActionData as AccessibilityActionData};
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
    app::client_runtime::{
        ActiveSession, ClientDiagnosticCode, ClientRuntime, ClientRuntimeEffect, ClientScreen,
        MainMenuRoute,
    },
    engine::{
        backend::BackendKind,
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
        workshop::build_workshop_scene_frame,
    },
    scenario::store::ScenarioStore,
    ui::{
        AtlasMetrics, UiBatch,
        accessibility::{
            AnnouncementKind, FocusManager, InputModality, SemanticActionId, SemanticAnnouncement,
            SemanticNode, SemanticRole, SemanticTree,
        },
        creator::CreatorDraft,
        guide::{GuideAction, GuideLocation, build_guide_frame},
        library::{
            LIBRARY_RENAME_FIELD_ACTION, LibraryConfirmationKind, LibraryConfirmationRequest,
            LibraryUiContext, LibraryUiIntent, LibraryUiModel, library_confirmation_order,
            rename_draft_acceptable,
        },
        platform::{
            LibrarySheet, LibraryView, LibraryViewAction, PlatformUiAction, PlatformUiFrame,
            ShellPlatformInput, ShellUiAction, build_library_platform_frame,
            build_shell_platform_frame, build_workshop_platform_frame_for_view,
            clamp_library_row_start, draw_workshop_scene,
        },
        workshop::{
            CreatorTool, WorkshopUiContext, WorkshopUiIntent, WorkshopUiModel, creator_modal_order,
            default_creator_batch, removal_modal_order,
        },
        workshop_layout::WorkshopLayout,
        workshop_view::WorkshopViewState,
    },
    workshop::{
        session::WorkshopAction,
        store::{MemoryWorkshopStore, WorkshopStore},
    },
};

#[cfg(not(target_arch = "wasm32"))]
use crate::ui::platform_native::NativeSemanticAdapter;
#[cfg(target_arch = "wasm32")]
use crate::ui::platform_web::DomSemanticMirror;

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

#[derive(Debug)]
pub enum AppEvent {
    GpuInitFinished {
        generation: u64,
    },
    #[cfg(not(target_arch = "wasm32"))]
    Accessibility(accesskit_winit::Event),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<accesskit_winit::Event> for AppEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::Accessibility(event)
    }
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

mod durable_exit;
use durable_exit::{DurableExitAction, DurableExitState};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveCreatorEditor {
    tool: CreatorTool,
    draft: Option<CreatorDraft>,
    editing_field: Option<String>,
}

impl ActiveCreatorEditor {
    fn modal_order(&self) -> Vec<SemanticActionId> {
        // Owned by `ui::workshop::creator_defaults`, beside the controls themselves.
        creator_modal_order(self.draft.as_ref())
    }
}

enum AppEventProxy {
    Live(EventLoopProxy<AppEvent>),
    #[cfg(test)]
    Headless,
}

impl AppEventProxy {
    fn live(&self) -> EventLoopProxy<AppEvent> {
        match self {
            Self::Live(proxy) => proxy.clone(),
            #[cfg(test)]
            Self::Headless => {
                panic!("headless App tests cannot initialize platform event delivery")
            }
        }
    }
}

/// Shared native/browser lifecycle owner. Platform entry points decide how the
/// asynchronous GPU future is driven; campaign and editor transitions remain in
/// `AppCore` and therefore stay testable without a window or adapter.
pub struct App<
    S: ScenarioStore,
    P: PreferencesStore = MemoryPreferencesStore,
    W: WorkshopStore = MemoryWorkshopStore,
> {
    runtime: ClientRuntime<S, P, W>,
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
    backend_kind: Option<BackendKind>,
    platform_ui: Option<PlatformUiFrame>,
    workshop_ui: Option<WorkshopUiModel>,
    /// The model behind the Library frame, resolved against
    /// `PlatformUiAction::Library`. `None` whenever another screen is built,
    /// so a stale Library control cannot activate after the screen changes.
    library_ui: Option<LibraryUiModel>,
    /// Pure client selection, like `selected_workshop_entity`: no runtime
    /// state owns it, and the model filters it against the current list.
    selected_library_slot: Option<crate::workshop::store::SlotId>,
    /// The Library confirmation the user opened. While it is set the focus
    /// manager holds a modal scope over its controls.
    library_confirmation: Option<LibraryConfirmationRequest>,
    /// The rename dialog's draft, and whether a keystroke has replaced the
    /// name it opened with.
    library_rename_draft: String,
    library_rename_edited: bool,
    /// The control that opened the Library, focused again when it closes.
    library_return_focus: Option<SemanticActionId>,
    /// Row window and Compact sheet the Library frame is drawn for.
    library_view: LibraryView,
    /// A control the next frame should focus. `request_focus` only accepts a
    /// control already in the focus order, and one that a view action has just
    /// brought on screen is not in it until that frame is installed.
    pending_focus: Option<SemanticActionId>,
    ui_focus: FocusManager,
    player_guide: Option<(GuideLocation, FocusManager)>,
    selected_workshop_entity: Option<nyon_workshop_core::EntityId>,
    pending_removal: Option<nyon_workshop_core::EntityId>,
    active_creator: Option<ActiveCreatorEditor>,
    workshop_view: WorkshopViewState,
    continue_bootstrap_started: bool,
    quit_requested: bool,
    durable_exit: DurableExitState,
    #[cfg(not(target_arch = "wasm32"))]
    semantic_adapter: NativeSemanticAdapter,
    #[cfg(not(target_arch = "wasm32"))]
    accesskit_adapter: Option<accesskit_winit::Adapter>,
    #[cfg(target_arch = "wasm32")]
    semantic_adapter: Option<DomSemanticMirror>,
    event_proxy: AppEventProxy,
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

impl<S: ScenarioStore, P: PreferencesStore> App<S, P, MemoryWorkshopStore> {
    pub fn new(core: AppCore<S, P>, event_proxy: EventLoopProxy<AppEvent>) -> Self {
        Self::with_workshop_store(core, MemoryWorkshopStore::default(), event_proxy)
    }
}

impl<S: ScenarioStore, P: PreferencesStore, W: WorkshopStore> App<S, P, W> {
    pub fn with_workshop_store(
        core: AppCore<S, P>,
        workshop_store: W,
        event_proxy: EventLoopProxy<AppEvent>,
    ) -> Self {
        Self::with_event_proxy(core, workshop_store, AppEventProxy::Live(event_proxy))
    }

    fn with_event_proxy(
        core: AppCore<S, P>,
        workshop_store: W,
        event_proxy: AppEventProxy,
    ) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (gpu_init_sender, gpu_init_receiver) = mpsc::channel();

        Self {
            runtime: ClientRuntime::new(core, workshop_store),
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
            backend_kind: None,
            platform_ui: None,
            workshop_ui: None,
            library_ui: None,
            selected_library_slot: None,
            library_confirmation: None,
            library_rename_draft: String::new(),
            library_rename_edited: false,
            library_return_focus: None,
            library_view: LibraryView::default(),
            pending_focus: None,
            ui_focus: FocusManager::new(std::iter::empty()),
            player_guide: None,
            selected_workshop_entity: None,
            pending_removal: None,
            active_creator: None,
            workshop_view: WorkshopViewState::default(),
            continue_bootstrap_started: false,
            quit_requested: false,
            durable_exit: DurableExitState::default(),
            #[cfg(not(target_arch = "wasm32"))]
            semantic_adapter: NativeSemanticAdapter::default(),
            #[cfg(not(target_arch = "wasm32"))]
            accesskit_adapter: None,
            #[cfg(target_arch = "wasm32")]
            semantic_adapter: DomSemanticMirror::new()
                .map_err(|error| {
                    log::warn!("browser semantic mirror unavailable: {error:?}");
                    error
                })
                .ok(),
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
        self.runtime.classic()
    }

    pub fn runtime(&self) -> &ClientRuntime<S, P, W> {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut ClientRuntime<S, P, W> {
        &mut self.runtime
    }

    pub fn take_fatal_message(&mut self) -> Option<String> {
        self.fatal_message.take()
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: impl Into<String>) {
        if self.fatal_message.is_none() {
            let message = message.into();
            #[cfg(target_arch = "wasm32")]
            {
                log::error!("{message}");
                crate::platform::web::report_graphics_failed(&message);
            }
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
        let event_proxy = self.event_proxy.live();
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
        let event_proxy = self.event_proxy.live();
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
        let backend_kind = gpu.backend_kind();
        let gpu_advisory = if backend_kind.is_some_and(BackendKind::is_low_capability) {
            None
        } else {
            match GpuAdvisory::new(&gpu.adapter, &gpu.device, &gpu.queue) {
                Ok(advisory) => advisory,
                Err(error) => {
                    log::warn!("GPU advisory unavailable: {error}");
                    None
                }
            }
        };
        self.device_epoch = self.device_epoch.wrapping_add(1);
        self.runtime
            .classic_mut()
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
        self.backend_kind = backend_kind;
        self.gpu = Some(gpu);
        #[cfg(target_arch = "wasm32")]
        crate::platform::web::report_graphics_ready(
            backend_kind.map_or("UNKNOWN", BackendKind::label),
        );
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
        self.runtime
            .classic_mut()
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
        if let Some((location, _)) = self.player_guide.as_ref() {
            let guide = build_guide_frame(
                *location,
                self.runtime.classic().viewport(),
                self.runtime.classic().preferences().high_contrast,
                self.ui_focus.focused(),
            );
            self.ui_batch.clear();
            self.underlay_batch.clear();
            self.overlay_batch.clear();
            // Draw body after the opaque guide background has been installed.
            self.install_guide_frame(guide);
            return;
        }
        match self.runtime.screen() {
            ClientScreen::ClassicSector => self.build_classic_frame(),
            ClientScreen::GalaxyWorkshop => self.build_workshop_frame(),
            ClientScreen::Library => self.build_library_frame(),
            ClientScreen::MainMenu
            | ClientScreen::Settings
            | ClientScreen::Loading
            | ClientScreen::RecoverableError => self.build_shell_frame(),
        }
    }

    fn build_classic_frame(&mut self) {
        self.platform_ui = None;
        self.workshop_ui = None;
        self.leave_library();
        let mut guidance_nodes = Vec::new();
        match self.runtime.classic().mode() {
            AppMode::Playing => {
                let layout = GameLayout::with_user_scale(
                    self.runtime.classic().viewport(),
                    self.runtime.classic().preferences().ui_scale.factor(),
                );
                build_command_deck_underlay(
                    self.runtime.classic().simulation().state(),
                    self.runtime.classic().view_state(),
                    &layout,
                    &mut self.underlay_batch,
                );
                draw_advisory(
                    self.runtime.classic().simulation().state(),
                    self.runtime.classic().advisory_snapshot(),
                    self.runtime.classic().advisory_updating(),
                    &layout,
                    &mut self.underlay_batch,
                );
                self.overlay_batch.clear();
                if let Some(message) = self.runtime.classic().recoverable_message() {
                    draw_recoverable_message(message, &layout, &mut self.overlay_batch);
                }
                let ui_frame = self.runtime.classic().ui_frame();
                append_ui_primitives(&ui_frame, &mut self.overlay_batch);
                if let Some(marker) = crate::ui::start_marker::start_marker(
                    self.runtime.classic().onboarding_step(),
                    self.runtime.classic().simulation().state(),
                    &self.runtime.classic().scene_frame(),
                    &ui_frame,
                ) {
                    marker.draw(&mut self.overlay_batch);
                    guidance_nodes.push(SemanticNode::text("classic.start-here", SemanticRole::Text,
                        format!("START HERE: {}", marker.world_name),
                        "Tutorial paused. Click the marked Union world, or SKIP to play this same match now. F1 opens the player guide."));
                }
                if let Err(error) = build_ui_batch(&ui_frame, &self.ui_metrics, &mut self.ui_batch)
                {
                    self.ui_batch.clear();
                    self.runtime
                        .classic_mut()
                        .set_recoverable_message(format!("UI batch failed: {error}"));
                }
            }
            AppMode::EditingScenario => {
                self.ui_batch.clear();
                self.underlay_batch.clear();
                self.overlay_batch.clear();
                if let Some(editor) = self.runtime.classic().editor() {
                    crate::editor::build_frame(editor, &mut self.underlay_batch);
                }
                if let Some(message) = self.runtime.classic().recoverable_message() {
                    draw_recoverable_message(
                        message,
                        &GameLayout::new(self.runtime.classic().viewport()),
                        &mut self.overlay_batch,
                    );
                }
            }
            AppMode::Settings => {
                self.underlay_batch.clear();
                self.overlay_batch.clear();
                let ui_frame = self.runtime.classic().ui_frame();
                append_ui_primitives(&ui_frame, &mut self.overlay_batch);
                if let Err(error) = build_ui_batch(&ui_frame, &self.ui_metrics, &mut self.ui_batch)
                {
                    self.ui_batch.clear();
                    self.runtime
                        .classic_mut()
                        .set_recoverable_message(format!("UI batch failed: {error}"));
                }
                if let Some(message) = self.runtime.classic().recoverable_message() {
                    draw_recoverable_message(
                        message,
                        &GameLayout::with_user_scale(
                            self.runtime.classic().viewport(),
                            self.runtime.classic().preferences().ui_scale.factor(),
                        ),
                        &mut self.overlay_batch,
                    );
                }
            }
        }
        self.sync_semantics(&SemanticTree {
            root: SemanticNode::container(
                "classic.application",
                SemanticRole::Application,
                "NYON Classic Sector",
                guidance_nodes,
            ),
            announcements: Vec::new(),
        });
    }

    fn build_workshop_frame(&mut self) {
        self.ui_batch.clear();
        self.underlay_batch.clear();
        self.overlay_batch.clear();
        self.leave_library();
        let preferences = self.runtime.classic().preferences();
        let Some((snapshot, redo_children)) = (match self.runtime.active_session() {
            ActiveSession::Workshop(session) => {
                Some((session.snapshot().clone(), session.history().redo_choices()))
            }
            ActiveSession::None | ActiveSession::Classic => None,
        }) else {
            if let Err(error) = self.runtime.return_to_main_menu() {
                log::warn!("failed to recover the main-menu route: {error}");
            }
            self.build_shell_frame();
            return;
        };
        let catalog = match self.runtime.active_session() {
            ActiveSession::Workshop(session) => session.history().catalog(),
            ActiveSession::None | ActiveSession::Classic => {
                unreachable!("the active Workshop session was validated above")
            }
        };
        if self
            .selected_workshop_entity
            .is_some_and(|entity| !snapshot.state.contains_entity(entity))
        {
            self.selected_workshop_entity = None;
        }
        let model = WorkshopUiModel::build(
            &snapshot,
            WorkshopUiContext {
                backend: self.backend_kind,
                catalog_hash: Some(catalog.catalog_hash()),
                catalog: Some(catalog),
                selected_entity: self.selected_workshop_entity,
                redo_children: &redo_children,
                pending_removal: self.pending_removal,
                reduced_motion: preferences.motion
                    == crate::presentation::MotionPreference::Reduced,
                high_contrast: preferences.high_contrast,
                creator_form: self.active_creator.as_ref().map(|editor| editor.tool),
                creator_draft: self
                    .active_creator
                    .as_ref()
                    .and_then(|editor| editor.draft.as_ref()),
            },
        );
        let scene = build_workshop_scene_frame(
            &snapshot.state,
            self.selected_workshop_entity,
            preferences.high_contrast,
        );
        let workshop_layout = WorkshopLayout::resolve(
            self.runtime.classic().viewport(),
            preferences.ui_scale.factor(),
        )
        .expect("validated application viewport must support Workshop layout");
        draw_workshop_scene(
            &scene,
            &workshop_layout,
            &mut self.underlay_batch,
            preferences.high_contrast,
        );
        if let Some(focused) = self.ui_focus.focused() {
            self.workshop_view
                .reveal_action(&model, &workshop_layout, focused);
        }
        let frame = build_workshop_platform_frame_for_view(
            &model,
            workshop_layout,
            &self.workshop_view,
            self.ui_focus.focused(),
        );
        self.workshop_ui = Some(model);
        self.install_platform_frame(frame);
    }

    /// Every non-Library frame calls this, so leaving the screen by Close, by
    /// Escape, or by any other route drops the model and the selection alike,
    /// and the next visit starts from nothing. A selection kept across visits
    /// would be filtered against a list it was never made from.
    fn leave_library(&mut self) {
        // One-shot: the first frame after the Library focuses the control
        // that opened it. Later frames find nothing to take.
        if let Some(target) = self.library_return_focus.take() {
            self.pending_focus = Some(target);
        }
        self.close_library_confirmation();
        self.library_ui = None;
        self.selected_library_slot = None;
        self.library_view = LibraryView::default();
    }

    fn build_library_frame(&mut self) {
        self.ui_batch.clear();
        self.underlay_batch.clear();
        self.overlay_batch.clear();
        self.workshop_ui = None;
        let preferences = self.runtime.classic().preferences();
        let model = LibraryUiModel::build(LibraryUiContext {
            slots: self.runtime.library_slots(),
            status: self.runtime.library_slots_status(),
            selected_slot: self.selected_library_slot,
            resident_slot: self.runtime.resident_slot(),
            workshop_active: matches!(self.runtime.active_session(), ActiveSession::Workshop(_)),
            replacement_blocked: self.runtime.resident_workshop_blocks_replacement(),
            // Task 11: every handoff and file choice needs an adapter. No
            // product entry point installs one before the §8 spike, so the
            // imports and Save copy stay disabled while both exports prepare
            // bytes to Ready.
            handoff_available: self.runtime.transfer_available(),
            confirmation: self.library_confirmation,
            rename_draft: Some(self.library_rename_draft.as_str()),
        });
        // A confirmation whose save left the list builds no dialog, so the
        // trap it opened must close before this frame's order is installed.
        if model.confirmation.is_none() {
            self.close_library_confirmation();
        }
        let viewport = self.runtime.classic().viewport();
        let scale = preferences.ui_scale.factor();
        // The Library opens from the main menu, which the shell draws through a
        // 640 by 480 floor. A window below what the Workshop layout accepts is
        // reachable in a browser and for a frame at startup, and the
        // placeholder this replaced never panicked on one, so neither may
        // this. The floor resolves at every scale the layout allows.
        let layout = WorkshopLayout::resolve(viewport, scale)
            .or_else(|_| {
                let finite = if viewport.is_finite() {
                    viewport
                } else {
                    glam::Vec2::ZERO
                };
                WorkshopLayout::resolve(finite.max(glam::Vec2::new(640.0, 480.0)), scale)
            })
            .expect("a 640 by 480 floor always resolves");
        self.library_view.row_start = clamp_library_row_start(&model, self.library_view.row_start);
        // The sheet is a Compact surface. A window that grows out of Compact
        // docks both panels, so a sheet left open would reappear unasked on
        // the next shrink.
        if layout.mode != crate::ui::workshop_layout::WorkshopLayoutMode::Compact {
            self.library_view.sheet = None;
        }
        let frame = build_library_platform_frame(
            &model,
            layout,
            self.library_view,
            self.ui_focus.focused(),
            preferences.high_contrast,
        );
        self.library_ui = Some(model);
        self.install_platform_frame(frame);
    }

    fn build_shell_frame(&mut self) {
        self.ui_batch.clear();
        self.underlay_batch.clear();
        self.overlay_batch.clear();
        self.workshop_ui = None;
        self.leave_library();
        let capabilities = self.runtime.menu_capabilities();
        let recovery_message = self
            .runtime
            .recovery_diagnostic()
            .map(|diagnostic| safe_client_diagnostic(diagnostic.code));
        let frame = build_shell_platform_frame(ShellPlatformInput {
            screen: self.runtime.screen(),
            capabilities: &capabilities,
            credits_visible: self.runtime.credits_visible(),
            recovery_message,
            continue_available: self.runtime.continue_available(),
            backend: self.backend_kind,
            preferences: self.runtime.classic().preferences(),
            viewport: self.runtime.classic().viewport(),
            focused: self.ui_focus.focused(),
        });
        self.install_platform_frame(frame);
    }

    fn install_platform_frame(&mut self, mut frame: PlatformUiFrame) {
        self.prepare_platform_frame(&mut frame);
        let result = crate::ui::platform::install_platform_batches(
            &frame,
            &self.ui_metrics,
            &mut self.ui_batch,
            &mut self.overlay_batch,
        )
        .map(|_| ());
        self.finish_platform_frame(frame, result);
    }

    fn install_guide_frame(&mut self, mut guide: crate::ui::guide::GuideFrame) {
        self.prepare_platform_frame(&mut guide.platform);
        let result = crate::ui::platform::install_guide_batches(
            &guide,
            &self.ui_metrics,
            &mut self.ui_batch,
            &mut self.overlay_batch,
        );
        self.finish_platform_frame(guide.platform, result);
    }

    fn prepare_platform_frame(&mut self, frame: &mut PlatformUiFrame) {
        if self.durable_exit.is_pending() {
            frame.append_status_line(
                "Waiting for Workshop save and Continue selection; press Escape to cancel exit"
                    .to_owned(),
            );
            frame.semantics.announcements.push(SemanticAnnouncement {
                kind: AnnouncementKind::Status,
                code: "durable-exit-pending",
                message:
                    "Exit is waiting for the Workshop save to complete. Press Escape to cancel."
                        .to_owned(),
            });
        }
        self.ui_focus
            .replace_active_order(frame.focus_order())
            .expect("a presented modal retains an actionable control");
        if let Some(target) = self.pending_focus.take() {
            self.ui_focus.request_focus(&target);
        }
        let focused = self.ui_focus.focused();
        frame.reconcile_focused(focused);
    }

    fn finish_platform_frame(
        &mut self,
        frame: PlatformUiFrame,
        result: Result<(), crate::ui::UiBatchError>,
    ) {
        if let Err(error) = result {
            // Never format atlas keys, unsupported characters, or imported text.
            let (code, requested) = match error {
                crate::ui::UiBatchError::Capacity { requested } => ("glyph-capacity", requested),
                crate::ui::UiBatchError::PanelCapacity { requested } => {
                    ("panel-capacity", requested)
                }
                crate::ui::UiBatchError::NonFiniteGeometry => ("geometry", 0),
                crate::ui::UiBatchError::UnsupportedCharacter(_) => ("unsupported-character", 0),
                crate::ui::UiBatchError::MissingEntry(_) => ("missing-atlas-entry", 0),
            };
            log::warn!(
                "Platform UI composition failed: code={code} requested={requested} controls={} records={}",
                frame.controls.len(),
                frame.visible_nodes.len()
            );
        }
        self.sync_semantics(&frame.semantics);
        self.platform_ui = Some(frame);
    }

    fn sync_semantics(&mut self, tree: &SemanticTree) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.semantic_adapter.sync(tree, self.ui_focus.focused());
            let update = self.semantic_adapter.full_tree_update();
            if let Some(adapter) = &mut self.accesskit_adapter {
                adapter.update_if_active(|| update);
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(adapter) = &mut self.semantic_adapter
            && let Err(error) = adapter.sync(tree)
        {
            log::warn!("failed to update browser semantic mirror: {error:?}");
        }
    }

    fn drain_semantic_actions(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        let actions = self.semantic_adapter.drain_actions().collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let actions = self
            .semantic_adapter
            .as_mut()
            .map(|adapter| adapter.drain_actions().collect::<Vec<_>>())
            .unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        let edits = self
            .semantic_adapter
            .as_mut()
            .map(|adapter| adapter.drain_edits().collect::<Vec<_>>())
            .unwrap_or_default();
        for action in actions {
            self.activate_platform_action_id(&action, InputModality::Keyboard);
        }
        #[cfg(target_arch = "wasm32")]
        for (action, value) in edits {
            self.apply_creator_semantic_value(&action, value);
        }
    }

    fn apply_creator_semantic_value(&mut self, action: &SemanticActionId, value: String) {
        if self.player_guide.is_some() {
            return;
        }
        if action.as_str() == LIBRARY_RENAME_FIELD_ACTION {
            self.set_library_rename_draft(value);
            return;
        }
        let Some(field_id) = action.as_str().strip_prefix("creator.field.") else {
            return;
        };
        if let Some(editor) = &mut self.active_creator
            && editor
                .draft
                .as_mut()
                .is_some_and(|draft| draft.set_text(field_id, value).is_ok())
        {
            editor.editing_field = Some(field_id.to_owned());
        }
    }

    fn activate_platform_action_id(
        &mut self,
        action_id: &SemanticActionId,
        modality: InputModality,
    ) {
        let platform_action = self
            .platform_ui
            .as_ref()
            .and_then(|frame| frame.action(action_id))
            .cloned();
        if let Some(action) = platform_action {
            self.apply_platform_action(action, modality);
            return;
        }
        if self.player_guide.is_some() {
            return;
        }
        if let Some(intent) = self
            .workshop_ui
            .as_ref()
            .and_then(|model| model.activate(action_id, modality))
        {
            self.apply_workshop_intent(intent);
        }
    }

    fn apply_platform_action(&mut self, action: PlatformUiAction, modality: InputModality) {
        match action {
            PlatformUiAction::Guide(action) => self.apply_guide_action(action),
            PlatformUiAction::Shell(action) => self.apply_shell_action(action),
            PlatformUiAction::Workshop(action_id) => {
                if let Some(intent) = self
                    .workshop_ui
                    .as_ref()
                    .and_then(|model| model.activate(&action_id, modality))
                {
                    self.apply_workshop_intent(intent);
                }
            }
            PlatformUiAction::WorkshopView(action) => {
                self.apply_workshop_view_action(action);
            }
            PlatformUiAction::Library(action_id) => {
                if let Some(intent) = self
                    .library_ui
                    .as_ref()
                    .and_then(|model| model.activate(&action_id, modality))
                {
                    self.apply_library_intent(intent);
                }
            }
            PlatformUiAction::LibraryView(action) => self.apply_library_view_action(action),
        }
    }

    /// Moves the row window by the rows the current frame placed, so Next
    /// starts at the first row not yet shown and never skips one. Both ends
    /// are no-ops. Focus stays on the pager, because the row it was on may
    /// have just left the frame.
    fn apply_library_view_action(&mut self, action: LibraryViewAction) {
        let (Some(model), Some(frame)) = (self.library_ui.as_ref(), self.platform_ui.as_ref())
        else {
            return;
        };
        let shown = model
            .rows
            .iter()
            .filter(|row| {
                frame
                    .controls
                    .iter()
                    .any(|control| control.action_id == row.control.action_id)
            })
            .count()
            .max(1);
        let start = self.library_view.row_start;
        match action {
            LibraryViewAction::NextRows if start + shown < model.rows.len() => {
                self.library_view.row_start = start + shown;
            }
            LibraryViewAction::NextRows => {}
            LibraryViewAction::PreviousRows => {
                self.library_view.row_start = start.saturating_sub(shown);
            }
            LibraryViewAction::OpenTransfer => {
                self.library_view.sheet = Some(LibrarySheet::Transfer);
                self.pending_focus = Some(LibraryViewAction::CloseSheet.action_id());
                return;
            }
            LibraryViewAction::CloseSheet => {
                // Back to whatever opened the sheet.
                self.pending_focus = match self.library_view.sheet.take() {
                    Some(LibrarySheet::Transfer) => {
                        Some(LibraryViewAction::OpenTransfer.action_id())
                    }
                    _ => self
                        .selected_library_slot
                        .map(|slot| SemanticActionId::new(format!("library.slot.{}", slot.0))),
                };
                return;
            }
        }
        self.ui_focus.request_focus(&action.action_id());
    }

    /// Opens the trap before the model is rebuilt, from the owning module's
    /// order, exactly as the Workshop removal dialog does. A rename starts
    /// from the save's current name, and the first keystroke replaces it.
    fn open_library_confirmation(&mut self, request: LibraryConfirmationRequest) {
        if self.library_confirmation.is_some() {
            return;
        }
        if request.kind == LibraryConfirmationKind::Rename {
            let Some(name) = self.runtime.library_slots().and_then(|list| {
                list.slots
                    .iter()
                    .find(|summary| summary.id == request.slot)
                    .map(|summary| summary.name.as_str().to_owned())
            }) else {
                return;
            };
            self.library_rename_draft = name;
            self.library_rename_edited = false;
        }
        self.library_confirmation = Some(request);
        let _ = self
            .ui_focus
            .open_modal(library_confirmation_order(request.kind));
    }

    /// Replaces the rename draft from a keystroke or a browser edit.
    ///
    /// Only while a rename is open, and only with text the field may hold;
    /// anything else is refused and the draft is left as it was. Returns
    /// whether the edit was applied.
    fn set_library_rename_draft(&mut self, value: String) -> bool {
        let renaming = self
            .library_confirmation
            .is_some_and(|request| request.kind == LibraryConfirmationKind::Rename);
        if !renaming || !rename_draft_acceptable(&value) {
            return false;
        }
        self.library_rename_draft = value;
        self.library_rename_edited = true;
        true
    }

    /// Closes the dialog and its trap; focus returns to the control that
    /// opened it, whose identifier survives the Archive/Unarchive flip.
    fn close_library_confirmation(&mut self) {
        self.library_rename_draft.clear();
        self.library_rename_edited = false;
        if self.library_confirmation.take().is_some() {
            self.ui_focus.close_modal();
        }
    }

    /// Submits exactly the open request. A request that does not match what is
    /// open is stale and ignored. A refusal leaves the dialog open, so the user
    /// sees why nothing happened rather than a dialog that silently vanished.
    fn submit_library_confirmation(
        &mut self,
        request: LibraryConfirmationRequest,
    ) -> Result<(), crate::app::client_runtime::ClientRuntimeError> {
        if self.library_confirmation != Some(request) {
            log::warn!("ignored a stale Library confirmation: {request:?}");
            return Ok(());
        }
        let result = match request.kind {
            // Validated again here: Enter in the field submits whatever the
            // draft is, and a refusal must keep the draft for the user to fix.
            LibraryConfirmationKind::Rename => {
                match crate::workshop::store::SlotName::new(self.library_rename_draft.as_str()) {
                    Ok(name) => self.runtime.rename_library_slot(request.slot, name),
                    Err(error) => {
                        log::info!("Library rename refused: {error}");
                        return Ok(());
                    }
                }
            }
            LibraryConfirmationKind::Archive => self.runtime.archive_library_slot(request.slot),
            LibraryConfirmationKind::Unarchive => self.runtime.unarchive_library_slot(request.slot),
        };
        if result.is_ok() {
            self.close_library_confirmation();
        }
        result
    }

    /// Every Library intent, each to its route. `activate` refuses a disabled
    /// control, so an intent arrives here only when the model's gate allowed
    /// it; the runtime re-checks each gate and refuses with a logged error.
    fn apply_library_intent(&mut self, intent: LibraryUiIntent) {
        let result = match intent {
            LibraryUiIntent::Close => {
                self.runtime.close_library();
                Ok(())
            }
            LibraryUiIntent::RefreshSlots => self.runtime.refresh_library_slots(),
            LibraryUiIntent::RetrySlotRequest => self.runtime.retry_library_slot_request(),
            LibraryUiIntent::CancelSlotRequest => self.runtime.cancel_library_slot_request(),
            LibraryUiIntent::RenameSlot { slot } => {
                self.open_library_confirmation(LibraryConfirmationRequest {
                    kind: LibraryConfirmationKind::Rename,
                    slot,
                });
                Ok(())
            }
            LibraryUiIntent::ArchiveSlot { slot } => {
                self.open_library_confirmation(LibraryConfirmationRequest {
                    kind: LibraryConfirmationKind::Archive,
                    slot,
                });
                Ok(())
            }
            LibraryUiIntent::UnarchiveSlot { slot } => {
                self.open_library_confirmation(LibraryConfirmationRequest {
                    kind: LibraryConfirmationKind::Unarchive,
                    slot,
                });
                Ok(())
            }
            LibraryUiIntent::CancelConfirmation => {
                self.close_library_confirmation();
                Ok(())
            }
            LibraryUiIntent::SubmitConfirmation(request) => {
                self.submit_library_confirmation(request)
            }
            LibraryUiIntent::SelectSlot(slot) => {
                self.selected_library_slot = Some(slot);
                // Compact has no docked panel: selecting a row is the one
                // gesture that reaches its actions.
                if self.platform_ui.as_ref().is_some_and(|frame| {
                    frame.layout.mode == crate::ui::workshop_layout::WorkshopLayoutMode::Compact
                }) {
                    self.library_view.sheet = Some(LibrarySheet::Actions);
                    self.pending_focus = Some(LibraryViewAction::CloseSheet.action_id());
                }
                Ok(())
            }
            LibraryUiIntent::OpenSlot { slot, generation } => {
                self.runtime.open_library_slot(slot, generation)
            }
            LibraryUiIntent::AcceptOpen => self.runtime.accept_library_open(),
            LibraryUiIntent::UseForContinue { slot, generation } => {
                self.runtime.use_library_slot_for_continue(slot, generation)
            }
            LibraryUiIntent::ExportSlot { slot, generation } => {
                self.runtime.export_library_slot(slot, generation)
            }
            LibraryUiIntent::AcceptExportRecovery => self.runtime.accept_library_export_recovery(),
            LibraryUiIntent::HandOffExport => self.runtime.hand_off_library_export(),
            LibraryUiIntent::ExportActiveArchive => self.runtime.export_active_workshop(),
            LibraryUiIntent::ExportActivePack => self.runtime.export_active_pack(),
            LibraryUiIntent::ImportPack => self.runtime.import_library_pack(),
            LibraryUiIntent::ImportArchive => self.runtime.import_library_archive(),
        };
        if let Err(error) = result {
            log::warn!("Library intent was rejected: {error}");
        }
    }

    fn apply_guide_action(&mut self, action: GuideAction) {
        match action {
            GuideAction::Open if self.player_guide.is_none() => {
                let previous = std::mem::replace(&mut self.ui_focus, FocusManager::new([]));
                self.player_guide =
                    Some((GuideLocation::for_screen(self.runtime.screen()), previous));
                self.interaction = InteractionController::default();
                self.input = InputState::default();
            }
            GuideAction::Show(location) => {
                if let Some((current, _)) = &mut self.player_guide {
                    *current = location;
                }
            }
            GuideAction::Close | GuideAction::MainMenu => {
                if let Some((_, previous)) = self.player_guide.take() {
                    self.ui_focus = previous;
                }
                if action == GuideAction::MainMenu {
                    self.active_creator = None;
                    self.pending_removal = None;
                    self.close_workshop_modal_focus();
                    if let Err(error) = self.runtime.return_to_main_menu() {
                        log::warn!("guide main-menu transition was rejected: {error}");
                    }
                }
            }
            GuideAction::Open => {}
        }
        self.last_frame = None;
        self.build_frame();
    }

    fn apply_shell_action(&mut self, action: ShellUiAction) {
        match action {
            ShellUiAction::Menu(route) => match self.runtime.select_menu_route(route) {
                Ok(ClientRuntimeEffect::None | ClientRuntimeEffect::CreditsOpened) => {
                    if route == MainMenuRoute::Settings {
                        self.runtime.classic_mut().open_settings();
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                Ok(ClientRuntimeEffect::QuitRequested) => self.request_durable_exit(),
                Err(error) => log::warn!("shell action was rejected: {error}"),
            },
            ShellUiAction::CloseSettings => {
                self.runtime.close_settings();
                self.runtime.classic_mut().close_settings();
            }
            ShellUiAction::CycleUiScale => {
                let next = match self.runtime.classic().preferences().ui_scale {
                    crate::preferences::UiScale::Percent85 => {
                        crate::preferences::UiScale::Percent100
                    }
                    crate::preferences::UiScale::Percent100 => {
                        crate::preferences::UiScale::Percent115
                    }
                    crate::preferences::UiScale::Percent115 => {
                        crate::preferences::UiScale::Percent130
                    }
                    crate::preferences::UiScale::Percent130 => {
                        crate::preferences::UiScale::Percent85
                    }
                };
                self.runtime
                    .classic_mut()
                    .handle_settings_action(crate::app::settings::SettingsAction::SetUiScale(next));
            }
            ShellUiAction::ToggleReducedMotion => {
                let reduced = self.runtime.classic().preferences().motion
                    != crate::presentation::MotionPreference::Reduced;
                self.runtime.classic_mut().handle_settings_action(
                    crate::app::settings::SettingsAction::SetMotion(if reduced {
                        crate::presentation::MotionPreference::Reduced
                    } else {
                        crate::presentation::MotionPreference::Full
                    }),
                );
            }
            ShellUiAction::ToggleHighContrast => {
                let enabled = !self.runtime.classic().preferences().high_contrast;
                self.runtime.classic_mut().handle_settings_action(
                    crate::app::settings::SettingsAction::SetHighContrast(enabled),
                );
            }
            ShellUiAction::DismissCredits => self.runtime.dismiss_credits(),
            ShellUiAction::DismissRecovery => self.runtime.dismiss_recovery(),
            ShellUiAction::ContinueRecovery => {
                if let Err(error) = self.runtime.select_menu_route(MainMenuRoute::Continue) {
                    log::warn!("recovered Continue was rejected: {error}");
                }
            }
            ShellUiAction::ReturnToMainMenu => {
                if let Err(error) = self.runtime.return_to_main_menu() {
                    log::warn!("main-menu transition was rejected: {error}");
                }
            }
        }
    }

    fn close_workshop_modal_focus(&mut self) {
        self.ui_focus.close_modal();
        self.workshop_view.reset_modal();
    }

    fn apply_workshop_intent(&mut self, intent: WorkshopUiIntent) {
        match intent {
            WorkshopUiIntent::OpenCreatorForm { tool, subject } => {
                self.workshop_view.reset_modal();
                if let Some(subject) = subject {
                    self.selected_workshop_entity = Some(subject);
                }
                let draft = self.runtime.workshop_snapshot().and_then(|snapshot| {
                    default_creator_batch(snapshot, self.selected_workshop_entity, tool)
                        .ok()
                        .and_then(|batch| CreatorDraft::from_batch(&snapshot.state, &batch).ok())
                });
                let editor = ActiveCreatorEditor {
                    tool,
                    draft,
                    editing_field: None,
                };
                let modal_order = editor.modal_order();
                self.active_creator = Some(editor);
                let _ = self.ui_focus.open_modal(modal_order);
            }
            WorkshopUiIntent::SelectEntity(entity) => {
                self.selected_workshop_entity = Some(entity);
                self.active_creator = None;
                self.close_workshop_modal_focus();
            }
            WorkshopUiIntent::Dispatch(action) => {
                let submitted_creator = matches!(action, WorkshopAction::Submit(_));
                if let Err(error) = self.runtime.enqueue_workshop_action(action) {
                    log::warn!("Workshop UI action was rejected: {error}");
                } else if submitted_creator {
                    self.active_creator = None;
                    self.close_workshop_modal_focus();
                }
            }
            WorkshopUiIntent::OpenRemovalConfirmation(entity) => {
                self.workshop_view.reset_modal();
                self.pending_removal = Some(entity);
                // The IDs belong to `ui::workshop::removal`; spelling them here again is
                // how a rename there would silently point the trap at nothing.
                let _ = self.ui_focus.open_modal(removal_modal_order());
            }
            WorkshopUiIntent::CloseRemovalConfirmation => {
                self.pending_removal = None;
                self.close_workshop_modal_focus();
            }
            WorkshopUiIntent::SetReducedMotion(enabled) => {
                self.update_workshop_presentation_preferences(Some(enabled), None);
            }
            WorkshopUiIntent::SetHighContrast(enabled) => {
                self.update_workshop_presentation_preferences(None, Some(enabled));
            }
            WorkshopUiIntent::EditCreatorField { field_id, cycle } => {
                if cycle
                    && let Some(editor) = &mut self.active_creator
                    && let Some(draft) = &mut editor.draft
                    && let Err(error) = draft.cycle(&field_id, 1)
                {
                    log::warn!("Workshop creator choice was rejected: {error}");
                }
                if let Some(editor) = &mut self.active_creator {
                    editor.editing_field = None;
                }
            }
            WorkshopUiIntent::CloseCreatorForm => {
                self.active_creator = None;
                self.close_workshop_modal_focus();
            }
            WorkshopUiIntent::OpenLibrary => match self.runtime.open_library() {
                Ok(()) => {
                    self.library_return_focus = Some(SemanticActionId::new(
                        crate::ui::workshop::WORKSHOP_LIBRARY_ACTION,
                    ));
                }
                Err(error) => log::warn!("Workshop Library entry was rejected: {error}"),
            },
            WorkshopUiIntent::ReturnToMainMenu => {
                self.active_creator = None;
                self.pending_removal = None;
                self.close_workshop_modal_focus();
                if let Err(error) = self.runtime.return_to_main_menu() {
                    log::warn!("Workshop main-menu transition was rejected: {error}");
                }
            }
        }
    }

    fn update_workshop_presentation_preferences(
        &mut self,
        reduced_motion: Option<bool>,
        high_contrast: Option<bool>,
    ) {
        self.runtime.classic_mut().open_settings();
        if let Some(enabled) = reduced_motion {
            self.runtime.classic_mut().handle_settings_action(
                crate::app::settings::SettingsAction::SetMotion(if enabled {
                    crate::presentation::MotionPreference::Reduced
                } else {
                    crate::presentation::MotionPreference::Full
                }),
            );
        }
        if let Some(enabled) = high_contrast {
            self.runtime.classic_mut().handle_settings_action(
                crate::app::settings::SettingsAction::SetHighContrast(enabled),
            );
        }
        self.runtime.classic_mut().close_settings();
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_semantic_actions();
        let now = Instant::now();
        let delta = self
            .last_frame
            .replace(now)
            .map_or(Duration::ZERO, |previous| {
                now.saturating_duration_since(previous)
            });
        let delta = if self.player_guide.is_some() {
            Duration::ZERO
        } else {
            delta
        };
        let _ = self.runtime.update(delta);
        self.poll_durable_exit();
        if self.runtime.screen() == ClientScreen::ClassicSector && self.player_guide.is_none() {
            self.runtime.classic_mut().advance(delta);
            self.poll_advisory();
            self.submit_pending_advisory();
        }
        self.build_frame();

        if self.quit_requested {
            event_loop.exit();
            return;
        }

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

    fn request_durable_exit(&mut self) {
        let requires_save = matches!(
            self.runtime.active_session(),
            ActiveSession::Workshop(session) if !session.continue_ready()
        );
        match self.durable_exit.request(requires_save) {
            DurableExitAction::ExitNow => self.quit_requested = true,
            DurableExitAction::PrepareWorkshop => {
                if let Err(error) = self.runtime.return_to_main_menu() {
                    log::warn!("Workshop durable exit preparation was rejected: {error}");
                }
            }
            DurableExitAction::Wait => {}
        }
    }

    fn poll_durable_exit(&mut self) {
        if !self.durable_exit.is_pending() {
            return;
        }
        let continue_ready = matches!(
            self.runtime.active_session(),
            ActiveSession::Workshop(session) if session.continue_ready()
        );
        match self.durable_exit.observe_workshop(continue_ready) {
            DurableExitAction::ExitNow => self.quit_requested = true,
            DurableExitAction::PrepareWorkshop => unreachable!("polling cannot start an exit"),
            DurableExitAction::Wait => {
                if let Err(error) = self.runtime.return_to_main_menu() {
                    log::warn!("Workshop durable exit preparation is still pending: {error}");
                }
            }
        }
    }

    pub(super) fn cancel_durable_exit(&mut self) -> bool {
        self.durable_exit.cancel()
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
        let scene = (self.runtime.screen() == ClientScreen::ClassicSector
            && self.runtime.classic().mode() == AppMode::Playing)
            .then(|| self.runtime.classic().scene_frame());
        let physical_size = gpu.physical_size();
        let render_frame = RenderFrame {
            logical_viewport: self.runtime.classic().viewport().to_array(),
            physical_target: [physical_size.width, physical_size.height],
            quality: self
                .runtime
                .classic()
                .presentation_preferences()
                .graphics_quality,
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
        let Some(request) = self.runtime.classic_mut().take_gpu_request() else {
            return;
        };
        let Some(advisory) = &mut self.gpu_advisory else {
            self.runtime
                .classic_mut()
                .fail_device_epoch("GPU advisory backend is unavailable");
            return;
        };
        if let Err(error) = advisory.submit(&request) {
            self.runtime
                .classic_mut()
                .fail_device_epoch(error.to_string());
            self.gpu_advisory = None;
        }
    }

    fn poll_advisory(&mut self) {
        let Some(advisory) = &mut self.gpu_advisory else {
            return;
        };
        match advisory.poll() {
            Ok(Some(completion)) => {
                self.runtime
                    .classic_mut()
                    .complete_gpu(completion.metadata, completion.result);
            }
            Ok(None) => {}
            Err(error) => {
                self.runtime
                    .classic_mut()
                    .fail_device_epoch(error.to_string());
                self.gpu_advisory = None;
            }
        }
    }
}

impl<S: ScenarioStore + 'static, P: PreferencesStore + 'static, W: WorkshopStore + 'static>
    ApplicationHandler<AppEvent> for App<S, P, W>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.runtime.classic_mut().on_resume();
        self.last_frame = None;
        if self.window.is_none() {
            let attributes = Window::default_attributes().with_title("NYON // Galaxy Workshop");
            #[cfg(not(target_arch = "wasm32"))]
            let attributes = attributes
                .with_inner_size(LogicalSize::new(1440.0, 900.0))
                .with_min_inner_size(LogicalSize::new(960.0, 600.0))
                .with_visible(false);
            #[cfg(target_arch = "wasm32")]
            let attributes = attributes
                .with_append(true)
                .with_prevent_default(true)
                .with_focusable(true);
            match event_loop.create_window(attributes) {
                Ok(window) => {
                    window.set_ime_allowed(true);
                    let window = Arc::new(window);
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        self.accesskit_adapter =
                            Some(accesskit_winit::Adapter::with_event_loop_proxy(
                                event_loop,
                                &window,
                                self.event_proxy.live(),
                            ));
                        window.set_visible(true);
                    }
                    self.window = Some(window);
                    self.update_viewport();
                }
                Err(error) => {
                    self.fail(event_loop, format!("window creation failed: {error}"));
                    return;
                }
            }
        }
        if !self.continue_bootstrap_started {
            self.continue_bootstrap_started = true;
            if let Err(error) = self.runtime.begin_continue_bootstrap() {
                log::warn!("Continue bootstrap could not start: {error}");
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.initialize_gpu(event_loop);
        #[cfg(target_arch = "wasm32")]
        self.initialize_gpu();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.runtime.classic_mut().on_suspend();
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
            #[cfg(not(target_arch = "wasm32"))]
            AppEvent::Accessibility(event) => {
                if self.window.as_ref().map(|window| window.id()) != Some(event.window_id) {
                    return;
                }
                match event.window_event {
                    accesskit_winit::WindowEvent::InitialTreeRequested => {
                        let update = self.semantic_adapter.full_tree_update();
                        if let Some(adapter) = &mut self.accesskit_adapter {
                            adapter.update_if_active(|| update);
                        }
                    }
                    accesskit_winit::WindowEvent::ActionRequested(request) => {
                        match request.action {
                            AccessibilityAction::Click => {
                                self.semantic_adapter
                                    .request_node_action(request.target_node);
                            }
                            AccessibilityAction::Focus => {
                                if let Some(action) = self
                                    .semantic_adapter
                                    .focus_action_for_node(request.target_node)
                                {
                                    self.ui_focus.request_focus(&action);
                                }
                            }
                            AccessibilityAction::SetValue => {
                                let value = match request.data {
                                    Some(AccessibilityActionData::Value(value)) => {
                                        Some(value.into_string())
                                    }
                                    Some(AccessibilityActionData::NumericValue(value)) => {
                                        Some(value.to_string())
                                    }
                                    _ => None,
                                };
                                if let (Some(action), Some(value)) = (
                                    self.semantic_adapter
                                        .action_for_node(request.target_node)
                                        .cloned(),
                                    value,
                                ) {
                                    self.apply_creator_semantic_value(&action, value);
                                }
                            }
                            _ => {}
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
                }
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
        #[cfg(not(target_arch = "wasm32"))]
        if let (Some(adapter), Some(window)) = (&mut self.accesskit_adapter, &self.window) {
            adapter.process_event(window, &event);
        }
        match event {
            WindowEvent::CloseRequested => self.request_durable_exit(),
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
                if self.player_guide.is_none()
                    && self.runtime.screen() == ClientScreen::ClassicSector
                    && let Some(editor) = self.runtime.classic_mut().editor_mut()
                {
                    editor.set_ime_preedit(text);
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if self.player_guide.is_none()
                    && self.runtime.screen() == ClientScreen::ClassicSector
                    && let Some(editor) = self.runtime.classic_mut().editor_mut()
                {
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
                if self.player_guide.is_none()
                    && self.runtime.screen() == ClientScreen::ClassicSector
                {
                    if pointer_active {
                        self.runtime.classic_mut().clear_hovered_world();
                    } else {
                        let hud_consumed =
                            self.runtime.classic().ui_frame().consumes_pointer(logical);
                        self.runtime
                            .classic_mut()
                            .set_scene_cursor(logical, hud_consumed);
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if self.player_guide.is_none()
                    && self.runtime.screen() == ClientScreen::ClassicSector
                {
                    self.runtime.classic_mut().clear_hovered_world();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = wheel_logical_delta(delta, self.window_scale_factor());
                self.input.add_wheel_delta(delta);
                if self.runtime.screen() == ClientScreen::GalaxyWorkshop {
                    self.scroll_workshop_view(delta.y);
                }
                if self.player_guide.is_none()
                    && self.runtime.screen() == ClientScreen::ClassicSector
                {
                    if let Some(editor) = self.runtime.classic_mut().editor_mut() {
                        editor.scroll_by(-delta.y);
                    } else if self.runtime.classic().mode() == AppMode::Playing
                        && !self
                            .runtime
                            .classic()
                            .ui_frame()
                            .consumes_pointer(self.input.cursor_logical)
                    {
                        self.runtime.classic_mut().zoom_camera(delta.y);
                    }
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

impl<S: ScenarioStore, P: PreferencesStore, W: WorkshopStore> App<S, P, W> {
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

pub(crate) const fn safe_client_diagnostic(code: ClientDiagnosticCode) -> &'static str {
    match code {
        ClientDiagnosticCode::Store => "Workshop storage is temporarily unavailable.",
        ClientDiagnosticCode::StoreProtocol => {
            "Workshop storage returned an unexpected recoverable result."
        }
        ClientDiagnosticCode::Catalog => "The built-in Workshop catalog did not validate.",
        ClientDiagnosticCode::Archive => "The selected Workshop archive did not validate.",
        ClientDiagnosticCode::ContinueUnavailable => {
            "No explicitly selected valid Workshop save is available."
        }
        ClientDiagnosticCode::WorkshopInactive => "Galaxy Workshop is not currently active.",
        ClientDiagnosticCode::ResidentSlot => {
            "Close the Workshop using that slot before archiving it."
        }
        ClientDiagnosticCode::RouteUnavailable => {
            "That product route is unavailable from the current screen."
        }
        ClientDiagnosticCode::StaleSave => {
            "That save changed before it opened. Refresh the list and try again."
        }
        ClientDiagnosticCode::Transfer => "The portable copy could not be handed off.",
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
mod library_frame_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod modal_lifecycle_tests;

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

    #[test]
    fn durable_exit_waits_for_continue_selection_and_can_be_cancelled() {
        let mut state = DurableExitState::default();
        assert_eq!(state.request(true), DurableExitAction::PrepareWorkshop);
        assert!(state.is_pending());
        assert_eq!(state.request(true), DurableExitAction::Wait);
        assert_eq!(state.observe_workshop(false), DurableExitAction::Wait);
        assert!(state.cancel());
        assert!(!state.is_pending());

        assert_eq!(state.request(true), DurableExitAction::PrepareWorkshop);
        assert_eq!(state.observe_workshop(true), DurableExitAction::ExitNow);
        assert!(!state.is_pending());
        assert_eq!(state.request(false), DurableExitAction::ExitNow);
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
