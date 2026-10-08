//! A provider's volumes as its listing streams them, kept.

use std::collections::HashMap;

use diverge_sdk::provider::endpoints::volumes::list::server::response::{Frame, Volume};
use tokio::sync::{Mutex, watch};

use super::Fail;

/// How far a provider's listing has come.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Whole {
    /// The volumes running when the listing opened are still
    /// arriving.
    Listing,
    /// The provider said the listing is whole; every change since is
    /// in the mirror.
    Listed,
    /// The listing ended in the provider's error, or short: the
    /// mirror is not to be trusted until the provider connects and
    /// lists again.
    Failed(String),
}

/// One connected provider's volumes, as its `volumes::list` stream
/// tells them: added, changed and removed as they come, from the
/// connection's attach to its end. What every reading of a provider's
/// volumes on the daemon reads, instead of asking the provider.
#[derive(Debug)]
pub struct Mirror {
    /// The volumes, by name.
    volumes: Mutex<HashMap<String, Volume>>,
    /// Whether the listing is whole yet.
    whole: watch::Sender<Whole>,
}

impl Mirror {
    /// Empty, still listing.
    pub fn new() -> Self {
        let (whole, _) = watch::channel(Whole::Listing);
        Mirror {
            volumes: Mutex::new(HashMap::new()),
            whole,
        }
    }

    /// One frame of the provider's listing taken in; `true` when it
    /// changed what the mirror holds or says of itself.
    pub async fn apply(&self, frame: Frame) -> bool {
        match frame {
            Frame::Added(volume) | Frame::Changed(volume) => {
                let mut volumes = self.volumes.lock().await;
                volumes.insert(volume.name.clone(), volume.clone()) != Some(volume)
            }
            Frame::Removed(volume) => self.volumes.lock().await.remove(&volume.name).is_some(),
            Frame::Listed => self.whole.send_replace(Whole::Listed) != Whole::Listed,
            Frame::Error(error) => {
                self.failed(super::describe(&error));
                true
            }
        }
    }

    /// The listing failed: nothing here is to be trusted.
    pub fn failed(&self, why: String) {
        self.whole.send_replace(Whole::Failed(why));
    }

    /// Whether the listing is whole now.
    pub fn whole(&self) -> Whole {
        self.whole.borrow().clone()
    }

    /// Wait for the listing to be whole; the provider's failure is
    /// the error.
    pub async fn listed(&self) -> Result<(), Fail> {
        let mut whole = self.whole.subscribe();
        let state = whole
            .wait_for(|whole| *whole != Whole::Listing)
            .await
            .map(|whole| whole.clone())
            .unwrap_or_else(|_| Whole::Failed("the provider's listing ended".to_string()));
        match state {
            Whole::Listed => Ok(()),
            Whole::Failed(why) => Err(Fail::Error(why)),
            Whole::Listing => Err(Fail::Error("the provider's listing ended".to_string())),
        }
    }

    /// The volumes now, by name.
    pub async fn volumes(&self) -> Vec<Volume> {
        let mut volumes: Vec<Volume> = self.volumes.lock().await.values().cloned().collect();
        volumes.sort_by(|a, b| a.name.cmp(&b.name));
        volumes
    }
}

impl Default for Mirror {
    fn default() -> Self {
        Mirror::new()
    }
}
