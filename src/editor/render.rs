use glam::Vec2;

use crate::engine::primitives::PrimitiveBatch;

use super::{EditorRect, EditorState, WidgetKind};

/// Emits editor geometry solely from the ordered widget list.
pub fn build_frame(editor: &EditorState, batch: &mut PrimitiveBatch) {
    batch.clear();
    let viewport = editor.viewport;
    batch.quad(viewport * 0.5, viewport * 0.5, [0.018, 0.027, 0.075, 1.0]);
    for widget in editor.widgets.iter().filter(|widget| widget.visible) {
        let focused = editor.focused.as_deref() == Some(widget.id.as_str());
        let color = if widget.kind == WidgetKind::Confirmation {
            [0.12, 0.08, 0.22, 1.0]
        } else if widget.error.is_some() {
            [0.65, 0.12, 0.16, 1.0]
        } else if focused {
            [0.08, 0.28, 0.38, 1.0]
        } else if widget.enabled {
            [0.06, 0.14, 0.24, 1.0]
        } else {
            [0.04, 0.07, 0.11, 1.0]
        };
        batch.quad(
            widget.rect.center(),
            Vec2::new(widget.rect.width, widget.rect.height) * 0.5,
            color,
        );
        if focused {
            outline_rect(batch, widget.rect, 2.0, [0.2, 0.95, 1.0, 1.0]);
        }
        let max_chars = ((widget.rect.width - 16.0).max(0.0) / 6.0) as usize;
        batch.text(
            Vec2::new(widget.rect.x + 8.0, widget.rect.y + 5.0),
            1.0,
            [0.72, 0.92, 1.0, 1.0],
            &truncate_ascii(&widget.label, max_chars),
        );
        if !widget.value.is_empty() {
            batch.text(
                Vec2::new(widget.rect.x + 8.0, widget.rect.y + 20.0),
                1.0,
                [0.9, 0.98, 1.0, 1.0],
                &truncate_ascii(&widget.value, max_chars),
            );
        }
        if let Some(error) = &widget.error {
            batch.text(
                Vec2::new(widget.rect.x + 8.0, widget.rect.y + 34.0),
                0.7,
                [1.0, 0.78, 0.72, 1.0],
                &truncate_ascii(&error.to_ascii_uppercase(), max_chars),
            );
        }
    }
}

fn outline_rect(batch: &mut PrimitiveBatch, rect: EditorRect, width: f32, color: [f32; 4]) {
    let min = Vec2::new(rect.x, rect.y);
    let max = Vec2::new(rect.x + rect.width, rect.y + rect.height);
    batch.line(min, Vec2::new(max.x, min.y), width, color);
    batch.line(Vec2::new(max.x, min.y), max, width, color);
    batch.line(max, Vec2::new(min.x, max.y), width, color);
    batch.line(Vec2::new(min.x, max.y), min, width, color);
}

fn truncate_ascii(value: &str, maximum: usize) -> String {
    if value.len() <= maximum {
        return value.to_owned();
    }
    if maximum <= 3 {
        return value.chars().take(maximum).collect();
    }
    let mut truncated: String = value.chars().take(maximum - 3).collect();
    truncated.push_str("...");
    truncated
}
