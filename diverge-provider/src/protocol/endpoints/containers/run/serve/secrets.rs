//! The secrets' values, fed to the container: a channel on the begin
//! scope, the same on either family.

use std::sync::Arc;

use super::super::encoded::encoded;
use super::super::run::Run;
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::containers::secrets::feed;

/// Serve one, to the end: the values go to the proxy on the begin's
/// secrets channel, held until the program has applied them, and the
/// proxy's one answer — `Fed`, or the program's refusal — goes on the
/// caller's channel, then the finish; or the finish alone where the
/// proxy could not serve it. The values are never looked at here.
pub(crate) async fn secrets(run: Arc<Run>, channel: u32, values: Values) {
    let answer = match run.begin.secrets(values).await {
        Ok(()) => encoded(&feed::response::Frame::Fed),
        Err(UnaryError::Refused(error)) => encoded(&feed::response::Frame::Error(error)),
        Err(_) => None,
    };
    run.respond(channel, answer).await;
    run.finish(channel).await;
}
