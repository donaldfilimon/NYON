"""Behavioral checks for the frozen Vertex source validator."""

import pathlib
import shutil
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools/validate-vertex-layout.py"
RUST = (ROOT / "src/engine/primitives.rs").read_text()
WGSL = (ROOT / "assets/shaders/primitives.wgsl").read_text()


def replace_once(source: str, old: str, new: str) -> str:
    if source.count(old) != 1:
        raise AssertionError(f"Expected one fixture anchor {old!r}, found {source.count(old)}")
    return source.replace(old, new, 1)


class VertexLayoutTests(unittest.TestCase):
    def check_sources(self, rust: str = RUST, wgsl: str = WGSL) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "tools").mkdir()
            (root / "src/engine").mkdir(parents=True)
            (root / "assets/shaders").mkdir(parents=True)
            shutil.copyfile(SCRIPT, root / "tools/validate-vertex-layout.py")
            (root / "src/engine/primitives.rs").write_text(rust)
            (root / "assets/shaders/primitives.wgsl").write_text(wgsl)
            return subprocess.run(
                ["python3", str(root / "tools/validate-vertex-layout.py")],
                capture_output=True, text=True, check=False,
            )

    def assert_rejected(self, result: subprocess.CompletedProcess[str], diagnostic: str) -> None:
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(diagnostic, result.stdout)

    def test_current_frozen_source_passes(self) -> None:
        result = self.check_sources()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("VALIDATION PASSED", result.stdout)

    def test_wrong_shader_location_rejects(self) -> None:
        shader = replace_once(WGSL, "@location(1) color", "@location(9) color")
        self.assert_rejected(self.check_sources(wgsl=shader), "Missing VertexInput location 1")

    def test_duplicate_shader_location_rejects(self) -> None:
        shader = replace_once(WGSL, "@location(1) color", "@location(0) color")
        self.assert_rejected(self.check_sources(wgsl=shader), "Duplicate VertexInput location 0")

    def test_shader_type_rejects_even_with_correct_location(self) -> None:
        shader = replace_once(WGSL, "@location(1) color: vec4<f32>", "@location(1) color: vec2<f32>")
        self.assert_rejected(self.check_sources(wgsl=shader), "VertexInput location 1")

    def test_shader_declaration_order_does_not_define_location(self) -> None:
        shader = replace_once(
            WGSL,
            "    @location(0) position: vec2<f32>,\n    @location(1) color: vec4<f32>,",
            "    @location(1) color: vec4<f32>,\n    @location(0) position: vec2<f32>,",
        )
        result = self.check_sources(wgsl=shader)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_wrong_attribute_format_rejects(self) -> None:
        rust = replace_once(RUST, "3 => Uint32,", "3 => Float32x2,")
        self.assert_rejected(self.check_sources(rust=rust), "Attribute location 3")

    def test_wrong_attribute_location_rejects(self) -> None:
        rust = replace_once(RUST, "1 => Float32x4,", "9 => Float32x4,")
        self.assert_rejected(self.check_sources(rust=rust), "Missing attribute location 1")

    def test_duplicate_attribute_location_rejects(self) -> None:
        rust = replace_once(RUST, "1 => Float32x4,", "0 => Float32x4,")
        self.assert_rejected(self.check_sources(rust=rust), "Duplicate vertex attribute location 0")

    def test_field_type_and_computed_stride_reject(self) -> None:
        rust = replace_once(RUST, "pub color: [f32; 4],", "pub color: [f32; 2],")
        result = self.check_sources(rust=rust)
        self.assert_rejected(result, "Field 1")
        self.assertIn("Vertex sizeof: expected 36, got 28", result.stdout)

    def test_literal_descriptor_stride_rejects(self) -> None:
        rust = replace_once(
            RUST,
            "array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,",
            "array_stride: 40 as wgpu::BufferAddress,",
        )
        self.assert_rejected(self.check_sources(rust=rust), "LAYOUT array_stride: expected Vertex sizeof 36, got 40")

    def test_overaligned_repr_rejects(self) -> None:
        rust = replace_once(RUST, "#[repr(C)]", "#[repr(C, align(16))]")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported Vertex representation")

    def test_unsupported_field_declaration_rejects(self) -> None:
        rust = replace_once(RUST, "pub shape: u32,", "shape: u32,")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported Vertex field entry")

    def test_ambiguous_vertex_declaration_rejects(self) -> None:
        rust = RUST + "\npub struct Vertex { pub shape: u32, }\n"
        self.assert_rejected(self.check_sources(rust=rust), "Expected exactly one Vertex declaration")

    def test_commented_out_shader_declaration_cannot_substitute_for_real_one(self) -> None:
        shader = replace_once(WGSL, "struct VertexInput {", "struct Gone {")
        shader += "\n/* struct VertexInput { @location(0) position: vec2<f32>, } */\n"
        self.assert_rejected(self.check_sources(wgsl=shader), "Expected exactly one VertexInput declaration")

    def test_attribute_source_must_be_actual_macro(self) -> None:
        rust = replace_once(RUST, "wgpu::vertex_attr_array![", "other_macro![")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported ATTRIBUTES declaration")

    def test_attributes_must_belong_to_vertex_impl(self) -> None:
        rust = replace_once(RUST, "impl Vertex {", "impl Other {")
        self.assert_rejected(self.check_sources(rust=rust), "Expected exactly one impl Vertex block")

    def test_conditional_layout_declaration_rejects(self) -> None:
        rust = replace_once(RUST, "    pub const LAYOUT:", "    #[cfg(any())]\n    pub const LAYOUT:")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported LAYOUT declaration attribute")

    def test_conditional_attribute_macro_declaration_rejects(self) -> None:
        rust = replace_once(RUST, "    pub const ATTRIBUTES:", "    #[cfg(any())]\n    pub const ATTRIBUTES:")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported ATTRIBUTES declaration attribute")

    def test_conditional_impl_declaration_rejects(self) -> None:
        rust = replace_once(RUST, "impl Vertex {", "#[cfg(any())]\nimpl Vertex {")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported impl Vertex attribute")

    def test_conditional_struct_declaration_rejects_across_blank_line(self) -> None:
        rust = replace_once(RUST, "#[repr(C)]", "#[cfg(any())]\n\n#[repr(C)]")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported Vertex representation")

    def test_spaced_conditional_layout_declaration_rejects(self) -> None:
        rust = replace_once(RUST, "    pub const LAYOUT:", "    # [cfg(any())]\n    pub const LAYOUT:")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported LAYOUT declaration attribute")

    def test_spaced_conditional_attribute_macro_declaration_rejects(self) -> None:
        rust = replace_once(RUST, "    pub const ATTRIBUTES:", "    # [cfg(any())]\n    pub const ATTRIBUTES:")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported ATTRIBUTES declaration attribute")

    def test_spaced_conditional_impl_declaration_rejects(self) -> None:
        rust = replace_once(RUST, "impl Vertex {", "# [cfg(any())]\nimpl Vertex {")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported impl Vertex attribute")

    def test_comment_between_attribute_tokens_rejects(self) -> None:
        rust = replace_once(RUST, "    pub const LAYOUT:", "    # /* comment */ [cfg(any())]\n    pub const LAYOUT:")
        self.assert_rejected(self.check_sources(rust=rust), "Unsupported LAYOUT declaration attribute")

if __name__ == "__main__":
    unittest.main()
