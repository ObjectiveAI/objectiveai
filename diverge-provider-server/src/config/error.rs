//! Why the provider's directory or its file could not be used.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// What [`dir`](super::dir) and [`load`](super::load) fail with: the
/// arguments, the host, the filesystem, or the file's own content.
#[derive(Debug)]
pub enum Error {
    /// An argument the provider does not take, or a `--config` with
    /// no directory after it.
    Arguments(String),
    /// No directory was named and the host has no home directory to
    /// put one under.
    Home,
    /// A directory or a file could not be made or read.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// The file does not parse, or says something no section takes.
    Parse {
        /// The file.
        path: PathBuf,
        /// Where in it, and what was wrong.
        source: serde_path_to_error::Error<serde_yaml_ng::Error>,
    },
    /// A store's or a fixed volume's path is not absolute.
    Relative(PathBuf),
    /// A store's capacity is `0`.
    Capacity(PathBuf),
    /// A fixed volume's path is not an existing directory.
    Missing(PathBuf),
    /// A fixed volume's name is not one a volume may have, or two
    /// fixed volumes have it.
    FixedName(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Arguments(argument) => {
                write!(f, "the argument `{argument}` is not one the provider takes; `--config <dir>` is")
            }
            Error::Home => write!(f, "no directory was named and the host has no home directory"),
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Parse { path, source } => write!(f, "`{}` could not be read: {source}", path.display()),
            Error::Relative(path) => write!(f, "the volume path `{}` is not absolute", path.display()),
            Error::Capacity(path) => write!(f, "the store `{}` has a capacity of 0", path.display()),
            Error::Missing(path) => write!(f, "the fixed volume `{}` is not a directory that exists", path.display()),
            Error::FixedName(name) => write!(f, "the fixed volume name `{name}` is not usable, or is used twice"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Arguments(_) => None,
            Error::Home => None,
            Error::Io { source, .. } => Some(source),
            Error::Parse { source, .. } => Some(source),
            Error::Relative(_) => None,
            Error::Capacity(_) => None,
            Error::Missing(_) => None,
            Error::FixedName(_) => None,
        }
    }
}
