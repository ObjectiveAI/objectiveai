//! Bytes as JSON: standard base64, padded.
//!
//! What every JSON form of the crate's that carries raw bytes writes
//! them as — a command's output, a message's body — for
//! `#[serde(serialize_with = "…::serialize", deserialize_with = "…::deserialize")]`.

use bytes::Bytes;

/// The bytes as one string of standard base64.
pub fn serialize<S: serde::Serializer>(bytes: &Bytes, serializer: S) -> Result<S::Ok, S::Error> {
    use ::base64::Engine as _;
    serializer.serialize_str(&::base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// One string of standard base64, as the bytes it says. A string that is
/// not base64 does not deserialize.
pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Bytes, D::Error> {
    use ::base64::Engine as _;
    let text = <String as serde::Deserialize>::deserialize(deserializer)?;
    ::base64::engine::general_purpose::STANDARD.decode(text).map(Bytes::from).map_err(serde::de::Error::custom)
}
