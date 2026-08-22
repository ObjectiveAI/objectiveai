//! Running an agent in a container, and relaying both directions.

use std::pin::pin;

use bytes::Bytes;
use eventsource_stream::Eventsource as _;
use futures_util::future::{self, Either};
use futures_util::{SinkExt as _, StreamExt as _};
use http_body_util::{BodyStream, Full};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use super::super::{channel_request, response};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::agentic_loop::run::client::request;
use crate::endpoints::agentic_loop::run::client::request::agent::Agent;
use crate::frame::client::ClientFrame;
use crate::server::channel::Channel;
use crate::server::container::Container;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::deployment::Deployment;
use crate::server::mcp_conduit;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::error::Error;
use crate::shared::http;

/// Run the loop and relay it, until it ends or the caller goes.
///
/// Two directions at once, which is what makes this the largest
/// handler. Chunks come out of the container and go down to the caller;
/// tool calls come out of the container the other way and go out on
/// channels, and the caller's answers come back and go in. Neither
/// waits for the other, because neither is in the other's loop.
///
/// # It takes no `client_identity`
///
/// Alone among the handlers. This endpoint's request carries no
/// identity and a [`ContainerDeployer`] takes none, so there is nothing
/// an identity would be used for — the container gets no mounts, no
/// volumes and no environment, so there is no namespace to resolve it
/// against.
///
/// # The request is forwarded verbatim
///
/// The bytes the caller sent, minus the tag that said which request it
/// was. It is decoded here only to work out what to deploy, and what
/// goes into the container is the original JSON rather than a
/// re-serialization of what came out of the decoder — so a field this
/// crate does not know about survives the trip.
///
/// # The container is stopped on every path
///
/// Including the ones that failed, and including the caller simply
/// leaving. There is no destructor doing it, because teardown in this
/// crate is a method; there is one call, after the loop, and every exit
/// goes through it.
pub async fn handle<D>(mut scope: ScopeHandle, deployer: &D)
where
    D: ContainerDeployer,
    D::Error: Into<Error>,
    <D::Container as Container>::Error: Into<Error>,
{
    let (agent, body) = match request::Frame::decode(scope.request()) {
        // The payload is a tag byte and then the request's own JSON, so
        // what the container should be handed is everything after the
        // first byte.
        Ok(frame) => (
            frame.agent.clone(),
            Bytes::copy_from_slice(&scope.request()[1..]),
        ),
        Err(error) => {
            let error = Error(Value::String(error.to_string()));
            write(&mut scope, &response::Frame::Error(error)).await;
            scope.send_response_finish().await;
            return;
        }
    };

    let deployment = Deployment {
        memory: memory(&agent),
        disk: disk(&agent),
        environment: Default::default(),
        // An agent works in the container it was given. Nothing of the
        // caller's is mounted into it.
        mounts: Vec::new(),
        // The loop, then the MCP conduit — in that order, and the
        // `connect` calls below have to agree with it.
        // TODO: settled when the images are.
        ports: vec![8080, 8081],
    };

    let container = match deployer.registry(&deployment, image(&agent)).await {
        Ok(container) => container,
        Err(error) => {
            write(&mut scope, &response::Frame::Error(error.into())).await;
            scope.send_response_finish().await;
            return;
        }
    };

    relay(&mut scope, &container, body).await;

    container.stop().await;
    scope.send_response_finish().await;
}

/// Which image runs this agent.
///
/// Every upstream is a different program with a different way of being
/// driven, so it is a different image — and which one is not the
/// caller's to choose, since a caller that could name an image could
/// name any image.
fn image(agent: &Agent) -> &'static str {
    match agent {
        // TODO: none of these images are published yet.
        Agent::Openrouter(_) => "TODO",
        Agent::ClaudeAgentSdk(_) => "TODO",
        Agent::CodexSdk(_) => "TODO",
        Agent::Python(_) => "TODO",
    }
}

/// How much memory this agent's container may have, in bytes.
///
/// # Three of them are the image's number and one is the caller's
///
/// Which is the asymmetry worth understanding. For a model-backed
/// agent, the image is a known program doing a known job, and what it
/// needs is a property of the image — so this is the only place that
/// could know, and it says.
///
/// A [`Python`](Agent::Python) agent is the other way round: the image
/// is an interpreter, and the thing that actually runs arrived with the
/// request. Its author is the only party who knows, which is why
/// [`python::Agent::memory`](crate::endpoints::agentic_loop::run::client::request::agent::python::Agent::memory)
/// exists and says so in the same terms — a provider "cannot infer it:
/// the source is opaque until it runs, and by then the number is
/// already needed".
///
/// So a limit belongs to whoever knows what is going to run. Three
/// times that is us and once it is the caller, and reading the caller's
/// number is not a courtesy — ignoring it means the kernel kills a
/// script that said what it needed.
fn memory(agent: &Agent) -> u64 {
    match agent {
        // TODO: numbers nobody has justified, and which belong with the
        // images once those exist.
        Agent::Openrouter(_) => 512 * 1024 * 1024,
        Agent::ClaudeAgentSdk(_) => 2 * 1024 * 1024 * 1024,
        Agent::CodexSdk(_) => 2 * 1024 * 1024 * 1024,
        Agent::Python(agent) => agent.memory,
    }
}

/// How much this agent's container may write, in bytes.
///
/// The same split [`memory`] makes, for the same reason: three images
/// whose appetite is a property of the image, and one whose is a
/// property of what the caller sent. See
/// [`python::Agent::disk`](crate::endpoints::agentic_loop::run::client::request::agent::python::Agent::disk).
fn disk(agent: &Agent) -> u64 {
    match agent {
        // TODO: as above.
        Agent::Openrouter(_) => 256 * 1024 * 1024,
        Agent::ClaudeAgentSdk(_) => 4 * 1024 * 1024 * 1024,
        Agent::CodexSdk(_) => 4 * 1024 * 1024 * 1024,
        Agent::Python(agent) => agent.disk,
    }
}

/// Something a worker wants done on the scope.
///
/// The workers do not hold the [`ScopeHandle`] and cannot: this half's
/// writing belongs to the individual scope, and there is one of it. So
/// they ask, over a queue, and the task that owns the scope does it.
///
/// Which also settles the thing a mutex could not. Watching for the
/// caller leaving means awaiting
/// [`recv_channel_request`](ScopeHandle::recv_channel_request), and a
/// mutex held across that would be held for the whole run — starving
/// exactly the writers it was there to let cooperate.
enum Write {
    /// A frame for the scope's own stream.
    Response(Vec<u8>),
    /// Open a channel, and hand the channel back.
    ///
    /// The reply is what makes this different from the others: a
    /// [`Channel`] is what a request produces, only the scope may
    /// produce one, and the worker is what has to read it.
    ChannelRequest(Vec<u8>, oneshot::Sender<Channel>),
    /// Nothing more is coming.
    ///
    /// Sent rather than inferred from the queue closing, because the
    /// two workers hold a sender each: one of them finishing leaves the
    /// other's alive, and a scope waiting for both would outlive the
    /// loop it was reporting.
    Done,
}

/// Drive the container until the loop ends or the caller goes.
///
/// Three parts: a task that asks the agent and reads its stream, a task
/// working the MCP conduit, and this, which owns the scope and does
/// every write either of them asks for.
///
/// # The conduit is open before the request is sent
///
/// Which is the ordering that matters and the reason the request is not
/// sent from here. A container is free to want a tool call while it is
/// still working out how to answer, so the pipe it would ask on has to
/// be dialled first — and the loop below has to already be RUNNING,
/// because an answer needs a channel opened, and only this task can
/// open one.
///
/// A version that sent the request here and then started the loop
/// deadlocks on exactly that: the container waits for its tool answer,
/// this waits for the container's head, and the queue that would have
/// resolved it is not being read by anyone.
async fn relay<C>(scope: &mut ScopeHandle, container: &C, body: Bytes)
where
    C: Container,
    C::Error: Into<Error>,
{
    let (writes, mut queue) = mpsc::unbounded_channel();

    // TODO: both ports are settled when the images are. The conduit
    // first, deliberately — see above.
    let (mcp_reader, mcp_writer) = match container.connect(8081).await {
        Ok(pipe) => pipe,
        Err(error) => {
            write(scope, &response::Frame::Error(error.into())).await;
            return;
        }
    };
    let mcp = tokio::spawn(mcp(mcp_reader, mcp_writer, writes.clone()));

    let (agent_reader, agent_writer) = match container.connect(8080).await {
        Ok(pipe) => pipe,
        Err(error) => {
            mcp.abort();
            write(scope, &response::Frame::Error(error.into())).await;
            return;
        }
    };
    let agent =
        tokio::spawn(agent(agent_reader, agent_writer, body, writes));

    loop {
        // The borrows end with the block, so the scope is free again by
        // the time there is something to write on it.
        let write = {
            let queued = pin!(queue.recv());
            let caller = pin!(scope.recv_channel_request());
            match future::select(queued, caller).await {
                Either::Left((Some(write), _)) => write,
                // Both workers are gone without having said so, which
                // only a panic produces.
                Either::Left((None, _)) => break,
                // Nothing is defined on a channel a caller opens here,
                // so there is nothing to do with one but carry on.
                Either::Right((Some(_), _)) => continue,
                // The caller is gone. Nothing it asked for matters now.
                Either::Right((None, _)) => break,
            }
        };

        match write {
            // A frame that would not encode arrives empty, and an empty
            // response is not a legal one. It is dropped here because
            // this is where the wire is, and the worker that produced
            // it had nowhere to report the failure anyway.
            Write::Response(bytes) if bytes.is_empty() => {}
            Write::Response(bytes) => scope.send_response(&bytes).await,
            Write::ChannelRequest(bytes, back) => {
                let channel = scope.send_channel_request(&bytes).await;
                // A worker that stopped waiting drops the channel here,
                // which tells the session the number is free again.
                let _ = back.send(channel);
            }
            Write::Done => break,
        }
    }

    agent.abort();
    mcp.abort();
}

/// Ask the agent to run, and turn what comes back into frames.
///
/// A straight loop with no knowledge of the conduit beside it. It sends
/// the request rather than being handed the answer, because sending it
/// is the part that must not happen on the task doing the writing —
/// see [`relay`].
///
/// An event that is not a chunk ends the loop and is reported, because
/// a stream that has started saying things this crate cannot read is
/// not one to keep relaying.
async fn agent<R, W, E>(
    reader: R,
    writer: W,
    body: Bytes,
    writes: mpsc::UnboundedSender<Write>,
) where
    R: futures_util::Stream<Item = Result<Bytes, E>> + Send + Unpin + 'static,
    W: futures_util::Sink<Bytes, Error = E> + Send + Unpin + 'static,
    E: Into<Error> + Send + 'static,
{
    // TODO: the path is settled when the images are.
    let request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri("/")
        .header(hyper::header::HOST, "container")
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header(hyper::header::ACCEPT, "text/event-stream")
        .body(Full::new(body));

    let response = match request {
        Ok(request) => {
            crate::server::http::request(reader, writer, request).await
        }
        Err(error) => Err(Error(Value::String(error.to_string()))),
    };

    let response = match response {
        Ok(response) => response,
        Err(error) => return done(&writes, Some(error)),
    };

    // A status is the container's answer about itself, and this is the
    // one place to judge it: everything after here assumes a stream.
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error = format!("the agent container answered {status}");
        return done(&writes, Some(Error(Value::String(error))));
    }

    // The body is a stream of frames, of which the data ones are the
    // bytes; a trailer is not an event and there are none here anyway.
    let events = BodyStream::new(response.into_body()).filter_map(|frame| {
        async {
            match frame {
                Ok(frame) => frame.into_data().ok().map(Ok),
                Err(error) => {
                    Some(Err(std::io::Error::other(error.to_string())))
                }
            }
        }
    });
    let mut events = pin!(events.eventsource());

    while let Some(event) = events.next().await {
        let chunk = event
            .map_err(|error| Error(Value::String(error.to_string())))
            .and_then(|event| {
                serde_json::from_str(&event.data)
                    .map_err(|error| Error(Value::String(error.to_string())))
            });

        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => return done(&writes, Some(error)),
        };

        if writes.send(encoded(&response::Frame::Chunk(chunk))).is_err() {
            return;
        }
    }

    // The agent is finished, so the scope is. Whatever the conduit was
    // doing was in service of a loop that is over.
    done(&writes, None)
}

/// Say the loop is over, reporting a failure first if there was one.
fn done(writes: &mpsc::UnboundedSender<Write>, error: Option<Error>) {
    if let Some(error) = error {
        let _ = writes.send(encoded(&response::Frame::Error(error)));
    }
    let _ = writes.send(Write::Done);
}

/// Work the MCP conduit, in both of its directions.
///
/// A straight loop with no knowledge of the chunks beside it. Each
/// message that arrives is an exchange the container wants performed,
/// and each one gets a task of its own — a tool call that takes a while
/// must not hold up the next one, which is the whole reason the conduit
/// carries an exchange number.
///
/// # It does not end the scope
///
/// A conduit that ends means the agent has no tools left, not that the
/// agent is finished. What it is doing without them is its own
/// business, and it is still being relayed.
async fn mcp<R, W, E>(
    reader: R,
    writer: W,
    writes: mpsc::UnboundedSender<Write>,
) where
    R: futures_util::Stream<Item = Result<Bytes, E>> + Unpin,
    W: futures_util::Sink<Bytes> + Send + Unpin + 'static,
    E: Into<Error>,
{
    let writer = std::sync::Arc::new(tokio::sync::Mutex::new(writer));
    let mut messages = pin!(mcp_conduit::messages(reader));
    // Owned here rather than spawned loose, so that aborting this task
    // aborts the exchanges it started: dropping the set cancels every
    // one of them, and a tool call outliving the loop it was serving
    // would be holding a channel on a scope that has finished.
    let mut exchanges = tokio::task::JoinSet::new();

    while let Some(message) = messages.next().await {
        // Finished ones, so the set does not grow for the whole run.
        while exchanges.try_join_next().is_some() {}

        let message = match message {
            Ok(message) => message,
            // The pipe is broken, so there is nothing to read and
            // nothing to answer into. The loop above carries on.
            Err(_) => return,
        };

        let request = match http::request::Request::decode(&message.payload) {
            Ok(request) => request,
            // A payload that is not a request must not reach the caller
            // as though it were. Everything else on the conduit is
            // still good, so only this exchange is dropped.
            Err(_) => continue,
        };

        let mut bytes = Vec::new();
        if channel_request::Frame(request)
            .encode(&mut Writer::new(&mut bytes))
            .is_err()
        {
            continue;
        }

        let (back, channel) = oneshot::channel();
        if writes.send(Write::ChannelRequest(bytes, back)).is_err() {
            return;
        }

        exchanges.spawn(exchange(
            message.exchange,
            channel,
            std::sync::Arc::clone(&writer),
        ));
    }
}

/// Pump one caller's answer back down the conduit.
///
/// # Whole frames arrive here
///
/// Not payloads. A
/// [`Session`](crate::server::session::Session) forwards a
/// [`ChannelResponseFinish`](ClientFrame::ChannelResponseFinish) to the
/// receiver and only then closes the channel, so what comes off it is
/// client frames and the stream ends by itself afterwards. The finish
/// carries nothing to forward; the stream ending is what says the
/// exchange is over, and that is what the empty message below means.
async fn exchange<W>(
    exchange: u32,
    channel: oneshot::Receiver<Channel>,
    writer: std::sync::Arc<tokio::sync::Mutex<W>>,
) where
    W: futures_util::Sink<Bytes> + Unpin,
{
    // Gone means the scope ended between asking and being answered.
    let Ok(mut channel) = channel.await else {
        return;
    };

    while let Some(bytes) = channel.response_receiver.recv().await {
        if let Ok(ClientFrame::ChannelResponse { payload, .. }) =
            ClientFrame::decode(&bytes)
        {
            let message = mcp_conduit::encode(exchange, payload);
            // One message, and the lock is held for exactly that. A
            // conduit shared by several exchanges is the reason there
            // is a lock at all.
            if writer.lock().await.send(message).await.is_err() {
                return;
            }
        }
    }

    let message = mcp_conduit::encode(exchange, &[]);
    let _ = writer.lock().await.send(message).await;
}

/// Encode a frame as a write, or as an empty one if it will not encode.
///
/// The one failure with nowhere to report it: the channel for saying so
/// is the thing that would not serialize. An empty payload is not a
/// legal response, so it is dropped rather than sent.
fn encoded(frame: &response::Frame) -> Write {
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_err() {
        bytes.clear();
    }
    Write::Response(bytes)
}

/// Write one frame, or write nothing if it will not encode.
///
/// Used only where there is no worker yet to route through. Once there
/// is, a frame travels as a [`Write`] like anything else.
async fn write(scope: &mut ScopeHandle, frame: &response::Frame) {
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_ok() {
        scope.send_response(&bytes).await;
    }
}
