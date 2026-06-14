//! Local read + list of published agents/swarms/functions/profiles.
//!
//! Repositories live at `state_dir/<kind>/<owner>/<repository>` (the same
//! layout [`publish`](super::publish) writes to) and are git repos. Reads
//! resolve the file either from the working tree (resolving HEAD for the
//! reported commit) or from a specific commit; listing walks the kind
//! directory as a concurrent stream, resolving each repo's HEAD and
//! validating its JSON, skipping anything that fails.
//!
//! All directory/file IO uses `tokio::fs`; git (libgit2, blocking) runs on
//! `spawn_blocking`.

use std::path::{Path, PathBuf};
use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::RemotePath;
use serde::de::DeserializeOwned;

use super::publish::Kind;
use super::{Client, Error};

impl Client {
    /// `state_dir/<kind>/<owner>/<repository>`.
    fn repo_path(&self, kind: Kind, owner: &str, repository: &str) -> PathBuf {
        self.state_dir()
            .join(kind.as_str())
            .join(owner)
            .join(repository)
    }

    /// Resolves the HEAD commit SHA for a repository.
    pub fn resolve_head(
        &self,
        kind: Kind,
        owner: &str,
        repository: &str,
    ) -> Result<String, Error> {
        Ok(head_sha(&self.repo_path(kind, owner, repository))?)
    }

    /// Reads and deserializes a kind's JSON file from a repository.
    ///
    /// `commit = Some` reads from that specific git commit; `commit = None`
    /// reads the working tree and resolves HEAD for the reported commit.
    /// Returns `Ok(None)` if the repository or file does not exist.
    pub async fn read_json<T: DeserializeOwned>(
        &self,
        kind: Kind,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<Option<(T, String)>, Error> {
        let repo_path = self.repo_path(kind, owner, repository);
        let file_name = kind.filename();

        let (content, resolved) = match commit {
            Some(sha) => match read_file_at_commit(&repo_path, file_name, sha) {
                Ok(content) => (content, sha.to_string()),
                Err(e) if is_not_found(&e) => return Ok(None),
                Err(e) => return Err(e),
            },
            None => {
                let file_path = repo_path.join(file_name);
                match tokio::fs::read_to_string(&file_path).await {
                    Ok(content) => {
                        let resolved = head_sha(&repo_path)
                            .unwrap_or_else(|_| "HEAD".to_string());
                        (content, resolved)
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok(None);
                    }
                    Err(e) => return Err(Error::Read(file_path, e)),
                }
            }
        };

        let value = serde_json::from_str::<T>(&content)
            .map_err(|e| Error::Parse(repo_path.join(file_name), e))?;
        Ok(Some((value, resolved)))
    }

    /// Streams every valid repository of `kind` under `state_dir/<kind>`.
    ///
    /// The outer stream lists the `<owner>` directories; each owner is then
    /// flat-mapped into an inner stream listing its `<repository>`
    /// directories, and each repo resolves to a `Filesystem` remote path by
    /// reading HEAD and parsing the kind's JSON. Repos whose HEAD or JSON
    /// fails are skipped. Nothing is buffered into a `Vec`: directories are
    /// walked lazily as the stream is polled.
    pub async fn list(
        &self,
        kind: Kind,
    ) -> Pin<Box<dyn Stream<Item = RemotePath> + Send>> {
        let kind_dir = self.state_dir().join(kind.as_str());
        Box::pin(
            // Outer stream: the `<owner>` directories under `<kind>`.
            read_dir_stream(kind_dir)
                .filter_map(|owner| async move { is_dir(&owner).await.then_some(owner) })
                // Each owner becomes its own inner stream of repositories.
                .flat_map(move |owner| {
                    let owner_name = owner.file_name().to_string_lossy().into_owned();
                    // Inner stream: the `<repository>` directories of this owner.
                    read_dir_stream(owner.path())
                        .filter_map(|repo| async move {
                            is_dir(&repo).await.then_some(repo)
                        })
                        .map(move |repo| {
                            let owner = owner_name.clone();
                            let repository =
                                repo.file_name().to_string_lossy().into_owned();
                            resolve_entry(kind, owner, repository, repo.path())
                        })
                        .buffer_unordered(16)
                        .filter_map(|opt| async move { opt })
                }),
        )
    }
}

/// Whether a directory entry is itself a directory (false on any IO error).
async fn is_dir(entry: &tokio::fs::DirEntry) -> bool {
    entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false)
}

/// Lazily streams the entries of `dir`. The directory is opened on first
/// poll; if it can't be opened the stream is simply empty.
fn read_dir_stream(
    dir: PathBuf,
) -> impl Stream<Item = tokio::fs::DirEntry> + Send {
    enum State {
        Unopened(PathBuf),
        Open(tokio::fs::ReadDir),
    }
    futures::stream::unfold(State::Unopened(dir), |state| async move {
        let mut read_dir = match state {
            State::Unopened(dir) => tokio::fs::read_dir(&dir).await.ok()?,
            State::Open(read_dir) => read_dir,
        };
        match read_dir.next_entry().await {
            Ok(Some(entry)) => Some((entry, State::Open(read_dir))),
            _ => None,
        }
    })
}

/// Resolves one candidate repo to a `Filesystem` remote path, or `None` if
/// its HEAD can't be resolved or its JSON doesn't parse.
async fn resolve_entry(
    kind: Kind,
    owner: String,
    repository: String,
    repo_path: PathBuf,
) -> Option<RemotePath> {
    // git HEAD resolution is blocking — run it off the async runtime.
    let head_path = repo_path.clone();
    let commit = tokio::task::spawn_blocking(move || head_sha(&head_path))
        .await
        .ok()?
        .ok()?;
    // Validate the JSON parses as the kind's definition.
    let content =
        tokio::fs::read_to_string(repo_path.join(kind.filename())).await.ok()?;
    if !validates(kind, &content) {
        return None;
    }
    Some(RemotePath::Filesystem { owner, repository, commit })
}

/// Resolves the HEAD commit SHA for a repository directory (blocking git).
fn head_sha(repo_path: &Path) -> Result<String, git2::Error> {
    let repo = git2::Repository::open(repo_path)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}

/// Whether `content` parses as the kind's base/full definition.
fn validates(kind: Kind, content: &str) -> bool {
    match kind {
        Kind::Agents => serde_json::from_str::<
            objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks,
        >(content)
        .is_ok(),
        Kind::Swarms => {
            serde_json::from_str::<objectiveai_sdk::swarm::RemoteSwarmBase>(
                content,
            )
            .is_ok()
        }
        Kind::Functions => serde_json::from_str::<
            objectiveai_sdk::functions::FullRemoteFunction,
        >(content)
        .is_ok(),
        Kind::Profiles => serde_json::from_str::<
            objectiveai_sdk::functions::RemoteProfile,
        >(content)
        .is_ok(),
    }
}

/// Returns true if the git error represents a "not found" condition.
fn is_not_found(e: &Error) -> bool {
    match e {
        Error::Git(e) => {
            e.code() == git2::ErrorCode::NotFound
                || e.class() == git2::ErrorClass::Object
                || e.class() == git2::ErrorClass::Reference
        }
        _ => false,
    }
}

/// Reads a file from a git repository at a specific commit (blocking git).
fn read_file_at_commit(
    repo_path: &Path,
    file_name: &str,
    commit_sha: &str,
) -> Result<String, Error> {
    let repo = git2::Repository::open(repo_path)?;
    let oid = git2::Oid::from_str(commit_sha)?;
    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;
    let entry = tree.get_name(file_name).ok_or_else(|| {
        git2::Error::from_str(&format!(
            "{} not found at commit {}",
            file_name, commit_sha
        ))
    })?;
    let blob = repo.find_blob(entry.id())?;
    String::from_utf8(blob.content().to_vec())
        .map_err(|e| Error::Utf8(repo_path.join(file_name), e))
}
