//! The word that a provider's capacity was swapped, for the lists
//! kept open.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::capacity::server::response::Frame;
use tokio::sync::broadcast;

/// How many swaps a listing may fall behind before it is told so and
/// reads everything again.
const SWAPS_BEHIND: usize = 256;

/// One provider's capacity, swapped: which provider, and the answer
/// cached now. Unlike the word that a kind changed, this carries the
/// value, so a listing patches the one item it names and reads
/// nothing again.
#[derive(Debug, Clone)]
pub struct CapacityChange {
    /// The provider whose capacity it is.
    pub identity: Identity,
    /// The capacity cached now.
    pub capacity: Arc<Frame>,
}

/// Where every swap is told, and every provider listing hears it.
#[derive(Debug)]
pub struct CapacityChanges {
    /// The word, to every listing subscribed.
    sender: broadcast::Sender<CapacityChange>,
}

impl CapacityChanges {
    /// A provider's capacity was swapped. With no listing open,
    /// nothing.
    pub fn changed(&self, change: CapacityChange) {
        let _ = self.sender.send(change);
    }

    /// Every swap from now on, for a listing: subscribed before the
    /// items are read, so a swap between the reading and the hearing
    /// is in one or the other.
    pub fn subscribe(&self) -> broadcast::Receiver<CapacityChange> {
        self.sender.subscribe()
    }
}

impl Default for CapacityChanges {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(SWAPS_BEHIND);
        CapacityChanges { sender }
    }
}
