//! Starting an agent container.

use std::fmt;
use std::sync::Arc;

use super::super::{channel_request, channel_response, request};
use super::execute_handle::ExecuteHandle;
use super::ExecuteStream;
use crate::client::handle::{Handle, SendError};
use crate::client::{
    Answerers, CommandRunner, ConnectionAuthorizer, FuseServer, McpServer, OciStore, PostgresDialer, Vault,
};
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::endpoints::containers::agents::run::server;
use crate::endpoints::containers::client::{Ask, Encoders, Scoped, Writes, encoded, serve};
use crate::frame;
use crate::shared::containers::postgres;
use crate::shared::containers::response::Id;
use crate::shared::containers::write_bytes;
use crate::shared::containers::response::VolumeHeld;
use crate::shared::error::Error;

/// Run an agent container, and hold it.
///
/// The rest of this crate describes the exchange; this performs it.
/// The request goes out, the main stream's first frame decides —
/// the container's id, or the provider's error, or a finish with
/// nothing before it, which is the provider that could not serve the
/// run at all — and then two things start: a task that reads every
/// channel the provider opens on the scope and answers it through
/// `answerers`, and the [`ExecuteHandle`] a caller keeps.
///
/// # What comes back, and what does not
///
/// The id names the container to anything outside — a connector, a
/// later request — and the handle is the scope: for as long as it is
/// held the container runs, and every channel a caller may open into
/// the container is a method on it. The main stream carries the
/// agent's conversation after the id, which the [`ExecuteStream`] hands out
/// chunk by chunk until the run ends — the end
/// [`ExecuteHandle::wait`] reports.
///
/// # Dropping the handle ends nothing
///
/// The scope lives until the provider finishes it, which a
/// [`stop`](ExecuteHandle::stop) provokes and a dropped connection
/// forces. A caller that drops every clone of the handle without
/// stopping leaves the container running until the connection goes;
/// the serving task goes on answering the provider's asks until the
/// scope's request stream ends, which the router closes with the
/// scope.
pub async fn execute<O, A, P, C, V, M, F>(
    handle: &Handle,
    request: &request::Frame,
    answerers: Answerers<O, A, P, C, V, M, F>,
) -> Result<(Id, ExecuteHandle, ExecuteStream), ExecuteError>
where
    O: OciStore + 'static,
    A: ConnectionAuthorizer + 'static,
    P: PostgresDialer + 'static,
    C: CommandRunner + 'static,
    V: Vault + 'static,
    M: McpServer + 'static,
    F: FuseServer + 'static,
{
    let mut payload = Vec::new();
    request
        .encode(&mut Writer::new(&mut payload))
        .map_err(ExecuteError::Request)?;
    let mut scope = handle
        .send_request(&payload)
        .await
        .map_err(ExecuteError::Send)?;
    let bytes = scope
        .response_receiver
        .recv()
        .await
        .ok_or(ExecuteError::Closed)?;
    let envelope = frame::server::ServerFrame::decode(&bytes).map_err(ExecuteError::Frame)?;
    let payload = match envelope {
        frame::server::ServerFrame::Response { payload, .. } => payload,
        frame::server::ServerFrame::ResponseFinish { .. } => return Err(ExecuteError::Unanswered),
        _ => return Err(ExecuteError::Misrouted),
    };
    let frame = match server::response::Frame::decode(payload) {
        Ok(frame) => frame,
        Err(error) => {
            // The provider believes the run started. A caller that
            // cannot read the id cannot name the container either, so
            // the run is stopped rather than left to the connection's
            // end.
            stop(handle, scope.scope).await;
            return Err(ExecuteError::Response(error));
        }
    };
    let id = match frame {
        server::response::Frame::Id(id) => id,
        server::response::Frame::VolumeHeld(refused) => {
            return Err(ExecuteError::VolumeHeld(refused));
        }
        server::response::Frame::Error(error) => return Err(ExecuteError::Provider(error)),
        // The agent speaks only after the id; a chunk first is a
        // provider out of order, which is a frame this end cannot
        // place.
        server::response::Frame::Chunk(_) => return Err(ExecuteError::Misrouted),
    };

    let writes = Arc::new(Writes::new());
    tokio::spawn(serve::serve(
        scope.request_receiver,
        handle.clone(),
        scope.scope,
        Arc::clone(&writes),
        answerers,
        decode_ask,
        ENCODERS,
    ));
    let scoped = Arc::new(Scoped::new(handle.clone(), scope.scope, writes, scope.response_receiver));
    Ok((id, ExecuteHandle::new(Arc::clone(&scoped)), ExecuteStream::new(scoped)))
}

/// The stop, sent for a run this end cannot hold.
async fn stop(handle: &Handle, scope: u32) {
    let mut payload = Vec::new();
    if channel_request::Frame::Stop.encode(&mut Writer::new(&mut payload)).is_ok() {
        let _ = handle.send_channel_request(scope, &payload).await;
    }
}

/// This family's server-opened ask, read into the one owned form.
fn decode_ask(payload: &[u8]) -> Option<Ask> {
    server::channel_request::Frame::decode(payload).ok().map(Ask::from)
}

/// This family's frames, for the shared answers.
const ENCODERS: Encoders = Encoders {
    postgres_half,
    write_body,
    write_error,
};

fn postgres_half(connection_id: u32) -> Option<Vec<u8>> {
    encoded(&channel_request::Frame::Postgres(postgres::request::Postgres { connection_id }))
}

fn write_body(piece: &[u8]) -> Option<Vec<u8>> {
    encoded(&channel_response::write_bytes::Frame::Body(write_bytes::response::Frame(piece)))
}

fn write_error(error: &Error) -> Option<Vec<u8>> {
    encoded(&channel_response::write_bytes::Frame::Error(error.clone()))
}

/// A run that never started.
///
/// Seven of these are this end's view of something going wrong. The
/// last two are the provider saying so itself, and they are the only
/// ones that mean the exchange worked: a volume the request named is
/// under a stat, an edit or a delete, or the provider could not run
/// the container.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The connection ended before anything came back.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it: the provider could
    /// not serve the run at all.
    Unanswered,
    /// A frame arrived that does not belong on the main stream.
    Misrouted,
    /// The response frame did not parse.
    Response(server::response::FrameError),
    /// The provider refused the run: the named volume is under a
    /// stat, an edit or a delete. Nothing was fetched and nothing was
    /// deployed; wait for that to end, or name another volume, and
    /// ask again.
    VolumeHeld(VolumeHeld),
    /// The provider could not run the container, and said so.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => write!(f, "agents run request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the run answered"),
            ExecuteError::Frame(error) => write!(f, "agents run answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the run finished without an answer"),
            ExecuteError::Misrouted => {
                f.write_str("a frame arrived that does not belong on the main stream")
            }
            ExecuteError::Response(error) => write!(f, "agents run answer did not parse: {error}"),
            ExecuteError::VolumeHeld(refused) => write!(
                f,
                "the volume `{}` is under a stat, an edit or a delete",
                refused.name
            ),
            ExecuteError::Provider(_) => f.write_str("the provider could not run the container"),
        }
    }
}

impl std::error::Error for ExecuteError {
    /// [`Provider`](ExecuteError::Provider) has no source: what it
    /// carries is not a Rust error and deliberately does not implement
    /// one — see [`shared::error::Error`](crate::shared::error::Error).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::Misrouted
            | ExecuteError::VolumeHeld(_)
            | ExecuteError::Provider(_) => None,
        }
    }
}
