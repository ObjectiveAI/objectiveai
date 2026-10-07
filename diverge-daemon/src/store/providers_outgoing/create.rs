//! Adding an outgoing provider.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use crate::store::{Error, OutgoingId};

/// What a new outgoing provider is made of.
#[derive(Debug, Clone)]
pub struct New {
    /// The address, as given.
    pub address: String,
    /// How the daemon authenticates there.
    pub mode: Mode,
    /// Who added it.
    pub creator: Creator,
}

/// What an add came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The provider, added.
    Created(OutgoingId),
    /// The address is another provider's already; nothing was added.
    Exists,
}

/// Insert the provider, never yet dialled.
pub async fn create(conn: &mut PgConnection, new: &New) -> Result<Created, Error> {
    let inserted = sqlx::query("INSERT INTO diverge.providers_outgoing (address, mode, creator) VALUES ($1, $2, $3) RETURNING id")
        .bind(&new.address)
        .bind(Json(&new.mode))
        .bind(Json(&new.creator))
        .fetch_one(&mut *conn)
        .await;
    match inserted {
        Ok(row) => Ok(Created::Created(OutgoingId(row.try_get("id")?))),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}
