//! Host contrast proof over actual emitted SDF glyph/fill instances; not raster or display calibration.
use glam::Vec2;
use nyon::{
    sandbox_session::SandboxSession,
    ui::{
        AtlasMetrics,
        platform_sdf::{PlatformPanelRole, build_platform_ui_batch},
    },
};
fn luminance(color: [f32; 4], srgb: bool) -> f64 {
    let channel = |value: f32| {
        let value = f64::from(value);
        if srgb {
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        } else {
            value
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}
fn composite(color: [f32; 4], backdrop: [f32; 4]) -> [f32; 4] {
    [
        color[0] * color[3] + backdrop[0] * (1.0 - color[3]),
        color[1] * color[3] + backdrop[1] * (1.0 - color[3]),
        color[2] * color[3] + backdrop[2] * (1.0 - color[3]),
        1.0,
    ]
}
fn ratio(foreground: [f32; 4], background: [f32; 4], srgb: bool) -> f64 {
    let a = luminance(foreground, srgb);
    let b = luminance(background, srgb);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
#[test]
fn active_living_glyphs_meet_four_point_five_against_emitted_fill_in_all_states() {
    let metrics = AtlasMetrics::embedded().unwrap();
    for high_contrast in [false, true] {
        for (focused, selected) in [(false, false), (true, false), (false, true), (true, true)] {
            let session = SandboxSession::open().unwrap();
            let mut frame = session.frame(Vec2::new(1280.0, 800.0), high_contrast);
            let controls: Vec<_> = frame
                .controls
                .iter_mut()
                .filter(|c| c.enabled)
                .map(|c| {
                    c.focused = focused;
                    c.selected = selected;
                    c.semantic_id.clone()
                })
                .collect();
            assert!(!controls.is_empty());
            let output = build_platform_ui_batch(&frame, &metrics).unwrap();
            let mut checked = 0;
            for id in controls {
                let panel = output
                    .panel_witnesses
                    .iter()
                    .find(|p| {
                        p.role == PlatformPanelRole::ControlFill
                            && p.owning_node.as_ref() == Some(&id)
                    })
                    .unwrap();
                let fill = output.batch.panels()[panel.emitted_panel_range[0]].color;
                for run in output.text_runs.iter().filter(|r| r.semantic_id == id) {
                    for glyph in &output.batch.glyphs()
                        [run.emitted_glyph_range[0]..run.emitted_glyph_range[1]]
                    {
                        for backdrop in [[0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0]] {
                            let background = composite(fill, backdrop);
                            let foreground = composite(glyph.color, background);
                            for srgb in [false, true] {
                                let actual = ratio(foreground, background, srgb);
                                assert!(
                                    actual >= 4.5,
                                    "contrast {actual:.3} high={high_contrast} focused={focused} selected={selected} srgb={srgb} control={} glyph={:?} fill={fill:?}",
                                    id.as_str(),
                                    glyph.color
                                );
                            }
                        }
                        checked += 1;
                    }
                }
            }
            assert!(checked > 0);
        }
    }
}
