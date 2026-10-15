//! The program's own `_meta`, carried out on its request.

use rmcp::model::{RequestMetaObject, RequestParamsMeta};

/// Put the `_meta` the program sent back into the params going out.
///
/// rmcp takes `_meta` off the wire into the request's context, so a
/// relay that forwarded the params as decoded would forward them
/// without it. What was sent is extended into what goes out, key by
/// key, and nothing is set that the program did not send: the proxy
/// attests nothing under `_meta`.
pub fn carry<P: RequestParamsMeta>(params: &mut P, sent: &RequestMetaObject) {
    params.meta_or_default().extend(sent.clone());
}
