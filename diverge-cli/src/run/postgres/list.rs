//! `diverge run postgres list`: listing the container connections open
//! through the database, and keeping the list, as
//! [`list`](diverge_sdk::daemon::endpoints::postgres::list) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::postgres::list::{client::request, server::response};

use crate::run::{Emitter, Error, Target, shape};

/// Listing the container connections open through the database.
/// There is exactly one database and the list is whole, so the
/// request names nothing, narrows nothing, and takes no body: it is
/// `{}` on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {}

/// The request put, every item printed as it comes, through the
/// word that the list is whole; the changes after it are not this
/// command's.
pub async fn run(
    Arguments {}: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request = request::Frame {};
    let handle = target.dial().await?;
    shape::stream::<_, response::Frame, _>(&handle, &request, emitter, shape::at_listed).await
}
