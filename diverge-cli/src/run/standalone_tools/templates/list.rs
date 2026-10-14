//! `diverge run standalone-tools templates list`: listing a caller's
//! templates, as
//! [`list`](diverge_sdk::daemon::endpoints::standalone_tools::templates::list)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::templates::list::{
    client::request, server::response,
};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error, body, shape};

/// Listing a caller's templates.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
}

/// The request put, every item printed as it comes, through the
/// word that the list is whole; the changes after it are not this
/// command's.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::stream::<_, response::Frame, _>(handle, &request, emitter, shape::at_listed).await
}
