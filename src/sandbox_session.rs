//! A paused galaxy session the shell can step without entering Classic Sector
//! or Galaxy Workshop.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
    time::Duration,
};

use glam::Vec2;
use nyon_workshop_core::living::{
    LivingArchiveReplayStatusV2, LivingArchiveReplayV2, LivingBookmarkV2, LivingBranchIdV2,
    LivingCommandEnvelopeV2, LivingCommandModeV2, LivingCommandV2, LivingComparisonReplayV2,
    LivingComparisonV2, LivingEntityIdV2, LivingEventProvenanceV2, LivingGalaxyAuthorityV2,
    LivingGalaxyStateV2, LivingHistoryViewV2, LivingRevisionIdV2, LivingStepJobErrorV2,
    LivingStepJobV2, LivingStepPreparationV2, LivingStepProgressV2, LivingTickReceiptV2,
    LivingTickV2, begin_compare_living_v2, begin_redo_living_v2, begin_restore_living_view_v2,
    begin_undo_living_v2, encode_living_archive_v2,
};

use crate::living_audio::{AudioChannel, LivingAudioSettings};
use crate::living_camera::{LivingCameraLevel, LivingCameraScope, LivingCameraState};
use crate::living_chronicle::{ChronicleCategory, LivingChronicle, LivingChronicleFilter};
use crate::living_content::LivingContent;
use crate::living_creator::{LivingCreatorDraft, LivingCreatorTool};
use crate::living_navigation::{LivingEntityKind, LivingEntitySelection, navigator_rows};
use crate::living_store::{
    LivingGenerationV2, LivingPackDescriptorV2, LivingPackReferenceV2, LivingSlotDescriptorV2,
};
use nyon_workshop_core::living::LivingCatalogHashV2;

use crate::ui::accessibility::{
    SemanticActionId, SemanticNode, SemanticNodeId, SemanticRole, SemanticTree,
};
use crate::ui::platform::{
    PlatformBackground, PlatformControl, PlatformRect, PlatformSightedText, PlatformUiAction,
    PlatformUiFrame,
};
use crate::ui::workshop_layout::WorkshopLayout;
use crate::ui::{AtlasMetrics, FontWeight, workshop::InspectorTextKind};

/// Supported simulation speeds; wall time never enters canonical authority.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SandboxSpeed {
    #[default]
    Normal,
    Fast,
    Fastest,
}

impl SandboxSpeed {
    pub const fn multiplier(self) -> u32 {
        match self {
            Self::Normal => 1,
            Self::Fast => 4,
            Self::Fastest => 20,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SandboxUiAction {
    TogglePause,
    Step,
    Speed(SandboxSpeed),
    SelectWorld(LivingEntityIdV2),
    SelectEntity(LivingEntitySelection),
    NavigatorPicker,
    IntroPicker,
    IntroStart(bool),
    IntroCreate,
    ChroniclePicker,
    ChronicleOptions(bool),
    ChronicleInspect(Option<u16>),
    ChronicleCategory(Option<ChronicleCategory>),
    ChronicleFollowActor(bool),
    ChronicleFollowEntity(bool),
    ChroniclePause(ChronicleCategory),
    CameraPicker,
    CameraPan(i16, i16),
    CameraOrbit(i16, i16),
    CameraZoom(i16),
    FocusSelection,
    FitScope,
    ToggleCameraTool,
    InspectorPicker,
    NavigatorFilter(Option<LivingEntityKind>),
    NavigatorTypes,
    ToggleNavigatorGroup(LivingEntitySelection),
    NavigatorLeft(LivingEntitySelection),
    NavigatorRight(LivingEntitySelection),
    RevealSelection,
    InspectorPage(bool),
    DecisionPage(bool),
    OpenCreator(LivingCreatorTool),
    CreatorField(u16),
    CreatorTool(bool),
    CreatorPreview,
    CreatorScene(bool),
    CreatorFitPreview,
    SnapCreatorParent(LivingEntitySelection),
    CreatorApply,
    CreatorCancel,
    Save,
    SaveAndLeave,
    ListSaves,
    ListPacks,
    OpenPackLibrary,
    PackLibraryBack,
    SelectPack(LivingCatalogHashV2),
    SetPackArchived {
        hash: LivingCatalogHashV2,
        generation: LivingGenerationV2,
        archived: bool,
    },
    ReviewDeletePack {
        hash: LivingCatalogHashV2,
        generation: LivingGenerationV2,
    },
    ConfirmDeletePack {
        hash: LivingCatalogHashV2,
        generation: LivingGenerationV2,
    },
    OpenSave(u64),
    ArchiveSave(u64),
    RestoreSave(u64),
    DeleteSave(u64),
    ConfirmDeleteSave(u64),
    RenameSave(u64),
    ConfirmRenameSave(u64),
    CancelRenameSave,
    ContentPicker,
    SavesPage(bool),
    OpenContent(LivingContent),
    ExportGalaxy,
    PrepareExport,
    VerifyNotebookEvidence,
    ExportNotebook,
    ImportNotebook,
    ConfirmExport(u8),
    ExportCatalog,
    TransferPicker,
    ImportGalaxy,
    ImportCatalog,
    HistoryPicker,
    RestoreHistoryView(LivingHistoryViewV2),
    HistoryBranch(LivingBranchIdV2),
    HistoryRevision(Option<LivingRevisionIdV2>),
    HistoryRevisionDetails(LivingBranchIdV2, LivingRevisionIdV2),
    HistoryDetailsClose,
    RestoreRequestedHistory,
    ComparisonBranch {
        left: bool,
        branch: LivingBranchIdV2,
    },
    CompareRequestedHistory,
    Undo,
    Redo,
    RecordBookmark,
    RenameBookmark(u8),
    RecordComparison,
    NotebookPicker,
    ExperimentDetails(Option<u8>),
    RecordExperiment(u8),
    RestoreBookmark(u8),
    CompareBookmark(u8),
    ReturnPresent,
    BeginBranch,
    CancelHistory,
    NavigateGalaxy,
    NavigateSystem,
    NavigateWorld,
    AudioPicker,
    ToggleMute,
    CycleAudio(AudioChannel),
    ApplyImport,
    CancelImport,
}

struct StarterGuidance {
    branch: nyon_workshop_core::living::LivingBranchIdV2,
    world: LivingEntityIdV2,
    industry: LivingEntityIdV2,
    before: nyon_workshop_core::living::LivingInventoryV2,
    before_tick: u64,
    blocked: Option<(
        LivingTickReceiptV2,
        nyon_workshop_core::living::LivingExplainedEventV2,
    )>,
    recovered: Option<(
        LivingTickReceiptV2,
        nyon_workshop_core::living::LivingExplainedEventV2,
        nyon_workshop_core::living::LivingInventoryV2,
    )>,
}

struct CreatorPreview {
    envelope: LivingCommandEnvelopeV2,
    branch: nyon_workshop_core::living::LivingBranchIdV2,
    projected: LivingGalaxyStateV2,
    message: Vec<String>,
    fork: bool,
}

/// A hold belongs to exactly one resident session and can only be ended once.
pub struct SandboxHold {
    scope: Rc<()>,
    id: u64,
}

impl SandboxHold {
    pub(crate) fn input_serial(&self) -> u64 {
        self.id
    }
}

const PACE_WINDOW_NANOS: u128 = 1_000_000_000;
const TICK_NANOS: u128 = 100_000_000;

#[derive(Clone, Copy, Default)]
struct PublicationSample {
    age: u128,
    count: u64,
}

/// Exact trailing (now - one second, now] publication counts. Transport admits
/// at most 200 ticks per second plus 20 carried requests and one fractional
/// request, so 256 slots suffice regardless of redraw frequency. Equal-time
/// publications share a slot; elapsed time comes exclusively from update.
struct PublicationPace {
    samples: [PublicationSample; 256],
    elapsed: u128,
    limited_age: Option<u128>,
}

impl Default for PublicationPace {
    fn default() -> Self {
        Self {
            samples: [PublicationSample::default(); 256],
            elapsed: 0,
            limited_age: None,
        }
    }
}

impl PublicationPace {
    fn record(&mut self, elapsed: Duration, committed: u64, limited: bool) -> Option<f64> {
        let nanos = elapsed.as_nanos();
        self.elapsed = self.elapsed.saturating_add(nanos).min(PACE_WINDOW_NANOS);
        for sample in &mut self.samples {
            sample.age = sample.age.saturating_add(nanos);
            if sample.age >= PACE_WINDOW_NANOS {
                *sample = PublicationSample::default();
            }
        }
        self.limited_age = if limited {
            Some(0)
        } else {
            self.limited_age
                .map(|age| age.saturating_add(nanos))
                .filter(|age| *age < PACE_WINDOW_NANOS)
        };
        if committed != 0 {
            let index = self
                .samples
                .iter()
                .position(|sample| sample.count != 0 && sample.age == 0)
                .or_else(|| self.samples.iter().position(|sample| sample.count == 0))
                .expect("transport admission bounds publication samples below 256");
            let sample = &mut self.samples[index];
            sample.age = 0;
            sample.count = sample.count.saturating_add(committed);
        }
        (self.elapsed != 0 && self.limited_age.is_some()).then(|| {
            let count: f64 = self.samples.iter().map(|sample| sample.count as f64).sum();
            count * TICK_NANOS as f64 / self.elapsed as f64
        })
    }
}

/// The session installed when the main menu opens the galaxy sandbox.
pub struct SandboxSession {
    authority: LivingGalaxyAuthorityV2,
    paused: bool,
    speed: SandboxSpeed,
    debt_nanos: u128,
    due_ticks: u64,
    explicit_step: bool,
    step_job: Option<LivingStepJobV2>,
    last_step_visits: usize,
    suspended: bool,
    hold_scope: Rc<()>,
    holds: BTreeSet<u64>,
    next_hold: u64,
    fault: Option<String>,
    limited_speed: Option<f64>,
    publication_pace: PublicationPace,
    selected_world: Option<LivingEntityIdV2>,
    selected_entity: Option<LivingEntitySelection>,
    selected_system: Option<LivingEntityIdV2>,
    selected_ancestors: Vec<LivingEntitySelection>,
    navigator_picker: bool,
    inspector_picker: bool,
    navigator_query: String,
    navigator_filter: Option<LivingEntityKind>,
    navigator_types: bool,
    navigator_expanded: BTreeSet<LivingEntitySelection>,
    navigator_reveal: bool,
    requested_focus: Option<SemanticActionId>,
    inspector_page: usize,
    receipts: VecDeque<LivingTickReceiptV2>,
    decisions: VecDeque<(
        LivingTickReceiptV2,
        nyon_workshop_core::living::LivingDecisionReceiptV2,
    )>,
    selected_decision: usize,
    creator: Option<LivingCreatorDraft>,
    creator_hold: Option<SandboxHold>,
    creator_message: Option<String>,
    creator_preview: Option<CreatorPreview>,
    creator_scene: bool,
    dirty: bool,
    content_picker: bool,
    content_seed: String,
    storage_status: String,
    saved_rows: Vec<LivingSlotDescriptorV2>,
    storage_busy: bool,
    saves_page: usize,
    history_picker: bool,
    history_configured: bool,
    history_branch: LivingBranchIdV2,
    history_revision: Option<LivingRevisionIdV2>,
    history_tick: String,
    comparison_left: LivingBranchIdV2,
    comparison_right: LivingBranchIdV2,
    comparison_tick: String,
    history_details: Option<(LivingBranchIdV2, LivingRevisionIdV2)>,
    history_job: Option<LivingArchiveReplayV2>,
    comparison_job: Option<LivingComparisonReplayV2>,
    comparison: Option<LivingComparisonV2>,
    present: Option<LivingBookmarkV2>,
    bookmarks: Vec<LivingBookmarkV2>,
    named_bookmarks: Vec<nyon_workshop_core::living::LivingNamedBookmarkV2>,
    notebook_experiments: Vec<nyon_workshop_core::living::LivingNotebookExperimentV2>,
    notebook_evidence: Option<nyon_workshop_core::living::LivingNotebookEvidenceV2>,
    notebook_replay_verified: BTreeSet<u64>,
    notebook_trade_completions: Vec<nyon_workshop_core::living::LivingNotebookTradeCompletionV2>,
    notebook_supply_completions: Vec<nyon_workshop_core::living::LivingNotebookSupplyCompletionV2>,
    notebook_corridor_observations:
        Vec<nyon_workshop_core::living::LivingNotebookCorridorObservationPairV2>,
    bookmark_label: String,
    notebook_picker: bool,
    notebook_details: Option<u8>,
    notebook_generation: u64,
    notebook_saved_generation: u64,
    bookmark_receipts: Vec<Option<LivingTickReceiptV2>>,
    bookmark_experiments: Vec<Option<u8>>,
    history_message: Option<String>,
    creator_branch: bool,
    transfer_capabilities: [bool; 3],
    delete_review: Option<LivingSlotDescriptorV2>,
    delete_name: String,
    living_continue_projection: Option<Option<u64>>,
    rename_slot: Option<u64>,
    rename_name: String,
    transfer_picker: bool,
    export_prepared: bool,
    export_status: String,
    export_confirmable: [bool; 3],
    library_picker: bool,
    pack_picker: bool,
    pack_rows: Vec<LivingPackDescriptorV2>,
    pack_references: BTreeMap<LivingCatalogHashV2, Vec<LivingPackReferenceV2>>,
    pack_projection_known: bool,
    pack_busy: bool,
    selected_pack: Option<LivingCatalogHashV2>,
    pack_delete_review: Option<LivingPackDescriptorV2>,
    pack_delete_hash: String,
    builtin_pack: LivingCatalogHashV2,
    scene_zoom: u8,
    cameras: RefCell<BTreeMap<LivingCameraScope, LivingCameraState>>,
    last_scene_bounds: Cell<PlatformRect>,
    last_creator_scene_top: Cell<Option<f32>>,
    intro_picker: bool,
    starter_guidance: Option<StarterGuidance>,
    intro_dismissed: bool,
    chronicle_picker: bool,
    chronicle_options: bool,
    chronicle_selected: Option<u16>,
    chronicle: LivingChronicle,
    chronicle_filter: LivingChronicleFilter,
    chronicle_pause: BTreeSet<ChronicleCategory>,
    camera_picker: bool,
    camera_tool: bool,
    audio_picker: bool,
    audio_settings: LivingAudioSettings,
    import_preview: Option<String>,
}

impl SandboxSession {
    /// Open the built-in galaxy, paused on its genesis boundary.
    pub fn open() -> Result<Self, String> {
        Ok(Self::from_authority(LivingContent::ValeConfluence.open()?))
    }

    pub fn from_authority(authority: LivingGalaxyAuthorityV2) -> Self {
        let selected_world = authority
            .state()
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.name.as_str() == "Aster Vale")
            .or_else(|| authority.state().worlds.as_slice().first())
            .map(|world| world.id);
        let selected_system = selected_world.and_then(|id| {
            authority
                .state()
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.id == id)
                .map(|world| world.system)
        });
        let history_branch = authority.branch();
        let mut session = Self {
            authority,
            paused: true,
            speed: SandboxSpeed::Normal,
            debt_nanos: 0,
            due_ticks: 0,
            explicit_step: false,
            step_job: None,
            last_step_visits: 0,
            suspended: false,
            hold_scope: Rc::new(()),
            holds: BTreeSet::new(),
            next_hold: 0,
            fault: None,
            limited_speed: None,
            publication_pace: PublicationPace::default(),
            selected_world,
            selected_entity: selected_world
                .map(|id| LivingEntitySelection::new(LivingEntityKind::World, id)),
            selected_system,
            selected_ancestors: selected_system
                .map(|id| LivingEntitySelection::new(LivingEntityKind::System, id))
                .into_iter()
                .collect(),
            navigator_picker: false,
            inspector_picker: false,
            navigator_query: String::new(),
            navigator_filter: None,
            navigator_types: false,
            navigator_expanded: BTreeSet::new(),
            navigator_reveal: false,
            requested_focus: None,
            inspector_page: 0,
            receipts: VecDeque::new(),
            decisions: VecDeque::new(),
            selected_decision: 0,
            creator: None,
            creator_hold: None,
            creator_message: None,
            creator_preview: None,
            creator_scene: false,
            dirty: true,
            content_picker: false,
            content_seed: "4E594F4E5F47414C".to_owned(),
            storage_status: "Unsaved changes".to_owned(),
            saved_rows: Vec::new(),
            storage_busy: false,
            saves_page: 0,
            history_picker: false,
            history_configured: false,
            history_branch,
            history_revision: None,
            history_tick: "0".into(),
            comparison_left: history_branch,
            comparison_right: history_branch,
            comparison_tick: "0".into(),
            history_details: None,
            history_job: None,
            comparison_job: None,
            comparison: None,
            present: None,
            bookmarks: Vec::new(),
            named_bookmarks: Vec::new(),
            notebook_experiments: Vec::new(),
            notebook_evidence: None,
            notebook_replay_verified: BTreeSet::new(),
            notebook_trade_completions: Vec::new(),
            notebook_supply_completions: Vec::new(),
            notebook_corridor_observations: Vec::new(),
            bookmark_label: "Recorded state".to_owned(),
            notebook_picker: false,
            notebook_details: None,
            notebook_generation: 0,
            notebook_saved_generation: 0,
            bookmark_receipts: Vec::new(),
            bookmark_experiments: Vec::new(),
            history_message: None,
            creator_branch: false,
            transfer_capabilities: [cfg!(target_arch = "wasm32"); 3],
            delete_review: None,
            delete_name: String::new(),
            living_continue_projection: None,
            rename_slot: None,
            rename_name: String::new(),
            transfer_picker: false,
            export_prepared: false,
            export_status: "Prepare an exact snapshot before exporting.".to_owned(),
            export_confirmable: [false; 3],
            library_picker: false,
            pack_picker: false,
            pack_rows: Vec::new(),
            pack_references: BTreeMap::new(),
            pack_projection_known: false,
            pack_busy: false,
            selected_pack: None,
            pack_delete_review: None,
            pack_delete_hash: String::new(),
            builtin_pack: nyon_workshop_core::living::living_core_pack_v2()
                .expect("embedded Living pack validates")
                .catalog_hash(),
            scene_zoom: 1,
            cameras: RefCell::new(BTreeMap::new()),
            last_scene_bounds: Cell::new(PlatformRect::from_xywh(0.0, 0.0, 1280.0, 720.0)),
            last_creator_scene_top: Cell::new(None),
            intro_picker: false,
            starter_guidance: None,
            intro_dismissed: false,
            chronicle_picker: false,
            chronicle_options: false,
            chronicle_selected: None,
            chronicle: LivingChronicle::default(),
            chronicle_filter: LivingChronicleFilter::default(),
            chronicle_pause: BTreeSet::new(),
            camera_picker: false,
            camera_tool: false,
            audio_picker: false,
            audio_settings: LivingAudioSettings::default(),
            import_preview: None,
        };
        let _ = session.seed_latest_boundary();
        session
    }

    // Archive replay regenerates the latest explanations. Older cache rows
    // remain unavailable; a projected receipt must match this exact state.
    fn seed_latest_boundary(&mut self) -> Result<(), String> {
        if let Some(receipt) = self.authority.last_receipt().cloned()
            && receipt.tick == self.state().tick
            && self.state().digest().ok() == Some(receipt.state_digest)
        {
            let decision = self
                .authority
                .last_decision()
                .filter(|decision| {
                    decision.tick == receipt.tick
                        && decision.receipt_digest == receipt.digest
                        && decision.state_after == receipt.state_digest
                })
                .cloned();
            self.record_chronicle(&receipt, decision.as_ref())?;
            if let Some(decision) = decision.filter(|decision| !decision.intents.is_empty()) {
                self.decisions.push_back((receipt.clone(), decision));
            }
            self.receipts.push_back(receipt);
        }
        Ok(())
    }

    pub(crate) fn presentation_scope(&self) -> Rc<()> {
        Rc::clone(&self.hold_scope)
    }

    /// Modal operation context excludes edited text and private preview state:
    /// editing then Preview remains ordered within the same owner.
    pub(crate) fn presentation_input_context(&self) -> String {
        if let Some(row) = &self.pack_delete_review {
            return format!("pack-delete:{}:{}", hex(&row.hash.0), row.generation.0);
        }
        if self.notebook_picker {
            return format!("notebook:{:?}", self.notebook_details);
        }
        if self.pack_picker {
            return format!("pack-library:{:?}", self.selected_pack);
        }
        if let Some(row) = &self.delete_review {
            return format!("delete:{}:{:?}", row.id.0, row.generation);
        }
        if let Some(id) = self.rename_slot {
            return format!("rename:{id}");
        }
        format!(
            "detail:{:?}|creator:{}:branch:{}|navigator:{}:types:{}|inspector:{}|intro:{}|content:{}|history:{}|library:{}|audio:{}|camera:{}|chronicle:{}:options:{}|transfer:{}|import:{}",
            self.history_details,
            self.creator.is_some(),
            self.creator_branch,
            self.navigator_picker,
            self.navigator_types,
            self.inspector_picker,
            self.intro_picker,
            self.content_picker,
            self.history_picker,
            self.library_picker,
            self.audio_picker,
            self.camera_picker,
            self.chronicle_picker,
            self.chronicle_options,
            self.transfer_picker,
            self.import_preview.is_some(),
        )
    }

    /// Captured semantic input ownership: resident, held modal, and private
    /// creator draft. Presentation edits and preview retain the draft token.
    pub(crate) fn presentation_input_owner(&self) -> (Rc<()>, Option<u64>, Option<Rc<()>>) {
        (
            Rc::clone(&self.hold_scope),
            self.creator_hold.as_ref().map(|hold| hold.id),
            self.creator
                .as_ref()
                .map(LivingCreatorDraft::input_identity),
        )
    }

    pub fn open_library(&mut self) -> Result<(), String> {
        self.notebook_picker = false;
        self.notebook_details = None;
        if !self.creator_modal_open() {
            self.creator_hold = Some(self.begin_hold()?);
        }
        self.creator = None;
        self.content_picker = false;
        self.history_picker = false;
        self.audio_picker = false;
        self.rename_slot = None;
        self.camera_picker = false;
        self.chronicle_picker = false;
        self.intro_picker = false;
        self.navigator_picker = false;
        self.inspector_picker = false;
        self.transfer_picker = false;
        self.library_picker = true;
        self.inspector_page = 0;
        Ok(())
    }
    pub fn scene_frame(
        &self,
        bounds: PlatformRect,
    ) -> crate::presentation::living::LivingSceneFrame {
        self.last_scene_bounds.set(bounds);
        self.ensure_camera_scope(bounds);
        let mut frame =
            crate::presentation::living::LivingSceneFrame::extract(self.state(), bounds);
        frame.selected_world = self.selected_world;
        frame.selected_entity = self.selected_entity();
        frame.apply_camera(bounds, self.camera_scope(), self.camera_state());
        if let Some(preview) = &self.creator_preview {
            let cursor = self.authority.published_cursor();
            if preview.branch == self.authority.branch()
                && preview.envelope.expected_tick == cursor.tick
                && preview.envelope.expected_committed_revision == cursor.committed_revision
                && preview.envelope.expected_pending_sequence == cursor.pending_sequence
            {
                use crate::presentation::living::{LivingGhostKind as G, LivingGhostMarker};
                let mut candidate = crate::presentation::living::LivingSceneFrame::extract(
                    &preview.projected,
                    bounds,
                );
                candidate.apply_camera(bounds, self.camera_scope(), self.camera_state());
                // Camera clipping cannot determine entity creation or removal.
                let prior_ids = navigator_rows(self.state())
                    .into_iter()
                    .map(|row| row.selection)
                    .collect::<BTreeSet<_>>();
                let next_ids = navigator_rows(&preview.projected)
                    .into_iter()
                    .map(|row| row.selection)
                    .collect::<BTreeSet<_>>();
                let command_json =
                    serde_json::to_value(&preview.envelope.command).unwrap_or_default();
                let mut referenced = BTreeSet::new();
                collect_preview_references(&command_json, &mut referenced);
                for marker in candidate.entities.iter().filter(|marker| marker.visible) {
                    let prior = prior_ids.contains(&marker.selection);
                    let kind = if !prior {
                        Some(G::Added)
                    } else if referenced.contains(&marker.selection.id)
                        && crate::living_navigation::entity_details(
                            self.state(),
                            self.authority.catalog(),
                            marker.selection,
                        ) != crate::living_navigation::entity_details(
                            &preview.projected,
                            self.authority.catalog(),
                            marker.selection,
                        )
                    {
                        Some(G::Changed)
                    } else {
                        None
                    };
                    if let Some(kind) = kind {
                        frame.ghosts.push(LivingGhostMarker {
                            selection: marker.selection,
                            position: marker.position,
                            kind,
                        });
                    }
                }
                for marker in frame.entities.iter().filter(|marker| marker.visible) {
                    if !next_ids.contains(&marker.selection) {
                        frame.ghosts.push(LivingGhostMarker {
                            selection: marker.selection,
                            position: marker.position,
                            kind: G::Removed,
                        });
                    }
                }
            }
        }
        frame
    }
    pub fn camera_scope(&self) -> LivingCameraScope {
        let world = self.selected_world.filter(|id| {
            self.state()
                .worlds
                .as_slice()
                .iter()
                .any(|world| world.id == *id)
        });
        let system = self
            .selected_system
            .filter(|id| {
                self.state()
                    .systems
                    .as_slice()
                    .iter()
                    .any(|system| system.id == *id)
            })
            .or_else(|| {
                world.and_then(|id| {
                    self.state()
                        .worlds
                        .as_slice()
                        .iter()
                        .find(|world| world.id == id)
                        .map(|world| world.system)
                })
            });
        if self.scene_zoom >= 8 && world.is_some() {
            LivingCameraScope {
                level: LivingCameraLevel::World,
                entity: world,
            }
        } else if self.scene_zoom >= 3 && system.is_some() {
            LivingCameraScope {
                level: LivingCameraLevel::System,
                entity: system,
            }
        } else {
            LivingCameraScope {
                level: LivingCameraLevel::Galaxy,
                entity: None,
            }
        }
    }
    /// First visible projection fits the scope once. Later renders, resizes and
    /// autonomous growth retain presentation framing; authority is untouched.
    fn ensure_camera_scope(&self, bounds: PlatformRect) {
        if !bounds.min.is_finite()
            || !bounds.max.is_finite()
            || bounds.width() <= 0.0
            || bounds.height() <= 0.0
        {
            return;
        }
        let scope = self.camera_scope();
        self.cameras.borrow_mut().entry(scope).or_insert_with(|| {
            crate::presentation::living::LivingSceneFrame::fitted_camera(
                self.state(),
                bounds,
                scope,
            )
        });
    }

    pub fn camera_state(&self) -> LivingCameraState {
        self.cameras
            .borrow()
            .get(&self.camera_scope())
            .copied()
            .unwrap_or_default()
    }
    pub const fn camera_tool(&self) -> bool {
        self.camera_tool
    }
    pub fn camera_pan(&mut self, delta: Vec2) -> bool {
        if !delta.is_finite() {
            return false;
        }
        self.prune_camera_scopes();
        self.ensure_camera_scope(self.last_scene_bounds.get());
        let scope = self.camera_scope();
        self.cameras.get_mut().entry(scope).or_default().pan(delta)
    }
    pub fn camera_orbit(&mut self, degrees: Vec2) -> bool {
        if !degrees.is_finite() {
            return false;
        }
        self.prune_camera_scopes();
        self.ensure_camera_scope(self.last_scene_bounds.get());
        let scope = self.camera_scope();
        self.cameras
            .get_mut()
            .entry(scope)
            .or_default()
            .orbit(degrees)
    }
    pub fn camera_zoom(&mut self, factor: f32, anchor: Vec2, bounds: PlatformRect) -> bool {
        if !bounds.min.is_finite()
            || !bounds.max.is_finite()
            || bounds.width() <= 0.0
            || bounds.height() <= 0.0
            || !factor.is_finite()
            || factor <= 0.0
            || !anchor.is_finite()
        {
            return false;
        }
        self.prune_camera_scopes();
        self.ensure_camera_scope(bounds);
        let scope = self.camera_scope();
        self.cameras
            .get_mut()
            .entry(scope)
            .or_default()
            .zoom_at(factor, anchor - (bounds.min + bounds.max) * 0.5)
    }
    pub fn focus_camera_selection(&mut self) -> bool {
        let Some(selection) = self.selected_entity() else {
            return false;
        };
        self.scene_zoom = match selection.kind {
            LivingEntityKind::Civilization => 1,
            LivingEntityKind::System
            | LivingEntityKind::Star
            | LivingEntityKind::Lane
            | LivingEntityKind::Hazard => 3,
            _ => 8,
        };
        self.prune_camera_scopes();
        let scope = self.camera_scope();
        let state = self.cameras.get_mut().entry(scope).or_default();
        state.focus_target = Some(selection);
        state.pan = Vec2::ZERO;
        state.zoom = 1.0;
        true
    }
    /// Reveals the validated private candidate without changing live selection or authority.
    pub fn fit_creator_preview(&mut self) -> Result<(), String> {
        let preview = self
            .creator_preview
            .as_ref()
            .ok_or("Preview the creator draft before fitting its changes")?;
        let cursor = self.authority.published_cursor();
        if preview.branch != self.authority.branch()
            || preview.envelope.expected_tick != cursor.tick
            || preview.envelope.expected_committed_revision != cursor.committed_revision
            || preview.envelope.expected_pending_sequence != cursor.pending_sequence
        {
            return Err("Creator preview is stale; preview the current state again".into());
        }
        let bounds = self.last_scene_bounds.get();
        let before = crate::presentation::living::LivingSceneFrame::extract(self.state(), bounds);
        let after =
            crate::presentation::living::LivingSceneFrame::extract(&preview.projected, bounds);
        let mut points = before
            .entities
            .iter()
            .chain(&after.entities)
            .map(|marker| marker.position);
        let Some(first) = points.next() else {
            return Err("The preview contains no visible entities to fit".into());
        };
        let (mut min, mut max) = (first, first);
        for point in points {
            min = min.min(point);
            max = max.max(point);
        }
        let mut visible = bounds;
        visible.max.y = self
            .last_creator_scene_top
            .get()
            .map_or(bounds.max.y - 192.0, |top| top - 8.0)
            .clamp(bounds.min.y + 1.0, bounds.max.y);
        let available = (visible.max - visible.min - Vec2::splat(64.0)).max(Vec2::ONE);
        let extent = (max - min).max(Vec2::ONE);
        let zoom = (available.x / extent.x)
            .min(available.y / extent.y)
            .clamp(0.25, 8.0);
        let camera = LivingCameraState {
            zoom,
            pan: (((visible.min + visible.max) - (bounds.min + bounds.max)) * 0.5
                + ((bounds.min + bounds.max) - (min + max)) * (0.5 * zoom))
                .clamp(Vec2::splat(-4096.0), Vec2::splat(4096.0)),
            ..Default::default()
        };
        self.scene_zoom = 1;
        self.prune_camera_scopes();
        let scope = self.camera_scope();
        self.cameras.get_mut().insert(scope, camera);
        self.creator_scene = true;
        Ok(())
    }

    pub fn fit_camera_scope(&mut self) {
        self.prune_camera_scopes();
        let scope = self.camera_scope();
        let camera = crate::presentation::living::LivingSceneFrame::fitted_camera(
            self.state(),
            self.last_scene_bounds.get(),
            scope,
        );
        self.cameras.get_mut().insert(scope, camera);
    }

    pub fn set_audio_projection(&mut self, settings: LivingAudioSettings) {
        self.audio_settings = settings;
    }
    pub fn import_preview_open(&self) -> bool {
        self.import_preview.is_some()
    }
    pub fn set_import_preview(&mut self, summary: Option<&str>) {
        if summary.is_some()
            && !self.creator_modal_open()
            && let Ok(hold) = self.begin_hold()
        {
            self.creator_hold = Some(hold);
        }
        if summary.is_some() {
            self.notebook_picker = false;
            self.notebook_details = None;
            self.rename_slot = None;
            self.camera_picker = false;
            self.chronicle_picker = false;
            self.intro_picker = false;
            self.navigator_picker = false;
            self.inspector_picker = false;
            self.transfer_picker = false;
            self.creator = None;
            self.creator_preview = None;
            self.content_picker = false;
            self.history_picker = false;
            self.audio_picker = false;
        }
        self.import_preview = summary.map(str::to_owned);
        if summary.is_none()
            && !self.creator_modal_open()
            && let Some(hold) = self.creator_hold.take()
        {
            self.end_hold(hold);
        }
    }
    pub fn set_export_projection(&mut self, prepared: bool, status: &str, confirmable: [bool; 3]) {
        self.export_prepared = prepared;
        self.export_status = status.to_owned();
        self.export_confirmable = confirmable;
    }
    pub fn set_transfer_capabilities(&mut self, export: bool, import: bool, catalog: bool) {
        self.transfer_capabilities = [export, import, catalog];
    }
    pub fn bookmarks(&self) -> &[LivingBookmarkV2] {
        &self.bookmarks
    }
    pub fn bookmark_experiment(&self, index: usize) -> Option<&'static str> {
        self.bookmark_experiments
            .get(index)
            .copied()
            .flatten()
            .and_then(|index| EXPERIMENTS.get(usize::from(index)).copied())
    }
    pub fn bookmark_receipt(&self, index: usize) -> Option<&LivingTickReceiptV2> {
        self.bookmark_receipts.get(index).and_then(Option::as_ref)
    }
    pub fn comparison(&self) -> Option<&LivingComparisonV2> {
        self.comparison.as_ref()
    }

    pub fn set_storage_projection(
        &mut self,
        status: &str,
        rows: &[LivingSlotDescriptorV2],
        busy: bool,
    ) {
        self.storage_status = status.to_owned();
        self.saved_rows = rows.to_vec();
        self.storage_busy = busy;
    }

    pub fn authority(&self) -> &LivingGalaxyAuthorityV2 {
        &self.authority
    }

    pub fn install_authority(&mut self, authority: LivingGalaxyAuthorityV2) {
        *self = Self::from_authority(authority);
    }

    pub fn export_bytes(&self) -> Result<Vec<u8>, String> {
        encode_living_archive_v2(&self.authority).map_err(|error| error.to_string())
    }

    pub fn bookmark(&self) -> Result<LivingBookmarkV2, String> {
        self.authority.bookmark().map_err(|error| error.to_string())
    }

    pub const fn dirty(&self) -> bool {
        self.dirty || self.notebook_generation != self.notebook_saved_generation
    }

    pub const fn notebook_generation(&self) -> u64 {
        self.notebook_generation
    }
    pub fn mark_notebook_unsaved(&mut self) -> Result<(), String> {
        self.notebook_generation = self.next_notebook_generation()?;
        Ok(())
    }
    pub fn mark_notebook_saved(&mut self, expected: u64) -> bool {
        if expected == self.notebook_generation {
            self.notebook_saved_generation = expected;
            true
        } else {
            false
        }
    }
    fn next_notebook_generation(&self) -> Result<u64, String> {
        self.notebook_generation
            .checked_add(1)
            .ok_or_else(|| "notebook generation capacity is exhausted".to_owned())
    }

    pub fn mark_saved(&mut self, bookmark: LivingBookmarkV2) -> bool {
        if self.bookmark().ok() == Some(bookmark) {
            self.dirty = false;
            true
        } else {
            false
        }
    }

    pub fn paused(&self) -> bool {
        self.paused
            || self.suspended
            || !self.holds.is_empty()
            || self.fault.is_some()
            || self.authority.is_viewing_history()
    }

    pub fn state(&self) -> &LivingGalaxyStateV2 {
        self.authority.state()
    }

    pub fn tick(&self) -> u64 {
        self.authority.state().tick.0
    }

    pub const fn speed(&self) -> SandboxSpeed {
        self.speed
    }

    pub fn set_speed(&mut self, speed: SandboxSpeed) {
        self.speed = speed;
        self.cancel_step();
    }

    pub fn toggle_pause(&mut self) {
        if self.authority.is_viewing_history() {
            return;
        }
        self.set_paused(!self.paused);
    }

    pub fn set_paused(&mut self, paused: bool) {
        if !paused && self.authority.is_viewing_history() {
            return;
        }
        self.paused = paused;
        self.cancel_step();
    }

    pub fn set_suspended(&mut self, suspended: bool) {
        if self.suspended != suspended {
            self.suspended = suspended;
            self.cancel_step();
        }
    }

    pub fn begin_hold(&mut self) -> Result<SandboxHold, String> {
        let id = self.next_hold;
        self.next_hold = id.checked_add(1).ok_or("session hold capacity exhausted")?;
        self.holds.insert(id);
        self.cancel_step();
        Ok(SandboxHold {
            scope: Rc::clone(&self.hold_scope),
            id,
        })
    }

    pub fn end_hold(&mut self, hold: SandboxHold) -> bool {
        if !Rc::ptr_eq(&hold.scope, &self.hold_scope) {
            return false;
        }
        let removed = self.holds.remove(&hold.id);
        if removed {
            self.cancel_step();
        }
        removed
    }

    pub fn apply_action(&mut self, action: SandboxUiAction) -> Result<(), String> {
        if self.import_preview.is_some()
            && matches!(
                action,
                SandboxUiAction::OpenCreator(_)
                    | SandboxUiAction::IntroPicker
                    | SandboxUiAction::ChroniclePicker
                    | SandboxUiAction::CameraPicker
                    | SandboxUiAction::NavigatorPicker
                    | SandboxUiAction::InspectorPicker
                    | SandboxUiAction::RevealSelection
                    | SandboxUiAction::ContentPicker
                    | SandboxUiAction::AudioPicker
                    | SandboxUiAction::HistoryPicker
                    | SandboxUiAction::NotebookPicker
                    | SandboxUiAction::BeginBranch
            )
        {
            return Err("apply or cancel the imported galaxy preview first".to_owned());
        }
        if self.notebook_picker
            && matches!(
                action,
                SandboxUiAction::HistoryPicker
                    | SandboxUiAction::OpenCreator(_)
                    | SandboxUiAction::ContentPicker
                    | SandboxUiAction::TransferPicker
                    | SandboxUiAction::CameraPicker
                    | SandboxUiAction::NavigatorPicker
                    | SandboxUiAction::InspectorPicker
                    | SandboxUiAction::ChroniclePicker
                    | SandboxUiAction::IntroPicker
                    | SandboxUiAction::AudioPicker
            )
        {
            self.apply_action(SandboxUiAction::CreatorCancel)?;
        }
        match action {
            SandboxUiAction::IntroPicker => {
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.creator_hold = Some(self.begin_hold()?);
                self.intro_picker = true;
                self.inspector_page = 0;
            }
            SandboxUiAction::IntroStart(run) => {
                self.intro_dismissed = true;
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.set_paused(!run);
            }
            SandboxUiAction::IntroCreate => {
                let world = self.starter_guidance.as_ref().map(|guide| guide.world);
                self.intro_dismissed = true;
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.set_paused(true);
                if let Some(world) = world.filter(|id| {
                    self.state()
                        .worlds
                        .as_slice()
                        .iter()
                        .any(|world| world.id == *id)
                }) {
                    self.select_entity(LivingEntitySelection::new(LivingEntityKind::World, world))?;
                    self.apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::Facility))?;
                    self.creator_message = Some("Suggested experiment: add a Solar Array on Aster Vale. Review its creator override and watch authenticated production facts; success is never timed.".to_owned());
                } else {
                    self.apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))?;
                }
            }
            SandboxUiAction::ChroniclePicker => {
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.creator_hold = Some(self.begin_hold()?);
                self.chronicle_picker = true;
                self.chronicle_options = false;
                self.chronicle_selected = None;
                self.inspector_page = 0;
            }
            SandboxUiAction::ChronicleInspect(index) => {
                self.chronicle_selected = index;
                self.inspector_page = 0;
            }
            SandboxUiAction::ChronicleOptions(options) => {
                self.chronicle_selected = None;
                self.chronicle_options = options;
                self.inspector_page = 0;
            }
            SandboxUiAction::ChronicleCategory(category) => {
                self.chronicle_filter.category = category;
                self.inspector_page = 0;
            }
            SandboxUiAction::ChronicleFollowActor(enabled) => {
                self.chronicle_filter.actor = enabled
                    .then(|| self.selected_entity())
                    .flatten()
                    .filter(|selection| selection.kind == LivingEntityKind::Civilization)
                    .map(|selection| selection.id);
                self.inspector_page = 0;
            }
            SandboxUiAction::ChronicleFollowEntity(enabled) => {
                self.chronicle_filter.entity = enabled
                    .then(|| self.selected_entity())
                    .flatten()
                    .map(|selection| selection.id);
                self.inspector_page = 0;
            }
            SandboxUiAction::ChroniclePause(category) => {
                if !self.chronicle_pause.remove(&category) {
                    self.chronicle_pause.insert(category);
                }
                self.chronicle
                    .set_pause_categories(self.chronicle_pause.iter().copied());
            }
            SandboxUiAction::CameraPicker => {
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.creator_hold = Some(self.begin_hold()?);
                self.camera_picker = true;
                self.inspector_page = 0;
            }
            SandboxUiAction::CameraPan(x, y) => {
                self.camera_pan(Vec2::new(f32::from(x), f32::from(y)));
            }
            SandboxUiAction::CameraOrbit(x, y) => {
                self.camera_orbit(Vec2::new(f32::from(x), f32::from(y)));
            }
            SandboxUiAction::CameraZoom(percent) => {
                self.prune_camera_scopes();
                self.ensure_camera_scope(self.last_scene_bounds.get());
                let scope = self.camera_scope();
                self.cameras
                    .get_mut()
                    .entry(scope)
                    .or_default()
                    .zoom_at(1.0 + f32::from(percent) / 100.0, Vec2::ZERO);
            }
            SandboxUiAction::ToggleCameraTool => {
                self.camera_tool = !self.camera_tool;
            }
            SandboxUiAction::FocusSelection => {
                self.focus_camera_selection();
            }
            SandboxUiAction::FitScope => self.fit_camera_scope(),
            SandboxUiAction::CreatorFitPreview => self.fit_creator_preview()?,
            SandboxUiAction::NavigateGalaxy => self.scene_zoom = 1,
            SandboxUiAction::NavigateSystem => self.scene_zoom = 3,
            SandboxUiAction::NavigateWorld => self.scene_zoom = 8,
            SandboxUiAction::TogglePause => {
                if self.authority.is_viewing_history() {
                    return Err(
                        "Viewing history: Return to present or Branch here before running".into(),
                    );
                }
                self.toggle_pause();
            }
            SandboxUiAction::Step => {
                if self.authority.is_viewing_history() {
                    return Err(
                        "Viewing history: Return to present or Branch here before stepping".into(),
                    );
                }
                if !self.paused || self.suspended || !self.holds.is_empty() {
                    return Err("pause the galaxy before stepping".to_owned());
                }
                if let Some(fault) = &self.fault {
                    return Err(fault.clone());
                }
                // Each manual step starts its own observation; repeated input
                // while processing requests no additional tick.
                if !self.explicit_step {
                    self.publication_pace = PublicationPace::default();
                    self.limited_speed = None;
                }
                self.debt_nanos = 0;
                self.due_ticks = 1;
                self.explicit_step = true;
            }
            SandboxUiAction::Speed(speed) => self.set_speed(speed),
            SandboxUiAction::SelectWorld(world) => self.select_world(world)?,
            SandboxUiAction::SelectEntity(selection) => {
                self.select_entity(selection)?;
                if self.navigator_picker {
                    self.apply_action(SandboxUiAction::CreatorCancel)?;
                    self.apply_action(SandboxUiAction::InspectorPicker)?;
                }
            }
            SandboxUiAction::NavigatorPicker | SandboxUiAction::InspectorPicker => {
                let navigator = action == SandboxUiAction::NavigatorPicker;
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.creator_hold = Some(self.begin_hold()?);
                self.navigator_types = false;
                self.navigator_reveal = false;
                self.requested_focus = None;
                self.navigator_picker = navigator;
                self.inspector_picker = !navigator;
                self.inspector_page = 0;
            }
            SandboxUiAction::NavigatorTypes => {
                self.navigator_types = !self.navigator_types;
                self.inspector_page = 0;
                self.navigator_reveal = false;
            }
            SandboxUiAction::NavigatorFilter(kind) => {
                self.navigator_types = false;
                self.navigator_filter = kind;
                self.inspector_page = 0;
                self.navigator_reveal = false;
            }
            SandboxUiAction::NavigatorLeft(selection)
            | SandboxUiAction::NavigatorRight(selection) => {
                let rows = navigator_rows(self.state());
                let row = rows
                    .iter()
                    .find(|row| row.selection == selection)
                    .ok_or("navigator entity absent")?;
                let target = if matches!(action, SandboxUiAction::NavigatorLeft(_)) {
                    if self.navigator_expanded.remove(&selection) {
                        selection
                    } else {
                        row.parent.unwrap_or(selection)
                    }
                } else if rows.iter().any(|row| row.parent == Some(selection)) {
                    if self.navigator_expanded.insert(selection) {
                        selection
                    } else {
                        rows.iter()
                            .find(|row| row.parent == Some(selection))
                            .map_or(selection, |row| row.selection)
                    }
                } else {
                    selection
                };
                self.select_entity(target)?;
                // Reveal ancestors without reopening the selected node we just collapsed.
                let mut key = rows
                    .iter()
                    .find(|row| row.selection == target)
                    .and_then(|row| row.parent);
                while let Some(parent) = key {
                    self.navigator_expanded.insert(parent);
                    key = rows
                        .iter()
                        .find(|row| row.selection == parent)
                        .and_then(|row| row.parent);
                }
                self.navigator_types = false;
                self.navigator_query.clear();
                self.navigator_filter = None;
                self.navigator_reveal = true;
                self.requested_focus = Some(SemanticActionId::new(format!(
                    "sandbox.navigator.entity.{:?}.{}",
                    target.kind,
                    crate::living_navigation::entity_id(target.id)
                )));
            }
            SandboxUiAction::ToggleNavigatorGroup(selection) => {
                if !self.navigator_expanded.remove(&selection) {
                    self.navigator_expanded.insert(selection);
                }
                self.navigator_reveal = false;
            }
            SandboxUiAction::RevealSelection => {
                self.apply_action(SandboxUiAction::NavigatorPicker)?;
                self.navigator_types = false;
                self.navigator_query.clear();
                self.navigator_filter = None;
                let rows = navigator_rows(self.state());
                let mut key = self.selected_entity();
                while let Some(selection) = key {
                    self.navigator_expanded.insert(selection);
                    key = rows
                        .iter()
                        .find(|row| row.selection == selection)
                        .and_then(|row| row.parent);
                }
                self.navigator_reveal = true;
                self.requested_focus = self.selected_entity().map(|selection| {
                    SemanticActionId::new(format!(
                        "sandbox.navigator.entity.{:?}.{}",
                        selection.kind,
                        crate::living_navigation::entity_id(selection.id)
                    ))
                });
            }
            SandboxUiAction::ToggleMute
            | SandboxUiAction::CycleAudio(_)
            | SandboxUiAction::ApplyImport
            | SandboxUiAction::CancelImport
            | SandboxUiAction::VerifyNotebookEvidence
            | SandboxUiAction::PrepareExport
            | SandboxUiAction::ExportNotebook
            | SandboxUiAction::ImportNotebook
            | SandboxUiAction::ConfirmExport(_)
            | SandboxUiAction::ExportCatalog
            | SandboxUiAction::ExportGalaxy
            | SandboxUiAction::ImportGalaxy
            | SandboxUiAction::ImportCatalog
            | SandboxUiAction::Save
            | SandboxUiAction::SaveAndLeave
            | SandboxUiAction::ListSaves
            | SandboxUiAction::ArchiveSave(_)
            | SandboxUiAction::RestoreSave(_)
            | SandboxUiAction::ConfirmDeleteSave(_)
            | SandboxUiAction::ConfirmRenameSave(_)
            | SandboxUiAction::OpenSave(_)
            | SandboxUiAction::OpenContent(_) => {
                return Err("this action requires the Living storage coordinator".to_owned());
            }
            SandboxUiAction::DeleteSave(id) => {
                let descriptor = self
                    .saved_rows
                    .iter()
                    .find(|row| row.id.0 == id)
                    .filter(|row| row.archived)
                    .cloned()
                    .ok_or("Only an archived listed galaxy can enter permanent-deletion review")?;
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.creator_hold = Some(self.begin_hold()?);
                self.delete_review = Some(descriptor);
                self.delete_name.clear();
                self.inspector_page = 0;
            }
            SandboxUiAction::RenameSave(slot) => {
                let name = self
                    .saved_rows
                    .iter()
                    .find(|row| row.id.0 == slot)
                    .ok_or("saved galaxy is absent from the current Library listing")?
                    .name
                    .clone();
                if !self.creator_modal_open() {
                    self.creator_hold = Some(self.begin_hold()?);
                }
                self.creator = None;
                self.creator_scene = false;
                self.history_picker = false;
                self.content_picker = false;
                self.audio_picker = false;
                self.camera_picker = false;
                self.chronicle_picker = false;
                self.intro_picker = false;
                self.navigator_picker = false;
                self.inspector_picker = false;
                self.transfer_picker = false;
                self.library_picker = false;
                self.rename_slot = Some(slot);
                self.rename_name = name;
            }
            SandboxUiAction::CancelRenameSave => {
                self.rename_slot = None;
                self.library_picker = true;
            }
            SandboxUiAction::TransferPicker => {
                if !self.creator_modal_open() {
                    self.creator_hold = Some(self.begin_hold()?);
                }
                self.rename_slot = None;
                self.transfer_picker = true;
                self.creator = None;
                self.creator_scene = false;
                self.content_picker = false;
                self.history_picker = false;
                self.audio_picker = false;
                self.library_picker = false;
            }
            SandboxUiAction::AudioPicker => {
                if !self.creator_modal_open() {
                    self.creator_hold = Some(self.begin_hold()?);
                }
                self.rename_slot = None;
                self.camera_picker = false;
                self.chronicle_picker = false;
                self.intro_picker = false;
                self.navigator_picker = false;
                self.inspector_picker = false;
                self.transfer_picker = false;
                self.library_picker = false;
                self.audio_picker = true;
                self.creator = None;
                self.creator_scene = false;
                self.content_picker = false;
                self.history_picker = false;
            }
            SandboxUiAction::HistoryPicker => {
                if !self.creator_modal_open() {
                    self.creator_hold = Some(self.begin_hold()?);
                }
                self.rename_slot = None;
                self.camera_picker = false;
                self.chronicle_picker = false;
                self.intro_picker = false;
                self.navigator_picker = false;
                self.inspector_picker = false;
                self.transfer_picker = false;
                self.audio_picker = false;
                self.library_picker = false;
                if !self.history_configured {
                    self.history_branch = self.authority.branch();
                    self.history_revision = self.authority.published_cursor().committed_revision;
                    self.comparison_left = self.authority.branch();
                    self.comparison_right = self
                        .authority
                        .archived()
                        .first()
                        .map_or(self.authority.branch(), |branch| branch.branch());
                    self.history_tick = self.tick().to_string();
                    self.comparison_tick = self.tick().to_string();
                    self.history_configured = true;
                }
                self.history_picker = true;
                self.creator = None;
                self.creator_scene = false;
                self.content_picker = false;
                self.inspector_page = 0;
            }
            SandboxUiAction::ListPacks => {
                return Err("Pack storage requires the application Library adapter".into());
            }
            SandboxUiAction::OpenPackLibrary => {
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.creator_hold = Some(self.begin_hold()?);
                self.pack_picker = true;
                self.selected_pack = None;
                self.inspector_page = 0;
            }
            SandboxUiAction::PackLibraryBack => {
                self.selected_pack = None;
                self.pack_delete_review = None;
                self.pack_delete_hash.clear();
                self.inspector_page = 0;
            }
            SandboxUiAction::SelectPack(hash) => {
                self.pack_rows
                    .iter()
                    .find(|row| row.hash == hash)
                    .ok_or("Catalog pack is not in the current Library projection")?;
                self.selected_pack = Some(hash);
                self.pack_delete_review = None;
                self.inspector_page = 0;
            }
            SandboxUiAction::ReviewDeletePack { hash, generation } => {
                let row = self
                    .pack_rows
                    .iter()
                    .find(|row| row.hash == hash && row.generation == generation)
                    .ok_or("Catalog pack changed; refresh Library")?;
                if !row.archived {
                    return Err("Archive a catalog pack before reviewing permanent deletion".into());
                }
                self.pack_delete_review = Some(row.clone());
                self.pack_delete_hash.clear();
                self.selected_pack = Some(hash);
                self.inspector_page = 0;
            }
            SandboxUiAction::SetPackArchived { .. } | SandboxUiAction::ConfirmDeletePack { .. } => {
                return Err("Pack mutation requires the application Library adapter".into());
            }
            SandboxUiAction::HistoryBranch(branch) => {
                let revision = self
                    .retained_branch(branch)?
                    .published_cursor()
                    .committed_revision;
                self.history_branch = branch;
                self.history_revision = revision;
                self.history_details = None;
            }
            SandboxUiAction::HistoryRevision(revision) => {
                self.validate_history_revision(self.history_branch, revision)?;
                self.history_revision = revision;
            }
            SandboxUiAction::HistoryDetailsClose => {
                self.history_details = None;
                self.inspector_page = 0;
            }
            SandboxUiAction::HistoryRevisionDetails(branch, revision) => {
                self.validate_history_revision(branch, Some(revision))?;
                self.history_details = Some((branch, revision));
                self.inspector_page = 0;
            }
            SandboxUiAction::RestoreRequestedHistory => {
                let tick = canonical_history_tick(&self.history_tick)?;
                self.validate_history_revision(self.history_branch, self.history_revision)?;
                self.apply_action(SandboxUiAction::RestoreHistoryView(LivingHistoryViewV2 {
                    branch: self.history_branch,
                    revision: self.history_revision,
                    tick,
                }))?;
            }
            SandboxUiAction::ComparisonBranch { left, branch } => {
                self.retained_branch(branch)?;
                if left {
                    self.comparison_left = branch;
                } else {
                    self.comparison_right = branch;
                }
            }
            SandboxUiAction::CompareRequestedHistory => {
                if self.history_job.is_some() || self.comparison_job.is_some() {
                    return Err("finish or cancel current history reconstruction".into());
                }
                let tick = canonical_history_tick(&self.comparison_tick)?;
                self.retained_branch(self.comparison_left)?;
                self.retained_branch(self.comparison_right)?;
                let job = begin_compare_living_v2(
                    &self.authority,
                    self.comparison_left,
                    self.comparison_right,
                    tick,
                )
                .map_err(|error| error.to_string())?;
                self.comparison_job = Some(job);
                self.history_message =
                    Some(format!("Comparing selected branches at tick {}", tick.0));
            }
            SandboxUiAction::CancelHistory => {
                self.history_job = None;
                self.comparison_job = None;
                self.history_message =
                    Some("History reconstruction cancelled; current view preserved.".to_owned());
            }
            SandboxUiAction::NotebookPicker => {
                self.apply_action(SandboxUiAction::CreatorCancel)?;
                self.creator_hold = Some(self.begin_hold()?);
                self.notebook_picker = true;
                self.notebook_details = None;
                self.inspector_page = 0;
            }
            SandboxUiAction::ExperimentDetails(index) => {
                if index.is_some_and(|index| usize::from(index) >= EXPERIMENTS.len()) {
                    return Err("Suggested experiment is absent".into());
                }
                self.notebook_details = index;
                self.inspector_page = 0;
            }
            SandboxUiAction::RecordExperiment(index) => {
                let prompt = EXPERIMENTS
                    .get(usize::from(index))
                    .ok_or("experiment is absent")?;
                let witness = self.experiment_receipt_witness(index)?;
                let evidence = self.prepare_experiment_evidence(index)?;
                self.apply_action(SandboxUiAction::RecordBookmark)?;
                self.notebook_evidence = Some(evidence);
                if let Some(entry) = self.bookmark_experiments.last_mut() {
                    *entry = Some(index);
                }
                self.history_message = Some(format!(
                    "Recorded receipt witness: {witness}. Suggested experiment: {prompt} No broader completion or causal claim is implied."
                ));
            }
            SandboxUiAction::RenameBookmark(index) => {
                let generation = self.next_notebook_generation()?;
                let label = nyon_workshop_core::living::LivingNameV2::new(&self.bookmark_label)
                    .map_err(|error| error.to_string())?;
                self.named_bookmarks
                    .get_mut(usize::from(index))
                    .ok_or("bookmark is absent")?
                    .label = label;
                self.notebook_generation = generation;
            }
            SandboxUiAction::RecordComparison => {
                self.record_comparison_experiment()?;
            }
            SandboxUiAction::RecordBookmark => {
                let generation = self.next_notebook_generation()?;
                if self.bookmarks.len() >= 64 {
                    return Err("notebook supports at most 64 exact state bookmarks".to_owned());
                }
                let bookmark = self.bookmark()?;
                let receipt = self
                    .receipts
                    .back()
                    .filter(|receipt| {
                        receipt.tick == bookmark.view.tick
                            && receipt.state_digest == bookmark.digest
                    })
                    .cloned();
                let label = nyon_workshop_core::living::LivingNameV2::new(&self.bookmark_label)
                    .map_err(|error| error.to_string())?;
                self.named_bookmarks
                    .push(nyon_workshop_core::living::LivingNamedBookmarkV2 {
                        ordinal: self
                            .named_bookmarks
                            .last()
                            .map_or(Some(1), |row| row.ordinal.checked_add(1))
                            .ok_or("bookmark ordinal capacity is exhausted")?,
                        label,
                        view: bookmark.view,
                        digest: bookmark.digest,
                    });
                self.bookmarks.push(bookmark);
                self.bookmark_receipts.push(receipt);
                self.bookmark_experiments.push(None);
                self.notebook_generation = generation;
                self.history_message = Some(
                    "Recorded exact branch, creator cursor, tick and state digest.".to_owned(),
                );
            }
            SandboxUiAction::Undo
            | SandboxUiAction::Redo
            | SandboxUiAction::RestoreBookmark(_)
            | SandboxUiAction::ReturnPresent
            | SandboxUiAction::RestoreHistoryView(_) => {
                if self.history_job.is_some() || self.comparison_job.is_some() {
                    return Err("finish or cancel current history reconstruction".to_owned());
                }
                let job = match action {
                    SandboxUiAction::Undo => begin_undo_living_v2(&self.authority),
                    SandboxUiAction::Redo => begin_redo_living_v2(&self.authority),
                    SandboxUiAction::RestoreBookmark(index) => {
                        let bookmark = *self
                            .bookmarks
                            .get(usize::from(index))
                            .ok_or("notebook bookmark absent")?;
                        begin_restore_living_view_v2(&self.authority, bookmark.view)
                    }
                    SandboxUiAction::RestoreHistoryView(view) => {
                        begin_restore_living_view_v2(&self.authority, view)
                    }
                    SandboxUiAction::ReturnPresent => {
                        let view = self
                            .present
                            .map(|bookmark| bookmark.view)
                            .or_else(|| self.authority.present_view())
                            .ok_or("already at the present")?;
                        begin_restore_living_view_v2(&self.authority, view)
                    }
                    _ => unreachable!(),
                }
                .map_err(|error| error.to_string())?;
                if self.present.is_none() && action != SandboxUiAction::ReturnPresent {
                    self.present = Some(self.bookmark()?);
                }
                self.set_paused(true);
                self.history_job = Some(job);
                self.history_message = Some("Reconstructing history.".to_owned());
            }
            SandboxUiAction::CompareBookmark(index) => {
                if self.history_job.is_some() || self.comparison_job.is_some() {
                    return Err("finish or cancel current history reconstruction".to_owned());
                }
                let bookmark = *self
                    .bookmarks
                    .get(usize::from(index))
                    .ok_or("notebook bookmark absent")?;
                let tick = self.state().tick;
                self.comparison_job = Some(
                    begin_compare_living_v2(
                        &self.authority,
                        self.authority.branch(),
                        bookmark.view.branch,
                        tick,
                    )
                    .map_err(|error| error.to_string())?,
                );
                self.history_message =
                    Some(format!("Comparing retained branches at tick {}", tick.0));
            }
            SandboxUiAction::BeginBranch => {
                if !self.creator_modal_open() {
                    self.creator_hold = Some(self.begin_hold()?);
                }
                self.history_picker = false;
                self.content_picker = false;
                self.creator_branch = true;
                self.creator = Some(LivingCreatorDraft::new(
                    LivingCreatorTool::System,
                    self.state(),
                    self.selected_world,
                ));
                self.creator_preview = None;
                self.inspector_page = 0;
            }
            SandboxUiAction::SavesPage(back) => {
                self.saves_page = if back {
                    self.saves_page.saturating_sub(1)
                } else {
                    self.saves_page.saturating_add(1)
                };
            }
            SandboxUiAction::ContentPicker => {
                if !self.creator_modal_open() {
                    self.creator_hold = Some(self.begin_hold()?);
                }
                self.rename_slot = None;
                self.camera_picker = false;
                self.chronicle_picker = false;
                self.intro_picker = false;
                self.navigator_picker = false;
                self.inspector_picker = false;
                self.transfer_picker = false;
                self.audio_picker = false;
                self.library_picker = false;
                self.history_picker = false;
                self.content_picker = true;
                self.creator = None;
                self.creator_scene = false;
                self.creator_preview = None;
                self.inspector_page = 0;
            }
            SandboxUiAction::OpenCreator(tool) => {
                if !self.creator_modal_open() {
                    self.creator_hold = Some(self.begin_hold()?);
                }
                self.content_picker = false;
                self.history_picker = false;
                self.audio_picker = false;
                self.library_picker = false;
                self.creator_branch = false;
                self.rename_slot = None;
                self.camera_picker = false;
                self.chronicle_picker = false;
                self.intro_picker = false;
                self.navigator_picker = false;
                self.inspector_picker = false;
                self.transfer_picker = false;
                self.creator_scene = false;
                self.creator = Some(LivingCreatorDraft::new_for_selection(
                    tool,
                    self.state(),
                    self.selected_entity(),
                ));
                self.creator_message = None;
                self.creator_preview = None;
                self.inspector_page = 0;
            }
            SandboxUiAction::DecisionPage(back) => {
                if let Some((_, receipt)) = self.decisions.back()
                    && !receipt.intents.is_empty()
                {
                    self.selected_decision = if back {
                        self.selected_decision.saturating_sub(1)
                    } else {
                        (self.selected_decision + 1).min(receipt.intents.len() - 1)
                    };
                    self.inspector_page = 0;
                }
            }
            SandboxUiAction::CreatorField(_) => {}
            SandboxUiAction::CreatorTool(back) => {
                if let Some(draft) = &self.creator {
                    let tools = draft.matching_tools();
                    if !tools.is_empty() {
                        let current = tools
                            .iter()
                            .position(|tool| *tool == draft.tool)
                            .unwrap_or(0);
                        let index = if back {
                            (current + tools.len() - 1) % tools.len()
                        } else {
                            (current + 1) % tools.len()
                        };
                        let query = draft.query.clone();
                        let mut next = LivingCreatorDraft::new_for_selection(
                            tools[index],
                            self.state(),
                            self.selected_entity(),
                        );
                        next.query = query;
                        self.creator_scene = false;
                        self.creator = Some(next);
                        self.creator_message = None;
                        self.creator_preview = None;
                        self.inspector_page = 0;
                    }
                }
            }
            SandboxUiAction::CreatorCancel => {
                self.notebook_picker = false;
                self.notebook_details = None;
                if self.intro_picker {
                    self.intro_dismissed = true;
                }
                self.requested_focus = None;
                self.pack_picker = false;
                self.pack_delete_review = None;
                self.pack_delete_hash.clear();
                self.delete_review = None;
                self.delete_name.clear();
                self.rename_slot = None;
                self.audio_picker = false;
                self.library_picker = false;
                self.camera_picker = false;
                self.chronicle_picker = false;
                self.intro_picker = false;
                self.navigator_picker = false;
                self.inspector_picker = false;
                self.transfer_picker = false;
                self.import_preview = None;
                self.creator = None;
                self.creator_scene = false;
                self.content_picker = false;
                self.history_picker = false;
                self.creator_branch = false;
                self.creator_message = None;
                self.creator_preview = None;
                if let Some(hold) = self.creator_hold.take() {
                    self.end_hold(hold);
                }
                self.inspector_page = 0;
            }
            SandboxUiAction::CreatorScene(open) => {
                if self.creator.is_none() {
                    return Err("creator fields are closed".to_owned());
                }
                self.creator_scene = open;
            }
            SandboxUiAction::SnapCreatorParent(selection) => {
                self.creator_preview = None;
                if !crate::living_navigation::entity_exists(self.state(), selection) {
                    self.creator_message =
                        Some("Snap rejected: parent absent. Preview discarded.".to_owned());
                    return Err("snap parent absent".to_owned());
                }
                let draft = self.creator.as_ref().ok_or("creator fields are closed")?;
                use LivingCreatorTool as T;
                let key = match (draft.tool, selection.kind) {
                    (T::Star | T::World, LivingEntityKind::System) => "0.system.id",
                    (
                        T::Deposit | T::Facility | T::Construction | T::HullJob | T::Colony,
                        LivingEntityKind::World,
                    ) => "0.world.id",
                    _ => {
                        self.creator_preview = None;
                        self.creator_message = Some("Snap rejected: this tool has no compatible parent field. Preview discarded.".to_owned());
                        return Err(
                            "this tool has no compatible parent field for the selected entity"
                                .to_owned(),
                        );
                    }
                };
                self.edit_creator_field(
                    key,
                    &crate::living_navigation::entity_id(selection.id).to_ascii_lowercase(),
                )?;
                self.creator_message = Some(format!(
                    "Draft {key} snapped to {}. Preview again before Apply.",
                    crate::living_navigation::entity_id(selection.id)
                ));
            }
            SandboxUiAction::CreatorPreview => self.preview_creator()?,
            SandboxUiAction::CreatorApply => self.apply_creator()?,
            SandboxUiAction::InspectorPage(back) => {
                self.requested_focus = None;
                self.navigator_reveal = false;
                self.inspector_page = if back {
                    self.inspector_page.saturating_sub(1)
                } else {
                    self.inspector_page.saturating_add(1)
                };
            }
        }
        Ok(())
    }

    pub fn preview_creator(&mut self) -> Result<(), String> {
        let result: Result<CreatorPreview, String> = (|| {
            let operations = self
                .creator
                .as_ref()
                .ok_or("creator dialog is closed")?
                .operations_reviewed(self.state())?;
            let cursor = self.authority.published_cursor();
            let envelope = LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Paused,
                command: LivingCommandV2::CreatorBatch { operations },
            };
            let mut candidate = self.authority.clone();
            if self.creator_branch {
                candidate
                    .fork_view_paused(envelope.command.operations().to_vec())
                    .map_err(|error| error.to_string())?;
            } else {
                candidate
                    .apply_paused(envelope.clone())
                    .map_err(|error| error.to_string())?;
            }
            let mut message =
                crate::living_creator::removal_review_lines(envelope.command.operations());
            message.extend(projection_changes(self.state(), candidate.state()));
            let before_ids = navigator_rows(self.state())
                .into_iter()
                .map(|row| row.selection)
                .collect::<BTreeSet<_>>();
            let after_ids = navigator_rows(candidate.state())
                .into_iter()
                .map(|row| row.selection)
                .collect::<BTreeSet<_>>();
            for (label, ids) in [
                ("Added", after_ids.difference(&before_ids)),
                ("Removed", before_ids.difference(&after_ids)),
            ] {
                for selection in ids {
                    message.push(format!(
                        "{label} {} / {}",
                        selection.kind.label(),
                        crate::living_navigation::entity_id(selection.id)
                    ));
                }
            }
            Ok(CreatorPreview {
                envelope,
                branch: self.authority.branch(),
                projected: candidate.state().clone(),
                message,
                fork: self.creator_branch,
            })
        })();
        match result {
            Ok(preview) => {
                self.creator_message =
                    Some("Preview valid. Review every change before Apply.".to_owned());
                self.creator_preview = Some(preview);
                self.creator_scene = true;
                self.inspector_page = 0;
                Ok(())
            }
            Err(error) => {
                self.creator_message = Some(error.clone());
                self.creator_preview = None;
                Err(error)
            }
        }
    }

    pub fn apply_creator(&mut self) -> Result<(), String> {
        let preview = self
            .creator_preview
            .as_ref()
            .ok_or("preview this intervention before applying")?;
        if self.authority.branch() != preview.branch {
            return Err("preview belongs to another history branch".to_owned());
        }
        let cursor = self.authority.published_cursor();
        if cursor.tick != preview.envelope.expected_tick
            || cursor.committed_revision != preview.envelope.expected_committed_revision
            || cursor.pending_sequence != preview.envelope.expected_pending_sequence
        {
            return Err("creator preview is stale; preview the current state again".to_owned());
        }
        if preview.fork {
            self.authority
                .fork_view_paused(preview.envelope.command.operations().to_vec())
                .map_err(|error| error.to_string())?;
            self.receipts.clear();
            self.chronicle.clear();
            self.decisions.clear();
            self.selected_decision = 0;
            if let Some(receipt) = self.authority.last_receipt().cloned() {
                self.receipts.push_back(receipt);
            }
            self.present = None;
        } else {
            let receipt = self
                .authority
                .apply_paused(preview.envelope.clone())
                .map_err(|error| error.to_string())?;
            if self.receipts.len() == 16 {
                self.receipts.pop_front();
            }
            self.receipts.push_back(receipt);
        }
        debug_assert_eq!(self.state(), &preview.projected);
        self.dirty = true;
        if let Some(receipt) = self.receipts.back().cloned() {
            self.record_chronicle(&receipt, None)?;
        }
        self.recover_selection();
        self.dirty = true;
        self.creator_preview = None;
        self.apply_action(SandboxUiAction::CreatorCancel)
    }

    /// First-entry presentation only; restored documents can choose whether to show it.
    pub fn present_intro(&mut self) -> Result<(), String> {
        if self.intro_dismissed {
            return Ok(());
        }
        if let Some(world) = self
            .state()
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.name.as_str() == "Aster Vale" && world.inventory.energy == 0)
            .cloned()
            && let Some(industry) = self
                .state()
                .facilities
                .as_slice()
                .iter()
                .find(|facility| {
                    facility.world == world.id && facility.definition.as_str() == "foundry"
                })
                .map(|facility| facility.id)
            && !self.state().facilities.as_slice().iter().any(|facility| {
                facility.world == world.id && facility.definition.as_str() == "solar_array"
            })
        {
            self.starter_guidance = Some(StarterGuidance {
                branch: self.authority.branch(),
                world: world.id,
                industry,
                before: world.inventory,
                before_tick: self.tick(),
                blocked: None,
                recovered: None,
            });
            self.select_entity(LivingEntitySelection::new(
                LivingEntityKind::World,
                world.id,
            ))?;
            self.scene_zoom = 3;
            self.focus_camera_selection();
        }
        self.apply_action(SandboxUiAction::IntroPicker)
    }
    pub fn starter_guidance_complete(&self) -> bool {
        self.starter_guidance
            .as_ref()
            .is_some_and(|guide| guide.recovered.is_some())
    }

    pub fn starter_guidance_lines(&self) -> Vec<String> {
        let Some(guide) = &self.starter_guidance else {
            return Vec::new();
        };
        let mut lines = vec![
            "Vale Confluence: Aster Vale begins with 0 energy and no Solar Array. Its foundry is a real affected industry. Add power or correct a supply route; every tool remains available.".to_owned(),
            format!("Observed baseline tick {}: energy {}, ore {}, alloy {}. Exact world {}; foundry {}.", guide.before_tick, guide.before.energy, guide.before.ore, guide.before.alloy, crate::living_navigation::entity_id(guide.world), crate::living_navigation::entity_id(guide.industry)),
        ];
        if let Some(recipe) = crate::living_navigation::entity_details(
            self.state(),
            self.authority.catalog(),
            LivingEntitySelection::new(LivingEntityKind::Facility, guide.industry),
        ) {
            lines.extend(recipe.into_iter().filter(|line| {
                line.contains("Recipe") || line.contains("Input") || line.contains("Output")
            }));
        }
        if let Some((receipt, event)) = &guide.blocked {
            lines.push(format!("Authenticated energy blockage at tick {}: event {} / ordinal {} / receipt {} / {:?}.", receipt.tick.0, hex_id(&event.event_id.0), event.ordinal, hex_id(&receipt.digest.0), event.facts.blocked_reasons));
        } else {
            lines.push("Step to an actual foundry attempt and inspect Chronicle's authenticated blocked reason. No timer counts as a witness.".to_owned());
        }
        if let Some((receipt, event, inventory)) = &guide.recovered {
            lines.push(format!("Guidance production witness recorded at tick {}: event {} / receipt {} / actual {:?}. Observed inventory after this completed boundary: energy {}, ore {}, alloy {}. These receipts do not infer a broader causal explanation.",receipt.tick.0,hex_id(&event.event_id.0),hex_id(&receipt.digest.0),event.facts.resources,inventory.energy,inventory.ore,inventory.alloy));
        } else {
            lines.push("Completion waits for the exact foundry's authenticated energy input and alloy output after its blocked witness. Inventory changes alone do not complete guidance.".to_owned());
        }
        lines
    }

    fn record_starter_guidance(
        &mut self,
        receipt: &LivingTickReceiptV2,
        facts: Option<&nyon_workshop_core::living::LivingEventFactsReceiptV2>,
    ) {
        use nyon_workshop_core::living::{
            LivingBlockedReasonV2 as B, LivingEventResourceRoleV2 as Role, LivingResourceV2 as R,
        };
        let branch = self.authority.branch();
        let Some(guide) = self.starter_guidance.as_ref() else {
            return;
        };
        if guide.branch != branch
            || receipt.tick.0 < guide.before_tick
            || !self.state().facilities.as_slice().iter().any(|row| {
                row.id == guide.industry
                    && row.world == guide.world
                    && row.definition.as_str() == "foundry"
            })
        {
            self.starter_guidance = None;
            return;
        }
        let Some(facts) = facts else {
            return;
        };
        if facts.verify_receipt(receipt).is_err() {
            return;
        }
        let inventory = self
            .state()
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == guide.world)
            .map(|world| world.inventory);
        let Some(inventory) = inventory else {
            self.starter_guidance = None;
            return;
        };
        let guide = self.starter_guidance.as_mut().expect("retained guide");
        for event in &facts.events {
            if event.facts.entity != Some(guide.industry) {
                continue;
            }
            if event.facts.blocked_reasons.iter().any(|reason| {
                matches!(
                    reason,
                    B::MissingInput {
                        resource: R::Energy,
                        ..
                    }
                )
            }) && guide.blocked.is_none()
            {
                guide.blocked = Some((receipt.clone(), event.clone()));
            }
            if guide
                .blocked
                .as_ref()
                .is_some_and(|(blocked, _)| blocked.tick < receipt.tick)
                && event.facts.resources.iter().any(|fact| {
                    fact.role == Role::Input && fact.resource == R::Energy && fact.quantity > 0
                })
                && event.facts.resources.iter().any(|fact| {
                    fact.role == Role::Input && fact.resource == R::Ore && fact.quantity > 0
                })
                && event.facts.resources.iter().any(|fact| {
                    fact.role == Role::Output && fact.resource == R::Alloy && fact.quantity > 0
                })
                && guide.recovered.is_none()
            {
                guide.recovered = Some((receipt.clone(), event.clone(), inventory));
            }
        }
    }

    pub const fn intro_dismissed(&self) -> bool {
        self.intro_dismissed
    }

    pub fn set_living_continue_projection(&mut self, slot: Option<u64>) {
        self.living_continue_projection = Some(slot);
    }
    pub(crate) fn pack_library_open(&self) -> bool {
        self.pack_picker
    }

    /// Updates pending work without claiming that the catalog list was loaded.
    pub fn set_pack_pending(&mut self, busy: bool) {
        self.pack_busy = busy;
    }

    pub fn set_pack_projection(
        &mut self,
        rows: &[LivingPackDescriptorV2],
        references: &BTreeMap<LivingCatalogHashV2, Vec<LivingPackReferenceV2>>,
        busy: bool,
    ) {
        self.pack_rows = rows.to_vec();
        self.pack_references = references.clone();
        self.pack_projection_known = true;
        self.pack_busy = busy;
    }
    pub fn pack_delete_confirmation_matches(
        &self,
        hash: LivingCatalogHashV2,
        generation: LivingGenerationV2,
    ) -> bool {
        self.pack_projection_known
            && !self.pack_busy
            && hash != self.builtin_pack
            && hash != self.authority.catalog().catalog_hash()
            && self.pack_references.get(&hash).is_some_and(Vec::is_empty)
            && self
                .pack_delete_review
                .as_ref()
                .is_some_and(|row| row.hash == hash && row.generation == generation && row.archived)
            && self.pack_rows.iter().any(|row| {
                row.hash == hash
                    && row.generation == generation
                    && row.archived
                    && row.diagnostic.is_none()
            })
            && self.pack_delete_hash == hex(&hash.0)
    }
    pub fn dismiss_pack_delete_confirmation(&mut self) {
        self.pack_delete_review = None;
        self.pack_delete_hash.clear();
        self.inspector_page = 0;
    }

    pub fn delete_review_descriptor(&self) -> Option<&LivingSlotDescriptorV2> {
        self.delete_review.as_ref()
    }
    pub fn delete_confirmation_matches(
        &self,
        id: u64,
        generation: crate::living_store::LivingGenerationV2,
        name: &str,
    ) -> bool {
        self.living_continue_projection.is_some()
            && self.delete_review.as_ref().is_some_and(|row| {
                row.archived
                    && row.id.0 == id
                    && row.generation == generation
                    && row.name == name
                    && self.delete_name == row.name
            })
    }
    pub fn dismiss_delete_confirmation(&mut self) {
        if self.delete_review.is_some() {
            let _ = self.apply_action(SandboxUiAction::CreatorCancel);
        }
    }

    pub fn creator_modal_open(&self) -> bool {
        self.creator.is_some()
            || self.notebook_picker
            || self.pack_picker
            || self.delete_review.is_some()
            || self.intro_picker
            || self.chronicle_picker
            || self.camera_picker
            || self.navigator_picker
            || self.inspector_picker
            || self.content_picker
            || self.history_picker
            || self.rename_slot.is_some()
            || self.transfer_picker
            || self.library_picker
            || self.audio_picker
            || self.import_preview.is_some()
    }

    pub fn content_seed(&self) -> Result<u64, String> {
        if self.content_seed.is_empty()
            || !self.content_seed.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            return Err("enter one to sixteen hexadecimal seed digits".to_owned());
        }
        u64::from_str_radix(&self.content_seed, 16)
            .map_err(|_| "seed is outside u64 range".to_owned())
    }

    pub fn creator_field_value(&self, key: &str) -> Option<&str> {
        if self.pack_delete_review.is_some() && key == "pack_delete_hash" {
            return Some(&self.pack_delete_hash);
        }
        if self.delete_review.is_some() && key == "delete_name" {
            return Some(&self.delete_name);
        }
        if self.navigator_picker && key == "navigator_query" {
            return Some(&self.navigator_query);
        }
        if self.history_picker {
            match key {
                "history_tick" => return Some(&self.history_tick),
                "compare_tick" => return Some(&self.comparison_tick),
                _ => {}
            }
        }
        if (self.history_picker || self.notebook_picker) && key == "bookmark_label" {
            return Some(&self.bookmark_label);
        }
        if self.rename_slot.is_some() && key == "save_name" {
            Some(&self.rename_name)
        } else if self.content_picker && key == "seed" {
            Some(&self.content_seed)
        } else {
            self.creator.as_ref().and_then(|draft| draft.value(key))
        }
    }

    pub fn edit_creator_field(&mut self, key: &str, value: &str) -> Result<(), String> {
        if value.bytes().any(|byte| !(32..=126).contains(&byte)) {
            return Err(
                "Use printable ASCII characters supported by the current text renderer".to_owned(),
            );
        }
        if self.pack_delete_review.is_some() && key == "pack_delete_hash" {
            if value.len() > 64 {
                return Err("pack confirmation has at most 64 hexadecimal characters".into());
            }
            self.pack_delete_hash = value.into();
            return Ok(());
        }
        if self.delete_review.is_some() && key == "delete_name" {
            if value.len() > 4096 {
                return Err("Confirmation draft exceeds 4096 UTF-8 bytes".to_owned());
            }
            self.delete_name = value.to_owned();
            return Ok(());
        }
        if self.navigator_picker && key == "navigator_query" {
            if value.len() > 4096 {
                return Err("navigator search exceeds 4096 UTF-8 bytes".to_owned());
            }
            self.navigator_query = value.to_owned();
            self.inspector_page = 0;
            self.navigator_reveal = false;
            return Ok(());
        }
        if self.history_picker && matches!(key, "history_tick" | "compare_tick") {
            if value.len() > 20 {
                return Err("tick draft has at most 20 decimal digits".into());
            }
            if key == "history_tick" {
                self.history_tick = value.into();
            } else {
                self.comparison_tick = value.into();
            }
            return Ok(());
        }
        if (self.history_picker || self.notebook_picker) && key == "bookmark_label" {
            if value.len() > 4096 {
                return Err("bookmark label draft exceeds 4096 UTF-8 bytes".to_owned());
            }
            self.bookmark_label = value.to_owned();
            return Ok(());
        }
        if self.rename_slot.is_some() && key == "save_name" {
            if value.len() > 4096 {
                return Err("name draft exceeds 4096 UTF-8 bytes".to_owned());
            }
            self.rename_name = value.to_owned();
            return Ok(());
        }
        if self.content_picker && key == "seed" {
            if value.len() > 16 {
                return Err("seed has at most 16 hexadecimal digits".to_owned());
            }
            self.content_seed = value.to_owned();
            return Ok(());
        }
        self.creator
            .as_mut()
            .ok_or("creator dialog is closed")?
            .edit(key, value)?;
        self.creator_message = None;
        self.creator_preview = None;
        Ok(())
    }

    fn recover_selection(&mut self) {
        if self.selected_entity.is_some_and(|selected| {
            !crate::living_navigation::entity_exists(self.state(), selected)
        }) {
            if let Some(parent) = self.selected_entity() {
                let _ = self.select_entity(parent);
                if parent.kind == LivingEntityKind::System {
                    self.selected_world = None;
                    self.scene_zoom = 3;
                }
            } else {
                self.selected_entity = None;
                self.selected_world = None;
                self.selected_system = None;
                self.selected_ancestors.clear();
                self.scene_zoom = 1;
            }
            self.prune_camera_scopes();
            // Removing a focused target is an explicit recovery boundary.
            // Fit its surviving scope once; ordinary renders retain user framing.
            let recovered_scope = self.camera_scope();
            self.cameras.get_mut().remove(&recovered_scope);
            self.ensure_camera_scope(self.last_scene_bounds.get());
        }
        let invalid_scopes: Vec<_> = self
            .cameras
            .borrow()
            .iter()
            .filter(|(_, camera)| {
                camera.focus_target.is_some_and(|target| {
                    !crate::living_navigation::entity_exists(self.state(), target)
                })
            })
            .map(|(scope, _)| *scope)
            .collect();
        let current_scope = self.camera_scope();
        let refit_current = invalid_scopes.contains(&current_scope);
        for scope in invalid_scopes {
            self.cameras.get_mut().remove(&scope);
        }
        if refit_current {
            self.ensure_camera_scope(self.last_scene_bounds.get());
        }
    }

    pub fn contextual_creator_tool(&self) -> LivingCreatorTool {
        use LivingCreatorTool as T;
        use LivingEntityKind as K;
        match self.selected_entity().map(|selection| selection.kind) {
            Some(K::System) => T::World,
            Some(K::Star) => T::EditStar,
            Some(K::World | K::Colony) => T::Facility,
            Some(K::Deposit) => T::EditDeposit,
            Some(K::Facility) => T::FacilityEnabled,
            Some(K::ConstructionJob) => T::CancelConstruction,
            Some(K::HullJob) => T::CancelHullJob,
            Some(K::Civilization) => T::Policy,
            Some(K::Lane) => T::EditLane,
            Some(K::Fleet) => T::FleetOrder,
            Some(K::Hull) => T::EditHull,
            Some(K::Route) => T::EditRoute,
            Some(K::Shipment) => T::ShipmentDisposition,
            Some(K::Hazard) => T::EditHazard,
            None => T::System,
        }
    }

    pub fn selected_world(&self) -> Option<LivingEntityIdV2> {
        self.selected_world
    }

    pub fn take_requested_focus(&mut self) -> Option<SemanticActionId> {
        self.requested_focus.take()
    }

    pub fn selected_entity(&self) -> Option<LivingEntitySelection> {
        self.selected_entity
            .filter(|selection| crate::living_navigation::entity_exists(self.state(), *selection))
            .or_else(|| {
                self.selected_ancestors.iter().copied().find(|selection| {
                    crate::living_navigation::entity_exists(self.state(), *selection)
                })
            })
    }
    pub fn select_entity(&mut self, selection: LivingEntitySelection) -> Result<(), String> {
        let rows = navigator_rows(self.state());
        let row = rows
            .iter()
            .find(|row| row.selection == selection)
            .ok_or("selected entity is absent from this galaxy")?;
        let mut selected_ancestors = Vec::new();
        let mut parent = row.parent;
        while let Some(key) = parent {
            selected_ancestors.push(key);
            parent = rows
                .iter()
                .find(|row| row.selection == key)
                .and_then(|row| row.parent);
        }
        let mut ancestor = Some(selection);
        let mut system = None;
        while let Some(key) = ancestor {
            if key.kind == LivingEntityKind::System {
                system = Some(key.id);
                break;
            }
            ancestor = rows
                .iter()
                .find(|row| row.selection == key)
                .and_then(|row| row.parent);
        }
        self.selected_world = row.world.or_else(|| {
            system.and_then(|system| {
                self.state()
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|world| world.system == system)
                    .map(|world| world.id)
            })
        });
        self.selected_system = system;
        self.selected_ancestors = selected_ancestors;
        self.selected_entity = Some(selection);
        self.inspector_page = 0;
        Ok(())
    }
    pub fn select_world(&mut self, world: LivingEntityIdV2) -> Result<(), String> {
        self.select_entity(LivingEntitySelection::new(LivingEntityKind::World, world))
    }

    fn retained_branch(&self, id: LivingBranchIdV2) -> Result<&LivingGalaxyAuthorityV2, String> {
        self.authority
            .retained_history_branch(id)
            .ok_or_else(|| "selected branch is not retained in this galaxy".into())
    }
    fn validate_history_revision(
        &self,
        branch: LivingBranchIdV2,
        revision: Option<LivingRevisionIdV2>,
    ) -> Result<(), String> {
        let authority = self.retained_branch(branch)?;
        if revision.is_some_and(|id| {
            !authority
                .revisions()
                .iter()
                .any(|revision| revision.id == id)
        }) {
            return Err("revision does not belong to the selected retained branch".into());
        }
        Ok(())
    }
    fn requested_history_rows(&self) -> Vec<(String, String, SandboxUiAction)> {
        let mut rows = vec![
            (
                "sandbox.notebook.open".into(),
                "Experiments notebook".into(),
                SandboxUiAction::NotebookPicker,
            ),
            (
                "sandbox.creator.field.history_tick".into(),
                "Requested history tick".into(),
                SandboxUiAction::CreatorField(0),
            ),
            (
                "sandbox.history.requested.restore".into(),
                "Restore requested branch / cursor / tick".into(),
                SandboxUiAction::RestoreRequestedHistory,
            ),
            (
                "sandbox.creator.field.compare_tick".into(),
                "Common comparison tick".into(),
                SandboxUiAction::CreatorField(0),
            ),
            (
                "sandbox.history.requested.compare".into(),
                "Compare selected left and right branches".into(),
                SandboxUiAction::CompareRequestedHistory,
            ),
            (
                "sandbox.history.requested.genesis".into(),
                "Select genesis creator cursor".into(),
                SandboxUiAction::HistoryRevision(None),
            ),
        ];
        for branch in std::iter::once(&self.authority)
            .chain(self.authority.archived())
            .filter_map(|branch| self.authority.retained_history_branch(branch.branch()))
        {
            let id = branch.branch();
            let number = branch.state().branch_sequence;
            for (suffix, label, action) in [
                (
                    "view",
                    format!("History branch {number}"),
                    SandboxUiAction::HistoryBranch(id),
                ),
                (
                    "left",
                    format!("Comparison left: branch {number}"),
                    SandboxUiAction::ComparisonBranch {
                        left: true,
                        branch: id,
                    },
                ),
                (
                    "right",
                    format!("Comparison right: branch {number}"),
                    SandboxUiAction::ComparisonBranch {
                        left: false,
                        branch: id,
                    },
                ),
            ] {
                rows.push((
                    format!("sandbox.history.choose.{suffix}.{}", hex_id(&id.0)),
                    label,
                    action,
                ));
            }
        }
        if let Ok(branch) = self.retained_branch(self.history_branch) {
            for revision in branch.revisions() {
                rows.push((
                    format!("sandbox.history.cursor.{}", hex_id(&revision.id.0)),
                    format!(
                        "Select creator cursor {} / accepted tick {}",
                        revision.ordinal, revision.tick.0
                    ),
                    SandboxUiAction::HistoryRevision(Some(revision.id)),
                ));
                rows.push((
                    format!("sandbox.history.details.{}", hex_id(&revision.id.0)),
                    format!("Read creator command {}", revision.ordinal),
                    SandboxUiAction::HistoryRevisionDetails(branch.branch(), revision.id),
                ));
            }
        }
        if let Some(comparison) = &self.comparison {
            let mut facts = vec![
                format!("Completed comparison tick {}", comparison.tick.0),
                format!(
                    "Left branch {} / cursor {:?}",
                    hex_id(&comparison.left.view.branch.0),
                    comparison.left.view.revision.map(|id| hex_id(&id.0))
                ),
                format!(
                    "Right branch {} / cursor {:?}",
                    hex_id(&comparison.right.view.branch.0),
                    comparison.right.view.revision.map(|id| hex_id(&id.0))
                ),
            ];
            facts.extend(projection_changes(
                &comparison.left_state,
                &comparison.right_state,
            ));
            if comparison.left_state.relations != comparison.right_state.relations {
                facts.push(format!(
                    "Left directed relationships: {}",
                    serde_json::to_string(&comparison.left_state.relations)
                        .expect("typed relationships serialize")
                ));
                facts.push(format!(
                    "Right directed relationships: {}",
                    serde_json::to_string(&comparison.right_state.relations)
                        .expect("typed relationships serialize")
                ));
            }
            if comparison.left_state.agreements != comparison.right_state.agreements
                || comparison.left_state.wars != comparison.right_state.wars
            {
                facts.push(format!(
                    "Left treaties/wars: {} / {}",
                    serde_json::to_string(&comparison.left_state.agreements)
                        .expect("typed treaties serialize"),
                    serde_json::to_string(&comparison.left_state.wars)
                        .expect("typed wars serialize")
                ));
                facts.push(format!(
                    "Right treaties/wars: {} / {}",
                    serde_json::to_string(&comparison.right_state.agreements)
                        .expect("typed treaties serialize"),
                    serde_json::to_string(&comparison.right_state.wars)
                        .expect("typed wars serialize")
                ));
            }
            let creator_inputs = |bookmark: LivingBookmarkV2| {
                self.retained_branch(bookmark.view.branch)
                    .map(|branch| {
                        let ordinal = bookmark.view.revision.and_then(|id| {
                            branch
                                .revisions()
                                .iter()
                                .find(|revision| revision.id == id)
                                .map(|revision| revision.ordinal)
                        });
                        branch
                            .revisions()
                            .iter()
                            .filter(|revision| {
                                ordinal.is_some_and(|ordinal| revision.ordinal <= ordinal)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            };
            let left_inputs = creator_inputs(comparison.left);
            let right_inputs = creator_inputs(comparison.right);
            for (label, selected, other) in [
                ("Left", &left_inputs, &right_inputs),
                ("Right", &right_inputs, &left_inputs),
            ] {
                for revision in selected
                    .iter()
                    .filter(|revision| !other.iter().any(|other| other.id == revision.id))
                {
                    facts.push(format!(
                        "{label}-only creator input {} at {}: {}",
                        revision.ordinal,
                        revision.tick.0,
                        serde_json::to_string(&revision.command)
                            .expect("typed creator command serializes")
                    ));
                }
            }
            facts.push("Freight totals require actual delivery witnesses; pending manifests are not cumulative delivered freight.".into());
            for (index, chunk) in ui_text_chunks(&facts.join(" ")).enumerate() {
                rows.push((
                    format!("sandbox.history.detail.line.comparison.{index}"),
                    chunk.to_owned(),
                    SandboxUiAction::CreatorField(0),
                ));
            }
        }
        if let Some((branch, id)) = self.history_details
            && let Ok(branch) = self.retained_branch(branch)
            && let Some(revision) = branch.revisions().iter().find(|revision| revision.id == id)
        {
            rows.clear();
            rows.push((
                "sandbox.history.details.back".into(),
                "Back to history controls".into(),
                SandboxUiAction::HistoryDetailsClose,
            ));
            let detail = format!(
                "Creator {} at tick {}. Parent {:?}. Command {}",
                revision.ordinal,
                revision.tick.0,
                revision.parent.map(|id| hex_id(&id.0)),
                serde_json::to_string(&revision.command).expect("typed creator command serializes")
            );
            for (index, chunk) in ui_text_chunks(&detail).enumerate() {
                rows.push((
                    format!("sandbox.history.detail.line.{index}"),
                    chunk.to_owned(),
                    SandboxUiAction::CreatorField(0),
                ));
            }
        }
        rows
    }

    pub fn record_comparison_experiment(&mut self) -> Result<(), String> {
        use nyon_workshop_core::living::{
            LivingNameV2, LivingNamedBookmarkV2, LivingNotebookExperimentV2,
            LivingNotebookMetricV2 as M, LivingNotebookObservationV2, measure_living_notebook_v2,
        };
        let generation = self.next_notebook_generation()?;
        let comparison = self
            .comparison
            .as_ref()
            .ok_or("finish a matching-tick comparison before recording measurements")?;
        if self.bookmarks.len() > 62 || self.notebook_experiments.len() >= 64 {
            return Err("notebook capacity is full".to_owned());
        }
        let label = LivingNameV2::new(&self.bookmark_label).map_err(|error| error.to_string())?;
        let mut scopes = vec![
            (M::ColonyCount, None),
            (M::FacilityCount, None),
            (M::HullCount, None),
            (M::Tick, None),
        ];
        if let Some(world) = self.selected_world {
            scopes.extend([
                (M::InventoryEnergy, Some(world)),
                (M::InventoryOre, Some(world)),
                (M::InventoryAlloy, Some(world)),
                (M::Owner, Some(world)),
            ]);
        }
        scopes.sort();
        let observations = scopes
            .into_iter()
            .map(|(metric, entity)| {
                Ok(LivingNotebookObservationV2 {
                    metric,
                    entity,
                    left: measure_living_notebook_v2(&comparison.left_state, metric, entity)
                        .map_err(|error| error.to_string())?,
                    right: measure_living_notebook_v2(&comparison.right_state, metric, entity)
                        .map_err(|error| error.to_string())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let left = self
            .named_bookmarks
            .last()
            .map_or(Some(1), |row| row.ordinal.checked_add(1))
            .filter(|ordinal| *ordinal < u64::MAX)
            .ok_or("bookmark ordinal capacity is exhausted")?;
        let experiment_ordinal = self
            .notebook_experiments
            .last()
            .map_or(Some(1), |row| row.ordinal.checked_add(1))
            .ok_or("experiment ordinal capacity is exhausted")?;
        for (ordinal, side, name) in [
            (left, comparison.left, "Comparison left"),
            (left + 1, comparison.right, "Comparison right"),
        ] {
            self.named_bookmarks.push(LivingNamedBookmarkV2 {
                ordinal,
                label: LivingNameV2::new(name).map_err(|error| error.to_string())?,
                view: side.view,
                digest: side.digest,
            });
            self.bookmarks.push(side);
            self.bookmark_receipts.push(None);
            self.bookmark_experiments.push(None);
        }
        self.notebook_experiments.push(LivingNotebookExperimentV2 {
            ordinal: experiment_ordinal,
            label,
            left,
            right: left + 1,
            comparison_tick: comparison.tick,
            observations,
        });
        self.notebook_generation = generation;
        self.history_message = Some("Recorded measured matching-tick outcomes; these observations do not establish causation.".to_owned());
        Ok(())
    }

    pub fn import_notebook(
        &mut self,
        validated: nyon_workshop_core::living::ValidatedLivingNotebookV2,
    ) -> Result<(), String> {
        let generation = self
            .notebook_generation
            .checked_add(2)
            .ok_or("notebook generation capacity is exhausted")?;
        self.install_notebook(validated)?;
        self.notebook_generation = generation;
        Ok(())
    }

    pub fn install_notebook(
        &mut self,
        validated: impl std::borrow::Borrow<nyon_workshop_core::living::ValidatedLivingNotebookV2>,
    ) -> Result<(), String> {
        let validated = validated.borrow();
        let notebook = validated.notebook();
        if notebook.catalog_hash != self.authority.catalog().catalog_hash()
            || notebook.genesis_digest
                != self
                    .authority
                    .genesis_digest()
                    .map_err(|error| error.to_string())?
            || notebook.bookmarks.iter().any(|row| {
                row.view.branch != self.authority.branch()
                    && !self
                        .authority
                        .archived()
                        .iter()
                        .any(|branch| branch.branch() == row.view.branch)
            })
        {
            return Err("notebook does not belong to this resident galaxy".to_owned());
        }
        if !validated
            .matches_retained_history(&self.authority)
            .map_err(|error| error.to_string())?
        {
            return Err(
                "notebook does not match this resident galaxy's retained history".to_owned(),
            );
        }
        let generation = self.next_notebook_generation()?;
        self.notebook_trade_completions = validated.trade_completions().to_vec();
        self.notebook_supply_completions = validated.supply_completions().to_vec();
        self.notebook_corridor_observations = validated.corridor_observations().to_vec();
        self.bookmarks = notebook
            .bookmarks
            .iter()
            .map(|row| LivingBookmarkV2 {
                view: row.view,
                digest: row.digest,
            })
            .collect();
        self.bookmark_receipts = vec![None; self.bookmarks.len()];
        self.bookmark_experiments = vec![None; self.bookmarks.len()];
        self.named_bookmarks = notebook.bookmarks.clone();
        self.notebook_generation = generation;
        self.notebook_saved_generation = generation;
        self.notebook_experiments = notebook.experiments.clone();
        self.notebook_evidence = notebook
            .evidence
            .clone()
            .map(|mut section| {
                section.next_ordinal = section.next_ordinal.max(
                    self.notebook_evidence
                        .as_ref()
                        .map_or(1, |old| old.next_ordinal),
                );
                section
            })
            .or_else(|| {
                self.notebook_evidence.as_ref().map(|old| {
                    nyon_workshop_core::living::LivingNotebookEvidenceV2 {
                        next_ordinal: old.next_ordinal,
                        records: Vec::new(),
                    }
                })
            });
        self.notebook_replay_verified.clear();
        if let Some(evidence) = &self.notebook_evidence {
            for record in &evidence.records {
                self.notebook_replay_verified.insert(record.ordinal);
                if let Some(index) = self
                    .named_bookmarks
                    .iter()
                    .position(|row| row.ordinal == record.bookmark)
                {
                    self.bookmark_experiments[index] = Some(experiment_index(record.experiment));
                }
            }
        }
        self.history_message =
            Some("Installed validated named bookmarks and measured experiments.".to_owned());
        Ok(())
    }

    /// Publish only derived replay evidence for the exact resident metadata.
    pub fn accept_notebook_validation(
        &mut self,
        validated: &nyon_workshop_core::living::ValidatedLivingNotebookV2,
    ) -> Result<(), String> {
        let notebook = validated.notebook();
        if notebook.bookmarks != self.named_bookmarks
            || notebook.experiments != self.notebook_experiments
            || notebook.evidence != self.notebook_evidence
            || !validated
                .matches_retained_history(&self.authority)
                .map_err(|error| error.to_string())?
        {
            return Err("Replay evidence does not match the resident notebook".into());
        }
        self.notebook_trade_completions = validated.trade_completions().to_vec();
        self.notebook_supply_completions = validated.supply_completions().to_vec();
        self.notebook_corridor_observations = validated.corridor_observations().to_vec();
        self.notebook_replay_verified = self
            .notebook_evidence
            .as_ref()
            .into_iter()
            .flat_map(|section| &section.records)
            .map(|record| record.ordinal)
            .collect();
        Ok(())
    }

    pub fn notebook_snapshot(
        &self,
    ) -> (
        Vec<nyon_workshop_core::living::LivingNamedBookmarkV2>,
        Vec<nyon_workshop_core::living::LivingNotebookExperimentV2>,
    ) {
        (
            self.named_bookmarks.clone(),
            self.notebook_experiments.clone(),
        )
    }

    /// Recorded exact portable observations, independent of the disposable receipt cache.
    pub fn notebook_evidence_snapshot(
        &self,
    ) -> Option<nyon_workshop_core::living::LivingNotebookEvidenceV2> {
        self.notebook_evidence.clone()
    }

    fn prepare_experiment_evidence(
        &self,
        index: u8,
    ) -> Result<nyon_workshop_core::living::LivingNotebookEvidenceV2, String> {
        use nyon_workshop_core::living::{
            LivingNotebookEventReferenceV2, LivingNotebookEvidenceRecordV2,
            LivingNotebookEvidenceV2, LivingNotebookExperimentKindV2 as K,
            LivingNotebookRouteAttemptReferenceV2,
        };
        let experiment = match index {
            0 => K::SupplyChain,
            1 => K::AutonomousSettlement,
            2 => K::TradeDelivery,
            3 => K::CorridorImpact,
            5 => K::DormantRecovery,
            _ => return Err("This experiment requires additional authenticated witnesses".into()),
        };
        let bookmark = self.bookmark()?;
        let receipt = self
            .receipts
            .back()
            .filter(|receipt| {
                receipt.tick == bookmark.view.tick && receipt.state_digest == bookmark.digest
            })
            .ok_or("No ordinary receipt matches this exact bookmark state")?;
        let events: Vec<_> = receipt
            .events
            .iter()
            .filter(|event| self.experiment_event_supported(index, receipt, event))
            .map(|event| LivingNotebookEventReferenceV2 {
                ordinal: event.ordinal,
                id: event.id,
                kind: event.kind,
            })
            .collect();
        let route_attempts = if index == 3 {
            let mut attempts: Vec<_> = self
                .authority
                .last_corridor_restrictions()
                .iter()
                .filter(|proof| {
                    proof.tick() == receipt.tick && proof.receipt_digest() == receipt.digest
                })
                .map(|proof| LivingNotebookRouteAttemptReferenceV2 {
                    route: proof.route(),
                    hazard: proof.hazard().id,
                })
                .collect();
            attempts.sort_by_key(|attempt| (attempt.route, attempt.hazard));
            attempts.dedup();
            (!attempts.is_empty()).then_some(attempts)
        } else {
            None
        };
        let reference_count = events.len() + route_attempts.as_ref().map_or(0, Vec::len);
        if reference_count == 0 || reference_count > 32 {
            return Err("An evidence record requires 1 to 32 exact supporting references".into());
        }
        let mut evidence = self
            .notebook_evidence
            .clone()
            .unwrap_or(LivingNotebookEvidenceV2 {
                next_ordinal: 1,
                records: Vec::new(),
            });
        if evidence.records.len() >= 128 {
            return Err("Notebook supports at most 128 evidence records".into());
        }
        let ordinal = evidence.next_ordinal;
        evidence.next_ordinal = ordinal
            .checked_add(1)
            .ok_or("Evidence ordinal capacity is exhausted")?;
        let bookmark = self
            .named_bookmarks
            .last()
            .map_or(Some(1), |row| row.ordinal.checked_add(1))
            .ok_or("Bookmark ordinal capacity is exhausted")?;
        evidence.records.push(LivingNotebookEvidenceRecordV2 {
            ordinal,
            bookmark,
            experiment,
            receipt_digest: receipt.digest,
            events,
            route_attempts,
        });
        Ok(evidence)
    }

    pub fn decision_receipts(
        &self,
    ) -> &VecDeque<(
        LivingTickReceiptV2,
        nyon_workshop_core::living::LivingDecisionReceiptV2,
    )> {
        &self.decisions
    }
    pub fn receipts(&self) -> &VecDeque<LivingTickReceiptV2> {
        &self.receipts
    }

    fn experiment_event_supported(
        &self,
        index: u8,
        receipt: &LivingTickReceiptV2,
        event: &nyon_workshop_core::living::LivingEventV2,
    ) -> bool {
        use nyon_workshop_core::living::{LivingCivilizationStatusV2 as S, LivingEventKindV2 as E};
        let autonomous = matches!(event.provenance, LivingEventProvenanceV2::Autonomous { .. });
        match index {
            0 => nyon_workshop_core::living::supports_living_supply_observation_v2(
                &self.authority,
                receipt,
                event,
            ),
            1 => autonomous && event.kind == E::SettlementClaimResolved,
            3 => nyon_workshop_core::living::supports_living_corridor_wait_observation_v2(
                &self.authority,
                receipt,
                event,
            ),
            2 => {
                let Some(sidecar) = self.authority.last_event_facts() else {
                    return false;
                };
                // Authority publishes only verified sidecars. Check this exact
                // boundary without rehashing the full receipt for every event.
                if sidecar.receipt_digest != receipt.digest
                    || sidecar.tick != receipt.tick
                    || sidecar.state_digest != receipt.state_digest
                {
                    return false;
                }
                let Ok(position) = sidecar
                    .events
                    .binary_search_by_key(&event.ordinal, |entry| entry.ordinal)
                else {
                    return false;
                };
                let entry = &sidecar.events[position];
                if !autonomous || entry.event_id != event.id {
                    return false;
                }
                match event.kind {
                    E::AgreementFormed => entry.facts.agreement.as_ref().is_some_and(|agreement| {
                        agreement.kind == nyon_workshop_core::living::LivingAgreementKindV2::Trade
                            && agreement.participant_a < agreement.participant_b
                            && agreement.start_tick == receipt.tick
                            && agreement.end_tick > agreement.start_tick
                    }),
                    E::ShipmentDelivered => entry.facts.delivery.as_ref().is_some_and(|delivery| {
                        delivery.dispatch_kind
                            == Some(nyon_workshop_core::living::LivingRouteKindV2::Trade)
                            && delivery.source_world != delivery.destination_world
                            && delivery.dispatch_owner != delivery.recipient
                            && delivery.recipient == delivery.intended_receiver
                            && delivery.units > 0
                    }),
                    _ => false,
                }
            }
            5 if autonomous => event.kind == E::CivilizationReactivated,
            5 if event.kind == E::CreatorIntervention => {
                let LivingEventProvenanceV2::Creator { revision } = event.provenance else {
                    return false;
                };
                self.authority
                    .last_creator_recoveries()
                    .iter()
                    .any(|proof| {
                        proof.revision() == revision
                            && proof.receipt_digest() == receipt.digest
                            && self
                                .state()
                                .civilizations
                                .as_slice()
                                .iter()
                                .any(|actor| actor.id == proof.actor() && actor.status == S::Active)
                            && self.state().colonies.as_slice().iter().any(|colony| {
                                colony.world == proof.world() && colony.owner == proof.actor()
                            })
                    })
            }
            _ => false,
        }
    }

    fn experiment_receipt_witness(&self, index: u8) -> Result<String, String> {
        use nyon_workshop_core::living::LivingEventKindV2 as E;
        let bookmark = self.bookmark()?;
        let receipt = self
            .receipts
            .back()
            .filter(|receipt| {
                receipt.tick == bookmark.view.tick && receipt.state_digest == bookmark.digest
            })
            .ok_or("Pause on an actual committed witness boundary before recording")?;
        match index {
            1 if self.authority.revisions().is_empty()=>{},
            1=>return Err("Autonomous settlement evidence requires a selected history with no creator interventions".to_owned()),
            5=>{},
            0=>{},
            2=>{},
            3=>{
                if let Some(proof) = self.authority.last_corridor_restrictions().iter().find(|proof| proof.tick() == receipt.tick && proof.receipt_digest() == receipt.digest) {
                    return Ok(format!("Corridor route {} restricted by hazard {} at tick {}; eligible {} units, restricted batch {}, actually dispatched {}. Downstream wait requires a separate later observation", hex(&proof.route().0), hex(&proof.hazard().id.0), receipt.tick.0, proof.eligible_batch(), proof.restricted_batch(), proof.actual_dispatched_units()));
                }
            },
            4=>return Err("War-and-alternative evidence requires matched war parties and changed branch inputs, not an arbitrary nonempty receipt".to_owned()),
            6=>return Err("Portable reproduction requires a verified exported/imported artifact pair; a simulation receipt cannot prove file transfer".to_owned()),
            _=>return Err("Suggested experiment is absent".to_owned()),
        };
        let event = receipt
            .events
            .iter()
            .find(|event| self.experiment_event_supported(index, receipt, event))
            .ok_or(
                "The current receipt does not contain this experiment's exact recovery or autonomous witness",
            )?;
        let actor_note = if let LivingEventProvenanceV2::Creator { revision } = event.provenance {
            let actors: Vec<_> = self
                .authority
                .last_creator_recoveries()
                .iter()
                .filter(|proof| {
                    proof.revision() == revision && proof.receipt_digest() == receipt.digest
                })
                .map(|proof| {
                    format!(
                        "recovered civilization {} / owned colony {}",
                        hex(&proof.actor().0),
                        hex(&proof.world().0)
                    )
                })
                .collect();
            actors.join("; ")
        } else if index == 2 {
            let facts =
                self.authority
                    .last_event_facts()
                    .and_then(|sidecar| {
                        sidecar.events.iter().find(|entry| {
                            entry.ordinal == event.ordinal && entry.event_id == event.id
                        })
                    })
                    .map(|entry| &entry.facts)
                    .ok_or("Trade witness details are unavailable")?;
            if let Some(delivery) = &facts.delivery {
                format!(
                    "per event delivered {} {} from world {} to {}; sender {}; recipient {}; formation and delivery are separate observations",
                    delivery.units,
                    delivery.resource.as_wire_str(),
                    hex(&delivery.source_world.0),
                    hex(&delivery.destination_world.0),
                    hex(&delivery.dispatch_owner.0),
                    hex(&delivery.recipient.0)
                )
            } else if let Some(agreement) = &facts.agreement {
                format!(
                    "trade agreement between {} and {}, ticks {} to {} exclusive; delivery requires a separate observation",
                    hex(&agreement.participant_a.0),
                    hex(&agreement.participant_b.0),
                    agreement.start_tick.0,
                    agreement.end_tick.0
                )
            } else {
                return Err("Trade witness details are unavailable".into());
            }
        } else if index == 0 {
            let facts =
                self.authority
                    .last_event_facts()
                    .and_then(|sidecar| {
                        sidecar.events.iter().find(|entry| {
                            entry.ordinal == event.ordinal && entry.event_id == event.id
                        })
                    })
                    .map(|entry| &entry.facts)
                    .ok_or("Supply witness details are unavailable")?;
            if let Some(delivery) = &facts.delivery {
                format!(
                    "per event {} {} delivered from world {} to {}; sender {}; recipient {}; production requires a separate matching observation",
                    delivery.units,
                    delivery.resource.as_wire_str(),
                    hex(&delivery.source_world.0),
                    hex(&delivery.destination_world.0),
                    hex(&delivery.dispatch_owner.0),
                    hex(&delivery.recipient.0)
                )
            } else {
                format!(
                    "facility {}; actor {}; actual resources {:?}; inter-system delivery requires a separate matching observation",
                    facts
                        .entity
                        .map_or_else(|| "unknown".into(), |id| hex(&id.0)),
                    facts
                        .actor
                        .map_or_else(|| "unowned".into(), |id| hex(&id.0)),
                    facts.resources
                )
            }
        } else {
            "actor not recorded".to_owned()
        };
        Ok(format!(
            "{} at tick {}, event {}, ordinal {}; {}",
            if index == 5 && event.kind == E::CreatorIntervention {
                "creator recovery"
            } else {
                event.kind.as_wire_str()
            },
            receipt.tick.0,
            hex(&event.id.0),
            event.ordinal,
            actor_note
        ))
    }

    pub fn chronicle_rows(&self) -> Vec<&crate::living_chronicle::LivingChronicleRow> {
        self.chronicle.rows(&self.chronicle_filter)
    }
    fn record_chronicle(
        &mut self,
        receipt: &LivingTickReceiptV2,
        decision: Option<&nyon_workshop_core::living::LivingDecisionReceiptV2>,
    ) -> Result<(), String> {
        let facts = self
            .authority
            .last_event_facts()
            .filter(|facts| {
                facts.receipt_digest == receipt.digest
                    && facts.tick == receipt.tick
                    && facts.state_digest == receipt.state_digest
            })
            .cloned();
        match self
            .chronicle
            .record_with_facts(receipt, decision, facts.as_ref())
        {
            Ok(Some(_)) => self.set_paused(true),
            Ok(None) => {}
            Err(error) => {
                self.set_paused(true);
                self.fault = Some(error.clone());
                return Err(error);
            }
        }
        self.record_starter_guidance(receipt, facts.as_ref());
        Ok(())
    }
    fn prune_camera_scopes(&mut self) {
        let valid: BTreeSet<_> = std::iter::once(LivingCameraScope {
            level: LivingCameraLevel::Galaxy,
            entity: None,
        })
        .chain(
            self.state()
                .systems
                .as_slice()
                .iter()
                .map(|row| LivingCameraScope {
                    level: LivingCameraLevel::System,
                    entity: Some(row.id),
                }),
        )
        .chain(
            self.state()
                .worlds
                .as_slice()
                .iter()
                .map(|row| LivingCameraScope {
                    level: LivingCameraLevel::World,
                    entity: Some(row.id),
                }),
        )
        .collect();
        self.cameras
            .get_mut()
            .retain(|scope, _| valid.contains(scope));
        debug_assert!(self.cameras.get_mut().len() <= 577);
    }

    pub fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }

    fn cancel_step(&mut self) {
        if let Some(mut job) = self.step_job.take() {
            job.cancel();
        }
        self.debt_nanos = 0;
        self.due_ticks = 0;
        self.explicit_step = false;
        self.limited_speed = None;
        self.publication_pace = PublicationPace::default();
    }

    /// A prospective boundary never changes the published presentation.
    pub fn processing(&self) -> bool {
        self.due_ticks != 0 || self.step_job.is_some()
    }

    /// Compatibility helper for callers explicitly requesting synchronous work.
    /// Interactive Step and Run are polled only by update.
    pub fn step_once(&mut self) -> Result<(), String> {
        if self.authority.is_viewing_history() {
            return Err("Viewing history: Return to present or Branch here before stepping".into());
        }
        if let Some(fault) = &self.fault {
            return Err(fault.clone());
        }
        self.cancel_step();
        self.due_ticks = 1;
        while self.due_ticks != 0 {
            self.poll_step(1024)?;
        }
        Ok(())
    }

    fn record_step_publication(&mut self) -> Result<(), String> {
        let receipt = self
            .authority
            .last_receipt()
            .expect("successful step seals receipt")
            .clone();
        let decision = self
            .authority
            .last_decision()
            .filter(|decision| decision.receipt_digest == receipt.digest)
            .cloned();
        self.dirty = true;
        self.record_chronicle(&receipt, decision.as_ref())?;
        if let Some(decision) = self.authority.last_decision()
            && decision.tick == receipt.tick
            && decision.receipt_digest == receipt.digest
            && decision.state_after == receipt.state_digest
            && !decision.intents.is_empty()
        {
            if self.decisions.len() == 16 {
                self.decisions.pop_front();
            }
            self.decisions
                .push_back((receipt.clone(), decision.clone()));
            self.selected_decision = 0;
        }
        if self.receipts.len() == 16 {
            self.receipts.pop_front();
        }
        self.receipts.push_back(receipt);
        self.dirty = true;
        Ok(())
    }

    fn poll_step(&mut self, budget: usize) -> Result<u64, String> {
        self.last_step_visits = 0;
        if self.due_ticks == 0 {
            return Ok(0);
        }
        if self
            .step_job
            .as_ref()
            .is_some_and(|job| !job.is_current(&self.authority))
            && let Some(mut job) = self.step_job.take()
        {
            job.cancel();
        }
        let mut job = match self.step_job.take() {
            Some(job) => job,
            None => match self.authority.begin_step() {
                Ok(job) => job,
                Err(error) => {
                    let fault = error.to_string();
                    self.set_paused(true);
                    self.fault = Some(fault.clone());
                    return Err(fault);
                }
            },
        };
        let progress = job.poll(budget);
        self.last_step_visits = job.last_poll_visits();
        match progress {
            Ok(LivingStepProgressV2::Pending) => {
                self.step_job = Some(job);
                return Ok(0);
            }
            Ok(
                LivingStepProgressV2::NeedsSynchronousCompletion
                | LivingStepProgressV2::NeedsSynchronousCreatorOperation,
            )
            | Err(LivingStepJobErrorV2::Fault(_)) => {}
            Err(error) => return Err(error.to_string()),
        }
        // Transitional API: due intents run before dispatch; sealing and
        // delegated creator work are explicitly outside counted poll visits.
        let preparation = match job.finish_synchronously(&self.authority) {
            Ok(preparation) => preparation,
            Err(LivingStepJobErrorV2::Stale) => return Ok(0),
            Err(error) => return Err(error.to_string()),
        };
        let (publication, fault) = match preparation {
            LivingStepPreparationV2::Ready(prepared) => {
                (self.authority.commit_step(prepared), None)
            }
            LivingStepPreparationV2::Failed(failure) => {
                let fault = failure.fault().to_string();
                (self.authority.commit_step_failure(failure), Some(fault))
            }
        };
        match publication {
            Ok(retired) => drop(retired), // Reclamation remains outside the commit fence.
            Err(rejection) if rejection.reason() == &LivingStepJobErrorV2::Stale => return Ok(0),
            Err(rejection) => return Err(rejection.to_string()),
        }
        if let Some(fault) = fault {
            self.set_paused(true);
            self.fault = Some(fault.clone());
            return Err(fault);
        }
        self.due_ticks -= 1;
        if self.due_ticks == 0 {
            self.explicit_step = false;
        }
        self.record_step_publication()?;
        Ok(1)
    }

    fn poll_history(&mut self) -> Result<(), String> {
        if let Some(mut job) = self.history_job.take() {
            match job.poll(128).map_err(|error| error.to_string())? {
                LivingArchiveReplayStatusV2::Pending => self.history_job = Some(job),
                LivingArchiveReplayStatusV2::Ready => {
                    let next = job
                        .finish()
                        .map_err(|error| error.to_string())?
                        .into_authority();
                    self.cancel_step();
                    self.authority = next;
                    self.recover_selection();
                    self.starter_guidance = None;
                    self.receipts.clear();
                    self.chronicle.clear();
                    self.decisions.clear();
                    self.selected_decision = 0;
                    self.creator_preview = None;
                    self.fault = None;
                    self.seed_latest_boundary()?;
                    self.dirty = true;
                    if self.present == self.bookmark().ok() {
                        self.present = None;
                    }
                    self.history_message = Some(
                        if self.authority.is_viewing_history() {
                            "Viewing history; creator inputs require Branch here."
                        } else {
                            "Selected history reconstructed and validated."
                        }
                        .to_owned(),
                    );
                }
            }
        }
        if let Some(mut job) = self.comparison_job.take() {
            match job.poll(128).map_err(|error| error.to_string())? {
                LivingArchiveReplayStatusV2::Pending => self.comparison_job = Some(job),
                LivingArchiveReplayStatusV2::Ready => {
                    self.comparison = Some(job.finish().map_err(|error| error.to_string())?);
                    self.history_message =
                        Some("Matching-tick comparison ready; live state unchanged.".to_owned());
                }
            }
        }
        Ok(())
    }

    /// Request ten boundaries per wall second at 1x. At most twenty requests
    /// survive an update; one 1024-record budget is shared by resident work.
    /// The transitional executor completes at most one synchronous suffix per
    /// update. This is a record budget, not a whole-frame latency guarantee.
    pub fn update(&mut self, elapsed: Duration) -> Result<u64, String> {
        self.last_step_visits = 0;
        self.poll_history()?;
        if self.suspended
            || !self.holds.is_empty()
            || self.fault.is_some()
            || self.authority.is_viewing_history()
            || (self.paused && !self.explicit_step)
        {
            self.cancel_step();
            return Ok(0);
        }
        let mut capped = false;
        if !self.paused {
            self.debt_nanos = self.debt_nanos.saturating_add(
                elapsed
                    .as_nanos()
                    .saturating_mul(u128::from(self.speed.multiplier())),
            );
            let requested = self.debt_nanos / TICK_NANOS;
            capped = requested + u128::from(self.due_ticks) > 20;
            self.due_ticks = (u128::from(self.due_ticks) + requested).min(20) as u64;
            self.debt_nanos %= TICK_NANOS;
        }
        let committed = self.poll_step(1024)?;
        self.limited_speed =
            self.publication_pace
                .record(elapsed, committed, capped || self.processing());
        Ok(committed)
    }

    fn inspector_lines(&self) -> Vec<String> {
        let state = self.state();
        let selected = self
            .selected_world
            .and_then(|id| state.worlds.as_slice().iter().find(|row| row.id == id))
            .or_else(|| state.worlds.as_slice().first());
        let mut lines = self
            .selected_entity()
            .and_then(|selection| {
                crate::living_navigation::entity_details(state, self.authority.catalog(), selection)
            })
            .unwrap_or_default();
        if let Some(selection) = self.selected_entity()
            && selection.kind == LivingEntityKind::Civilization
        {
            lines.push("Recorded civilization intentions / limited snapshot".to_owned());
            for (receipt, decision) in &self.decisions {
                for intent in decision
                    .intents
                    .iter()
                    .filter(|intent| intent.actor == selection.id)
                {
                    lines.push(format!(
                        "Ordinary decision receipt {} / tick {}",
                        hex(&receipt.digest.0),
                        receipt.tick.0
                    ));
                    lines.extend(decision_lines(intent));
                }
            }
        }
        if state.tick.0 == 0
            && selected.is_some_and(|world| {
                world.name.as_str() == "Aster Vale" && world.inventory.energy == 0
            })
        {
            lines.push("Vale Confluence: Aster Vale has no stored power or solar array. Its extractor needs 2 energy and its foundry needs 4 energy plus 4 ore. Preview a solar array or a supply route, then compare actual inventory and blocked reasons after running.".to_owned());
            lines.push("Guide: select a world; Run or Step time; open Creator tools and Preview before Apply. Tools are available immediately.".to_owned());
        }
        if let Some(fault) = &self.fault {
            lines.push(format!("Simulation stopped: {fault}"));
        }
        if let Some((receipt, decision)) = self.decisions.back()
            && let Some(intent) = decision.intents.get(
                self.selected_decision
                    .min(decision.intents.len().saturating_sub(1)),
            )
        {
            lines.push(format!(
                "Decision {} of {} / tick {}",
                self.selected_decision + 1,
                decision.intents.len(),
                decision.tick.0
            ));
            lines.push(format!("Ordinary receipt {}", hex(&receipt.digest.0)));
            lines.extend(decision_lines(intent));
        }
        if let Some(world) = selected {
            lines.push(format!("WORLD {}", world.name.as_str()));
            let owner = world.owner.and_then(|id| {
                state
                    .civilizations
                    .as_slice()
                    .iter()
                    .find(|row| row.id == id)
            });
            lines.push(format!(
                "Owner: {}",
                owner.map_or("Unowned", |row| row.name.as_str())
            ));
            lines.push(format!(
                "Energy {} / Ore {} / Alloy {}",
                world.inventory.energy, world.inventory.ore, world.inventory.alloy
            ));
            if let Some(colony) = state
                .colonies
                .as_slice()
                .iter()
                .find(|row| row.world == world.id)
            {
                lines.push(format!(
                    "Colony hub; blocked: {:?}",
                    colony.hub.blocked_reasons.as_slice()
                ));
            } else {
                lines.push("No colony hub".to_owned());
            }
            for facility in state
                .facilities
                .as_slice()
                .iter()
                .filter(|row| row.world == world.id)
            {
                lines.push(format!(
                    "{}: {} / {:?}; due {:?}; blocked {:?}",
                    facility.definition.as_str(),
                    if facility.disabled {
                        "disabled"
                    } else {
                        "enabled"
                    },
                    facility.status,
                    facility.next_due.map(|tick| tick.0),
                    facility.blocked_reasons.as_slice()
                ));
            }
            for job in state
                .construction_jobs
                .as_slice()
                .iter()
                .filter(|row| row.world == world.id)
            {
                lines.push(format!(
                    "Build {}: accepted {}; completes {}",
                    job.definition.as_str(),
                    job.accepted_tick.0,
                    job.completion_tick.0
                ));
            }
            for job in state
                .hull_jobs
                .as_slice()
                .iter()
                .filter(|row| row.world == world.id)
            {
                lines.push(format!(
                    "Hull job: accepted {}; completes {}",
                    job.accepted_tick.0, job.completion_tick.0
                ));
            }
        } else {
            lines.push("Galaxy has no worlds".to_owned());
        }
        lines.push("CHRONICLE / newest first".to_owned());
        lines.push("Cache: latest 16 receipts; latest 256 events".to_owned());
        if let Some(comparison) = &self.comparison {
            lines.push(format!("Comparison at tick {}", comparison.tick.0));
            lines.push(format!("Left digest {}", hex(&comparison.left.digest.0)));
            lines.push(format!("Right digest {}", hex(&comparison.right.digest.0)));
            lines.extend(projection_changes(
                &comparison.left_state,
                &comparison.right_state,
            ));
        }
        for experiment in &self.notebook_experiments {
            lines.push(format!(
                "Notebook: {} / matching tick {}",
                experiment.label.as_str(),
                experiment.comparison_tick.0
            ));
            for observation in &experiment.observations {
                lines.push(format!(
                    "{} {}: left {}; right {}",
                    notebook_metric_label(observation.metric),
                    observation
                        .entity
                        .map_or_else(|| "galaxy".to_owned(), |entity| hex(&entity.0)),
                    notebook_value_label(&observation.left),
                    notebook_value_label(&observation.right)
                ));
            }
        }
        let mut count = 0;
        for receipt in self.receipts.iter().rev() {
            lines.push(format!(
                "Tick {}: {} recorded events",
                receipt.tick.0,
                receipt.events.len()
            ));
            for event in receipt.events.iter().rev() {
                if count == 256 {
                    lines.push("Earlier events omitted from this view".to_owned());
                    return lines;
                }
                let provenance = match event.provenance {
                    LivingEventProvenanceV2::Autonomous { phase } => {
                        format!("Autonomous / {phase:?}")
                    }
                    LivingEventProvenanceV2::Creator { revision } => {
                        format!("Creator / {}", hex(&revision.0))
                    }
                };
                lines.push(format!("{:?} / {provenance}", event.kind));
                lines.push(format!("Event {}", hex(&event.id.0)));
                count += 1;
            }
        }
        if self.history_picker {
            lines.push(format!(
                "History branch {} / cursor {} / requested tick {}",
                hex_id(&self.history_branch.0),
                self.history_revision
                    .map_or_else(|| "genesis".into(), |id| hex_id(&id.0)),
                self.history_tick
            ));
            lines.push(format!(
                "Comparison left {} / right {} / common tick {}",
                hex_id(&self.comparison_left.0),
                hex_id(&self.comparison_right.0),
                self.comparison_tick
            ));
            lines.push("Suggested experiments (record an exact bookmark and inspect its supporting receipts):".to_owned());
            for prompt in [
                "Establish a two-system alloy supply chain.",
                "Observe settlement without creator intervention.",
                "Open a trade agreement and inspect real deliveries.",
                "Disrupt a corridor and identify affected industries.",
                "Observe a war, then branch an alternative diplomatic setup.",
                "Recover a dormant civilization through settlement or intervention.",
                "Export, import, and reproduce a selected state.",
            ] {
                lines.push(prompt.to_owned());
            }
        }
        if self.receipts.is_empty() {
            lines.push("No committed tick receipts yet".to_owned());
        }
        lines
    }

    fn inspector(
        &self,
        layout: &WorkshopLayout,
        viewport: Vec2,
        top: f32,
        bottom: f32,
        controls: &mut Vec<PlatformControl>,
        nodes: &mut Vec<SemanticNode>,
    ) -> Vec<PlatformSightedText> {
        let scale = layout.ui_scale;
        let target = (44.0 * scale).max(44.0);
        let stride = target + 6.0 * scale;
        let width = (320.0 * scale).min(viewport.x - 16.0);
        let panel = PlatformRect::from_xywh(
            viewport.x - width - 8.0,
            top,
            width,
            (bottom - top).max(0.0),
        );
        if panel.height() < 132.0 * scale {
            return Vec::new();
        }
        let worlds = self.state().worlds.as_slice();
        let selected = self
            .selected_world
            .and_then(|id| worlds.iter().position(|row| row.id == id))
            .unwrap_or(0);
        let mut add_control = |id: &str,
                               label: &str,
                               action: SandboxUiAction,
                               bounds: PlatformRect,
                               enabled: bool| {
            let action_id = SemanticActionId::new(id);
            let mut node = SemanticNode::control(
                format!("{id}.node"),
                SemanticRole::Button,
                label,
                "Living Galaxy inspector",
                enabled,
                false,
                action_id.clone(),
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            controls.push(PlatformControl {
                semantic_id: SemanticNodeId::new(format!("{id}.node")),
                action_id,
                action: PlatformUiAction::Sandbox(action),
                bounds,
                label: label.to_owned(),
                description: "Living Galaxy inspector".to_owned(),
                enabled,
                selected: false,
                focused: false,
                icon: None,
            });
        };
        let half = (width - 8.0) / 2.0;
        if !worlds.is_empty() {
            let previous = worlds[(selected + worlds.len() - 1) % worlds.len()].id;
            let next = worlds[(selected + 1) % worlds.len()].id;
            add_control(
                "sandbox.world.previous",
                "Previous world",
                SandboxUiAction::SelectWorld(previous),
                PlatformRect::from_xywh(panel.min.x, panel.min.y, half, target),
                worlds.len() > 1,
            );
            add_control(
                "sandbox.world.next",
                "Next world",
                SandboxUiAction::SelectWorld(next),
                PlatformRect::from_xywh(panel.min.x + half + 8.0, panel.min.y, half, target),
                worlds.len() > 1,
            );
        }
        let has_decisions = self
            .decisions
            .back()
            .is_some_and(|(_, receipt)| !receipt.intents.is_empty());
        if let Some((_, receipt)) = self.decisions.back()
            && !receipt.intents.is_empty()
        {
            for (index, (id, label, back, enabled)) in [
                (
                    "sandbox.decision.previous",
                    "Previous decision",
                    true,
                    self.selected_decision > 0,
                ),
                (
                    "sandbox.decision.next",
                    "Next decision",
                    false,
                    self.selected_decision + 1 < receipt.intents.len(),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                add_control(
                    id,
                    label,
                    SandboxUiAction::DecisionPage(back),
                    PlatformRect::from_xywh(
                        panel.min.x + index as f32 * (half + 8.0),
                        panel.min.y + stride,
                        half,
                        target,
                    ),
                    enabled,
                );
            }
        }
        let header = if has_decisions {
            2.0 * stride - 2.0 * scale
        } else {
            target + 4.0 * scale
        };
        let metrics = AtlasMetrics::embedded().expect("embedded atlas metrics");
        let mut wrapped = Vec::new();
        for source in self.inspector_lines() {
            let mut line = String::new();
            for character in source.chars() {
                let candidate = format!("{line}{character}");
                if !line.is_empty()
                    && metrics
                        .measure_text(15.0 * scale, FontWeight::Regular, &candidate)
                        .map_or(true, |measure| measure.advance > width - 12.0)
                {
                    wrapped.push(std::mem::take(&mut line));
                }
                line.push(character);
            }
            wrapped.push(line);
        }
        let line_height = 20.25 * scale;
        let capacity = ((panel.height() - header - target - 4.0 * scale) / line_height)
            .floor()
            .max(1.0) as usize;
        let pages = wrapped.len().div_ceil(capacity).max(1);
        let page = self.inspector_page.min(pages - 1);
        add_control(
            "sandbox.inspector.previous",
            "Previous page",
            SandboxUiAction::InspectorPage(true),
            PlatformRect::from_xywh(panel.min.x, panel.max.y - target, half, target),
            page > 0,
        );
        add_control(
            "sandbox.inspector.next",
            "Next page",
            SandboxUiAction::InspectorPage(false),
            PlatformRect::from_xywh(panel.min.x + half + 8.0, panel.max.y - target, half, target),
            page + 1 < pages,
        );
        let mut sighted = Vec::new();
        for (offset, line) in wrapped
            .iter()
            .skip(page * capacity)
            .take(capacity)
            .enumerate()
        {
            let id = format!("sandbox.inspector.line.{}", page * capacity + offset);
            let bounds = PlatformRect::from_xywh(
                panel.min.x + 4.0,
                panel.min.y + header + offset as f32 * line_height,
                width - 8.0,
                line_height,
            );
            let mut node = SemanticNode::text(
                &id,
                SemanticRole::Text,
                line,
                "Authoritative world and receipt projection",
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            sighted.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new(id),
                kind: InspectorTextKind::Fact,
                label: line.clone(),
                value: None,
                lines: vec![line.clone()],
                bounds,
                clip: panel,
            });
        }
        sighted
    }

    fn notebook_overlay(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        let scale = scale.max(1.0);
        let width = (720.0 * scale).min(viewport.x - 32.0);
        let height = (640.0 * scale).min(viewport.y - 32.0);
        let modal = PlatformRect::from_xywh(
            (viewport.x - width) / 2.0,
            (viewport.y - height) / 2.0,
            width,
            height,
        );
        let target = 44.01 * scale;
        let stride = target + 6.0 * scale;
        let mut nodes = Vec::new();
        let mut controls = Vec::new();
        let mut text = Vec::new();
        let heading = "Experiments notebook";
        let bounds =
            PlatformRect::from_xywh(modal.min.x + 12.0, modal.min.y + 12.0, width - 24.0, target);
        let mut node = SemanticNode::text(
            "sandbox.notebook.title",
            SemanticRole::Heading,
            heading,
            "Optional evidence-backed experiments; no rewards or failure timer",
        );
        node.bounds = Some(bounds.into());
        nodes.push(node);
        text.push(PlatformSightedText {
            semantic_id: SemanticNodeId::new("sandbox.notebook.title"),
            kind: InspectorTextKind::Fact,
            label: heading.into(),
            value: None,
            lines: wrap_text(heading, width - 24.0, scale),
            bounds,
            clip: modal,
        });
        let mut rows: Vec<(String, String, SandboxUiAction, bool, Option<String>)> = Vec::new();
        for (ordinal, chunk) in
            ui_text_chunks("Optional experiments; no rewards or failure timer.").enumerate()
        {
            rows.push((
                format!("fact.scope.{ordinal}"),
                chunk.to_owned(),
                SandboxUiAction::CreatorField(0),
                false,
                None,
            ));
        }
        if let Some(index) = self.notebook_details {
            rows.push((
                "back".into(),
                "Back to all experiments".into(),
                SandboxUiAction::ExperimentDetails(None),
                true,
                None,
            ));
            let text = format!(
                "{} {}",
                EXPERIMENTS[usize::from(index)],
                EXPERIMENT_GUIDANCE[usize::from(index)]
            );
            for (ordinal, chunk) in ui_text_chunks(&text).enumerate() {
                rows.push((
                    format!("fact.prompt.{ordinal}"),
                    chunk.to_owned(),
                    SandboxUiAction::CreatorField(0),
                    false,
                    None,
                ));
            }
            let witness = self.experiment_receipt_witness(index);
            let text = match &witness {
                Ok(value) => format!(
                    "Available exact witness: {value}. Recording preserves this boundary; it does not certify every experiment condition."
                ),
                Err(reason) => format!("Not certified: {reason}"),
            };
            for (ordinal, chunk) in ui_text_chunks(&text).enumerate() {
                rows.push((
                    format!("fact.evidence.{ordinal}"),
                    chunk.to_owned(),
                    SandboxUiAction::CreatorField(0),
                    false,
                    None,
                ));
            }
            rows.push((
                "record".into(),
                "Record available receipt witness".into(),
                SandboxUiAction::RecordExperiment(index),
                witness.is_ok(),
                None,
            ));
            if index == 0 {
                rows.push((
                    "supply.verify".into(),
                    "Verify supply observations".into(),
                    SandboxUiAction::VerifyNotebookEvidence,
                    !self.storage_busy
                        && self
                            .notebook_evidence
                            .as_ref()
                            .is_some_and(|section| !section.records.is_empty()),
                    None,
                ));
                if let Some(proof) = self.notebook_supply_completions.first() {
                    let message = format!(
                        "Replay-verified: Alloy production and matching inter-system delivery observed. Branch {}; production tick {}; delivery tick {}; facility {}; {} ore consumed, {} alloy produced; {} {} delivered. This does not identify the same fungible units or establish dispatch causation.",
                        hex(&proof.production_view().branch.0),
                        proof.production_view().tick.0,
                        proof.delivery_view().tick.0,
                        hex(&proof.production_facility().0),
                        proof.ore_input(),
                        proof.alloy_output(),
                        proof.delivery().units,
                        proof.delivery().resource.as_wire_str(),
                    );
                    for (ordinal, chunk) in ui_text_chunks(&message).enumerate() {
                        rows.push((
                            format!("fact.supply.complete.{ordinal}"),
                            chunk.to_owned(),
                            SandboxUiAction::CreatorField(0),
                            false,
                            None,
                        ));
                    }
                    for (name, bookmark) in [
                        ("production", proof.production_bookmark()),
                        ("delivery", proof.delivery_bookmark()),
                    ] {
                        if let Some(position) = self
                            .named_bookmarks
                            .iter()
                            .position(|row| row.ordinal == bookmark)
                        {
                            rows.push((
                                format!("supply.open.{name}"),
                                format!("Open verified supply {name} bookmark"),
                                SandboxUiAction::RestoreBookmark(position as u8),
                                self.history_job.is_none(),
                                None,
                            ));
                        }
                    }
                } else {
                    rows.push(("fact.supply.incomplete".into(),
                        "Supply completion waits for replay-verified matching alloy production and inter-system delivery observations.".into(),
                        SandboxUiAction::CreatorField(0), false, None));
                }
            }
            if index == 3 {
                rows.push((
                    "corridor.verify".into(),
                    "Verify matching corridor observations".into(),
                    SandboxUiAction::VerifyNotebookEvidence,
                    !self.storage_busy
                        && self
                            .notebook_evidence
                            .as_ref()
                            .is_some_and(|section| !section.records.is_empty()),
                    None,
                ));
                if let Some(proof) = self.notebook_corridor_observations.first() {
                    let restriction = proof.restriction();
                    let message = format!(
                        "Replay-verified: Restricted dispatch and matching later input wait observed. Branch {}; dispatch tick {}; wait tick {}; route {}; hazard {}; eligible {} units, restricted {} units, actually dispatched {} units; facility {} needs {} {}, available {}. This does not establish sole causation or complete the controlled corridor experiment.",
                        hex(&proof.attempt_view().branch.0),
                        proof.attempt_view().tick.0,
                        proof.wait_view().tick.0,
                        hex(&proof.attempt_locator().route.0),
                        hex(&proof.attempt_locator().hazard.0),
                        restriction.eligible_batch(),
                        restriction.restricted_batch(),
                        restriction.actual_dispatched_units(),
                        hex(&proof.facility().0),
                        proof.required(),
                        proof.resource().as_wire_str(),
                        proof.available(),
                    );
                    for (ordinal, chunk) in ui_text_chunks(&message).enumerate() {
                        rows.push((
                            format!("fact.corridor.observed.{ordinal}"),
                            chunk.to_owned(),
                            SandboxUiAction::CreatorField(0),
                            false,
                            None,
                        ));
                    }
                    for (name, label, bookmark) in [
                        ("dispatch", "dispatch", proof.attempt_bookmark()),
                        ("input_wait", "input wait", proof.wait_bookmark()),
                    ] {
                        if let Some(position) = self
                            .named_bookmarks
                            .iter()
                            .position(|row| row.ordinal == bookmark)
                        {
                            rows.push((
                                format!("corridor.open.{name}"),
                                format!("Open verified corridor {label} bookmark"),
                                SandboxUiAction::RestoreBookmark(position as u8),
                                self.history_job.is_none(),
                                None,
                            ));
                        }
                    }
                } else {
                    rows.push(("fact.corridor.incomplete".into(),
                        "Matching corridor observations wait for replay-verified restricted dispatch and a later input wait on the same resource and destination.".into(),
                        SandboxUiAction::CreatorField(0), false, None));
                }
            }
            if index == 2 {
                rows.push((
                    "trade.verify".into(),
                    "Verify matching trade observations".into(),
                    SandboxUiAction::VerifyNotebookEvidence,
                    !self.storage_busy
                        && self
                            .notebook_evidence
                            .as_ref()
                            .is_some_and(|section| !section.records.is_empty()),
                    None,
                ));
                if let Some(proof) = self.notebook_trade_completions.first() {
                    let message = format!(
                        "Replay-verified: Agreement and matching delivery observed. Branch {}; agreement tick {}; accepted delivery tick {}; {} {}. This does not establish that the agreement caused dispatch.",
                        hex(&proof.delivery_view().branch.0),
                        proof.formation_view().tick.0,
                        proof.delivery_view().tick.0,
                        proof.delivery().units,
                        proof.delivery().resource.as_wire_str(),
                    );
                    for (ordinal, chunk) in ui_text_chunks(&message).enumerate() {
                        rows.push((
                            format!("fact.trade.complete.{ordinal}"),
                            chunk.to_owned(),
                            SandboxUiAction::CreatorField(0),
                            false,
                            None,
                        ));
                    }
                    for (name, bookmark) in [
                        ("formation", proof.formation_bookmark()),
                        ("delivery", proof.delivery_bookmark()),
                    ] {
                        if let Some(position) = self
                            .named_bookmarks
                            .iter()
                            .position(|row| row.ordinal == bookmark)
                        {
                            rows.push((
                                format!("trade.open.{name}"),
                                format!("Open verified trade {name} bookmark"),
                                SandboxUiAction::RestoreBookmark(position as u8),
                                self.history_job.is_none(),
                                None,
                            ));
                        }
                    }
                } else {
                    rows.push(("fact.trade.incomplete".into(), "Linked trade completion waits for replay-verified matching formation and delivery observations.".into(), SandboxUiAction::CreatorField(0), false, None));
                }
            }
        } else {
            for (index, prompt) in EXPERIMENTS.iter().enumerate() {
                rows.push((
                    format!("experiment.{index}"),
                    format!("{}. {prompt}", index + 1),
                    SandboxUiAction::ExperimentDetails(Some(index as u8)),
                    true,
                    None,
                ));
            }
            rows.push((
                "field".into(),
                "Bookmark / comparison label".into(),
                SandboxUiAction::CreatorField(0),
                true,
                Some(self.bookmark_label.clone()),
            ));
            rows.push((
                "bookmark".into(),
                "Record exact before / after state".into(),
                SandboxUiAction::RecordBookmark,
                self.bookmarks.len() < 64,
                None,
            ));
            rows.push((
                "history".into(),
                "Choose branches and matching tick".into(),
                SandboxUiAction::HistoryPicker,
                true,
                None,
            ));
            rows.push((
                "comparison".into(),
                "Record completed comparison measurements".into(),
                SandboxUiAction::RecordComparison,
                self.comparison.is_some(),
                None,
            ));
            if let Some(message) = &self.history_message {
                for (ordinal, chunk) in ui_text_chunks(message).enumerate() {
                    rows.push((
                        format!("fact.message.{ordinal}"),
                        chunk.to_owned(),
                        SandboxUiAction::CreatorField(0),
                        false,
                        None,
                    ));
                }
            }
            for (index, bookmark) in self.named_bookmarks.iter().enumerate() {
                rows.push((
                    format!("bookmark.open.{index}"),
                    format!(
                        "{} / tick {}",
                        bookmark.label.as_str(),
                        bookmark.view.tick.0
                    ),
                    SandboxUiAction::RestoreBookmark(index as u8),
                    self.history_job.is_none(),
                    None,
                ));
                let receipt = self.bookmark_receipts.get(index).and_then(Option::as_ref);
                let references: Vec<_> = self
                    .notebook_evidence
                    .as_ref()
                    .into_iter()
                    .flat_map(|section| &section.records)
                    .filter(|record| record.bookmark == bookmark.ordinal)
                    .collect();
                let supporting = if !references.is_empty() {
                    references.iter().map(|record| {
                        let events = record.events.iter().map(|event| format!("{} ordinal {} kind {}", hex(&event.id.0), event.ordinal, event.kind.as_wire_str())).collect::<Vec<_>>().join("; ");
                        let attempts = record.route_attempts.as_deref().unwrap_or_default().iter().map(|attempt| format!("route {} / hazard {}", hex(&attempt.route.0), hex(&attempt.hazard.0))).collect::<Vec<_>>().join("; ");
                        format!("{} observation {:?}; receipt {}; events {}; corridor route attempts {}. No broader completion or causal claim.", if self.notebook_replay_verified.contains(&record.ordinal) { "Replay-verified" } else { "Current authoritative" }, record.experiment, hex(&record.receipt_digest.0), events, attempts)
                    }).collect::<Vec<_>>().join(" ")
                } else {
                    receipt.map_or_else(|| "Supporting receipt unavailable in this session cache; no outcome certification".into(), |receipt| format!("receipt {}", hex(&receipt.digest.0)))
                };
                let fact = format!(
                    "Branch {} / cursor {} / tick {} / state {} / {}",
                    hex_id(&bookmark.view.branch.0),
                    bookmark
                        .view
                        .revision
                        .map_or_else(|| "genesis".into(), |id| hex_id(&id.0)),
                    bookmark.view.tick.0,
                    hex(&bookmark.digest.0),
                    supporting
                );
                for (ordinal, chunk) in ui_text_chunks(&fact).enumerate() {
                    rows.push((
                        format!("fact.bookmark.{index}.{ordinal}"),
                        chunk.to_owned(),
                        SandboxUiAction::CreatorField(0),
                        false,
                        None,
                    ));
                }
            }
            for experiment in &self.notebook_experiments {
                let fact = format!(
                    "Comparison {} / common tick {} / bookmark {} versus {} / measurements {:?}. A single comparison is not a general causal law.",
                    experiment.label.as_str(),
                    experiment.comparison_tick.0,
                    experiment.left,
                    experiment.right,
                    experiment.observations
                );
                for (ordinal, chunk) in ui_text_chunks(&fact).enumerate() {
                    rows.push((
                        format!("fact.comparison.{}.{ordinal}", experiment.ordinal),
                        chunk.to_owned(),
                        SandboxUiAction::CreatorField(0),
                        false,
                        None,
                    ));
                }
            }
        }
        if self.notebook_details.is_some() {
            rows.sort_by_key(|(suffix, _, _, _, _)| match suffix.as_str() {
                "back" => 0,
                "record" => 1,
                "trade.verify" | "supply.verify" => 2,
                name if name.starts_with("trade.open.") || name.starts_with("supply.open.") => 3,
                _ => 4,
            });
        }
        let capacity = ((height - 176.0 * scale) / stride).floor().max(1.0) as usize;
        let pages = rows.len().div_ceil(capacity).max(1);
        let page = self.inspector_page.min(pages - 1);
        for (index, (suffix, label, action, enabled, value)) in
            rows.iter().enumerate().skip(page * capacity).take(capacity)
        {
            let id = if suffix == "field" {
                "sandbox.creator.field.bookmark_label".into()
            } else {
                format!("sandbox.notebook.{suffix}")
            };
            push_control(
                &mut controls,
                &mut nodes,
                &id,
                label,
                *action,
                PlatformRect::from_xywh(
                    modal.min.x + 12.0,
                    modal.min.y + 64.0 * scale + (index - page * capacity) as f32 * stride,
                    width - 24.0,
                    target,
                ),
                *enabled,
                if value.is_some() {
                    SemanticRole::TextInput
                } else if !enabled && suffix.starts_with("fact.") {
                    SemanticRole::Status
                } else {
                    SemanticRole::Button
                },
                value.as_deref(),
            );
        }
        let footer = modal.max.y - 100.0 * scale;
        for (index, (suffix, label, action, enabled)) in [
            (
                "previous",
                "Previous page",
                SandboxUiAction::InspectorPage(true),
                page > 0,
            ),
            (
                "next",
                "Next page",
                SandboxUiAction::InspectorPage(false),
                page + 1 < pages,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            push_control(
                &mut controls,
                &mut nodes,
                &format!("sandbox.notebook.{suffix}"),
                label,
                action,
                PlatformRect::from_xywh(
                    modal.min.x + 12.0 + index as f32 * (width - 24.0) / 2.0,
                    footer,
                    (width - 32.0) / 2.0,
                    target,
                ),
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        push_control(
            &mut controls,
            &mut nodes,
            "sandbox.notebook.close",
            "Hide notebook",
            SandboxUiAction::CreatorCancel,
            PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.max.y - 48.0 * scale,
                width - 24.0,
                target,
            ),
            true,
            SemanticRole::Button,
            None,
        );
        (modal, nodes, controls, text)
    }

    fn pack_overlay(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        let scale = scale.max(1.0);
        let width = (720.0 * scale).min(viewport.x - 32.0);
        let height = (640.0 * scale).min(viewport.y - 32.0);
        let modal = PlatformRect::from_xywh(
            (viewport.x - width) / 2.0,
            (viewport.y - height) / 2.0,
            width,
            height,
        );
        let target = 44.01 * scale;
        let stride = target + 6.0 * scale;
        let mut nodes = Vec::new();
        let mut controls = Vec::new();
        let mut text = Vec::new();
        let heading = if self.pack_delete_review.is_some() {
            "Delete archived catalog pack permanently"
        } else {
            "Catalog packs / archived packs still count toward capacity"
        };
        let bounds =
            PlatformRect::from_xywh(modal.min.x + 12.0, modal.min.y + 12.0, width - 24.0, target);
        let mut node = SemanticNode::text(
            "sandbox.packs.title",
            SemanticRole::Heading,
            heading,
            "Independent Living catalog pack Library",
        );
        node.bounds = Some(bounds.into());
        nodes.push(node);
        text.push(PlatformSightedText {
            semantic_id: SemanticNodeId::new("sandbox.packs.title"),
            kind: InspectorTextKind::Fact,
            label: heading.into(),
            value: None,
            lines: wrap_text(heading, width - 24.0, scale),
            bounds,
            clip: modal,
        });
        let mut rows: Vec<(String, String, SandboxUiAction, bool, Option<String>)> = Vec::new();
        rows.push((
            "refresh".into(),
            "Refresh exact pack and retained reference lists".into(),
            SandboxUiAction::ListPacks,
            !self.pack_busy,
            None,
        ));
        if let Some(hash) = self.selected_pack {
            rows.push((
                "back".into(),
                "Back to all catalog packs".into(),
                SandboxUiAction::PackLibraryBack,
                true,
                None,
            ));
            if let Some(row) = self.pack_rows.iter().find(|row| row.hash == hash) {
                let mut facts=vec![format!("Pack {} / generation {} / {}",hex(&hash.0),row.generation.0,if row.archived {"archived"} else {"active"}),
                    "Deletion is irreversible. Export the catalog first. It never cascades into galaxies or changes Continue.".into()];
                if hash == self.builtin_pack {
                    facts.push("Built-in core catalog is protected.".into());
                }
                if hash == self.authority.catalog().catalog_hash() {
                    facts.push("Resident galaxy uses this catalog; close that resident before pack deletion.".into());
                }
                match self.pack_references.get(&hash) {
                    None => facts.push(
                        "Retained references are unknown; deletion is disabled until refreshed."
                            .into(),
                    ),
                    Some(references) if references.is_empty() => facts.push(
                        "No retained galaxies reference this catalog in the completed listing."
                            .into(),
                    ),
                    Some(references) => {
                        for reference in references {
                            facts.push(format!(
                                "Blocker: {} / slot {} / retained generation {} / {}",
                                reference.name,
                                reference.slot.0,
                                reference.retained_generation.0,
                                if reference.archived {
                                    "archived galaxy"
                                } else {
                                    "active galaxy"
                                }
                            ));
                        }
                    }
                }
                for (index, line) in facts
                    .iter()
                    .flat_map(|fact| wrap_text(fact, width - 24.0, scale))
                    .enumerate()
                {
                    rows.push((
                        format!("fact.{index}"),
                        line,
                        SandboxUiAction::CreatorField(0),
                        false,
                        None,
                    ));
                }
                if let Some(review) = &self.pack_delete_review {
                    rows.push((
                        "field".into(),
                        "Type the exact displayed 64-character pack hash".into(),
                        SandboxUiAction::CreatorField(0),
                        !self.pack_busy,
                        Some(self.pack_delete_hash.clone()),
                    ));
                    rows.push((
                        "confirm".into(),
                        "Delete this exact archived pack permanently".into(),
                        SandboxUiAction::ConfirmDeletePack {
                            hash: review.hash,
                            generation: review.generation,
                        },
                        self.pack_delete_confirmation_matches(review.hash, review.generation),
                        None,
                    ));
                } else {
                    rows.push((
                        "archive".into(),
                        if row.archived {
                            "Restore catalog pack".into()
                        } else {
                            "Archive catalog pack".into()
                        },
                        SandboxUiAction::SetPackArchived {
                            hash,
                            generation: row.generation,
                            archived: !row.archived,
                        },
                        !self.pack_busy
                            && row.diagnostic.is_none()
                            && hash != self.builtin_pack
                            && hash != self.authority.catalog().catalog_hash(),
                        None,
                    ));
                    rows.push((
                        "review".into(),
                        "Review permanent deletion / reference blockers".into(),
                        SandboxUiAction::ReviewDeletePack {
                            hash,
                            generation: row.generation,
                        },
                        !self.pack_busy && row.archived && row.diagnostic.is_none(),
                        None,
                    ));
                }
            }
        } else {
            if !self.pack_projection_known {
                rows.push((
                    "unknown".into(),
                    "Pack catalog has not been loaded; refresh to inspect it.".into(),
                    SandboxUiAction::CreatorField(0),
                    false,
                    None,
                ));
            }
            for row in &self.pack_rows {
                rows.push((
                    format!("select.{}", hex(&row.hash.0)),
                    format!(
                        "{}{}{}",
                        hex(&row.hash.0),
                        if row.archived { " / archived" } else { "" },
                        if row.diagnostic.is_some() {
                            " / recovery required"
                        } else {
                            ""
                        }
                    ),
                    SandboxUiAction::SelectPack(row.hash),
                    !self.pack_busy,
                    None,
                ));
            }
        }
        let capacity = ((height - 176.0 * scale) / stride).floor().max(1.0) as usize;
        let pages = rows.len().div_ceil(capacity).max(1);
        let page = self.inspector_page.min(pages - 1);
        for (index, (suffix, label, action, enabled, value)) in
            rows.iter().enumerate().skip(page * capacity).take(capacity)
        {
            let id = if suffix == "field" {
                "sandbox.creator.field.pack_delete_hash".into()
            } else {
                format!("sandbox.packs.{suffix}")
            };
            push_control(
                &mut controls,
                &mut nodes,
                &id,
                label,
                *action,
                PlatformRect::from_xywh(
                    modal.min.x + 12.0,
                    modal.min.y + 64.0 * scale + (index - page * capacity) as f32 * stride,
                    width - 24.0,
                    target,
                ),
                *enabled,
                if value.is_some() {
                    SemanticRole::TextInput
                } else if !enabled && suffix.starts_with("fact.") {
                    SemanticRole::Status
                } else {
                    SemanticRole::Button
                },
                value.as_deref(),
            );
        }
        let footer = modal.max.y - 100.0 * scale;
        for (index, (suffix, label, action, enabled)) in [
            (
                "previous",
                "Previous page",
                SandboxUiAction::InspectorPage(true),
                page > 0,
            ),
            (
                "next",
                "Next page",
                SandboxUiAction::InspectorPage(false),
                page + 1 < pages,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            push_control(
                &mut controls,
                &mut nodes,
                &format!("sandbox.packs.{suffix}"),
                label,
                action,
                PlatformRect::from_xywh(
                    modal.min.x + 12.0 + index as f32 * (width - 24.0) / 2.0,
                    footer,
                    (width - 32.0) / 2.0,
                    target,
                ),
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        push_control(
            &mut controls,
            &mut nodes,
            "sandbox.packs.close",
            "Close pack Library",
            SandboxUiAction::CreatorCancel,
            PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.max.y - 48.0 * scale,
                width - 24.0,
                target,
            ),
            true,
            SemanticRole::Button,
            None,
        );
        (modal, nodes, controls, text)
    }

    fn delete_overlay(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        let scale = scale.max(1.0);
        let width = (640.0 * scale).min(viewport.x - 32.0);
        let height = (640.0 * scale).min(viewport.y - 60.0);
        let modal = PlatformRect::from_xywh(
            (viewport.x - width) / 2.0,
            (viewport.y - height) / 2.0,
            width,
            height,
        );
        let mut nodes = Vec::new();
        let mut controls = Vec::new();
        let mut text = Vec::new();
        let row = self
            .delete_review
            .as_ref()
            .expect("deletion dialog owns captured descriptor");
        let continue_status = match self.living_continue_projection {
            None => "Unknown: refresh Library selection before confirming".to_owned(),
            Some(Some(id)) if id == row.id.0 => {
                "This galaxy is global Continue; selection must be cleared durably first".to_owned()
            }
            Some(_) => "This galaxy is not global Continue".to_owned(),
        };
        let paragraphs = [
            format!(
                "Delete permanently: {}. Archived galaxy slot {}, generation {}. This is irreversible. Export a backup first.",
                row.name, row.id.0, row.generation.0
            ),
            format!(
                "Content pack {} remains retained; this deletes exactly this galaxy slot, not its pack or another galaxy.",
                hex(&row.catalog_hash.0)
            ),
            format!(
                "Continue: {continue_status}. Active resident sessions are protected. A failed delete leaves the archived slot recoverable."
            ),
            format!("Type the exact displayed name: {}", row.name),
        ];
        let lines = paragraphs
            .iter()
            .flat_map(|line| wrap_text(line, width - 24.0, scale))
            .collect::<Vec<_>>();
        let capacity = ((height - 174.0 * scale) / (22.0 * scale)).floor().max(1.0) as usize;
        let pages = lines.len().div_ceil(capacity).max(1);
        let page = self.inspector_page.min(pages - 1);
        for (index, line) in lines
            .iter()
            .skip(page * capacity)
            .take(capacity)
            .enumerate()
        {
            let id = format!("sandbox.delete.fact.{index}");
            let bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + (12.0 + index as f32 * 22.0) * scale,
                width - 24.0,
                22.0 * scale,
            );
            let mut node = SemanticNode::text(
                &id,
                SemanticRole::Text,
                line,
                "Permanent deletion review of captured archived generation",
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            text.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new(id),
                kind: InspectorTextKind::Fact,
                label: line.clone(),
                value: None,
                lines: vec![line.clone()],
                bounds,
                clip: modal,
            });
        }
        let half = (width - 32.0) / 2.0;
        for (index, (id, label, action, enabled)) in [
            (
                "previous",
                "Previous",
                SandboxUiAction::InspectorPage(true),
                page > 0,
            ),
            (
                "next",
                "Next",
                SandboxUiAction::InspectorPage(false),
                page + 1 < pages,
            ),
            (
                "confirm",
                "Delete permanently",
                SandboxUiAction::ConfirmDeleteSave(row.id.0),
                self.delete_confirmation_matches(row.id.0, row.generation, &row.name)
                    && !self.storage_busy,
            ),
            ("cancel", "Cancel", SandboxUiAction::CreatorCancel, true),
        ]
        .into_iter()
        .enumerate()
        {
            let y = if index < 2 {
                modal.max.y - 150.0 * scale
            } else {
                modal.max.y - 50.0 * scale
            };
            let bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0 + (index % 2) as f32 * (half + 8.0),
                y,
                half,
                44.0 * scale,
            );
            push_control(
                &mut controls,
                &mut nodes,
                &format!("sandbox.delete.{id}"),
                label,
                action,
                bounds,
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        push_control(
            &mut controls,
            &mut nodes,
            "sandbox.creator.field.delete_name",
            "Exact name",
            SandboxUiAction::CreatorField(0),
            PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.max.y - 100.0 * scale,
                width - 24.0,
                44.0 * scale,
            ),
            true,
            SemanticRole::TextInput,
            Some(&self.delete_name),
        );
        (modal, nodes, controls, text)
    }

    fn intro_overlay(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        let scale = scale.max(1.0);
        let width = (640.0 * scale).min(viewport.x - 32.0);
        let height = (640.0 * scale).min(viewport.y - 60.0);
        let modal = PlatformRect::from_xywh(
            (viewport.x - width) / 2.0,
            (viewport.y - height) / 2.0,
            width,
            height,
        );
        let mut nodes = Vec::new();
        let mut controls = Vec::new();
        let mut text = Vec::new();
        let paragraphs = [
            "1. Select: click an object or search Navigator. Inspector reads inventories, ownership and recorded reasons. Selection leaves the galaxy unchanged.",
            "2. Time: Explore paused, Step one boundary, then Run at 1x (10 ticks/second). Pause before comparing. Chronicle records what actually happened.",
            "3. Create: open Creator tools, edit a draft, Preview its validated effects, then Apply or Cancel. History preserves branches; Save protects your session.",
        ];
        let lines = self
            .starter_guidance_lines()
            .into_iter()
            .chain(paragraphs.into_iter().map(str::to_owned))
            .flat_map(|line| wrap_text(&line, width - 24.0, scale))
            .collect::<Vec<_>>();
        let capacity = ((height - 174.0 * scale) / (22.0 * scale)).floor().max(1.0) as usize;
        let pages = lines.len().div_ceil(capacity).max(1);
        let page = self.inspector_page.min(pages - 1);
        for (index, line) in lines
            .iter()
            .skip(page * capacity)
            .take(capacity)
            .enumerate()
        {
            let id = format!("sandbox.intro.text.{index}");
            let bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + (12.0 + index as f32 * 22.0) * scale,
                width - 24.0,
                22.0 * scale,
            );
            let mut node = SemanticNode::text(
                &id,
                SemanticRole::Text,
                line,
                "Selection, time and creation introduction",
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            text.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new(id),
                kind: InspectorTextKind::Fact,
                label: line.clone(),
                value: None,
                lines: vec![line.clone()],
                bounds,
                clip: modal,
            });
        }
        let half = (width - 32.0) / 2.0;
        for (index, (id, label, action, enabled)) in [
            (
                "previous",
                "Previous",
                SandboxUiAction::InspectorPage(true),
                page > 0,
            ),
            (
                "next",
                "Next",
                SandboxUiAction::InspectorPage(false),
                page + 1 < pages,
            ),
            (
                "explore",
                "Explore paused / Dismiss",
                SandboxUiAction::IntroStart(false),
                true,
            ),
            ("run", "Run", SandboxUiAction::IntroStart(true), true),
            (
                "create",
                if self.starter_guidance.is_some() {
                    "Add power: Solar draft"
                } else {
                    "Creator tools"
                },
                SandboxUiAction::IntroCreate,
                true,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let bounds = if index < 4 {
                PlatformRect::from_xywh(
                    modal.min.x + 12.0 + (index % 2) as f32 * (half + 8.0),
                    modal.max.y - (150.0 - (index / 2) as f32 * 50.0) * scale,
                    half,
                    44.0 * scale,
                )
            } else {
                PlatformRect::from_xywh(
                    modal.min.x + 12.0,
                    modal.max.y - 50.0 * scale,
                    width - 24.0,
                    44.0 * scale,
                )
            };
            push_control(
                &mut controls,
                &mut nodes,
                &format!("sandbox.intro.{id}"),
                label,
                action,
                bounds,
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        (modal, nodes, controls, text)
    }

    fn chronicle_overlay(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        let scale = scale.max(1.0);
        let width = (640.0 * scale).min(viewport.x - 32.0);
        let height = (640.0 * scale).min(viewport.y - 60.0);
        let modal = PlatformRect::from_xywh(
            (viewport.x - width) / 2.0,
            (viewport.y - height) / 2.0,
            width,
            height,
        );
        let mut nodes = Vec::new();
        let mut controls = Vec::new();
        let mut text = Vec::new();
        if let Some(index) = self.chronicle_selected
            && let Some(row) = self.chronicle_rows().get(usize::from(index))
        {
            let detail = format!(
                "{}\nCategory {}. Tick {} through {}. Count {}.\nActor {:?}; entity {:?}.\nReceipt {:?}. First receipt {:?}.\nEvent ordinals {:?} through {:?}; IDs {:?} through {:?}.\nProvenance {:?}. Decision ordinal {:?}, phase {:?}, score {:?}, outcome {:?}.",
                row.label,
                row.category.label(),
                row.tick_first,
                row.tick_last,
                row.count,
                row.actor,
                row.entity,
                row.receipt_digest,
                row.first_receipt_digest,
                row.event_ordinal_first,
                row.event_ordinal_last,
                row.event_id_first,
                row.event_id_last,
                row.provenance,
                row.decision_ordinal,
                row.decision_phase,
                row.score,
                row.outcome
            );
            let lines = detail
                .lines()
                .flat_map(|line| wrap_text(line, width - 24.0, scale))
                .collect::<Vec<_>>();
            let capacity = ((height - 114.0 * scale) / (22.0 * scale)).floor().max(1.0) as usize;
            let pages = lines.len().div_ceil(capacity).max(1);
            let page = self.inspector_page.min(pages - 1);
            for (local, line) in lines
                .iter()
                .skip(page * capacity)
                .take(capacity)
                .enumerate()
            {
                let id = format!("sandbox.chronicle.detail.{local}");
                let bounds = PlatformRect::from_xywh(
                    modal.min.x + 12.0,
                    modal.min.y + (12.0 + local as f32 * 22.0) * scale,
                    width - 24.0,
                    22.0 * scale,
                );
                let mut node = SemanticNode::text(
                    &id,
                    SemanticRole::Text,
                    line,
                    "Exact Chronicle receipt references",
                );
                node.bounds = Some(bounds.into());
                nodes.push(node);
                text.push(PlatformSightedText {
                    semantic_id: SemanticNodeId::new(id),
                    kind: InspectorTextKind::Fact,
                    label: line.clone(),
                    value: None,
                    lines: vec![line.clone()],
                    bounds,
                    clip: modal,
                });
            }
            for (index, (id, label, action, enabled)) in [
                (
                    "previous",
                    "Previous",
                    SandboxUiAction::InspectorPage(true),
                    page > 0,
                ),
                (
                    "next",
                    "Next",
                    SandboxUiAction::InspectorPage(false),
                    page + 1 < pages,
                ),
                (
                    "back",
                    "Back to Chronicle",
                    SandboxUiAction::ChronicleInspect(None),
                    true,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let bounds = if index < 2 {
                    PlatformRect::from_xywh(
                        modal.min.x + 12.0 + index as f32 * (width - 24.0) / 2.0,
                        modal.max.y - 100.0 * scale,
                        (width - 32.0) / 2.0,
                        44.0 * scale,
                    )
                } else {
                    PlatformRect::from_xywh(
                        modal.min.x + 12.0,
                        modal.max.y - 50.0 * scale,
                        width - 24.0,
                        44.0 * scale,
                    )
                };
                push_control(
                    &mut controls,
                    &mut nodes,
                    &format!("sandbox.chronicle.detail.{id}"),
                    label,
                    action,
                    bounds,
                    enabled,
                    SemanticRole::Button,
                    None,
                );
            }
            return (modal, nodes, controls, text);
        }
        let mut actions = vec![(
            "All categories".to_owned(),
            SandboxUiAction::ChronicleCategory(None),
        )];
        for category in ChronicleCategory::ALL {
            actions.push((
                format!("Filter: {}", category.label()),
                SandboxUiAction::ChronicleCategory(Some(category)),
            ));
            actions.push((
                format!(
                    "Pause on {}: {}",
                    category.label(),
                    if self.chronicle_pause.contains(&category) {
                        "on"
                    } else {
                        "off"
                    }
                ),
                SandboxUiAction::ChroniclePause(category),
            ));
        }
        actions.push((
            if self.chronicle_filter.actor.is_some() {
                "Stop following actor"
            } else {
                "Follow selected civilization"
            }
            .to_owned(),
            SandboxUiAction::ChronicleFollowActor(self.chronicle_filter.actor.is_none()),
        ));
        actions.push((
            if self.chronicle_filter.entity.is_some() {
                "Stop following entity"
            } else {
                "Follow selected entity"
            }
            .to_owned(),
            SandboxUiAction::ChronicleFollowEntity(self.chronicle_filter.entity.is_none()),
        ));
        let rows = self.chronicle_rows();
        let capacity = ((height - 164.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize;
        if !self.chronicle_options {
            actions.clear();
        }
        let total = actions.len()
            + if self.chronicle_options {
                0
            } else {
                rows.len()
            };
        let pages = total.div_ceil(capacity).max(1);
        let page = self.inspector_page.min(pages - 1);
        let header = format!(
            "Chronicle: {} rows. Ordinary event actor/entity unknown. Page {}/{}",
            rows.len(),
            page + 1,
            pages
        );
        push_control(
            &mut controls,
            &mut nodes,
            "sandbox.chronicle.options",
            &format!(
                "{} · {}",
                if self.chronicle_options {
                    "Show Chronicle"
                } else {
                    "Filters & pause categories"
                },
                header
            ),
            SandboxUiAction::ChronicleOptions(!self.chronicle_options),
            PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + 12.0,
                width - 24.0,
                44.0 * scale,
            ),
            true,
            SemanticRole::Button,
            None,
        );
        for index in (page * capacity)..((page + 1) * capacity).min(total) {
            let bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + (64.0 + (index - page * capacity) as f32 * 50.0) * scale,
                width - 24.0,
                44.0 * scale,
            );
            let id = format!("sandbox.chronicle.row.{index}");
            if let Some((label, action)) = actions.get(index) {
                let enabled = match action {
                    SandboxUiAction::ChronicleFollowActor(true) => self
                        .selected_entity()
                        .is_some_and(|s| s.kind == LivingEntityKind::Civilization),
                    SandboxUiAction::ChronicleFollowEntity(true) => {
                        self.selected_entity().is_some()
                    }
                    _ => true,
                };
                push_control(
                    &mut controls,
                    &mut nodes,
                    &id,
                    label,
                    *action,
                    bounds,
                    enabled,
                    SemanticRole::Button,
                    None,
                );
            } else {
                let row = rows[index - actions.len()];
                let label = format!(
                    "Tick {}–{} · {} · {}",
                    row.tick_first,
                    row.tick_last,
                    row.category.label(),
                    row.label
                );
                push_control(
                    &mut controls,
                    &mut nodes,
                    &id,
                    &label,
                    SandboxUiAction::ChronicleInspect(Some((index - actions.len()) as u16)),
                    bounds,
                    true,
                    SemanticRole::Button,
                    None,
                );
            }
        }
        for (index, (id, label, action, enabled)) in [
            (
                "previous",
                "Previous page",
                SandboxUiAction::InspectorPage(true),
                page > 0,
            ),
            (
                "next",
                "Next page",
                SandboxUiAction::InspectorPage(false),
                page + 1 < pages,
            ),
            ("close", "Close", SandboxUiAction::CreatorCancel, true),
        ]
        .into_iter()
        .enumerate()
        {
            let bounds = if index < 2 {
                PlatformRect::from_xywh(
                    modal.min.x + 12.0 + index as f32 * (width - 24.0) / 2.0,
                    modal.max.y - 100.0 * scale,
                    (width - 32.0) / 2.0,
                    44.0 * scale,
                )
            } else {
                PlatformRect::from_xywh(
                    modal.min.x + 12.0,
                    modal.max.y - 50.0 * scale,
                    width - 24.0,
                    44.0 * scale,
                )
            };
            push_control(
                &mut controls,
                &mut nodes,
                &format!("sandbox.chronicle.{id}"),
                label,
                action,
                bounds,
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        (modal, nodes, controls, text)
    }

    fn camera_overlay(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        let scale = scale.max(1.0);
        let width = (640.0 * scale).min(viewport.x - 32.0);
        let height = (640.0 * scale).min(viewport.y - 60.0);
        let modal = PlatformRect::from_xywh(
            (viewport.x - width) / 2.0,
            (viewport.y - height) / 2.0,
            width,
            height,
        );
        let mut controls = Vec::new();
        let mut nodes = Vec::new();
        let mut text = Vec::new();
        let target = 44.0 * scale;
        let half = (width - 32.0) / 2.0;
        let state = self.camera_state();
        let label = format!(
            "Camera {:?}: zoom {:.2}x, yaw {:.0}, pitch {:.0}",
            self.camera_scope().level,
            state.zoom,
            state.yaw_degrees,
            state.pitch_degrees
        );
        let status_bounds =
            PlatformRect::from_xywh(modal.min.x + 12.0, modal.min.y + 12.0, width - 24.0, target);
        let mut status = SemanticNode::text(
            "sandbox.camera.status",
            SemanticRole::Status,
            &label,
            "Presentation camera state",
        );
        status.bounds = Some(status_bounds.into());
        nodes.push(status);
        text.push(PlatformSightedText {
            semantic_id: SemanticNodeId::new("sandbox.camera.status"),
            kind: InspectorTextKind::Fact,
            label: label.clone(),
            value: None,
            lines: wrap_text(&label, width - 24.0, scale),
            bounds: status_bounds,
            clip: modal,
        });
        let actions = [
            ("pan.left", "Pan left", SandboxUiAction::CameraPan(-40, 0)),
            ("pan.right", "Pan right", SandboxUiAction::CameraPan(40, 0)),
            ("pan.up", "Pan up", SandboxUiAction::CameraPan(0, -40)),
            ("pan.down", "Pan down", SandboxUiAction::CameraPan(0, 40)),
            ("zoom.in", "Zoom in", SandboxUiAction::CameraZoom(25)),
            ("zoom.out", "Zoom out", SandboxUiAction::CameraZoom(-20)),
            (
                "orbit.left",
                "Orbit left",
                SandboxUiAction::CameraOrbit(-5, 0),
            ),
            (
                "orbit.right",
                "Orbit right",
                SandboxUiAction::CameraOrbit(5, 0),
            ),
            ("pitch.up", "Tilt up", SandboxUiAction::CameraOrbit(0, 5)),
            (
                "pitch.down",
                "Tilt down",
                SandboxUiAction::CameraOrbit(0, -5),
            ),
            (
                "focus",
                "Focus selection (F)",
                SandboxUiAction::FocusSelection,
            ),
            ("fit", "Fit scope (Home)", SandboxUiAction::FitScope),
            (
                "tool",
                if self.camera_tool {
                    "Camera drag tool: on"
                } else {
                    "Camera drag tool: off"
                },
                SandboxUiAction::ToggleCameraTool,
            ),
        ];
        let capacity = ((height - 164.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize * 2;
        let pages = actions.len().div_ceil(capacity).max(1);
        let page = self.inspector_page.min(pages - 1);
        for (index, (id, label, action)) in actions
            .into_iter()
            .enumerate()
            .skip(page * capacity)
            .take(capacity)
        {
            let local = index - page * capacity;
            push_control(
                &mut controls,
                &mut nodes,
                &format!("sandbox.camera.{id}"),
                label,
                action,
                PlatformRect::from_xywh(
                    modal.min.x + 12.0 + (local % 2) as f32 * (half + 8.0),
                    modal.min.y + (64.0 + (local / 2) as f32 * 50.0) * scale,
                    half,
                    target,
                ),
                action != SandboxUiAction::FocusSelection || self.selected_entity().is_some(),
                SemanticRole::Button,
                None,
            );
        }
        for (index, (id, label, back, enabled)) in [
            ("sandbox.camera.previous", "Previous page", true, page > 0),
            ("sandbox.camera.next", "Next page", false, page + 1 < pages),
        ]
        .into_iter()
        .enumerate()
        {
            push_control(
                &mut controls,
                &mut nodes,
                id,
                label,
                SandboxUiAction::InspectorPage(back),
                PlatformRect::from_xywh(
                    modal.min.x + 12.0 + index as f32 * (half + 8.0),
                    modal.max.y - 100.0 * scale,
                    half,
                    target,
                ),
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        push_control(
            &mut controls,
            &mut nodes,
            "sandbox.camera.close",
            "Close",
            SandboxUiAction::CreatorCancel,
            PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.max.y - 50.0 * scale,
                width - 24.0,
                target,
            ),
            true,
            SemanticRole::Button,
            None,
        );
        (modal, nodes, controls, text)
    }

    fn entity_overlay(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        let scale = scale.max(1.0);
        let width = (640.0 * scale).min(viewport.x - 32.0);
        let height = (640.0 * scale).min(viewport.y - 60.0);
        let modal = PlatformRect::from_xywh(
            (viewport.x - width) / 2.0,
            (viewport.y - height) / 2.0,
            width,
            height,
        );
        let mut nodes = Vec::new();
        let mut controls = Vec::new();
        let mut text = Vec::new();
        let target = 44.0 * scale;
        let stride = 50.0 * scale;
        let inset = modal.min + Vec2::splat(12.0);
        let half = (width - 32.0) / 2.0;
        let pages;
        let page;
        if self.navigator_picker {
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.creator.field.navigator_query",
                "Search entities or full ID",
                SandboxUiAction::CreatorField(0),
                PlatformRect::from_xywh(inset.x, inset.y, width - 24.0, target),
                true,
                SemanticRole::TextInput,
                Some(&self.navigator_query),
            );
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.navigator.types",
                &format!(
                    "Filter: {}",
                    self.navigator_filter
                        .map_or("All entities", LivingEntityKind::label)
                ),
                SandboxUiAction::NavigatorTypes,
                PlatformRect::from_xywh(inset.x, modal.min.y + 64.0 * scale, half, target),
                true,
                SemanticRole::Button,
                None,
            );
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.navigator.reveal",
                "Reveal selection",
                SandboxUiAction::RevealSelection,
                PlatformRect::from_xywh(
                    inset.x + half + 8.0,
                    modal.min.y + 64.0 * scale,
                    half,
                    target,
                ),
                self.selected_entity().is_some(),
                SemanticRole::Button,
                None,
            );
            let capacity = ((height - 214.0 * scale) / stride).floor().max(1.0) as usize;
            if self.navigator_types {
                let mut kinds = vec![None];
                kinds.extend(LivingEntityKind::ALL.into_iter().map(Some));
                pages = kinds.len().div_ceil(capacity);
                page = self.inspector_page.min(pages - 1);
                for (index, kind) in kinds
                    .into_iter()
                    .enumerate()
                    .skip(page * capacity)
                    .take(capacity)
                {
                    push_control(
                        &mut controls,
                        &mut nodes,
                        &format!("sandbox.navigator.filter.{index}"),
                        kind.map_or("All entities", LivingEntityKind::label),
                        SandboxUiAction::NavigatorFilter(kind),
                        PlatformRect::from_xywh(
                            inset.x,
                            modal.min.y + 114.0 * scale + (index - page * capacity) as f32 * stride,
                            width - 24.0,
                            target,
                        ),
                        true,
                        SemanticRole::Option,
                        None,
                    );
                }
            } else {
                let rows = crate::living_navigation::visible_navigator_rows(
                    self.state(),
                    &self.navigator_query,
                    self.navigator_filter,
                    &self.navigator_expanded,
                );
                pages = rows.len().div_ceil(capacity).max(1);
                page = if self.navigator_reveal {
                    rows.iter()
                        .position(|row| Some(row.selection) == self.selected_entity())
                        .map_or(0, |index| index / capacity)
                } else {
                    self.inspector_page.min(pages - 1)
                };
                let mut tree_nodes = Vec::new();
                for (index, row) in rows.iter().enumerate().skip(page * capacity).take(capacity) {
                    let y = modal.min.y + 114.0 * scale + (index - page * capacity) as f32 * stride;
                    let indent = row.depth as f32 * 12.0 * scale;
                    let key = format!(
                        "{:?}.{}",
                        row.selection.kind,
                        crate::living_navigation::entity_id(row.selection.id)
                    );
                    let expanded = self.navigator_expanded.contains(&row.selection)
                        || !self.navigator_query.is_empty()
                        || self.navigator_filter.is_some();
                    let extra = if row.has_children { target + 4.0 } else { 0.0 };
                    if row.has_children {
                        push_control(
                            &mut controls,
                            &mut tree_nodes,
                            &format!("sandbox.navigator.expand.{key}"),
                            if expanded { "Collapse" } else { "Expand" },
                            SandboxUiAction::ToggleNavigatorGroup(row.selection),
                            PlatformRect::from_xywh(inset.x + indent, y, target, target),
                            true,
                            SemanticRole::Button,
                            None,
                        );
                        tree_nodes.last_mut().unwrap().expanded = Some(expanded);
                    }
                    let label = format!("{}: {}", row.selection.kind.label(), row.label);
                    push_control(
                        &mut controls,
                        &mut tree_nodes,
                        &format!("sandbox.navigator.entity.{key}"),
                        &label,
                        SandboxUiAction::SelectEntity(row.selection),
                        PlatformRect::from_xywh(
                            inset.x + indent + extra,
                            y,
                            width - 24.0 - indent - extra,
                            target,
                        ),
                        true,
                        SemanticRole::TreeItem,
                        None,
                    );
                    let node = tree_nodes.last_mut().unwrap();
                    node.hierarchy_level = Some(row.depth as u32 + 1);
                    node.selected = Some(row.selection) == self.selected_entity();
                    if row.has_children {
                        node.expanded = Some(expanded);
                    }
                    node.description = format!(
                        "{} / full ID {}",
                        label,
                        crate::living_navigation::entity_id(row.selection.id)
                    );
                    let control = controls.last_mut().unwrap();
                    control.selected = node.selected;
                    control.focused = self.requested_focus.as_ref() == Some(&control.action_id);
                }
                nodes.push(SemanticNode::container(
                    "sandbox.navigator.tree",
                    SemanticRole::Tree,
                    "Galaxy entities",
                    tree_nodes,
                ));
            }
        } else {
            let lines = self
                .inspector_lines()
                .iter()
                .flat_map(|line| wrap_text(line, width - 24.0, scale))
                .collect::<Vec<_>>();
            let line_height = 20.25 * scale;
            let capacity = ((height - 164.0 * scale) / line_height).floor().max(1.0) as usize;
            pages = lines.len().div_ceil(capacity).max(1);
            page = self.inspector_page.min(pages - 1);
            let heading = self.selected_entity().map_or_else(
                || "No selected entity".to_owned(),
                |selection| {
                    format!(
                        "{} / {}",
                        selection.kind.label(),
                        crate::living_navigation::entity_id(selection.id)
                    )
                },
            );
            let bounds = PlatformRect::from_xywh(inset.x, inset.y, width - 24.0, target);
            let mut node = SemanticNode::text(
                "sandbox.entity.title",
                SemanticRole::Heading,
                &heading,
                "Selected stable entity",
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            text.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new("sandbox.entity.title"),
                kind: InspectorTextKind::Fact,
                label: heading.clone(),
                value: None,
                lines: wrap_text(&heading, width - 24.0, scale),
                bounds,
                clip: modal,
            });
            for (index, line) in lines
                .iter()
                .enumerate()
                .skip(page * capacity)
                .take(capacity)
            {
                let id = format!("sandbox.entity.fact.{index}");
                let bounds = PlatformRect::from_xywh(
                    inset.x,
                    modal.min.y + 64.0 * scale + (index - page * capacity) as f32 * line_height,
                    width - 24.0,
                    line_height,
                );
                let mut node = SemanticNode::text(
                    &id,
                    SemanticRole::Text,
                    line,
                    "Authoritative selected entity fact",
                );
                node.bounds = Some(bounds.into());
                nodes.push(node);
                text.push(PlatformSightedText {
                    semantic_id: SemanticNodeId::new(id),
                    kind: InspectorTextKind::Fact,
                    label: line.clone(),
                    value: None,
                    lines: vec![line.clone()],
                    bounds,
                    clip: modal,
                });
            }
        }
        let footer = modal.max.y - 100.0 * scale;
        for (index, (id, label, back, enabled)) in [
            ("sandbox.entity.previous", "Previous page", true, page > 0),
            ("sandbox.entity.next", "Next page", false, page + 1 < pages),
        ]
        .into_iter()
        .enumerate()
        {
            push_control(
                &mut controls,
                &mut nodes,
                id,
                label,
                SandboxUiAction::InspectorPage(back),
                PlatformRect::from_xywh(
                    inset.x + index as f32 * (half + 8.0),
                    footer,
                    half,
                    target,
                ),
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        push_control(
            &mut controls,
            &mut nodes,
            "sandbox.entity.close",
            "Close",
            SandboxUiAction::CreatorCancel,
            PlatformRect::from_xywh(inset.x, modal.max.y - 50.0 * scale, width - 24.0, target),
            true,
            SemanticRole::Button,
            None,
        );
        (modal, nodes, controls, text)
    }

    pub fn creator_scene_open(&self) -> bool {
        self.creator_scene && self.creator.is_some()
    }

    fn creator_scene_overlay(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        let target = 44.01 * scale.max(1.0);
        let modal = PlatformRect::from_xywh(
            16.0,
            viewport.y - target * 3.0 - 60.0,
            viewport.x - 32.0,
            target * 3.0 + 44.0,
        );
        self.last_creator_scene_top.set(Some(modal.min.y));
        let mut nodes = Vec::new();
        let mut controls = Vec::new();
        let mut text = Vec::new();
        let label = format!(
            "{} Plus = added; cross = removed; double ring = changed facts. Canvas picks edit compatible draft parents only.",
            self.creator_message
                .as_deref()
                .unwrap_or("Validated private preview.")
        );
        let bounds = PlatformRect::from_xywh(
            modal.min.x + 8.0,
            modal.min.y + 8.0,
            modal.max.x - modal.min.x - 16.0,
            target,
        );
        let mut node = SemanticNode::text(
            "sandbox.creator.scene.status",
            SemanticRole::Status,
            &label,
            "Preview facts and ghost legend",
        );
        node.bounds = Some(bounds.into());
        nodes.push(node);
        text.push(PlatformSightedText {
            semantic_id: SemanticNodeId::new("sandbox.creator.scene.status"),
            kind: InspectorTextKind::Fact,
            label: label.clone(),
            value: None,
            lines: vec![label],
            bounds,
            clip: modal,
        });
        let width = (modal.max.x - modal.min.x - 32.0) / 3.0;
        for (index, (id, label, action, enabled)) in [
            (
                "fields",
                "Back to fields",
                SandboxUiAction::CreatorScene(false),
                true,
            ),
            (
                "preview",
                "Preview again",
                SandboxUiAction::CreatorPreview,
                true,
            ),
            (
                "apply",
                "Apply reviewed",
                SandboxUiAction::CreatorApply,
                self.creator_preview.is_some(),
            ),
            (
                "fit",
                "Fit / reveal changes",
                SandboxUiAction::CreatorFitPreview,
                self.creator_preview.is_some(),
            ),
            ("cancel", "Cancel", SandboxUiAction::CreatorCancel, true),
        ]
        .into_iter()
        .enumerate()
        {
            push_control(
                &mut controls,
                &mut nodes,
                &format!("sandbox.creator.scene.{id}"),
                label,
                action,
                PlatformRect::from_xywh(
                    modal.min.x + 8.0 + (index % 3) as f32 * (width + 8.0),
                    bounds.max.y + 8.0 + (index / 3) as f32 * (target + 8.0),
                    width,
                    target,
                ),
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        (modal, nodes, controls, text)
    }

    fn creator_frame(
        &self,
        viewport: Vec2,
        scale: f32,
    ) -> (
        PlatformRect,
        Vec<SemanticNode>,
        Vec<PlatformControl>,
        Vec<PlatformSightedText>,
    ) {
        // Text can shrink with preferences; interactive targets retain 44 logical pixels.
        if self.notebook_picker {
            return self.notebook_overlay(viewport, scale);
        }
        if self.pack_picker {
            return self.pack_overlay(viewport, scale);
        }
        if self.creator_scene && self.creator.is_some() {
            return self.creator_scene_overlay(viewport, scale);
        }
        if self.delete_review.is_some() {
            return self.delete_overlay(viewport, scale);
        }
        if self.intro_picker {
            return self.intro_overlay(viewport, scale);
        }
        if self.chronicle_picker {
            return self.chronicle_overlay(viewport, scale);
        }
        if self.camera_picker {
            return self.camera_overlay(viewport, scale);
        }
        if self.navigator_picker || self.inspector_picker {
            return self.entity_overlay(viewport, scale);
        }
        let scale = scale.max(1.0);
        let width = (640.0 * scale).min(viewport.x - 32.0);
        let height = (640.0 * scale).min(viewport.y - 60.0);
        let modal = PlatformRect::from_xywh(
            (viewport.x - width) * 0.5,
            (viewport.y - height) * 0.5,
            width,
            height,
        );
        let mut nodes = Vec::new();
        let mut controls = Vec::new();
        let mut text = Vec::new();
        if let Some(slot) = self.rename_slot {
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.creator.field.save_name",
                "Saved galaxy name",
                SandboxUiAction::CreatorField(0),
                PlatformRect::from_xywh(
                    modal.min.x + 12.0,
                    modal.min.y + 64.0 * scale,
                    width - 24.0,
                    44.0 * scale,
                ),
                !self.storage_busy,
                SemanticRole::TextInput,
                Some(&self.rename_name),
            );
            let message = "Use 1–64 printable ASCII characters, with single spaces between words.";
            let bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + 12.0,
                width - 24.0,
                44.0 * scale,
            );
            let mut node = SemanticNode::text(
                "sandbox.library.rename.help",
                SemanticRole::Status,
                message,
                "Saved name requirements",
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            text.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new("sandbox.library.rename.help"),
                kind: InspectorTextKind::Fact,
                label: message.to_owned(),
                value: None,
                lines: wrap_text(message, width - 24.0, scale),
                bounds,
                clip: modal,
            });
            let valid = !self.rename_name.is_empty()
                && self.rename_name.len() <= 64
                && self
                    .rename_name
                    .bytes()
                    .all(|byte| (b' '..=b'~').contains(&byte))
                && !self.rename_name.starts_with(' ')
                && !self.rename_name.ends_with(' ')
                && !self.rename_name.contains("  ");
            let half = (width - 32.0) / 2.0;
            for (index, (id, label, action, enabled)) in [
                (
                    "sandbox.library.rename.apply",
                    "Rename",
                    SandboxUiAction::ConfirmRenameSave(slot),
                    valid && !self.storage_busy,
                ),
                (
                    "sandbox.library.rename.cancel",
                    "Cancel rename",
                    SandboxUiAction::CancelRenameSave,
                    true,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                push_control(
                    &mut controls,
                    &mut nodes,
                    id,
                    label,
                    action,
                    PlatformRect::from_xywh(
                        modal.min.x + 12.0 + index as f32 * (half + 8.0),
                        modal.max.y - 48.0 * scale,
                        half,
                        44.0 * scale,
                    ),
                    enabled,
                    SemanticRole::Button,
                    None,
                );
            }
            return (modal, nodes, controls, text);
        } else if self.transfer_picker {
            let half = (width - 32.0) / 2.0;
            let status_bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + 12.0,
                width - 24.0,
                44.0 * scale,
            );
            let mut status = SemanticNode::text(
                "sandbox.transfer.status",
                SemanticRole::Status,
                &self.export_status,
                "Portable transfer progress",
            );
            status.bounds = Some(status_bounds.into());
            nodes.push(status);
            text.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new("sandbox.transfer.status"),
                kind: InspectorTextKind::Fact,
                label: self.export_status.clone(),
                value: None,
                lines: wrap_text(&self.export_status, width - 24.0, scale)
                    .into_iter()
                    .take(2)
                    .collect(),
                bounds: status_bounds,
                clip: modal,
            });
            let available = self.transfer_capabilities[0] && self.export_prepared;
            let mut rows = vec![
                (
                    "sandbox.export.prepare".to_owned(),
                    "Prepare exact export".to_owned(),
                    SandboxUiAction::PrepareExport,
                    self.transfer_capabilities[0],
                ),
                (
                    "sandbox.export".to_owned(),
                    "Export Galaxy".to_owned(),
                    SandboxUiAction::ExportGalaxy,
                    available,
                ),
                (
                    "sandbox.catalog.export".to_owned(),
                    "Export Catalog".to_owned(),
                    SandboxUiAction::ExportCatalog,
                    available,
                ),
                (
                    "sandbox.notebook.export".to_owned(),
                    "Export Notebook".to_owned(),
                    SandboxUiAction::ExportNotebook,
                    available,
                ),
                (
                    "sandbox.import".to_owned(),
                    "Import Galaxy".to_owned(),
                    SandboxUiAction::ImportGalaxy,
                    self.transfer_capabilities[1],
                ),
                (
                    "sandbox.pack".to_owned(),
                    "Import Catalog".to_owned(),
                    SandboxUiAction::ImportCatalog,
                    self.transfer_capabilities[2],
                ),
                (
                    "sandbox.notebook.import".to_owned(),
                    "Import Notebook".to_owned(),
                    SandboxUiAction::ImportNotebook,
                    self.transfer_capabilities[1],
                ),
            ];
            for (index, label) in [
                "Confirm Galaxy downloaded",
                "Confirm Catalog downloaded",
                "Confirm Notebook downloaded",
            ]
            .into_iter()
            .enumerate()
            {
                if self.export_confirmable[index] {
                    rows.push((
                        format!("sandbox.export.confirm.{index}"),
                        label.to_owned(),
                        SandboxUiAction::ConfirmExport(index as u8),
                        true,
                    ));
                }
            }
            let capacity =
                ((height - 164.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize * 2;
            let page = self
                .inspector_page
                .min(rows.len().div_ceil(capacity).saturating_sub(1));
            for (index, (id, label, action, enabled)) in rows
                .into_iter()
                .enumerate()
                .skip(page * capacity)
                .take(capacity)
            {
                let local = index - page * capacity;
                push_control(
                    &mut controls,
                    &mut nodes,
                    &id,
                    &label,
                    action,
                    PlatformRect::from_xywh(
                        modal.min.x + 12.0 + (local % 2) as f32 * (half + 8.0),
                        modal.min.y + (64.0 + (local / 2) as f32 * 50.0) * scale,
                        half,
                        44.0 * scale,
                    ),
                    enabled && !self.storage_busy,
                    SemanticRole::Button,
                    None,
                );
            }
        } else if self.library_picker {
            let header = &self.storage_status;
            let bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + 12.0,
                width - 24.0,
                44.0 * scale,
            );
            let mut node = SemanticNode::text(
                "sandbox.library.status",
                SemanticRole::Status,
                header,
                "Living Library status",
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            text.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new("sandbox.library.status"),
                kind: InspectorTextKind::Fact,
                label: header.clone(),
                value: None,
                lines: storage_status_display_lines(header, bounds.width(), scale, 2),
                bounds,
                clip: modal,
            });
            let capacity = ((height - 164.0 * scale) / (100.0 * scale))
                .floor()
                .max(1.0) as usize;
            let pages = self.saved_rows.len().div_ceil(capacity).max(1);
            let page = self.inspector_page.min(pages - 1);
            let half = (width - 32.0) / 2.0;
            for (index, row) in self
                .saved_rows
                .iter()
                .enumerate()
                .skip(page * capacity)
                .take(capacity)
            {
                let y = modal.min.y + (64.0 + (index - page * capacity) as f32 * 100.0) * scale;
                let choices = [
                    (
                        "open",
                        format!(
                            "{}{}",
                            row.name,
                            if row.archived { " (archived)" } else { "" }
                        ),
                        SandboxUiAction::OpenSave(row.id.0),
                        !row.archived,
                    ),
                    (
                        "archive",
                        if row.archived {
                            "Restore".to_owned()
                        } else {
                            "Archive".to_owned()
                        },
                        if row.archived {
                            SandboxUiAction::RestoreSave(row.id.0)
                        } else {
                            SandboxUiAction::ArchiveSave(row.id.0)
                        },
                        true,
                    ),
                    (
                        "rename",
                        "Rename".to_owned(),
                        SandboxUiAction::RenameSave(row.id.0),
                        true,
                    ),
                    (
                        "delete",
                        "Delete archived galaxy".to_owned(),
                        SandboxUiAction::DeleteSave(row.id.0),
                        row.archived,
                    ),
                ];
                for (choice, (suffix, label, action, enabled)) in choices.into_iter().enumerate() {
                    push_control(
                        &mut controls,
                        &mut nodes,
                        &format!("sandbox.library.{suffix}.{index}"),
                        &label,
                        action,
                        PlatformRect::from_xywh(
                            modal.min.x + 12.0 + (choice % 2) as f32 * (half + 8.0),
                            y + (choice / 2) as f32 * 50.0 * scale,
                            half,
                            44.0 * scale,
                        ),
                        enabled && !self.storage_busy,
                        SemanticRole::Button,
                        None,
                    );
                }
            }
        } else if self.audio_picker || self.import_preview.is_some() {
            let heading = self.import_preview.as_deref().unwrap_or("Audio settings");
            let bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + 12.0,
                width - 24.0,
                if self.audio_picker {
                    44.0 * scale
                } else {
                    height - 74.0 * scale
                },
            );
            let mut node = SemanticNode::text(
                "sandbox.modal.summary",
                SemanticRole::Status,
                heading,
                "Preview or settings",
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            text.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new("sandbox.modal.summary"),
                kind: InspectorTextKind::Fact,
                label: heading.to_owned(),
                value: None,
                lines: wrap_text(heading, width - 24.0, scale),
                bounds,
                clip: modal,
            });
            if self.audio_picker {
                let settings = self.audio_settings;
                let choices = [
                    (
                        "sandbox.audio.mute",
                        format!("Mute: {}", if settings.muted { "on" } else { "off" }),
                        SandboxUiAction::ToggleMute,
                    ),
                    (
                        "sandbox.audio.master",
                        format!("Master: {}%", settings.master),
                        SandboxUiAction::CycleAudio(AudioChannel::Master),
                    ),
                    (
                        "sandbox.audio.ambience",
                        format!("Ambience: {}%", settings.ambience),
                        SandboxUiAction::CycleAudio(AudioChannel::Ambience),
                    ),
                    (
                        "sandbox.audio.music",
                        format!("Music: {}%", settings.music),
                        SandboxUiAction::CycleAudio(AudioChannel::Music),
                    ),
                    (
                        "sandbox.audio.effects",
                        format!("Effects: {}%", settings.effects),
                        SandboxUiAction::CycleAudio(AudioChannel::Effects),
                    ),
                ];
                let capacity =
                    ((height - 164.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize * 2;
                let page = self
                    .inspector_page
                    .min(choices.len().div_ceil(capacity) - 1);
                let half = (width - 32.0) / 2.0;
                for (index, (id, label, action)) in choices
                    .into_iter()
                    .enumerate()
                    .skip(page * capacity)
                    .take(capacity)
                {
                    push_control(
                        &mut controls,
                        &mut nodes,
                        id,
                        &label,
                        action,
                        PlatformRect::from_xywh(
                            modal.min.x
                                + 12.0
                                + ((index - page * capacity) % 2) as f32 * (half + 8.0),
                            modal.min.y
                                + (64.0 + ((index - page * capacity) / 2) as f32 * 50.0) * scale,
                            half,
                            44.0 * scale,
                        ),
                        true,
                        SemanticRole::Button,
                        None,
                    );
                }
            } else {
                for (index, (id, label, action)) in [
                    (
                        "sandbox.import.apply",
                        "Apply imported galaxy",
                        SandboxUiAction::ApplyImport,
                    ),
                    (
                        "sandbox.import.cancel",
                        "Cancel import",
                        SandboxUiAction::CancelImport,
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    push_control(
                        &mut controls,
                        &mut nodes,
                        id,
                        label,
                        action,
                        PlatformRect::from_xywh(
                            modal.min.x + 12.0 + index as f32 * (width - 24.0) / 2.0,
                            modal.max.y - 48.0 * scale,
                            (width - 32.0) / 2.0,
                            44.0 * scale,
                        ),
                        true,
                        SemanticRole::Button,
                        None,
                    );
                }
                return (modal, nodes, controls, text);
            }
        } else if self.history_picker {
            let header = match &self.history_message {
                Some(message) => message.clone(),
                None => "History and experiments notebook".to_owned(),
            };
            let bounds = PlatformRect::from_xywh(
                modal.min.x + 12.0,
                modal.min.y + 12.0,
                width - 24.0,
                44.0 * scale,
            );
            let mut node = SemanticNode::text(
                "sandbox.history.status",
                SemanticRole::Status,
                &header,
                "History reconstruction status",
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            text.push(PlatformSightedText {
                semantic_id: SemanticNodeId::new("sandbox.history.status"),
                kind: InspectorTextKind::Fact,
                label: header.clone(),
                value: None,
                lines: vec![header],
                bounds,
                clip: modal,
            });
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.creator.field.bookmark_label",
                "Bookmark label",
                SandboxUiAction::CreatorField(0),
                PlatformRect::from_xywh(
                    modal.min.x + 12.0,
                    modal.min.y + 64.0 * scale,
                    width - 24.0,
                    44.0 * scale,
                ),
                true,
                SemanticRole::TextInput,
                Some(&self.bookmark_label),
            );
            let actions = [
                (
                    "sandbox.history.undo",
                    "Undo at current tick",
                    SandboxUiAction::Undo,
                ),
                (
                    "sandbox.history.redo",
                    "Redo at current tick",
                    SandboxUiAction::Redo,
                ),
                (
                    "sandbox.history.bookmark",
                    "Record state bookmark",
                    SandboxUiAction::RecordBookmark,
                ),
                (
                    "sandbox.history.present",
                    "Return to present",
                    SandboxUiAction::ReturnPresent,
                ),
                (
                    "sandbox.history.branch",
                    "Branch here",
                    SandboxUiAction::BeginBranch,
                ),
                (
                    "sandbox.history.cancel",
                    "Cancel reconstruction",
                    SandboxUiAction::CancelHistory,
                ),
            ];
            let mut rows = actions
                .into_iter()
                .map(|(id, label, action)| (id.to_owned(), label.to_owned(), action))
                .collect::<Vec<_>>();
            for (index, label) in [
                "Alloy supply-chain evidence",
                "Autonomous settlement evidence",
                "Trade-delivery evidence",
                "Corridor disruption evidence",
                "War and alternative-diplomacy evidence",
                "Dormant recovery evidence",
                "Portable state reproduction evidence",
            ]
            .into_iter()
            .enumerate()
            {
                rows.push((
                    format!("sandbox.notebook.record.{index}"),
                    label.to_owned(),
                    SandboxUiAction::RecordExperiment(index as u8),
                ));
            }
            rows.push((
                "sandbox.notebook.comparison".to_owned(),
                "Record comparison measurements".to_owned(),
                SandboxUiAction::RecordComparison,
            ));
            for (index, bookmark) in self.bookmarks.iter().enumerate() {
                rows.push((
                    format!("sandbox.notebook.rename.{index}"),
                    "Apply label to bookmark".to_owned(),
                    SandboxUiAction::RenameBookmark(index as u8),
                ));
                rows.push((
                    format!("sandbox.history.open.{index}"),
                    format!(
                        "{} / tick {}",
                        self.named_bookmarks
                            .get(index)
                            .map_or("Recorded state", |row| row.label.as_str()),
                        bookmark.view.tick.0
                    ),
                    SandboxUiAction::RestoreBookmark(index as u8),
                ));
                rows.push((
                    format!("sandbox.history.compare.{index}"),
                    "Compare at current tick".to_owned(),
                    SandboxUiAction::CompareBookmark(index as u8),
                ));
            }
            rows.splice(0..0, self.requested_history_rows());
            // Retained branches are reachable independently of optional notebook metadata.
            for branch in std::iter::once(&self.authority)
                .chain(self.authority.archived())
                .filter_map(|branch| self.authority.retained_history_branch(branch.branch()))
            {
                let view = LivingHistoryViewV2 {
                    branch: branch.branch(),
                    revision: branch.published_cursor().committed_revision,
                    tick: branch.state().tick,
                };
                rows.push((
                    format!("sandbox.history.branch.{}", hex_id(&view.branch.0)),
                    format!(
                        "Branch {} / retained tick {}",
                        branch.state().branch_sequence,
                        view.tick.0
                    ),
                    SandboxUiAction::RestoreHistoryView(view),
                ));
            }
            rows.push((
                "sandbox.history.genesis".to_owned(),
                "Genesis / tick 0".to_owned(),
                SandboxUiAction::RestoreHistoryView(LivingHistoryViewV2 {
                    branch: self.authority.branch(),
                    revision: None,
                    tick: nyon_workshop_core::living::LivingTickV2(0),
                }),
            ));
            if self.history_details.is_some() {
                rows = self.requested_history_rows();
            }
            let revision_count = if self.history_details.is_some() {
                0
            } else {
                self.authority.revisions().len()
            };
            let half = (width - 32.0) / 2.0;
            let capacity =
                ((height - 214.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize * 2;
            let page = self.inspector_page.min(
                (rows.len() + revision_count)
                    .div_ceil(capacity)
                    .saturating_sub(1),
            );
            let total = rows.len() + revision_count;
            for index in page * capacity..((page + 1) * capacity).min(total) {
                let revision_row;
                let (id, label, action) = if let Some(row) = rows.get(index) {
                    row
                } else {
                    let revision = &self.authority.revisions()[index - rows.len()];
                    revision_row = (
                        format!("sandbox.history.revision.{}", hex_id(&revision.id.0)),
                        format!(
                            "Creator revision {} / tick {} / {} operations",
                            revision.ordinal,
                            revision.tick.0,
                            match &revision.command {
                                LivingCommandV2::CreatorBatch { operations } => operations.len(),
                            }
                        ),
                        SandboxUiAction::RestoreHistoryView(LivingHistoryViewV2 {
                            branch: self.authority.branch(),
                            revision: Some(revision.id),
                            tick: revision.tick,
                        }),
                    );
                    &revision_row
                };
                let detail_line = id.starts_with("sandbox.history.detail.line.");
                let field_key = id.strip_prefix("sandbox.creator.field.");
                let enabled = !detail_line
                    && match action {
                        SandboxUiAction::CancelHistory => {
                            self.history_job.is_some() || self.comparison_job.is_some()
                        }
                        SandboxUiAction::RecordExperiment(index) => {
                            self.experiment_receipt_witness(*index).is_ok()
                        }
                        SandboxUiAction::RecordComparison => self.comparison.is_some(),
                        SandboxUiAction::ReturnPresent => {
                            self.present.is_some() || self.authority.present_view().is_some()
                        }
                        SandboxUiAction::BeginBranch => self.history_job.is_none(),
                        _ => self.history_job.is_none() && self.comparison_job.is_none(),
                    };
                push_control(
                    &mut controls,
                    &mut nodes,
                    id,
                    label,
                    *action,
                    PlatformRect::from_xywh(
                        modal.min.x + 12.0 + ((index - page * capacity) % 2) as f32 * (half + 8.0),
                        modal.min.y
                            + (114.0 + ((index - page * capacity) / 2) as f32 * 50.0) * scale,
                        half,
                        44.0 * scale,
                    ),
                    enabled,
                    if detail_line {
                        SemanticRole::Status
                    } else if field_key.is_some() {
                        SemanticRole::TextInput
                    } else {
                        SemanticRole::Button
                    },
                    field_key.and_then(|key| self.creator_field_value(key)),
                );
                let selected = match action {
                    SandboxUiAction::HistoryBranch(branch) => *branch == self.history_branch,
                    SandboxUiAction::HistoryRevision(revision) => {
                        *revision == self.history_revision
                    }
                    SandboxUiAction::ComparisonBranch { left, branch } => {
                        *branch
                            == if *left {
                                self.comparison_left
                            } else {
                                self.comparison_right
                            }
                    }
                    _ => false,
                };
                if selected {
                    controls.last_mut().expect("control added").selected = true;
                    nodes.last_mut().expect("semantic added").selected = true;
                }
            }
        } else if self.content_picker {
            let choices = [
                LivingContent::ValeConfluence,
                LivingContent::Commerce,
                LivingContent::FrontierFriction,
                LivingContent::InterruptedCorridor,
                LivingContent::FirstExpansion,
                LivingContent::Blank,
                LivingContent::Seeded(self.content_seed().unwrap_or(0)),
            ];
            let seed_id = "sandbox.creator.field.seed";
            push_control(
                &mut controls,
                &mut nodes,
                seed_id,
                "Seed (hexadecimal)",
                SandboxUiAction::CreatorField(0),
                PlatformRect::from_xywh(
                    modal.min.x + 12.0,
                    modal.min.y + 12.0,
                    width - 24.0,
                    44.0 * scale,
                ),
                true,
                SemanticRole::TextInput,
                Some(&self.content_seed),
            );
            let capacity = ((height - 164.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize;
            let page = self
                .inspector_page
                .min(choices.len().div_ceil(capacity).saturating_sub(1));
            for (index, content) in choices
                .into_iter()
                .enumerate()
                .skip(page * capacity)
                .take(capacity)
            {
                push_control(
                    &mut controls,
                    &mut nodes,
                    &format!("sandbox.content.{index}"),
                    content.label(),
                    SandboxUiAction::OpenContent(content),
                    PlatformRect::from_xywh(
                        modal.min.x + 12.0,
                        modal.min.y + (68.0 + index as f32 * 50.0) * scale,
                        width - 24.0,
                        44.0 * scale,
                    ),
                    !matches!(content, LivingContent::Seeded(_)) || self.content_seed().is_ok(),
                    SemanticRole::Button,
                    None,
                );
            }
        } else if let Some(draft) = &self.creator {
            let inset = modal.min + Vec2::splat(12.0);
            let half = (width - 32.0) / 2.0;
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.creator.field.query",
                "Search commands",
                SandboxUiAction::CreatorField(0),
                PlatformRect::from_xywh(inset.x, inset.y, width - 24.0, 44.0 * scale),
                true,
                SemanticRole::TextInput,
                Some(&draft.query),
            );
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.creator.tool.previous",
                "Previous tool",
                SandboxUiAction::CreatorTool(true),
                PlatformRect::from_xywh(inset.x, inset.y + 50.0 * scale, half, 44.0 * scale),
                true,
                SemanticRole::Button,
                None,
            );
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.creator.tool.next",
                draft.tool.label(),
                SandboxUiAction::CreatorTool(false),
                PlatformRect::from_xywh(
                    inset.x + half + 8.0,
                    inset.y + 50.0 * scale,
                    half,
                    44.0 * scale,
                ),
                true,
                SemanticRole::Button,
                None,
            );
            let capacity = ((height - 214.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize;
            let mut rows = Vec::new();
            if let Some(message) = &self.creator_message {
                rows.extend(
                    wrap_text(message, width - 24.0, scale)
                        .into_iter()
                        .map(|line| (None, line)),
                );
            }
            if let Some(preview) = &self.creator_preview {
                rows.extend(
                    preview
                        .message
                        .iter()
                        .flat_map(|line| wrap_text(line, width - 24.0, scale))
                        .map(|line| (None, line)),
                );
            }
            rows.extend(draft.fields.iter().map(|(key, value)| {
                (
                    Some(key.as_str()),
                    format!("{}: {}", key.replace('.', " "), value),
                )
            }));
            let pages = rows.len().div_ceil(capacity).max(1);
            let page = self.inspector_page.min(pages - 1);
            for (index, (key, label)) in
                rows.iter().enumerate().skip(page * capacity).take(capacity)
            {
                let bounds = PlatformRect::from_xywh(
                    inset.x,
                    inset.y + (100.0 + (index - page * capacity) as f32 * 50.0) * scale,
                    width - 24.0,
                    44.0 * scale,
                );
                if let Some(key) = key {
                    push_control(
                        &mut controls,
                        &mut nodes,
                        &format!("sandbox.creator.field.{key}"),
                        &key.replace('.', " "),
                        SandboxUiAction::CreatorField(index as u16),
                        bounds,
                        true,
                        SemanticRole::TextInput,
                        draft.value(key),
                    );
                } else {
                    let id = format!("sandbox.creator.preview.{index}");
                    let mut node =
                        SemanticNode::text(&id, SemanticRole::Status, label, "Creator preview");
                    node.bounds = Some(bounds.into());
                    nodes.push(node);
                    text.push(PlatformSightedText {
                        semantic_id: SemanticNodeId::new(id),
                        kind: InspectorTextKind::Fact,
                        label: label.clone(),
                        value: None,
                        lines: vec![label.clone()],
                        bounds,
                        clip: modal,
                    });
                }
            }
            let footer = modal.max.y - 100.0 * scale;
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.creator.page.previous",
                "Previous page",
                SandboxUiAction::InspectorPage(true),
                PlatformRect::from_xywh(inset.x, footer, half, 44.0 * scale),
                page > 0,
                SemanticRole::Button,
                None,
            );
            push_control(
                &mut controls,
                &mut nodes,
                "sandbox.creator.page.next",
                "Next page",
                SandboxUiAction::InspectorPage(false),
                PlatformRect::from_xywh(inset.x + half + 8.0, footer, half, 44.0 * scale),
                page + 1 < pages,
                SemanticRole::Button,
                None,
            );
        }
        if self.content_picker
            || self.history_picker
            || self.library_picker
            || self.audio_picker
            || self.transfer_picker
        {
            let capacity = if self.history_picker {
                ((height - 214.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize * 2
            } else if self.audio_picker || self.transfer_picker {
                ((height - 164.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize * 2
            } else if self.library_picker {
                ((height - 164.0 * scale) / (100.0 * scale))
                    .floor()
                    .max(1.0) as usize
            } else if self.content_picker {
                ((height - 164.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize
            } else {
                ((height - 326.0 * scale) / (50.0 * scale)).floor().max(1.0) as usize
            };
            let count = if self.transfer_picker {
                7 + self
                    .export_confirmable
                    .iter()
                    .filter(|enabled| **enabled)
                    .count()
            } else if self.audio_picker {
                5
            } else if self.content_picker {
                7
            } else if self.library_picker {
                self.saved_rows.len()
            } else if self.history_details.is_some() {
                self.requested_history_rows().len()
            } else {
                16 + self.bookmarks.len() * 3
                    + self.requested_history_rows().len()
                    + self.authority.archived().len()
                    + self.authority.revisions().len()
            };
            let pages = count.div_ceil(capacity).max(1);
            let page = self.inspector_page.min(pages - 1);
            let half = (width - 32.0) / 2.0;
            for (index, (id, label, back, enabled)) in [
                (
                    "sandbox.modal.page.previous",
                    "Previous page",
                    true,
                    page > 0,
                ),
                (
                    "sandbox.modal.page.next",
                    "Next page",
                    false,
                    page + 1 < pages,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                push_control(
                    &mut controls,
                    &mut nodes,
                    id,
                    label,
                    SandboxUiAction::InspectorPage(back),
                    PlatformRect::from_xywh(
                        modal.min.x + 12.0 + index as f32 * (half + 8.0),
                        modal.max.y - 98.0 * scale,
                        half,
                        44.0 * scale,
                    ),
                    enabled,
                    SemanticRole::Button,
                    None,
                );
            }
        }
        let thirds = (width - 40.0) / 3.0;
        for (index, (id, label, action, enabled)) in [
            (
                "sandbox.creator.preview",
                if self.library_picker {
                    "Catalog packs"
                } else {
                    "Preview"
                },
                if self.library_picker {
                    SandboxUiAction::ListPacks
                } else {
                    SandboxUiAction::CreatorPreview
                },
                self.creator.is_some() || self.library_picker,
            ),
            (
                "sandbox.creator.apply",
                "Apply",
                SandboxUiAction::CreatorApply,
                self.creator_preview.is_some(),
            ),
            (
                "sandbox.creator.cancel",
                "Cancel",
                if self.import_preview.is_some() {
                    SandboxUiAction::CancelImport
                } else {
                    SandboxUiAction::CreatorCancel
                },
                true,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            push_control(
                &mut controls,
                &mut nodes,
                id,
                label,
                action,
                PlatformRect::from_xywh(
                    modal.min.x + 12.0 + index as f32 * (thirds + 8.0),
                    modal.max.y - 48.0 * scale,
                    thirds,
                    44.0 * scale,
                ),
                enabled,
                SemanticRole::Button,
                None,
            );
        }
        (modal, nodes, controls, text)
    }

    /// The read model the shell paints: tick, one world inventory, diplomacy,
    /// and freight or fleets.
    pub fn frame(&self, viewport: Vec2, high_contrast: bool) -> PlatformUiFrame {
        self.frame_with_scale(viewport, 1.0, high_contrast)
    }

    pub fn frame_with_scale(
        &self,
        viewport: Vec2,
        ui_scale: f32,
        high_contrast: bool,
    ) -> PlatformUiFrame {
        let state = self.authority.state();
        let tick_line = format!(
            "tick {} {}",
            state.tick.0,
            if self.paused() { "paused" } else { "running" }
        );
        let inventory_line = state
            .worlds
            .as_slice()
            .iter()
            .find(|world| Some(world.id) == self.selected_world)
            .or_else(|| state.worlds.as_slice().first())
            .map(|world| {
                format!(
                    "{} energy {} ore {} alloy {}",
                    world.name.as_str(),
                    world.inventory.energy,
                    world.inventory.ore,
                    world.inventory.alloy
                )
            })
            .unwrap_or_else(|| "Select or create a world".to_owned());
        let diplomacy_line = format!(
            "relations {} wars {}",
            state.relations.as_slice().len(),
            state.wars.as_slice().len()
        );
        let motion_line = format!(
            "fleets {} shipments {}",
            state.fleets.as_slice().len(),
            state.shipments.as_slice().len()
        );
        let scope = self.camera_scope();
        let scope_name = match scope.level {
            LivingCameraLevel::Galaxy => "Galaxy".to_owned(),
            LivingCameraLevel::System => format!(
                "System / {}",
                self.state()
                    .systems
                    .as_slice()
                    .iter()
                    .find(|row| Some(row.id) == scope.entity)
                    .map_or("absent", |row| row.name.as_str())
            ),
            LivingCameraLevel::World => format!(
                "World / {}",
                self.state()
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|row| Some(row.id) == scope.entity)
                    .map_or("absent", |row| row.name.as_str())
            ),
        };
        let short_scope = if scope_name.len() > 22 {
            format!("{}...", &scope_name[..19])
        } else {
            scope_name
        };
        let identity_line = format!("Living Galaxy / {short_scope}");
        let mut status_lines = vec![
            identity_line,
            tick_line.clone(),
            inventory_line.clone(),
            diplomacy_line.clone(),
            motion_line.clone(),
        ];
        let speed_line = match self.limited_speed {
            Some(effective) => format!(
                "Speed {}x requested; {effective:.2}x effective; recent update work capped",
                self.speed.multiplier()
            ),
            None => format!("Speed {}x", self.speed.multiplier()),
        };
        status_lines.insert(2, speed_line);
        if self.processing() {
            status_lines.insert(
                1,
                "Processing next boundary; showing committed state".to_owned(),
            );
        }
        if let Some(fault) = &self.fault {
            status_lines.insert(1, format!("Simulation stopped: {fault}"));
        }
        let mut nodes: Vec<_> = status_lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                SemanticNode::text(
                    format!("sandbox.status.{index}"),
                    SemanticRole::Status,
                    line.clone(),
                    "Galaxy sandbox readout",
                )
            })
            .collect();
        let layout = WorkshopLayout::resolve(viewport, ui_scale).unwrap_or_else(|_| {
            WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.0)
                .expect("the fallback viewport resolves")
        });
        self.last_scene_bounds.set(layout.canvas);
        let target = (44.0 * layout.ui_scale).max(44.01);
        let stride = target + 6.0 * layout.ui_scale;
        let transport_top = viewport.y - 12.0 - target;
        let readout_origin = layout
            .left_panel
            .map_or(layout.top_bar.min, |panel| panel.min)
            .y
            + 16.0;
        // Four measured header lines need their full ink height before tools.
        let minimum_toolbar_top = readout_origin + 96.0;
        let storage_reserve = 34.0 * layout.ui_scale + 8.0;
        let available_rows = (((transport_top - minimum_toolbar_top - storage_reserve) / stride)
            .floor() as usize)
            .saturating_sub(1)
            .max(1);
        let toolbar_columns = 13usize.div_ceil(available_rows).max(2);
        let toolbar_rows = 13usize.div_ceil(toolbar_columns);
        let maximum_toolbar_top =
            transport_top - (toolbar_rows + 1) as f32 * stride - storage_reserve;
        let toolbar_top = (140.0 * layout.ui_scale)
            .max(minimum_toolbar_top)
            .min(maximum_toolbar_top.max(minimum_toolbar_top));
        let docked_inspector = viewport.x / layout.ui_scale >= 900.0;
        let left_width = if docked_inspector {
            (viewport.x - (320.0 * layout.ui_scale).min(viewport.x - 16.0) - 32.0).max(148.0)
        } else {
            (viewport.x - 32.0).max(148.0)
        };
        let toolbar_width = ((left_width - 8.0 * (toolbar_columns - 1) as f32)
            / toolbar_columns as f32)
            .clamp(44.01, 118.0 * layout.ui_scale);
        let toolbar_span =
            toolbar_width * toolbar_columns as f32 + 8.0 * (toolbar_columns - 1) as f32;
        let navigator_top = toolbar_top + toolbar_rows as f32 * stride;
        let entries = [
            (
                "sandbox.toggle_pause",
                if self.paused { "Run" } else { "Pause" },
                SandboxUiAction::TogglePause,
            ),
            ("sandbox.step", "Step", SandboxUiAction::Step),
            (
                "sandbox.speed.1",
                "1x",
                SandboxUiAction::Speed(SandboxSpeed::Normal),
            ),
            (
                "sandbox.speed.4",
                "4x",
                SandboxUiAction::Speed(SandboxSpeed::Fast),
            ),
            (
                "sandbox.speed.20",
                "20x",
                SandboxUiAction::Speed(SandboxSpeed::Fastest),
            ),
        ];
        let mut controls = Vec::new();
        let width = ((viewport.x - 48.0) / 5.0).clamp(44.0, 100.0 * layout.ui_scale);
        for (index, (id, label, action)) in entries.into_iter().enumerate() {
            let enabled = self.fault.is_none()
                && !self.authority.is_viewing_history()
                && !self.suspended
                && self.holds.is_empty()
                && (action != SandboxUiAction::Step || (self.paused && !self.processing()));
            let selected = matches!(action, SandboxUiAction::Speed(speed) if speed == self.speed);
            let action_id = SemanticActionId::new(id);
            let bounds = PlatformRect::from_xywh(
                8.0 + index as f32 * (width + 8.0),
                transport_top,
                width,
                target,
            );
            let mut node = SemanticNode::control(
                format!("{id}.node"),
                SemanticRole::Button,
                label,
                "Living Galaxy transport",
                enabled,
                selected,
                action_id.clone(),
            );
            node.bounds = Some(bounds.into());
            nodes.push(node);
            controls.push(PlatformControl {
                semantic_id: SemanticNodeId::new(format!("{id}.node")),
                action_id,
                action: PlatformUiAction::Sandbox(action),
                bounds,
                label: label.to_owned(),
                description: "Living Galaxy transport".to_owned(),
                enabled,
                selected,
                focused: false,
                icon: None,
            });
        }
        for (index, (id, label, action)) in [
            (
                "sandbox.chronicle",
                "Chronicle",
                SandboxUiAction::ChroniclePicker,
            ),
            ("sandbox.camera", "Camera", SandboxUiAction::CameraPicker),
            (
                "sandbox.navigator",
                "Navigator",
                SandboxUiAction::NavigatorPicker,
            ),
            (
                "sandbox.inspector",
                "Inspector",
                SandboxUiAction::InspectorPicker,
            ),
            ("sandbox.save", "Save", SandboxUiAction::Save),
            (
                "sandbox.save_leave",
                "Save & leave",
                SandboxUiAction::SaveAndLeave,
            ),
            ("sandbox.library", "Library", SandboxUiAction::ListSaves),
            ("sandbox.new", "New galaxy", SandboxUiAction::ContentPicker),
            (
                "sandbox.creator",
                "Selected tools",
                SandboxUiAction::OpenCreator(self.contextual_creator_tool()),
            ),
            (
                "sandbox.history",
                "History & notebook",
                SandboxUiAction::HistoryPicker,
            ),
            (
                "sandbox.transfer",
                "Transfer",
                SandboxUiAction::TransferPicker,
            ),
            (
                "sandbox.audio",
                "Audio settings",
                SandboxUiAction::AudioPicker,
            ),
            (
                "sandbox.intro",
                if self.starter_guidance_complete() {
                    "Guide: witness"
                } else {
                    "Guide"
                },
                SandboxUiAction::IntroPicker,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let bounds = PlatformRect::from_xywh(
                8.0 + (index % toolbar_columns) as f32 * (toolbar_width + 8.0),
                toolbar_top + (index / toolbar_columns) as f32 * stride,
                toolbar_width,
                target,
            );
            push_control(
                &mut controls,
                &mut nodes,
                id,
                label,
                action,
                bounds,
                !self.storage_busy
                    && match action {
                        SandboxUiAction::ExportGalaxy => {
                            self.transfer_capabilities[0] && self.export_prepared
                        }
                        SandboxUiAction::ImportGalaxy => self.transfer_capabilities[1],
                        SandboxUiAction::ImportCatalog => self.transfer_capabilities[2],
                        _ => true,
                    },
                SemanticRole::Button,
                None,
            );
        }
        for (index, (id, label, action)) in [
            (
                "sandbox.view.galaxy",
                "Galaxy",
                SandboxUiAction::NavigateGalaxy,
            ),
            (
                "sandbox.view.system",
                "System",
                SandboxUiAction::NavigateSystem,
            ),
            (
                "sandbox.view.world",
                "World",
                SandboxUiAction::NavigateWorld,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            push_control(
                &mut controls,
                &mut nodes,
                id,
                label,
                action,
                PlatformRect::from_xywh(
                    8.0 + index as f32 * toolbar_span / 3.0,
                    navigator_top,
                    toolbar_span / 3.0 - 4.0,
                    target,
                ),
                self.selected_world.is_some() || index == 0,
                SemanticRole::Button,
                None,
            );
        }
        let scope = self.camera_scope();
        let active = match scope.level {
            LivingCameraLevel::Galaxy => SandboxUiAction::NavigateGalaxy,
            LivingCameraLevel::System => SandboxUiAction::NavigateSystem,
            LivingCameraLevel::World => SandboxUiAction::NavigateWorld,
        };
        for control in controls.iter_mut().filter(|control| {
            matches!(
                control.action,
                PlatformUiAction::Sandbox(
                    SandboxUiAction::NavigateGalaxy
                        | SandboxUiAction::NavigateSystem
                        | SandboxUiAction::NavigateWorld
                )
            )
        }) {
            control.selected = control.action == PlatformUiAction::Sandbox(active);
            if let Some(node) = nodes.iter_mut().find(|node| node.id == control.semantic_id) {
                node.selected = control.selected;
                node.role = SemanticRole::Button;
                node.description = match scope.level {
                    LivingCameraLevel::Galaxy => "Camera scope: Galaxy".to_owned(),
                    LivingCameraLevel::System => format!(
                        "Camera scope: System / {}",
                        self.state()
                            .systems
                            .as_slice()
                            .iter()
                            .find(|row| Some(row.id) == scope.entity)
                            .map_or("absent", |row| row.name.as_str())
                    ),
                    LivingCameraLevel::World => format!(
                        "Camera scope: World / {}",
                        self.state()
                            .worlds
                            .as_slice()
                            .iter()
                            .find(|row| Some(row.id) == scope.entity)
                            .map_or("absent", |row| row.name.as_str())
                    ),
                };
                control.description = node.description.clone();
            }
        }
        let mut sighted_text = if docked_inspector {
            self.inspector(
                &layout,
                viewport,
                toolbar_top,
                transport_top - 8.0,
                &mut controls,
                &mut nodes,
            )
        } else {
            Vec::new()
        };
        let drawer = sighted_text.first().map(|text| text.clip);
        let storage_bounds = PlatformRect::from_xywh(
            8.0,
            transport_top - 32.0 * layout.ui_scale,
            toolbar_span,
            24.0 * layout.ui_scale,
        );
        let mut storage_node = SemanticNode::text(
            "sandbox.storage.status",
            SemanticRole::Status,
            &self.storage_status,
            "Durable Living storage status",
        );
        storage_node.bounds = Some(storage_bounds.into());
        nodes.push(storage_node);
        sighted_text.push(PlatformSightedText {
            semantic_id: SemanticNodeId::new("sandbox.storage.status"),
            kind: InspectorTextKind::Fact,
            label: self.storage_status.clone(),
            value: None,
            lines: storage_status_display_lines(
                &self.storage_status,
                storage_bounds.width(),
                layout.ui_scale,
                1,
            ),
            bounds: storage_bounds,
            clip: storage_bounds,
        });
        let modal = if self.creator_modal_open() {
            for control in &mut controls {
                control.enabled = false;
            }
            for node in &mut nodes {
                node.enabled = false;
            }
            let (bounds, modal_nodes, modal_controls, text) =
                self.creator_frame(viewport, layout.ui_scale);
            nodes.push(SemanticNode::container(
                "sandbox.creator.dialog",
                SemanticRole::Dialog,
                if self.pack_picker {
                    "Catalog pack Library"
                } else if self.delete_review.is_some() {
                    "Delete permanently"
                } else if self.intro_picker {
                    "Living Galaxy: selection, time, creation"
                } else if self.chronicle_picker {
                    "Chronicle"
                } else if self.camera_picker {
                    "Camera"
                } else if self.navigator_picker {
                    "Navigator"
                } else if self.inspector_picker {
                    "Entity inspector"
                } else if self.import_preview.is_some() {
                    "Import preview"
                } else if self.rename_slot.is_some() {
                    "Rename saved galaxy"
                } else if self.transfer_picker {
                    "Portable transfer"
                } else if self.library_picker {
                    "Living Library"
                } else if self.audio_picker {
                    "Audio settings"
                } else if self.content_picker {
                    "Create galaxy"
                } else if self.notebook_picker {
                    "Experiments notebook"
                } else if self.history_picker {
                    "History and notebook"
                } else {
                    "Creator intervention"
                },
                modal_nodes,
            ));
            controls.extend(modal_controls);
            sighted_text.extend(text);
            Some(bounds)
        } else {
            None
        };
        let logical_focus_order = controls
            .iter()
            .filter(|control| control.enabled)
            .map(|control| control.action_id.clone())
            .collect();
        let operational_status = Some(self.fault.as_ref().map_or_else(
            || {
                if self.authority.is_viewing_history() {
                    "Viewing history: Return to present or Branch here to run or step".into()
                } else {
                    status_lines[2].clone()
                }
            },
            |fault| format!("Simulation stopped: {fault}"),
        ));
        let mut frame = PlatformUiFrame {
            viewport,
            layout,
            background: PlatformBackground::ChromeOnly,
            drawer,
            modal,
            controls,
            logical_focus_order,
            semantics: SemanticTree {
                root: SemanticNode::container(
                    "sandbox.application",
                    SemanticRole::Application,
                    "Living Galaxy V2",
                    nodes,
                ),
                announcements: Vec::new(),
            },
            title: "NYON // Living Galaxy V2".to_owned(),
            status_lines,
            sighted_text,
            visible_nodes: Vec::new(),
            source_records: Vec::new(),
            operational_status,
            high_contrast,
        };
        frame.rebuild_visible_nodes();
        frame
    }
}

const EXPERIMENT_GUIDANCE: [&str; 7] = [
    "Start Commerce or create two systems. Bookmark before the intervention; inspect alloy production and actual inter-system delivery endpoints and amounts. Stockpiles alone do not establish a supply chain.",
    "Start First Expansion, record a before state, then run without creator revisions. Pause at an actual settlement resolution receipt. Preserve its branch and tick; fleet movement is not settlement.",
    "Start Commerce. Inspect actual agreement formation and freight delivery receipts with exact parties, resource, and amount. Agreement existence alone does not establish delivery.",
    "Start Interrupted Corridor or preview a route/hazard intervention. Bookmark before and after. Inspect exact corridor and blocked-industry facts; a coincident blocked event does not establish causality.",
    "Start Frontier Friction. Preserve an actual war receipt, branch a different diplomatic creator input, and compare at the same tick. One alternative is an observation, not a general causal law.",
    "Find a dormant civilization and bookmark before. After settlement or a reviewed intervention, preserve an actual reactivation receipt. A name or inventory change is not recovery evidence.",
    "Record an exact state, prepare archive/catalog/notebook transfer, import and compare the restored branch/cursor/tick/digest. Download handoff alone does not prove durable transfer or reproduction.",
];

fn experiment_index(kind: nyon_workshop_core::living::LivingNotebookExperimentKindV2) -> u8 {
    use nyon_workshop_core::living::LivingNotebookExperimentKindV2 as K;
    match kind {
        K::SupplyChain => 0,
        K::AutonomousSettlement => 1,
        K::TradeDelivery => 2,
        K::CorridorImpact => 3,
        K::WarAlternative => 4,
        K::DormantRecovery => 5,
        K::PortableReproduction => 6,
    }
}

const EXPERIMENTS: [&str; 7] = [
    "Establish a two-system alloy supply chain.",
    "Observe settlement without creator intervention.",
    "Open a trade agreement and inspect real deliveries.",
    "Disrupt a corridor and identify affected industries.",
    "Observe a war, then branch an alternative diplomatic setup.",
    "Recover a dormant civilization through settlement or intervention.",
    "Export, import, and reproduce a selected state.",
];

#[allow(clippy::too_many_arguments)]
fn push_control(
    controls: &mut Vec<PlatformControl>,
    nodes: &mut Vec<SemanticNode>,
    id: &str,
    label: &str,
    action: SandboxUiAction,
    bounds: PlatformRect,
    enabled: bool,
    role: SemanticRole,
    value: Option<&str>,
) {
    let action_id = SemanticActionId::new(id);
    let mut node = SemanticNode::control(
        format!("{id}.node"),
        role,
        label,
        "Living Galaxy",
        enabled,
        false,
        action_id.clone(),
    );
    node.bounds = Some(bounds.into());
    node.value = value.map(str::to_owned);
    nodes.push(node);
    controls.push(PlatformControl {
        semantic_id: SemanticNodeId::new(format!("{id}.node")),
        action_id,
        action: PlatformUiAction::Sandbox(action),
        bounds,
        label: match value {
            Some(value) => format!("{label}: {value}"),
            None => label.to_owned(),
        },
        description: "Living Galaxy".to_owned(),
        enabled,
        selected: false,
        focused: false,
        icon: None,
    });
}

fn projection_changes(before: &LivingGalaxyStateV2, after: &LivingGalaxyStateV2) -> Vec<String> {
    let mut changes = Vec::new();
    macro_rules! entity_changes {
        ($field:ident,$label:literal) => {
            for row in before.$field.as_slice() {
                match after
                    .$field
                    .as_slice()
                    .iter()
                    .find(|other| other.id == row.id)
                {
                    None => changes.push(format!("Removed {} {}", $label, hex(&row.id.0))),
                    Some(other) if other != row => {
                        changes.push(format!("Changed {} {}", $label, hex(&row.id.0)))
                    }
                    _ => {}
                }
            }
            for row in after.$field.as_slice() {
                if !before
                    .$field
                    .as_slice()
                    .iter()
                    .any(|other| other.id == row.id)
                {
                    changes.push(format!("Added {} {}", $label, hex(&row.id.0)));
                }
            }
        };
    }
    entity_changes!(systems, "system");
    entity_changes!(stars, "star");
    entity_changes!(worlds, "world");
    entity_changes!(lanes, "lane");
    entity_changes!(civilizations, "civilization");
    entity_changes!(deposits, "deposit");
    entity_changes!(facilities, "facility");
    entity_changes!(construction_jobs, "construction job");
    entity_changes!(hull_jobs, "hull job");
    entity_changes!(fleets, "fleet");
    entity_changes!(routes, "freight route");
    entity_changes!(shipments, "shipment");
    entity_changes!(hazards, "hazard");
    if before.relations != after.relations {
        changes.push("Directed relations changed".to_owned());
    }
    if before.agreements != after.agreements {
        changes.push("Bilateral agreements changed".to_owned());
    }
    if before.wars != after.wars {
        changes.push("War declarations changed".to_owned());
    }
    if before.colonies != after.colonies {
        changes.push("Colony hubs changed".to_owned());
    }
    if before.observations != after.observations {
        changes.push("Civilization observations changed".to_owned());
    }
    if before.settlement_claims != after.settlement_claims {
        changes.push("Settlement claims changed".to_owned());
    }
    if before.occupations != after.occupations {
        changes.push("Occupation progress changed".to_owned());
    }
    for (label, a, b) in [
        (
            "Systems",
            before.systems.as_slice().len(),
            after.systems.as_slice().len(),
        ),
        (
            "Worlds",
            before.worlds.as_slice().len(),
            after.worlds.as_slice().len(),
        ),
        (
            "Civilizations",
            before.civilizations.as_slice().len(),
            after.civilizations.as_slice().len(),
        ),
        (
            "Facilities",
            before.facilities.as_slice().len(),
            after.facilities.as_slice().len(),
        ),
        (
            "Fleets",
            before.fleets.as_slice().len(),
            after.fleets.as_slice().len(),
        ),
        (
            "Shipments",
            before.shipments.as_slice().len(),
            after.shipments.as_slice().len(),
        ),
        (
            "Routes",
            before.routes.as_slice().len(),
            after.routes.as_slice().len(),
        ),
        (
            "Lanes",
            before.lanes.as_slice().len(),
            after.lanes.as_slice().len(),
        ),
        (
            "Deposits",
            before.deposits.as_slice().len(),
            after.deposits.as_slice().len(),
        ),
        (
            "Hazards",
            before.hazards.as_slice().len(),
            after.hazards.as_slice().len(),
        ),
        (
            "Construction jobs",
            before.construction_jobs.as_slice().len(),
            after.construction_jobs.as_slice().len(),
        ),
        (
            "Hull jobs",
            before.hull_jobs.as_slice().len(),
            after.hull_jobs.as_slice().len(),
        ),
        (
            "Colonies",
            before.colonies.as_slice().len(),
            after.colonies.as_slice().len(),
        ),
    ] {
        if a != b {
            changes.push(format!("{label}: {a} -> {b}"));
        }
    }
    for world in before.worlds.as_slice() {
        match after
            .worlds
            .as_slice()
            .iter()
            .find(|row| row.id == world.id)
        {
            None => changes.push(format!("Remove world {}", world.name.as_str())),
            Some(updated) => {
                if world.inventory != updated.inventory {
                    changes.push(format!(
                        "{} stock: E{} O{} A{} -> E{} O{} A{}",
                        world.name.as_str(),
                        world.inventory.energy,
                        world.inventory.ore,
                        world.inventory.alloy,
                        updated.inventory.energy,
                        updated.inventory.ore,
                        updated.inventory.alloy
                    ));
                }
                if world.owner != updated.owner {
                    changes.push(format!("{} changes owner", world.name.as_str()));
                }
            }
        }
    }
    if changes.is_empty() {
        changes.push(
            "Creator inputs change; inspect the previewed state before acceptance.".to_owned(),
        );
    }
    changes
}

fn canonical_history_tick(value: &str) -> Result<LivingTickV2, String> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(
            "tick must be canonical unsigned decimal (0 or digits without leading zeroes)".into(),
        );
    }
    value
        .parse::<u64>()
        .map(LivingTickV2)
        .map_err(|_| "tick exceeds u64 capacity".into())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The atlas accepts printable ASCII only. Keep the exact diagnostic in the
/// status/semantic label, and bound only its sighted projection and layout work.
fn storage_status_display_lines(
    source: &str,
    width: f32,
    scale: f32,
    max_lines: usize,
) -> Vec<String> {
    const MAX_DISPLAY_CHARACTERS: usize = 512;
    let width = width - 2.0 * crate::ui::platform_sdf::text_ink_padding(scale).min(width * 0.25);
    if width <= 0.0 || max_lines == 0 {
        return Vec::new();
    }
    let mut characters = source.chars();
    let display: String = characters
        .by_ref()
        .take(MAX_DISPLAY_CHARACTERS)
        .map(|character| {
            if character.is_whitespace() {
                ' '
            } else if (' '..='~').contains(&character) {
                character
            } else {
                '?'
            }
        })
        .collect();
    let source_truncated = characters.next().is_some();
    let metrics = AtlasMetrics::embedded().expect("embedded atlas metrics");
    let fits = |line: &str| {
        metrics
            .measure_text(15.0 * scale, FontWeight::Regular, line)
            .is_ok_and(|measure| measure.advance <= width)
    };
    let mut lines = wrap_text(&display, width, scale);
    let oversized = lines.iter().position(|line| !fits(line));
    let truncated = source_truncated || lines.len() > max_lines || oversized.is_some();
    lines.truncate(oversized.map_or(max_lines, |index| (index + 1).min(max_lines)));
    if truncated && let Some(last) = lines.last_mut() {
        let suffix = ["...", "..", ".", ""]
            .into_iter()
            .find(|suffix| fits(suffix))
            .unwrap_or("");
        while !fits(&format!("{last}{suffix}")) {
            if last.pop().is_none() {
                break;
            }
        }
        last.push_str(suffix);
    }
    lines
}

fn wrap_text(source: &str, width: f32, scale: f32) -> Vec<String> {
    let metrics = AtlasMetrics::embedded().expect("embedded atlas metrics");
    let mut lines = Vec::new();
    let mut line = String::new();
    for character in source.chars() {
        let candidate = format!("{line}{character}");
        if character == '\n'
            || (!line.is_empty()
                && metrics
                    .measure_text(15.0 * scale, FontWeight::Regular, &candidate)
                    .map_or(true, |measure| measure.advance > width))
        {
            lines.push(std::mem::take(&mut line));
        }
        if character != '\n' {
            line.push(character);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn notebook_metric_label(
    metric: nyon_workshop_core::living::LivingNotebookMetricV2,
) -> &'static str {
    use nyon_workshop_core::living::LivingNotebookMetricV2 as M;
    match metric {
        M::InventoryEnergy => "Stored energy",
        M::InventoryOre => "Stored ore",
        M::InventoryAlloy => "Stored alloy",
        M::Owner => "Owner",
        M::ColonyCount => "Colonies",
        M::FacilityCount => "Facilities",
        M::HullCount => "Hulls",
        M::Tick => "Completed tick",
    }
}

fn notebook_value_label(value: &nyon_workshop_core::living::LivingNotebookValueV2) -> String {
    use nyon_workshop_core::living::LivingNotebookValueV2 as V;
    match value {
        V::Integer(value) => value.0.to_string(),
        V::Entity(Some(entity)) => hex(&entity.0),
        V::Entity(None) => "Unowned".to_owned(),
        V::Absent => "Absent in this history".to_owned(),
    }
}

fn decision_lines(intent: &nyon_workshop_core::living::LivingIntentDecisionV2) -> Vec<String> {
    use nyon_workshop_core::living::{LivingDecisionReasonV2 as R, LivingIntentActionV2 as A};
    let id = |value: LivingEntityIdV2| hex(&value.0);
    let mut lines = vec![
        format!(
            "Actor {} / candidate {} / phase {}",
            id(intent.actor),
            intent.ordinal,
            intent.phase.ordinal()
        ),
        format!(
            "Candidate {:?}; rank {}; tier {:?}",
            intent.outcome, intent.score.0, intent.tier
        ),
        format!(
            "Snapshot tick {}; digest {}",
            intent.snapshot_tick.0,
            hex(&intent.snapshot_digest.0)
        ),
    ];
    if let Some(tick) = intent.observed_tick {
        lines.push(format!("Observed target at tick {}", tick.0));
    }
    let action = match &intent.action {
        A::Construction {
            world,
            definition,
            scrap,
        } => {
            let replacement = scrap.map_or_else(String::new, |target| {
                format!("; scrap facility {}", id(target))
            });
            format!(
                "Construct {} on world {}{replacement}",
                definition.as_str(),
                id(*world)
            )
        }
        A::Hull { world, definition } => {
            format!("Build {} hull on world {}", definition.as_str(), id(*world))
        }
        A::Route {
            source,
            destination,
            resource,
            existing_route,
        } => format!(
            "{} {:?} freight from {} to {}",
            existing_route.map_or_else(
                || "Open".to_owned(),
                |route| format!("Reopen route {}:", id(route))
            ),
            resource,
            id(*source),
            id(*destination)
        ),
        A::Explore { fleet, destination } => {
            format!("Explore {} with fleet {}", id(*destination), id(*fleet))
        }
        A::Move { fleet, destination } => {
            format!("Move fleet {} to {}", id(*fleet), id(*destination))
        }
        A::Settle { fleet, destination } => {
            format!("Settle {} with fleet {}", id(*destination), id(*fleet))
        }
        A::Defend { fleet, world } => format!("Defend {} with fleet {}", id(*world), id(*fleet)),
        A::Attack { fleet, world } => format!("Attack {} with fleet {}", id(*world), id(*fleet)),
        A::Return { fleet, world } => format!("Return fleet {} to {}", id(*fleet), id(*world)),
        A::Agreement {
            peer,
            agreement_kind,
        } => format!("Propose {:?} with {}", agreement_kind, id(*peer)),
        A::War { peer } => format!("Propose war with {}", id(*peer)),
    };
    lines.push(action);
    for reason in &intent.reasons {
        lines.push(match reason {
            R::InsufficientResource {
                world,
                resource,
                required,
                available,
                reserve,
            } => format!(
                "Insufficient {:?} on {}: need {}; available {}; preserve {}",
                resource,
                id(*world),
                required,
                available,
                reserve
            ),
            R::OutputCapacity {
                world,
                resource,
                amount,
                available_capacity,
            } => format!(
                "Output {:?} on {}: amount {}; free capacity {}",
                resource,
                id(*world),
                amount,
                available_capacity
            ),
            R::QueueCapacity {
                world,
                queue,
                limit,
                reserved,
            } => format!(
                "{} queue on {}: {} reserved of {}",
                queue.as_str(),
                id(*world),
                reserved,
                limit
            ),
            R::TargetReserved { target, winner } => {
                format!("Target {} reserved by {}", id(*target), id(*winner))
            }
            R::LowerRank {
                winning_actor,
                winning_ordinal,
            } => format!(
                "Lower rank than actor {} candidate {}",
                id(*winning_actor),
                winning_ordinal
            ),
            R::MissingIntelligence { world } => format!("No observation of world {}", id(*world)),
            R::StaleObservation {
                world,
                observed_tick,
                maximum_age,
            } => format!(
                "Observation of {} at tick {} exceeds maximum age {}",
                id(*world),
                observed_tick.0,
                maximum_age.0
            ),
            R::InvalidTarget { target, rule } => {
                format!("Target {} blocked by {}", id(*target), rule.as_str())
            }
            R::DormantRestriction { actor } => {
                format!("Dormant actor {} cannot originate this action", id(*actor))
            }
            R::BilateralThreshold {
                peer,
                forward,
                reverse,
                required,
            } => format!(
                "Relations with {}: {} forward / {} reverse; each requires {}",
                id(*peer),
                forward,
                reverse,
                required
            ),
            R::TruceActive { peer, end_tick } => {
                format!("Truce with {} until tick {}", id(*peer), end_tick.0)
            }
            R::PolicyScore {
                policy,
                component,
                value,
            } => format!(
                "Policy {} / {} contributes {}",
                policy.as_str(),
                component.as_str(),
                value.0
            ),
            R::Forecast {
                world,
                horizon,
                resource,
                predicted,
                demand,
            } => format!(
                "Forecast {:?} on {} over {} ticks: {}; demand {}",
                resource,
                id(*world),
                horizon.0,
                predicted.0,
                demand.0
            ),
            R::Accepted { rule } => format!("Admitted by {}", rule.as_str()),
        });
    }
    lines.push("Candidate acceptance records an admitted intent; scheduled completion is reported separately.".to_owned());
    lines
}

fn collect_preview_references(value: &serde_json::Value, ids: &mut BTreeSet<LivingEntityIdV2>) {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(serde_json::Value::String(hex)) = fields.get("id")
                && hex.len() == 32
                && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                let mut bytes = [0; 16];
                for (index, byte) in bytes.iter_mut().enumerate() {
                    *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                        .expect("validated hex");
                }
                ids.insert(LivingEntityIdV2(bytes));
            }
            for value in fields.values() {
                collect_preview_references(value, ids);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_preview_references(value, ids);
            }
        }
        _ => {}
    }
}

fn hex_id(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn drain_resident(session: &mut SandboxSession, budget: usize) {
        for _ in 0..2_000_000 {
            if !session.processing() {
                return;
            }
            session.poll_step(budget).unwrap();
            assert!(session.last_step_visits <= budget);
        }
        panic!("resident preparation did not finish");
    }

    #[test]
    fn resident_step_matches_reference_through_due_intents_and_dispatch() {
        for budget in [1, 64, 1024] {
            let authority = LivingContent::FirstExpansion.open().unwrap();
            let mut reference = authority.clone();
            let mut session = SandboxSession::from_authority(authority);
            for _ in 0..55 {
                let before = session.bookmark().unwrap();
                session.apply_action(SandboxUiAction::Step).unwrap();
                assert_eq!(session.bookmark().unwrap(), before);
                drain_resident(&mut session, budget);
                reference.step().unwrap();
                assert_eq!(session.state(), reference.state());
                assert_eq!(session.authority.last_receipt(), reference.last_receipt());
                assert_eq!(session.authority.last_decision(), reference.last_decision());
                assert_eq!(
                    session.authority.last_event_facts(),
                    reference.last_event_facts()
                );
                assert_eq!(
                    session.export_bytes().unwrap(),
                    encode_living_archive_v2(&reference).unwrap()
                );
            }
        }
    }

    #[test]
    fn resident_pause_hold_suspension_and_replacement_cancel_private_work() {
        for mode in 0..4 {
            let mut session = SandboxSession::open().unwrap();
            let before = session.bookmark().unwrap();
            session.apply_action(SandboxUiAction::Step).unwrap();
            assert_eq!(session.poll_step(1).unwrap(), 0);
            assert!(session.step_job.is_some());
            match mode {
                0 => session.set_paused(true),
                1 => {
                    let hold = session.begin_hold().unwrap();
                    assert!(session.end_hold(hold));
                }
                2 => {
                    session.set_suspended(true);
                    session.set_suspended(false);
                }
                _ => session.install_authority(session.authority.clone()),
            }
            assert!(!session.processing());
            assert_eq!(session.update(Duration::ZERO).unwrap(), 0);
            assert_eq!(session.bookmark().unwrap(), before);
            assert!(session.receipts().is_empty());
            assert!(session.fault().is_none());
        }
    }

    #[test]
    fn resident_restarts_capture_with_the_full_new_pending_tail() {
        use nyon_workshop_core::living::{
            LivingCommandTargetV2, LivingCreatorOperationV2, LivingNameV2,
        };
        let mut session = SandboxSession::open().unwrap();
        session.apply_action(SandboxUiAction::Step).unwrap();
        session.poll_step(1).unwrap();
        let cursor = session.authority.published_cursor();
        session
            .authority
            .submit(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Running,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![LivingCreatorOperationV2::EditSystem {
                        target: LivingCommandTargetV2::Existing {
                            id: session.state().systems.as_slice()[0].id,
                        },
                        name: LivingNameV2::new("Accepted while pending").unwrap(),
                    }],
                },
            })
            .unwrap();
        assert!(
            !session
                .step_job
                .as_ref()
                .unwrap()
                .is_current(&session.authority)
        );
        let mut reference = session.authority.clone();
        reference.step().unwrap();
        drain_resident(&mut session, 64);
        assert_eq!(session.state(), reference.state());
        assert_eq!(
            session.export_bytes().unwrap(),
            encode_living_archive_v2(&reference).unwrap()
        );
        assert_eq!(session.receipts().len(), 1);
    }

    #[test]
    fn resident_update_shares_budget_and_explicit_step_requests_exactly_one() {
        let mut session = SandboxSession::open().unwrap();
        session.apply_action(SandboxUiAction::Step).unwrap();
        session.apply_action(SandboxUiAction::Step).unwrap();
        for _ in 0..1000 {
            session.update(Duration::from_secs(600)).unwrap();
            assert!(session.last_step_visits <= 1024);
            if !session.processing() {
                break;
            }
        }
        assert_eq!(session.tick(), 1);
        assert_eq!(session.update(Duration::from_secs(600)).unwrap(), 0);
        session.set_paused(false);
        session.update(Duration::from_secs(600)).unwrap();
        assert!(session.due_ticks <= 20);
        while session.processing() {
            session.update(Duration::ZERO).unwrap();
            assert!(session.last_step_visits <= 1024);
        }
        assert_eq!(session.tick(), 21);
    }

    #[test]
    fn transport_speed_cancels_burst_and_private_step_but_retains_accepted_commands() {
        use nyon_workshop_core::living::{
            LivingCommandTargetV2, LivingCreatorOperationV2, LivingNameV2,
        };
        let mut session = SandboxSession::open().unwrap();
        session.set_paused(false);
        session.set_speed(SandboxSpeed::Fastest);
        session.update(Duration::from_secs(1)).unwrap();
        assert!(session.due_ticks > 1);
        session.poll_step(1).unwrap();
        assert!(session.step_job.is_some());
        let cursor = session.authority.published_cursor();
        session
            .authority
            .submit(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Running,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![LivingCreatorOperationV2::EditSystem {
                        target: LivingCommandTargetV2::Existing {
                            id: session.state().systems.as_slice()[0].id,
                        },
                        name: LivingNameV2::new("Retained command").unwrap(),
                    }],
                },
            })
            .unwrap();
        let accepted = session.authority.published_cursor();
        let accepted_state = session.state().clone();
        let mut reference = session.authority.clone();
        session.set_speed(SandboxSpeed::Normal);
        assert!(!session.processing());
        assert!(!session.explicit_step);
        assert_eq!(session.debt_nanos, 0);
        assert_eq!(session.authority.published_cursor(), accepted);
        assert_eq!(session.state(), &accepted_state);
        assert_eq!(session.update(Duration::ZERO).unwrap(), 0);
        session.update(Duration::from_millis(100)).unwrap();
        while session.processing() {
            session.update(Duration::ZERO).unwrap();
        }
        reference.step().unwrap();
        assert_eq!(
            session.export_bytes().unwrap(),
            encode_living_archive_v2(&reference).unwrap()
        );
        session.set_paused(true);
        session.apply_action(SandboxUiAction::Step).unwrap();
        session.poll_step(1).unwrap();
        session.set_speed(SandboxSpeed::Fast);
        assert!(!session.processing());
        assert!(!session.explicit_step);
    }

    #[test]
    fn transport_pace_counts_zero_delta_publications_and_accessible_label() {
        let mut session = SandboxSession::open().unwrap();
        session.set_paused(false);
        session.set_speed(SandboxSpeed::Fast);
        session.update(Duration::from_secs(1)).unwrap();
        while session.processing() {
            session.update(Duration::ZERO).unwrap();
        }
        assert_eq!(session.tick(), 20);
        assert_eq!(session.limited_speed, Some(2.0));
        let frame = session.frame(Vec2::new(1280.0, 720.0), false);
        assert!(
            frame
                .semantics
                .nodes_depth_first()
                .iter()
                .any(|node| node.name
                    == "Speed 4x requested; 2.00x effective; recent update work capped")
        );
        let mut pace = PublicationPace::default();
        assert_eq!(pace.record(Duration::ZERO, 1, true), None);
        assert_eq!(pace.record(Duration::from_millis(100), 0, true), Some(1.0));
        assert_eq!(pace.record(Duration::ZERO, 1, false), Some(2.0));
        session.set_paused(true);
        session.apply_action(SandboxUiAction::Step).unwrap();
        session.update(Duration::from_millis(10)).unwrap();
        while session.processing() {
            session.update(Duration::ZERO).unwrap();
        }
        assert_eq!(session.tick(), 21);
        assert!(
            session
                .publication_pace
                .samples
                .iter()
                .any(|sample| sample.count != 0)
        );
        session.apply_action(SandboxUiAction::Step).unwrap();
        assert_eq!(session.limited_speed, None);
        assert_eq!(session.publication_pace.elapsed, 0);
    }

    #[test]
    fn transport_pace_exact_window_grouping_and_bounded_storage() {
        let mut grouped = PublicationPace::default();
        let mut split = PublicationPace::default();
        for _ in 0..300 {
            let expected = grouped.record(Duration::from_millis(5), 1, true);
            for _ in 0..4 {
                split.record(Duration::from_millis(1), 0, true);
            }
            assert_eq!(split.record(Duration::from_millis(1), 1, true), expected);
            assert!(
                split
                    .samples
                    .iter()
                    .filter(|sample| sample.count != 0)
                    .count()
                    <= 200
            );
        }
        let mut pace = PublicationPace::default();
        pace.record(Duration::from_millis(100), 2, true);
        assert_eq!(
            pace.record(Duration::from_nanos(999_999_999), 0, true),
            Some(0.2)
        );
        assert_eq!(pace.record(Duration::from_nanos(1), 0, true), Some(0.0));
        assert_eq!(pace.record(Duration::MAX, 1, true), Some(0.1));
        assert_eq!(
            pace.samples
                .iter()
                .filter(|sample| sample.count != 0)
                .count(),
            1
        );
        for _ in 0..10_000 {
            pace.record(Duration::ZERO, 1, true);
        }
        assert_eq!(
            pace.samples
                .iter()
                .filter(|sample| sample.count != 0)
                .count(),
            1
        );
        assert!(pace.record(Duration::ZERO, 0, false).unwrap().is_finite());
        assert_eq!(pace.record(Duration::from_secs(1), 0, false), None);
    }

    #[test]
    fn transport_controls_reset_measured_pace() {
        for mode in 0..6 {
            let mut session = SandboxSession::open().unwrap();
            session.set_paused(false);
            session.update(Duration::from_secs(3)).unwrap();
            assert!(session.limited_speed.is_some());
            match mode {
                0 => session.set_paused(true),
                1 => {
                    let hold = session.begin_hold().unwrap();
                    assert!(session.end_hold(hold));
                }
                2 => session.set_suspended(true),
                3 => session.set_speed(SandboxSpeed::Fastest),
                4 => session.install_authority(session.authority.clone()),
                _ => {
                    let view = session.bookmark().unwrap().view;
                    session
                        .apply_action(SandboxUiAction::RestoreHistoryView(view))
                        .unwrap();
                }
            }
            assert_eq!(session.limited_speed, None);
            assert_eq!(session.publication_pace.elapsed, 0);
            assert!(
                session
                    .publication_pace
                    .samples
                    .iter()
                    .all(|sample| sample.count == 0)
            );
        }
    }

    fn finish_ui_history(session: &mut SandboxSession) {
        for _ in 0..20_000 {
            session.update(Duration::ZERO).unwrap();
            if session.history_job.is_none() && session.comparison_job.is_none() {
                return;
            }
        }
        panic!("bounded UI history job did not finish");
    }

    #[test]
    fn pack_delete_review_requires_exact_generation_hash_and_complete_unreferenced_projection() {
        let mut session = SandboxSession::open().unwrap();
        let digest = session.state().digest().unwrap();
        let hash = LivingCatalogHashV2([42; 32]);
        let row = LivingPackDescriptorV2 {
            hash,
            generation: LivingGenerationV2(1),
            archived: true,
            diagnostic: None,
        };
        let mut references = BTreeMap::new();
        session.set_pack_projection(std::slice::from_ref(&row), &references, false);
        session
            .apply_action(SandboxUiAction::OpenPackLibrary)
            .unwrap();
        session
            .apply_action(SandboxUiAction::ReviewDeletePack {
                hash,
                generation: row.generation,
            })
            .unwrap();
        session
            .edit_creator_field("pack_delete_hash", &hex(&hash.0))
            .unwrap();
        assert!(!session.pack_delete_confirmation_matches(hash, row.generation));
        references.insert(
            hash,
            vec![LivingPackReferenceV2 {
                slot: crate::living_store::LivingSlotIdV2(7),
                name: "Archived blocker".into(),
                archived: true,
                retained_generation: LivingGenerationV2(3),
            }],
        );
        session.set_pack_projection(std::slice::from_ref(&row), &references, false);
        assert!(!session.pack_delete_confirmation_matches(hash, row.generation));
        references.insert(hash, Vec::new());
        session.set_pack_projection(std::slice::from_ref(&row), &references, false);
        assert!(session.pack_delete_confirmation_matches(hash, row.generation));
        assert!(!session.pack_delete_confirmation_matches(hash, LivingGenerationV2(2)));
        session.set_pack_projection(std::slice::from_ref(&row), &references, true);
        assert!(!session.pack_delete_confirmation_matches(hash, row.generation));
        let mut changed = row.clone();
        changed.generation = LivingGenerationV2(2);
        session.set_pack_projection(&[changed], &references, false);
        assert!(!session.pack_delete_confirmation_matches(hash, row.generation));
        let protected = LivingPackDescriptorV2 {
            hash: session.builtin_pack,
            ..row
        };
        references.insert(protected.hash, Vec::new());
        session.set_pack_projection(std::slice::from_ref(&protected), &references, false);
        session
            .apply_action(SandboxUiAction::ReviewDeletePack {
                hash: protected.hash,
                generation: protected.generation,
            })
            .unwrap();
        session
            .edit_creator_field("pack_delete_hash", &hex(&protected.hash.0))
            .unwrap();
        assert!(!session.pack_delete_confirmation_matches(protected.hash, protected.generation));
        assert_eq!(session.state().digest().unwrap(), digest);
    }

    #[test]
    fn reloaded_same_tick_history_disables_transport_without_latching_fault() {
        let mut original = SandboxSession::open().unwrap();
        for _ in 0..168 {
            original.step_once().unwrap();
        }
        original
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
            .unwrap();
        original.preview_creator().unwrap();
        original.apply_creator().unwrap();
        let present = original.bookmark().unwrap();
        let mut authority = original.authority.clone();
        authority
            .restore_view(LivingHistoryViewV2 {
                branch: authority.branch(),
                revision: None,
                tick: LivingTickV2(168),
            })
            .unwrap();
        let bytes = encode_living_archive_v2(&authority).unwrap();
        let authority =
            nyon_workshop_core::living::decode_living_archive_v2(&bytes, authority.catalog())
                .unwrap()
                .into_authority();
        let mut session = SandboxSession::from_authority(authority);
        let before = session.bookmark().unwrap();
        assert!(session.authority.is_viewing_history());
        assert!(
            session
                .authority
                .retained_history_branch(session.authority.branch())
                .unwrap()
                .published_cursor()
                .committed_revision
                .is_some()
        );
        for action in [SandboxUiAction::Step, SandboxUiAction::TogglePause] {
            assert!(session.apply_action(action).is_err());
        }
        assert!(session.step_once().is_err());
        assert_eq!(session.bookmark().unwrap(), before);
        assert!(session.fault().is_none());
        let frame = session.frame(Vec2::new(1280.0, 720.0), false);
        assert!(
            frame
                .operational_status
                .unwrap()
                .contains("Viewing history")
        );
        for control in frame.controls.iter().filter(|control| {
            matches!(
                control.action,
                PlatformUiAction::Sandbox(SandboxUiAction::Step | SandboxUiAction::TogglePause)
            )
        }) {
            assert!(!control.enabled);
        }
        session
            .apply_action(SandboxUiAction::ReturnPresent)
            .unwrap();
        finish_ui_history(&mut session);
        assert_eq!(session.bookmark().unwrap(), present);
        session.step_once().unwrap();
        assert_eq!(session.tick(), 169);
        assert!(session.fault().is_none());
        session
            .apply_action(SandboxUiAction::RestoreHistoryView(before.view))
            .unwrap();
        finish_ui_history(&mut session);
        session.apply_action(SandboxUiAction::BeginBranch).unwrap();
        session.preview_creator().unwrap();
        session.apply_creator().unwrap();
        session.step_once().unwrap();
        assert_eq!(session.tick(), 169);
        assert!(session.fault().is_none());
    }

    #[test]
    fn requested_history_ticks_revisions_comparison_and_return_are_exact() {
        let mut session = SandboxSession::open().unwrap();
        for _ in 0..5 {
            session.step_once().unwrap();
        }
        session
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
            .unwrap();
        session
            .edit_creator_field("0.name", "First intervention")
            .unwrap();
        session.preview_creator().unwrap();
        session.apply_creator().unwrap();
        let first = session.authority.revisions().last().unwrap().id;
        let first_boundary = session.bookmark().unwrap();
        session
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
            .unwrap();
        session
            .edit_creator_field("0.name", "Second same tick")
            .unwrap();
        session.preview_creator().unwrap();
        session.apply_creator().unwrap();
        let second = session.authority.revisions().last().unwrap().id;
        assert_ne!(first, second);
        let left = session.authority.branch();
        for _ in 0..5 {
            session.step_once().unwrap();
        }
        session.apply_action(SandboxUiAction::BeginBranch).unwrap();
        session
            .edit_creator_field("0.name", "Alternative input")
            .unwrap();
        session.preview_creator().unwrap();
        session.apply_creator().unwrap();
        let right = session.authority.branch();
        for _ in 0..2 {
            session.step_once().unwrap();
        }
        let present = session.bookmark().unwrap();
        let expected = session
            .authority
            .compare_at(left, right, LivingTickV2(10))
            .unwrap();
        session
            .apply_action(SandboxUiAction::HistoryPicker)
            .unwrap();
        session
            .apply_action(SandboxUiAction::ComparisonBranch {
                left: true,
                branch: left,
            })
            .unwrap();
        session
            .apply_action(SandboxUiAction::ComparisonBranch {
                left: false,
                branch: right,
            })
            .unwrap();
        session.edit_creator_field("compare_tick", "10").unwrap();
        session
            .apply_action(SandboxUiAction::CompareRequestedHistory)
            .unwrap();
        finish_ui_history(&mut session);
        assert_eq!(session.bookmark().unwrap(), present);
        let actual = session.comparison().unwrap();
        assert_eq!(actual.left, expected.left);
        assert_eq!(actual.right, expected.right);
        session
            .apply_action(SandboxUiAction::HistoryBranch(left))
            .unwrap();
        session
            .apply_action(SandboxUiAction::HistoryRevision(Some(first)))
            .unwrap();
        session.edit_creator_field("history_tick", "5").unwrap();
        session
            .apply_action(SandboxUiAction::RestoreRequestedHistory)
            .unwrap();
        finish_ui_history(&mut session);
        assert_eq!(session.bookmark().unwrap(), first_boundary);
        session
            .apply_action(SandboxUiAction::ReturnPresent)
            .unwrap();
        finish_ui_history(&mut session);
        assert_eq!(session.bookmark().unwrap(), present);
    }

    #[test]
    fn invalid_requested_history_and_cancel_preserve_installed_view_and_results() {
        let mut session = SandboxSession::open().unwrap();
        for _ in 0..12 {
            session.step_once().unwrap();
        }
        session
            .apply_action(SandboxUiAction::HistoryPicker)
            .unwrap();
        session.edit_creator_field("compare_tick", "8").unwrap();
        session
            .apply_action(SandboxUiAction::CompareRequestedHistory)
            .unwrap();
        finish_ui_history(&mut session);
        let previous = session.comparison().unwrap().left;
        let installed = session.bookmark().unwrap();
        for invalid in ["", "00", "+8", "-1", "1.0", "18446744073709551616", "13"] {
            session.edit_creator_field("history_tick", invalid).unwrap();
            assert!(
                session
                    .apply_action(SandboxUiAction::RestoreRequestedHistory)
                    .is_err()
            );
            session.edit_creator_field("compare_tick", invalid).unwrap();
            assert!(
                session
                    .apply_action(SandboxUiAction::CompareRequestedHistory)
                    .is_err()
            );
            assert_eq!(session.bookmark().unwrap(), installed);
            assert_eq!(session.comparison().unwrap().left, previous);
            assert!(session.history_job.is_none() && session.comparison_job.is_none());
            assert!(session.present.is_none());
        }
        let foreign = LivingBranchIdV2([255; 16]);
        assert!(
            session
                .apply_action(SandboxUiAction::HistoryBranch(foreign))
                .is_err()
        );
        assert!(
            session
                .apply_action(SandboxUiAction::HistoryRevision(Some(LivingRevisionIdV2(
                    [255; 32]
                ))))
                .is_err()
        );
        session.edit_creator_field("history_tick", "8").unwrap();
        session
            .apply_action(SandboxUiAction::RestoreRequestedHistory)
            .unwrap();
        session
            .apply_action(SandboxUiAction::CancelHistory)
            .unwrap();
        finish_ui_history(&mut session);
        assert_eq!(session.bookmark().unwrap(), installed);
        assert_eq!(session.comparison().unwrap().left, previous);
    }

    #[test]
    fn semantic_input_owner_survives_edit_preview_but_rejects_replaced_draft_and_modal() {
        let mut session = SandboxSession::open().unwrap();
        let resident = session.presentation_input_owner();
        session
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
            .unwrap();
        let original = session.presentation_input_owner();
        let context = session.presentation_input_context();
        assert!(Rc::ptr_eq(&resident.0, &original.0));
        assert_ne!(resident.1, original.1);
        let cloned = session.creator.as_ref().unwrap().clone();
        assert!(Rc::ptr_eq(
            original.2.as_ref().unwrap(),
            &cloned.input_identity()
        ));
        session
            .edit_creator_field("0.name", "Reviewed system")
            .unwrap();
        session.preview_creator().unwrap();
        let reviewed = session.presentation_input_owner();
        assert_eq!(context, session.presentation_input_context());
        assert_eq!(original.1, reviewed.1);
        assert!(Rc::ptr_eq(
            original.2.as_ref().unwrap(),
            reviewed.2.as_ref().unwrap()
        ));
        session
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
            .unwrap();
        let replaced = session.presentation_input_owner();
        assert_eq!(original.1, replaced.1);
        assert!(!Rc::ptr_eq(
            original.2.as_ref().unwrap(),
            replaced.2.as_ref().unwrap()
        ));
        session
            .apply_action(SandboxUiAction::CreatorCancel)
            .unwrap();
        session
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
            .unwrap();
        let reopened = session.presentation_input_owner();
        assert_ne!(replaced.1, reopened.1);
        assert!(!Rc::ptr_eq(
            replaced.2.as_ref().unwrap(),
            reopened.2.as_ref().unwrap()
        ));
        let other = SandboxSession::open().unwrap().presentation_input_owner();
        assert!(!Rc::ptr_eq(&resident.0, &other.0));
    }

    #[test]
    fn semantic_modal_context_changes_when_existing_hold_is_reused() {
        let mut session = SandboxSession::open().unwrap();
        session
            .apply_action(SandboxUiAction::NavigatorPicker)
            .unwrap();
        let navigator = session.presentation_input_context();
        let hold = session.presentation_input_owner().1;
        session.apply_action(SandboxUiAction::AudioPicker).unwrap();
        assert_eq!(hold, session.presentation_input_owner().1);
        assert_ne!(navigator, session.presentation_input_context());
        let audio = session.presentation_input_context();
        session
            .apply_action(SandboxUiAction::TransferPicker)
            .unwrap();
        assert_ne!(audio, session.presentation_input_context());
    }

    #[test]
    fn delete_review_requires_exact_name_generation_and_known_continue() {
        use crate::living_store::{LivingGenerationV2, LivingSlotIdV2};
        let mut session = SandboxSession::open().unwrap();
        let row = LivingSlotDescriptorV2 {
            id: LivingSlotIdV2(7),
            name: "My Galaxy".to_owned(),
            archived: true,
            generation: LivingGenerationV2(3),
            catalog_hash: session.authority.catalog().catalog_hash(),
            diagnostic: None,
        };
        session.set_storage_projection("Ready", std::slice::from_ref(&row), false);
        session
            .apply_action(SandboxUiAction::DeleteSave(7))
            .unwrap();
        let before = session.state().digest().unwrap();
        session
            .edit_creator_field("delete_name", "My Galaxy")
            .unwrap();
        assert!(!session.delete_confirmation_matches(7, row.generation, &row.name));
        session.set_living_continue_projection(Some(7));
        assert!(session.delete_confirmation_matches(7, row.generation, &row.name));
        assert!(!session.delete_confirmation_matches(7, LivingGenerationV2(4), &row.name));
        session
            .edit_creator_field("delete_name", "my galaxy")
            .unwrap();
        assert!(!session.delete_confirmation_matches(7, row.generation, &row.name));
        session.dismiss_delete_confirmation();
        assert!(session.delete_review_descriptor().is_none());
        assert_eq!(session.state().digest().unwrap(), before);
    }

    #[test]
    fn unrelated_receipt_cannot_certify_experiment() {
        let mut session = SandboxSession::open().unwrap();
        session.step_once().unwrap();
        let before = session.notebook_snapshot();
        let generation = session.notebook_generation();
        for index in 0..7 {
            assert!(
                session
                    .apply_action(SandboxUiAction::RecordExperiment(index))
                    .is_err()
            );
        }
        assert_eq!(session.notebook_snapshot(), before);
        assert_eq!(session.notebook_generation(), generation);
    }

    #[test]
    fn history_genesis_is_reachable_without_notebook_metadata() {
        let mut session = SandboxSession::open().unwrap();
        session.step_once().unwrap();
        session
            .apply_action(SandboxUiAction::HistoryPicker)
            .unwrap();
        let reachable = (0..8).any(|page| {
            session.inspector_page = page;
            session
                .frame(Vec2::new(800.0, 480.0), false)
                .controls
                .iter()
                .any(|control| {
                    matches!(
                        control.action,
                        PlatformUiAction::Sandbox(SandboxUiAction::RestoreHistoryView(_))
                    )
                })
        });
        assert!(reachable);
        let view = LivingHistoryViewV2 {
            branch: session.authority.branch(),
            revision: None,
            tick: nyon_workshop_core::living::LivingTickV2(0),
        };
        session
            .apply_action(SandboxUiAction::RestoreHistoryView(view))
            .unwrap();
        for _ in 0..16 {
            session.update(Duration::ZERO).unwrap();
        }
        assert_eq!(session.tick(), 0);
        assert!(session.notebook_snapshot().0.is_empty());
    }

    #[test]
    fn navigator_arrows_expand_collapse_and_follow_parent_child_without_commands() {
        let mut session = SandboxSession::open().unwrap();
        let rows = navigator_rows(session.state());
        let child = rows
            .iter()
            .find(|row| row.parent.is_some())
            .unwrap()
            .clone();
        let parent = child.parent.unwrap();
        let before = session.state().digest().unwrap();
        session.navigator_expanded.remove(&parent);
        session
            .apply_action(SandboxUiAction::NavigatorRight(parent))
            .unwrap();
        assert_eq!(session.selected_entity(), Some(parent));
        assert!(session.navigator_expanded.contains(&parent));
        session
            .apply_action(SandboxUiAction::NavigatorRight(parent))
            .unwrap();
        let first_child = rows
            .iter()
            .find(|row| row.parent == Some(parent))
            .unwrap()
            .selection;
        assert_eq!(session.selected_entity(), Some(first_child));
        session.navigator_expanded.remove(&first_child);
        session
            .apply_action(SandboxUiAction::NavigatorLeft(first_child))
            .unwrap();
        assert_eq!(session.selected_entity(), Some(parent));
        session
            .apply_action(SandboxUiAction::NavigatorLeft(parent))
            .unwrap();
        assert!(!session.navigator_expanded.contains(&parent));
        assert!(session.take_requested_focus().is_some());
        assert!(session.take_requested_focus().is_none());
        assert_eq!(session.state().digest().unwrap(), before);
    }

    #[test]
    fn preview_ghost_and_parent_snap_are_private_and_stale_preview_disappears() {
        use crate::presentation::living::LivingGhostKind;
        let mut session = SandboxSession::open().unwrap();
        let digest = session.state().digest().unwrap();
        session
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
            .unwrap();
        session.preview_creator().unwrap();
        let bounds = PlatformRect::from_xywh(0.0, 0.0, 800.0, 480.0);
        let scene = session.scene_frame(bounds);
        assert!(
            scene
                .ghosts
                .iter()
                .any(|ghost| ghost.kind == LivingGhostKind::Added)
        );
        let frame = session.frame(Vec2::new(800.0, 480.0), false);
        assert!(frame.modal.unwrap().min.y > 250.0);
        assert!(
            frame
                .controls
                .iter()
                .filter(|control| control.enabled)
                .all(|control| control.bounds.max.y - control.bounds.min.y >= 44.0)
        );
        session
            .edit_creator_field("0.name", "Changed draft")
            .unwrap();
        assert!(session.scene_frame(bounds).ghosts.is_empty());
        session
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::World))
            .unwrap();
        let system = session.state().systems.as_slice()[1].id;
        session
            .apply_action(SandboxUiAction::SnapCreatorParent(
                LivingEntitySelection::new(LivingEntityKind::System, system),
            ))
            .unwrap();
        assert_eq!(
            session.creator_field_value("0.system.id"),
            Some(
                crate::living_navigation::entity_id(system)
                    .to_ascii_lowercase()
                    .as_str()
            )
        );
        assert!(session.scene_frame(bounds).ghosts.is_empty());
        assert_eq!(session.state().digest().unwrap(), digest);
        session.preview_creator().unwrap();
        // The initial camera now remembers its fit. New draft entities can lie
        // outside that framing; the explicit preview fit reveals both states.
        session
            .apply_action(SandboxUiAction::CreatorFitPreview)
            .unwrap();
        assert!(
            session
                .scene_frame(bounds)
                .ghosts
                .iter()
                .any(|ghost| ghost.selection.kind == LivingEntityKind::World)
        );
        assert_eq!(session.state().digest().unwrap(), digest);
        let incompatible = LivingEntitySelection::new(
            LivingEntityKind::Civilization,
            session.state().civilizations.as_slice()[0].id,
        );
        assert!(
            session
                .apply_action(SandboxUiAction::SnapCreatorParent(incompatible))
                .is_err()
        );
        assert!(session.scene_frame(bounds).ghosts.is_empty());
        assert_eq!(session.state().digest().unwrap(), digest);
    }

    #[test]
    fn chronicle_uses_exact_producer_facts_and_creator_never_inherits_them() {
        let mut session = SandboxSession::open().unwrap();
        for _ in 0..20 {
            session.step_once().unwrap();
        }
        let receipt = session.receipts().back().unwrap();
        let facts = session.authority.last_event_facts().unwrap();
        facts.verify_receipt(receipt).unwrap();
        let explained = session
            .chronicle_rows()
            .into_iter()
            .find(|row| {
                row.facts
                    .as_ref()
                    .is_some_and(|facts| !facts.blocked_reasons.is_empty())
            })
            .unwrap();
        assert_eq!(explained.receipt_digest, receipt.digest);
        assert!(explained.actor.is_some());
        assert!(explained.entity.is_some());
        session
            .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
            .unwrap();
        session.preview_creator().unwrap();
        session.apply_creator().unwrap();
        assert!(session.authority.last_event_facts().is_none());
        let receipt = session.receipts().back().unwrap();
        assert!(
            session
                .chronicle_rows()
                .into_iter()
                .filter(|row| row.receipt_digest == receipt.digest)
                .all(|row| row.facts.is_none())
        );
    }

    #[test]
    fn retained_fault_blocks_retries_and_disables_transport() {
        let mut session = SandboxSession::open().unwrap();
        let digest = session.state().digest().unwrap();
        // Inject only a presentation fault latch; authority remains private and
        // untouched. Atomic authority failures are covered by the core suite.
        session.fault = Some("test boundary fault".to_owned());
        session.set_paused(false);
        assert!(session.paused());
        assert_eq!(session.update(Duration::from_secs(1)).unwrap(), 0);
        assert_eq!(session.step_once().unwrap_err(), "test boundary fault");
        assert_eq!(session.state().digest().unwrap(), digest);
        let frame = session.frame(Vec2::new(1280.0, 720.0), false);
        assert!(
            frame
                .controls
                .iter()
                .filter(|control| matches!(
                    control.action,
                    PlatformUiAction::Sandbox(
                        SandboxUiAction::TogglePause
                            | SandboxUiAction::Step
                            | SandboxUiAction::Speed(_)
                    )
                ))
                .all(|control| !control.enabled)
        );
        assert!(frame.status_lines[1].contains("test boundary fault"));
    }
}

/// Keep display rows within their byte budget without corrupting Unicode text.
fn ui_text_chunks(text: &str) -> impl Iterator<Item = &str> {
    let mut remaining = text;
    std::iter::from_fn(move || {
        if remaining.is_empty() {
            return None;
        }
        let mut end = remaining.len().min(32);
        while !remaining.is_char_boundary(end) {
            end -= 1;
        }
        if end < remaining.len()
            && let Some((index, character)) = remaining[..end]
                .char_indices()
                .rfind(|(_, character)| character.is_whitespace())
            && remaining[..index]
                .chars()
                .any(|character| !character.is_whitespace())
        {
            end = index + character.len_utf8();
        }
        let (row, tail) = remaining.split_at(end);
        remaining = tail;
        Some(row)
    })
}

#[cfg(test)]
mod unicode_display_tests {
    use super::ui_text_chunks;

    #[test]
    fn adjacent_full_width_tokens_do_not_create_blank_rows() {
        let text = "0123456789abcdef0123456789abcdef 0123456789abcdef0123456789abcdef";
        let rows: Vec<_> = ui_text_chunks(text).collect();
        assert_eq!(rows.concat(), text);
        assert!(
            rows.iter()
                .all(|row| row.len() <= 32 && !row.trim().is_empty())
        );
    }

    #[test]
    fn readable_rows_keep_words_and_full_entity_ids_together_when_they_fit() {
        let text = "World / d55d23459d7bef7418c57828392d1f26 has matching delivery observations";
        let rows: Vec<_> = ui_text_chunks(text).collect();
        assert_eq!(rows.concat(), text);

        assert!(rows.contains(&"d55d23459d7bef7418c57828392d1f26"));
        assert!(rows.iter().all(|row| row.len() <= 32));
        assert!(!rows.iter().any(|row| row.ends_with("observ")));
    }

    #[test]
    fn inspector_rows_preserve_unicode_at_byte_boundaries() {
        for text in [
            "",
            "a",
            "1234567890123456789012345678901🌌界é",
            "星系文明🌌".repeat(24).as_str(),
        ] {
            let rows: Vec<_> = ui_text_chunks(text).collect();
            assert_eq!(rows.concat(), text);
            assert!(rows.iter().all(|row| !row.is_empty() && row.len() <= 32));
        }
    }
}

#[cfg(test)]
mod storage_status_display_tests {
    use super::*;
    use crate::ui::platform_sdf::build_platform_ui_batch;

    fn assert_status_composes(raw: &str, library: bool) {
        let metrics = AtlasMetrics::embedded().unwrap();
        let mut session = SandboxSession::open().unwrap();
        if library {
            session.open_library().unwrap();
        }
        let bookmark = session.bookmark().unwrap();
        let digest = session.state().digest().unwrap();
        let dirty = session.dirty();
        session.set_storage_projection(raw, &[], false);
        let id = if library {
            "sandbox.library.status"
        } else {
            "sandbox.storage.status"
        };
        for _ in 0..3 {
            let frame = session.frame(Vec2::new(1280.0, 800.0), false);
            build_platform_ui_batch(&frame, &metrics).unwrap();
            let status = frame
                .sighted_text
                .iter()
                .find(|text| text.semantic_id.as_str() == id)
                .unwrap();
            assert_eq!(status.label.as_bytes(), raw.as_bytes());
            assert_eq!(session.storage_status.as_bytes(), raw.as_bytes());
            let nodes = frame.semantics.nodes_depth_first();
            let node = nodes.iter().find(|node| node.id.as_str() == id).unwrap();
            assert_eq!(node.name.as_bytes(), raw.as_bytes());
            assert!(
                status
                    .lines
                    .iter()
                    .all(|line| { line.bytes().all(|byte| (b' '..=b'~').contains(&byte)) })
            );
        }
        session.set_storage_projection("Saved", &[], false);
        let frame = session.frame(Vec2::new(1280.0, 800.0), false);
        let output = build_platform_ui_batch(&frame, &metrics).unwrap();
        assert!(
            output
                .text_runs
                .iter()
                .any(|run| { run.semantic_id.as_str() == id && run.exact_text == "Saved" })
        );
        assert_eq!(session.bookmark().unwrap(), bookmark);
        assert_eq!(session.state().digest().unwrap(), digest);
        assert_eq!(session.dirty(), dirty);
    }

    #[test]
    fn storage_status_footer_composes_raw_exception_without_changing_semantics() {
        for raw in [
            "quota\nretry",
            "quota\r\nretry",
            "quota\tretry",
            "quota\0\x01retry",
            "quota é界🌌 retry",
        ] {
            assert_status_composes(raw, false);
        }
    }

    #[test]
    fn storage_status_library_composes_raw_exception_without_changing_semantics() {
        for raw in [
            "quota\rretry",
            "quota\nretry",
            "quota\tretry",
            "quota\0\x01retry",
            "quota é界🌌 retry",
        ] {
            assert_status_composes(raw, true);
        }
    }

    #[test]
    fn storage_status_long_exception_fits_bounded_rows_at_supported_scales() {
        let metrics = AtlasMetrics::embedded().unwrap();
        let raw = format!(
            "QuotaExceededError\n\r\t界\0{} retry save or refresh",
            "long detail ".repeat(1_000)
        );
        for library in [false, true] {
            let mut session = SandboxSession::open().unwrap();
            if library {
                session.open_library().unwrap();
            }
            session.set_storage_projection(&raw, &[], false);
            for viewport in [
                Vec2::new(800.0, 480.0),
                Vec2::new(1280.0, 800.0),
                Vec2::new(1920.0, 1080.0),
            ] {
                for scale in [0.85, 1.0, 1.30] {
                    let frame = session.frame_with_scale(viewport, scale, false);
                    let output = build_platform_ui_batch(&frame, &metrics).unwrap();
                    for status in frame.sighted_text.iter().filter(|text| {
                        matches!(
                            text.semantic_id.as_str(),
                            "sandbox.storage.status" | "sandbox.library.status"
                        )
                    }) {
                        let max_lines = if status.semantic_id.as_str() == "sandbox.storage.status" {
                            1
                        } else {
                            2
                        };
                        assert!(status.lines.len() <= max_lines);
                        assert!(status.lines.last().unwrap().ends_with("..."));
                        assert_eq!(status.label.as_bytes(), raw.as_bytes());
                        let width = status.bounds.width()
                            - 2.0 * crate::ui::platform_sdf::text_ink_padding(scale);
                        for line in &status.lines {
                            assert!(line.len() <= 512);
                            assert!(line.bytes().all(|byte| (b' '..=b'~').contains(&byte)));
                            assert!(
                                metrics
                                    .measure_text(15.0 * scale, FontWeight::Regular, line)
                                    .unwrap()
                                    .advance
                                    <= width
                            );
                        }
                        let runs: Vec<_> = output
                            .text_runs
                            .iter()
                            .filter(|run| run.semantic_id == status.semantic_id)
                            .collect();
                        // Modal scope hides the footer while Library is open.
                        if !library || status.semantic_id.as_str() == "sandbox.library.status" {
                            assert_eq!(runs.len(), status.lines.len());
                            for (run, line) in runs.iter().zip(&status.lines) {
                                assert_eq!(&run.exact_text, line);
                                assert!(run.clip.contains_rect(run.bounds));
                            }
                        }
                    }
                }
            }
            assert_eq!(session.storage_status, raw);
        }
    }

    #[test]
    fn storage_status_projection_bounds_prefix_and_handles_subglyph_width() {
        let metrics = AtlasMetrics::embedded().unwrap();
        assert_eq!(
            storage_status_display_lines("a\nb\rc\td\0é", 1000.0, 1.0, 1),
            ["a b c d??"]
        );
        let lines = storage_status_display_lines(&"x".repeat(10_000), 10_000.0, 1.0, 1);
        assert_eq!(lines[0], format!("{}...", "x".repeat(512)));
        for width in [0.0, 0.1, 1.0, 3.0, 6.0, 10.0, 20.0] {
            let padded_width =
                width - 2.0 * crate::ui::platform_sdf::text_ink_padding(1.0).min(width * 0.25);
            let lines = storage_status_display_lines("WW\n界\0long", width, 1.0, 2);
            assert!(lines.len() <= 2);
            assert!(lines.iter().all(|line| {
                metrics
                    .measure_text(15.0, FontWeight::Regular, line)
                    .unwrap()
                    .advance
                    <= padded_width
            }));
        }
    }
}
