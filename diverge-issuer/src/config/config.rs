//! The document itself.

use serde::{Deserialize, Serialize};

/// The port an issuer listens on when the document names none, which
/// no other Diverge program takes.
pub const DEFAULT_PORT: u16 = 14981;

/// The whole of `issuer.yaml`: what the issuer is told.
///
/// An absent file is this type's [`Default`], and the issuer runs on
/// it: [`DEFAULT_PORT`], the issuer named by its loopback address, and
/// the lifetimes each field's doc gives. Every field may be absent,
/// meaning its default. An unknown key is an error, so a misspelled
/// setting is refused rather than ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The TCP port the issuer serves HTTP on, on every interface.
    pub port: u16,
    /// The issuer identifier: the URL every token's `iss` is, exactly,
    /// and every endpoint is under. It is `https`, or `http` for a
    /// loopback host, with no query and no fragment; the issuer serves
    /// plain HTTP, so an `https` issuer is one a proxy in front of it
    /// terminates TLS for. Absent means `http://127.0.0.1:<port>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    /// How long an access token is good for, in seconds. Absent means
    /// `600`.
    pub access_token_seconds: u64,
    /// How long a refresh token is good for, in days, counted from
    /// when it was issued; a refresh issues a new one. Absent means
    /// `30`.
    pub refresh_token_days: u64,
    /// How long an authorization code is good for, in seconds. Absent
    /// means `60`.
    pub code_seconds: u64,
    /// How long a device code and its user code are good for, in
    /// seconds. Absent means `600`.
    pub device_code_seconds: u64,
    /// The seconds a device waits between one poll of the token
    /// endpoint and the next. Absent means `5`.
    pub device_interval_seconds: u64,
}

impl Config {
    /// The issuer identifier: `issuer` as the document gives it, or
    /// `http://127.0.0.1:<port>` where it gives none.
    pub fn issuer(&self) -> String {
        match &self.issuer {
            Some(issuer) => issuer.clone(),
            None => format!("http://127.0.0.1:{}", self.port),
        }
    }

    /// The URL of the endpoint at `path` under the issuer: `path`
    /// appended to the issuer identifier, a slash between them.
    pub fn endpoint(&self, path: &str) -> String {
        format!("{}/{path}", self.issuer().trim_end_matches('/'))
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: DEFAULT_PORT,
            issuer: None,
            access_token_seconds: 600,
            refresh_token_days: 30,
            code_seconds: 60,
            device_code_seconds: 600,
            device_interval_seconds: 5,
        }
    }
}
