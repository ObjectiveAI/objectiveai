//! What a run of a hook fails with.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

/// A hook that did not answer.
///
/// The first five are found before the hook runs: the name, the
/// folder and its manifest, the platform. The next four are this
/// end's failure to run it or to hand it the input.
/// [`Status`](Self::Status) is the hook's own: it exited non-zero,
/// and what it wrote is carried so the operator can read why.
/// [`Parse`](Self::Parse) is a hook that exited `0` with a stdout that
/// is not the expected output, and it carries the path inside the
/// document where the mismatch is.
#[derive(Debug)]
pub enum Error {
    /// The name is not one path component, so it cannot name a folder
    /// under `hooks/`.
    Name(String),
    /// The manifest could not be read: the folder or the file is
    /// absent, or unreadable.
    Read {
        /// The manifest's path.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// The manifest is not a manifest.
    Manifest {
        /// The manifest's path.
        path: PathBuf,
        /// Why, with the path inside the document.
        source: serde_path_to_error::Error<serde_yaml_ng::Error>,
    },
    /// The manifest has no command for this platform.
    Unsupported {
        /// The hook.
        name: String,
        /// This platform, as the manifest would spell it.
        platform: &'static str,
    },
    /// This platform's command is an empty array.
    Empty {
        /// The hook.
        name: String,
    },
    /// The input would not serialize as JSON.
    Encode(serde_json::Error),
    /// The program could not be started.
    Spawn {
        /// The program as it was resolved.
        program: String,
        /// Why the OS would not start it.
        source: io::Error,
    },
    /// The input could not be written to the program's stdin.
    Stdin(io::Error),
    /// The program could not be waited for.
    Wait(io::Error),
    /// The program exited non-zero.
    Status {
        /// How it exited.
        status: ExitStatus,
        /// What it wrote to stdout, lossily decoded.
        stdout: String,
        /// What it wrote to stderr, lossily decoded.
        stderr: String,
    },
    /// The program exited `0`, and its stdout is not the output's type.
    Parse(serde_path_to_error::Error<serde_json::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Name(name) => {
                write!(f, "`{name}` is not a hook name: one path component is required")
            }
            Error::Read { path, source } => {
                write!(f, "the hook manifest `{}` could not be read: {source}", path.display())
            }
            Error::Manifest { path, source } => write!(
                f,
                "the hook manifest `{}` did not parse at `{}`: {}",
                path.display(),
                source.path(),
                source.inner()
            ),
            Error::Unsupported { name, platform } => {
                write!(f, "the hook `{name}` has no command for {platform}")
            }
            Error::Empty { name } => {
                write!(f, "the hook `{name}` names no program for this platform")
            }
            Error::Encode(error) => {
                write!(f, "the input did not serialize: {error}")
            }
            Error::Spawn { program, source } => {
                write!(f, "the hook program `{program}` could not be started: {source}")
            }
            Error::Stdin(error) => {
                write!(f, "the input could not be written to the hook: {error}")
            }
            Error::Wait(error) => {
                write!(f, "the hook could not be waited for: {error}")
            }
            Error::Status { status, stdout, stderr } => {
                write!(f, "the hook exited with {status}")?;
                if !stdout.trim().is_empty() {
                    write!(f, "; stdout: {}", stdout.trim_end())?;
                }
                if !stderr.trim().is_empty() {
                    write!(f, "; stderr: {}", stderr.trim_end())?;
                }
                Ok(())
            }
            Error::Parse(error) => write!(
                f,
                "the hook's output did not parse at `{}`: {}",
                error.path(),
                error.inner()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Read { source, .. } | Error::Spawn { source, .. } => Some(source),
            Error::Manifest { source, .. } => Some(source),
            Error::Encode(error) => Some(error),
            Error::Stdin(error) | Error::Wait(error) => Some(error),
            Error::Parse(error) => Some(error),
            Error::Name(_)
            | Error::Unsupported { .. }
            | Error::Empty { .. }
            | Error::Status { .. } => None,
        }
    }
}
