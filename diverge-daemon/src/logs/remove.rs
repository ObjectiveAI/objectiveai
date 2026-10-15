//! Removing an agent's log.

use std::path::Path;

use super::{Error, dir};
use crate::store::AgentId;

/// Remove the agent's log directory, whole; one that was never made
/// is nothing to remove.
pub async fn remove(root: &Path, id: AgentId) -> Result<(), Error> {
    let dir = dir(root, id);
    match tokio::fs::remove_dir_all(&dir).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::io(&dir, source)),
    }
}
