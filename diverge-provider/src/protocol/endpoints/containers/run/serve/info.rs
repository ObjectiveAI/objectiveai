//! The image's `/DIVERGE.md`, what the container's arguments may be:
//! a channel on the begin scope, the same on either family.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::run::Run;
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::shared::containers::info;

/// Serve one, to the end: the proxy's one answer on the caller's
/// channel — the document, parsed; that the image ships none; or the
/// proxy's error — then the finish, or the finish alone where the
/// proxy could not serve it. The document is never looked at here.
pub(crate) async fn info(run: Arc<Run>, channel: u32) {
    let answer = match run.begin.info().await {
        Ok(Some(document)) => encoded(&info::response::Frame::Info(document)),
        Ok(None) => encoded(&info::response::Frame::Missing),
        Err(UnaryError::Refused(error)) => encoded(&info::response::Frame::Error(error)),
        Err(_) => None,
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}
