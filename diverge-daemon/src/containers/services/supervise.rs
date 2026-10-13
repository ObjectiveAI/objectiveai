//! The supervisor: a service's container kept running while it is
//! held.

use std::sync::Arc;
use std::time::Duration;

use super::{State, start};
use crate::containers::stop;
use crate::daemon::Daemon;
use crate::store::{DependencyToolId, dependency_tools};

/// How long a supervisor waits after a start that failed before it
/// tries again: long enough not to hammer a provider that is down,
/// short enough that a service is back soon after one comes up.
const RETRY: Duration = Duration::from_secs(5);

/// Keep the service's container running until the service is
/// retired: while nothing holds it the supervisor waits; while
/// something does, its container is started — on the first candidate
/// provider that runs it — and waited on to its end, and started
/// again; a start that fails is tried again after `RETRY`. A row
/// that is gone is a service deleted, and the supervisor ends. The
/// run's end is taken down here as well as by its waiter, so that the
/// next start does not find the old run still live.
pub async fn supervise(daemon: Arc<Daemon>, id: DependencyToolId, state: Arc<State>) {
    let mut held = state.held.subscribe();
    loop {
        if !*held.borrow_and_update() {
            if held.changed().await.is_err() {
                return;
            }
            continue;
        }
        let service = match daemon.store.acquire().await {
            Ok(mut conn) => dependency_tools::by_id(&mut conn, id, false).await,
            Err(error) => Err(error),
        };
        let service = match service {
            Ok(Some(service)) => service,
            Ok(None) => return,
            Err(_) => {
                tokio::time::sleep(RETRY).await;
                continue;
            }
        };
        match start(&daemon, &service).await {
            Ok(run) => {
                run.handle.wait().await;
                stop::ended_tool(&daemon, run).await;
            }
            Err(_) => tokio::time::sleep(RETRY).await,
        }
    }
}
