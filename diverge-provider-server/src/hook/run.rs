//! Running a hook once, and reading what it answered.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

use super::{Error, MANIFEST, Manifest};

/// Run the hook `name` once on `input` and read its output.
///
/// In order: the name is checked — one path component, not empty,
/// not `.` or `..`, no `/` or `\` — and refused before the filesystem
/// is touched; `hooks_dir/<name>/hook.yaml` is read and parsed; this
/// platform's command is taken, or the hook is refused as
/// unsupported here; the input is serialized as one line of JSON;
/// the program is started with the hook's folder as its working
/// directory, stdin, stdout and stderr piped, and is killed if this
/// future is dropped; the line is written and stdin closed; the exit
/// is awaited, with no timeout.
///
/// Exit `0`: stdout is one JSON document, read into `O`, the path of
/// any mismatch kept in the error. Any other exit: an error carrying
/// the status and what the hook wrote to stdout and stderr.
pub async fn run<I, O>(hooks_dir: &Path, name: &str, input: &I) -> Result<O, Error>
where
    I: Serialize + ?Sized,
    O: DeserializeOwned,
{
    let folder = folder(hooks_dir, name)?;
    let manifest = folder.join(MANIFEST);
    let bytes = tokio::fs::read(&manifest)
        .await
        .map_err(|source| Error::Read {
            path: manifest.clone(),
            source,
        })?;
    let manifest: Manifest = serde_path_to_error::deserialize(
        serde_yaml_ng::Deserializer::from_slice(&bytes),
    )
    .map_err(|source| Error::Manifest {
        path: folder.join(MANIFEST),
        source,
    })?;
    let command = manifest.command().ok_or_else(|| Error::Unsupported {
        name: name.to_owned(),
        platform: Manifest::PLATFORM,
    })?;
    let (first, args) = command.split_first().ok_or_else(|| Error::Empty {
        name: name.to_owned(),
    })?;
    let program = program(&folder, first);

    let mut line = serde_json::to_vec(input).map_err(Error::Encode)?;
    line.push(b'\n');

    let mut child = Command::new(&program)
        .args(args)
        .current_dir(&folder)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|source| Error::Spawn {
            program: program.display().to_string(),
            source,
        })?;

    // Taken out and dropped after the write, so the hook sees EOF and
    // knows the line is whole.
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&line).await.map_err(Error::Stdin)?;
        stdin.shutdown().await.map_err(Error::Stdin)?;
    }

    let output = child.wait_with_output().await.map_err(Error::Wait)?;
    if !output.status.success() {
        return Err(Error::Status {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let mut deserializer = serde_json::Deserializer::from_slice(&output.stdout);
    serde_path_to_error::deserialize(&mut deserializer).map_err(Error::Parse)
}

/// `hooks_dir/<name>`, for a name that is one path component.
///
/// A name that is empty, `.`, `..`, or holds a separator could name
/// something outside `hooks/`, and is refused instead.
fn folder(hooks_dir: &Path, name: &str) -> Result<PathBuf, Error> {
    let component = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\']);
    if !component {
        return Err(Error::Name(name.to_owned()));
    }
    Ok(hooks_dir.join(name))
}

/// The program to start: a relative first element that exists in the
/// folder is that file, so "in here" holds on Windows too, where a
/// relative program resolves against the parent's directory rather
/// than the child's; anything else is passed as written — an
/// absolute path, or a bare name the OS resolves on `PATH`.
fn program(folder: &Path, first: &str) -> PathBuf {
    let first = Path::new(first);
    if first.is_relative() {
        let inside = folder.join(first);
        if inside.exists() {
            return inside;
        }
    }
    first.to_path_buf()
}
