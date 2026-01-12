use std::str::FromStr;

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

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError<const PFX_1: char, const PFX_2: char, const PFX_3: char> {
    #[error(
        "invalid prefix: expected {}{}{} but got {}",
        PFX_1,
        PFX_2,
        PFX_3,
        _0
    )]
    InvalidPrefix(String),
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
    pub fn new() -> Self {
        PrefixedUuid {
            uuid: uuid::Uuid::new_v4(),
        }
    }

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
