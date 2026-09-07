//! The strict Living Galaxy V2 canonical JSON wire.
//!
//! Normative rules, from
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` section 10:
//! canonical documents are UTF-8 JSON with no byte order mark and no
//! insignificant whitespace, the schema-declared field order is normative,
//! every field is present, an optional value is an explicit `null` or its
//! value and is never omitted, logical maps are arrays sorted by stable
//! identifier, generic JSON maps are forbidden, and there are no
//! floating-point values. Decoders reject duplicate, unknown and reordered
//! fields, invalid Unicode and noncanonical escapes, and any input whose
//! decode and re-encode are not byte-identical.

use serde::{Serialize, de::DeserializeOwned};

/// Largest accepted Living V2 archive document.
pub const LIVING_MAX_ARCHIVE_BYTES_V2: usize = 32 * 1_024 * 1_024;
/// Largest accepted Living V2 catalog pack document.
pub const LIVING_MAX_PACK_BYTES_V2: usize = 2 * 1_024 * 1_024;
/// Deepest accepted container nesting. The outermost container is level one.
pub const LIVING_MAX_CANONICAL_DEPTH_V2: u32 = 32;

const LIVING_BYTE_ORDER_MARK: [u8; 3] = [0xef, 0xbb, 0xbf];

/// Every way a Living V2 document can fail the canonical wire contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingWireErrorV2 {
    /// The document exceeds its declared byte bound; rejected before parsing.
    #[error("Living V2 document has {actual} bytes; the limit is {limit}")]
    TooLarge {
        /// Measured document length.
        actual: usize,
        /// Declared limit for this document kind.
        limit: usize,
    },
    /// A byte order mark is valid UTF-8 but is not canonical here.
    #[error("Living V2 documents carry no byte order mark")]
    ByteOrderMark,
    /// The bytes are not valid UTF-8.
    #[error("Living V2 documents are UTF-8")]
    Utf8,
    /// Container nesting exceeds the declared bound.
    #[error("Living V2 documents nest at most {limit} levels")]
    DepthExceeded {
        /// Declared nesting limit.
        limit: u32,
    },
    /// A floating-point token appeared; V2 canonical data has none.
    #[error("Living V2 canonical data contains no floating-point values")]
    Float,
    /// The bytes are not one strictly typed JSON document of the target type.
    /// This covers malformed JSON, unknown and duplicate fields, out-of-range
    /// integers, and unsorted or duplicated keyed arrays.
    #[error("Living V2 document is not one strictly typed canonical JSON document")]
    Json,
    /// The document parses but its re-encoding differs byte for byte, so the
    /// input carried reordered fields, insignificant whitespace, a
    /// noncanonical escape, or an omitted optional value.
    #[error("Living V2 document is not in canonical byte form")]
    NonCanonical,
}

/// Decode one canonical Living V2 document of the declared type.
///
/// The pipeline is byte bound, byte order mark, UTF-8, a structural pre-scan
/// for nesting depth and floating-point tokens, strict typed deserialization,
/// and finally an exact decode/re-encode byte comparison. The pre-scan is not
/// redundant: the underlying parser's own recursion guard is far looser than
/// the declared 32-level bound.
pub fn decode_canonical_v2<T>(bytes: &[u8], limit: usize) -> Result<T, LivingWireErrorV2>
where
    T: DeserializeOwned + Serialize,
{
    if bytes.len() > limit {
        return Err(LivingWireErrorV2::TooLarge {
            actual: bytes.len(),
            limit,
        });
    }
    if bytes.starts_with(&LIVING_BYTE_ORDER_MARK) {
        return Err(LivingWireErrorV2::ByteOrderMark);
    }
    let text = str::from_utf8(bytes).map_err(|_| LivingWireErrorV2::Utf8)?;
    scan_canonical_structure(text)?;
    let value: T = serde_json::from_str(text).map_err(|_| LivingWireErrorV2::Json)?;
    let reencoded = serde_json::to_vec(&value).map_err(|_| LivingWireErrorV2::Json)?;
    if reencoded != bytes {
        return Err(LivingWireErrorV2::NonCanonical);
    }
    Ok(value)
}

/// Encode one value as canonical Living V2 bytes within its byte bound.
pub fn encode_canonical_v2<T>(value: &T, limit: usize) -> Result<Vec<u8>, LivingWireErrorV2>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(value).map_err(|_| LivingWireErrorV2::Json)?;
    if bytes.len() > limit {
        return Err(LivingWireErrorV2::TooLarge {
            actual: bytes.len(),
            limit,
        });
    }
    let text = str::from_utf8(&bytes).map_err(|_| LivingWireErrorV2::Utf8)?;
    scan_canonical_structure(text)?;
    Ok(bytes)
}

fn scan_canonical_structure(text: &str) -> Result<(), LivingWireErrorV2> {
    let bytes = text.as_bytes();
    let mut depth = 0_u32;
    let mut index = 0_usize;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => index = scan_string(bytes, index)?,
            b'{' | b'[' => {
                depth += 1;
                if depth > LIVING_MAX_CANONICAL_DEPTH_V2 {
                    return Err(LivingWireErrorV2::DepthExceeded {
                        limit: LIVING_MAX_CANONICAL_DEPTH_V2,
                    });
                }
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            b'-' | b'0'..=b'9' => index = scan_number(bytes, index)?,
            _ => index += 1,
        }
    }
    Ok(())
}

fn scan_string(bytes: &[u8], start: usize) -> Result<usize, LivingWireErrorV2> {
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Ok(index + 1),
            _ => index += 1,
        }
    }
    Err(LivingWireErrorV2::Json)
}

fn scan_number(bytes: &[u8], start: usize) -> Result<usize, LivingWireErrorV2> {
    let mut index = start;
    if bytes[index] == b'-' {
        index += 1;
    }
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if index < bytes.len() && matches!(bytes[index], b'.' | b'e' | b'E') {
        return Err(LivingWireErrorV2::Float);
    }
    Ok(index)
}

/// A record that carries the stable identifier its collection is sorted by.
pub trait LivingKeyedV2 {
    /// The stable ordering key.
    type Key: Ord;

    /// The record's stable identifier.
    fn living_key(&self) -> Self::Key;
}

/// A logical map encoded as an array sorted strictly ascending by stable
/// identifier. Decoding rejects unsorted arrays and duplicate identifiers, so
/// no insertion order can reach the authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LivingSortedVecV2<T>(Vec<T>);

impl<T> LivingSortedVecV2<T>
where
    T: LivingKeyedV2,
{
    /// Build a sorted collection, rejecting unsorted or duplicated keys.
    pub fn new(items: Vec<T>) -> Result<Self, LivingWireErrorV2> {
        if !is_strictly_sorted(&items) {
            return Err(LivingWireErrorV2::Json);
        }
        Ok(Self(items))
    }

    /// The records in stable identifier order.
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }

    /// Consume the collection, yielding the records in stable order.
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<T> Default for LivingSortedVecV2<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

fn is_strictly_sorted<T>(items: &[T]) -> bool
where
    T: LivingKeyedV2,
{
    items
        .windows(2)
        .all(|pair| pair[0].living_key() < pair[1].living_key())
}

impl<'de, T> serde::Deserialize<'de> for LivingSortedVecV2<T>
where
    T: serde::Deserialize<'de> + LivingKeyedV2,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let items = Vec::<T>::deserialize(deserializer)?;
        for pair in items.windows(2) {
            let left = pair[0].living_key();
            let right = pair[1].living_key();
            if left == right {
                return Err(serde::de::Error::custom(
                    "keyed array repeats a stable identifier",
                ));
            }
            if left > right {
                return Err(serde::de::Error::custom(
                    "keyed array is not sorted by stable identifier",
                ));
            }
        }
        Ok(Self(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_structural_pre_scan_ignores_braces_and_digits_inside_strings() {
        let text = r#"{"name":"[{ 1.5 }]"}"#;
        assert!(scan_canonical_structure(text).is_ok());
    }

    #[test]
    fn the_structural_pre_scan_rejects_an_unterminated_string() {
        assert_eq!(
            scan_canonical_structure("{\"name\":\"unterminated"),
            Err(LivingWireErrorV2::Json)
        );
    }

    #[test]
    fn negative_integers_are_canonical_and_exponents_are_not() {
        assert!(scan_canonical_structure(r#"{"a":-100}"#).is_ok());
        assert_eq!(
            scan_canonical_structure(r#"{"a":1E3}"#),
            Err(LivingWireErrorV2::Float)
        );
    }
}
