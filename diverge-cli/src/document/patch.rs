//! A patch to a document, read from the command line.

use diverge_sdk::daemon::patch::Patch;

use super::Error;

/// The patch a `patch` applies, read from `text` as YAML — JSON being
/// YAML — as the SDK's [`Patch`] type reads it: a list of operations.
/// Text that is not one is [`Error::Patch`].
pub fn patch(text: &str) -> Result<Patch, Error> {
    serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_str(text))
        .map_err(Error::Patch)
}
