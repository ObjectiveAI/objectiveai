//! A command run in a container, its stream sent on.

use diverge_sdk::shared::containers::exec;
use diverge_sdk::wire::encode::Encode;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;

use super::cancelled;
use crate::containers::Opened;
use crate::serve::reply;

/// Run `command` in the container and send every frame of its stream,
/// as the family's answer `R`.
///
/// The rules:
///
/// 1. Each frame goes on as it comes, paced by the client's reading:
///    the command waits while the client falls behind.
/// 2. The stream's end is the command's: its exit or signal sent, or
///    the container stopped under it.
/// 3. A stream that breaks is the error, after what was sent.
/// 4. The client's cancel, or the client gone, drops the stream, which
///    abandons it — and that kills the command, at the proxy.
/// 5. The [`Opened`] is held for the whole of it: the command is a use
///    of the container, which holds it up.
pub async fn exec<R>(scope: &ScopeHandle, opened: &Opened, command: exec::request::Request) -> Result<(), String>
where
    R: From<exec::response::Owned> + Encode,
{
    let mut stream = opened.exec(command).await?;
    loop {
        tokio::select! {
            frame = stream.next() => match frame {
                // Rule 1.
                Some(Ok(frame)) => reply::reply(scope, &R::from(frame)).await,
                // Rule 3.
                Some(Err(error)) => return Err(error),
                // Rule 2.
                None => return Ok(()),
            },
            // Rule 4.
            () = cancelled(scope) => return Ok(()),
        }
    }
}
