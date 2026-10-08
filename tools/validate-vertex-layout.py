#!/usr/bin/env python3
"""Check the frozen CPU vertex, wgpu attribute, and WGSL input contract.

Only the declarations used by primitives.rs and primitives.wgsl are supported.
Unrecognized syntax fails closed rather than being silently skipped.
"""

import re
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class VertexField:
    name: str
    offset: int
    size: int
    type_str: str


@dataclass(frozen=True)
class VertexAttribute:
    location: int
    offset: int
    size: int
    format: str


@dataclass(frozen=True)
class ShaderInput:
    location: int
    name: str
    type_str: str


# Every supported repr(C) field has 4-byte alignment.
RUST_TYPES = {"[f32;2]": 8, "[f32;4]": 16, "u32": 4}
ATTRIBUTE_FORMATS = {"Float32x2": 8, "Float32x4": 16, "Uint32": 4}
FROZEN = (
    ("position", "[f32;2]", 0, 0, "Float32x2", "vec2<f32>"),
    ("color", "[f32;4]", 8, 1, "Float32x4", "vec4<f32>"),
    ("local", "[f32;2]", 24, 2, "Float32x2", "vec2<f32>"),
    ("shape", "u32", 32, 3, "Uint32", "u32"),
)


def source_without_comments(content: str) -> str:
    """Blank C-style comments and quoted strings while retaining source positions."""
    result = list(content)
    index = 0
    while index < len(content):
        if content.startswith("//", index):
            end = content.find("\n", index)
            end = len(content) if end < 0 else end
            result[index:end] = " " * (end - index)
            index = end
        elif content.startswith("/*", index):
            start = index
            depth = 1
            index += 2
            while index < len(content) and depth:
                if content.startswith("/*", index):
                    depth += 1
                    index += 2
                elif content.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                raise ValueError("Unclosed source comment")
            result[start:index] = ["\n" if char == "\n" else " " for char in content[start:index]]
        elif content[index] == '"':
            start = index
            index += 1
            while index < len(content):
                if content[index] == "\\":
                    index += 2
                elif content[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError("Unclosed source string")
            result[start:index] = ["\n" if char == "\n" else " " for char in content[start:index]]
        else:
            index += 1
    return "".join(result)


def unique_declaration(content: str, name: str, pattern: str) -> tuple[re.Match[str], int]:
    occurrences = list(re.finditer(r"\b(?:pub\s+)?(?:struct|const)\s+" + name + r"\b", content))
    if len(occurrences) != 1:
        raise ValueError(f"Expected exactly one {name} declaration, found {len(occurrences)}")
    start = occurrences[0].start()
    match = re.match(pattern, content[start:], re.DOTALL)
    if match is None:
        raise ValueError(f"Unsupported {name} declaration")
    return match, start


def declaration_attributes(content: str, start: int) -> list[str]:
    """Read attributes attached to a declaration, across optional whitespace."""
    attributes = []
    cursor = start
    while True:
        while cursor and content[cursor - 1].isspace():
            cursor -= 1
        if not cursor or content[cursor - 1] != "]":
            break
        end = cursor
        depth = 0
        while cursor:
            cursor -= 1
            if content[cursor] == "]":
                depth += 1
            elif content[cursor] == "[":
                depth -= 1
                if depth == 0:
                    break
        if depth:
            break
        while cursor and content[cursor - 1].isspace():
            cursor -= 1
        if not cursor or content[cursor - 1] != "#":
            break
        cursor -= 1
        attributes.append(content[cursor:end].strip())
    return attributes[::-1]


def parse_entries(body: str, pattern: str, label: str) -> list[tuple[str, ...]]:
    """Consume every byte of a comma-terminated, whitespace-separated list."""
    entries = []
    cursor = 0
    regex = re.compile(pattern, re.DOTALL)
    while cursor < len(body):
        match = regex.match(body, cursor)
        if match is None:
            if body[cursor:].strip():
                raise ValueError(f"Unsupported {label} entry near {body[cursor:cursor + 60]!r}")
            break
        entries.append(match.groups())
        cursor = match.end()
    if not entries:
        raise ValueError(f"No {label} entries")
    return entries


def vertex_impl_body(content: str) -> str:
    starts = list(re.finditer(r"\bimpl\s+Vertex\s*\{", content))
    if len(starts) != 1:
        raise ValueError(f"Expected exactly one impl Vertex block, found {len(starts)}")
    if declaration_attributes(content, starts[0].start()):
        raise ValueError("Unsupported impl Vertex attribute")
    start = starts[0].end()
    depth = 1
    for index in range(start, len(content)):
        if content[index] == "{":
            depth += 1
        elif content[index] == "}":
            depth -= 1
            if depth == 0:
                return content[start:index]
    raise ValueError("Unclosed impl Vertex block")


def parse_rust_vertex(file_path: Path) -> tuple[list[VertexField], int, list[VertexAttribute], int]:
    content = source_without_comments(file_path.read_text())
    struct, start = unique_declaration(content, "Vertex", r"pub\s+struct\s+Vertex\s*\{([^{}]*)\}")
    annotations = declaration_attributes(content, start)
    reprs = [item for item in annotations if re.match(r"#\[\s*repr\b", item)]
    if reprs != ["#[repr(C)]"] or any(
        not re.fullmatch(r"#\[\s*derive\([^\n]*\)\s*\]", item) and item != "#[repr(C)]"
        for item in annotations
    ):
        raise ValueError("Unsupported Vertex representation or struct attribute; expected #[repr(C)]")

    raw_fields = parse_entries(
        struct.group(1), r"\s*pub\s+(\w+)\s*:\s*(\[f32;\s*\d+\]|\w+)\s*,",
        "Vertex field",
    )
    fields = []
    end = 0
    for name, raw_type in raw_fields:
        type_str = re.sub(r"\s+", "", raw_type)
        if type_str not in RUST_TYPES:
            raise ValueError(f"Unsupported Rust type {raw_type} for field {name}")
        if any(field.name == name for field in fields):
            raise ValueError(f"Duplicate Vertex field {name}")
        offset = (end + 3) // 4 * 4
        size = RUST_TYPES[type_str]
        fields.append(VertexField(name, offset, size, type_str))
        end = offset + size
    struct_stride = (end + 3) // 4 * 4

    impl_body = vertex_impl_body(content)
    macro, macro_start = unique_declaration(
        impl_body, "ATTRIBUTES",
        r"pub\s+const\s+ATTRIBUTES\s*:\s*\[\s*wgpu::VertexAttribute\s*;\s*(\d+)\s*\]"
        r"\s*=\s*wgpu::vertex_attr_array!\s*\[([^\]]*)\]\s*;",
    )
    if declaration_attributes(impl_body, macro_start):
        raise ValueError("Unsupported ATTRIBUTES declaration attribute")
    raw_attributes = parse_entries(macro.group(2), r"\s*(\d+)\s*=>\s*(\w+)\s*,", "ATTRIBUTES")
    if int(macro.group(1)) != len(raw_attributes):
        raise ValueError("ATTRIBUTES array length differs from macro entries")
    vertex_attributes = []
    offset = 0
    for raw_location, format_str in raw_attributes:
        if format_str not in ATTRIBUTE_FORMATS:
            raise ValueError(f"Unsupported vertex format {format_str}")
        location = int(raw_location)
        if any(item.location == location for item in vertex_attributes):
            raise ValueError(f"Duplicate vertex attribute location {location}")
        size = ATTRIBUTE_FORMATS[format_str]
        vertex_attributes.append(VertexAttribute(location, offset, size, format_str))
        offset += size

    layout, layout_start = unique_declaration(
        impl_body, "LAYOUT",
        r"pub\s+const\s+LAYOUT\s*:\s*wgpu::VertexBufferLayout\s*<\s*'static\s*>"
        r"\s*=\s*wgpu::VertexBufferLayout\s*\{([^{}]*)\}\s*;",
    )
    if declaration_attributes(impl_body, layout_start):
        raise ValueError("Unsupported LAYOUT declaration attribute")
    values = parse_entries(layout.group(1), r"\s*(\w+)\s*:\s*([^,]+)\s*,", "LAYOUT")
    if len(values) != 3 or {key for key, _ in values} != {"array_stride", "step_mode", "attributes"}:
        raise ValueError("LAYOUT must declare array_stride, step_mode, and attributes exactly once")
    descriptor = {key: value.strip() for key, value in values}
    if descriptor["step_mode"] != "wgpu::VertexStepMode::Vertex" or descriptor["attributes"] != "&Self::ATTRIBUTES":
        raise ValueError("Unsupported LAYOUT step_mode or attributes expression")
    stride_expr = descriptor["array_stride"]
    if re.fullmatch(r"std::mem::size_of::<Self>\(\)\s+as\s+wgpu::BufferAddress", stride_expr):
        descriptor_stride = struct_stride
    elif re.fullmatch(r"\d+(?:\s+as\s+wgpu::BufferAddress)?", stride_expr):
        descriptor_stride = int(stride_expr.split()[0])
    else:
        raise ValueError(f"Unsupported LAYOUT array_stride expression {stride_expr!r}")
    return fields, struct_stride, vertex_attributes, descriptor_stride


def parse_wgsl_vertex(file_path: Path) -> list[ShaderInput]:
    content = source_without_comments(file_path.read_text())
    vertex, _ = unique_declaration(content, "VertexInput", r"struct\s+VertexInput\s*\{([^{}]*)\}")
    raw_inputs = parse_entries(
        vertex.group(1),
        r"\s*@location\(\s*(\d+)\s*\)\s+(\w+)\s*:\s*([^,\n]+)\s*,",
        "VertexInput",
    )
    inputs = []
    for raw_location, name, raw_type in raw_inputs:
        location = int(raw_location)
        type_str = re.sub(r"\s+", "", raw_type)
        if type_str not in {"vec2<f32>", "vec4<f32>", "u32"}:
            raise ValueError(f"Unsupported WGSL type {raw_type} for {name}")
        if any(item.location == location for item in inputs):
            raise ValueError(f"Duplicate VertexInput location {location}")
        if any(item.name == name for item in inputs):
            raise ValueError(f"Duplicate VertexInput field {name}")
        inputs.append(ShaderInput(location, name, type_str))
    return inputs


def compare_layouts(
    fields: list[VertexField], struct_stride: int,
    attributes: list[VertexAttribute], descriptor_stride: int,
    inputs: list[ShaderInput],
) -> list[str]:
    errors = []
    if len(fields) != len(FROZEN):
        errors.append(f"Vertex field count: expected {len(FROZEN)}, got {len(fields)}")
    if len(attributes) != len(FROZEN):
        errors.append(f"Attribute count: expected {len(FROZEN)}, got {len(attributes)}")
    if len(inputs) != len(FROZEN):
        errors.append(f"VertexInput count: expected {len(FROZEN)}, got {len(inputs)}")
    for index, expected in enumerate(FROZEN):
        name, type_str, offset, location, format_str, shader_type = expected
        if index < len(fields):
            field = fields[index]
            if (field.name, field.type_str, field.offset) != (name, type_str, offset):
                errors.append(f"Field {index}: expected {name} {type_str} offset {offset}, got {field.name} {field.type_str} offset {field.offset}")
        matching_attribute = next((item for item in attributes if item.location == location), None)
        if matching_attribute is None:
            errors.append(f"Missing attribute location {location} ({name})")
        elif (matching_attribute.format, matching_attribute.offset) != (format_str, offset):
            errors.append(f"Attribute location {location}: expected {format_str} offset {offset}, got {matching_attribute.format} offset {matching_attribute.offset}")
        matching_input = next((item for item in inputs if item.location == location), None)
        if matching_input is None:
            errors.append(f"Missing VertexInput location {location} ({name})")
        elif (matching_input.name, matching_input.type_str) != (name, shader_type):
            errors.append(f"VertexInput location {location}: expected {name} {shader_type}, got {matching_input.name} {matching_input.type_str}")
    if struct_stride != 36:
        errors.append(f"Vertex sizeof: expected 36, got {struct_stride}")
    if descriptor_stride != struct_stride:
        errors.append(f"LAYOUT array_stride: expected Vertex sizeof {struct_stride}, got {descriptor_stride}")
    if attributes and attributes[-1].offset + attributes[-1].size != descriptor_stride:
        errors.append(f"Attribute extent {attributes[-1].offset + attributes[-1].size} differs from LAYOUT stride {descriptor_stride}")
    return errors


def main() -> int:
    repo_root = Path(__file__).parent.parent
    rust_file = repo_root / "src/engine/primitives.rs"
    wgsl_file = repo_root / "assets/shaders/primitives.wgsl"
    try:
        fields, struct_stride, attributes, descriptor_stride = parse_rust_vertex(rust_file)
        inputs = parse_wgsl_vertex(wgsl_file)
        errors = compare_layouts(fields, struct_stride, attributes, descriptor_stride, inputs)
    except (OSError, ValueError) as error:
        print(f"ERROR: {error}")
        return 1
    print("Rust Vertex layout:")
    for field in fields:
        print(f"  {field.name}: offset={field.offset}, size={field.size}, type={field.type_str}")
    print(f"  sizeof: {struct_stride}; LAYOUT stride: {descriptor_stride}")
    print("Attributes:")
    for attribute in attributes:
        print(f"  location={attribute.location}, offset={attribute.offset}, format={attribute.format}")
    print("WGSL VertexInput:")
    for item in inputs:
        print(f"  location={item.location}, name={item.name}, type={item.type_str}")
    if errors:
        print("VALIDATION FAILED:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print("VALIDATION PASSED: frozen Vertex, attributes, and shader inputs match.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
