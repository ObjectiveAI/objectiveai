//! One ask, one answer: the relay every one-message exchange rides.

use bytes::Bytes;
use diverge_sdk::wire::server::channel::Channel;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::answer::{self, Answer};
use crate::own::Own;
use crate::proxy::{Begun, Proxy};

/// Why an ask has no answer.
pub enum Asked {
    /// The ask would not encode — the one failure that is this side's
    /// own, which asking again cannot change.
    Encode,
    /// The finish with nothing before it: the caller could not serve
    /// the ask.
    Empty,
    /// The channel closed without a finish: the connection went, or
    /// the scope did, and nothing is re-asked.
    Died,
}

/// Ask once on `scope` and take the answer: the first message on the
/// channel, then the finish. Extras are ignored rather than obeyed; a
/// channel that dies, or finishes with nothing, is the failure it is.
/// No retry, per the wire: neither a vault operation nor a mounted
/// file's write is safe to repeat.
pub async fn ask(scope: &ScopeHandle, payload: &[u8]) -> Result<Bytes, Asked> {
    let mut channel = scope.send_channel_request(payload).await;
    let mut first: Option<Bytes> = None;
    loop {
        match answer::next(&mut channel).await {
            Some(Answer::Frame(bytes)) => {
                first.get_or_insert(bytes);
            }
            Some(Answer::Finish) => return first.ok_or(Asked::Empty),
            None => return Err(Asked::Died),
        }
    }
}

/// Open one of the proxy's own asks on the begin scope, waiting for
/// the scope if the connection has not begun — the park — and hand
/// back the channel its answer arrives on, with the scope it rides.
pub async fn open(proxy: &Proxy, own: Own<'_>) -> Result<(Begun, Channel), Asked> {
    let begun = proxy.begun().await.ok_or(Asked::Died)?;
    let payload = own.encoded(begun.family).ok_or(Asked::Encode)?;
    let channel = begun.scope.send_channel_request(&payload).await;
    Ok((begun, channel))
}

/// One of the proxy's own asks on the begin scope, and its one
/// answer: [`open`] and [`ask`] in one.
pub async fn own(proxy: &Proxy, own: Own<'_>) -> Result<Bytes, Asked> {
    let begun = proxy.begun().await.ok_or(Asked::Died)?;
    let payload = own.encoded(begun.family).ok_or(Asked::Encode)?;
    ask(&begun.scope, &payload).await
}
