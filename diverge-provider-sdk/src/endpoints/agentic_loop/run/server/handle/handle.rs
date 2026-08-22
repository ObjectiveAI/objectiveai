//! Running an agent in a container, and relaying both directions.

use std::pin::{Pin, pin};

use bytes::Bytes;
use eventsource_stream::Eventsource as _;
use futures_util::future::{self, Either};
use futures_util::stream::{self, SelectAll};
use futures_util::{SinkExt as _, Stream, StreamExt as _};
use http_body_util::{BodyStream, Full};
use serde_json::Value;

use super::super::{channel_request, response};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::agentic_loop::run::client::request;
use crate::endpoints::agentic_loop::run::client::request::agent::Agent;
use crate::frame::client::ClientFrame;
use crate::server::container::Container;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::deployment::Deployment;
use crate::server::mcp_conduit;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::error::Error;
use crate::shared::http;

/// The image that runs an OpenRouter agent.
// TODO: no image is published yet.
const OPENROUTER_IMAGE: &str = "TODO";

/// The image that runs a Claude Agent SDK agent.
// TODO: no image is published yet.
const CLAUDE_AGENT_SDK_IMAGE: &str = "TODO";

/// The image that runs a Codex SDK agent.
// TODO: no image is published yet.
const CODEX_SDK_IMAGE: &str = "TODO";

/// The image that runs a Python agent.
// TODO: no image is published yet.
const PYTHON_IMAGE: &str = "TODO";

/// How much memory an agent container may have, in bytes.
// TODO: a number nobody has justified.
const MEMORY: u64 = 2 * 1024 * 1024 * 1024;

/// How much an agent container may write, in bytes.
// TODO: a number nobody has justified.
const DISK: u64 = 2 * 1024 * 1024 * 1024;

/// Where the image serves the loop.
// TODO: settled when the image is.
const AGENT_PORT: u16 = 8080;

/// Where the image listens for the MCP conduit.
// TODO: settled when the image is.
const MCP_PORT: u16 = 8081;

/// What the request is POSTed to.
// TODO: settled when the image is.
const AGENT_PATH: &str = "/";

/// Run the loop and relay it, until it ends or the caller goes.
///
/// Two directions at once, which is what makes this the largest
/// handler. Chunks come out of the container and go down to the caller;
/// tool calls come out of the container the other way and go out on
/// channels, and the caller's answers come back and go in. Neither
/// waits for the other.
///
/// # It takes no `client_identity`
///
/// Alone among the handlers. This endpoint's request carries no
/// identity and a
/// [`ContainerDeployer`] takes none, so there is nothing an identity
/// would be used for — the container gets no mounts, no volumes and no
/// environment, so there is no namespace to resolve it against.
///
/// # The request is forwarded verbatim
///
/// The bytes the caller sent, minus the tag that said which request it
/// was. It is decoded here only to pick the image, and what goes into
/// the container is the original JSON rather than a re-serialization of
/// what came out of the decoder — so a field this crate does not know
/// about survives the trip.
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
    let (image, body) = match request::Frame::decode(scope.request()) {
        // The payload is a tag byte and then the request's own JSON, so
        // what the container should be handed is everything after the
        // first byte.
        Ok(frame) => (
            image(&frame.agent),
            Bytes::copy_from_slice(&scope.request()[1..]),
        ),
        Err(error) => {
            let error = Error(Value::String(error.to_string()));
            send(&mut scope, &response::Frame::Error(error)).await;
            scope.send_response_finish().await;
            return;
        }
    };

    let deployment = Deployment {
        memory: MEMORY,
        disk: DISK,
        environment: Default::default(),
        // An agent works in the container it was given. Nothing of the
        // caller's is mounted into it.
        mounts: Vec::new(),
        ports: vec![AGENT_PORT, MCP_PORT],
    };

    let container = match deployer.registry(&deployment, image).await {
        Ok(container) => container,
        Err(error) => {
            send(&mut scope, &response::Frame::Error(error.into())).await;
            scope.send_response_finish().await;
            return;
        }
    };

    if let Err(error) = relay(&mut scope, &container, body).await {
        send(&mut scope, &response::Frame::Error(error)).await;
    }

    container.stop().await;
    scope.send_response_finish().await;
}

/// Which image runs this agent.
///
/// The one thing the request is read for. Every upstream is a different
/// program with a different way of being driven, so it is a different
/// image — and which one is not the caller's to choose, since a caller
/// that could name an image could name any image.
fn image(agent: &Agent) -> &'static str {
    match agent {
        Agent::Openrouter(_) => OPENROUTER_IMAGE,
        Agent::ClaudeAgentSdk(_) => CLAUDE_AGENT_SDK_IMAGE,
        Agent::CodexSdk(_) => CODEX_SDK_IMAGE,
        Agent::Python(_) => PYTHON_IMAGE,
    }
}

/// What one source produced.
///
/// Everything the loop waits on is turned into one of these and merged
/// into a single stream, because a [`ScopeHandle`] cannot be written
/// from two places at once — the client's handle is the cloneable one,
/// and this half's writing belongs to the individual scope. So there is
/// one loop, and every write is in it.
enum Event {
    /// One chunk out of the agent, or an answer that was not one.
    Chunk(Result<response::AgenticLoopChunk, Error>),
    /// The agent's stream ended, which is the loop finishing.
    ChunkEnd,
    /// The container wants an MCP exchange performed.
    Conduit(Result<mcp_conduit::Message, Error>),
    /// The conduit ended, which says nothing about the loop.
    ConduitEnd,
    /// One frame of a caller's answer, whole and undecoded.
    Answer(u32, Bytes),
    /// A caller's answer ended, so the exchange is over.
    AnswerEnd(u32),
}

/// Every source, merged, so one loop can do all the writing.
type Sources = SelectAll<Pin<Box<dyn Stream<Item = Event> + Send>>>;

/// Drive the container until the loop ends, the caller goes, or
/// something fails.
///
/// [`Err`] is a failure worth telling the caller about. [`Ok`] is any
/// ordinary end — the agent finished, or the caller left.
async fn relay<C>(
    scope: &mut ScopeHandle,
    container: &C,
    body: Bytes,
) -> Result<(), Error>
where
    C: Container,
    C::Error: Into<Error>,
{
    let request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(AGENT_PATH)
        .header(hyper::header::HOST, "container")
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header(hyper::header::ACCEPT, "text/event-stream")
        .body(Full::new(body))
        .map_err(|error| Error(Value::String(error.to_string())))?;

    let response =
        crate::server::http::request(container, AGENT_PORT, request).await?;

    // A status is the container's answer about itself and this is the
    // one place to judge it: everything after here assumes a stream.
    if !response.status().is_success() {
        let status = response.status().as_u16();
        return Err(Error(Value::String(format!(
            "the agent container answered {status}"
        ))));
    }

    let (conduit_reader, mut conduit_writer) =
        container.connect(MCP_PORT).await.map_err(Into::into)?;

    let chunks = BodyStream::new(response.into_body())
        .filter_map(|frame| async {
            match frame {
                Ok(frame) => frame.into_data().ok().map(Ok),
                Err(error) => {
                    Some(Err(std::io::Error::other(error.to_string())))
                }
            }
        })
        .eventsource()
        .map(|event| {
            Event::Chunk(
                event
                    .map_err(|error| Error(Value::String(error.to_string())))
                    .and_then(|event| {
                        serde_json::from_str(&event.data).map_err(|error| {
                            Error(Value::String(error.to_string()))
                        })
                    }),
            )
        })
        .chain(stream::once(async { Event::ChunkEnd }));

    let conduit = mcp_conduit::messages(conduit_reader)
        .map(|message| {
            Event::Conduit(
                message
                    .map_err(|error| match error {
                        mcp_conduit::MessageError::Pipe(error) => error.into(),
                        mcp_conduit::MessageError::Truncated(len) => {
                            Error(Value::String(format!(
                                "the mcp conduit ended with {len} bytes unread"
                            )))
                        }
                    }),
            )
        })
        .chain(stream::once(async { Event::ConduitEnd }));

    let mut sources: Sources = SelectAll::new();
    sources.push(Box::pin(chunks));
    sources.push(Box::pin(conduit));

    loop {
        // The borrows end with the block, so the scope is free again by
        // the time there is something to write on it.
        let step = {
            let next = pin!(sources.next());
            let caller = pin!(scope.recv_channel_request());
            match future::select(next, caller).await {
                Either::Left((Some(event), _)) => Step::Event(event),
                // Every source is exhausted, which the agent's own end
                // has usually already broken out of.
                Either::Left((None, _)) => Step::Stop,
                // Nothing is defined on a channel a caller opens here,
                // so there is nothing to do with one but carry on.
                Either::Right((Some(_), _)) => Step::Ignore,
                // The caller is gone. Nothing it asked for matters now.
                Either::Right((None, _)) => Step::Stop,
            }
        };

        let event = match step {
            Step::Ignore => continue,
            Step::Stop => return Ok(()),
            Step::Event(event) => event,
        };

        match event {
            Event::Chunk(Ok(chunk)) => {
                send(scope, &response::Frame::Chunk(chunk)).await
            }
            Event::Chunk(Err(error)) => return Err(error),
            Event::ChunkEnd => return Ok(()),

            Event::Conduit(Ok(message)) => {
                let exchange = message.exchange;
                let channel = open(scope, &message.payload).await?;
                sources.push(Box::pin(answers(exchange, channel)));
            }
            Event::Conduit(Err(error)) => return Err(error),
            // The tools are gone and the agent is not. Whatever it does
            // next is its own business, and it is still being relayed.
            Event::ConduitEnd => {}

            Event::Answer(exchange, bytes) => {
                // Whole client frames arrive here. A response carries a
                // payload to forward; a finish carries nothing, and the
                // stream ending is what says so anyway.
                if let Ok(ClientFrame::ChannelResponse { payload, .. }) =
                    ClientFrame::decode(&bytes)
                {
                    let message = mcp_conduit::encode(exchange, payload);
                    let _ = conduit_writer.send(message).await;
                }
            }
            Event::AnswerEnd(exchange) => {
                let message = mcp_conduit::encode(exchange, &[]);
                let _ = conduit_writer.send(message).await;
            }
        }
    }
}

/// What one turn of the loop found to do.
enum Step {
    /// A source produced something.
    Event(Event),
    /// Something arrived that this endpoint does not define.
    Ignore,
    /// There is nothing left to relay.
    Stop,
}

/// Ask the caller to perform one MCP exchange.
///
/// The container's payload is already a
/// [`Request`](http::request::Request) and the channel carries one, so
/// this decodes it only to encode it again — which sounds wasteful and
/// is the point. A payload that is not a request must not reach the
/// caller as though it were, and the decode is what establishes that.
async fn open(
    scope: &mut ScopeHandle,
    payload: &[u8],
) -> Result<crate::server::channel::Channel, Error> {
    let request = http::request::Request::decode(payload)
        .map_err(|error| Error(Value::String(error.to_string())))?;

    let mut bytes = Vec::new();
    channel_request::Frame(request)
        .encode(&mut Writer::new(&mut bytes))
        .map_err(|error| Error(Value::String(error.to_string())))?;

    Ok(scope.send_channel_request(&bytes).await)
}

/// One channel's answers, as events tagged with their exchange.
///
/// The whole [`Channel`](crate::server::channel::Channel) is carried
/// rather than the receiver taken out of it, because dropping one is
/// what tells the session the number is free again — and a receiver
/// moved out of it would have left that behind.
fn answers(
    exchange: u32,
    channel: crate::server::channel::Channel,
) -> impl Stream<Item = Event> {
    stream::unfold(channel, |mut channel| async move {
        let bytes = channel.response_receiver.recv().await?;
        Some((bytes, channel))
    })
    .map(move |bytes| Event::Answer(exchange, bytes))
    .chain(stream::once(async move { Event::AnswerEnd(exchange) }))
}

/// Write one frame, or write nothing if it will not encode.
///
/// The one failure with nowhere to report it: the channel for saying so
/// is the thing that would not serialize.
async fn send(scope: &mut ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
