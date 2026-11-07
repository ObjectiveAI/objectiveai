//! One answer that may be long in coming, and may be called off.

use super::{Cancel, one_shot};
use crate::wire::client::handle::Handle;
use crate::wire::client::scope::Scope;
use crate::wire::decode::Decode;
use crate::wire::encode::Encode;
use crate::wire::frame;

/// Open a scope whose one answer may be waited for or cancelled.
pub async fn execute<Q, A>(handle: &Handle, request: &Q) -> Result<Pending<A>, super::OpenError<Q::Error>>
where
    Q: Encode,
    A: for<'a> Decode<'a>,
{
    let scope = one_shot::open::<Q, ()>(handle, request).await.map_err(super::OpenError::from)?;
    let cancel = Cancel::new(handle.clone(), scope.scope);
    Ok(Pending { scope, cancel, answer: std::marker::PhantomData })
}

/// A scope opened and not yet answered: a message sent and not yet
/// taken. [`wait`](Self::wait) reads the one answer, or the finish
/// with none before it; [`cancel`](Self::cancel) asks the daemon to
/// call it off, and whether an answer comes then says which came first.
#[must_use = "a scope that is not waited on grows a queue nobody reads"]
#[derive(Debug)]
pub struct Pending<A> {
    scope: Scope,
    cancel: Cancel,
    answer: std::marker::PhantomData<fn() -> A>,
}

impl<A> Pending<A> {
    /// The scope's number, for a caller that keeps its own books.
    pub fn scope(&self) -> u32 {
        self.scope.scope
    }

    /// The cancel, to hold apart from the wait: a caller that waits on
    /// one task and cancels from another clones this.
    pub fn cancel(&self) -> &Cancel {
        &self.cancel
    }

    /// Wait for the one answer: the daemon's, whichever it is, or none.
    ///
    /// The first frame on the scope decides, by these rules:
    ///
    /// 1. **A response.** The answer, decoded: `Ok(Some(answer))`,
    ///    whichever variant the daemon chose; the finish that follows
    ///    it is never read, as [`one_shot`] leaves it.
    /// 2. **The finish.** No answer before it — the bare finish:
    ///    `Ok(None)`. A cancel that arrived in time ends a scope so, and
    ///    so does a request the daemon could not serve, as
    ///    [`endpoints`](crate::daemon::endpoints) states. A cancel that
    ///    came too late changes nothing, and the answer comes as rule 1.
    /// 3. **Anything else.** [`Error::Misrouted`](one_shot::Error::Misrouted):
    ///    no other frame can be the first on a main stream.
    /// 4. **Nothing.** The connection ended first:
    ///    [`Error::Closed`](one_shot::Error::Closed). There is no
    ///    timeout.
    pub async fn wait<E>(mut self) -> Result<Option<A>, one_shot::Error<std::convert::Infallible, E>>
    where
        A: for<'a> Decode<'a, Error = E>,
    {
        let bytes = self.scope.response_receiver.recv().await.ok_or(one_shot::Error::Closed)?;
        match frame::server::ServerFrame::decode(&bytes).map_err(one_shot::Error::Frame)? {
            frame::server::ServerFrame::Response { payload, .. } => {
                A::decode(payload).map(Some).map_err(one_shot::Error::Response)
            }
            frame::server::ServerFrame::ResponseFinish { .. } => Ok(None),
            _ => Err(one_shot::Error::Misrouted),
        }
    }
}
