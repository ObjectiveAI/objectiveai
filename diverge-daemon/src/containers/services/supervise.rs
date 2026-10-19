//! The supervisor: a service's container kept running while it is
//! held.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use super::{Outcome, State, start};
use crate::containers::{ToolKey, ToolRun, stop};
use crate::daemon::Daemon;
use crate::store::{DependencyToolId, ToolId, dependency_tools, tools};

/// How long a supervisor waits after a start that failed before it
/// tries again: long enough not to hammer a provider that is down,
/// short enough that a service is back soon after one comes up.
const RETRY: Duration = Duration::from_secs(5);

/// Keep the service's container running until the service is
/// retired: while nothing holds it the supervisor waits; while
/// something does, its container is started — on the first candidate
/// provider that runs it — and waited on to its end, and started
/// again; a start that fails is tried again after `RETRY`. Each
/// start's outcome is said on `outcomes` — up once the run is live,
/// or why it failed — which is what a registration that declared the
/// service waits on. A row that is gone is a service deleted, and the
/// supervisor ends. The run's end is taken down here as well as by
/// its waiter, so that the next start does not find the old run
/// still live.
pub async fn supervise(daemon: Arc<Daemon>, key: ToolKey, state: Arc<State>, outcomes: watch::Sender<Option<Outcome>>) {
    let mut held = state.held.subscribe();
    loop {
        if !*held.borrow_and_update() {
            if held.changed().await.is_err() {
                return;
            }
            continue;
        }
        let started = match key {
            ToolKey::Record(id) => record(&daemon, id).await,
            ToolKey::Dependency(id) => dependency(&daemon, id).await,
        };
        match started {
            Started::Gone => return,
            Started::Run(run) => {
                outcomes.send_replace(Some(Outcome::Up));
                run.handle.wait().await;
                stop::ended_tool(&daemon, run).await;
            }
            Started::Failed(reason) => {
                outcomes.send_replace(Some(Outcome::Failed(reason)));
                tokio::time::sleep(RETRY).await;
            }
        }
    }
}

/// One start's outcome, as the supervisor acts on it.
enum Started {
    /// The run, up, to be waited on.
    Run(Arc<ToolRun>),
    /// The row is gone: the service is deleted.
    Gone,
    /// The store could not be read, or the container would not start,
    /// in its words: tried again after `RETRY`.
    Failed(String),
}

/// One start of a tool on record's container.
async fn record(daemon: &Arc<Daemon>, id: ToolId) -> Started {
    let tool = match daemon.store.acquire().await {
        Ok(mut conn) => tools::by_id(&mut conn, id, false).await,
        Err(error) => Err(error),
    };
    match tool {
        Ok(Some(tool)) => match crate::containers::start::tool(daemon, &tool).await {
            Ok(run) => Started::Run(run),
            Err(error) => Started::Failed(error.to_string()),
        },
        Ok(None) => Started::Gone,
        Err(error) => Started::Failed(error.to_string()),
    }
}

/// One start of a dependency service's container.
async fn dependency(daemon: &Arc<Daemon>, id: DependencyToolId) -> Started {
    let service = match daemon.store.acquire().await {
        Ok(mut conn) => dependency_tools::by_id(&mut conn, id, false).await,
        Err(error) => Err(error),
    };
    match service {
        Ok(Some(service)) => match start(daemon, &service).await {
            Ok(run) => Started::Run(run),
            Err(error) => Started::Failed(error.to_string()),
        },
        Ok(None) => Started::Gone,
        Err(error) => Started::Failed(error.to_string()),
    }
}
