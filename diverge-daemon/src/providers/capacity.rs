//! Asking a provider its capacity, again and again, for the
//! connection's life.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::capacity::client::execute;
use diverge_sdk::wire::client::handle::Handle;

use crate::daemon::{Capacity, Daemon, Kind};

/// Ask the provider on `handle` its capacity, swap the answer into
/// `cell`, wait, and ask again, until the connection ends.
///
/// # The wait starts after the swap
///
/// Ask, swap, then wait `capacity_poll` of the configuration — or
/// less, when the cell is nudged by something that changed the
/// capacity — then ask again. A slow provider stretches the interval
/// rather than piling asks up; a nudge that lands mid-ask is kept
/// and ends the next wait at once. Nothing waits for the answer: a
/// reader reads what the cell holds now.
///
/// # What is said, and what is not
///
/// An answer that differs from the last is the word that the
/// provider's kind changed, on which a list kept open reads the
/// items again and sends the item as changed; the same answer again
/// is nothing. A provider
/// that did not answer — the scope finished bare, or the answer did
/// not parse — leaves the last answer standing. The connection gone
/// — the request not sent, or the connection closed before the
/// answer — ends this; the cell goes with the connection's slot.
pub async fn poll_capacity(daemon: Arc<Daemon>, identity: Identity, handle: Handle, cell: Arc<Capacity>) {
    loop {
        match execute::execute(&handle).await {
            Ok(frame) => {
                if cell.swap(frame) {
                    daemon.live.changed(Kind::of_provider(&identity));
                }
            }
            Err(execute::ExecuteError::Send(_) | execute::ExecuteError::Closed) => return,
            Err(_) => {}
        }
        tokio::select! {
            () = tokio::time::sleep(daemon.capacity_poll) => {}
            () = cell.nudged() => {}
        }
    }
}
