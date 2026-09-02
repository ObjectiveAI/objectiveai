//! A run that started: its id, and its events.

/// What [`run`](fn@super::run) hands back: the run's id — which a fresh
/// run's session is also named after, and which the approval route
/// needs — and the event stream.
pub struct Run<S> {
    /// The gateway's `run_id`.
    pub run_id: String,
    /// The events, as they come.
    pub events: S,
}
