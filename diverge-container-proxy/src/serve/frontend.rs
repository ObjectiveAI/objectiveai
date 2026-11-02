//! A frontend request: one HTTP request to the program's server,
//! beneath `/frontend`, its answer streamed and its body taken.

use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::CHUNK_SIZE;
use diverge_sdk::container_proxy::inside::port;
use diverge_sdk::container_proxy::outside::endpoints::frontend::client::{channel_request, request};
use diverge_sdk::container_proxy::outside::endpoints::frontend::server::response::Frame;
use diverge_sdk::shared::containers::frontend::output::Head;
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::{Notify, mpsc};

use crate::encode::encoded;
use crate::info::PATH;

/// What the client put in after the head: a piece of the body, or its
/// end.
enum Piece {
    Body(Bytes),
    End,
}

/// Serve one frontend request, as
/// [`frontend`](diverge_sdk::shared::containers::frontend) states.
///
/// The rules:
///
/// 1. The image's [`PATH`] declares `frontend: true`, and the path is
///    [`reachable`](diverge_sdk::shared::containers::frontend::request::Head::reachable);
///    otherwise one `Error`, and the finish.
/// 2. The request goes to the program's server on the loopback at
///    [`port`], with the head's method, path and headers, its `Host`
///    the server's own. It follows no redirect and goes through no
///    proxy. A request whose head carries `Content-Length` or
///    `Transfer-Encoding` has a body: the client's `Body` channels,
///    in their order, up to `End`. Any other has none.
/// 3. Each channel is finished once its piece is taken; a `Stop`, the
///    client gone, or the responses abandoned drops the request, and
///    only the finish follows.
/// 4. The answer is one `Head`, its status and its headers as the
///    program sent them, then its body in `Body` frames of at most
///    [`CHUNK_SIZE`], and the finish. A body broken off is one `Error`
///    after what was sent. A program not reached is one `Error`.
/// 5. A `101` to a request with no body is an upgrade: what the program
///    sends goes out as `Body` frames, and the client's `Body` pieces
///    are written to it, `End` closing this side, until the program
///    closes its side.
pub async fn frontend(scope: ScopeHandle, frame: request::Frame) {
    let scope = Arc::new(scope);
    let head = frame.0;
    // Rule 1.
    if !declared().await {
        return error(&scope, "frontend", "the image declares no frontend").await;
    }
    if !head.reachable() {
        return error(&scope, "path", "the path is not beneath /frontend").await;
    }
    let method = match reqwest::Method::from_bytes(head.method.as_bytes()) {
        Ok(method) => method,
        Err(_) => return error(&scope, "method", "the method is not a method").await,
    };
    let mut headers = reqwest::header::HeaderMap::new();
    for (name, value) in &head.headers {
        let (Ok(name), Ok(value)) = (reqwest::header::HeaderName::from_bytes(name.as_bytes()), reqwest::header::HeaderValue::from_str(value)) else {
            return error(&scope, "header", &format!("the header {name} is not a header")).await;
        };
        if name != reqwest::header::HOST {
            headers.append(name, value);
        }
    }
    let has_body = headers.contains_key(reqwest::header::CONTENT_LENGTH) || headers.contains_key(reqwest::header::TRANSFER_ENCODING);

    // Rule 3, on its own task.
    let (pieces, taken) = mpsc::channel::<Piece>(1);
    let stopped = Arc::new(Notify::new());
    let input = tokio::spawn(take_input(Arc::clone(&scope), pieces, Arc::clone(&stopped)));

    // Rule 2.
    let client = match reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).no_proxy().build() {
        Ok(client) => client,
        Err(reason) => {
            input.abort();
            return error(&scope, "client", &reason.to_string()).await;
        }
    };
    let mut builder = client.request(method, format!("http://127.0.0.1:{}{}", port(), head.path)).headers(headers);
    let mut kept = None;
    if has_body {
        let body = futures_util::stream::unfold(taken, |mut taken| async move {
            match taken.recv().await {
                Some(Piece::Body(bytes)) => Some((Ok::<Bytes, std::io::Error>(bytes), taken)),
                Some(Piece::End) | None => None,
            }
        });
        builder = builder.body(reqwest::Body::wrap_stream(body));
    } else {
        kept = Some(taken);
    }
    let response = tokio::select! {
        response = builder.send() => response,
        () = stopped.notified() => return end(&scope, input).await,
        () = scope.responses_gone() => return end(&scope, input).await,
    };
    let response = match response {
        Ok(response) => response,
        Err(reason) => {
            input.abort();
            return error(&scope, "unreached", &reason.to_string()).await;
        }
    };

    // Rule 4.
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| (name.as_str().to_owned(), String::from_utf8_lossy(value.as_bytes()).into_owned()))
        .collect();
    send(&scope, &Frame::Head(Head { status, headers })).await;

    // Rule 5.
    if status == 101
        && let Some(taken) = kept.take()
    {
        let upgraded = match response.upgrade().await {
            Ok(upgraded) => upgraded,
            Err(reason) => {
                send(&scope, &Frame::Error(failure("upgrade", &reason.to_string()))).await;
                return end(&scope, input).await;
            }
        };
        return upgrade(&scope, upgraded, taken, &stopped, input).await;
    }
    // A request with no body takes no pieces: what comes is let go.
    drop(kept);
    let mut body = response.bytes_stream();
    loop {
        tokio::select! {
            piece = body.next() => match piece {
                Some(Ok(bytes)) => {
                    for chunk in bytes.chunks(CHUNK_SIZE) {
                        send(&scope, &Frame::Body(chunk)).await;
                    }
                }
                Some(Err(reason)) => {
                    send(&scope, &Frame::Error(failure("body", &reason.to_string()))).await;
                    break;
                }
                None => break,
            },
            () = stopped.notified() => break,
            () = scope.responses_gone() => break,
        }
    }
    end(&scope, input).await;
}

/// Rule 5: both ways at once, until the program closes its side, a
/// stop, or the client gone.
async fn upgrade(scope: &ScopeHandle, upgraded: reqwest::Upgraded, mut taken: mpsc::Receiver<Piece>, stopped: &Notify, input: tokio::task::JoinHandle<()>) {
    let (mut reading, mut writing) = tokio::io::split(upgraded);
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut open = true;
    loop {
        tokio::select! {
            read = reading.read(&mut buffer) => match read {
                Ok(n) if n > 0 => send(scope, &Frame::Body(&buffer[..n])).await,
                _ => break,
            },
            piece = taken.recv(), if open => match piece {
                Some(Piece::Body(bytes)) => {
                    if writing.write_all(&bytes).await.is_err() {
                        open = false;
                    }
                }
                Some(Piece::End) | None => {
                    let _ = writing.shutdown().await;
                    open = false;
                }
            },
            () = stopped.notified() => break,
            () = scope.responses_gone() => break,
        }
    }
    end(scope, input).await;
}

/// Rule 3: every channel request in order, each finished once its
/// piece is taken, until a stop or the client is gone, either of which
/// tells `stopped`.
async fn take_input(scope: Arc<ScopeHandle>, pieces: mpsc::Sender<Piece>, stopped: Arc<Notify>) {
    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Body(bytes)) => {
                // A piece nobody takes — a request with no body — is let go.
                let _ = pieces.send(Piece::Body(Bytes::copy_from_slice(bytes))).await;
            }
            Ok(channel_request::Frame::End) => {
                let _ = pieces.send(Piece::End).await;
            }
            Ok(channel_request::Frame::Stop) => {
                scope.send_channel_response_finish(channel).await;
                break;
            }
            Err(_) => {}
        }
        scope.send_channel_response_finish(channel).await;
    }
    stopped.notify_one();
}

/// Whether the image's document declares a frontend. No document, or
/// one that does not parse, declares none.
async fn declared() -> bool {
    match tokio::fs::read_to_string(PATH).await {
        Ok(text) => Document::parse(&text).is_ok_and(|document| document.frontend),
        Err(_) => false,
    }
}

/// The input let go, and the finish.
async fn end(scope: &ScopeHandle, input: tokio::task::JoinHandle<()>) {
    input.abort();
    scope.send_response_finish().await;
}

/// One frame, as the scope's response.
async fn send(scope: &ScopeHandle, frame: &Frame<'_>) {
    if let Some(payload) = encoded(frame) {
        scope.send_response(&payload).await;
    }
}

/// An error in the proxy's words.
fn failure(kind: &str, reason: &str) -> Error {
    Error(serde_json::json!({ "kind": kind, "error": reason }))
}

/// Rules 1 and 4: the error, then the finish.
async fn error(scope: &ScopeHandle, kind: &str, reason: &str) {
    send(scope, &Frame::Error(failure(kind, reason))).await;
    scope.send_response_finish().await;
}
