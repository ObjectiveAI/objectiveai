//! A transfer's destination, judged by its own grant.

use diverge_sdk::daemon::grant::{agents, dependencies, tools, volumes};
use diverge_sdk::daemon::transfer::Destination;
use sqlx::PgConnection;

use crate::daemon::Daemon;
use crate::judge::{self, Standing};
use crate::serve::volumes::{Located, locate};
use crate::serve;
use crate::store;

/// Whether the standing may land files at the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judged {
    /// It may.
    Allowed,
    /// The grants do not reach the destination.
    Forbidden,
    /// The destination names an agent, a tool, a dependency tool or a
    /// volume that is none — a dependency tool that does not run among
    /// them.
    NoDestination,
    /// The destination's provider could not be asked.
    Error(String),
}

/// A transfer lands by `upload` over the destination agent, tool,
/// dependency template or volume — the grant an upload there would
/// take. A dependency tool is judged by its template.
pub async fn judged(conn: &mut PgConnection, daemon: &Daemon, standing: &Standing, destination: &Destination) -> Result<Judged, store::Error> {
    Ok(match destination {
        Destination::Agent { agent, .. } => {
            let Some(record) = store::agents::by_reference(conn, agent, false).await? else {
                return Ok(Judged::NoDestination);
            };
            let active = serve::agents::active(daemon, record.id).await;
            if judge::agents::over(standing, agents::Over::Upload, &record, active) { Judged::Allowed } else { Judged::Forbidden }
        }
        Destination::Tool { tool, .. } => {
            let Some(tool) = serve::tools::resolve(conn, tool, false).await? else {
                return Ok(Judged::NoDestination);
            };
            if serve::tools::reaches(conn, daemon, standing, tools::Over::Upload, &tool).await? {
                Judged::Allowed
            } else {
                Judged::Forbidden
            }
        }
        Destination::Dependency { dependency, .. } => {
            let Some((record, _)) = serve::dependencies::resolve(conn, daemon, dependency).await? else {
                return Ok(Judged::NoDestination);
            };
            if serve::dependencies::reaches(conn, daemon, standing, dependencies::Over::Upload, &record).await? {
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
