//! Recording an occurrence.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::events::Action;
use sqlx::PgConnection;
use sqlx::types::Json;

use super::action_name;
use crate::store::Error;

/// Record that `action` passed from `sender` to `receiver`: the edge
/// added with a count of `1` where the graph had none, else its count
/// raised by one. The unique constraint over the three is what makes
/// the choice, so two occurrences racing each other add one edge and
/// count both; the row is the caller's connection's, committed as it
/// commits.
pub async fn record(conn: &mut PgConnection, sender: &Creator, receiver: &Creator, action: Action) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO diverge.events (action, sender, receiver) VALUES ($1, $2, $3) \
         ON CONFLICT (sender, receiver, action) DO UPDATE SET count = diverge.events.count + 1",
    )
    .bind(action_name(action))
    .bind(Json(sender))
    .bind(Json(receiver))
    .execute(&mut *conn)
    .await?;
    Ok(())
}
