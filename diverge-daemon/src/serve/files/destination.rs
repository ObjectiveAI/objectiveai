//! A transfer's destination, judged by its own grant.

use diverge_sdk::daemon::grant::{agents, tools, volumes};
use diverge_sdk::daemon::transfer::Destination;
use sqlx::PgConnection;

use crate::daemon::Daemon;
use crate::judge::{self, Standing};
use crate::serve::volumes::{Located, locate};
use crate::serve::{agents as serve_agents, tools as serve_tools};
use crate::store::{self, agents as store_agents, tools as store_tools};

/// Whether the standing may land files at the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judged {
    /// It may.
    Allowed,
    /// The grants do not reach the destination.
    Forbidden,
    /// The destination names an agent, a tool or a volume that is
    /// none.
    NoDestination,
    /// The destination's provider could not be asked.
    Error(String),
}

/// A transfer lands by `upload` over the destination agent, tool or
/// volume — the grant an upload there would take.
pub async fn judged(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, destination: &Destination) -> Result<Judged, store::Error> {
    Ok(match destination {
        Destination::Agent { agent, .. } => {
            let Some(record) = store_agents::by_reference(conn, agent, false).await? else {
                return Ok(Judged::NoDestination);
            };
            let active = serve_agents::active(daemon, record.id).await;
            if judge::agents::over(standing, agents::Over::Upload, &record, active) { Judged::Allowed } else { Judged::Forbidden }
        }
        Destination::Tool { tool, .. } => {
            let Some(record) = store_tools::by_reference(conn, tool, false).await? else {
                return Ok(Judged::NoDestination);
            };
            let attached = serve_tools::agents_of(conn, record.id).await?;
            let active = serve_tools::active(daemon, record.id).await;
            if judge::tools::over(standing, tools::Over::Upload, &record, active, &attached) {
                Judged::Allowed
            } else {
                Judged::Forbidden
            }
        }
        Destination::Volume { volume, .. } => match locate(conn, daemon, volume).await? {
            Located::Volume(listed) => {
                if judge::volumes::over(standing, volumes::Over::Upload, &listed) {
                    Judged::Allowed
                } else {
                    Judged::Forbidden
                }
            }
            Located::None => Judged::NoDestination,
            Located::Failed(error) => Judged::Error(error),
        },
    })
}
