//! One message a loop is asked.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

/// One message, as its enqueue carried it: the key and the content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The caller's key, as the enqueue gave it; what each of the
    /// message's user parts carries.
    pub key: String,
    /// The message's content, in order.
    pub content: Vec<ContentBlock>,
}
