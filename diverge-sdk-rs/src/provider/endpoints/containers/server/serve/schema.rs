//! The schema of the container's arguments: a channel on the begin
//! scope, the same on either family.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::run::Run;
use crate::provider::endpoints::containers::client::UnaryError;
use crate::shared::containers::schema;

/// Serve one, to the end: the proxy's one answer on the caller's
/// channel, then the finish — or the finish alone where the proxy
/// could not serve it.
pub(crate) async fn schema(run: Arc<Run>, channel: u32) {
    let answer = match run.begin.schema().await {
        Ok(schema) => encoded(&schema::response::Frame::Schema(schema)),
        Err(UnaryError::Refused(error)) => encoded(&schema::response::Frame::Error(error)),
        Err(_) => None,
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}
