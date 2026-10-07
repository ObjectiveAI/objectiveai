//! The database: not served yet.

use bytes::Bytes;
use diverge_sdk::provider::client::PostgresDialer;
use futures_util::stream;
use tokio::sync::mpsc::UnboundedReceiver;

use super::Answerer;

/// Every database connection a container opens is declined — the
/// empty finish — until the database step serves one scope per
/// container.
impl PostgresDialer for Answerer {
    type Connection = stream::Empty<Bytes>;

    async fn dial(&self, _: u32, _: UnboundedReceiver<Bytes>) -> Option<Self::Connection> {
        None
    }
}
