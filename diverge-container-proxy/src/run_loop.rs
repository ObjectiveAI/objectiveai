//! The loop, relayed: the server's `/run-loop` on one side, the
//! harness's `/run-loop/agent` on the other, the proxy between.
//!
//! The proxy runs no loop. The harness beside it does, and attaches
//! here to be handed the request and to stream what the loop says;
//! the server opens the other side to hand the request over and to
//! read the stream. The proxy carries the request one way and every
//! frame the other, verbatim — it decodes none of it — and tells each
//! side how the other ended.
//!
//! # Whoever is first waits
//!
//! A harness that attaches before any server has asked waits for the
//! request; a server that asks before any harness has attached waits
//! for one. Nothing times anything out, and whichever side leaves
//! ends the other's wait. One run at a time: a second `/run-loop`
//! while one is in progress is `409`, and so is a second harness
//! while one is attached.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use diverge_provider_sdk::encode::{Encode as _, Writer};
use diverge_provider_sdk::shared::containers::run_loop::response;
use diverge_provider_sdk::shared::error::Error;
use futures_util::future;
use futures_util::{SinkExt as _, StreamExt as _};
use std::pin::pin;
use tokio::sync::{Notify, mpsc};

/// Where the server and the harness meet.
pub struct RunLoop {
    /// The attached harness's inbox, while one is attached.
    harness: Mutex<Option<mpsc::Sender<Run>>>,
    /// Rung when a harness attaches, for a server already waiting.
    attached: Notify,
    /// Whether a server holds `/run-loop`.
    running: Mutex<bool>,
}

/// One run, handed to the harness: the request, and where its frames
/// go back.
struct Run {
    request: Bytes,
    frames: mpsc::UnboundedSender<Relay>,
}

/// What the harness's side tells the server's.
enum Relay {
    /// One frame, verbatim.
    Frame(Bytes),
    /// The harness closed cleanly: the loop is over.
    Finished,
    /// The harness vanished: the loop died.
    Died,
}

/// A server holding `/run-loop`; released on drop.
pub struct Claim {
    run_loop: Arc<RunLoop>,
}

/// A harness attached at `/run-loop/agent`; detached on drop.
pub struct Attachment {
    run_loop: Arc<RunLoop>,
    runs: mpsc::Receiver<Run>,
}

impl RunLoop {
    pub fn new() -> Self {
        RunLoop {
            harness: Mutex::new(None),
            attached: Notify::new(),
            running: Mutex::new(false),
        }
    }

    /// Take `/run-loop`, or learn that a run is in progress.
    fn claim(self: &Arc<Self>) -> Option<Claim> {
        let mut running = self.running.lock().ok()?;
        if *running {
            return None;
        }
        *running = true;
        Some(Claim {
            run_loop: Arc::clone(self),
        })
    }

    /// Attach a harness, or learn that one is attached.
    fn attach(self: &Arc<Self>) -> Option<Attachment> {
        let mut harness = self.harness.lock().ok()?;
        if harness.is_some() {
            return None;
        }
        let (sender, runs) = mpsc::channel(1);
        *harness = Some(sender);
        drop(harness);
        self.attached.notify_waiters();
        Some(Attachment {
            run_loop: Arc::clone(self),
            runs,
        })
    }

    /// Hand a run to the attached harness, if there is one and it is
    /// listening; the run comes back otherwise.
    fn offer(&self, run: Run) -> Result<(), Run> {
        let sender = match self.harness.lock() {
            Ok(harness) => harness.clone(),
            Err(_) => None,
        };
        let Some(sender) = sender else {
            return Err(run);
        };
        match sender.try_send(run) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(run)) => Err(run),
            Err(mpsc::error::TrySendError::Closed(run)) => {
                // The harness left without detaching cleanly yet;
                // its slot is free for the next.
                if let Ok(mut harness) = self.harness.lock() {
                    *harness = None;
                }
                Err(run)
            }
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if let Ok(mut running) = self.run_loop.running.lock() {
            *running = false;
        }
    }
}

impl Drop for Attachment {
    fn drop(&mut self) {
        if let Ok(mut harness) = self.run_loop.harness.lock() {
            *harness = None;
        }
    }
}

/// `/run-loop`: the server asks for the loop. Refused with `409`
/// while a run is in progress, before the upgrade.
pub async fn server(
    State(run_loop): State<Arc<RunLoop>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let Some(claim) = run_loop.claim() else {
        return StatusCode::CONFLICT.into_response();
    };
    upgrade
        .on_upgrade(move |socket| serve_server(socket, run_loop, claim))
        .into_response()
}

/// Serve the server's side: the request in, a harness found, the
/// frames out.
///
/// The request is the first binary message; anything else is the
/// clean close with nothing before it. Then the harness: the attached
/// one, or the next to attach, the wait watching the server's socket
/// so a server that leaves is not waited for. Then the relay: every
/// frame the harness sends goes out as one binary message; the
/// harness finishing closes cleanly; the harness dying sends one
/// `Error` and closes; the server leaving drops the relay, which the
/// harness's side notices on its next frame.
async fn serve_server(socket: WebSocket, run_loop: Arc<RunLoop>, _claim: Claim) {
    let (mut sink, mut stream) = socket.split();
    let request = match stream.next().await {
        Some(Ok(Message::Binary(bytes))) => bytes,
        _ => {
            let _ = sink.close().await;
            return;
        }
    };

    let (frames, mut relay) = mpsc::unbounded_channel();
    let mut run = Run { request, frames };
    loop {
        // Arm the notification BEFORE looking, so a harness that
        // attaches between the look and the wait is not missed.
        let notified = run_loop.attached.notified();
        let mut notified = pin!(notified);
        notified.as_mut().enable();
        match run_loop.offer(run) {
            Ok(()) => break,
            Err(back) => run = back,
        }
        let reading = pin!(stream.next());
        match future::select(notified, reading).await {
            future::Either::Left(_) => {}
            future::Either::Right((Some(Ok(Message::Ping(_) | Message::Pong(_))), _)) => {}
            future::Either::Right(_) => return,
        }
    }

    loop {
        let next = pin!(relay.recv());
        let reading = pin!(stream.next());
        match future::select(next, reading).await {
            future::Either::Left((Some(Relay::Frame(bytes)), _)) => {
                if sink.send(Message::Binary(bytes)).await.is_err() {
                    return;
                }
            }
            future::Either::Left((Some(Relay::Finished), _)) => {
                let _ = sink.close().await;
                return;
            }
            future::Either::Left((Some(Relay::Died), _)) | future::Either::Left((None, _)) => {
                let error = response::Frame::Error(Error(serde_json::json!({
                    "kind": "loop",
                    "error": "the loop ended without finishing",
                })));
                let mut bytes = Vec::new();
                if error.encode(&mut Writer::new(&mut bytes)).is_ok() {
                    let _ = sink.send(Message::Binary(bytes.into())).await;
                }
                let _ = sink.close().await;
                return;
            }
            future::Either::Right((Some(Ok(Message::Ping(_) | Message::Pong(_))), _)) => {}
            future::Either::Right(_) => return,
        }
    }
}

/// `/run-loop/agent`: the harness attaches. Refused with `409` while
/// one is attached, before the upgrade.
pub async fn harness(
    State(run_loop): State<Arc<RunLoop>>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let Some(attachment) = run_loop.attach() else {
        return StatusCode::CONFLICT.into_response();
    };
    upgrade
        .on_upgrade(move |socket| serve_harness(socket, attachment))
        .into_response()
}

/// Serve the harness's side: wait for a run, hand it the request,
/// relay what it says.
///
/// The wait watches the harness's socket, so a harness that leaves
/// is detached rather than handed a run nobody will serve. Once the
/// request is sent, every binary message the harness sends is a
/// frame for the server; its clean close is the loop finished; its
/// socket ending any other way is the loop dead. A server that has
/// gone shows as the relay refusing a frame, at which the harness's
/// socket is closed.
async fn serve_harness(mut socket: WebSocket, mut attachment: Attachment) {
    let run = loop {
        let next = pin!(attachment.runs.recv());
        let reading = pin!(socket.next());
        match future::select(next, reading).await {
            future::Either::Left((Some(run), _)) => break run,
            future::Either::Left((None, _)) => return,
            future::Either::Right((Some(Ok(Message::Ping(_) | Message::Pong(_))), _)) => {}
            future::Either::Right(_) => return,
        }
    };

    if socket.send(Message::Binary(run.request)).await.is_err() {
        let _ = run.frames.send(Relay::Died);
        return;
    }
    loop {
        match socket.next().await {
            Some(Ok(Message::Binary(bytes))) => {
                if run.frames.send(Relay::Frame(bytes)).is_err() {
                    // The server left; the loop's words have nowhere
                    // to go.
                    let _ = socket.close().await;
                    return;
                }
            }
            Some(Ok(Message::Close(_))) => {
                let _ = run.frames.send(Relay::Finished);
                return;
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            _ => {
                let _ = run.frames.send(Relay::Died);
                return;
            }
        }
    }
}
