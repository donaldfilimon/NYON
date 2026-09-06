use serde::{Deserialize, Serialize};

pub const GALAXY_COORDINATE_LIMIT: i64 = 131_072 * 1_024;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct WorkshopTick(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RevisionId(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntityId(pub [u8; 16]);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BranchId(pub [u8; 16]);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CatalogHash(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateDigest(pub [u8; 32]);

macro_rules! fixed_hex_id {
    ($type:ident, $length:expr) => {
        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut output = String::with_capacity($length * 2);
                const HEX: &[u8; 16] = b"0123456789abcdef";
                for byte in self.0 {
                    output.push(char::from(HEX[usize::from(byte >> 4)]));
                    output.push(char::from(HEX[usize::from(byte & 0x0f)]));
                }
                serializer.serialize_str(&output)
            }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let input = String::deserialize(deserializer)?;
                if input.len() != $length * 2
                    || !input
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(serde::de::Error::custom("invalid canonical identifier"));
                }
                let mut bytes = [0_u8; $length];
                for (index, pair) in input.as_bytes().chunks_exact(2).enumerate() {
                    bytes[index] = (hex_nibble(pair[0])
                        .ok_or_else(|| serde::de::Error::custom("invalid canonical identifier"))?
                        << 4)
                        | hex_nibble(pair[1]).ok_or_else(|| {
                            serde::de::Error::custom("invalid canonical identifier")
                        })?;
                }
                Ok(Self(bytes))
            }
        }
    };
}

fixed_hex_id!(RevisionId, 32);
fixed_hex_id!(EntityId, 16);
fixed_hex_id!(BranchId, 16);
fixed_hex_id!(CatalogHash, 32);
fixed_hex_id!(StateDigest, 32);

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct BatchLocalId(pub u16);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CatalogId(Box<str>);

impl CatalogId {
    pub fn new(value: impl AsRef<str>) -> Result<Self, IdentifierError> {
        let value = value.as_ref();
        if is_catalog_id(value) {
            Ok(Self(value.into()))
        } else {
            Err(IdentifierError::CatalogId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CatalogId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for CatalogId {
    type Error = IdentifierError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for CatalogId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ObjectName(Box<str>);

impl ObjectName {
    pub fn new(value: impl AsRef<str>) -> Result<Self, IdentifierError> {
        let value = value.as_ref();
        if is_name(value) {
            Ok(Self(value.into()))
        } else {
            Err(IdentifierError::ObjectName)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ObjectName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for ObjectName {
    type Error = IdentifierError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for ObjectName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct GalaxyCoord(i64);

impl GalaxyCoord {
    pub fn new(value: i64) -> Result<Self, IdentifierError> {
        if (-GALAXY_COORDINATE_LIMIT..=GALAXY_COORDINATE_LIMIT).contains(&value) {
            Ok(Self(value))
        } else {
            Err(IdentifierError::CoordinateOutOfRange)
        }
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for GalaxyCoord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GalaxyPointV1 {
    pub x: GalaxyCoord,
    pub y: GalaxyCoord,
}

impl GalaxyPointV1 {
    pub fn new(x: i64, y: i64) -> Result<Self, IdentifierError> {
        Ok(Self {
            x: GalaxyCoord::new(x)?,
            y: GalaxyCoord::new(y)?,
        })
    }

    pub fn distance_units(self, other: Self) -> Result<u64, IdentifierError> {
        let dx = i128::from(self.x.get())
            .checked_sub(i128::from(other.x.get()))
            .ok_or(IdentifierError::DistanceOverflow)?;
        let dy = i128::from(self.y.get())
            .checked_sub(i128::from(other.y.get()))
            .ok_or(IdentifierError::DistanceOverflow)?;
        let square = dx
            .checked_mul(dx)
            .and_then(|x| dy.checked_mul(dy).and_then(|y| x.checked_add(y)))
            .ok_or(IdentifierError::DistanceOverflow)?;
        let root = integer_sqrt(square as u128);
        u64::try_from(root).map_err(|_| IdentifierError::DistanceOverflow)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ObjectRefV1 {
    Existing(EntityId),
    Local(BatchLocalId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentifierError {
    #[error("catalog identifier is invalid")]
    CatalogId,
    #[error("object name is invalid")]
    ObjectName,
    #[error("galaxy coordinate is outside the supported range")]
    CoordinateOutOfRange,
    #[error("lane distance cannot be represented")]
    DistanceOverflow,
}

pub(crate) fn is_pack_id(value: &str) -> bool {
    (1..=48).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

pub(crate) fn is_catalog_id(value: &str) -> bool {
    (1..=32).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

pub(crate) fn is_name(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
        && !value.starts_with(' ')
        && !value.ends_with(' ')
        && !value.contains("  ")
}

pub(crate) fn is_description(value: &str) -> bool {
    value.len() <= 512 && value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

fn integer_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut low = 1_u128;
    let mut high = value.min(u128::from(u64::MAX) + 1);
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_bounds_and_integer_distance_are_exact() {
        assert!(GalaxyCoord::new(GALAXY_COORDINATE_LIMIT).is_ok());
        assert!(GalaxyCoord::new(-GALAXY_COORDINATE_LIMIT).is_ok());
        assert!(GalaxyCoord::new(GALAXY_COORDINATE_LIMIT + 1).is_err());
        assert_eq!(
            GalaxyPointV1::new(0, 0)
                .unwrap()
                .distance_units(GalaxyPointV1::new(3, 4).unwrap())
                .unwrap(),
            5
        );
    }
}
