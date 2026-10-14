//! `diverge run agents logs`: an agent's log: everything its
//! conversation has carried, read, as
//! [`logs`](diverge_sdk::daemon::endpoints::agents::logs) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::agents::logs::{client::request, server::response};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error, body, shape};

/// An agent's log: everything its conversation has carried, read.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
}

/// The request put, every item printed as it comes, to the log's
/// end or, watching, as far as the request asked.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::stream::<_, response::Frame, _>(handle, &request, emitter, shape::never).await
}
