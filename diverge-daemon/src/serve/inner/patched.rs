//! A record's editable members, patched: the document the patch is
//! applied to, and the result read back as the same type.

use diverge_sdk::daemon::patch::{self, Operation};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// `current` as JSON with `patch` applied, read back as `E`: what every
/// edit does first. The sentence of either failure — an operation
/// that could not be applied, named with its place; a result that is
/// not an `E`, named with the member that was wrong — is the edit's
/// `patch` error, and nothing changed.
pub fn patched<E>(current: &E, patch: &[Operation]) -> Result<E, String>
where
    E: Serialize + DeserializeOwned,
{
    let document = serde_json::to_value(current).map_err(|error| format!("the record could not be written as JSON: {error}"))?;
    let result = patch::apply(document, patch).map_err(|error| error.to_string())?;
    serde_path_to_error::deserialize(result).map_err(|error: serde_path_to_error::Error<serde_json::Error>| format!("the result is not what the record may be: {error}"))
}
