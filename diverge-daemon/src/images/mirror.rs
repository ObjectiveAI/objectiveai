//! A provider's Store of the daemon's as its listing streams it, kept.

use std::collections::HashMap;

use diverge_sdk::provider::endpoints::images::list::server::response::{Frame, Image};
use tokio::sync::{Mutex, watch};

use crate::volumes::{Whole, describe};

/// One connected provider's Store of the daemon's, as its
/// `images::list` stream tells it: images added, changed and removed
/// as they come, from the connection's attach to its end. What the
/// daemon's `images list` reads, instead of asking the provider.
#[derive(Debug)]
pub struct Mirror {
    /// The images, by digest.
    images: Mutex<HashMap<String, Image>>,
    /// Whether the listing is whole yet.
    whole: watch::Sender<Whole>,
}

impl Mirror {
    /// Empty, still listing.
    pub fn new() -> Self {
        let (whole, _) = watch::channel(Whole::Listing);
        Mirror {
            images: Mutex::new(HashMap::new()),
            whole,
        }
    }

    /// One frame of the provider's listing taken in; `true` when it
    /// changed what the mirror holds or says of itself.
    pub async fn apply(&self, frame: Frame) -> bool {
        match frame {
            Frame::Added(image) | Frame::Changed(image) => {
                let mut images = self.images.lock().await;
                images.insert(image.digest.clone(), image.clone()) != Some(image)
            }
            Frame::Removed(image) => self.images.lock().await.remove(&image.digest).is_some(),
            Frame::Listed => self.whole.send_replace(Whole::Listed) != Whole::Listed,
            Frame::Unsupported => {
                self.failed("the provider has no Store".to_string());
                true
            }
            Frame::Error(error) => {
                self.failed(describe(&error));
                true
            }
            Frame::AlreadyOpen => {
                self.failed("this connection has an image listing open already".to_string());
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

    /// The images now, by digest.
    pub async fn images(&self) -> Vec<Image> {
        let mut images: Vec<Image> = self.images.lock().await.values().cloned().collect();
        images.sort_by(|a, b| a.digest.cmp(&b.digest));
        images
    }
}

impl Default for Mirror {
    fn default() -> Self {
        Mirror::new()
    }
}
