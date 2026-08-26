//! A stream that yields exactly one item, then completes.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;

/// A stream that yields exactly one item, then completes.
///
/// Ported from the api crate's `util::StreamOnce`: it re-emits the
/// first item [`fetch`](crate::fetch::fetch) consumed for its
/// error-or-first-chunk check, and — unlike `stream::iter` — it is
/// [`Unpin`] whenever its item is, which is what lets `fetch` promise
/// `Unpin` on the stream it returns.
pub struct StreamOnce<T>(Option<T>);

impl<T> StreamOnce<T> {
    /// Creates a new single-item stream containing the given item.
    pub fn new(item: T) -> Self {
        Self(Some(item))
    }
}

impl<T> Stream for StreamOnce<T>
where
    T: Unpin,
{
    type Item = T;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.as_mut().get_mut().0.take())
    }
}
