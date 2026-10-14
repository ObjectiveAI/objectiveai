//! `diverge run standalone-tools detach`: detaching a tool from an
//! agent, by their names, as
//! [`detach`](diverge_sdk::daemon::endpoints::standalone_tools::detach)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::detach::{client::request, server::response};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error, body, shape};

/// Detaching a tool from an agent, by their names.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
}

/// The request put, its one answer printed.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::one_shot::<_, response::Frame, _>(handle, &request, emitter).await
}
