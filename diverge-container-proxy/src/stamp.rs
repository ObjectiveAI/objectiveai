//! The container's image, put under `_meta` on every MCP exchange
//! the proxy relays.

use diverge_provider_sdk::shared::containers::request::Image;
use rmcp::model::{
    GetMeta as _, MetaObject, RequestMetaObject, RequestParamsMeta, Resource, ServerNotification, Tool,
};
use serde_json::Value;

/// The one `_meta` key the proxy sets. Its form is MCP's rule for
/// `_meta` names — a prefix of dotted labels, a slash, a name — so it
/// collides with nobody's.
pub const KEY: &str = "diverge.network/image";

/// The image the container was made from, as the value under
/// [`KEY`]: `{"name":…,"digest":…}`, built once from what the begin
/// request carried and cloned onto each exchange.
///
/// A container never addresses one server over another, so this is
/// how the two ends of a tool call learn who is on the other side: a
/// program's request outward carries the calling image, and a tool
/// container's answers outward carry the serving image, on the
/// result, on each tool and resource of a list, and on each
/// notification. The key is replaced wherever the program set it —
/// the value is the proxy's to attest — and every other key travels
/// as sent.
#[derive(Debug, Clone)]
pub struct Stamp {
    image: Value,
}

impl Stamp {
    pub fn new(image: &Image) -> Self {
        Stamp {
            image: serde_json::json!({
                "name": image.name,
                "digest": image.digest,
            }),
        }
    }

    /// A request outward: the program's own `_meta` first — rmcp
    /// strips it off the wire into the request's context, so it would
    /// otherwise be lost on the way out — then the key.
    pub fn request<P: RequestParamsMeta>(&self, params: &mut P, sent: &RequestMetaObject) {
        let meta = params.meta_or_default();
        meta.extend(sent.clone());
        meta.insert(KEY.to_string(), self.image.clone());
    }

    /// A result's, a tool's or a resource's `_meta`.
    pub fn meta(&self, meta: &mut Option<MetaObject>) {
        meta.get_or_insert_with(MetaObject::new)
            .insert(KEY.to_string(), self.image.clone());
    }

    /// One tool of a list.
    pub fn tool(&self, tool: &mut Tool) {
        self.meta(&mut tool.meta);
    }

    /// One resource of a list.
    pub fn resource(&self, resource: &mut Resource) {
        self.meta(&mut resource.meta);
    }

    /// One notification, whichever kind: its `_meta` rides rmcp's
    /// extensions and is written into the params on the way out.
    pub fn notification(&self, notification: &mut ServerNotification) {
        notification.get_meta_mut().insert(KEY.to_string(), self.image.clone());
    }
}
