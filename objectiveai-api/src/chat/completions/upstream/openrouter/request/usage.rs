//! Usage reporting options for OpenRouter requests.

use serde::{Deserialize, Serialize};

/// Configuration for usage reporting in the response.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Usage {
    /// Whether to include usage statistics in the response.
    pub include: bool,
}
