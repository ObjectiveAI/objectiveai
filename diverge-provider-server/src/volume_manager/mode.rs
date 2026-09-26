//! A stored volume's mode, kept beside its image.

use std::path::{Path, PathBuf};

use diverge_sdk::provider::endpoints::volumes::Mode;
use serde::{Deserialize, Serialize};
use tokio::fs;

use super::Error;

/// What the mode file holds: the volume's [`Mode`]. One JSON object,
/// `{"mode":"persistent"}`, so that what comes to be kept beside the
/// image later has a place to land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModeFile {
    /// The volume's mode, as a listing reports it.
    pub mode: Mode,
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
pub async fn read_mode(path: &Path) -> Result<Option<ModeFile>, Error> {
    match fs::read(path).await {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// The mode file written whole: to a temporary beside it, renamed
/// over it, so a reader sees the old file or the new and never a
/// part.
pub async fn write_mode(path: &Path, mode: ModeFile) -> Result<(), Error> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec(&mode)?).await?;
    fs::rename(&temporary, path).await?;
    Ok(())
}
