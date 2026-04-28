use futures::stream::BoxStream;

/// Result of `Thread.run_streamed()` — an async stream of
/// [`super::super::ThreadEvent`]. Mirrors `StreamedTurn` in `thread.py:39-43`
/// (`@dataclass(frozen=True) StreamedTurn { events: AsyncIterator[ThreadEvent] }`).
///
/// Not serializable: the `events` field carries a live stream handle, not
/// data that ever appears on the wire.
pub struct StreamedTurn {
    pub events: BoxStream<'static, super::super::ThreadEvent>,
}
