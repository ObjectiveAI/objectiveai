//! A listed item patched in place by a capacity swap, with no reading
//! of the records.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::capacity::server::response::Frame as Capacity;

/// How a listing takes a provider's capacity swap: which of its keys
/// the provider is listed under, if any, and the item patched with
/// the capacity and the frame that tells the client so. A source
/// whose items carry no capacity — every list but the two provider
/// lists — is [`Unpatched`], and hears no swap.
pub trait Patch<K, I, Fr> {
    /// The key the provider is listed under, or `None` for a provider
    /// this listing never lists.
    fn key(&self, identity: &Identity) -> Option<K>;

    /// The item patched with the capacity, and the frame that tells
    /// the client the swap; `None` sends nothing.
    fn apply(&self, item: &mut I, capacity: &Capacity) -> Option<Fr>;
}

/// A listing that hears no capacity swap.
pub struct Unpatched;

impl<K, I, Fr> Patch<K, I, Fr> for Unpatched {
    fn key(&self, _: &Identity) -> Option<K> {
        None
    }

    fn apply(&self, _: &mut I, _: &Capacity) -> Option<Fr> {
        None
    }
}
