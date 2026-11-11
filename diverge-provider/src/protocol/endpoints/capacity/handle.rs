//! Answering a capacity request, from a scope and a reporter.

use diverge_sdk::shared::Identity;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::protocol::capacity_reporter::CapacityReporter;

/// Answer with the four numbers, and end the scope.
///
/// The reporter is asked once, for the connection's identity, and
/// its answer is the response. Nothing is checked against the
/// capabilities: the endpoint is served whatever they hold out, and
/// a number for a resource the provider does not keep is the
/// reporter's to answer as any number, which the caller does not
/// read.
///
/// # A reporter that will not answer is a bare finish
///
/// The endpoint defines no failure frame, so a reporter's error has
/// nowhere to go: the scope finishes with no response before it,
/// which the wire reads as the request not served, and the caller's
/// executor as unanswered. A response that will not ENCODE ends the
/// same way.
///
/// # Nothing is decoded here
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, so a scope arriving here already
/// parsed as a capacity ask — which carried nothing.
pub async fn handle<C>(scope: ScopeHandle, client_identity: &Identity, reporter: &C)
where
    C: CapacityReporter,
{
    // The answer is taken off the result at once: an error is dropped
    // here, before anything is awaited, so nothing of the reporter's
    // crosses the task's awaits.
    let frame = reporter.capacity(client_identity).await.ok();
    if let Some(frame) = frame {
        let mut buffer = Vec::new();
        if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
            scope.send_response(&buffer).await;
        }
    }
    scope.send_response_finish().await;
}
