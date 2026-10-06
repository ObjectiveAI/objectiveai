//! One answer that may be long in coming, and may be called off.

use super::{Cancel, one_shot};
use crate::wire::client::handle::Handle;
use crate::wire::client::scope::Scope;
use crate::wire::decode::Decode;
use crate::wire::encode::Encode;

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
/// taken. [`wait`](Self::wait) reads the one answer; [`cancel`](Self::cancel)
/// asks the daemon to call it off, and the answer then says which came
/// first.
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

    /// Wait for the one answer: the daemon's, whichever it is — after
    /// a cancel, the one that says whether the cancel came first.
    pub async fn wait<E>(mut self) -> Result<A, one_shot::Error<std::convert::Infallible, E>>
    where
        A: for<'a> Decode<'a, Error = E>,
    {
        let bytes = self.scope.response_receiver.recv().await.ok_or(one_shot::Error::Closed)?;
        one_shot::answer(&bytes)
    }
}
