//! Parent → child startup handshake for the `instance` subprocess.
//!
//! Purpose is purely the "are you us?" gate — only an
//! `objectiveai-cli` parent that explicitly inherited the read end of
//! an anonymous pipe and fed it the magic header can pass the check.
//! A shell user invoking `objectiveai-cli instance` directly has no
//! inherited fd; a curious user passing
//! `OBJECTIVEAI_INSTANCE_PIPE=0` (stdin) fails the magic-line check.
//!
//! After the handshake passes the instance runs identically to today
//! — same NDJSON stdout, same AF_UNIX pipe lifecycle, same SLOT_TAKEN
//! exit semantics.

use std::io::{BufRead, BufReader, Read, Write};

use os_pipe::{PipeReader, PipeWriter};

use super::request::InstanceRequest;

/// Env var the parent stamps with the inherited pipe's raw fd
/// (Unix) / handle (Windows) as a base-10 string.
pub const PIPE_ENV: &str = "OBJECTIVEAI_INSTANCE_PIPE";

/// Magic header line written by the parent and verified by the child
/// before deserializing the request blob. Bumped if the wire format
/// changes incompatibly.
pub const MAGIC: &str = "objectiveai-instance/1";

/// Error shape for both sides of the handshake. Carried as a String
/// since the child surfaces it as a process-exit error message.
pub type HandshakeError = String;

/// Parent-side: serialize `request` and write the framed handshake
/// (magic + JSON blob + EOF) to `writer`. The writer is consumed (and
/// dropped at the end) so the child sees EOF after the JSON blob.
pub fn write_request(
    mut writer: PipeWriter,
    request: &InstanceRequest,
) -> std::io::Result<()> {
    writer.write_all(MAGIC.as_bytes())?;
    writer.write_all(b"\n")?;
    let blob = serde_json::to_vec(request).expect("InstanceRequest serializes");
    writer.write_all(&blob)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    // Drop on return closes the pipe → child sees EOF.
    Ok(())
}

/// Child-side: locate the inherited pipe via `PIPE_ENV`, validate the
/// magic header, then deserialize the trailing JSON as
/// [`InstanceRequest`].
///
/// Returns `Err` with a user-facing message on any failure (no pipe,
/// bad magic, malformed JSON). The caller surfaces it as an exit-with-
/// nonzero error.
pub fn read_request() -> Result<InstanceRequest, HandshakeError> {
    let raw = std::env::var(PIPE_ENV).map_err(|_| {
        format!(
            "{PIPE_ENV} is not set — `objectiveai-cli instance` is invoked internally by the cli; do not run it directly"
        )
    })?;
    let reader = reader_from_env(&raw)?;
    let mut buf = BufReader::new(reader);

    let mut header = String::new();
    buf.read_line(&mut header)
        .map_err(|e| format!("failed to read handshake header: {e}"))?;
    let header = header.trim_end_matches(['\r', '\n']);
    if header != MAGIC {
        return Err(format!(
            "handshake header mismatch (expected `{MAGIC}`, got `{header}`) — `objectiveai-cli instance` is invoked internally by the cli; do not run it directly"
        ));
    }

    let mut json = Vec::new();
    buf.read_to_end(&mut json)
        .map_err(|e| format!("failed to read handshake body: {e}"))?;
    let mut de = serde_json::Deserializer::from_slice(&json);
    serde_path_to_error::deserialize(&mut de)
        .map_err(|e| format!("handshake body parse error at `{}`: {}", e.path(), e.inner()))
}

#[cfg(unix)]
fn reader_from_env(raw: &str) -> Result<PipeReader, HandshakeError> {
    use std::os::fd::FromRawFd;
    let fd: i32 = raw
        .parse()
        .map_err(|_| format!("{PIPE_ENV}=`{raw}` is not a valid fd"))?;
    // SAFETY: parent stamped this env var with a fd it inherited into
    // us; ownership of the fd is transferred from the env value.
    Ok(unsafe { PipeReader::from_raw_fd(fd) })
}

#[cfg(windows)]
fn reader_from_env(raw: &str) -> Result<PipeReader, HandshakeError> {
    use std::os::windows::io::{FromRawHandle, RawHandle};
    let handle: isize = raw
        .parse()
        .map_err(|_| format!("{PIPE_ENV}=`{raw}` is not a valid handle"))?;
    // SAFETY: parent stamped this env var with a handle it inherited
    // into us; ownership is transferred from the env value.
    Ok(unsafe { PipeReader::from_raw_handle(handle as RawHandle) })
}
