//! Streamable-HTTP response parsing.
//!
//! The MCP wire spec lets a server respond to a POSTed JSON-RPC request
//! with either a bare JSON body (`Content-Type: application/json`) or an
//! SSE envelope (`Content-Type: text/event-stream`) wrapping a single
//! `data:` line whose payload is the same JSON-RPC response. Real-world
//! servers vary — `rmcp`'s `StreamableHttpService` always uses SSE, while
//! many production MCP servers reply with bare JSON. Clients have to
//! tolerate both.

use futures_util::TryStreamExt;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, Lines};
use tokio_util::io::StreamReader;

/// Boxed line-buffered reader over an SSE response body. Used to stream
/// `data:` lines incrementally — initial event from the `initialize`
/// response in [`super::Client::connect`] (when the server replies with
/// SSE), followed by subsequent notifications consumed by
/// [`super::Connection`]'s list-changed listener task.
///
/// Boxed to keep the type carryable across crate boundaries without
/// leaking the reqwest/tokio-util/futures-util generics. Callers treat
/// it as an async line iterator and don't care about the underlying
/// transport.
pub type LinesStream = Lines<Box<dyn AsyncBufRead + Send + Unpin>>;

/// Wraps a streaming `reqwest::Response` body into a [`LinesStream`].
/// Errors from the underlying byte stream are surfaced as
/// `std::io::Error` so the line reader can hand them back via
/// `next_line()`.
pub(crate) fn lines_from_response(response: reqwest::Response) -> LinesStream {
    let bytes = response
        .bytes_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let reader = StreamReader::new(bytes);
    let boxed: Box<dyn AsyncBufRead + Send + Unpin> = Box::new(reader);
    boxed.lines()
}

/// Reads the next complete SSE event from `lines` and parses its `data:`
/// payload as `T`. SSE events are terminated by a blank line; any
/// `data:` lines before that blank are concatenated. Other SSE fields
/// (`id:`, `event:`, `retry:`, comments) are ignored.
///
/// Returns `MalformedResponse` if the stream ends before a complete
/// event is delivered, or if the payload doesn't deserialize as `T`.
pub(crate) async fn read_next_sse_event<T: serde::de::DeserializeOwned>(
    url: &str,
    lines: &mut LinesStream,
) -> Result<T, super::Error> {
    let mut payload = String::new();
    loop {
        let line = lines
            .next_line()
            .await
            .map_err(|e| super::Error::MalformedResponse {
                url: url.to_string(),
                message: format!("error reading SSE line: {e}"),
            })?;
        match line {
            None => {
                return Err(super::Error::MalformedResponse {
                    url: url.to_string(),
                    message: "SSE stream ended before a complete event was delivered".into(),
                });
            }
            Some(l) if l.is_empty() => {
                if payload.is_empty() {
                    // Blank line before any data — ignore (heartbeat / pre-event).
                    continue;
                }
                return serde_json::from_str(&payload).map_err(|e| super::Error::MalformedResponse {
                    url: url.to_string(),
                    message: format!(
                        "SSE event payload did not deserialize as JSON-RPC response: {e}; payload starts with: {}",
                        payload.chars().take(200).collect::<String>(),
                    ),
                });
            }
            Some(l) => {
                if let Some(data) = l.strip_prefix("data: ").or_else(|| l.strip_prefix("data:")) {
                    payload.push_str(data);
                }
                // Other SSE fields and comments are skipped silently.
            }
        }
    }
}

/// Parses a JSON-RPC response from a streamable-HTTP `Response` *body*,
/// consuming the entire body. Accepts either a bare JSON body or an SSE
/// envelope; in the SSE case every `data:` line is concatenated and
/// parsed as a single JSON document.
///
/// Used for the unary-JSON path in [`super::Client::connect`] and for
/// every subsequent RPC. The streaming variant
/// ([`read_next_sse_event`]) is used when the caller wants to keep the
/// underlying SSE stream alive for further events after the first one.
pub(crate) async fn parse_streamable_http_response<T: serde::de::DeserializeOwned>(
    url: &str,
    response: reqwest::Response,
) -> Result<T, super::Error> {
    let bytes = response.bytes().await.map_err(|source| super::Error::Request {
        url: url.to_string(),
        source,
    })?;
    if let Ok(v) = serde_json::from_slice::<T>(&bytes) {
        return Ok(v);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        super::Error::MalformedResponse {
            url: url.to_string(),
            message: "response body is not valid UTF-8".into(),
        }
    })?;
    let collected: String = text
        .lines()
        .filter_map(|l| l.strip_prefix("data: ").or_else(|| l.strip_prefix("data:")))
        .collect();
    serde_json::from_str(&collected).map_err(|e| {
        super::Error::MalformedResponse {
            url: url.to_string(),
            message: format!(
                "neither JSON nor SSE-wrapped JSON: {e}; body starts with: {}",
                text.chars().take(200).collect::<String>(),
            ),
        }
    })
}
