//! Running an agent in a container, and relaying both directions.

use std::pin::pin;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use futures_util::StreamExt as _;
use futures_util::future::{self, Either};
use serde_json::Value;
use serde_json::value::RawValue;

use super::super::response;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::agentic_loop::run::client::request;
use crate::endpoints::agentic_loop::run::client::request::agent::Agent;
use crate::frame::client::ClientFrame;
use crate::server::container::{Body, Container, HttpResponseWriter};
use crate::server::container_deployer::ContainerDeployer;
use crate::server::deployment::Deployment;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::error::Error;
use crate::shared::http;

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
/// Including the ones that failed, and including the caller simply
/// leaving. There is no destructor doing it, because teardown in this
/// crate is a method; there is one call, after the relay, and every
/// exit goes through it.
pub async fn handle<D>(
    scope: ScopeHandle,
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

    let agent = match request::Frame::decode(scope.request()) {
        Ok(frame) => frame.agent.clone(),
        Err(error) => {
            let error = Error(Value::String(error.to_string()));
            write(&scope, &response::Frame::Error(error)).await;
            finish(scope).await;
            return;
        }
    };

    // The payload is a tag byte and then the request's own JSON, so what
    // the container is handed is everything after the first byte. Owned
    // because the task that sends it outlives this borrow.
    let body = Bytes::copy_from_slice(&scope.request()[1..]);

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
/// Three parts, and this one is the smallest: a task serving the
/// agent's tool calls, a task running the agent, and this waiting for
/// whichever ending comes first.
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
async fn relay<C>(scope: &Arc<ScopeHandle>, container: &Arc<C>, body: Bytes)
where
    C: Container + 'static,
    C::Error: Into<Error>,
{
    // TODO: the port is settled when the images are.
    let requests = match container.http_serve(8081).await {
        Ok(requests) => requests,
        Err(error) => {
            write(scope, &response::Frame::Error(error.into())).await;
            return;
        }
    };
    let mut mcp = tokio::spawn(mcp(requests, Arc::clone(scope)));

    let mut agent =
        tokio::spawn(agent(Arc::clone(container), body, Arc::clone(scope)));

    // The agent finishing is the loop being over; the caller leaving is
    // nobody being left to tell. Nothing else ends this.
    let stopped = {
        let hangup = pin!(hangup(scope));
        match future::select(&mut agent, hangup).await {
            Either::Left((Ok(()), _)) => true,
            // The agent's own failures are frames before it returns, so
            // the only thing left here is the task having stopped
            // existing. Which is the one failure it could not report,
            // and it must not read as a clean finish: a caller that saw
            // silence would conclude the loop simply had nothing more
            // to say.
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
/// to one of ours goes to that channel's own receiver — so nothing is
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

    let request = http::request::Request {
        method: http::request::Method::Post,
        // TODO: settled when the images are.
        path: "/".to_owned(),
        headers: Default::default(),
        body: Some(body),
    };

    // TODO: the port is settled when the images are.
    let (head, body) = match container.http_call(8080, request).await {
        Ok(answer) => answer,
        Err(error) => {
            return write(&scope, &response::Frame::Error(error.into())).await;
        }
    };

    // A status is the container's answer about itself, and this is the
    // one place to judge it: everything after here assumes chunks.
    if !(200..300).contains(&head.status) {
        let status = head.status;
        let error = format!("the agent container answered {status}");
        let error = Error(Value::String(error));
        return write(&scope, &response::Frame::Error(error)).await;
    }

    let mut chunks = Chunks::new(&head);
    match body {
        Body::Single(bytes) => {
            if !chunks.feed(&scope, &bytes).await {
                return;
            }
        }
        Body::Stream(mut body) => {
            while let Some(piece) = body.next().await {
                if !chunks.feed(&scope, &piece).await {
                    return;
                }
            }
        }
    }
    chunks.finish(&scope).await;
}

/// Chunks, out of however the body turned out to be delivered.
///
/// A body arrives in pieces the transport chose, which are not the
/// pieces the agent wrote — so something has to hold the leftovers and
/// hand over a chunk each time a whole one is there. This is that.
struct Chunks {
    /// Whether the body is an event stream.
    ///
    /// False means the whole body is one chunk and nothing delimits
    /// anything, so the buffer below simply accumulates until the body
    /// ends.
    events: bool,
    /// What has arrived and not yet been used.
    buffer: BytesMut,
}

impl Chunks {
    /// Read the head to learn how to read the body.
    ///
    /// Which is what a head is for, and why the answer is a head and a
    /// body rather than one thing: `text/event-stream` is many chunks
    /// and anything else is one, and the status line already had to
    /// arrive before either.
    ///
    /// Matching only the media type, since a charset or a boundary can
    /// follow it and neither changes what is being read.
    fn new(head: &http::response::Head) -> Self {
        let events = head
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .and_then(|(_, value)| value.split(';').next())
            .is_some_and(|kind| {
                kind.trim().eq_ignore_ascii_case("text/event-stream")
            });
        Chunks {
            events,
            buffer: BytesMut::new(),
        }
    }

    /// Take one piece of the body, sending whatever it completed.
    ///
    /// Returns whether to keep going. `false` means a chunk did not
    /// deserialize and the run has been ended with an error frame: a
    /// stream that has started saying things this crate cannot read is
    /// not one to keep relaying.
    async fn feed(&mut self, scope: &ScopeHandle, piece: &[u8]) -> bool {
        self.buffer.extend_from_slice(piece);
        if !self.events {
            return true;
        }
        while let Some(data) = self.event() {
            if !send(scope, &data).await {
                return false;
            }
        }
        true
    }

    /// Take the last chunk, if the body ended holding one.
    ///
    /// A unary body is entirely this: nothing is sent until here,
    /// because nothing said where it ended until it ended. An event
    /// stream usually has nothing left, and has something when the last
    /// event ran to the end of the body without a blank line after it.
    async fn finish(mut self, scope: &ScopeHandle) {
        if self.buffer.is_empty() {
            return;
        }
        if !self.events {
            send(scope, &self.buffer).await;
            return;
        }
        // Terminating the buffer is what makes `event` see the last
        // one, which a producer that closed the connection instead of
        // writing a blank line did not leave behind.
        self.buffer.extend_from_slice(b"\n\n");
        if let Some(data) = self.event() {
            send(scope, &data).await;
        }
    }

    /// Take the next whole event's data, if there is one.
    ///
    /// Server-sent events, and only as much of them as this needs.
    /// Events are separated by a blank line; within one, a `data:`
    /// field contributes a line to the payload, with one optional space
    /// after the colon.
    ///
    /// Every other field — `event`, `id`, `retry` — and every comment is
    /// skipped, because nothing here dispatches on them: one kind of
    /// thing arrives on this stream, and its own `type` says which. An
    /// event carrying no data is passed over entirely, which is what a
    /// keep-alive is.
    fn event(&mut self) -> Option<Bytes> {
        loop {
            let (end, separator) = self.boundary()?;
            let event = self.buffer.split_to(end);
            let _ = self.buffer.split_to(separator);

            // Sliced first, because `BytesMut` has a `split` of its
            // own and the one wanted here is the slice's.
            let event: &[u8] = &event;
            let mut data = BytesMut::new();
            for line in event.split(|byte| *byte == b'\n') {
                let line = line.strip_suffix(b"\r").unwrap_or(line);
                let Some(rest) = line.strip_prefix(b"data:") else {
                    continue;
                };
                let rest = rest.strip_prefix(b" ").unwrap_or(rest);
                if !data.is_empty() {
                    data.extend_from_slice(b"\n");
                }
                data.extend_from_slice(rest);
            }
            if !data.is_empty() {
                return Some(data.freeze());
            }
        }
    }

    /// Where the next event ends, and how many bytes separate it from
    /// the one after.
    ///
    /// A blank line, in any of the three spellings the format allows.
    fn boundary(&self) -> Option<(usize, usize)> {
        let bytes = &self.buffer[..];
        (0..bytes.len()).find_map(|at| {
            let rest = &bytes[at..];
            if rest.starts_with(b"\r\n\r\n") {
                Some((at, 4))
            } else if rest.starts_with(b"\n\n") || rest.starts_with(b"\r\r") {
                Some((at, 2))
            } else {
                None
            }
        })
    }
}

/// Send one chunk's JSON down the scope.
///
/// Returns whether to keep going. It is deserialized rather than
/// relayed, so that a body which has stopped being chunks is caught
/// here rather than by the caller.
async fn send(scope: &ScopeHandle, data: &[u8]) -> bool {
    match serde_json::from_slice(data) {
        Ok(chunk) => {
            write(scope, &response::Frame::Chunk(chunk)).await;
            true
        }
        Err(error) => {
            let error = Error(Value::String(error.to_string()));
            write(scope, &response::Frame::Error(error)).await;
            false
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
async fn mcp<S, W>(requests: S, scope: Arc<ScopeHandle>)
where
    S: futures_util::Stream<Item = (Bytes, W)> + Send + Unpin + 'static,
    W: HttpResponseWriter + Send + 'static,
{
    let mut requests = requests;
    let mut calls = tokio::task::JoinSet::new();

    while let Some((request, writer)) = requests.next().await {
        // Finished ones, so the set does not grow for the life of the
        // run. An agent makes a great many tool calls.
        while calls.try_join_next().is_some() {}

        calls.spawn(call(request, writer, Arc::clone(&scope)));
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

/// Relay one tool call out, and its answer back.
///
/// # The request is forwarded verbatim
///
/// It arrives as an encoded
/// [`Request`](crate::shared::http::request::Request), and a
/// [`channel_request::Frame`](crate::endpoints::agentic_loop::run::server::channel_request::Frame)
/// is that request with nothing in front of it — so the bytes go out as
/// they came in. Decoding one only to encode it again would be taking a
/// request apart and putting it back together to prove it could be.
///
/// # The answer is always finished
///
/// Including when it went wrong, and including when there was none. A
/// response that is never terminated leaves the agent unable to tell a
/// complete answer from a truncated one, and it has no other way to
/// find out.
async fn call<W>(request: Bytes, writer: W, scope: Arc<ScopeHandle>)
where
    W: HttpResponseWriter,
{
    let mut channel = scope.send_channel_request(&request).await;
    let mut writer = writer;

    while let Some(bytes) = channel.response_receiver.recv().await {
        // Whole client frames arrive here: a session forwards the
        // finish to the receiver and only then closes the channel, so
        // the stream ends by itself and there is nothing to do about a
        // finish but let it pass.
        let Ok(ClientFrame::ChannelResponse { payload, .. }) =
            ClientFrame::decode(&bytes)
        else {
            continue;
        };

        let written = match http::response::Frame::decode(payload) {
            Ok(http::response::Frame::Head(head)) => {
                writer.head(head).await.is_ok()
            }
            Ok(http::response::Frame::Body(body)) => {
                writer.body(body).await.is_ok()
            }
            // A frame this crate cannot read is not one to relay, and
            // the rest of the answer may still be good.
            Err(_) => true,
        };
        if !written {
            break;
        }
    }

    let _ = writer.finish().await;
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
async fn write(scope: &ScopeHandle, frame: &response::Frame) {
    let mut bytes = Vec::new();
    if frame.encode(&mut Writer::new(&mut bytes)).is_ok() {
        scope.send_response(&bytes).await;
    }
}
