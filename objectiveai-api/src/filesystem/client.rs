use crate::retrieval::Kind;
use std::path::{Path, PathBuf};

/// Reads files from local git repositories on the filesystem.
///
/// Repositories are organized by kind under the base directory:
/// - Functions: `{base_dir}/functions/{owner}/{repository}/`
/// - Profiles: `{base_dir}/profiles/{owner}/{repository}/`
#[derive(Debug, Clone)]
pub struct Client {
    pub base_dir: PathBuf,
}

impl Client {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Returns the repository path for the given kind, owner, and repository.
    pub fn repo_path(&self, kind: Kind, owner: &str, repository: &str) -> PathBuf {
        self.base_dir.join(kind.as_str()).join(owner).join(repository)
    }

    /// Resolves the HEAD commit SHA for a repository.
    pub fn resolve_head(&self, kind: Kind, owner: &str, repository: &str) -> Result<String, super::Error> {
        let repo_path = self.repo_path(kind, owner, repository);
        let repo = git2::Repository::open(&repo_path)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Reads a file's raw content from a repository.
    ///
    /// If `commit` is `Some`, reads from that specific git commit.
    /// If `commit` is `None`, reads from the working tree and resolves HEAD.
    ///
    /// Returns `Ok(None)` if the repository or file does not exist.
    /// Returns `Ok(Some((content, resolved_commit)))` on success.
    pub async fn read_file(
        &self,
        kind: Kind,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
        file_name: &str,
    ) -> Result<Option<(String, String)>, super::Error> {
        let repo_path = self.repo_path(kind, owner, repository);

        match commit {
            Some(sha) => {
                match read_file_at_commit(&repo_path, file_name, sha) {
                    Ok(content) => Ok(Some((content, sha.to_string()))),
                    Err(e) if is_not_found(&e) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            None => {
                let file_path = repo_path.join(file_name);
                match tokio::fs::read_to_string(&file_path).await {
                    Ok(content) => {
                        let resolved = self
                            .resolve_head(kind, owner, repository)
                            .unwrap_or_else(|_| "HEAD".to_string());
                        Ok(Some((content, resolved)))
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(e) => Err(e.into()),
                }
            }
        }
    }

    /// Reads and deserializes a JSON file from a repository.
    ///
    /// If `commit` is `Some`, reads from that specific git commit.
    /// If `commit` is `None`, reads from the working tree and resolves HEAD.
    ///
    /// Returns `Ok(None)` if the repository or file does not exist.
    /// Returns `Ok(Some((value, resolved_commit)))` on success.
    pub async fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        kind: Kind,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
        file_name: &str,
    ) -> Result<Option<(T, String)>, super::Error> {
        let Some((content, resolved_commit)) =
            self.read_file(kind, owner, repository, commit, file_name).await?
        else {
            return Ok(None);
        };

        let mut de = serde_json::Deserializer::from_str(&content);
        let value = serde_path_to_error::deserialize(&mut de)?;
        Ok(Some((value, resolved_commit)))
    }

}

/// Returns true if the git error represents a "not found" condition.
fn is_not_found(e: &super::Error) -> bool {
    match e {
        super::Error::Git(e) => {
            e.code() == git2::ErrorCode::NotFound
                || e.class() == git2::ErrorClass::Object
                || e.class() == git2::ErrorClass::Reference
        }
        _ => false,
    }
}

/// Reads a file from a git repository at a specific commit.
fn read_file_at_commit(
    repo_path: &Path,
    file_name: &str,
    commit_sha: &str,
) -> Result<String, super::Error> {
    let repo = git2::Repository::open(repo_path)?;
    let oid = git2::Oid::from_str(commit_sha)?;
    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;
    let entry = tree
        .get_name(file_name)
        .ok_or_else(|| git2::Error::from_str(&format!("{} not found at commit {}", file_name, commit_sha)))?;
    let blob = repo.find_blob(entry.id())?;
    let content = std::str::from_utf8(blob.content())?;
    Ok(content.to_string())
}
