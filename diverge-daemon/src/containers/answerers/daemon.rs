//! The container's `/daemon` connections, served under the grants its
//! program declared.

use std::sync::Arc;

use diverge_sdk::provider::client::Daemon as DaemonAnswerer;
use diverge_sdk::shared::containers::daemon as pair;
use tokio::sync::mpsc::UnboundedReceiver;

use super::Answerer;
use crate::judge::{Declared, Fixed, Who};
use diverge_sdk::daemon::grant::Converted;
use crate::serve::{self, PairFrames};

/// A connection the container's program opened, taken when the
/// container is a tool's — served under the grants its program
/// declared at its registration, none if it declared none, with the
/// agents its `attached` reaches read fresh — and declined for an
/// agent's, which asks the daemon nothing: served by
/// [`serve::serve_pair`], the same session and dispatch a socket gets,
/// for that standing — no token passes. Every frame the program sends
/// is a use of the container.
impl DaemonAnswerer for Answerer {
    type Frames = PairFrames;

    async fn connect(&self, _: u32, from_program: UnboundedReceiver<pair::client::Owned>) -> Option<Self::Frames> {
        let tool = self.tool?;
        let permissions = self.permissions.lock().await.clone();
        let who = Who::Tool(Declared {
            fixed: Arc::new(Fixed::new(self.sender.clone(), Converted::from(permissions))),
            tool,
        });
        Some(serve::serve_pair(Arc::clone(&self.daemon), who, from_program, Some(self.touched.clone())))
    }
}
