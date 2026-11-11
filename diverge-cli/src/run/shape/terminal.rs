//! A terminal, wired to this command's own stdin and stdout.

use bytes::Bytes;
use diverge_sdk::daemon::client::stream::StreamError;
use futures_util::{Stream, StreamExt as _};
use serde::Serialize;
use std::future::Future;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::exchange;
use crate::run::Error;

/// What one answer of a terminal says, whichever family's it is.
pub enum Said {
    /// Bytes the terminal printed.
    Stdout(Bytes),
    /// The shell exited, with this code.
    Exited(i32),
    /// A signal ended it.
    Signaled(i32),
    /// Not found, forbidden, or an error: no terminal.
    Refused,
}

/// The most this command reads of its stdin at once, which is the most
/// one `Stdin` carries.
const READ: usize = 64 * 1024;

/// The byte a terminal reads as the end of its input: Ctrl-D.
const EOF: &[u8] = &[0x04];

/// Wire a terminal to this command.
///
/// The rules:
///
/// 1. What this command reads on its stdin goes to the terminal's, as
///    it comes, each piece once the last is written.
/// 2. This command's stdin ending sends the terminal Ctrl-D, its own
///    end of input, which a shell reads as `exit` once it has read
///    everything before it; the terminal is not closed, so what was
///    sent runs to the end.
/// 3. What the terminal prints goes to this command's stdout as it
///    comes; an answer that is no output — not found, forbidden, an
///    error — is its JSON line on stderr.
/// 4. This command exits with the shell's code: its own, `128` and the
///    signal for a signal, `1` for anything else. Ending this command
///    early closes the terminal, which kills the shell.
pub async fn terminal<F, E, S, W, Fut>(mut answers: S, stdin: W, said: fn(&F) -> Said) -> Result<(), Error>
where
    F: Serialize,
    E: std::fmt::Display,
    S: Stream<Item = Result<F, StreamError<E>>> + Unpin,
    W: Fn(Vec<u8>) -> Fut + Send + 'static,
    Fut: Future<Output = bool> + Send,
{
    // Rules 1 and 2.
    let feeding = tokio::spawn(async move {
        let mut input = tokio::io::stdin();
        let mut buffer = vec![0u8; READ];
        loop {
            match input.read(&mut buffer).await {
                Ok(0) | Err(_) => {
                    stdin(EOF.to_vec()).await;
                    return;
                }
                Ok(read) => {
                    if !stdin(buffer[..read].to_vec()).await {
                        return;
                    }
                }
            }
        }
    });
    let mut code = 1;
    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();
    let ended = async {
        while let Some(item) = answers.next().await {
            let answer = item.map_err(exchange::stream)?;
            // Rule 3.
            match said(&answer) {
                Said::Stdout(bytes) => {
                    stdout.write_all(&bytes).await.map_err(|error| Error::Exchange(error.to_string()))?;
                    stdout.flush().await.map_err(|error| Error::Exchange(error.to_string()))?;
                }
                Said::Exited(exited) => code = exited,
                Said::Signaled(signal) => code = 128 + signal,
                Said::Refused => {
                    let line = serde_json::to_string(&answer).unwrap_or_default();
                    stderr.write_all(format!("{line}\n").as_bytes()).await.map_err(|error| Error::Exchange(error.to_string()))?;
                }
            }
        }
        Ok::<(), Error>(())
    }
    .await;
    feeding.abort();
    ended?;
    // Rule 4.
    let _ = stdout.flush().await;
    let _ = stderr.flush().await;
    std::process::exit(code);
}
