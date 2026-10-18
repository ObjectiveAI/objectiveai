//! The account a container runs under, checked.

use diverge_sdk::daemon::endpoints::accounts::Reference;
use diverge_sdk::daemon::grant::accounts::Over;
use sqlx::PgConnection;

use super::Checked;
use crate::daemon::Daemon;
use crate::judge::{self, Standing};
use crate::store::{self, AccountId, accounts};

/// The account named, by name: `NoAccount` for one the daemon does
/// not have, `Forbidden` for one the standing holds no `assign`
/// grant over, else its id; none named is none.
pub async fn account(
    conn: &mut PgConnection,
    standing: &Standing,
    daemon: &Daemon,
    name: Option<&str>,
) -> Result<Checked<Option<AccountId>>, store::Error> {
    let Some(name) = name else {
        return Ok(Checked::Ok(None));
    };
    let reference = Reference::Name { name: name.to_string() };
    let Some(account) = accounts::by_reference(conn, &reference, false).await? else {
        return Ok(Checked::NoAccount);
    };
    let connected = daemon.live.is_connected(account.id).await;
    if !judge::accounts::over(standing, Over::Assign, &account, connected) {
        return Ok(Checked::Forbidden);
    }
    Ok(Checked::Ok(Some(account.id)))
}
