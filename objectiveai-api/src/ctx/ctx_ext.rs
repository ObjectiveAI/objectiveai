//! Context extension trait for per-request customization.

/// Extension trait for providing per-request customization.
///
/// Implementations can provide BYOK (Bring Your Own Key) API keys
/// for upstream providers, allowing users to use their own API keys
/// instead of ObjectiveAI's pooled keys.
#[async_trait::async_trait]
pub trait ContextExt {
    /// Returns the user's OpenRouter authorization token.
    async fn openrouter_authorization(&self) -> Option<std::sync::Arc<String>>;

    /// Returns the user's GitHub authorization token.
    async fn github_authorization(&self) -> Option<std::sync::Arc<String>>;

    /// Returns the user's MCP authorization headers.
    async fn mcp_authorization(
        &self,
    ) -> Option<std::sync::Arc<std::collections::HashMap<String, String>>>;

    /// Returns the commit author name.
    async fn commit_author_name(&self) -> Option<std::sync::Arc<String>>;

    /// Returns the commit author email.
    async fn commit_author_email(&self) -> Option<std::sync::Arc<String>>;
}
