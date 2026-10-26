//! Recording an occurrence.

use diverge_sdk::daemon::creator::Creator;
use sqlx::PgConnection;
use sqlx::types::Json;

use crate::store::Error;

/// Record that something passed from `sender` to `receiver`: the edge
/// added with a count of `1` where the graph had none, else its count
/// raised by one. The unique constraint over the two is what makes
/// the choice, so two occurrences racing each other add one edge and
/// count both; the row is the caller's connection's, committed as it
/// commits.
pub async fn record(conn: &mut PgConnection, sender: &Creator, receiver: &Creator) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO diverge.events (sender, receiver) VALUES ($1, $2) \
         ON CONFLICT (sender, receiver) DO UPDATE SET count = diverge.events.count + 1",
    )
    .bind(Json(sender))
    .bind(Json(receiver))
    .execute(&mut *conn)
    .await?;
    Ok(())
}
