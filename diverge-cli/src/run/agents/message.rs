//! `diverge run agents message`: sending an agent a message, as
//! [`message`](diverge_sdk::daemon::endpoints::agents::message) states
//! it.

use clap::Args;
use diverge_sdk::daemon::endpoints::agents::message::{client::request, server::response};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error, body, shape};

/// Sending an agent a message.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
}

/// The request put, its answer waited for and printed.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::pending::<_, response::Frame, _>(handle, &request, emitter).await
}
