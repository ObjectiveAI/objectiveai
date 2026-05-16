use crate::remote::Remote;

/// Remote path reference in `key=value,key=value` format.
///
/// Supported keys:
/// - `favorite` — favorite name
/// - `remote` — `github`, `filesystem`, or `mock`
/// - `owner` — github/filesystem owner
/// - `repository` — github/filesystem repository
/// - `name` — mock name
/// - `commit` — optional commit (github/filesystem)
///
/// # Examples
///
/// ```text
/// favorite=my-resource
/// remote=github,owner=user,repository=repo
/// remote=mock,name=test
/// ```
#[derive(Clone, Debug)]
pub struct PathRef {
    pub favorite: Option<String>,
    pub remote: Option<Remote>,
    pub owner: Option<String>,
    pub repository: Option<String>,
    pub name: Option<String>,
    pub commit: Option<String>,
}

impl std::str::FromStr for PathRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut path_ref = PathRef {
            favorite: None,
            remote: None,
            owner: None,
            repository: None,
            name: None,
            commit: None,
        };

        for pair in s.split(',') {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("expected key=value, got: {pair}"))?;
            match key.trim() {
                "favorite" => path_ref.favorite = Some(value.trim().to_string()),
                "remote" => path_ref.remote = Some(parse_remote(value.trim())?),
                "owner" => path_ref.owner = Some(value.trim().to_string()),
                "repository" => path_ref.repository = Some(value.trim().to_string()),
                "name" => path_ref.name = Some(value.trim().to_string()),
                "commit" => path_ref.commit = Some(value.trim().to_string()),
                other => return Err(format!("unknown key: {other}")),
            }
        }

        Ok(path_ref)
    }
}

impl std::fmt::Display for PathRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(fav) = &self.favorite {
            parts.push(format!("favorite={fav}"));
        }
        if let Some(remote) = &self.remote {
            let r = match remote {
                Remote::Github => "github",
                Remote::Filesystem => "filesystem",
                Remote::Mock => "mock",
            };
            parts.push(format!("remote={r}"));
        }
        if let Some(owner) = &self.owner {
            parts.push(format!("owner={owner}"));
        }
        if let Some(repo) = &self.repository {
            parts.push(format!("repository={repo}"));
        }
        if let Some(name) = &self.name {
            parts.push(format!("name={name}"));
        }
        if let Some(commit) = &self.commit {
            parts.push(format!("commit={commit}"));
        }
        write!(f, "{}", parts.join(","))
    }
}

fn parse_remote(s: &str) -> Result<Remote, String> {
    match s {
        "github" => Ok(Remote::Github),
        "filesystem" => Ok(Remote::Filesystem),
        "mock" => Ok(Remote::Mock),
        _ => Err(format!(
            "unknown remote: {s} (expected github, filesystem, or mock)"
        )),
    }
}

impl PathRef {
    pub fn resolve(self) -> Result<objectiveai_sdk::RemotePathCommitOptional, crate::error::Error> {
        self.remote
            .ok_or(crate::error::Error::MissingArgs(
                "remote is required",
            ))?
            .into_path(self.owner, self.repository, self.name, self.commit)
            .ok_or(crate::error::Error::MissingArgs(
                "owner and repository are required for github/filesystem, name for mock",
            ))
    }
}
