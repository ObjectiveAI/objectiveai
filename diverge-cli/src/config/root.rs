//! Finding the root.

use std::io;
use std::path::PathBuf;

use super::Error;

/// The directory looked for in the working directory, and under the
/// home directory.
const DIVERGE: &str = ".diverge";

/// The root, from `given` — the `--config` directory, if one was —
/// else the environment, else the working directory's `.diverge`
/// where it is a directory, else the home directory's; made if
/// absent, and made absolute.
///
/// The first two and the last are what every Diverge program does:
/// `DIVERGE_CONFIG` names the root when nothing was given, and the
/// home directory — `HOME`, or `USERPROFILE` on Windows — holds it
/// when neither did, a host with no home being [`Error::Home`]. The
/// working directory's turn is this program's alone, taken only when
/// neither an argument nor the environment names a root, and only
/// when the directory exists — its `cli.yaml` need not. The path is
/// made absolute against the working directory without following
/// links. A directory that cannot be read or made is [`Error::Io`].
pub async fn root(given: Option<PathBuf>) -> Result<PathBuf, Error> {
    let root = match given.or_else(|| std::env::var_os("DIVERGE_CONFIG").map(PathBuf::from)) {
        Some(root) => root,
        None => match local().await? {
            Some(local) => local,
            None => home()?.join(DIVERGE),
        },
    };
    let root = std::path::absolute(&root).map_err(|source| Error::Io {
        path: root.clone(),
        source,
    })?;
    tokio::fs::create_dir_all(&root).await.map_err(|source| Error::Io {
        path: root.clone(),
        source,
    })?;
    Ok(root)
}

/// The working directory's `.diverge`, if it is a directory.
async fn local() -> Result<Option<PathBuf>, Error> {
    let here = std::env::current_dir().map_err(|source| Error::Io {
        path: PathBuf::from("."),
        source,
    })?;
    let local = here.join(DIVERGE);
    match tokio::fs::metadata(&local).await {
        Ok(metadata) if metadata.is_dir() => Ok(Some(local)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io { path: local, source }),
    }
}

/// The home directory: `HOME`, and on Windows `USERPROFILE` where
/// `HOME` is not set.
fn home() -> Result<PathBuf, Error> {
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home));
    }
    if cfg!(windows) {
        if let Some(home) = std::env::var_os("USERPROFILE") {
            return Ok(PathBuf::from(home));
        }
    }
    Err(Error::Home)
}
