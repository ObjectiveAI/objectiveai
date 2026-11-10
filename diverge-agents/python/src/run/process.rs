//! The Python process: spawned, fed, waited for, and read.

use std::process::Stdio;

use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

use super::{Envelope, Error, Feed};
use crate::registration;

/// Where the script runs from: root's home, the image's working
/// directory, where whatever the script writes stays for the
/// container's life.
const CWD: &str = "/root";

/// Run the harness once and take its envelope.
///
/// `python3 <harness> <source>`, the feed on stdin, stdout and stderr
/// captured whole. The feed is written and the process waited for
/// TOGETHER: the harness reads stdin to its end before it writes
/// anything, so neither pipe can fill against the other, and a
/// process that dies before reading — a source that will not parse —
/// closes the pipe under the write, which is the exit's to report,
/// not the write's.
///
/// The exit classifies, in the CLI's order: a nonzero status, or a
/// signal, is the script raising, stderr its traceback; a clean exit
/// whose last non-empty stdout line is not the envelope is the
/// harness broken — the script did something to the machinery — and
/// that line is quoted back.
pub async fn invoke(feed: &Feed<'_>) -> Result<Envelope, Error> {
    let stdin = serde_json::to_vec(feed).map_err(Error::Feed)?;

    let mut child = Command::new("python3")
        .arg(registration::HARNESS)
        .arg(registration::SOURCE)
        .current_dir(CWD)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::Spawn)?;

    let mut pipe = child.stdin.take().expect("stdin was piped");
    let write = async move {
        let _ = pipe.write_all(&stdin).await;
        let _ = pipe.shutdown().await;
    };
    let (_, output) = futures_util::future::join(write, child.wait_with_output()).await;
    let output = output.map_err(Error::Wait)?;

    if !output.status.success() {
        return Err(Error::Exception {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    serde_json::from_str(line).map_err(|error| Error::Harness {
        line: line.to_string(),
        error,
    })
}
