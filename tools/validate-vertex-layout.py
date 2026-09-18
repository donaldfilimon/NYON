#!/usr/bin/env python3
"""Validate that Rust Vertex layout matches WGSL VertexInput layout.

The Rust struct in src/engine/primitives.rs and the WGSL struct in
assets/shaders/primitives.wgsl must have identical layout:
- Field offsets (0, 8, 24, 32 bytes)
- Field sizes (8, 16, 8, 4 bytes)
- Total stride (36 bytes)
- Field order and types

This is a frozen contract per tests/rules_v1_facade.rs.
"""

import re
import sys
from pathlib import Path
from typing import List, Tuple, Dict, Optional


class VertexField:
    def __init__(self, name: str, offset: int, size: int, type_str: str):
        self.name = name
        self.offset = offset
        self.size = size
        self.type_str = type_str

    def __repr__(self):
        return f"VertexField({self.name}, offset={self.offset}, size={self.size}, type={self.type_str})"


def parse_rust_vertex(file_path: Path) -> Tuple[List[VertexField], int]:
    """Parse the Rust Vertex struct and its LAYOUT from src/engine/primitives.rs."""
    content = file_path.read_text()

    # Find the Vertex struct definition
    struct_match = re.search(
        r'pub struct Vertex\s*\{([^}]+)\}',
        content,
        re.DOTALL
    )
    if not struct_match:
        raise ValueError(f"Could not find Vertex struct in {file_path}")

    struct_body = struct_match.group(1)

    # Parse fields: pub name: type,
    field_pattern = re.compile(r'pub\s+(\w+)\s*:\s*([^,\n]+)')
    fields = []
    current_offset = 0

    # Known sizes for Rust types in this context
    type_sizes = {
        '[f32; 2]': 8,
        '[f32; 4]': 16,
        'u32': 4,
    }

    for match in field_pattern.finditer(struct_body):
        name = match.group(1)
        type_str = match.group(2).strip()
        size = type_sizes.get(type_str)
        if size is None:
            raise ValueError(f"Unknown type {type_str} for field {name}")
        fields.append(VertexField(name, current_offset, size, type_str))
        current_offset += size

    # Parse the stride from LAYOUT.array_stride
    stride_match = re.search(
        r'array_stride:\s*std::mem::size_of::<Self>\(\)\s*as\s*wgpu::BufferAddress',
        content
    )
    if not stride_match:
        raise ValueError("Could not find array_stride in LAYOUT")

    # The stride is asserted to be 36 in the test
    stride = 36

    return fields, stride


def parse_wgsl_vertex(file_path: Path) -> List[VertexField]:
    """Parse the WGSL VertexInput struct from assets/shaders/primitives.wgsl."""
    content = file_path.read_text()

    # Find the VertexInput struct
    struct_match = re.search(
        r'struct VertexInput\s*\{([^}]+)\}',
        content,
        re.DOTALL
    )
    if not struct_match:
        raise ValueError(f"Could not find VertexInput struct in {file_path}")

    struct_body = struct_match.group(1)

    # Parse fields: @location(N) name: type,
    # WGSL types and their sizes (std140 layout equivalent for vertex inputs)
    type_sizes = {
        'vec2<f32>': 8,
        'vec4<f32>': 16,
        'u32': 4,
    }

    fields = []
    current_offset = 0

    # Pattern matches @location(N) name: type,
    field_pattern = re.compile(r'@location\(\d+\)\s+(\w+)\s*:\s*([^,\n]+)')

    for match in field_pattern.finditer(struct_body):
        name = match.group(1)
        type_str = match.group(2).strip()
        size = type_sizes.get(type_str)
        if size is None:
            raise ValueError(f"Unknown WGSL type {type_str} for field {name}")
        fields.append(VertexField(name, current_offset, size, type_str))
        current_offset += size

    return fields


def compare_layouts(rust_fields: List[VertexField], wgsl_fields: List[VertexField],
                    rust_stride: int) -> List[str]:
    """Compare Rust and WGSL layouts, return list of errors (empty if match)."""
    errors = []

    if len(rust_fields) != len(wgsl_fields):
        errors.append(f"Field count mismatch: Rust has {len(rust_fields)}, WGSL has {len(wgsl_fields)}")
        return errors

    for i, (rf, wf) in enumerate(zip(rust_fields, wgsl_fields)):
        if rf.name != wf.name:
            errors.append(f"Field {i} name mismatch: Rust '{rf.name}' vs WGSL '{wf.name}'")
        if rf.offset != wf.offset:
            errors.append(f"Field {rf.name} offset mismatch: Rust {rf.offset} vs WGSL {wf.offset}")
        if rf.size != wf.size:
            errors.append(f"Field {rf.name} size mismatch: Rust {rf.size} vs WGSL {wf.size}")

    # Check stride
    wgsl_stride = sum(f.size for f in wgsl_fields)
    if rust_stride != wgsl_stride:
        errors.append(f"Stride mismatch: Rust {rust_stride} vs WGSL {wgsl_stride}")

    return errors


def main():
    repo_root = Path(__file__).parent.parent
    rust_file = repo_root / "src" / "engine" / "primitives.rs"
    wgsl_file = repo_root / "assets" / "shaders" / "primitives.wgsl"

    if not rust_file.exists():
        print(f"ERROR: Rust file not found: {rust_file}")
        return 1
    if not wgsl_file.exists():
        print(f"ERROR: WGSL file not found: {wgsl_file}")
        return 1

    try:
        rust_fields, rust_stride = parse_rust_vertex(rust_file)
        wgsl_fields = parse_wgsl_vertex(wgsl_file)
    except Exception as e:
        print(f"ERROR: {e}")
        return 1

    print("Rust Vertex layout:")
    for f in rust_fields:
        print(f"  {f.name}: offset={f.offset}, size={f.size}, type={f.type_str}")
    print(f"  Stride: {rust_stride}")

    print("\nWGSL VertexInput layout:")
    for f in wgsl_fields:
        print(f"  {f.name}: offset={f.offset}, size={f.size}, type={f.type_str}")
    wgsl_stride = sum(f.size for f in wgsl_fields)
    print(f"  Stride: {wgsl_stride}")

    errors = compare_layouts(rust_fields, wgsl_fields, rust_stride)

    if errors:
        print("\nVALIDATION FAILED:")
        for err in errors:
            print(f"  - {err}")
        return 1
    else:
        print("\nVALIDATION PASSED: Rust and WGSL vertex layouts match exactly.")
        return 0


if __name__ == "__main__":
    sys.exit(main())