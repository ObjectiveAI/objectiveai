//! A stream that yields exactly one item, then completes. Lets unary
//! delegations be lifted into a `Stream` return type without machinery.

use futures::Stream;

pub struct StreamOnce<T>(Option<T>);

impl<T> StreamOnce<T> {
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
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::task::Poll::Ready(self.as_mut().get_mut().0.take())
    }
}
