//! A command's stream, printed: as JSON lines, or raw.

use bytes::Bytes;
use diverge_sdk::daemon::client::stream::StreamError;
use futures_util::{Stream, StreamExt as _};
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;

use super::exchange;
use crate::run::{Emitter, Error};

/// What one answer of an exec says, whichever family's it is.
pub enum Said {
    /// Bytes the command wrote to its stdout.
    Stdout(Bytes),
    /// Bytes it wrote to its stderr.
    Stderr(Bytes),
    /// It exited, with this code.
    Exited(i32),
    /// A signal ended it.
    Signaled(i32),
    /// Not found, forbidden, or an error: no command output.
    Refused,
}

/// Print an exec's answers.
///
/// The rules:
///
/// 1. Without `raw`, every answer is one JSON line, as every leaf
///    prints, its bytes in base64.
/// 2. With `raw`, stdout's bytes go to this command's stdout and
///    stderr's to its stderr, as they come; an answer that is no output
///    — not found, forbidden, an error — is its JSON line on stderr.
///    The command then exits with the command's code: its own, `128`
///    and the signal for a signal, `1` for anything else.
pub async fn exec<F, E, S>(mut answers: S, raw: bool, emitter: &mut Emitter, said: fn(&F) -> Said) -> Result<(), Error>
where
    F: Serialize,
    E: std::fmt::Display,
    S: Stream<Item = Result<F, StreamError<E>>> + Unpin,
{
    let mut code = 1;
    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();
    while let Some(item) = answers.next().await {
        let answer = item.map_err(exchange::stream)?;
        // Rule 1.
        if !raw {
            emitter.emit(&answer).await?;
            continue;
        }
        // Rule 2.
        match said(&answer) {
            Said::Stdout(bytes) => stdout.write_all(&bytes).await.map_err(|error| Error::Exchange(error.to_string()))?,
            Said::Stderr(bytes) => stderr.write_all(&bytes).await.map_err(|error| Error::Exchange(error.to_string()))?,
            Said::Exited(exited) => code = exited,
            Said::Signaled(signal) => code = 128 + signal,
            Said::Refused => {
                let line = serde_json::to_string(&answer).unwrap_or_default();
                stderr.write_all(format!("{line}\n").as_bytes()).await.map_err(|error| Error::Exchange(error.to_string()))?;
            }
        }
    }
    if raw {
        let _ = stdout.flush().await;
        let _ = stderr.flush().await;
        std::process::exit(code);
    }
    Ok(())
}
