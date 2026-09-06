use nyon::ui::platform_sdf::PlatformTextOverflow;
use nyon::ui::platform_sdf::PlatformTextRole;

#[test]
fn public_visible_record_remains_constructible_outside_the_crate() {
    use nyon::ui::{
        accessibility::SemanticNodeId,
        platform::PlatformRect,
        platform_sdf::{PlatformVisibleNodeRecord, PlatformVisibleNodeState},
    };
    let record = PlatformVisibleNodeRecord {
        semantic_id: SemanticNodeId::new("external.status"),
        action_id: None,
        display_text: "Ready".to_owned(),
        semantic_name: "Ready".to_owned(),
        semantic_description: String::new(),
        semantic_value: None,
        role: PlatformTextRole::Status,
        overflow: PlatformTextOverflow::Wrap,
        bounds: PlatformRect::from_xywh(0.0, 0.0, 100.0, 40.0),
        clip: None,
        state: PlatformVisibleNodeState::default(),
        icon: None,
        prewrapped_lines: None,
    };
    assert!(record.informative());
}
