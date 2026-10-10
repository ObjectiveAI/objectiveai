//! Who may join a created tool from its provider.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::endpoints::tools::Admission;
use sqlx::postgres::PgRow;
use sqlx::{PgConnection, Row as _};

use crate::store::{Error, ToolId};

/// One admission as the store holds it: the tool it is on, what the
/// wire reports, and what it never does — the hash of the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The tool.
    pub tool: ToolId,
    /// The identity, and the address if one.
    pub admission: Admission,
    /// The SHA-256 of the key, hex.
    pub key_hash: String,
    /// When it was put down.
    pub created: DateTime<Utc>,
}

/// What putting one down came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    /// The admission is on the tool.
    Created,
    /// An admission for that identity is on the tool already; nothing
    /// changed.
    Exists,
}

/// The columns every admission query selects.
const SELECT: &str = "SELECT tool, identity, address, key_hash, created FROM diverge.admissions";

/// Every admission, in the order admitted: what a listing of tools
/// folds in, loaded once for the whole list.
pub async fn all(conn: &mut PgConnection) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("{SELECT} ORDER BY created, identity"))
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(record).collect()
}

/// The tool's admissions, in the order admitted.
pub async fn of_tool(conn: &mut PgConnection, tool: ToolId) -> Result<Vec<Record>, Error> {
    let rows = sqlx::query(&format!("{SELECT} WHERE tool = $1 ORDER BY created, identity"))
        .bind(tool.0)
        .fetch_all(&mut *conn)
        .await?;
    rows.iter().map(record).collect()
}

/// The admission whose key has the hash, if any: how a connector's
/// authorization is judged.
pub async fn by_key_hash(conn: &mut PgConnection, key_hash: &str) -> Result<Option<Record>, Error> {
    let row = sqlx::query(&format!("{SELECT} WHERE key_hash = $1"))
        .bind(key_hash)
        .fetch_optional(&mut *conn)
        .await?;
    row.as_ref().map(record).transpose()
}

/// Put the admission on the tool, with the hash of its key.
pub async fn create(conn: &mut PgConnection, tool: ToolId, admission: &Admission, key_hash: &str) -> Result<Created, Error> {
    let inserted = sqlx::query("INSERT INTO diverge.admissions (tool, identity, address, key_hash) VALUES ($1, $2, $3, $4)")
        .bind(tool.0)
        .bind(&admission.identity)
        .bind(admission.address.map(|address| address.to_string()))
        .bind(key_hash)
        .execute(&mut *conn)
        .await;
    match inserted {
        Ok(_) => Ok(Created::Created),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(Created::Exists),
        Err(error) => Err(error.into()),
    }
}

/// Take the admission for the identity off the tool, whether or not
/// one was there.
pub async fn delete(conn: &mut PgConnection, tool: ToolId, identity: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM diverge.admissions WHERE tool = $1 AND identity = $2")
        .bind(tool.0)
        .bind(identity)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The admission a row of [`SELECT`] holds.
fn record(row: &PgRow) -> Result<Record, Error> {
    let address: Option<String> = row.try_get("address")?;
    let address = match address {
        Some(value) => Some(value.parse().map_err(|source| Error::Address {
            value: value.clone(),
            source,
        })?),
        None => None,
    };
    Ok(Record {
        tool: ToolId(row.try_get("tool")?),
        admission: Admission {
            identity: row.try_get("identity")?,
            address,
        },
        key_hash: row.try_get("key_hash")?,
        created: row.try_get("created")?,
    })
}
