use crate::ctx;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Client {
    pub http_client: reqwest::Client,
    /// GitHub authorization token.
    pub authorization: Option<String>,
    pub user_agent: String,
    pub x_title: String,
    pub http_referer: String,
    pub backoff_current_interval: Duration,
    pub backoff_initial_interval: Duration,
    pub backoff_randomization_factor: f64,
    pub backoff_multiplier: f64,
    pub backoff_max_interval: Duration,
    pub backoff_max_elapsed_time: Duration,
}

impl Client {
    pub fn new(
        http_client: reqwest::Client,
        authorization: Option<String>,
        user_agent: String,
        x_title: String,
        http_referer: String,
        backoff_current_interval: Duration,
        backoff_initial_interval: Duration,
        backoff_randomization_factor: f64,
        backoff_multiplier: f64,
        backoff_max_interval: Duration,
        backoff_max_elapsed_time: Duration,
    ) -> Self {
        Self {
            http_client,
            authorization,
            user_agent,
            x_title,
            http_referer,
            backoff_current_interval,
            backoff_initial_interval,
            backoff_randomization_factor,
            backoff_multiplier,
            backoff_max_interval,
            backoff_max_elapsed_time,
        }
    }

    fn backoff(&self) -> backoff::ExponentialBackoff {
        backoff::ExponentialBackoff {
            current_interval: self.backoff_current_interval,
            initial_interval: self.backoff_initial_interval,
            randomization_factor: self.backoff_randomization_factor,
            multiplier: self.backoff_multiplier,
            max_interval: self.backoff_max_interval,
            max_elapsed_time: Some(self.backoff_max_elapsed_time),
            ..Default::default()
        }
    }

    /// Resolves the authorization token: per-request header → ext → self.authorization.
    async fn resolve_authorization<CTXEXT: ctx::ContextExt>(
        &self,
        ctx: &ctx::Context<CTXEXT>,
    ) -> Option<Arc<String>> {
        if let Some(token) = ctx.github_authorization().await {
            return Some(token);
        }
        self.authorization.as_ref().map(|t| Arc::new(t.clone()))
    }

    /// Adds authorization + standard headers to a request.
    fn request_headers(
        &self,
        mut req: reqwest::RequestBuilder,
        token: Option<&str>,
    ) -> reqwest::RequestBuilder {
        if let Some(token) = token {
            req = req.header(
                reqwest::header::AUTHORIZATION,
                ensure_bearer(token),
            );
        }
        req = req.header("user-agent", &self.user_agent);
        req = req.header("x-title", &self.x_title);
        req = req
            .header("referer", &self.http_referer)
            .header("http-referer", &self.http_referer);
        req
    }

    // ── Public fetch methods ───────────────────────────────────────

    /// Fetches the latest commit SHA for a repository.
    pub async fn fetch_latest_commit<CTXEXT: ctx::ContextExt>(
        &self,
        ctx: &ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
    ) -> Result<Option<String>, super::Error> {
        #[derive(serde::Deserialize)]
        struct Commit {
            sha: String,
        }
        let token = self.resolve_authorization(ctx).await;
        let token_str: Option<&str> = token.as_deref().map(|s| s.as_str());
        let http_request = self.request_headers(
            self.http_client
                .get(format!(
                    "https://api.github.com/repos/{}/{}/commits",
                    owner, repository,
                ))
                .header("accept", "application/vnd.github+json"),
            token_str,
        );
        backoff::future::retry(self.backoff(), || async {
            let response = http_request
                .try_clone()
                .unwrap()
                .send()
                .await
                .map_err(super::Error::RequestError)?;
            let code = response.status();
            if code.is_success() {
                let text = response
                    .text()
                    .await
                    .map_err(super::Error::ResponseError)?;
                let mut de = serde_json::Deserializer::from_str(&text);
                match serde_path_to_error::deserialize::<_, Vec<Commit>>(&mut de) {
                    Ok(commits) => Ok(commits.first().map(|c| c.sha.clone())),
                    Err(e) => Err(backoff::Error::transient(
                        super::Error::DeserializationError(e),
                    )),
                }
            } else if code == reqwest::StatusCode::NOT_FOUND {
                Ok(None)
            } else {
                Err(backoff::Error::transient(bad_status(response).await))
            }
        })
        .await
    }

    /// Fetches a file from a GitHub repository and returns the raw text content.
    /// Tries raw.githubusercontent.com first, falls back to the Contents API.
    pub async fn read_file<CTXEXT: ctx::ContextExt>(
        &self,
        ctx: &ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
        commit: &str,
        path: &str,
    ) -> Result<Option<String>, super::Error> {
        let token = self.resolve_authorization(ctx).await;
        let token_str: Option<&str> = token.as_deref().map(|s| s.as_str());
        backoff::future::retry(self.backoff(), || async {
            match self.fetch_file_raw(token_str, owner, repository, commit, path).await {
                Ok(opt) => Ok(opt),
                Err(e1) => match self
                    .fetch_file_api(token_str, owner, repository, commit, path)
                    .await
                {
                    Ok(opt) => Ok(opt),
                    Err(e2) => Err(backoff::Error::transient(
                        super::Error::MultipleErrors(Box::new(e1), Box::new(e2)),
                    )),
                },
            }
        })
        .await
    }

    /// Fetches a JSON file from a GitHub repository and deserializes it.
    pub async fn read_json<T, CTXEXT: ctx::ContextExt>(
        &self,
        ctx: &ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
        commit: &str,
        path: &str,
    ) -> Result<Option<T>, super::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match self.read_file(ctx, owner, repository, commit, path).await? {
            Some(text) => {
                let mut de = serde_json::Deserializer::from_str(&text);
                match serde_path_to_error::deserialize::<_, T>(&mut de) {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => Err(super::Error::DeserializationError(e)),
                }
            }
            None => Ok(None),
        }
    }

    // ── Private fetch helpers ──────────────────────────────────────

    async fn fetch_file_raw(
        &self,
        token: Option<&str>,
        owner: &str,
        repository: &str,
        commit: &str,
        path: &str,
    ) -> Result<Option<String>, super::Error> {
        let http_request = self.request_headers(
            self.http_client.get(format!(
                "https://raw.githubusercontent.com/{}/{}/{}/{}",
                owner, repository, commit, path,
            )),
            token,
        );
        let response = http_request
            .send()
            .await
            .map_err(super::Error::RequestError)?;
        let code = response.status();
        if code.is_success() {
            let text = response.text().await.map_err(super::Error::ResponseError)?;
            Ok(Some(text))
        } else if code == reqwest::StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            Err(bad_status(response).await)
        }
    }

    async fn fetch_file_api(
        &self,
        token: Option<&str>,
        owner: &str,
        repository: &str,
        commit: &str,
        path: &str,
    ) -> Result<Option<String>, super::Error> {
        let http_request = self.request_headers(
            self.http_client
                .get(format!(
                    "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
                    owner, repository, path, commit,
                ))
                .header("accept", "application/vnd.github.raw+json"),
            token,
        );
        let response = http_request
            .send()
            .await
            .map_err(super::Error::RequestError)?;
        let code = response.status();
        if code.is_success() {
            let text = response.text().await.map_err(super::Error::ResponseError)?;
            Ok(Some(text))
        } else if code == reqwest::StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            Err(bad_status(response).await)
        }
    }

}

/// Extracts a bad status error from a response.
async fn bad_status(response: reqwest::Response) -> super::Error {
    let code = response.status();
    match response.text().await {
        Ok(text) => super::Error::BadStatus {
            code,
            body: serde_json::from_str::<serde_json::Value>(&text)
                .unwrap_or(serde_json::Value::String(text)),
        },
        Err(_) => super::Error::BadStatus {
            code,
            body: serde_json::Value::Null,
        },
    }
}

/// Ensures a token has the "Bearer " prefix.
fn ensure_bearer(token: &str) -> String {
    if token.starts_with("Bearer ") {
        token.to_string()
    } else {
        format!("Bearer {}", token)
    }
}