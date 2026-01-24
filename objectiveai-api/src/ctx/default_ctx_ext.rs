use crate::chat;
use axum::http::HeaderMap;

/// Default context extension that extracts OpenRouter BYOK from request headers.
#[derive(Clone)]
pub struct DefaultContextExt {
    /// OpenRouter API key from the `authorization_openrouter` header.
    /// None if the header was not provided.
    pub openrouter_byok: Option<String>,
}

impl DefaultContextExt {
    /// Extracts the OpenRouter BYOK from request headers.
    ///
    /// Looks for the `authorization_openrouter` header and strips the "Bearer " prefix
    /// if present.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let openrouter_byok = headers
            .get("authorization_openrouter")
            .and_then(|v| v.to_str().ok())
            .map(|s| {
                if let Some(stripped) = s.strip_prefix("Bearer ") {
                    stripped.to_string()
                } else {
                    s.to_string()
                }
            });

        Self { openrouter_byok }
    }
}

#[async_trait::async_trait]
impl super::ContextExt for DefaultContextExt {
    async fn get_byok(
        &self,
        upstream: chat::completions::upstream::Upstream,
    ) -> Result<Option<String>, objectiveai::error::ResponseError> {
        match upstream {
            chat::completions::upstream::Upstream::OpenRouter => {
                Ok(self.openrouter_byok.clone())
            }
        }
    }
}
