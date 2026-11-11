//! A frontend request, its body this command's stdin and its answer
//! this command's output.

use bytes::Bytes;
use diverge_sdk::daemon::client::stream::StreamError;
use futures_util::{Stream, StreamExt as _};
use serde::Serialize;
use std::future::Future;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::exchange;
use crate::run::Error;

/// What one answer of a frontend request says, whichever family's it is.
pub enum Said {
    /// The response's head.
    Head,
    /// A piece of the response's body.
    Body(Bytes),
    /// Not found, forbidden, no frontend, or an error.
    Refused,
}

/// The most this command reads of its stdin at once, which is the most
/// one `Body` carries.
const READ: usize = 64 * 1024;

/// Send a frontend request from this command.
///
/// The rules:
///
/// 1. What this command reads on its stdin is the request's body, each
///    piece once the last is taken; its stdin ending is `End`.
/// 2. The response's head is one JSON line on stderr; the body goes to
///    stdout as it comes, raw. An answer that is no response — not
///    found, forbidden, no frontend, an error — is its JSON line on
///    stderr.
/// 3. This command exits `0` when the answer was a response that ended
///    whole, and `1` otherwise.
pub async fn frontend<F, E, S, B, BF, N, NF>(mut answers: S, body: B, end: N, said: fn(&F) -> Said) -> Result<(), Error>
where
    F: Serialize,
    E: std::fmt::Display,
    S: Stream<Item = Result<F, StreamError<E>>> + Unpin,
    B: Fn(Vec<u8>) -> BF + Send + 'static,
    BF: Future<Output = bool> + Send,
    N: FnOnce() -> NF + Send + 'static,
    NF: Future<Output = ()> + Send,
{
    // Rule 1.
    let feeding = tokio::spawn(async move {
        let mut input = tokio::io::stdin();
        let mut buffer = vec![0u8; READ];
        loop {
            match input.read(&mut buffer).await {
                Ok(0) | Err(_) => {
                    end().await;
                    return;
                }
                Ok(read) => {
                    if !body(buffer[..read].to_vec()).await {
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
            // Rule 2.
            match said(&answer) {
                Said::Head => {
                    let line = serde_json::to_string(&answer).unwrap_or_default();
                    stderr.write_all(format!("{line}\n").as_bytes()).await.map_err(|error| Error::Exchange(error.to_string()))?;
                    code = 0;
                }
                Said::Body(bytes) => {
                    stdout.write_all(&bytes).await.map_err(|error| Error::Exchange(error.to_string()))?;
                    stdout.flush().await.map_err(|error| Error::Exchange(error.to_string()))?;
                }
                Said::Refused => {
                    let line = serde_json::to_string(&answer).unwrap_or_default();
                    stderr.write_all(format!("{line}\n").as_bytes()).await.map_err(|error| Error::Exchange(error.to_string()))?;
                    code = 1;
                }
            }
        }
        Ok::<(), Error>(())
    }
    .await;
    feeding.abort();
    ended?;
    // Rule 3.
    let _ = stdout.flush().await;
    let _ = stderr.flush().await;
    std::process::exit(code);
}
