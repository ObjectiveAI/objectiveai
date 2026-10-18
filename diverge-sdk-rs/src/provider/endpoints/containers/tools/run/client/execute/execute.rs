//! Starting a tool container.

use std::fmt;
use std::sync::Arc;

use super::super::{channel_request, channel_response, request};
use super::execute_handle::ExecuteHandle;
use super::ExecuteStream;
use crate::wire::client::handle::{Handle, SendError};
use crate::provider::client::{
    Answerers, Daemon, FuseServer, McpServer, OciStore, PostgresDialer, Registrar,
};
use crate::wire::decode::Decode as _;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::containers::tools::run::server;
use crate::provider::endpoints::containers::client::{Ask, Encoders, Scoped, Writes, encoded, serve};
use crate::wire::frame;
use crate::shared::containers::daemon;
use crate::shared::containers::postgres;
use crate::shared::containers::response::Id;
use crate::shared::containers::write_bytes;
use crate::shared::containers::response::{
    VolumeFuseServed, VolumeHeld, VolumeMode, VolumeMountUnsupported, VolumeMounted,
};
use crate::shared::error::Error;

/// Run a tool container, and hold it.
///
/// The rest of this crate describes the exchange; this performs it.
/// The request goes out, and at once a task starts that reads every
/// channel the provider opens on the scope and answers it through
/// `answerers`; then the main stream's first frame decides — the
/// container's id, or the provider's error, or a finish with nothing
/// before it, which is the provider that could not serve the run at
/// all — and the [`ExecuteHandle`] a caller keeps is made.
///
/// # The serving task starts before the first frame is read
///
/// The provider asks before it answers: the image's manifest and
/// blobs, the container's registration — its secrets resolved and
/// fed — and the FUSE mounts' first reads are all channels it opens
/// on the scope before it sends the id. A run that read the id first
/// and served the channels after would wait for an id the provider
/// is waiting on it to earn, and neither would move. So the serving
/// task is spawned right after the request goes out; a refusal ends
/// it with the scope, since the router closes the scope's request
/// stream with the scope.
///
/// # What comes back, and what does not
///
/// The id names the container to anything outside — a later request
/// of the runner's — and the handle is the scope: for as long as it
/// is held the container runs, and every channel a caller may open
/// into the container is a method on it. The main stream carries the
/// container's activity after the id, which the [`ExecuteStream`]
/// hands out one word at a time until the run ends — the end
/// [`ExecuteHandle::ended`] reports.
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
pub async fn execute<O, T, P, D, M, F>(
    handle: &Handle,
    request: &request::Frame,
    answerers: Answerers<O, T, P, D, M, F>,
) -> Result<(Id, ExecuteHandle, ExecuteStream), ExecuteError>
where
    O: OciStore + 'static,
    T: Registrar + 'static,
    P: PostgresDialer + 'static,
    D: Daemon + 'static,
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
    // Served from here on: the provider's asks precede its id, and
    // the registration among them is answered by this end.
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
        server::response::Frame::VolumeMounted(refused) => {
            return Err(ExecuteError::VolumeMounted(refused));
        }
        server::response::Frame::VolumeFuseServed(refused) => {
            return Err(ExecuteError::VolumeFuseServed(refused));
        }
        server::response::Frame::VolumeMountUnsupported(refused) => {
            return Err(ExecuteError::VolumeMountUnsupported(refused));
        }
        server::response::Frame::VolumeMode(refused) => {
            return Err(ExecuteError::VolumeMode(refused));
        }
        server::response::Frame::Error(error) => return Err(ExecuteError::Provider(error)),
        // No exchange is in flight before the id; either word first
        // is a provider out of order, which is a frame this end
        // cannot place.
        server::response::Frame::Active | server::response::Frame::Inactive(_) => {
            return Err(ExecuteError::Misrouted);
        }
    };

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
    daemon_half,
    postgres_half,
    write_body,
    write_error,
};

fn daemon_half(connection_id: u32) -> Option<Vec<u8>> {
    encoded(&channel_request::Frame::Daemon(daemon::request::Daemon { connection_id }))
}

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
/// last seven are the provider saying so itself, and they are the
/// only ones that mean the exchange worked: a volume the request
/// named is held, mounted, FUSE served, of a mode the provider does
/// not mount, or not in the mode its mount names; the request names a
/// FUSE mount the provider will not make; or the provider could not
/// run the container.
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
    /// stat, a read, a write, a filetree, an edit or a delete.
    /// Nothing was fetched and nothing was deployed; wait for that to
    /// end, or name another volume, and ask again.
    VolumeHeld(VolumeHeld),
    /// The provider refused the run: a running container has the
    /// named volume, and the provider mounts a volume of its mode
    /// once. Nothing was fetched and nothing was deployed; stop that
    /// container, or name another volume, and ask again.
    VolumeMounted(VolumeMounted),
    /// The provider refused the run: a FUSE serve has the named
    /// volume, and the provider does not mount a volume of its mode
    /// beside one. Nothing was fetched and nothing was deployed; stop
    /// that FUSE serve, or name another volume, and ask again.
    VolumeFuseServed(VolumeFuseServed),
    /// The provider refused the run: it does not mount a volume of
    /// the named volume's mode. Nothing was fetched and nothing was
    /// deployed; edit the volume's mode, or ask another provider.
    VolumeMountUnsupported(VolumeMountUnsupported),
    /// The provider refused the run: the named volume is not in the
    /// mode the mount means it to have. Nothing was fetched and
    /// nothing was deployed; edit the volume or the mount, and ask
    /// again.
    VolumeMode(VolumeMode),
    /// The provider could not run the container, and said so.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "the request never went out: {error}"),
            ExecuteError::Request(error) => write!(f, "tools run request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the run answered"),
            ExecuteError::Frame(error) => write!(f, "tools run answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("the run finished without an answer"),
            ExecuteError::Misrouted => {
                f.write_str("a frame arrived that does not belong on the main stream")
            }
            ExecuteError::Response(error) => write!(f, "tools run answer did not parse: {error}"),
            ExecuteError::VolumeHeld(refused) => write!(
                f,
                "the volume `{}` is under a stat, a read, a write, a filetree, an edit or a delete",
                refused.name
            ),
            ExecuteError::VolumeMounted(refused) => write!(
                f,
                "a running container has the volume `{}`, and the provider mounts a volume of its mode once",
                refused.name
            ),
            ExecuteError::VolumeFuseServed(refused) => write!(
                f,
                "a FUSE serve has the volume `{}`, and the provider does not mount a volume of its mode beside one",
                refused.name
            ),
            ExecuteError::VolumeMountUnsupported(refused) => write!(
                f,
                "the provider does not mount a volume of the mode the volume `{}` is in",
                refused.name
            ),
            ExecuteError::VolumeMode(refused) => write!(
                f,
                "the volume `{}` is in mode `{:?}`, not the mode the mount names",
                refused.name, refused.mode
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
            | ExecuteError::VolumeMounted(_)
            | ExecuteError::VolumeFuseServed(_)
            | ExecuteError::VolumeMountUnsupported(_)
            | ExecuteError::VolumeMode(_)
            | ExecuteError::Provider(_) => None,
        }
    }
}
