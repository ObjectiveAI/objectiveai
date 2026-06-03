//! Typed paths shared across cli leaves.
//!
//! Every cli leaf that takes a docker-style `key=value,key=value` ref
//! parses it at `TryFrom<Args>` time so each leaf's `Request` carries
//! the pre-parsed form. There are **two distinct parsers** because
//! different leaves accept different sets of keys:
//!
//! - [`FromStr for crate::RemotePathCommitOptional`] — for leaves
//!   that take a *path only* (e.g. `config/<domain>/favorites/add`).
//!   Accepts `remote=<github|filesystem|mock>` plus the matching
//!   owner/repository/name/commit fields. Any other key (including
//!   `favorite=`) is an `unknown key` error.
//! - [`FromStr for RemotePathCommitOptionalOrFavorite`] — for leaves
//!   that take *either* a favorite name *or* a path (e.g. the `*get`
//!   leaves). A string of the exact form `favorite=<name>` becomes
//!   `Favorite(name)`; otherwise the string is parsed by the
//!   remote-only parser and lifted into `Resolved(path)`.
//!
//! Mixing `favorite=` with remote keys is an error in the
//! favorite-or-path parser.

use std::str::FromStr;

/// Wire-level source enum: which remote backend a path references.
/// Same shape as `crate::Remote` but lives under `cli::command` so it
/// can be `clap::ValueEnum` without dragging clap into the SDK's
/// non-cli surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Remote {
    Github,
    Filesystem,
    Mock,
}

impl Remote {
    /// Combine `(remote, owner?, repository?, name?, commit?)` into a
    /// concrete `RemotePathCommitOptional`. Returns `None` if a
    /// required field is missing for the chosen backend.
    pub fn into_path(
        self,
        owner: Option<String>,
        repository: Option<String>,
        name: Option<String>,
        commit: Option<String>,
    ) -> Option<crate::RemotePathCommitOptional> {
        match self {
            Remote::Github => Some(crate::RemotePathCommitOptional::Github {
                owner: owner?,
                repository: repository?,
                commit,
            }),
            Remote::Filesystem => Some(crate::RemotePathCommitOptional::Filesystem {
                owner: owner?,
                repository: repository?,
                commit,
            }),
            Remote::Mock => Some(crate::RemotePathCommitOptional::Mock { name: name? }),
        }
    }

    fn parse_keyword(s: &str) -> Result<Self, String> {
        match s {
            "github" => Ok(Self::Github),
            "filesystem" => Ok(Self::Filesystem),
            "mock" => Ok(Self::Mock),
            other => Err(format!(
                "unknown remote: {other} (expected github, filesystem, or mock)"
            )),
        }
    }
}

/// Tokenize a `key=value,key=value` string into a `Vec<(key, value)>`,
/// trimming whitespace around each token. Returns an error if any
/// pair lacks an `=`.
fn tokenize(s: &str) -> Result<Vec<(&str, &str)>, String> {
    s.split(',')
        .map(|pair| {
            pair.split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .ok_or_else(|| format!("expected key=value, got: {pair}"))
        })
        .collect()
}

impl FromStr for crate::RemotePathCommitOptional {
    type Err = String;

    /// Parse a remote-path ref. Accepted keys: `remote`, `owner`,
    /// `repository`, `name`, `commit`. Anything else (including
    /// `favorite=`) is an unknown-key error.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut remote: Option<Remote> = None;
        let mut owner: Option<String> = None;
        let mut repository: Option<String> = None;
        let mut name: Option<String> = None;
        let mut commit: Option<String> = None;
        for (k, v) in tokenize(s)? {
            match k {
                "remote" => remote = Some(Remote::parse_keyword(v)?),
                "owner" => owner = Some(v.to_string()),
                "repository" => repository = Some(v.to_string()),
                "name" => name = Some(v.to_string()),
                "commit" => commit = Some(v.to_string()),
                other => return Err(format!("unknown key: {other}")),
            }
        }
        let remote = remote.ok_or_else(|| "remote is required".to_string())?;
        remote
            .into_path(owner, repository, name, commit)
            .ok_or_else(|| {
                "owner and repository are required for github/filesystem, name for mock"
                    .to_string()
            })
    }
}

/// Either a fully resolved `RemotePathCommitOptional` or a favorite
/// name that the cli host resolves at handler time. Parsed from CLI
/// input via [`FromStr`] (docker-style `key=value,...`); after that
/// the typed enum is carried around in-process — `Request`s are never
/// actually serialized at runtime. The `serde` + `schemars::JsonSchema`
/// derives exist solely so the SDK can generate the JSON Schema for
/// this type (untagged so the schema describes either the path's
/// object form or a bare string).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum RemotePathCommitOptionalOrFavorite {
    #[schemars(title = "Resolved")]
    Resolved(crate::RemotePathCommitOptional),
    #[schemars(title = "Favorite")]
    Favorite(String),
}

impl RemotePathCommitOptionalOrFavorite {
    /// Reconstruct a docker-style `key=value,...` string from this
    /// typed value. Used by `into_command` to round-trip the
    /// `Request` back through argv for subprocess dispatch.
    pub fn into_arg_string(&self) -> String {
        match self {
            Self::Favorite(name) => format!("favorite={name}"),
            Self::Resolved(path) => remote_path_to_arg_string(path),
        }
    }
}

impl FromStr for RemotePathCommitOptionalOrFavorite {
    type Err = String;

    /// Parse a favorite-or-path ref. Two accepted forms:
    /// - `favorite=<name>` on its own — combining `favorite=` with
    ///   any other key is an error.
    /// - Anything else — delegated to
    ///   [`FromStr for crate::RemotePathCommitOptional`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pairs = tokenize(s)?;
        if let Some((_, value)) = pairs.iter().find(|(k, _)| *k == "favorite") {
            if pairs.len() > 1 {
                return Err(
                    "favorite= cannot be combined with other keys".to_string(),
                );
            }
            return Ok(Self::Favorite((*value).to_string()));
        }
        s.parse::<crate::RemotePathCommitOptional>().map(Self::Resolved)
    }
}

/// Serialize a `RemotePathCommitOptional` back into its docker-style
/// `key=value,...` form for round-tripping through argv.
pub fn remote_path_to_arg_string(path: &crate::RemotePathCommitOptional) -> String {
    match path {
        crate::RemotePathCommitOptional::Github {
            owner,
            repository,
            commit,
        } => {
            let mut s = format!("remote=github,owner={owner},repository={repository}");
            if let Some(c) = commit {
                s.push_str(&format!(",commit={c}"));
            }
            s
        }
        crate::RemotePathCommitOptional::Filesystem {
            owner,
            repository,
            commit,
        } => {
            let mut s = format!("remote=filesystem,owner={owner},repository={repository}");
            if let Some(c) = commit {
                s.push_str(&format!(",commit={c}"));
            }
            s
        }
        crate::RemotePathCommitOptional::Mock { name } => format!("remote=mock,name={name}"),
    }
}
