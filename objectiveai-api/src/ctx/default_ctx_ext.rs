/// Default context extension
#[derive(Clone)]
pub struct DefaultContextExt;

#[async_trait::async_trait]
impl super::ContextExt for DefaultContextExt {
    async fn openrouter_authorization(&self) -> Option<std::sync::Arc<String>> {
        None
    }

    async fn github_authorization(&self) -> Option<std::sync::Arc<String>> {
        None
    }

    async fn mcp_authorization(
        &self,
    ) -> Option<std::sync::Arc<std::collections::HashMap<String, String>>> {
        None
    }

    async fn commit_author_name(&self) -> Option<std::sync::Arc<String>> {
        None
    }

    async fn commit_author_email(&self) -> Option<std::sync::Arc<String>> {
        None
    }
}
