//! The loop over what the caller opens.

use std::pin::pin;
use std::sync::Arc;

use futures_util::future::{self, Either};

use super::super::family::{Family, Opened};
use super::super::run::Run;
use super::{filetree, postgres, read, write};
use crate::decode::Decode as _;
use crate::frame::client::ClientFrame;

/// How a scope stopped being served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    /// The caller said stop, or leave.
    Stopped,
    /// The container is gone, or the run a connector joined is over.
    Over,
    /// The caller went away: the connection ended.
    Gone,
}

/// Serve every channel the caller opens until the scope ends, and say
/// how.
///
/// Each frame off the inbox is one channel the caller opened, and
/// what it asks decides the task: the shared five here, the family's
/// own through [`Family::serve`]. A request that will not decode is
/// finished with nothing — the wire's could-not-serve — and the loop
/// goes on. A stop ends the loop at once; channels still being served
/// are the handler's to end.
///
/// Raced against [`Run::over`], because both are silent for
/// arbitrarily long, and
/// [`recv_channel_request`](crate::server::scope_handle::ScopeHandle::recv_channel_request)
/// is cancel-safe, so the side that loses the race loses nothing.
pub(crate) async fn serve<F: Family>(run: &Arc<Run>) -> End {
    loop {
        let request = pin!(run.scope.recv_channel_request());
        let over = pin!(run.over.notified());
        let bytes = match future::select(request, over).await {
            Either::Left((Some(bytes), _)) => bytes,
            Either::Left((None, _)) => return End::Gone,
            Either::Right(_) => return End::Over,
        };
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        let Ok(request) = F::Request::decode(payload) else {
            run.finish(channel).await;
            continue;
        };
        let run = Arc::clone(run);
        match F::classify(request) {
            Opened::Stop => return End::Stopped,
            Opened::Filetree => run.spawn(filetree::filetree::<F>(Arc::clone(&run), channel)).await,
            Opened::Read(path) => run.spawn(read::read::<F>(Arc::clone(&run), channel, path)).await,
            Opened::Write { write_id, path } => {
                run.spawn(write::write::<F>(Arc::clone(&run), channel, write_id, path)).await
            }
            Opened::Postgres(connection_id) => {
                run.spawn(postgres::postgres(Arc::clone(&run), channel, connection_id)).await
            }
            Opened::Exchange(exchange) => run.spawn(F::serve(Arc::clone(&run), channel, exchange)).await,
        }
    }
}
