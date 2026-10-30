//! A logs request's filter over one item, in either book.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::endpoints::logs::client::request::ItemType;

/// Whether an item is one the request asks for, by its kind — `name`,
/// as its own `type` carries it — and the time span. The index span
/// is the reader's: it chooses which items to look at, and this says
/// which of those to send. A `type` the item's book does not have
/// matches nothing.
pub fn matches(
    r#type: Option<ItemType>,
    created_from: Option<DateTime<Utc>>,
    created_to: Option<DateTime<Utc>>,
    name: &str,
    created: DateTime<Utc>,
) -> bool {
    r#type.is_none_or(|wanted| wanted.name() == name)
        && created_from.is_none_or(|from| created >= from)
        && created_to.is_none_or(|to| created <= to)
}
