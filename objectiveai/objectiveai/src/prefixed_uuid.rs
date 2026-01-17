//! Prefixed UUID type for ObjectiveAI identifiers.
//!
//! This module provides a generic UUID type with a 3-character prefix,
//! used throughout the ObjectiveAI API for type-safe identifiers.
//! For example, API keys use the prefix "apk" (e.g., `apk1234...`).

use std::str::FromStr;

/// A UUID with a 3-character prefix for type-safe identifiers.
///
/// This struct wraps a standard UUID and adds a compile-time prefix,
/// ensuring that different types of identifiers (API keys, ensemble IDs, etc.)
/// cannot be confused at the type level.
///
/// The prefix is specified as three `const char` generic parameters.
///
/// # Type Parameters
///
/// * `PFX_1` - First character of the prefix
/// * `PFX_2` - Second character of the prefix
/// * `PFX_3` - Third character of the prefix
///
/// # Examples
///
/// ```
/// use objectiveai::prefixed_uuid::PrefixedUuid;
///
/// // Define an API key type with prefix "apk"
/// type ApiKey = PrefixedUuid<'a', 'p', 'k'>;
///
/// // Create a new API key
/// let key = ApiKey::new();
/// println!("{}", key); // Outputs: apk<uuid>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PrefixedUuid<const PFX_1: char, const PFX_2: char, const PFX_3: char>
{
    uuid: uuid::Uuid,
}

impl<const PFX_1: char, const PFX_2: char, const PFX_3: char> From<uuid::Uuid>
    for PrefixedUuid<PFX_1, PFX_2, PFX_3>
{
    fn from(uuid: uuid::Uuid) -> Self {
        PrefixedUuid { uuid }
    }
}

/// Error type for parsing prefixed UUIDs from strings.
///
/// This enum represents the two possible failure modes when parsing
/// a prefixed UUID: an invalid prefix or an invalid UUID portion.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError<const PFX_1: char, const PFX_2: char, const PFX_3: char> {
    /// The string did not start with the expected prefix.
    #[error(
        "invalid prefix: expected {}{}{} but got {}",
        PFX_1,
        PFX_2,
        PFX_3,
        _0
    )]
    InvalidPrefix(String),
    /// The UUID portion of the string was invalid.
    #[error("invalid UUID: {0}")]
    InvalidUuid(uuid::Error),
}

impl<const PFX_1: char, const PFX_2: char, const PFX_3: char> FromStr
    for PrefixedUuid<PFX_1, PFX_2, PFX_3>
{
    type Err = ParseError<PFX_1, PFX_2, PFX_3>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() >= 3 + uuid::fmt::Simple::LENGTH && {
            let s_bytes = s.as_bytes();
            s_bytes[0] == (PFX_1 as u8)
                && s_bytes[1] == (PFX_2 as u8)
                && s_bytes[2] == (PFX_3 as u8)
        } {
            match uuid::Uuid::parse_str(&s[3..]) {
                Ok(uuid) => Ok(PrefixedUuid { uuid }),
                Err(e) => Err(ParseError::InvalidUuid(e)),
            }
        } else {
            Err(ParseError::InvalidPrefix(s.to_string()))
        }
    }
}

impl<const PFX_1: char, const PFX_2: char, const PFX_3: char>
    PrefixedUuid<PFX_1, PFX_2, PFX_3>
{
    /// Creates a new prefixed UUID with a random v4 UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// use objectiveai::prefixed_uuid::PrefixedUuid;
    ///
    /// type ApiKey = PrefixedUuid<'a', 'p', 'k'>;
    /// let key = ApiKey::new();
    /// ```
    pub fn new() -> Self {
        PrefixedUuid {
            uuid: uuid::Uuid::new_v4(),
        }
    }

    /// Returns the underlying UUID without the prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use objectiveai::prefixed_uuid::PrefixedUuid;
    ///
    /// type ApiKey = PrefixedUuid<'a', 'p', 'k'>;
    /// let key = ApiKey::new();
    /// let uuid = key.uuid();
    /// ```
    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }
}

impl<const PFX_1: char, const PFX_2: char, const PFX_3: char> std::fmt::Display
    for PrefixedUuid<PFX_1, PFX_2, PFX_3>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            PFX_1,
            PFX_2,
            PFX_3,
            self.uuid
                .simple()
                .encode_lower(&mut [0; uuid::fmt::Simple::LENGTH])
        )
    }
}

impl<const PFX_1: char, const PFX_2: char, const PFX_3: char> serde::Serialize
    for PrefixedUuid<PFX_1, PFX_2, PFX_3>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, const PFX_1: char, const PFX_2: char, const PFX_3: char>
    serde::Deserialize<'de> for PrefixedUuid<PFX_1, PFX_2, PFX_3>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PrefixedUuid::from_str(&s).map_err(serde::de::Error::custom)
    }
}
