//! A value's canonical bytes: what is hashed.

use serde::Serialize;

use super::sort_object_keys;

/// The value as compact JSON with every object key sorted at every
/// depth: the bytes every id that is a hash is the SHA-256 of. The
/// ordinary JSON failure for a value that does not serialize.
pub fn bytes<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut value = serde_json::to_value(value)?;
    sort_object_keys(&mut value);
    serde_json::to_vec(&value)
}
