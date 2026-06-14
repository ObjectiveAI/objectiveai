//! GitHub retrieval for agents, swarms, functions, and profiles.
//!
//! Extends [`HttpClient`] with methods that fetch `agent.json` / `swarm.json`
//! / `function.json` / `profile.json` for a `(owner, repository, commit)`
//! straight from GitHub, plus latest-commit resolution.
//!
//! The fetch strategy mirrors the server: try `raw.githubusercontent.com`
//! first, fall back to the GitHub Contents API, all wrapped in exponential
//! backoff. Authorization uses the client's [`HttpClient::x_github_authorization`]
//! as the GitHub `Bearer` token (the GitHub token), never the ObjectiveAI API
//! key in `authorization`.

use super::{HttpClient, HttpError};

impl HttpClient {
    /// Resolves the latest commit SHA for a repository.
    ///
    /// Returns `Ok(None)` if the repository (or its commit list) cannot be
    /// found.
    pub async fn github_resolve_commit(
        &self,
        owner: &str,
        repository: &str,
    ) -> Result<Option<String>, HttpError> {
        #[derive(serde::Deserialize)]
        struct Commit {
            sha: String,
        }
        let request = self.github_request_headers(
            self.http_client
                .get(format!(
                    "https://api.github.com/repos/{}/{}/commits",
                    owner, repository,
                ))
                .header("accept", "application/vnd.github+json"),
        );
        backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
            let response = request
                .try_clone()
                .unwrap()
                .send()
                .await
                .map_err(HttpError::HttpError)?;
            let code = response.status();
            if code.is_success() {
                let text =
                    response.text().await.map_err(HttpError::HttpError)?;
                let mut de = serde_json::Deserializer::from_str(&text);
                match serde_path_to_error::deserialize::<_, Vec<Commit>>(
                    &mut de,
                ) {
                    Ok(commits) => Ok(commits.first().map(|c| c.sha.clone())),
                    Err(e) => Err(backoff::Error::transient(
                        HttpError::DeserializationError(e),
                    )),
                }
            } else if code == reqwest::StatusCode::NOT_FOUND {
                Ok(None)
            } else {
                Err(backoff::Error::transient(
                    github_bad_status(response).await,
                ))
            }
        })
        .await
    }

    /// Fetches an Agent definition (`agent.json`) from GitHub.
    ///
    /// If `commit` is `None`, the latest commit is resolved first. Returns
    /// `Ok(None)` if the repository, commit, or file does not exist.
    pub async fn github_get_agent(
        &self,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<Option<crate::agent::RemoteAgentBaseWithFallbacks>, HttpError>
    {
        self.github_get_entity(owner, repository, commit, "agent.json")
            .await
    }

    /// Fetches a Swarm definition (`swarm.json`) from GitHub.
    ///
    /// If `commit` is `None`, the latest commit is resolved first. Returns
    /// `Ok(None)` if the repository, commit, or file does not exist.
    pub async fn github_get_swarm(
        &self,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<Option<crate::swarm::RemoteSwarmBase>, HttpError> {
        self.github_get_entity(owner, repository, commit, "swarm.json")
            .await
    }

    /// Fetches a Function definition (`function.json`) from GitHub.
    ///
    /// If `commit` is `None`, the latest commit is resolved first. Returns
    /// `Ok(None)` if the repository, commit, or file does not exist.
    pub async fn github_get_function(
        &self,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<Option<crate::functions::FullRemoteFunction>, HttpError> {
        self.github_get_entity(owner, repository, commit, "function.json")
            .await
    }

    /// Fetches a Profile definition (`profile.json`) from GitHub.
    ///
    /// If `commit` is `None`, the latest commit is resolved first. Returns
    /// `Ok(None)` if the repository, commit, or file does not exist.
    pub async fn github_get_profile(
        &self,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<Option<crate::functions::RemoteProfile>, HttpError> {
        self.github_get_entity(owner, repository, commit, "profile.json")
            .await
    }

    /// Resolves the commit (if not provided) and reads + deserializes the
    /// entity's JSON file.
    async fn github_get_entity<T: serde::de::DeserializeOwned>(
        &self,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
        path: &str,
    ) -> Result<Option<T>, HttpError> {
        let resolved;
        let commit = match commit {
            Some(commit) => commit,
            None => match self.github_resolve_commit(owner, repository).await? {
                Some(commit) => {
                    resolved = commit;
                    resolved.as_str()
                }
                None => return Ok(None),
            },
        };
        self.github_read_json(owner, repository, commit, path).await
    }

    /// Reads a JSON file from a GitHub repository and deserializes it.
    async fn github_read_json<T: serde::de::DeserializeOwned>(
        &self,
        owner: &str,
        repository: &str,
        commit: &str,
        path: &str,
    ) -> Result<Option<T>, HttpError> {
        match self.github_read_file(owner, repository, commit, path).await? {
            Some(text) => {
                let mut de = serde_json::Deserializer::from_str(&text);
                match serde_path_to_error::deserialize::<_, T>(&mut de) {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => Err(HttpError::DeserializationError(e)),
                }
            }
            None => Ok(None),
        }
    }

    /// Reads a file's raw text content from a GitHub repository.
    ///
    /// Tries `raw.githubusercontent.com` first, falls back to the Contents
    /// API, wrapped in exponential backoff. `Ok(None)` on `404`.
    async fn github_read_file(
        &self,
        owner: &str,
        repository: &str,
        commit: &str,
        path: &str,
    ) -> Result<Option<String>, HttpError> {
        backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
            match self
                .github_fetch_file_raw(owner, repository, commit, path)
                .await
            {
                Ok(opt) => Ok(opt),
                Err(e1) => match self
                    .github_fetch_file_api(owner, repository, commit, path)
                    .await
                {
                    Ok(opt) => Ok(opt),
                    Err(e2) => Err(backoff::Error::transient(
                        HttpError::MultipleErrors(Box::new(e1), Box::new(e2)),
                    )),
                },
            }
        })
        .await
    }

    /// Fetches a file via `raw.githubusercontent.com`.
    async fn github_fetch_file_raw(
        &self,
        owner: &str,
        repository: &str,
        commit: &str,
        path: &str,
    ) -> Result<Option<String>, HttpError> {
        let response = self
            .github_request_headers(self.http_client.get(format!(
                "https://raw.githubusercontent.com/{}/{}/{}/{}",
                owner, repository, commit, path,
            )))
            .send()
            .await
            .map_err(HttpError::HttpError)?;
        let code = response.status();
        if code.is_success() {
            let text = response.text().await.map_err(HttpError::HttpError)?;
            Ok(Some(text))
        } else if code == reqwest::StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            Err(github_bad_status(response).await)
        }
    }

    /// Fetches a file via the GitHub Contents API (`?ref={commit}`).
    async fn github_fetch_file_api(
        &self,
        owner: &str,
        repository: &str,
        commit: &str,
        path: &str,
    ) -> Result<Option<String>, HttpError> {
        let response = self
            .github_request_headers(
                self.http_client
                    .get(format!(
                        "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
                        owner, repository, path, commit,
                    ))
                    .header("accept", "application/vnd.github.raw+json"),
            )
            .send()
            .await
            .map_err(HttpError::HttpError)?;
        let code = response.status();
        if code.is_success() {
            let text = response.text().await.map_err(HttpError::HttpError)?;
            Ok(Some(text))
        } else if code == reqwest::StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            Err(github_bad_status(response).await)
        }
    }

    /// Adds the GitHub `Bearer` token (from `x_github_authorization`) plus the
    /// standard `user-agent` / `x-title` / `referer` headers to a request.
    fn github_request_headers(
        &self,
        mut request: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        if let Some(token) = &self.x_github_authorization {
            let key = token.strip_prefix("Bearer ").unwrap_or(token.as_str());
            request =
                request.header("authorization", format!("Bearer {}", key));
        }
        // GitHub requires a User-Agent; fall back to a default when unset.
        request = request.header(
            "user-agent",
            self.user_agent.as_deref().unwrap_or("objectiveai-sdk"),
        );
        if let Some(x_title) = &self.x_title {
            request = request.header("x-title", x_title);
        }
        if let Some(http_referer) = &self.http_referer {
            request = request
                .header("referer", http_referer)
                .header("http-referer", http_referer);
        }
        request
    }
}

/// Builds a `BadStatus` error from a non-success response.
async fn github_bad_status(response: reqwest::Response) -> HttpError {
    let code = response.status();
    match response.text().await {
        Ok(text) => HttpError::BadStatus {
            code,
            body: serde_json::from_str::<serde_json::Value>(&text)
                .unwrap_or(serde_json::Value::String(text)),
        },
        Err(_) => HttpError::BadStatus {
            code,
            body: serde_json::Value::Null,
        },
    }
}
