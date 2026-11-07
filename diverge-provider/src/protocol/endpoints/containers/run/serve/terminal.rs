//! Terminals the provider holds for the caller: each a terminal scope
//! on the proxy, by an id of this end's, until it is closed or its
//! end has been told.

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

/// How many ended terminals a run keeps the end of for an attach yet
/// to come, the oldest let go past it.
const ENDED: usize = 64;

/// The terminals a run holds, by id.
///
/// # What is kept, and for how long
///
/// 1. A terminal whose shell runs is held whole — the way into it,
///    its scrollback, its attach — until it is closed, or the run
///    shuts down.
/// 2. A terminal whose shell ended while attached has had its end
///    told: the end is handed to the attach, and the terminal is
///    forgotten there and then.
/// 3. A terminal whose shell ended with nobody attached is let go of
///    — scrollback, pump and the way in — and only its end is kept,
///    the few words an attach yet to come is told before its finish;
///    the attach that is told them forgets them.
/// 4. At most [`ENDED`] ends are kept: one past it lets the oldest
///    go, and an attach to that id is then an id that names nothing.
/// 5. A close forgets the terminal either way, and the run's shutdown
///    forgets them all.
#[derive(Default)]
pub(crate) struct Terminals {
    next: Mutex<u32>,
    open: Mutex<Held>,
}

/// The terminals running, and the ends of those that ended unheard.
#[derive(Default)]
struct Held {
    running: HashMap<u32, Arc<Session>>,
    ended: VecDeque<(u32, Owned)>,
}

impl Held {
    /// Take the kept end of `id`, if one is kept.
    fn take_ended(&mut self, id: u32) -> Option<Owned> {
        let position = self.ended.iter().position(|(ended, _)| *ended == id)?;
        self.ended.remove(position).map(|(_, frame)| frame)
    }
}

impl Terminals {
    /// Stop every terminal and forget every end: what a run's shutdown
    /// does. Rule 5.
    pub(crate) async fn stop_all(&self) {
        let running = {
            let mut held = self.open.lock().await;
            held.ended.clear();
            std::mem::take(&mut held.running)
        };
        for (_, session) in running {
            let _ = session.input.cancel().await;
        }
    }

    async fn get(&self, id: u32) -> Option<Arc<Session>> {
        self.open.lock().await.running.get(&id).cloned()
    }

    /// The shell of `id` ended with `frame`, which reached an attach
    /// when `told`: the terminal let go of, and its end kept for an
    /// attach to come when no attach had it. Rules 2, 3 and 4. A
    /// terminal already closed, or already forgotten by an attach that
    /// read its end off the state, keeps nothing.
    async fn ended(&self, id: u32, frame: Owned, told: bool) {
        let mut held = self.open.lock().await;
        if held.running.remove(&id).is_some() && !told {
            if held.ended.len() >= ENDED {
                held.ended.pop_front();
            }
            held.ended.push_back((id, frame));
        }
    }

    /// The end of `id` has been told: nothing of it is kept.
    async fn forget(&self, id: u32) {
        let mut held = self.open.lock().await;
        held.running.remove(&id);
        held.take_ended(id);
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
/// id, or the proxy's refusal. An id still running or with its end
/// still kept is never handed out again while it is.
pub(crate) async fn open(run: Arc<Run>, channel: u32, size: Open) {
    let answer = match terminal::execute(&run.proxy, size).await {
        Err(error) => Opened::Error(render::proxy(error)),
        Ok((output, input)) => {
            let session = Arc::new(Session { input, state: Mutex::new(State::default()) });
            let id = {
                let mut next = run.terminals.next.lock().await;
                let mut held = run.terminals.open.lock().await;
                loop {
                    *next = next.wrapping_add(1);
                    if !held.running.contains_key(&*next) && !held.ended.iter().any(|(ended, _)| *ended == *next) {
                        break;
                    }
                }
                held.running.insert(*next, Arc::clone(&session));
                *next
            };
            run.spawn(pump(Arc::clone(&run), id, session, output)).await;
            Opened::Opened(id)
        }
    };
    run.respond(channel, encoded(&answer)).await;
    run.finish(channel).await;
}

/// Every frame of a terminal's output, to its attach, paced by it, or
/// into its scrollback; the end handed to the attach, or kept for
/// whoever attaches next — see [`Terminals`], rules 2 to 4.
async fn pump(run: Arc<Run>, id: u32, session: Arc<Session>, mut output: terminal::ExecuteStream) {
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
        if last {
            let told = match attached {
                Some(sender) => sender.send(frame.clone()).await.is_ok(),
                None => false,
            };
            return run.terminals.ended(id, frame, told).await;
        }
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
    }
}

/// `terminal_attach`: the scrollback, then the output as it comes, then
/// how the shell ended, and the finish. The caller abandoning the
/// channel detaches; a later attach takes this one's place. A
/// terminal whose shell ended unheard is told by its kept end alone,
/// and an attach that tells an end forgets the terminal — see
/// [`Terminals`].
pub(crate) async fn attach(run: Arc<Run>, channel: u32, id: u32) {
    let session = {
        let mut held = run.terminals.open.lock().await;
        match held.running.get(&id).cloned() {
            Some(session) => Ok(session),
            None => Err(held.take_ended(id)),
        }
    };
    let session = match session {
        Ok(session) => session,
        Err(Some(ended)) => {
            run.respond(channel, encoded(&ended.as_frame())).await;
            return run.finish(channel).await;
        }
        Err(None) => {
            run.respond(channel, encoded(&Frame::Error(unknown(id)))).await;
            return run.finish(channel).await;
        }
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
        run.terminals.forget(id).await;
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

/// `terminal_close`: the shell killed and the terminal forgotten; a
/// terminal whose shell already ended has its kept end forgotten.
pub(crate) async fn close(run: Arc<Run>, channel: u32, id: u32) {
    let found = {
        let mut held = run.terminals.open.lock().await;
        match held.running.remove(&id) {
            Some(session) => Some(Some(session)),
            None => held.take_ended(id).map(|_| None),
        }
    };
    let answer = match found {
        None => Answer::Error(unknown(id)),
        Some(session) => {
            if let Some(session) = session {
                let _ = session.input.cancel().await;
            }
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
