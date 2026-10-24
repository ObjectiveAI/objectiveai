//! The tool container's activity, from the begin's main stream onto
//! the run's.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::{Run, send};
use diverge_sdk::container_proxy::outside::endpoints::tools::begin::client::execute::{Events, EventsError};
use diverge_sdk::provider::endpoints::containers::tools::run::client::execute::Event;
use diverge_sdk::provider::endpoints::containers::tools::run::server::response;

/// Everything the proxy says on its begin stream, as the run scope's
/// own frame of the same meaning — the first exchange in flight as
/// [`Active`](response::Frame::Active), the last out as
/// [`Inactive`](response::Frame::Inactive) with the rotating secrets
/// it collected, the values passed on unread — in order, until the
/// begin's stream ends, which is the proxy gone. A proxy that ends
/// its stream with an error — a collect that failed — is relayed as
/// the run's [`Error`](response::Frame::Error), in the proxy's words.
/// The tools family's frame, named outright: only a tool container
/// has exchanges to count.
pub(crate) async fn activity(run: Arc<Run>, mut events: Events) {
    while let Some(event) = events.next().await {
        let frame = match event {
            Ok(Event::Active) => response::Frame::Active,
            Ok(Event::Inactive(values)) => response::Frame::Inactive(values),
            Err(EventsError::Refused(error)) => response::Frame::Error(error),
            Err(_) => break,
        };
        send(&run.scope, encoded(&frame)).await;
    }
    run.over.notify_one();
}
