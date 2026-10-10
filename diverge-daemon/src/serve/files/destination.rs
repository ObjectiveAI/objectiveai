//! A transfer's destination, judged by its own grant.

use diverge_sdk::daemon::grant::{agents, tools, volumes};
use diverge_sdk::daemon::transfer::Destination;
use sqlx::PgConnection;

use crate::daemon::Daemon;
use crate::judge::{self, Standing};
use crate::serve::volumes::{Located, locate};
use crate::serve::tools::{Found, READ_ONLY};
use crate::serve::{agents as serve_agents, tools as serve_tools};
use crate::store::{self, agents as store_agents};

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
/// volume — the grant an upload there would take. A dependency tool
/// reached is the error an upload into it would be: it is read, not
/// changed.
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
            let Some(found) = serve_tools::resolve(conn, daemon, tool, false).await? else {
                return Ok(Judged::NoDestination);
            };
            if !serve_tools::reaches(conn, daemon, standing, tools::Over::Upload, &found).await? {
                Judged::Forbidden
            } else if let Found::Dependency(_) = found {
                Judged::Error(READ_ONLY.to_string())
            } else {
                Judged::Allowed
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
