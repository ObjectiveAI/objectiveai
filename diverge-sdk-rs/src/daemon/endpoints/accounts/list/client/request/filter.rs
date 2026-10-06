//! What narrows a list of accounts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

/// The filter over accounts: every member optional, and every one given
/// a condition an account passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends, and its program transforms what passes. A
/// filter with no member given passes every account.
///
/// # Any one of, every one of
///
/// A member that lists candidates matches an account that is any one of
/// them, holds any one of them, or was made by any one of them;
/// `all_tags` an account that carries every one of them, `any_tags` one
/// that carries any one of them. An empty list is absent, and matches
/// every account.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Any one of these names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// With a credential naming any one of these identities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<String>,
    /// Whether the account has a name: `true`, or none, `false`;
    /// absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<bool>,
    /// Whether the account has a credential: `true`, or none, `false`;
    /// absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentialed: Option<bool>,
    /// Holding any one of these roles, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Whether a client is connected as it now: `true`, or none,
    /// `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    /// Made by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// Every one of these among the account's tags, as
    /// [`tag`](crate::daemon::endpoints::accounts::tag) put them.
    /// Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the account's tags, as
    /// [`tag`](crate::daemon::endpoints::accounts::tag) put them; with
    /// `all_tags`, both hold. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
    /// A jq program, as the `jq` command takes one, run with each
    /// matching account — one
    /// [`Account`](crate::daemon::endpoints::accounts::list::server::response::Account)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. Absent, the accounts come back as they are. The daemon
    /// does not read the program beyond running it; one that will not
    /// compile, or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}
