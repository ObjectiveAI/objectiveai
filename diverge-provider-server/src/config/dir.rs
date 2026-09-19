//! Finding the provider's directory.

use std::ffi::OsString;
use std::path::PathBuf;

use super::Error;

/// The provider's directory, from `args` — the arguments after the
/// program's name — else the environment, else the home directory;
/// made if absent, with `hooks/` inside it, and made absolute.
///
/// `--config <dir>` is the one argument there is, and it may be given
/// once. Any other argument, or a `--config` with nothing after it,
/// is [`Error::Arguments`]. With no argument, `DIVERGE_PROVIDER_CONFIG`
/// names the directory; with neither, it is `.diverge/provider` under
/// the home directory — `HOME`, or `USERPROFILE` on Windows — and a
/// host with no home is [`Error::Home`]. The path is made absolute
/// against the working directory without following links, so what
/// names this provider's containers is the path as it was given.
pub async fn dir(args: impl Iterator<Item = OsString>) -> Result<PathBuf, Error> {
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
    let dir = match given.or_else(|| std::env::var_os("DIVERGE_PROVIDER_CONFIG").map(PathBuf::from)) {
        Some(dir) => dir,
        None => home()?.join(".diverge").join("provider"),
    };
    let dir = std::path::absolute(&dir).map_err(|source| Error::Io {
        path: dir.clone(),
        source,
    })?;
    let hooks = dir.join("hooks");
    tokio::fs::create_dir_all(&hooks).await.map_err(|source| Error::Io {
        path: hooks,
        source,
    })?;
    Ok(dir)
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
