use glam::Vec2;
use nyon::{
    living_navigation::{LivingEntityKind, LivingEntitySelection},
    presentation::living::LivingSceneFrame,
    ui::{AtlasMetrics, FontWeight, UiBatch, platform::PlatformRect},
};
use nyon_workshop_core::living::{LivingEntityIdV2, builtin_living_sandbox_v2};

fn fixture() -> LivingSceneFrame {
    let authority = builtin_living_sandbox_v2().unwrap();
    let mut frame = LivingSceneFrame::extract(
        authority.state(),
        PlatformRect::from_xywh(0.0, 0.0, 420.0, 220.0),
    );
    let mut marker = frame.entities[0].clone();
    marker.selection =
        LivingEntitySelection::new(LivingEntityKind::World, LivingEntityIdV2([1; 16]));
    marker.position = Vec2::new(210.0, 190.0);
    marker.visible = true;
    marker.name = "Wide WWW and narrow iii".into();
    frame.entities = vec![marker];
    frame.selected_entity = Some(frame.entities[0].selection);
    frame
}

#[test]
fn measured_labels_move_above_edge_and_contain_actual_atlas_quads() {
    let frame = fixture();
    let metrics = AtlasMetrics::embedded().unwrap();
    let labels = frame.measured_labels(&metrics);
    assert_eq!(labels.len(), 1);
    let label = &labels[0];
    assert!(label.bounds.max.y < frame.entities[0].position.y);
    let mut batch = UiBatch::default();
    batch
        .push_text(
            &metrics,
            label.baseline.to_array(),
            13.0,
            FontWeight::Regular,
            [1.0; 4],
            &label.text,
        )
        .unwrap();
    let canvas = frame.bounds.unwrap();
    for glyph in batch.glyphs() {
        let [x, y, width, height] = glyph.rect;
        let max = Vec2::new(x + width, y + height);
        assert!(!Vec2::new(x, y).cmplt(canvas.min).any());
        assert!(!max.cmpgt(canvas.max).any());
        assert!(
            !Vec2::new(x, y)
                .cmplt(label.bounds.min - Vec2::splat(0.001))
                .any()
        );
        assert!(!max.cmpgt(label.bounds.max + Vec2::splat(0.001)).any());
    }
    assert_eq!(labels, frame.measured_labels(&metrics));
}

#[test]
fn selected_label_has_priority_and_labels_avoid_other_marker_bodies() {
    let mut frame = fixture();
    frame.entities[0].position = Vec2::new(210.0, 100.0);
    frame.entities[0].name = "Selected".into();
    let mut other = frame.entities[0].clone();
    other.selection.id = LivingEntityIdV2([2; 16]);
    other.position = Vec2::new(210.0, 128.0);
    other.name = "Neighbour".into();
    frame.entities.push(other);
    let metrics = AtlasMetrics::embedded().unwrap();
    let labels = frame.measured_labels(&metrics);
    assert_eq!(labels[0].selection, frame.selected_entity.unwrap());
    assert!(
        labels[0].bounds.max.y < 100.0,
        "blocked lower anchor must use an alternative"
    );
    for (index, label) in labels.iter().enumerate() {
        for other in &labels[index + 1..] {
            assert!(label.bounds.intersection(other.bounds).is_none());
        }
        for marker in &frame.entities {
            let body = PlatformRect::from_xywh(
                marker.position.x - 16.0,
                marker.position.y - 16.0,
                32.0,
                32.0,
            );
            assert!(label.bounds.intersection(body).is_none());
        }
    }
}

#[test]
fn labels_use_glyph_advances_and_omit_uncontainable_names() {
    let mut frame = fixture();
    frame.entities[0].position = Vec2::new(210.0, 100.0);
    let metrics = AtlasMetrics::embedded().unwrap();
    frame.entities[0].name = "WWW".into();
    let wide = frame.measured_labels(&metrics)[0].bounds.width();
    frame.entities[0].name = "iii".into();
    let narrow = frame.measured_labels(&metrics)[0].bounds.width();
    assert!(
        wide > narrow,
        "equal byte lengths must not imply equal measured widths"
    );
    frame.entities[0].name = "W".repeat(200);
    assert!(
        frame.measured_labels(&metrics).is_empty(),
        "never clip or cap a label's measured width"
    );
}

#[test]
fn occluding_controls_reanchor_complete_glyph_quads_and_fully_hidden_labels_omit() {
    let mut frame = fixture();
    frame.entities[0].position = Vec2::new(210.0, 100.0);
    frame.entities[0].name = "Selected world".into();
    let metrics = AtlasMetrics::embedded().unwrap();
    let ordinary = frame.measured_labels(&metrics);
    assert_eq!(ordinary, frame.measured_labels_avoiding(&metrics, &[]));
    let control = ordinary[0].bounds;
    let labels = frame.measured_labels_avoiding(&metrics, &[control]);
    assert_eq!(labels.len(), 1);
    assert_ne!(labels[0].baseline, ordinary[0].baseline);
    assert_eq!(labels[0].selection, frame.selected_entity.unwrap());
    let mut batch = UiBatch::default();
    batch
        .push_text(
            &metrics,
            labels[0].baseline.to_array(),
            13.0,
            FontWeight::Regular,
            [1.0; 4],
            &labels[0].text,
        )
        .unwrap();
    for glyph in batch.glyphs() {
        let [x, y, width, height] = glyph.rect;
        assert!(
            PlatformRect::from_xywh(x, y, width, height)
                .intersection(control)
                .is_none(),
            "actual SDF quads must avoid control geometry"
        );
    }
    assert!(
        frame
            .measured_labels_avoiding(&metrics, &[frame.bounds.unwrap()])
            .is_empty()
    );
    assert_eq!(
        frame.entities.len(),
        1,
        "occlusion does not remove entities"
    );
}

#[test]
fn actual_inventory_readout_records_reanchor_labels_at_logical_browser_geometry() {
    use nyon::{
        living_content::LivingContent, sandbox_session::SandboxSession,
        ui::platform_sdf::PlatformTextRole,
    };
    let session = SandboxSession::from_authority(LivingContent::ValeConfluence.open().unwrap());
    let metrics = AtlasMetrics::embedded().unwrap();
    for scale in [1.0, 1.5] {
        let ui = session.frame_with_scale(Vec2::new(1084.0, 610.0), scale, false);
        let inventory = ui
            .visible_nodes
            .iter()
            .find(|r| {
                r.role == PlatformTextRole::Status
                    && r.display_text.contains("ore")
                    && r.display_text.contains("alloy")
            })
            .expect("actual inventory status record");
        let mut scene = fixture();
        scene.bounds = Some(ui.layout.canvas);
        scene.entities[0].position = Vec2::new(177.0, inventory.bounds.min.y - 19.0);
        scene.entities[0].name = "Otharbury".into();
        let mut prior_exclusions: Vec<_> = ui
            .controls
            .iter()
            .map(|c| c.bounds)
            .chain(ui.sighted_text.iter().map(|t| t.bounds))
            .collect();
        prior_exclusions.extend(ui.layout.chrome.iter().copied());
        prior_exclusions.extend(ui.drawer);
        let prior = scene.measured_labels_avoiding(&metrics, &prior_exclusions);
        // The reported normal-scale geometry reproduces the gap. Enlarged
        // chrome can already obscure all old anchors, so it is not a defect oracle.
        if scale == 1.0 {
            assert!(
                prior
                    .iter()
                    .any(|label| label.bounds.intersection(inventory.bounds).is_some()),
                "readouts were outside the prior exclusions"
            );
        }
        let labels = scene.measured_labels_for_ui(&metrics, &ui);
        for label in &labels {
            assert!(label.bounds.intersection(inventory.bounds).is_none());
            let mut batch = UiBatch::default();
            batch
                .push_text(
                    &metrics,
                    label.baseline.to_array(),
                    13.0,
                    FontWeight::Regular,
                    [1.0; 4],
                    &label.text,
                )
                .unwrap();
            for glyph in batch.glyphs() {
                let [x, y, w, h] = glyph.rect;
                assert!(
                    PlatformRect::from_xywh(x, y, w, h)
                        .intersection(inventory.bounds)
                        .is_none(),
                    "full glyph quad overlaps actual readout"
                );
            }
        }
        assert_eq!(
            scene.entities.len(),
            1,
            "omitted labels retain Navigator entities"
        );
    }
}
