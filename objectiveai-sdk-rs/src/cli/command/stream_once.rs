//! A stream that yields exactly one item, then completes. Lets unary
//! delegations be lifted into a `Stream` return type without machinery.

use futures::Stream;

pub struct StreamOnce<T>(Option<T>);

impl<T> StreamOnce<T> {
    pub fn new(item: T) -> Self {
        Self(Some(item))
    }
}

impl<T> Stream for StreamOnce<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // SAFETY: We hold `Option<T>`. Calling `take()` moves the
        // inner `T` out by value but leaves the option location
        // (which is what's pinned) in place — we never project into
        // the still-stored `T`. Soundness only requires that we never
        // observe the inner `T` through a pinned borrow before taking
        // ownership, which we don't.
        let this = unsafe { std::pin::Pin::get_unchecked_mut(self) };
        std::task::Poll::Ready(this.0.take())
    }
}
