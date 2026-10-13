//! Finding the root.

use std::path::PathBuf;

use diverge_sdk::config;

use super::Error;

/// The directory looked for in the working directory.
const DIVERGE: &str = ".diverge";

/// The root, from `given` — the `--config` directory, if one was —
/// else the environment, else the working directory's `.diverge`
/// where it is a directory, else the home directory's; made if
/// absent, and made absolute.
///
/// `given` and `DIVERGE_CONFIG` are the SDK's turn,
/// [`root_at`](config::root_at), and so is the home directory: the
/// working directory's is this program's alone, taken only when
/// neither an argument nor the environment names a root, and only
/// when the directory exists — its `cli.json` need not. A working
/// directory that cannot be read is [`Error::Io`].
pub async fn root(given: Option<PathBuf>) -> Result<PathBuf, Error> {
    if given.is_some() || std::env::var_os("DIVERGE_CONFIG").is_some() {
        return Ok(config::root_at(given).await?);
    }
    let here = std::env::current_dir().map_err(|source| Error::Io {
        path: PathBuf::from("."),
        source,
    })?;
    let local = here.join(DIVERGE);
    let exists = match tokio::fs::metadata(&local).await {
        Ok(metadata) => metadata.is_dir(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(source) => return Err(Error::Io { path: local, source }),
    };
    if exists {
        return Ok(config::root_at(Some(local)).await?);
    }
    Ok(config::root_at(None).await?)
}
