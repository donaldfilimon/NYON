use std::{collections::HashSet, fs};

use nyon::{
    engine::shader::validate_wgsl,
    ui::{
        AtlasMetrics, AtlasValidationError, EntryKind, FontWeight, MAX_UI_GLYPHS, MAX_UI_PANELS,
        UI_ATLAS_BYTES, UI_ATLAS_ENTRY_COUNT, UI_ATLAS_HEIGHT, UI_ATLAS_WIDTH, UI_SDF_WGSL,
        UiBatch, UiBatchError, UiGlyphInstance, UiIcon, UiPanelInstance, text,
    },
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct HashManifest {
    format_version: u32,
    algorithm: String,
    archives: Vec<ArchiveHash>,
    files: Vec<FileHash>,
}

#[derive(Deserialize)]
struct ArchiveHash {
    url: String,
    sha256: String,
    committed: bool,
}

#[derive(Deserialize)]
struct FileHash {
    path: String,
    sha256: String,
}

#[test]
fn embedded_atlas_metrics_and_payload_are_valid() {
    let metrics = AtlasMetrics::embedded().expect("committed metrics must parse");
    metrics.validate().expect("committed atlas must validate");
    assert_eq!(
        UI_ATLAS_BYTES.len(),
        (UI_ATLAS_WIDTH * UI_ATLAS_HEIGHT) as usize
    );
    assert!(UI_ATLAS_BYTES.iter().any(|value| *value > 128));
}

#[test]
fn atlas_has_unique_cells_and_exact_required_coverage() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let keys: HashSet<_> = metrics
        .entries
        .iter()
        .map(|entry| entry.key.as_str())
        .collect();
    let cells: HashSet<_> = metrics
        .entries
        .iter()
        .map(|entry| entry.atlas_bounds)
        .collect();
    assert_eq!(keys.len(), metrics.entries.len());
    assert_eq!(cells.len(), metrics.entries.len());
    assert_eq!(metrics.entries.len(), UI_ATLAS_ENTRY_COUNT);

    for weight in ["regular", "semibold"] {
        for codepoint in 0x20_u32..=0x7e {
            assert!(keys.contains(format!("{weight}:U+{codepoint:04X}").as_str()));
        }
    }
    for icon in UiIcon::ALL {
        assert!(keys.contains(icon.key()));
    }
}

#[test]
fn all_recorded_committed_hashes_match_and_archives_are_auditable() {
    let manifest: HashManifest = serde_json::from_str(include_str!("../assets/ui/hashes.json"))
        .expect("hash manifest must parse");
    assert_eq!(manifest.format_version, 1);
    assert_eq!(manifest.algorithm, "sha256");
    assert_eq!(manifest.archives.len(), 2);
    for archive in manifest.archives {
        assert!(archive.url.starts_with("https://github.com/"));
        assert_eq!(archive.sha256.len(), 64);
        assert!(archive.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(!archive.committed);
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in manifest.files {
        let bytes = fs::read(root.join(&file.path))
            .unwrap_or_else(|error| panic!("cannot read recorded file {}: {error}", file.path));
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            file.sha256,
            "{}",
            file.path
        );
    }
}

#[test]
fn ui_shader_and_instance_contract_stay_synchronized() {
    validate_wgsl("assets/shaders/ui_sdf.wgsl", UI_SDF_WGSL)
        .expect("the shipped UI shader must pass Naga validation");
    assert_eq!(std::mem::size_of::<UiGlyphInstance>(), 48);
    assert_eq!(UiGlyphInstance::LAYOUT.array_stride, 48);
    assert_eq!(std::mem::size_of::<UiPanelInstance>(), 32);
    assert_eq!(UiPanelInstance::LAYOUT.array_stride, 32);
    assert!(UI_SDF_WGSL.contains("@location(0) rect: vec4<f32>"));
    assert!(UI_SDF_WGSL.contains("@location(1) uv_rect: vec4<f32>"));
    assert!(UI_SDF_WGSL.contains("@location(2) color: vec4<f32>"));
    assert!(UI_SDF_WGSL.contains("textureSample(ui_atlas"));
    assert!(UI_SDF_WGSL.contains("fn panel_vs(input: PanelInput)"));
    assert!(UI_SDF_WGSL.contains("fn panel_fs(input: PanelOutput)"));
}

#[test]
fn ui_batch_is_bounded_and_rejects_non_ascii_text_atomically() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut batch = UiBatch::default();
    let width = batch
        .push_text(
            &metrics,
            [16.0, 48.0],
            20.0,
            FontWeight::SemiBold,
            [0.2, 0.9, 1.0, 1.0],
            "COMMAND READY",
        )
        .unwrap();
    assert!(width > 0.0);
    let before = batch.glyphs().len();
    assert_eq!(
        batch.push_text(
            &metrics,
            [0.0, 0.0],
            16.0,
            FontWeight::Regular,
            [1.0; 4],
            "INVALID ☃",
        ),
        Err(UiBatchError::UnsupportedCharacter('☃'))
    );
    assert_eq!(batch.glyphs().len(), before);

    for _ in batch.glyphs().len()..MAX_UI_GLYPHS {
        batch
            .push_icon(&metrics, UiIcon::Check, [0.0, 0.0, 16.0, 16.0], [1.0; 4])
            .unwrap();
    }
    assert!(matches!(
        batch.push_icon(&metrics, UiIcon::Check, [0.0, 0.0, 16.0, 16.0], [1.0; 4]),
        Err(UiBatchError::Capacity { .. })
    ));

    for _ in 0..MAX_UI_PANELS {
        batch
            .push_panel([0.0, 0.0, 44.0, 44.0], [0.0, 0.0, 0.0, 0.9])
            .unwrap();
    }
    assert!(matches!(
        batch.push_panel([0.0, 0.0, 44.0, 44.0], [0.0, 0.0, 0.0, 0.9]),
        Err(UiBatchError::PanelCapacity { .. })
    ));
}

#[test]
fn committed_interface_strings_are_printable_ascii() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut batch = UiBatch::default();
    for (index, value) in text::ALL.iter().enumerate() {
        assert!(
            value
                .chars()
                .all(|character| character.is_ascii_graphic() || character == ' ')
        );
        batch
            .push_text(
                &metrics,
                [8.0, 24.0 + index as f32 * 20.0],
                16.0,
                FontWeight::Regular,
                [1.0; 4],
                value,
            )
            .unwrap();
    }
    for value in [
        "SOURCE ASTER VALE",
        "TARGET BASTION REACH",
        "STRENGTH 120",
        "GUIDANCE 1/6",
    ] {
        batch
            .push_text(
                &metrics,
                [8.0, 320.0],
                16.0,
                FontWeight::Regular,
                [1.0; 4],
                value,
            )
            .unwrap();
    }
    assert!(!batch.glyphs().is_empty());

    let before = batch.glyphs().to_vec();
    assert_eq!(
        batch.push_text(
            &metrics,
            [8.0, 340.0],
            16.0,
            FontWeight::Regular,
            [1.0; 4],
            "OS ERROR: CAFÉ",
        ),
        Err(UiBatchError::UnsupportedCharacter('É'))
    );
    assert_eq!(batch.glyphs(), before);
}

#[test]
fn ui_batch_rejects_all_derived_non_finite_geometry_atomically() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut batch = UiBatch::default();
    batch
        .push_icon(&metrics, UiIcon::Check, [0.0, 0.0, 16.0, 16.0], [1.0; 4])
        .unwrap();
    let before = batch.glyphs().to_vec();

    assert_eq!(
        batch.push_text(
            &metrics,
            [0.0, 0.0],
            f32::MAX,
            FontWeight::Regular,
            [1.0; 4],
            "A",
        ),
        Err(UiBatchError::NonFiniteGeometry)
    );
    assert_eq!(batch.glyphs(), before);

    let long_spaces = " ".repeat(64);
    assert_eq!(
        batch.push_text(
            &metrics,
            [0.0, 0.0],
            f32::MAX / 2.0,
            FontWeight::Regular,
            [1.0; 4],
            &long_spaces,
        ),
        Err(UiBatchError::NonFiniteGeometry)
    );
    assert_eq!(batch.glyphs(), before);

    let mut zero_font_metrics = metrics.clone();
    zero_font_metrics.generator.font_px = 0.0;
    assert_eq!(
        batch.push_text(
            &zero_font_metrics,
            [0.0, 0.0],
            16.0,
            FontWeight::Regular,
            [1.0; 4],
            "A",
        ),
        Err(UiBatchError::NonFiniteGeometry)
    );
    assert_eq!(batch.glyphs(), before);

    let mut nan_font_metrics = metrics.clone();
    nan_font_metrics.generator.font_px = f32::NAN;
    assert_eq!(
        batch.push_text(
            &nan_font_metrics,
            [0.0, 0.0],
            16.0,
            FontWeight::Regular,
            [1.0; 4],
            "A",
        ),
        Err(UiBatchError::NonFiniteGeometry)
    );
    assert_eq!(batch.glyphs(), before);

    let mut huge_advance_metrics = metrics.clone();
    huge_advance_metrics.entries[33].advance = f32::MAX;
    assert_eq!(
        batch.push_text(
            &huge_advance_metrics,
            [f32::MAX, 0.0],
            40.0,
            FontWeight::Regular,
            [1.0; 4],
            "AA",
        ),
        Err(UiBatchError::NonFiniteGeometry)
    );
    assert_eq!(batch.glyphs(), before);
}

#[test]
fn metrics_reject_generator_mapping_and_overlap_drift() {
    let metrics = AtlasMetrics::embedded().unwrap();

    let mut changed_generator = metrics.clone();
    changed_generator.generator.font_px = 0.0;
    assert_eq!(
        changed_generator.validate(),
        Err(AtlasValidationError::GeneratorDescriptor)
    );

    let mut changed_count = metrics.clone();
    changed_count.entries.pop();
    assert!(matches!(
        changed_count.validate(),
        Err(AtlasValidationError::EntryCount { .. })
    ));

    let mut changed_key = metrics.clone();
    changed_key.entries[0].key = "regular:U+FFFF".into();
    assert!(matches!(
        changed_key.validate(),
        Err(AtlasValidationError::EntryMapping(_))
    ));

    let mut changed_codepoint = metrics.clone();
    changed_codepoint.entries[0].codepoint = Some(33);
    assert!(matches!(
        changed_codepoint.validate(),
        Err(AtlasValidationError::EntryMapping(_))
    ));

    let mut changed_weight = metrics.clone();
    changed_weight.entries[0].font_weight = Some("semibold".into());
    assert!(matches!(
        changed_weight.validate(),
        Err(AtlasValidationError::EntryMapping(_))
    ));

    let mut changed_kind = metrics.clone();
    changed_kind.entries[0].kind = EntryKind::Icon;
    assert!(matches!(
        changed_kind.validate(),
        Err(AtlasValidationError::EntryMapping(_))
    ));

    let mut changed_icon = metrics.clone();
    let icon_index = changed_icon.entries.len() - UiIcon::ALL.len();
    changed_icon.entries[icon_index].icon = Some("pause".into());
    assert!(matches!(
        changed_icon.validate(),
        Err(AtlasValidationError::EntryMapping(_))
    ));

    let mut overlap = metrics.clone();
    overlap.entries[1].atlas_bounds.x = 32;
    assert!(matches!(
        overlap.validate(),
        Err(AtlasValidationError::OverlappingBounds(_))
    ));

    let mut misaligned = metrics.clone();
    misaligned.entries[0].atlas_bounds.x = 1;
    assert!(matches!(
        misaligned.validate(),
        Err(AtlasValidationError::CellGeometry(_))
    ));
}
