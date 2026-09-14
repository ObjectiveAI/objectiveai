//! What a store keeps beside a volume: what a listing knows without
//! looking.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The two facts a listing reports that no walk can find: how big the
/// volume may be, and when it came into being. Kept as JSON at
/// [`path`], beside the volume's directory and outside it, so a
/// container mounting the volume never sees it.
///
/// ```json
/// {"bytes": 1073741824, "created": 1788393600}
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sidecar {
    /// How big the volume may be, in bytes: what its create asked for,
    /// as its last edit left it.
    pub bytes: u64,
    /// When the volume came into being, in seconds since the Unix
    /// epoch.
    pub created: u64,
}

/// Where a stored volume's sidecar is: `<store>/<identity>/.<name>`,
/// beside the volume's directory `<store>/<identity>/<name>/`. A name
/// never begins with `.`, which is what keeps the two apart — see
/// [`name_ok`].
pub fn path(store: &Path, identity: &str, name: &str) -> PathBuf {
    store.join(identity).join(format!(".{name}"))
}

/// Whether `name` may be a volume's: one path component, and not one
/// that names a sidecar or an instruction. Not empty, not beginning
/// with `.`, and holding no `/`, `\` or NUL. `.` and `..` are refused
/// by the first rule.
pub fn name_ok(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', '\0'])
}

impl Sidecar {
    /// Read the sidecar at `path`. A file that is not there, or will
    /// not parse, is the error.
    pub async fn read(path: &Path) -> io::Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        serde_json::from_slice(&bytes).map_err(io::Error::other)
    }

    /// Write the sidecar at `path`, whole or not at all: to a
    /// temporary beside it, renamed over it once written.
    pub async fn write(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        let Some(name) = path.file_name() else {
            return Err(io::Error::other("a sidecar path names no file"));
        };
        let temporary = path.with_file_name(format!("{}.writing", name.to_string_lossy()));
        tokio::fs::write(&temporary, &bytes).await?;
        tokio::fs::rename(&temporary, path).await
    }
}
