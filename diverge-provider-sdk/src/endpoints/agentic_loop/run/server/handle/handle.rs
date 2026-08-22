//! Running an agent in a container, and relaying both directions.

use std::pin::pin;

use bytes::Bytes;
use eventsource_stream::Eventsource as _;
use futures_util::future::{self, Either};
use futures_util::{SinkExt as _, StreamExt as _};
use http_body_util::{BodyStream, Full};
use serde_json::Value;
use std::sync::Arc;

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
/// # The identity goes to the deployer and no further
///
/// This endpoint's request carries none of its own, and the container
/// gets no mounts, no volumes and no environment — so there is no
/// namespace to resolve one against and nothing inside is told who
/// asked. What it is for is the deploy, which is a provider spending
/// its own capacity on somebody's behalf and is entitled to know
/// whose.
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
pub async fn handle<D>(
    scope: ScopeHandle,
    client_identity: &str,
    deployer: &D,
)
where
    D: ContainerDeployer,
    D::Error: Into<Error>,
    <D::Container as Container>::Error: Into<Error>,
{
    // Shared from here, because both workers write on it and neither
    // may hold it alone. `ScopeHandle`'s methods take `&self` for
    // exactly this; what stays exclusive is ENDING the scope, which is
    // why the finish below has to get the handle back out.
    let scope = Arc::new(scope);

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
            write(&scope, &response::Frame::Error(error)).await;
            finish(scope).await;
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

    let deployed =
        deployer.registry(client_identity, &deployment, image(&agent));
    let container = match deployed.await {
        Ok(container) => container,
        Err(error) => {
            write(&scope, &response::Frame::Error(error.into())).await;
            finish(scope).await;
            return;
        }
    };

    relay(&scope, &container, body).await;

    container.stop().await;
    finish(scope).await;
}

/// End the scope, which needs the handle back to itself.
///
/// [`send_response_finish`](ScopeHandle::send_response_finish) consumes
/// the handle, and that is what makes one finish per scope a fact
/// rather than a rule. So it cannot be reached through a share, and
/// this is where the shares are proved gone.
///
/// [`relay`] awaits both workers after aborting them, so their clones
/// are dropped before this is reached and there is one left. If somehow
/// there were not, the scope still ends when the last clone goes -- a
/// [`ScopeHandle`] tells the session on drop -- but with no finish
/// frame, and a caller would see the connection account for it rather
/// than the scope.
async fn finish(scope: Arc<ScopeHandle>) {
    if let Some(scope) = Arc::into_inner(scope) {
        scope.send_response_finish().await;
    }
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

/// Drive the container until the agent stops or the caller goes.
///
/// Three parts, and this is the smallest of them: a task that asks the
/// agent and reads its stream, a task working the MCP conduit, and this
/// waiting for whichever ending comes first.
///
/// # Neither worker knows the other exists
///
/// They share the scope and nothing else. Both write on it directly,
/// because a [`ScopeHandle`] takes `&self` --- so there is no queue
/// between them, no third party deciding whose turn it is, and a tool
/// call that takes a minute cannot delay a chunk.
///
/// # The conduit is open before the request is sent
///
/// Which is the ordering that matters, and the reason the request is
/// not sent from here. A container is free to want a tool call while it
/// is still working out how to answer, so the pipe it would ask on has
/// to be dialled first --- and the task that serves it has to be
/// running rather than waiting behind the very request it would be
/// unblocking.
async fn relay<C>(scope: &Arc<ScopeHandle>, container: &C, body: Bytes)
where
    C: Container,
    C::Error: Into<Error>,
{
    // TODO: both ports are settled when the images are. The conduit
    // first, deliberately --- see above.
    let (mcp_reader, mcp_writer) = match container.connect(8081).await {
        Ok(pipe) => pipe,
        Err(error) => {
            write(scope, &response::Frame::Error(error.into())).await;
            return;
        }
    };
    let mut mcp =
        tokio::spawn(mcp(mcp_reader, mcp_writer, Arc::clone(scope)));

    let (agent_reader, agent_writer) = match container.connect(8080).await {
        Ok(pipe) => pipe,
        Err(error) => {
            mcp.abort();
            let _ = (&mut mcp).await;
            write(scope, &response::Frame::Error(error.into())).await;
            return;
        }
    };
    let mut agent = tokio::spawn(agent(
        agent_reader,
        agent_writer,
        body,
        Arc::clone(scope),
    ));

    // The agent finishing is the loop being over; the caller leaving is
    // nobody being left to tell. Nothing else ends this.
    let stopped = {
        let hangup = pin!(hangup(scope));
        match future::select(&mut agent, hangup).await {
            Either::Left((Ok(()), _)) => true,
            // The agent's own failures are frames before it returns, so
            // the only thing left here is the task itself having
            // stopped existing. Which is the one failure it could not
            // report, and it must not read as a clean finish: a caller
            // that saw silence would conclude the loop simply had
            // nothing more to say.
            Either::Left((Err(_), _)) => {
                let error = "the agent relay stopped unexpectedly";
                let error = Error(Value::String(error.to_owned()));
                write(scope, &response::Frame::Error(error)).await;
                true
            }
            Either::Right(_) => false,
        }
    };

    // A `JoinHandle` that has already resolved must not be polled
    // again, which is why the two are not simply awaited together.
    if !stopped {
        agent.abort();
        let _ = (&mut agent).await;
    }
    mcp.abort();
    let _ = (&mut mcp).await;
}

/// Wait for the caller to leave.
///
/// This endpoint defines no channel for a caller to open, and an answer
/// to one of ours goes to that channel's own receiver --- so nothing is
/// ever expected here. What is being waited for is the [`None`]: the
/// session drops the sender when the scope ends, and that is how a
/// caller hanging up is learned at all.
///
/// Anything that does arrive is read and dropped, because a queue
/// nobody drains is memory the far side can grow.
async fn hangup(scope: &ScopeHandle) {
    while scope.recv_channel_request().await.is_some() {}
}

/// Ask the agent to run, and turn what comes back into frames.
///
/// A straight loop that mentions the conduit nowhere. It sends the
/// request itself rather than being handed the answer, because the
/// container may want a tool call before it answers and the task that
/// serves those has to already be running --- see [`relay`].
///
/// An event that is not a chunk ends it and is reported, because a
/// stream that has started saying things this crate cannot read is not
/// one to keep relaying.
async fn agent<R, W, E>(
    reader: R,
    writer: W,
    body: Bytes,
    scope: Arc<ScopeHandle>,
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

    let answer = match request {
        Ok(request) => {
            crate::server::http::request(reader, writer, request).await
        }
        Err(error) => Err(Error(Value::String(error.to_string()))),
    };

    let answer = match answer {
        Ok(answer) => answer,
        Err(error) => {
            return write(&scope, &response::Frame::Error(error)).await;
        }
    };

    // A status is the container's answer about itself, and this is the
    // one place to judge it: everything after here assumes a stream.
    if !answer.status().is_success() {
        let status = answer.status().as_u16();
        let error = format!("the agent container answered {status}");
        let error = Error(Value::String(error));
        return write(&scope, &response::Frame::Error(error)).await;
    }

    // The body is a stream of frames, of which the data ones are the
    // bytes; a trailer is not an event and there are none here anyway.
    let events = BodyStream::new(answer.into_body()).filter_map(|frame| {
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

        match chunk {
            Ok(chunk) => write(&scope, &response::Frame::Chunk(chunk)).await,
            Err(error) => {
                return write(&scope, &response::Frame::Error(error)).await;
            }
        }
    }
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
async fn mcp<R, W, E>(reader: R, writer: W, scope: Arc<ScopeHandle>)
where
    R: futures_util::Stream<Item = Result<Bytes, E>> + Unpin,
    W: futures_util::Sink<Bytes> + Send + Unpin + 'static,
    E: Into<Error>,
{
    let writer = Arc::new(tokio::sync::Mutex::new(writer));
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
            // still good, so only this exchange ends — and it has to
            // END, because nothing else is ever going to answer it.
            Err(_) => {
                ended(&writer, message.exchange).await;
                continue;
            }
        };

        let mut bytes = Vec::new();
        if channel_request::Frame(request)
            .encode(&mut Writer::new(&mut bytes))
            .is_err()
        {
            ended(&writer, message.exchange).await;
            continue;
        }

        // Opened here, on the scope itself. There is nothing between
        // this task and the wire.
        let channel = scope.send_channel_request(&bytes).await;

        exchanges.spawn(exchange(
            message.exchange,
            channel,
            Arc::clone(&writer),
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
    mut channel: Channel,
    writer: Arc<tokio::sync::Mutex<W>>,
) where
    W: futures_util::Sink<Bytes> + Unpin,
{
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

    ended(&writer, exchange).await;
}

/// Say an exchange is over, and nothing more.
///
/// The empty message, which is the conduit's only way of ending one.
/// Every path that stops serving an exchange goes through here — the
/// caller having finished its answer, and the two where a message never
/// became a channel request at all.
///
/// That second kind is why this is a function. A container that asked
/// and is told nothing waits for an answer that no longer has anything
/// to produce it, and it waits forever: nothing in this protocol times
/// out, and the conduit is the only place the news could arrive.
async fn ended<W>(writer: &tokio::sync::Mutex<W>, exchange: u32)
where
    W: futures_util::Sink<Bytes> + Unpin,
{
    let message = mcp_conduit::encode(exchange, &[]);
    let _ = writer.lock().await.send(message).await;
}

/// Write one frame, or write nothing if it will not encode.
///
/// The one failure with nowhere to report it: the channel for saying so
/// is the thing that would not serialize.
async fn write(scope: &ScopeHandle, frame: &response::Frame) {
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_ok() {
        scope.send_response(&bytes).await;
    }
}
