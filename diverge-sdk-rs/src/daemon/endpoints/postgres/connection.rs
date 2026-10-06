//! One container connection open through the database.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Container;

/// One connection a container holds open through the database now: the
/// container, and when the connection was opened. A container holding
/// several is listed once per connection. A list sends them oldest
/// opened first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Connection {
    /// The container holding it: see [`Container`].
    pub container: Container,
    /// When the container opened it, by the daemon's clock. On the wire
    /// an RFC 3339 timestamp in UTC.
    pub opened: DateTime<Utc>,
}
