//! Terminals the provider holds for the caller: each a terminal scope
//! on the proxy, by an id of this end's, until it is closed.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use bytes::Bytes;
use futures_util::StreamExt as _;
use serde_json::json;
use tokio::sync::{Mutex, mpsc};

use super::super::encoded::encoded;
use super::super::render;
use super::super::run::Run;
use diverge_sdk::container_proxy::outside::endpoints::terminal::client::execute as terminal;
use diverge_sdk::shared::containers::terminal::open::Open;
use diverge_sdk::shared::containers::terminal::output::{Frame, Owned};
use diverge_sdk::shared::containers::terminal::session::{Answer, Opened, SCROLLBACK};
use diverge_sdk::shared::error::Error;

/// The terminals a run holds, by id.
#[derive(Default)]
pub(crate) struct Terminals {
    next: Mutex<u32>,
    open: Mutex<HashMap<u32, Arc<Session>>>,
}

impl Terminals {
    /// Stop every terminal: what a run's shutdown does.
    pub(crate) async fn stop_all(&self) {
        for (_, session) in self.open.lock().await.drain() {
            let _ = session.input.stop().await;
        }
    }

    async fn get(&self, id: u32) -> Option<Arc<Session>> {
        self.open.lock().await.get(&id).cloned()
    }
}

/// One terminal: the way into it on the proxy, and what its output is
/// doing.
struct Session {
    input: terminal::ExecuteHandle,
    state: Mutex<State>,
}

/// Where a terminal's output goes: to the one attach, or into the
/// scrollback while there is none; and how its shell ended, once it
/// has.
#[derive(Default)]
struct State {
    scrollback: VecDeque<Bytes>,
    held: usize,
    attached: Option<(u64, mpsc::Sender<Owned>)>,
    attaches: u64,
    ended: Option<Owned>,
}

impl State {
    /// Keep `bytes` in the scrollback, the oldest let go past
    /// [`SCROLLBACK`].
    fn keep(&mut self, bytes: Bytes) {
        self.held += bytes.len();
        self.scrollback.push_back(bytes);
        while self.held > SCROLLBACK
            && let Some(oldest) = self.scrollback.pop_front()
        {
            self.held -= oldest.len();
        }
    }
}

/// `terminal_open`: a terminal scope on the proxy at `size`, held under
/// a fresh id, its output pumped until the shell ends; answered with the
/// id, or the proxy's refusal.
pub(crate) async fn open(run: Arc<Run>, channel: u32, size: Open) {
    let answer = match terminal::execute(&run.proxy, size).await {
        Err(error) => Opened::Error(render::proxy(error)),
        Ok((output, input)) => {
            let session = Arc::new(Session { input, state: Mutex::new(State::default()) });
            let id = {
                let mut next = run.terminals.next.lock().await;
                *next = next.wrapping_add(1);
                *next
            };
            run.terminals.open.lock().await.insert(id, Arc::clone(&session));
            run.spawn(pump(session, output)).await;
            Opened::Opened(id)
        }
    };
    run.respond(channel, encoded(&answer)).await;
    run.finish(channel).await;
}

/// Every frame of a terminal's output, to its attach, paced by it, or
/// into its scrollback; the end kept for whoever attaches next.
async fn pump(session: Arc<Session>, mut output: terminal::ExecuteStream) {
    loop {
        let frame = match output.next().await {
            Some(Ok(frame)) => frame,
            // The proxy's connection ended: the container stopped.
            Some(Err(terminal::ExecuteStreamError::Closed)) | None => Owned::Error(Error(json!({ "kind": "terminal", "error": "the container stopped" }))),
            Some(Err(error)) => Owned::Error(render::proxy(error)),
        };
        let last = !matches!(frame, Owned::Stdout(_));
        let attached = {
            let mut state = session.state.lock().await;
            if last {
                state.ended = Some(frame.clone());
            }
            state.attached.as_ref().map(|(_, sender)| sender.clone())
        };
        match attached {
            Some(sender) => {
                if let Err(unsent) = sender.send(frame).await
                    && let Owned::Stdout(bytes) = unsent.0
                {
                    session.state.lock().await.keep(bytes);
                }
            }
            None => {
                if let Owned::Stdout(bytes) = frame {
                    session.state.lock().await.keep(bytes);
                }
            }
        }
        if last {
            return;
        }
    }
}

/// `terminal_attach`: the scrollback, then the output as it comes, then
/// how the shell ended, and the finish. The caller abandoning the
/// channel detaches; a later attach takes this one's place.
pub(crate) async fn attach(run: Arc<Run>, channel: u32, id: u32) {
    let Some(session) = run.terminals.get(id).await else {
        run.respond(channel, encoded(&Frame::Error(unknown(id)))).await;
        return run.finish(channel).await;
    };
    let (sender, mut receiver) = mpsc::channel(1);
    let (mine, scrollback, ended) = {
        let mut state = session.state.lock().await;
        state.attaches += 1;
        let mine = state.attaches;
        state.attached = Some((mine, sender));
        state.held = 0;
        (mine, std::mem::take(&mut state.scrollback), state.ended.clone())
    };
    for bytes in scrollback {
        run.respond(channel, encoded(&Frame::Stdout(&bytes))).await;
    }
    if let Some(ended) = ended {
        run.respond(channel, encoded(&ended.as_frame())).await;
        return run.finish(channel).await;
    }
    loop {
        tokio::select! {
            frame = receiver.recv() => match frame {
                Some(frame) => {
                    let last = !matches!(frame, Owned::Stdout(_));
                    run.respond(channel, encoded(&frame.as_frame())).await;
                    if last {
                        break;
                    }
                }
                // Another attach took this one's place.
                None => break,
            },
            () = run.scope.answer_gone(channel) => {
                let mut state = session.state.lock().await;
                if state.attached.as_ref().is_some_and(|(attach, _)| *attach == mine) {
                    state.attached = None;
                }
                break;
            }
        }
    }
    run.finish(channel).await;
}

/// `terminal_stdin`: the bytes written, answered once they are.
pub(crate) async fn stdin(run: Arc<Run>, channel: u32, id: u32, bytes: Vec<u8>) {
    let answer = match run.terminals.get(id).await {
        None => Answer::Error(unknown(id)),
        Some(session) => match session.input.stdin(&bytes).await {
            Ok(()) => Answer::Done,
            Err(error) => Answer::Error(Error(json!({ "kind": "terminal", "error": error.to_string() }))),
        },
    };
    run.respond(channel, encoded(&answer)).await;
    run.finish(channel).await;
}

/// `terminal_resize`: the size set.
pub(crate) async fn resize(run: Arc<Run>, channel: u32, id: u32, size: Open) {
    let answer = match run.terminals.get(id).await {
        None => Answer::Error(unknown(id)),
        Some(session) => match session.input.resize(size).await {
            Ok(()) => Answer::Done,
            Err(error) => Answer::Error(Error(json!({ "kind": "terminal", "error": error.to_string() }))),
        },
    };
    run.respond(channel, encoded(&answer)).await;
    run.finish(channel).await;
}

/// `terminal_close`: the shell killed and the terminal forgotten.
pub(crate) async fn close(run: Arc<Run>, channel: u32, id: u32) {
    let answer = match run.terminals.open.lock().await.remove(&id) {
        None => Answer::Error(unknown(id)),
        Some(session) => {
            let _ = session.input.stop().await;
            Answer::Done
        }
    };
    run.respond(channel, encoded(&answer)).await;
    run.finish(channel).await;
}

/// The error for an id that names no open terminal.
fn unknown(id: u32) -> Error {
    Error(json!({ "kind": "terminal", "error": format!("no terminal {id} is open") }))
}
