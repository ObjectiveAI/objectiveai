//! The program's binary, beside this one.

use std::path::{Path, PathBuf};

use super::Error;

/// The path of `binary` in the directory this executable is in: the
/// daemon and the provider are installed beside the CLI, and nothing
/// else — not the working directory, not `PATH` — is looked in. Where
/// this executable is is the operating system's answer, and a host
/// that will not say is [`Error::Executable`]; nothing checks that
/// the file exists, which the spawn does.
pub fn beside(binary: &str) -> Result<PathBuf, Error> {
    let own = std::env::current_exe().map_err(Error::Executable)?;
    let dir = own.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok(dir.join(binary))
}
