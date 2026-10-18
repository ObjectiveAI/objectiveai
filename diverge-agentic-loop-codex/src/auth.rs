//! The login: the agent's method, applied with the secret it was fed.
//!
//! The agent names the method ([`Auth`]) and the proxy feeds the one
//! secret the method declares; at each loop's start
//! [`Auth::resolve`] turns the two into what the run needs — the
//! variables the process gets, and a file to write under the home
//! when the method wants one — without dialing anything. A declared
//! name the feed lacks, or carries in the wrong JSON type, is the
//! run's refusal, naming the name and never a value.
//!
//! What goes where is Codex's own, not this crate's: `CODEX_API_KEY`
//! and `CODEX_ACCESS_TOKEN` are the variables its credential store
//! reads, in that order and above `auth.json`; `auth.json` under
//! `CODEX_HOME` is the file it reads and, on a ChatGPT refresh,
//! rewrites in place; `OPENAI_IDENTITY_TOKEN_FILE` names the token a
//! workload identity login exchanges. `OPENAI_API_KEY` in the
//! environment is none of these: the built-in provider has no
//! `env_key`, so the `api_key` method hands the fed key to Codex as
//! `CODEX_API_KEY`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use diverge_sdk::shared::containers::secrets::Values;
use serde_json::Value;

use crate::agent::Auth;

/// Codex's home, fixed for the container's life: its default for the
/// root user, set explicitly on the process so the geometry never
/// moves.
pub const CODEX_HOME: &str = "/root/.codex";

/// The name of Codex's login file, under the home.
pub const AUTH_FILE: &str = "auth.json";

/// The name of the workload identity token's file, under the home.
pub const IDENTITY_TOKEN_FILE: &str = "identity-token";

/// The secret the `api_key` method declares: an OpenAI API key.
pub const OPENAI_API_KEY: &str = "OPENAI_API_KEY";

/// The secret the `access_token` method declares, and the variable
/// it is handed to Codex under: a personal access token or an Agent
/// Identity JWT.
pub const CODEX_ACCESS_TOKEN: &str = "CODEX_ACCESS_TOKEN";

/// The secret the `auth_file` method declares: `auth.json`, whole.
pub const CODEX_AUTH: &str = "CODEX_AUTH";

/// The secret the `workload_identity` method declares: the identity
/// token.
pub const OPENAI_IDENTITY_TOKEN: &str = "OPENAI_IDENTITY_TOKEN";

/// The variable Codex's credential store reads first: the API key.
pub const CODEX_API_KEY: &str = "CODEX_API_KEY";

/// The variable naming the file the identity token is read from.
pub const OPENAI_IDENTITY_TOKEN_FILE: &str = "OPENAI_IDENTITY_TOKEN_FILE";

/// The variable naming the federation rule the token is exchanged
/// against.
pub const OPENAI_FEDERATION_RULE_ID: &str = "OPENAI_FEDERATION_RULE_ID";

/// The variable carrying the workload's context, when the rule wants
/// one.
pub const OPENAI_WORKLOAD_IDENTITY_CONTEXT: &str = "OPENAI_WORKLOAD_IDENTITY_CONTEXT";

/// Where Codex keeps its login: [`AUTH_FILE`] under [`CODEX_HOME`].
pub fn auth_file() -> PathBuf {
    Path::new(CODEX_HOME).join(AUTH_FILE)
}

/// Where the identity token is written: [`IDENTITY_TOKEN_FILE`]
/// under [`CODEX_HOME`].
pub fn identity_token_file() -> PathBuf {
    Path::new(CODEX_HOME).join(IDENTITY_TOKEN_FILE)
}

/// The login, ready for the process: what [`Auth::resolve`] made of
/// the method and the fed secret.
pub struct Resolved {
    /// The variables the process gets, beside `CODEX_HOME`.
    pub env: BTreeMap<String, String>,
    /// A file to write under the home before the first turn, when
    /// the method wants one: `auth.json`, or the identity token.
    pub file: Option<(PathBuf, Vec<u8>)>,
}

impl Resolved {
    /// Write the file, when there is one, the home made as needed.
    pub async fn write(&self) -> io::Result<()> {
        let Some((path, bytes)) = &self.file else {
            return Ok(());
        };
        tokio::fs::create_dir_all(CODEX_HOME).await?;
        tokio::fs::write(path, bytes).await
    }
}

impl Auth {
    /// The method applied with the fed values: the one name it
    /// declares, taken from them and placed where Codex reads it.
    pub fn resolve(&self, values: &Values) -> Result<Resolved, Error> {
        let name = self.name();
        let value = values.get(name).ok_or(Error::Missing(name))?;
        let mut env = BTreeMap::new();
        let mut file = None;
        match self {
            Auth::ApiKey => {
                env.insert(CODEX_API_KEY.to_string(), text(name, value)?);
            }
            Auth::AccessToken => {
                env.insert(CODEX_ACCESS_TOKEN.to_string(), text(name, value)?);
            }
            Auth::AuthFile => {
                if !value.is_object() {
                    return Err(Error::NotObject(name));
                }
                let bytes = serde_json::to_vec(value).map_err(|_| Error::NotObject(name))?;
                file = Some((auth_file(), bytes));
            }
            Auth::WorkloadIdentity {
                federation_rule_id,
                context,
            } => {
                let token = text(name, value)?;
                let path = identity_token_file();
                env.insert(
                    OPENAI_IDENTITY_TOKEN_FILE.to_string(),
                    path.to_string_lossy().into_owned(),
                );
                env.insert(OPENAI_FEDERATION_RULE_ID.to_string(), federation_rule_id.clone());
                if let Some(context) = context {
                    env.insert(OPENAI_WORKLOAD_IDENTITY_CONTEXT.to_string(), context.clone());
                }
                file = Some((path, token.into_bytes()));
            }
        }
        Ok(Resolved { env, file })
    }

    /// What the loop leaves in its rotating secret, read back after
    /// the last turn: for `auth_file`, `auth.json` as Codex left it,
    /// under the name it was declared. `None` for the other methods,
    /// which hand nothing back, and for a file that cannot be read
    /// or is not JSON — the fed value then stands for the collect.
    pub async fn read_back(&self) -> Option<(&'static str, Value)> {
        match self {
            Auth::AuthFile => {
                let bytes = tokio::fs::read(auth_file()).await.ok()?;
                let value = serde_json::from_slice::<Value>(&bytes).ok()?;
                Some((CODEX_AUTH, value))
            }
            Auth::ApiKey | Auth::AccessToken | Auth::WorkloadIdentity { .. } => None,
        }
    }
}

/// The value as the text a variable or a token file carries.
fn text(name: &'static str, value: &Value) -> Result<String, Error> {
    match value {
        Value::String(text) => Ok(text.clone()),
        _ => Err(Error::NotText(name)),
    }
}

/// The login could not be made from the fed secrets.
#[derive(Debug)]
pub enum Error {
    /// The declared name is not among the fed values.
    Missing(&'static str),
    /// The fed value is not a JSON string, and the method wants one.
    NotText(&'static str),
    /// The fed value is not a JSON object, and `auth_file` wants one.
    NotObject(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(name) => write!(f, "the secret {name} was not fed"),
            Error::NotText(name) => write!(f, "the secret {name} is not a JSON string"),
            Error::NotObject(name) => write!(f, "the secret {name} is not a JSON object"),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// The failure as JSON, the shape every failure on the stream
    /// has. Names the secret, never a value.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "secrets",
            "error": self.to_string(),
        })
    }
}
