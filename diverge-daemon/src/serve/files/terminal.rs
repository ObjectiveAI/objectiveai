//! A terminal in a container, held for as long as the scope is open.

use diverge_sdk::daemon::endpoints::agents::terminal::client::channel_request;
use diverge_sdk::shared::containers::terminal;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::Encode;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;

use crate::containers::Opened;
use crate::serve::reply;

/// Open a terminal at `size` in the container, send its output as the
/// family's answer `R`, and serve the client's channels, until one of
/// the things that end it.
///
/// The rules:
///
/// 1. The terminal is the scope: the provider holds it by an id this
///    end keeps, and nobody else learns it.
/// 2. Its output goes on as it comes, paced by the client's reading:
///    the shell waits while the client falls behind.
/// 3. `Stdin` and `Resize` are served as they arrive, beside the
///    output, and each channel is finished once the provider has done
///    it; a failure is the scope's error.
/// 4. The scope ends at the shell's end — its exit or signal sent, or
///    the container stopped under it —, at the client's `Close`, or
///    at the client gone; a stream that breaks is the error, after
///    what was sent.
/// 5. However it ends, the terminal is closed: the shell killed, and
///    the provider forgets it.
/// 6. The [`Opened`] is held for the whole of it: the terminal is a
///    use of the container, which holds it up.
///
/// Every family's channels are the same bytes, read here as the
/// agents'.
pub async fn terminal<R>(scope: &ScopeHandle, opened: &Opened, size: terminal::open::Open) -> Result<(), String>
where
    R: From<terminal::output::Owned> + Encode,
{
    // Rule 1.
    let id = opened.terminal_open(size).await?;
    let ended = relay::<R>(scope, opened, id).await;
    // Rule 5. A close that fails is a container already gone, which
    // took the shell with it.
    let _ = opened.terminal_close(id).await;
    ended
}

/// Rules 2 to 4: the output out, the channels in.
async fn relay<R>(scope: &ScopeHandle, opened: &Opened, id: u32) -> Result<(), String>
where
    R: From<terminal::output::Owned> + Encode,
{
    let mut output = opened.terminal_attach(id).await?;
    let mut serving = FuturesUnordered::new();
    loop {
        tokio::select! {
            frame = output.next() => match frame {
                // Rule 2.
                Some(Ok(frame)) => reply::reply(scope, &R::from(frame)).await,
                // Rule 4.
                Some(Err(error)) => return Err(error),
                None => return Ok(()),
            },
            request = scope.recv_channel_request() => {
                // Rule 4: the client gone.
                let Some(bytes) = request else {
                    return Ok(());
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    // Rule 4.
                    Ok(channel_request::Frame::Close) => return Ok(()),
                    // Rule 3.
                    Ok(channel_request::Frame::Stdin(bytes)) => {
                        let bytes = bytes.to_vec();
                        serving.push(Box::pin(async move { (channel, opened.terminal_stdin(id, bytes).await) }) as std::pin::Pin<Box<dyn Future<Output = (u32, Result<(), String>)> + Send + '_>>);
                    }
                    Ok(channel_request::Frame::Resize(size)) => {
                        serving.push(Box::pin(async move { (channel, opened.terminal_resize(id, size).await) }));
                    }
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            Some((channel, done)) = serving.next(), if !serving.is_empty() => {
                scope.send_channel_response_finish(channel).await;
                done?;
            }
        }
    }
}
