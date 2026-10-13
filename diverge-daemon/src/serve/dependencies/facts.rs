//! What a dependency template is attached to and what runs from it,
//! loaded.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Provider;
use diverge_sdk::daemon::endpoints::dependencies::list::server::response::Run;
use diverge_sdk::daemon::grant::dependencies::Over;
use diverge_sdk::daemon::key;
use sqlx::PgConnection;

use crate::containers::{ToolKey, ToolRun};
use crate::daemon::Daemon;
use crate::judge::filter::dependencies::Facts;
use crate::judge::{self, Standing};
use crate::store::dependencies::Record;
use crate::store::{self, agents, dependencies};

/// The run as the list reports it, with the id of the template it
/// runs from: `None` for a record's run or a connected tool's, which
/// run from no dependency template.
pub fn run_of(run: &ToolRun) -> Option<(String, Run)> {
    let key::Tool::Dependency { template, owner } = &run.key else {
        return None;
    };
    Some((
        template.clone(),
        Run {
            owner: owner.clone(),
            provider: Provider {
                identity: run.provider.clone(),
            },
            id: run.container.clone().unwrap_or_default(),
        },
    ))
}

/// The order runs are listed in under one template: by their live
/// key, which is stable for a run's life, so that a listing read
/// twice reads the same order.
pub fn run_order(run: &ToolRun) -> (u8, u64) {
    match run.id {
        ToolKey::Record(id) => (0, u64::try_from(id.0).unwrap_or_default()),
        ToolKey::Dependency(id) => (1, id.0),
        ToolKey::Service(id) => (2, u64::try_from(id.0).unwrap_or_default()),
    }
}

/// The agents that depend on the template now, each as its key — its
/// name as it is called now — in order.
pub async fn agents_of(conn: &mut PgConnection, template: &str) -> Result<Vec<key::Agent>, store::Error> {
    let mut keys = Vec::new();
    for id in dependencies::agents::of_template(conn, template).await? {
        if let Some(agent) = agents::by_id(conn, id, false).await? {
            keys.push(agent.key());
        }
    }
    Ok(keys)
}

/// The tools running from the template now, in listing order.
pub async fn runs_of(daemon: &Daemon, template: &str) -> Vec<Run> {
    let mut runs = daemon.live.tool_runs().await;
    runs.sort_by_key(|run| run_order(run));
    runs.iter()
        .filter_map(|run| run_of(run))
        .filter(|(of, _)| of == template)
        .map(|(_, run)| run)
        .collect()
}

/// Whether the standing may do `action` to the template: its agents
/// and its runs loaded here.
pub async fn reaches(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, action: Over, record: &Record) -> Result<bool, store::Error> {
    let agents = agents_of(conn, &record.id).await?;
    let runs = runs_of(daemon, &record.id).await;
    Ok(judge::dependencies::over(
        standing,
        action,
        &Facts {
            record,
            agents: &agents,
            runs: &runs,
        },
    ))
}
