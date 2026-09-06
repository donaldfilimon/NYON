use glam::Vec2;

const SHAPE_KIND_MASK: u32 = 0xff;
const QUAD_SHAPE: u32 = 0;
const RING_SHAPE: u32 = 2;
const RING_WIDTH_MAX: u32 = (1 << 24) - 1;

/// A tightly packed vertex shared by every CPU-produced primitive.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub local: [f32; 2],
    pub shape: u32,
}

impl Vertex {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
        2 => Float32x2,
        3 => Uint32,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBUTES,
    };
}

/// CPU-owned geometry for one frame.  The renderer only uploads this packed data.
#[derive(Default)]
pub struct PrimitiveBatch {
    vertices: Vec<Vertex>,
}

impl PrimitiveBatch {
    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn quad(&mut self, center: Vec2, half_size: Vec2, color: [f32; 4]) {
        self.emit_rect(center, half_size.max(Vec2::ZERO), color, QUAD_SHAPE);
    }

    pub fn disc(&mut self, center: Vec2, radius: f32, color: [f32; 4]) {
        if !radius.is_finite() {
            return;
        }
        let radius = radius.max(0.0);
        // Metal does not reliably render the dedicated analytic-disc kind on
        // every supported device, while the ring path is shared and stable.
        // A ring whose thickness equals its radius is exactly a filled disc;
        // the shader has an explicit maximum-width endpoint for this case.
        self.emit_rect(
            center,
            Vec2::splat(radius),
            color,
            packed_ring_shape(radius, radius),
        );
    }

    pub fn ring(&mut self, center: Vec2, radius: f32, thickness: f32, color: [f32; 4]) {
        if !radius.is_finite() || !thickness.is_finite() {
            return;
        }
        let radius = radius.max(0.0);
        self.emit_rect(
            center,
            Vec2::splat(radius),
            color,
            packed_ring_shape(radius, thickness),
        );
    }

    pub fn line(&mut self, from: Vec2, to: Vec2, width: f32, color: [f32; 4]) {
        if !from.is_finite() || !to.is_finite() || !width.is_finite() || !color_is_finite(color) {
            return;
        }
        let delta = to - from;
        if !delta.is_finite() {
            return;
        }
        let length = delta.length();
        if !length.is_finite() {
            return;
        }
        let direction = if length > f32::EPSILON {
            delta / length
        } else {
            Vec2::X
        };
        if !direction.is_finite() {
            return;
        }
        let perpendicular = Vec2::new(-direction.y, direction.x);
        let half_width = width.max(0.0) * 0.5;
        let center = from + delta * 0.5;
        let along = direction * (length * 0.5);
        let across = perpendicular * half_width;
        self.emit_corners(
            [
                center - along - across,
                center + along - across,
                center + along + across,
                center - along + across,
            ],
            color,
            QUAD_SHAPE,
        );
    }

    pub fn text(&mut self, origin: Vec2, scale: f32, color: [f32; 4], value: &str) {
        if !origin.is_finite() || !scale.is_finite() || !color_is_finite(color) {
            return;
        }
        let scale = scale.max(0.0);
        for (glyph_index, byte) in value.bytes().enumerate() {
            let x_offset = glyph_index as f32 * 6.0 * scale;
            let glyph = glyph_for(byte).unwrap_or(BOXED_REPLACEMENT_GLYPH);
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) != 0 {
                        let cell_min = origin
                            + Vec2::new(x_offset + column as f32 * scale, row as f32 * scale);
                        self.quad(
                            cell_min + Vec2::splat(scale * 0.5),
                            Vec2::splat(scale * 0.5),
                            color,
                        );
                    }
                }
            }
        }
    }

    fn emit_rect(&mut self, center: Vec2, half_size: Vec2, color: [f32; 4], shape: u32) {
        if !center.is_finite() || !half_size.is_finite() || !color_is_finite(color) {
            return;
        }
        let min = center - half_size;
        let max = center + half_size;
        self.emit_corners(
            [
                Vec2::new(min.x, min.y),
                Vec2::new(max.x, min.y),
                Vec2::new(max.x, max.y),
                Vec2::new(min.x, max.y),
            ],
            color,
            shape,
        );
    }

    fn emit_corners(&mut self, corners: [Vec2; 4], color: [f32; 4], shape: u32) {
        if !color_is_finite(color) || corners.iter().any(|corner| !corner.is_finite()) {
            return;
        }
        const LOCAL: [Vec2; 4] = [
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        for index in [0, 1, 2, 0, 2, 3] {
            self.vertices.push(Vertex {
                position: corners[index].to_array(),
                color,
                local: LOCAL[index].to_array(),
                shape,
            });
        }
    }
}

fn packed_ring_shape(radius: f32, thickness: f32) -> u32 {
    let ratio = if radius > 0.0 {
        (thickness.max(0.0) / radius).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let width = (ratio * RING_WIDTH_MAX as f32).round() as u32;
    (width << 8) | (RING_SHAPE & SHAPE_KIND_MASK)
}

fn color_is_finite(color: [f32; 4]) -> bool {
    color.iter().all(|component| component.is_finite())
}

const BOXED_REPLACEMENT_GLYPH: [u8; 7] = [
    0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
];

fn glyph_for(byte: u8) -> Option<[u8; 7]> {
    let glyph = match byte {
        b'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        b'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        b'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        b'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        b'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        b'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        b'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        b'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        b'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        b'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        b'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        b'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        b'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        b'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        b'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        b'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        b'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        b'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        b'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        b'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        b'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        b'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        b'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        b'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        b'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        b'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        b'0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        b'1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        b'2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        b'3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        b'4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        b'5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        b'6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        b'7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        b'8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        b'9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b11100,
        ],
        b':' => [0, 0b00100, 0, 0, 0b00100, 0, 0],
        b'-' => [0, 0, 0, 0b11111, 0, 0, 0],
        b'/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0, 0],
        b'%' => [0b11001, 0b11010, 0b00100, 0b01000, 0b10110, 0b00110, 0],
        b'.' => [0, 0, 0, 0, 0, 0b00100, 0b00100],
        b' ' => [0; 7],
        _ => return None,
    };
    Some(glyph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn vertex_has_the_planned_packed_gpu_layout() {
        assert_eq!(std::mem::size_of::<Vertex>(), 36);
        assert_eq!(Vertex::LAYOUT.array_stride, 36);
        assert_eq!(Vertex::ATTRIBUTES[0].offset, 0);
        assert_eq!(Vertex::ATTRIBUTES[1].offset, 8);
        assert_eq!(Vertex::ATTRIBUTES[2].offset, 24);
        assert_eq!(Vertex::ATTRIBUTES[3].offset, 32);
    }

    #[test]
    fn quad_emits_two_triangles() {
        let mut batch = PrimitiveBatch::default();
        batch.quad(Vec2::ZERO, Vec2::ONE, [1.0; 4]);
        assert_eq!(batch.vertices().len(), 6);
        assert!(batch.vertices().iter().all(|vertex| vertex.shape == 0));
    }

    #[test]
    fn disc_uses_the_full_width_ring_path_and_line_remains_a_quad() {
        let mut batch = PrimitiveBatch::default();
        batch.disc(Vec2::ZERO, 5.0, [1.0; 4]);
        batch.ring(Vec2::ZERO, 8.0, 2.0, [1.0; 4]);
        batch.line(Vec2::ZERO, Vec2::X * 10.0, 2.0, [1.0; 4]);
        assert_eq!(batch.vertices().len(), 18);
        assert!(
            batch.vertices()[..6]
                .iter()
                .all(|vertex| vertex.shape == 0xffff_ff02)
        );
        assert!(
            batch.vertices()[6..12]
                .iter()
                .all(|vertex| vertex.shape & 0xff == 2)
        );
        assert!(
            batch.vertices()[12..]
                .iter()
                .all(|vertex| vertex.shape & 0xff == 0)
        );
    }

    #[test]
    fn ring_packs_width_without_changing_the_low_byte_kind() {
        let mut thin = PrimitiveBatch::default();
        thin.ring(Vec2::ZERO, 10.0, 2.0, [1.0; 4]);
        let mut thick = PrimitiveBatch::default();
        thick.ring(Vec2::ZERO, 10.0, 6.0, [1.0; 4]);

        assert!(
            thin.vertices()
                .iter()
                .all(|vertex| vertex.shape & 0xff == 2)
        );
        assert!(
            thick
                .vertices()
                .iter()
                .all(|vertex| vertex.shape & 0xff == 2)
        );
        assert_ne!(thin.vertices()[0].shape, thick.vertices()[0].shape);
        assert!(thin.vertices()[0].shape >> 8 < thick.vertices()[0].shape >> 8);
    }

    #[test]
    fn bitmap_glyph_vertices_stay_inside_their_seven_by_five_cell() {
        let origin = Vec2::new(10.0, 20.0);
        let mut batch = PrimitiveBatch::default();
        batch.text(origin, 1.0, [1.0; 4], "A");
        assert!(!batch.vertices().is_empty());
        assert!(batch.vertices().iter().all(|vertex| {
            (10.0..=15.0).contains(&vertex.position[0])
                && (20.0..=27.0).contains(&vertex.position[1])
        }));
    }

    #[test]
    fn unsupported_glyph_uses_boxed_question_fallback() {
        let mut fallback = PrimitiveBatch::default();
        fallback.text(Vec2::ZERO, 1.0, [1.0; 4], "@");
        let (triangles, remainder) = fallback.vertices().as_chunks::<6>();
        assert!(remainder.is_empty());
        let cells: Vec<(i32, i32)> = triangles
            .iter()
            .map(|cell| {
                assert!(cell.iter().all(|vertex| vertex.shape == 0));
                let x = cell
                    .iter()
                    .map(|vertex| vertex.position[0])
                    .fold(f32::INFINITY, f32::min);
                let y = cell
                    .iter()
                    .map(|vertex| vertex.position[1])
                    .fold(f32::INFINITY, f32::min);
                (x as i32, y as i32)
            })
            .collect();
        let expected = vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (0, 1),
            (4, 1),
            (0, 2),
            (4, 2),
            (0, 3),
            (4, 3),
            (0, 4),
            (4, 4),
            (0, 5),
            (4, 5),
            (0, 6),
            (1, 6),
            (2, 6),
            (3, 6),
            (4, 6),
        ];
        assert_eq!(cells, expected);
    }

    #[test]
    fn primitive_batch_rejects_non_finite_and_overflow_prone_geometry() {
        let mut batch = PrimitiveBatch::default();
        batch.quad(
            Vec2::splat(f32::MAX / 4.0),
            Vec2::splat(f32::MAX / 4.0),
            [1.0; 4],
        );
        assert_eq!(batch.vertices().len(), 6);
        assert_vertices_are_finite(batch.vertices());

        batch.clear();
        batch.quad(Vec2::ZERO, Vec2::ONE, [f32::NAN, 1.0, 1.0, 1.0]);
        batch.disc(Vec2::ZERO, f32::INFINITY, [1.0; 4]);
        batch.line(Vec2::splat(f32::MAX), Vec2::splat(-f32::MAX), 1.0, [1.0; 4]);
        assert!(batch.vertices().is_empty());
    }

    fn assert_vertices_are_finite(vertices: &[Vertex]) {
        assert!(vertices.iter().all(|vertex| {
            vertex
                .position
                .iter()
                .chain(vertex.color.iter())
                .chain(vertex.local.iter())
                .all(|component| component.is_finite())
        }));
    }
}
