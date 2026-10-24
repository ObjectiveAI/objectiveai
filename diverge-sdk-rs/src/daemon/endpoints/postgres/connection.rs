//! One container connection open through the database.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;
use super::Container;

/// One connection a container holds open through the database now: the
/// daemon's number for it, the container, and when the connection was
/// opened. A container holding several is listed once per connection,
/// each by its id. A list sends them oldest opened first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Connection {
    /// The daemon's number for this connection, given when it is
    /// opened and never again: what tells two connections of one
    /// container apart, and what a listing keys by. Nothing else is
    /// read into it.
    pub id: u64,
    /// The container holding it: see [`Container`].
    pub container: Container,
    /// When the container opened it, by the daemon's clock. On the wire
    /// an RFC 3339 timestamp in UTC.
    pub opened: DateTime<Utc>,
}

/// Keyed by the id: a container holding several connections is
/// listed once per connection, and the id tells them apart.
impl Keyed for Connection {
    type Key = u64;

    fn key(&self) -> u64 {
        self.id
    }
}
