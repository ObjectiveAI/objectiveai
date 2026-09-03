//! Running an agent in a container, and relaying both directions.

use std::pin::pin;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::future::{self, Either};
use futures_util::{Stream, StreamExt as _};
use rmcp::ErrorData;
use serde_json::Value;
use serde_json::value::RawValue;

use super::super::channel_request;
use super::super::response;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::agentic_loop::run::client::channel_response;
use crate::endpoints::agentic_loop::run::client::request;
use crate::endpoints::agentic_loop::run::client::request::agent::Agent;
use crate::frame::client::ClientFrame;
use crate::server::container::{Container, McpRequest, McpResponder};
use crate::server::container_deployer::ContainerDeployer;
use crate::server::deployment::Deployment;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::error::Error;
use crate::shared::mcp;

/// Run the loop and relay it, until it ends or the caller goes.
///
/// Two directions at once. Chunks come out of the container and go down
/// to the caller; tool calls come out the other way and go out on
/// channels, and the caller's answers go back in. Neither waits for the
/// other, because neither is in the other's loop.
///
/// # The identity goes to the deployer and no further
///
/// This endpoint's request carries none of its own, and the container
/// gets no mounts, no volumes and no environment — so there is no
/// namespace to resolve one against and nothing inside is told who
/// asked. What it is for is the deploy, which is a provider spending
/// its own capacity on somebody's behalf and is entitled to know whose.
///
/// # The container is stopped on every path
///
/// Including the ones that failed. There is no destructor doing it,
/// because teardown in this crate is a method; there is one call, after
/// the relay, and every exit goes through it.
///
/// # A caller hanging up does not reach it early
///
/// The run finishes first. Work that was done is work that was done,
/// and a provider that abandoned it the moment a connection dropped
/// would be giving it away — a caller could take as much as it wanted
/// and disconnect before anything could be counted.
///
/// So what a caller sees when it leaves is nothing, and what happens is
/// the rest of the run.
///
/// # The request arrives decoded, and the body arrives raw
///
/// [`server::handle`](crate::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
///
/// `body` is the same request as bytes, less the tag in front: the
/// caller's own JSON, which is what the container is handed. Handing
/// it the bytes rather than a re-serialization of `request` is what
/// lets a field this crate does not model survive the trip.
pub async fn handle<D>(
    scope: ScopeHandle,
    request: request::Frame,
    body: Bytes,
    client_identity: &str,
    deployer: &D,
) where
    D: ContainerDeployer,
    D::Error: Into<Error>,
    <D::Container as Container>::Error: Into<Error>,
{
    // Shared from here, because both workers write on it and neither
    // may hold it alone. `ScopeHandle`'s methods take `&self` for
    // exactly this; what stays exclusive is ENDING the scope, which is
    // why the finish has to get the handle back out.
    let scope = Arc::new(scope);

    let agent = request.agent;

    let deployment = Deployment {
        memory: memory(&agent),
        disk: disk(&agent),
        environment: Default::default(),
        // An agent works in the container it was given. Nothing of the
        // caller's is mounted into it.
        mounts: Vec::new(),
        // The loop, then the tool calls it makes — in that order, and
        // the calls below have to agree with it.
        // TODO: settled when the images are.
        ports: vec![8080, 8081],
    };

    let deployed =
        deployer.registry(client_identity, &deployment, image(&agent));
    let container = match deployed.await {
        Ok(container) => Arc::new(container),
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
        Agent::ClaudeCode(_) => "TODO",
        Agent::Codex(_) => "TODO",
        Agent::Hermes(_) => "TODO",
        Agent::Eliza(_) => "TODO",
        Agent::Python(_) => "TODO",
    }
}

/// How much memory this agent's container may have, in bytes.
///
/// # Four of them are the image's number and one is the caller's
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
/// So a limit belongs to whoever knows what is going to run. Four
/// times that is us and once it is the caller, and reading the caller's
/// number is not a courtesy — ignoring it means the kernel kills a
/// script that said what it needed.
fn memory(agent: &Agent) -> u64 {
    match agent {
        // TODO: numbers nobody has justified, and which belong with the
        // images once those exist.
        Agent::Openrouter(_) => 512 * 1024 * 1024,
        Agent::ClaudeCode(_) => 2 * 1024 * 1024 * 1024,
        Agent::Codex(_) => 2 * 1024 * 1024 * 1024,
        Agent::Hermes(_) => 2 * 1024 * 1024 * 1024,
        Agent::Eliza(_) => 2 * 1024 * 1024 * 1024,
        Agent::Python(agent) => agent.memory,
    }
}

/// How much this agent's container may write, in bytes.
///
/// The same split [`memory`] makes, for the same reason: four images
/// whose appetite is a property of the image, and one whose is a
/// property of what the caller sent. See
/// [`python::Agent::disk`](crate::endpoints::agentic_loop::run::client::request::agent::python::Agent::disk).
fn disk(agent: &Agent) -> u64 {
    match agent {
        // TODO: as above.
        Agent::Openrouter(_) => 256 * 1024 * 1024,
        Agent::ClaudeCode(_) => 4 * 1024 * 1024 * 1024,
        Agent::Codex(_) => 4 * 1024 * 1024 * 1024,
        Agent::Hermes(_) => 4 * 1024 * 1024 * 1024,
        Agent::Eliza(_) => 4 * 1024 * 1024 * 1024,
        Agent::Python(agent) => agent.disk,
    }
}

/// Drive the container until the agent stops.
///
/// Three parts, and this one is the smallest: a task serving the
/// agent's tool calls, a task running the agent, and this waiting for
/// the second of them.
///
/// # Neither worker knows the other exists
///
/// They share the scope and nothing else. Both write on it directly,
/// because a [`ScopeHandle`] takes `&self` — so there is no queue
/// between them and no third party deciding whose turn it is, and a
/// tool call that takes a minute cannot delay a chunk.
///
/// # Tool calls are served before the agent is asked to run
///
/// Which is the ordering that matters. An agent is free to want a tool
/// call while it is still working out how to answer, so the thing that
/// serves those has to be RUNNING rather than waiting behind the very
/// request it would be unblocking.
///
/// A version that ran the agent here and then started serving deadlocks
/// on exactly that: the container waits for its tool answer, this waits
/// for the container's answer, and nothing is left to resolve it.
///
/// # A caller leaving does not end the run
///
/// It used to. The agent was aborted, the container stopped, and
/// whatever it had done up to that point was done for nothing.
///
/// Which is the wrong way round, because the work was real. Tokens were
/// spent, an upstream was called, and a provider that throws that away
/// has given it away — a caller could take as much as it wanted and
/// disconnect before anything could be counted. So the run finishes,
/// and what it cost is a fact whatever happened to the connection.
///
/// The frames it produces after that go nowhere, and nothing here
/// pretends otherwise. Writing to a scope whose connection is gone is
/// a write that is dropped, which is what it should be: the answer had
/// somewhere to go and no longer does.
///
/// # What a caller leaving DOES do
///
/// Nothing, deliberately — but the consequence is worth stating,
/// because it is what keeps this from hanging. A tool call still opens
/// a channel, and a channel on a dead connection has no sender left, so
/// its answer stream ends at once. An agent asking for tools it can no
/// longer be given is told immediately that there are none, rather than
/// waiting for an answer nothing is coming to give.
async fn relay<C>(scope: &Arc<ScopeHandle>, container: &Arc<C>, body: Bytes)
where
    C: Container + 'static,
    C::Error: Into<Error>,
{
    // TODO: the port is settled when the images are.
    let requests = match container.mcp_serve(8081).await {
        Ok(requests) => requests,
        Err(error) => {
            write(scope, &response::Frame::Error(error.into())).await;
            return;
        }
    };
    let mut mcp = tokio::spawn(mcp(requests, Arc::clone(scope)));

    let mut agent =
        tokio::spawn(agent(Arc::clone(container), body, Arc::clone(scope)));

    // The agent finishing is the only thing that ends this. The race
    // is with the drain rather than against it: a caller that leaves
    // stops the draining and nothing else — see above.
    let finished = {
        let hangup = pin!(hangup(scope));
        match future::select(&mut agent, hangup).await {
            Either::Left((finished, _)) => finished,
            Either::Right((_, agent)) => agent.await,
        }
    };

    // The agent's own failures are frames before it returns, so the
    // only thing left here is the task having stopped existing. Which
    // is the one failure it could not report, and it must not read as a
    // clean finish: a caller that saw silence would conclude the loop
    // simply had nothing more to say.
    if finished.is_err() {
        let error = "the agent relay stopped unexpectedly";
        let error = Error(Value::String(error.to_owned()));
        write(scope, &response::Frame::Error(error)).await;
    }

    mcp.abort();
    let _ = (&mut mcp).await;
}

/// Read what the caller opens, and drop it.
///
/// This endpoint defines no channel for a caller to open, and an answer
/// to one of ours goes to that channel's own receiver — so nothing is
/// ever expected here. It is drained anyway, because a queue nobody
/// reads is memory the far side can grow.
///
/// # It returns when the caller leaves, and that is not an ending
///
/// The [`None`] means the session dropped the sender, which means the
/// connection is gone. Nothing acts on it: [`relay`] goes on waiting
/// for the agent, because the run is billable whether or not anyone is
/// still listening.
///
/// So returning here only stops the draining, which is exactly right —
/// there is nothing left to drain.
async fn hangup(scope: &ScopeHandle) {
    while scope.recv_channel_request().await.is_some() {}
}

/// Ask the agent to run, and turn what comes back into frames.
///
/// A straight loop that mentions the tool calls nowhere. It makes the
/// request itself rather than being handed the answer, because the
/// container may want a tool call before it answers and the thing that
/// serves those has to already be running — see [`relay`].
async fn agent<C>(container: Arc<C>, body: Bytes, scope: Arc<ScopeHandle>)
where
    C: Container,
    C::Error: Into<Error>,
{
    // Borrowed straight out of the bytes, which is what a [`RawValue`]
    // is for: the caller's request goes into the container as the JSON
    // it arrived as, rather than as a re-serialization of what came out
    // of a decoder, so a field this crate does not model survives.
    let body: &RawValue = match serde_json::from_slice(&body) {
        Ok(body) => body,
        Err(error) => {
            let error = Error(Value::String(error.to_string()));
            return write(&scope, &response::Frame::Error(error)).await;
        }
    };

    // TODO: the port is settled when the images are.
    let mut chunks = match container.agentic_loop(8080, body).await {
        Ok(chunks) => chunks,
        Err(error) => {
            return write(&scope, &response::Frame::Error(error.into())).await;
        }
    };

    while let Some(chunk) = chunks.next().await {
        match chunk {
            Ok(chunk) => write(&scope, &response::Frame::Chunk(chunk)).await,
            // One thing the agent said that could not be read. The
            // stream is free to go on — see
            // [`AgenticLoopStream`](Container::AgenticLoopStream) — but
            // the relay is not: an error frame is the last thing a
            // caller's stream accepts, so this is where it stops.
            Err(error) => {
                return write(&scope, &response::Frame::Error(error.into()))
                    .await;
            }
        }
    }
}

/// Serve the agent's tool calls, out to whoever holds the tools.
///
/// A straight loop that mentions the agent's own answer nowhere. Each
/// request the container makes becomes a channel, and what the caller
/// answers becomes the response — so what an agent talks to is an MCP
/// server living on the other side of the connection.
///
/// # One task per request
///
/// Because a tool call that takes a minute must not hold up the next
/// one, and an agent making several at once is the ordinary case. The
/// set is owned here rather than spawned loose, so aborting this aborts
/// what it started: a call outliving the loop it was serving would hold
/// a channel on a scope that has finished.
async fn mcp<S, R>(requests: S, scope: Arc<ScopeHandle>)
where
    S: Stream<Item = (McpRequest, R)> + Send + Unpin + 'static,
    R: McpResponder + Send + 'static,
{
    let mut requests = requests;
    let mut calls = tokio::task::JoinSet::new();

    while let Some((request, responder)) = requests.next().await {
        // Finished ones, so the set does not grow for the life of the
        // run. An agent makes a great many tool calls.
        while calls.try_join_next().is_some() {}

        calls.spawn(call(request, responder, Arc::clone(&scope)));
    }

    // Drained rather than dropped. The stream ending means the agent
    // has no more to ASK, which says nothing about the calls already in
    // flight — and dropping the set would cancel them, leaving answers
    // half written and writers never finished.
    //
    // Abandoning them is still what an abort does, and that is the
    // difference worth keeping: the run being over is not the same
    // event as the asking being over.
    while calls.join_next().await.is_some() {}
}

/// Relay one MCP exchange out, and its answer back.
///
/// # The responder is used on every path
///
/// Including the ones that went wrong. An agent that asked is an agent
/// waiting, and this connection is the only thing that can tell it
/// otherwise — so a caller that never answered, a frame that would not
/// decode and a channel that closed all end in an
/// [`ErrorData`] rather than in a return. See [`exchange`] for where
/// that happens.
///
/// # Which method answers is this function's to get right
///
/// A [`McpResponder`] has one per variant and does not check. The match
/// below is the whole of that guarantee, which is why each arm names
/// its request and its answer within a line of each other.
async fn call<R>(request: McpRequest, responder: R, scope: Arc<ScopeHandle>)
where
    R: McpResponder,
{
    match request {
        McpRequest::ListTools(params) => {
            let result = exchange(
                &scope,
                &channel_request::Frame::McpListTools(
                    mcp::list_tools::request::Request(params),
                ),
                |frame| match frame {
                    channel_response::mcp_list_tools::Frame::Result(
                        result,
                    ) => Ok(result),
                    channel_response::mcp_list_tools::Frame::Error(
                        error,
                    ) => Err(error),
                },
            )
            .await;
            let _ = responder.list_tools(result).await;
        }
        McpRequest::ListResources(params) => {
            let result = exchange(
                &scope,
                &channel_request::Frame::McpListResources(
                    mcp::list_resources::request::Request(params),
                ),
                |frame| match frame {
                    channel_response::mcp_list_resources::Frame::Result(
                        result,
                    ) => Ok(result),
                    channel_response::mcp_list_resources::Frame::Error(
                        error,
                    ) => Err(error),
                },
            )
            .await;
            let _ = responder.list_resources(result).await;
        }
        McpRequest::CallTool(params) => {
            let result = exchange(
                &scope,
                &channel_request::Frame::McpCallTool(
                    mcp::call_tool::request::Request(params),
                ),
                |frame| match frame {
                    channel_response::mcp_call_tool::Frame::Result(
                        result,
                    ) => Ok(result),
                    channel_response::mcp_call_tool::Frame::Error(
                        error,
                    ) => Err(error),
                },
            )
            .await;
            let _ = responder.call_tool(result).await;
        }
        McpRequest::ReadResource(params) => {
            let result = exchange(
                &scope,
                &channel_request::Frame::McpReadResource(
                    mcp::read_resource::request::Request(params),
                ),
                |frame| match frame {
                    channel_response::mcp_read_resource::Frame::Result(
                        result,
                    ) => Ok(result),
                    channel_response::mcp_read_resource::Frame::Error(
                        error,
                    ) => Err(error),
                },
            )
            .await;
            let _ = responder.read_resource(result).await;
        }
        McpRequest::Notifications => {
            notifications(responder, &scope).await;
        }
    }
}

/// One ask, one answer, and the failures told to the agent.
///
/// The unary shape, written once instead of four times. `split` is
/// which exchange this is: it takes the decoded answer frame apart into
/// the result the responder wants. A plain `fn` rather than a closure
/// bound, because there is nothing to capture — what varies between the
/// four exchanges is the frame type, and that is the type parameter.
///
/// Every way this can go wrong becomes an [`ErrorData`], because the
/// agent is waiting and silence is the one answer it cannot act on: no
/// answer at all is [`unanswered`], and an answer this crate could not
/// read is [`unreadable`].
async fn exchange<F, T>(
    scope: &ScopeHandle,
    frame: &channel_request::Frame,
    split: fn(F) -> Result<T, ErrorData>,
) -> Result<T, ErrorData>
where
    F: for<'a> Decode<'a, Error = mcp::FrameError>,
{
    match ask(scope, frame).await {
        Some(bytes) => match F::decode(&bytes) {
            Ok(frame) => split(frame),
            Err(_) => Err(unreadable()),
        },
        None => Err(unanswered()),
    }
}

/// Relay a notification stream out, and everything on it back.
///
/// The one exchange that is not answered once, so it is the one that
/// does not fit the shape of the four above: the channel stays open and
/// every frame on it is another notification.
///
/// # It always finishes
///
/// Whatever ended it — the caller saying so, a frame that would not
/// decode, the connection going. A container reading a stream that
/// simply stops has no way to tell an MCP server that finished from one
/// that was cut off, and this is the only thing that knows which.
async fn notifications<R>(mut responder: R, scope: &ScopeHandle)
where
    R: McpResponder,
{
    let mut payload = Vec::new();
    if channel_request::Frame::McpNotifications(
        mcp::notifications::request::Request,
    )
    .encode(&mut Writer::new(&mut payload))
    .is_err()
    {
        let _ = responder.notifications_finish(Some(unreadable())).await;
        return;
    }
    let mut channel = scope.send_channel_request(&payload).await;

    let error = loop {
        let Some(bytes) = channel.response_receiver.recv().await else {
            // The scope ended, which for a stream held open is the
            // ordinary way to find out there will be no more.
            break Some(unanswered());
        };
        let Ok(ClientFrame::ChannelResponse { payload, .. }) =
            ClientFrame::decode(&bytes)
        else {
            // A finish, which is the caller saying it will push no
            // more, or a frame with no business here. Both end it and
            // neither is a failure to report.
            break None;
        };
        match channel_response::mcp_notifications::Frame::decode(payload) {
            Ok(channel_response::mcp_notifications::Frame::Notification(
                notification,
            )) => {
                if responder.notification(notification).await.is_err() {
                    // The container stopped listening. Nothing left to
                    // relay to, and nothing to tell it about that.
                    return;
                }
            }
            // The caller's own server saying it will push no more, and
            // why. Nothing follows it.
            Ok(channel_response::mcp_notifications::Frame::Error(error)) => {
                break Some(error);
            }
            Err(_) => break Some(unreadable()),
        }
    };
    let _ = responder.notifications_finish(error).await;
}

/// Send one ask out on its own channel and take the one frame back.
///
/// The payload of the answer, refcounted out of the frame it arrived
/// in, or [`None`] if there was no answer to take — a scope that ended,
/// a channel that finished with nothing on it, a frame that would not
/// decode.
///
/// The channel is dropped on the way out rather than drained. Finishing
/// it is the caller's, and nothing here waits to see it done.
async fn ask(
    scope: &ScopeHandle,
    frame: &channel_request::Frame,
) -> Option<Bytes> {
    let mut payload = Vec::new();
    frame.encode(&mut Writer::new(&mut payload)).ok()?;
    let mut channel = scope.send_channel_request(&payload).await;

    let bytes = channel.response_receiver.recv().await?;
    let ClientFrame::ChannelResponse { payload, .. } =
        ClientFrame::decode(&bytes).ok()?
    else {
        // A finish with nothing before it, which is what a caller says
        // when it cannot serve the exchange at all.
        return None;
    };
    Some(bytes.slice_ref(payload))
}

/// Nobody answered, and the agent has to be told something.
///
/// A scope that ended, a caller that left, a channel that finished with
/// nothing on it. From inside the container these are one fact — the
/// tools are not reachable — and the JSON-RPC code for that is the
/// internal one, because the failure is on this side of the agent
/// rather than in what it asked for.
fn unanswered() -> ErrorData {
    ErrorData::internal_error("the caller did not answer", None)
}

/// Something answered and this crate could not read it.
///
/// Which is not the caller refusing — a refusal is a frame this
/// understands. It is a caller disagreeing with this one about what an
/// answer looks like, and an agent can do nothing about either, so both
/// arrive as an error rather than as silence.
fn unreadable() -> ErrorData {
    ErrorData::internal_error("the caller's answer could not be read", None)
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
/// there were not, the scope still ends when the last clone goes — a
/// [`ScopeHandle`] tells the session on drop — but with no finish
/// frame, and a caller would see the connection account for it rather
/// than the scope.
async fn finish(scope: Arc<ScopeHandle>) {
    if let Some(scope) = Arc::into_inner(scope) {
        scope.send_response_finish().await;
    }
}

/// Write one frame, or write nothing if it will not encode.
///
/// The one failure with nowhere to report it: the channel for saying so
/// is the thing that would not serialize.
async fn write(scope: &ScopeHandle, frame: &response::Frame<'_>) {
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_ok() {
        scope.send_response(&bytes).await;
    }
}
