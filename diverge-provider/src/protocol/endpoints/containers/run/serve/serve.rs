//! The loop over what the caller opens.

use std::sync::Arc;

use super::super::family::{Opened, Runs};
use super::super::run::Run;
use super::{daemon, exec, filetree, frontend, fuse_mount, info, postgres, read, secrets, terminal, transfer, write};
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;

/// How a scope stopped being served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    /// The caller said stop, or leave.
    Stopped,
    /// The container is gone.
    Over,
    /// The caller went away: the connection ended, or the caller reads
    /// no more of the scope's responses.
    Gone,
}

/// Serve every channel the caller opens until the scope ends, and say
/// how.
///
/// Each frame off the inbox is one channel the caller opened, and
/// what it asks decides the task: the shared ones here, the family's
/// own through [`Family::serve`](super::super::family::Family::serve).
/// A request that will not decode is
/// finished with nothing — the wire's could-not-serve — and the loop
/// goes on. A stop ends the loop at once; channels still being served
/// are the handler's to end.
///
/// Raced against [`Run::over`] and against the caller abandoning the
/// scope's responses, because all three are silent for arbitrarily
/// long, and
/// [`recv_channel_request`](diverge_sdk::wire::server::scope_handle::ScopeHandle::recv_channel_request)
/// is cancel-safe, so the side that loses the race loses nothing. A
/// scope whose responses nobody reads is a run nobody is watching:
/// it ends as [`End::Gone`], and the handler tears the run down.
pub(crate) async fn serve<F: Runs>(run: &Arc<Run>) -> End {
    loop {
        let bytes = tokio::select! {
            received = run.scope.recv_channel_request() => match received {
                Some(bytes) => bytes,
                None => return End::Gone,
            },
            () = run.over.notified() => return End::Over,
            () = run.scope.responses_gone() => return End::Gone,
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
            Opened::Exec(command) => run.spawn(exec::exec(Arc::clone(&run), channel, command)).await,
            Opened::TerminalOpen(size) => run.spawn(terminal::open(Arc::clone(&run), channel, size)).await,
            Opened::TerminalAttach(id) => run.spawn(terminal::attach(Arc::clone(&run), channel, id)).await,
            Opened::TerminalStdin(id, bytes) => run.spawn(terminal::stdin(Arc::clone(&run), channel, id, bytes)).await,
            Opened::TerminalResize(id, size) => run.spawn(terminal::resize(Arc::clone(&run), channel, id, size)).await,
            Opened::TerminalClose(id) => run.spawn(terminal::close(Arc::clone(&run), channel, id)).await,
            Opened::Frontend(request) => run.spawn(frontend::frontend::<F>(Arc::clone(&run), channel, request)).await,
            Opened::Write { write_id, path } => {
                run.spawn(write::write::<F>(Arc::clone(&run), channel, write_id, path)).await
            }
            Opened::Transfer { path, id, destination } => {
                run.spawn(transfer::transfer::<F>(Arc::clone(&run), channel, path, id, destination)).await
            }
            Opened::Daemon(connection_id) => {
                run.spawn(daemon::daemon(Arc::clone(&run), channel, connection_id)).await
            }
            Opened::Postgres(connection_id) => {
                run.spawn(postgres::postgres(Arc::clone(&run), channel, connection_id)).await
            }
            Opened::Info => run.spawn(info::info(Arc::clone(&run), channel)).await,
            Opened::Secrets(values) => run.spawn(secrets::secrets(Arc::clone(&run), channel, values)).await,
            Opened::FuseMount(request) => {
                run.spawn(fuse_mount::fuse_mount::<F>(Arc::clone(&run), channel, request)).await
            }
            Opened::Exchange(exchange) => run.spawn(F::serve(Arc::clone(&run), channel, exchange)).await,
        }
    }
}
