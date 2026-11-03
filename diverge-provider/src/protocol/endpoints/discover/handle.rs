//! Answering a discover request, from a scope and the discoverable
//! identities.

use diverge_sdk::provider::endpoints::discover::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::protocol::discoverable::Discoverable;

/// Answer with every identity `discoverable` holds — every connected
/// identity whose discover flag was `1`, in order — and end the scope.
///
/// Every connected peer may ask, and the asking identity is listed when
/// it is itself discoverable. Nothing is checked, because the dispatch
/// read the request once already, and it carries nothing beyond its
/// tag.
pub async fn handle(scope: ScopeHandle, discoverable: &Discoverable) {
    let frame = response::Frame { identities: discoverable.list().await };
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
