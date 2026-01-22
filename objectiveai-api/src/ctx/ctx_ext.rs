use crate::chat;

#[async_trait::async_trait]
pub trait ContextExt {
    async fn get_byok(
        &self,
        upstream: chat::completions::upstream::Upstream,
    ) -> Result<Option<String>, objectiveai::error::ResponseError>;
}
