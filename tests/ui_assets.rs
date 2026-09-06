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

#[test]
fn text_measurement_uses_exact_weighted_entries_and_matches_emission() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let wide = metrics
        .measure_text(40.0, FontWeight::Regular, "WWW")
        .unwrap();
    let narrow = metrics
        .measure_text(40.0, FontWeight::Regular, "iii")
        .unwrap();
    assert_ne!(wide.advance, narrow.advance);

    let regular = metrics
        .measure_text(40.0, FontWeight::Regular, "Aa")
        .unwrap();
    let semibold = metrics
        .measure_text(40.0, FontWeight::SemiBold, "Aa")
        .unwrap();
    let expected_regular = metrics.entries[(b'A' - b' ') as usize].advance
        + metrics.entries[(b'a' - b' ') as usize].advance;
    let expected_semibold = metrics.entries[95 + (b'A' - b' ') as usize].advance
        + metrics.entries[95 + (b'a' - b' ') as usize].advance;
    assert_eq!(regular.advance, expected_regular);
    assert_eq!(semibold.advance, expected_semibold);

    let mut batch = UiBatch::default();
    let emitted = batch
        .push_text(
            &metrics,
            [100.0, 100.0],
            40.0,
            FontWeight::Regular,
            [1.0; 4],
            "Aa",
        )
        .unwrap();
    assert_eq!(emitted, regular.advance);
    assert_eq!(batch.glyphs()[0].uv_rect[0], 64.0 / 1024.0);
    assert_eq!(batch.glyphs()[1].uv_rect[0], 64.0 / 1024.0);
    assert_ne!(batch.glyphs()[0].uv_rect[1], batch.glyphs()[1].uv_rect[1]);
}

#[test]
fn measurement_and_emission_reject_bad_input_and_mapping_atomically() {
    let metrics = AtlasMetrics::embedded().unwrap();
    assert_eq!(
        metrics.measure_text(16.0, FontWeight::Regular, "line\nbreak"),
        Err(UiBatchError::UnsupportedCharacter('\n'))
    );
    assert_eq!(
        metrics.measure_text(f32::NAN, FontWeight::Regular, "A"),
        Err(UiBatchError::NonFiniteGeometry)
    );

    let mut batch = UiBatch::default();
    batch
        .push_icon(&metrics, UiIcon::Check, [1.0, 2.0, 3.0, 4.0], [1.0; 4])
        .unwrap();
    let before = batch.glyphs().to_vec();
    let mut inconsistent = metrics.clone();
    inconsistent.entries[(b'A' - b' ') as usize].codepoint = Some('B' as u32);
    assert!(matches!(
        batch.push_text(
            &inconsistent,
            [0.0, 0.0],
            16.0,
            FontWeight::Regular,
            [1.0; 4],
            "A"
        ),
        Err(UiBatchError::MissingEntry(_))
    ));
    assert_eq!(batch.glyphs(), before);
    let mut inconsistent_key = metrics.clone();
    inconsistent_key.entries[(b'A' - b' ') as usize].key = "regular:U+0042".into();
    assert!(matches!(
        inconsistent_key.measure_text(16.0, FontWeight::Regular, "A"),
        Err(UiBatchError::MissingEntry(_))
    ));
    assert_eq!(
        batch.push_text_clipped(
            &metrics,
            [0.0, 0.0],
            16.0,
            FontWeight::Regular,
            [1.0; 4],
            "A",
            Some([0.0, 0.0, f32::INFINITY, 1.0])
        ),
        Err(UiBatchError::NonFiniteGeometry)
    );
    assert_eq!(batch.glyphs(), before);
}

#[test]
fn clipped_text_and_icons_crop_rect_and_uv_proportionally() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let cases = [
        (
            [108.0, 52.0, 48.0, 64.0],
            [108.0, 52.0, 48.0, 64.0],
            [0.25, 0.0],
        ),
        (
            [92.0, 52.0, 48.0, 64.0],
            [92.0, 52.0, 48.0, 64.0],
            [0.0, 0.0],
        ),
        (
            [92.0, 68.0, 64.0, 48.0],
            [92.0, 68.0, 64.0, 48.0],
            [0.0, 0.25],
        ),
        (
            [92.0, 52.0, 64.0, 48.0],
            [92.0, 52.0, 64.0, 48.0],
            [0.0, 0.0],
        ),
    ];
    let entry = &metrics.entries[(b'A' - b' ') as usize];
    let base_uv = [
        entry.atlas_bounds.x as f32 / 1024.0,
        entry.atlas_bounds.y as f32 / 1024.0,
    ];
    for (clip, expected_rect, uv_offset) in cases {
        let mut batch = UiBatch::default();
        batch
            .push_text_clipped(
                &metrics,
                [100.0, 100.0],
                40.0,
                FontWeight::Regular,
                [1.0; 4],
                "A",
                Some(clip),
            )
            .unwrap();
        let glyph = batch.glyphs()[0];
        assert_eq!(glyph.rect, expected_rect);
        assert_eq!(glyph.uv_rect[0], base_uv[0] + uv_offset[0] * 64.0 / 1024.0);
        assert_eq!(glyph.uv_rect[1], base_uv[1] + uv_offset[1] * 64.0 / 1024.0);
        assert_eq!(glyph.uv_rect[2], expected_rect[2] / 1024.0);
        assert_eq!(glyph.uv_rect[3], expected_rect[3] / 1024.0);
    }

    let mut icon_batch = UiBatch::default();
    icon_batch
        .push_icon_clipped(
            &metrics,
            UiIcon::Check,
            [10.0, 20.0, 40.0, 40.0],
            [1.0; 4],
            Some([20.0, 30.0, 20.0, 20.0]),
        )
        .unwrap();
    let icon = icon_batch.glyphs()[0];
    assert_eq!(icon.rect, [20.0, 30.0, 20.0, 20.0]);
    assert_eq!(icon.uv_rect[2], 32.0 / 1024.0);
    assert_eq!(icon.uv_rect[3], 32.0 / 1024.0);
}

#[test]
fn invisible_text_retains_advance_without_consuming_capacity() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let measured = metrics
        .measure_text(40.0, FontWeight::Regular, "A A")
        .unwrap();
    let mut batch = UiBatch::default();
    assert_eq!(
        batch
            .push_text(
                &metrics,
                [0.0, 48.0],
                40.0,
                FontWeight::Regular,
                [1.0; 4],
                "",
            )
            .unwrap(),
        0.0
    );
    batch
        .push_text(
            &metrics,
            [0.0, 48.0],
            40.0,
            FontWeight::Regular,
            [1.0; 4],
            "A",
        )
        .unwrap();
    batch
        .push_text(
            &metrics,
            [0.0, 96.0],
            40.0,
            FontWeight::Regular,
            [1.0; 4],
            "A",
        )
        .unwrap();
    assert_ne!(batch.glyphs()[0].rect[1], batch.glyphs()[1].rect[1]);
    batch.clear();
    let emitted = batch
        .push_text_clipped(
            &metrics,
            [100.0, 100.0],
            40.0,
            FontWeight::Regular,
            [1.0; 4],
            "A A",
            Some([0.0, 0.0, 0.0, 0.0]),
        )
        .unwrap();
    assert_eq!(emitted, measured.advance);
    assert!(batch.glyphs().is_empty());
    batch
        .push_text_clipped(
            &metrics,
            [100.0, 100.0],
            40.0,
            FontWeight::Regular,
            [1.0; 4],
            "A",
            Some([156.0, 52.0, 1.0, 64.0]),
        )
        .unwrap();
    assert!(batch.glyphs().is_empty());
}

#[test]
fn capacity_boundaries_are_atomic_for_nonempty_batches() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut batch = UiBatch::default();
    for _ in 0..MAX_UI_GLYPHS - 1 {
        batch
            .push_icon(&metrics, UiIcon::Check, [0.0, 0.0, 1.0, 1.0], [1.0; 4])
            .unwrap();
    }
    assert_eq!(batch.glyphs().len(), MAX_UI_GLYPHS - 1);
    let before = batch.glyphs().to_vec();
    assert!(matches!(
        batch.push_text(
            &metrics,
            [0.0, 48.0],
            40.0,
            FontWeight::Regular,
            [1.0; 4],
            "AA"
        ),
        Err(UiBatchError::Capacity { requested }) if requested == MAX_UI_GLYPHS + 1
    ));
    assert_eq!(batch.glyphs(), before);
    batch
        .push_icon(&metrics, UiIcon::Check, [0.0, 0.0, 1.0, 1.0], [1.0; 4])
        .unwrap();
    assert_eq!(batch.glyphs().len(), MAX_UI_GLYPHS);
    let before = batch.glyphs().to_vec();
    assert!(matches!(
        batch.push_icon(&metrics, UiIcon::Check, [0.0, 0.0, 1.0, 1.0], [1.0; 4]),
        Err(UiBatchError::Capacity { requested }) if requested == MAX_UI_GLYPHS + 1
    ));
    assert_eq!(batch.glyphs(), before);

    let mut panels = UiBatch::default();
    for _ in 0..MAX_UI_PANELS - 1 {
        panels.push_panel([0.0, 0.0, 1.0, 1.0], [1.0; 4]).unwrap();
    }
    panels.push_panel([0.0, 0.0, 1.0, 1.0], [1.0; 4]).unwrap();
    assert_eq!(panels.panels().len(), MAX_UI_PANELS);
    let before = panels.panels().to_vec();
    assert!(matches!(
        panels.push_panel([0.0, 0.0, 1.0, 1.0], [1.0; 4]),
        Err(UiBatchError::PanelCapacity { requested }) if requested == MAX_UI_PANELS + 1
    ));
    assert_eq!(panels.panels(), before);
}

#[test]
fn text_emission_stops_at_the_first_capacity_breach() {
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut batch = UiBatch::default();
    batch
        .push_icon(&metrics, UiIcon::Check, [0.0, 0.0, 1.0, 1.0], [1.0; 4])
        .unwrap();
    let before = batch.glyphs().to_vec();
    let value = format!("{}☃", "A".repeat(MAX_UI_GLYPHS + 256));

    assert_eq!(
        batch.push_text(
            &metrics,
            [0.0, 48.0],
            40.0,
            FontWeight::Regular,
            [1.0; 4],
            &value,
        ),
        Err(UiBatchError::Capacity {
            requested: MAX_UI_GLYPHS + 1,
        })
    );
    assert_eq!(batch.glyphs(), before);
}
