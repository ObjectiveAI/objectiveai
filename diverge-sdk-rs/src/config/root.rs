//! Finding the root.

use std::ffi::OsString;
use std::path::PathBuf;

use super::Error;

/// The root, from `args` — the arguments after the program's name —
/// else the environment, else the home directory; made if absent,
/// and made absolute.
///
/// `--config <dir>` is the one argument there is, and it may be given
/// once. Any other argument, or a `--config` with nothing after it,
/// is [`Error::Arguments`]. With no argument, `DIVERGE_CONFIG` names
/// the root; with neither, it is `.diverge` under the home directory
/// — `HOME`, or `USERPROFILE` on Windows — and a host with no home is
/// [`Error::Home`]. The path is made absolute against the working
/// directory without following links, so what a program names by it
/// — a provider its containers — is the path as it was given.
pub async fn root(args: impl Iterator<Item = OsString>) -> Result<PathBuf, Error> {
    let mut given: Option<PathBuf> = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--config" && given.is_none() {
            match args.next() {
                Some(path) => given = Some(PathBuf::from(path)),
                None => return Err(Error::Arguments("--config".to_string())),
            }
        } else {
            return Err(Error::Arguments(arg.to_string_lossy().into_owned()));
        }
    }
    let root = match given.or_else(|| std::env::var_os("DIVERGE_CONFIG").map(PathBuf::from)) {
        Some(root) => root,
        None => home()?.join(".diverge"),
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

/// The home directory: `HOME`, and on Windows `USERPROFILE` where
/// `HOME` is not set.
fn home() -> Result<PathBuf, Error> {
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home));
    }
    if cfg!(windows)
        && let Some(home) = std::env::var_os("USERPROFILE")
    {
        return Ok(PathBuf::from(home));
    }
    Err(Error::Home)
}
