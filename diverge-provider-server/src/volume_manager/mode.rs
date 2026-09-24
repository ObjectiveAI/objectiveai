//! A stored volume's persist mode, kept beside its image.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use super::Error;

/// What the mode file holds: whether the volume keeps what containers
/// write into it. One JSON object, `{"persist":true}`, so that what
/// comes to be kept beside the image later has a place to land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    /// The volume's `persist`, as a listing reports it.
    pub persist: bool,
}

/// Where a stored volume's mode file is: `<store>/<identity>/.<name>`,
/// the dotfile beside the image. A volume's name never begins with a
/// dot — see [`ok`](super::ok) — so the mode file is never taken for
/// a volume, and no volume is ever taken for a mode file.
pub fn mode_path(store: &Path, client_identity: &str, name: &str) -> PathBuf {
    store.join(client_identity).join(format!(".{name}"))
}

/// The mode file read: [`None`] when there is none, which is an image
/// that is not a volume.
pub async fn read_mode(path: &Path) -> Result<Option<Mode>, Error> {
    let bytes = match fs::read(path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::Io(error)),
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

/// The mode file written, whole: to a temporary beside it and renamed
/// over, so a reader sees the old mode or the new and never a torn
/// one, and a crash between the two leaves the old.
pub async fn write_mode(path: &Path, mode: Mode) -> Result<(), Error> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let bytes = serde_json::to_vec(&mode)?;
    fs::write(&temporary, bytes).await?;
    fs::rename(&temporary, path).await?;
    Ok(())
}
