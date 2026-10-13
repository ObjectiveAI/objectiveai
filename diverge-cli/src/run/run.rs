//! The command, done.

use std::time::Duration;

use super::{Emitter, Error, Request, dial, endpoints};
use crate::arguments;
use crate::config::Selected;

/// Put the request `run` names to the daemon `selected` names, as
/// its account: the request read from the name and the body, the
/// daemon dialled, the exchange run — within `--timeout`, when one is given
/// — and every answer printed.
pub async fn run(selected: &Selected<'_>, run: arguments::Run) -> Result<(), Error> {
    let request = Request::new(&run.path, run.body.as_deref())?;
    let mut emitter = Emitter::new(run.jq);
    let exchange = async {
        let handle = dial(&selected.daemon.address, &selected.account.key).await?;
        endpoints::run(&request, &handle, run.local.as_deref(), &mut emitter).await
    };
    match run.timeout {
        None => exchange.await,
        Some(seconds) => match tokio::time::timeout(Duration::from_secs(seconds), exchange).await {
            Ok(outcome) => outcome,
            Err(_) => Err(Error::Timeout(seconds)),
        },
    }
}
