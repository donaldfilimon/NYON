use naga::valid::{Capabilities, ValidationFlags, Validator};

pub const PRIMITIVES_WGSL: &str = include_str!("../../assets/shaders/primitives.wgsl");
pub const SPACE_WGSL: &str = include_str!("../../assets/shaders/space.wgsl");
pub const WORLDS_WGSL: &str = include_str!("../../assets/shaders/worlds.wgsl");
pub const ROUTES_WGSL: &str = include_str!("../../assets/shaders/routes.wgsl");
pub const POSTPROCESS_WGSL: &str = include_str!("../../assets/shaders/postprocess.wgsl");

#[derive(Debug, thiserror::Error)]
pub enum ShaderError {
    #[error("WGSL parse failed for {label}:\n{diagnostic}")]
    Parse { label: String, diagnostic: String },
    #[error("WGSL validation failed for {label}:\n{diagnostic}")]
    Validation { label: String, diagnostic: String },
}

/// Parses and validates WGSL before it is passed to a graphics API.
pub fn validate_wgsl(label: &str, source: &str) -> Result<naga::Module, ShaderError> {
    let module = naga::front::wgsl::parse_str(source).map_err(|error| ShaderError::Parse {
        label: label.to_owned(),
        diagnostic: error.emit_to_string_with_path(source, label),
    })?;

    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .map_err(|error| ShaderError::Validation {
            label: label.to_owned(),
            diagnostic: error.emit_to_string_with_path(source, label),
        })?;

    Ok(module)
}
