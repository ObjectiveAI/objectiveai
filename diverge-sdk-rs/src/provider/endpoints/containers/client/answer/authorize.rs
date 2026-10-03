//! Whether a connector may attach, or a lister may see, from the
//! runner.

use std::sync::Arc;

use super::send::{Stop, finish, respond};
use crate::provider::client::ConnectionAuthorizer;
use crate::wire::client::handle::Handle;
use crate::shared::containers::authorize;

/// One frame, yes or no, then the finish.
pub(crate) async fn authorize_connect<A: ConnectionAuthorizer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    request: authorize::request::AuthorizeConnect,
    authorizer: Arc<A>,
) -> Result<(), Stop> {
    let frame = authorizer.authorize_connect(&request).await;
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// One frame, yes or no, then the finish: the same shape, for a
/// lister.
pub(crate) async fn authorize_list<A: ConnectionAuthorizer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    request: authorize::request::AuthorizeList,
    authorizer: Arc<A>,
) -> Result<(), Stop> {
    let frame = authorizer.authorize_list(&request).await;
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}
