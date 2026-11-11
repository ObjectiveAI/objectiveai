//! The client.

use std::collections::HashMap;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::{Stream, StreamExt as _, future};
use reqwest::{RequestBuilder, Response, StatusCode, header};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::config::containers::{Credential, Registry};
use crate::host::image_manager::manifest::ACCEPT;

/// A blob's body, piece by piece; it ends short where the transfer
/// failed, which the reader's digest check sees.
pub type Body = Pin<Box<dyn Stream<Item = Bytes> + Send>>;

/// How a repository was last authorized.
#[derive(Debug, Clone)]
enum Auth {
    /// The Basic credential.
    Basic,
    /// A token from the registry's token service.
    Bearer(String),
}

/// The client: the registries the configuration lists, with their
/// credentials, and the authorization each repository was last given.
/// See [`registry_client`](super) for what it asks and how it
/// authenticates.
#[derive(Debug)]
pub struct RegistryClient {
    http: reqwest::Client,
    registries: Vec<Registry>,
    /// By `<host>/<name>`.
    auths: Mutex<HashMap<String, Auth>>,
}

impl RegistryClient {
    /// A client of `registries`; one that cannot be made — no TLS on
    /// this host — is the reason.
    pub fn new(registries: &[Registry]) -> Result<Self, String> {
        let http = reqwest::Client::builder().build().map_err(|error| error.to_string())?;
        Ok(RegistryClient {
            http,
            registries: registries.to_vec(),
            auths: Mutex::new(HashMap::new()),
        })
    }

    /// Whether the configuration lists `host`: a reference naming any
    /// other is never asked.
    pub fn lists(&self, host: &str) -> bool {
        self.registries.iter().any(|registry| registry.host == host)
    }

    /// The manifest under `digest` in `<host>/<name>`, whole; `None`
    /// where the registry answers `404`.
    pub async fn manifest(&self, host: &str, name: &str, digest: &str) -> Result<Option<Bytes>, String> {
        let Some(response) = self.get(host, name, &format!("manifests/{digest}"), Some(ACCEPT)).await? else {
            return Ok(None);
        };
        response.bytes().await.map(Some).map_err(|error| error.to_string())
    }

    /// The blob under `digest` in `<host>/<name>`, piece by piece;
    /// `None` where the registry answers `404`.
    pub async fn blob(&self, host: &str, name: &str, digest: &str) -> Result<Option<Body>, String> {
        let Some(response) = self.get(host, name, &format!("blobs/{digest}"), None).await? else {
            return Ok(None);
        };
        let body = response
            .bytes_stream()
            .take_while(|piece| future::ready(piece.is_ok()))
            .filter_map(|piece| future::ready(piece.ok()));
        Ok(Some(Box::pin(body)))
    }

    /// One `GET` of `<path>` under the repository, authorized as the
    /// repository last was, and once more after a `401`'s challenge is
    /// answered. `None` for a `404`.
    async fn get(&self, host: &str, name: &str, path: &str, accept: Option<&str>) -> Result<Option<Response>, String> {
        let Some(registry) = self.registries.iter().find(|registry| registry.host == host) else {
            return Err(format!("the registry `{host}` is not one the provider uses"));
        };
        let (api, name) = api(host, name);
        let url = format!("https://{api}/v2/{name}/{path}");
        let key = format!("{host}/{name}");
        let auth = self.auths.lock().await.get(&key).cloned();
        let response = self.send(&url, accept, registry.credential.as_ref(), auth.as_ref()).await?;
        let response = if response.status() == StatusCode::UNAUTHORIZED {
            let challenge = response
                .headers()
                .get(header::WWW_AUTHENTICATE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let auth = self.answer(&challenge, registry.credential.as_ref()).await?;
            self.auths.lock().await.insert(key, auth.clone());
            self.send(&url, accept, registry.credential.as_ref(), Some(&auth)).await?
        } else {
            response
        };
        match response.status() {
            StatusCode::OK => Ok(Some(response)),
            StatusCode::NOT_FOUND => Ok(None),
            status => Err(format!("`{url}` answered {status}")),
        }
    }

    /// The request sent, with `auth`.
    async fn send(&self, url: &str, accept: Option<&str>, credential: Option<&Credential>, auth: Option<&Auth>) -> Result<Response, String> {
        let mut request = self.http.get(url);
        if let Some(accept) = accept {
            request = request.header(header::ACCEPT, accept);
        }
        request = authorize(request, credential, auth);
        request.send().await.map_err(|error| error.to_string())
    }

    /// A `401`'s challenge answered: `Basic` with the credential,
    /// `Bearer` with a token from the realm. A challenge of any other
    /// scheme, or `Basic` with no credential, is the error.
    async fn answer(&self, challenge: &str, credential: Option<&Credential>) -> Result<Auth, String> {
        let (scheme, parameters) = challenge.split_once(' ').unwrap_or((challenge, ""));
        if scheme.eq_ignore_ascii_case("basic") {
            return match credential {
                Some(_) => Ok(Auth::Basic),
                None => Err("the registry asks for a credential the configuration does not list".to_string()),
            };
        }
        if !scheme.eq_ignore_ascii_case("bearer") {
            return Err(format!("the registry's challenge `{challenge}` is not one the provider answers"));
        }
        let parameters = parse(parameters);
        let realm = parameters.get("realm").ok_or("the registry's Bearer challenge names no realm")?;
        let mut query = Vec::new();
        for key in ["service", "scope"] {
            if let Some(value) = parameters.get(key) {
                query.push((key, value.as_str()));
            }
        }
        let mut request = self.http.get(realm.as_str()).query(&query);
        if let Some(credential) = credential {
            request = request.basic_auth(&credential.username, Some(&credential.password));
        }
        let response = request.send().await.map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("the token service answered {}", response.status()));
        }
        let body = json(response).await?;
        body.get("token")
            .or_else(|| body.get("access_token"))
            .and_then(Value::as_str)
            .map(|token| Auth::Bearer(token.to_string()))
            .ok_or_else(|| "the token service answered no token".to_string())
    }
}

/// The JSON of a response, read whole.
async fn json(response: Response) -> Result<Value, String> {
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

/// The request with its authorization: a Bearer token, the Basic
/// credential, or none.
fn authorize(request: RequestBuilder, credential: Option<&Credential>, auth: Option<&Auth>) -> RequestBuilder {
    match (auth, credential) {
        (Some(Auth::Bearer(token)), _) => request.bearer_auth(token),
        (Some(Auth::Basic), Some(credential)) => request.basic_auth(&credential.username, Some(&credential.password)),
        _ => request,
    }
}

/// The host the API is reached at, and the name as that host knows
/// it: Docker Hub's API host, and `library/` before a one-segment
/// name there.
fn api(host: &str, name: &str) -> (String, String) {
    if host == "docker.io" {
        let name = if name.contains('/') { name.to_string() } else { format!("library/{name}") };
        ("registry-1.docker.io".to_string(), name)
    } else {
        (host.to_string(), name.to_string())
    }
}

/// A challenge's parameters, `key="value"` or `key=value`, separated
/// by commas; a comma inside quotes is the value's.
fn parse(parameters: &str) -> HashMap<String, String> {
    let mut parsed = HashMap::new();
    let mut rest = parameters.trim();
    while !rest.is_empty() {
        let Some((key, after)) = rest.split_once('=') else {
            break;
        };
        let key = key.trim().trim_start_matches(',').trim().to_ascii_lowercase();
        let (value, after) = if let Some(quoted) = after.strip_prefix('"') {
            match quoted.find('"') {
                Some(end) => (&quoted[..end], &quoted[end + 1..]),
                None => (quoted, ""),
            }
        } else {
            match after.find(',') {
                Some(end) => (&after[..end], &after[end..]),
                None => (after, ""),
            }
        };
        parsed.insert(key, value.to_string());
        rest = after.trim_start().trim_start_matches(',').trim_start();
    }
    parsed
}
