//! Typed paths shared across cli leaves.
//!
//! Every cli leaf that takes a docker-style `key=value,key=value` ref
//! parses it at `TryFrom<Args>` time so each leaf's `Request` carries
//! the pre-parsed form: [`FromStr for crate::RemotePathCommitOptional`]
//! accepts `remote=<github|client|mock>` plus the matching
//! owner/repository/name/commit fields. Any other key is an
//! `unknown key` error.

use std::str::FromStr;

/// Wire-level source enum: which remote backend a path references.
/// Same shape as `crate::Remote` but lives under `cli::command` so it
/// can be `clap::ValueEnum` without dragging clap into the SDK's
/// non-cli surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Remote {
    Github,
    Client,
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
            Remote::Client => Some(crate::RemotePathCommitOptional::Client {
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
            "client" => Ok(Self::Client),
            "mock" => Ok(Self::Mock),
            other => Err(format!(
                "unknown remote: {other} (expected github, client, or mock)"
            )),
        }
    }
}

/// Tokenize a `key=value,key=value` string into a `Vec<(key, value)>`,
/// trimming whitespace around each token. Returns an error if any
/// pair lacks an `=`.
pub(crate) fn tokenize(s: &str) -> Result<Vec<(&str, &str)>, String> {
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
    /// `repository`, `name`, `commit`. Anything else is an
    /// unknown-key error.
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
                "owner and repository are required for github/client, name for mock"
                    .to_string()
            })
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
        crate::RemotePathCommitOptional::Client {
            owner,
            repository,
            commit,
        } => {
            let mut s = format!("remote=client,owner={owner},repository={repository}");
            if let Some(c) = commit {
                s.push_str(&format!(",commit={c}"));
            }
            s
        }
        crate::RemotePathCommitOptional::Mock { name } => format!("remote=mock,name={name}"),
    }
}
