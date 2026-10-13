//! What `diverge-postgres` is told: the root, and the cluster's
//! settings.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

/// The two arguments, both required and each given once:
/// `--root <dir>`, the root the cluster lives under at
/// `<root>/daemon/postgres/`, and `--max-connections <n>`, the most
/// connections the postmaster accepts at once. The daemon gives both
/// when it starts the program; a start by hand gives the same two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arguments {
    /// The root.
    pub root: PathBuf,
    /// The postmaster's `max_connections`.
    pub max_connections: u32,
}

/// The arguments, read from `args` — the arguments after the
/// program's name. Any other argument, one with nothing after it, one
/// given twice, a `--max-connections` that is not a number, or either
/// missing is the [`Error`].
pub fn parse(args: impl Iterator<Item = OsString>) -> Result<Arguments, Error> {
    let mut root: Option<PathBuf> = None;
    let mut max_connections: Option<u32> = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--root" && root.is_none() {
            match args.next() {
                Some(path) => root = Some(PathBuf::from(path)),
                None => return Err(Error::Missing("--root")),
            }
        } else if arg == "--max-connections" && max_connections.is_none() {
            match args.next().map(|value| value.to_string_lossy().parse::<u32>()) {
                Some(Ok(value)) => max_connections = Some(value),
                Some(Err(_)) => return Err(Error::NotANumber),
                None => return Err(Error::Missing("--max-connections")),
            }
        } else {
            return Err(Error::Unknown(arg.to_string_lossy().into_owned()));
        }
    }
    Ok(Arguments {
        root: root.ok_or(Error::Missing("--root"))?,
        max_connections: max_connections.ok_or(Error::Missing("--max-connections"))?,
    })
}

/// Why the arguments could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An argument the program does not take, or one given twice.
    Unknown(String),
    /// An argument the program needs was not given, or was given with
    /// nothing after it.
    Missing(&'static str),
    /// `--max-connections` was given something that is not a number.
    NotANumber,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unknown(argument) => write!(
                f,
                "the argument `{argument}` is not one this program takes; `--root <dir>` and `--max-connections <n>` are, once each"
            ),
            Error::Missing(argument) => write!(f, "`{argument}` was not given, or was given nothing"),
            Error::NotANumber => write!(f, "`--max-connections` was given something that is not a number"),
        }
    }
}

impl std::error::Error for Error {}
