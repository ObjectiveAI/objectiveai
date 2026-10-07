//! What an account may do, read fresh.

use diverge_sdk::daemon::grant::Grant;
use sqlx::types::Json;
use sqlx::{PgConnection, Row as _};

use super::{AccountId, Error};

/// The identity an account is served under and every grant of every
/// role it holds, in no order — or `None` when no account has the
/// id, which is an account deleted since its client was admitted.
///
/// Read for every request rather than once at admission, so that a
/// role edited, a role given or taken, or an account renamed applies
/// to the account's next request, as the wire says it does.
pub async fn of_account(conn: &mut PgConnection, id: AccountId) -> Result<Option<(String, Vec<Grant>)>, Error> {
    let Some(row) = sqlx::query("SELECT name, identity FROM diverge.accounts WHERE id = $1")
        .bind(id.0)
        .fetch_optional(&mut *conn)
        .await?
    else {
        return Ok(None);
    };
    let name: Option<String> = row.try_get("name")?;
    let identity: Option<String> = row.try_get("identity")?;
    let identity = name.or(identity).unwrap_or_default();
    let rows = sqlx::query(
        "SELECT r.grants FROM diverge.roles r JOIN diverge.account_roles ar ON ar.role = r.id WHERE ar.account = $1",
    )
    .bind(id.0)
    .fetch_all(&mut *conn)
    .await?;
    let mut grants = Vec::new();
    for row in rows {
        let Json(held): Json<Vec<Grant>> = row.try_get("grants")?;
        grants.extend(held);
    }
    Ok(Some((identity, grants)))
}
