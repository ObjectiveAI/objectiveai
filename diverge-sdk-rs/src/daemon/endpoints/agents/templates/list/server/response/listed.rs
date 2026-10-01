//! One template, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::templates::Template;

/// One template of the caller's: its id, when it was made, and the
/// template whole.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listed {
    /// The id: the template's hash, as [`templates`](crate::daemon::endpoints::agents::templates)
    /// states it.
    pub id: String,
    /// When the create made it.
    pub created: DateTime<Utc>,
    /// The template, as it was handed in: the bytes the id is the
    /// hash of.
    pub template: Template,
}
