//! Fetching an issuer's key set.

use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use reqwest::header::CACHE_CONTROL;

use super::entry::Entry;
use super::{KeySets, VerifyError};
use crate::identities::discovery::{at, discover};

/// The shortest a `max-age` holds a key set for.
const SHORTEST: Duration = Duration::from_secs(60);

/// The longest a `max-age` holds a key set for.
const LONGEST: Duration = Duration::from_secs(24 * 60 * 60);

impl KeySets {
    /// `issuer`'s key set, fetched now — see the [rules](super), rule
    /// 6 — and when it stops being fresh, rule 4.
    pub(super) async fn fetch(&self, issuer: &str) -> Result<Entry, VerifyError> {
        let failed = |why: String| VerifyError::Fetch(why);
        let discovery = discover(&self.http, issuer).await.map_err(|error| failed(error.to_string()))?;
        let jwks_uri =
            discovery.jwks_uri.ok_or_else(|| failed(format!("the document at `{}` names no jwks_uri", at(issuer))))?;
        let response = self.http.get(&jwks_uri).send().await.map_err(|error| failed(error.to_string()))?;
        let response = response.error_for_status().map_err(|error| failed(error.to_string()))?;
        let lifetime = max_age(response.headers().get(CACHE_CONTROL).and_then(|value| value.to_str().ok()))
            .map(|age| age.clamp(SHORTEST, LONGEST))
            .unwrap_or(self.lifetime);
        let keys: JwkSet = response.json().await.map_err(|error| failed(error.to_string()))?;
        let fetched = Instant::now();
        Ok(Entry { keys: Arc::new(keys), fetched, expires: fetched + lifetime })
    }
}

/// The `max-age` a `Cache-Control` value names, where it names one.
fn max_age(cache_control: Option<&str>) -> Option<Duration> {
    cache_control?
        .split(',')
        .filter_map(|directive| directive.trim().strip_prefix("max-age="))
        .find_map(|seconds| seconds.trim_matches('"').parse().ok())
        .map(Duration::from_secs)
}
