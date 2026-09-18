#!/usr/bin/env rust-script
//! Validate that Rust Vertex layout matches WGSL VertexInput layout.
//!
//! The Rust struct in src/engine/primitives.rs and the WGSL struct in
//! assets/shaders/primitives.wgsl must have identical layout:
//! - Field offsets (0, 8, 24, 32 bytes)
//! - Field sizes (8, 16, 8, 4 bytes)
//! - Total stride (36 bytes)
//! - Field order and types
//!
//! This is a frozen contract per tests/rules_v1_facade.rs.

use std::path::Path;
use syn::{parse_file, ItemStruct, Fields, Type, TypeArray, TypeTuple, Ident, TypePath};
use quote::ToTokens;
use thiserror::Error;

#[derive(Debug, Error)]
enum ValidationError {
    #[error("Could not find Vertex struct in {0}")]
    VertexStructNotFound(String),
    #[error("Could not find VertexInput struct in {0}")]
    VertexInputStructNotFound(String),
    #[error("Parse error: {0}")]
    ParseError(#[from] syn::Error),
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, PartialEq)]
struct VertexField {
    name: String,
    offset: usize,
    size: usize,
    type_str: String,
}

impl std::fmt::Display for VertexField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (offset={}, size={}, type={})", self.name, self.offset, self.size, self.type_str)
    }
}

/// Type sizes for Rust/WGSL types in this context
const RUST_TYPE_SIZES: &[(&str, usize)] = &[
    ("f32", 4),
    ("u32", 4),
];

const WGSL_TYPE_SIZES: &[(&str, usize)] = &[
    ("f32", 4),
    ("u32", 4),
];

fn get_rust_type_size(ty: &Type) -> Result<usize, ValidationError> {
    match ty {
        Type::Array(TypeArray { elem, len, .. }) => {
            let elem_size = get_rust_type_size(elem)?;
            let len = match len {
                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(lit), .. }) => {
                    lit.base10_parse::<usize>().map_err(|_| ValidationError::Validation("Invalid array length".into()))?
                }
                _ => return Err(ValidationError::Validation("Unsupported array length expression".into())),
            };
            Ok(elem_size * len)
        }
        Type::Path(syn::TypePath { path, .. }) => {
            let type_name = path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            RUST_TYPE_SIZES.iter()
                .find(|(name, _)| *name == type_name)
                .map(|(_, size)| *size)
                .ok_or_else(|| ValidationError::Validation(format!("Unknown Rust type: {}", type_name)))
        }
        Type::Tuple(TypeTuple { elems, .. }) => {
            elems.iter().map(get_rust_type_size).sum::<Result<usize, _>>()
        }
        _ => Err(ValidationError::Validation(format!("Unsupported Rust type: {}", ty.to_token_stream()))),
    }
}

fn get_wgsl_type_size(type_str: &str) -> Result<usize, ValidationError> {
    // Handle vec2<f32>, vec4<f32>, u32, f32
    if type_str.starts_with("vec") {
        let parts: Vec<&str> = type_str.trim_matches(|c| c == '<' || c == '>').split('<').collect();
        if parts.len() == 2 {
            let dim = parts[0].strip_prefix("vec").unwrap_or("").parse::<usize>().unwrap_or(0);
            let elem_size = get_wgsl_type_size(parts[1])?;
            return Ok(dim * elem_size);
        }
    }
    
    WGSL_TYPE_SIZES.iter()
        .find(|(name, _)| *name == type_str)
        .map(|(_, size)| *size)
        .ok_or_else(|| ValidationError::Validation(format!("Unknown WGSL type: {}", type_str)))
}

fn parse_rust_vertex(content: &str) -> Result<(Vec<VertexField>, usize), ValidationError> {
    let ast = parse_file(content)?;
    
    for item in ast.items {
        if let syn::Item::Struct(ItemStruct { ident, fields, .. }) = item {
            if ident == "Vertex" {
                let fields = match fields {
                    Fields::Named(named) => named.named,
                    _ => return Err(ValidationError::Validation("Vertex struct must have named fields".into())),
                };
                
                let mut fields_vec = Vec::new();
                let mut current_offset = 0;
                
                for field in fields {
                    let name = field.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                    let size = get_rust_type_size(&field.ty)?;
                    fields_vec.push(VertexField {
                        name,
                        offset: current_offset,
                        size,
                        type_str: field.ty.to_token_stream().to_string().replace(' ', ""),
                    });
                    current_offset += size;
                }
                
                // Stride is the total size (36 for Vertex)
                let stride = current_offset;
                return Ok((fields_vec, stride));
            }
        }
    }
    
    Err(ValidationError::VertexStructNotFound("src/engine/primitives.rs".into()))
}

fn parse_wgsl_vertex(content: &str) -> Result<Vec<VertexField>, ValidationError> {
    // Simple regex-based parsing for WGSL - WGSL is simpler than Rust
    let struct_regex = regex::Regex::new(r"struct\s+VertexInput\s*\{([^}]+)\}")?;
    let field_regex = regex::Regex::new(r"@location\(\d+\)\s+(\w+)\s*:\s*([^,\n]+)")?;
    
    let struct_match = struct_regex.captures(content)
        .ok_or_else(|| ValidationError::VertexInputStructNotFound("assets/shaders/primitives.wgsl".into()))?;
    
    let struct_body = &struct_match[1];
    let mut fields = Vec::new();
    let mut current_offset = 0;
    
    for cap in field_regex.captures_iter(struct_body) {
        let name = cap[1].to_string();
        let type_str = cap[2].trim().to_string();
        let size = get_wgsl_type_size(&type_str)?;
        
        fields.push(VertexField {
            name,
            offset: current_offset,
            size,
            type_str,
        });
        current_offset += size;
    }
    
    Ok(fields)
}

fn compare_layouts(rust_fields: &[VertexField], wgsl_fields: &[VertexField], rust_stride: usize) -> Result<(), ValidationError> {
    if rust_fields.len() != wgsl_fields.len() {
        return Err(ValidationError::Validation(format!(
            "Field count mismatch: Rust has {}, WGSL has {}",
            rust_fields.len(), wgsl_fields.len()
        )));
    }
    
    for (rf, wf) in rust_fields.iter().zip(wgsl_fields.iter()) {
        if rf.name != wf.name {
            return Err(ValidationError::Validation(format!(
                "Field name mismatch: Rust '{}' vs WGSL '{}'", rf.name, wf.name
            )));
        }
        if rf.offset != wf.offset {
            return Err(ValidationError::Validation(format!(
                "Field '{}' offset mismatch: Rust {} vs WGSL {}", rf.name, rf.offset, wf.offset
            )));
        }
        if rf.size != wf.size {
            return Err(ValidationError::Validation(format!(
                "Field '{}' size mismatch: Rust {} vs WGSL {}", rf.name, rf.size, wf.size
            )));
        }
    }
    
    let wgsl_stride: usize = wgsl_fields.iter().map(|f| f.size).sum();
    if rust_stride != wgsl_stride {
        return Err(ValidationError::Validation(format!(
            "Stride mismatch: Rust {} vs WGSL {}", rust_stride, wgsl_stride
        )));
    }
    
    Ok(())
}

fn main() -> Result<(), ValidationError> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let rust_file = repo_root.join("src/engine/primitives.rs");
    let wgsl_file = repo_root.join("assets/shaders/primitives.wgsl");
    
    let rust_content = std::fs::read_to_string(&rust_file)?;
    let wgsl_content = std::fs::read_to_string(&wgsl_file)?;
    
    let (rust_fields, rust_stride) = parse_rust_vertex(&rust_content)?;
    let wgsl_fields = parse_wgsl_vertex(&wgsl_content)?;
    
    println!("Rust Vertex layout:");
    for f in &rust_fields {
        println!("  {}: offset={}, size={}, type={}", f.name, f.offset, f.size, f.type_str);
    }
    println!("  Stride: {}", rust_stride);
    
    println!("\nWGSL VertexInput layout:");
    for f in &wgsl_fields {
        println!("  {}: offset={}, size={}, type={}", f.name, f.offset, f.size, f.type_str);
    }
    let wgsl_stride: usize = wgsl_fields.iter().map(|f| f.size).sum();
    println!("  Stride: {}", wgsl_stride);
    
    compare_layouts(&rust_fields, &wgsl_fields, rust_stride)?;
    
    println!("\nVALIDATION PASSED: Rust and WGSL vertex layouts match exactly.");
    Ok(())
}