//! The deployer agent a container names, checked.

use diverge_sdk::daemon::creator;
use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::Checked;
use crate::store::{self, agents};

/// The deployer agent named, as it is now — its template, its index
/// and its name, kept stable past its deletion: `Error` for one that
/// is not there, else the snapshot; none named is none.
pub async fn deployer(
    conn: &mut PgConnection,
    reference: Option<&reference::Agent>,
) -> Result<Checked<Option<creator::Agent>>, store::Error> {
    let Some(reference) = reference else {
        return Ok(Checked::Ok(None));
    };
    match agents::by_reference(conn, reference, false).await? {
        Some(agent) => Ok(Checked::Ok(Some(agent.snapshot()))),
        None => Ok(Checked::Error("the deployer agent named is none the daemon has".to_string())),
    }
}
