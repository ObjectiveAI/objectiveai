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
    pub commit_author_name: String,
    pub commit_author_email: String,
}

impl Client {
    pub fn new(base_dir: PathBuf, commit_author_name: String, commit_author_email: String) -> Self {
        Self { base_dir, commit_author_name, commit_author_email }
    }

    /// Removes the entire base directory and all its contents.
    pub fn clear(&self) -> std::io::Result<()> {
        if self.base_dir.exists() {
            std::fs::remove_dir_all(&self.base_dir)?;
        }
        Ok(())
    }

    /// Returns the repository path for the given kind, owner, and repository.
    pub fn repo_path(&self, kind: Kind, owner: &str, repository: &str) -> PathBuf {
        self.base_dir.join(kind.as_str()).join(owner).join(repository)
    }

    /// Checks whether a repository exists on the filesystem as an initialized git repository.
    pub fn repository_exists(&self, kind: Kind, owner: &str, repository: &str) -> bool {
        let repo_path = self.repo_path(kind, owner, repository);
        git2::Repository::open(&repo_path).is_ok()
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

    /// Publishes files to a local git repository.
    ///
    /// Handles any initial state: creates the directory if needed, initializes
    /// or resets the git repository, writes files, and commits.
    ///
    /// Returns the commit SHA on success.
    pub async fn publish<CTXEXT: crate::ctx::ContextExt>(
        &self,
        ctx: &crate::ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        kind: Kind,
        repository: &str,
        files: &[(&str, &str)],
        commit_message: &str,
    ) -> Result<(String, String), super::Error> {
        let (ctx_name, ctx_email) = tokio::join!(
            ctx.commit_author_name(),
            ctx.commit_author_email(),
        );
        let commit_author_name = ctx_name
            .map(|a| a.to_string())
            .unwrap_or_else(|| self.commit_author_name.clone());
        let commit_author_email = ctx_email
            .map(|a| a.to_string())
            .unwrap_or_else(|| self.commit_author_email.clone());
        // Owner is the resolved commit author name.
        let owner = commit_author_name.clone();
        let repo_path = self.repo_path(kind, &owner, repository);

        // Create directory recursively if needed.
        std::fs::create_dir_all(&repo_path)?;

        // Open or initialize the git repository.
        let repo = match git2::Repository::open(&repo_path) {
            Ok(repo) => {
                // Reset working tree to clean state.
                let mut checkout = git2::build::CheckoutBuilder::new();
                checkout.force();
                checkout.remove_untracked(true);
                if let Ok(head) = repo.head() {
                    if let Ok(commit) = head.peel_to_commit() {
                        repo.reset(
                            commit.as_object(),
                            git2::ResetType::Hard,
                            Some(&mut checkout),
                        )?;
                    }
                }
                repo
            }
            Err(_) => git2::Repository::init(&repo_path)?,
        };

        // Write files to the working tree.
        for (name, content) in files {
            let file_path = repo_path.join(name);
            std::fs::write(&file_path, content)?;
        }

        // Stage all files.
        let mut index = repo.index()?;
        for (name, _) in files {
            index.add_path(Path::new(name))?;
        }
        index.write()?;
        let tree_oid = index.write_tree()?;

        // If the tree is identical to the parent's tree, skip the commit.
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        if let Some(ref parent) = parent {
            if parent.tree_id() == tree_oid {
                return Ok((owner, parent.id().to_string()));
            }
        }

        let tree = repo.find_tree(tree_oid)?;

        // Create commit.
        let sig = git2::Signature::now(&commit_author_name, &commit_author_email)?;
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        let commit_oid = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            commit_message,
            &tree,
            &parents,
        )?;

        Ok((owner, commit_oid.to_string()))
    }

    /// Publishes files to a local git repository and pushes to a remote.
    ///
    /// Like [`publish`], but also sets the remote URL, fetches existing content,
    /// and pushes the commit using the provided token for authentication.
    ///
    /// Returns the commit SHA on success.
    pub async fn publish_and_push<CTXEXT: crate::ctx::ContextExt>(
        &self,
        ctx: &crate::ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        kind: Kind,
        repository: &str,
        files: &[(&str, &str)],
        commit_message: &str,
        remote_url: &str,
        token: &str,
    ) -> Result<(String, String), super::Error> {
        let (ctx_name, ctx_email) = tokio::join!(
            ctx.commit_author_name(),
            ctx.commit_author_email(),
        );
        let commit_author_name = ctx_name
            .map(|a| a.to_string())
            .unwrap_or_else(|| self.commit_author_name.clone());
        let commit_author_email = ctx_email
            .map(|a| a.to_string())
            .unwrap_or_else(|| self.commit_author_email.clone());
        // Owner is the resolved commit author name.
        let owner = commit_author_name.clone();
        let repo_path = self.repo_path(kind, &owner, repository);

        // Create directory recursively if needed.
        std::fs::create_dir_all(&repo_path)?;

        // Open or initialize the git repository.
        let repo = match git2::Repository::open(&repo_path) {
            Ok(r) => r,
            Err(_) => git2::Repository::init(&repo_path)?,
        };

        // Set or update remote URL.
        match repo.find_remote("origin") {
            Ok(_) => {
                repo.remote_set_url("origin", remote_url)?;
            }
            Err(_) => {
                repo.remote("origin", remote_url)?;
            }
        }

        // Fetch from remote (may have content from a previous push or manual commit).
        {
            for branch in &["main", "master"] {
                let mut callbacks = git2::RemoteCallbacks::new();
                callbacks.credentials(|_url, _username_from_url, _allowed_types| {
                    git2::Cred::userpass_plaintext("x-access-token", token)
                });
                let mut fetch_options = git2::FetchOptions::new();
                fetch_options.remote_callbacks(callbacks);
                let mut remote = repo.find_remote("origin")?;
                let _ = remote.fetch(&[branch], Some(&mut fetch_options), None);
            }

            // Reset to remote HEAD if it has commits.
            for branch in &["refs/remotes/origin/main", "refs/remotes/origin/master"] {
                if let Ok(reference) = repo.find_reference(branch) {
                    if let Ok(commit) = reference.peel_to_commit() {
                        let mut checkout = git2::build::CheckoutBuilder::new();
                        checkout.force();
                        checkout.remove_untracked(true);
                        let _ = repo.reset(
                            commit.as_object(),
                            git2::ResetType::Hard,
                            Some(&mut checkout),
                        );
                        let local_branch = branch.replace("refs/remotes/origin/", "");
                        let _ = repo.set_head(
                            &format!("refs/heads/{}", local_branch),
                        );
                        break;
                    }
                }
            }
        }

        // Write files to the working tree.
        for (name, content) in files {
            let file_path = repo_path.join(name);
            std::fs::write(&file_path, content)?;
        }

        // Stage all files.
        let mut index = repo.index()?;
        for (name, _) in files {
            index.add_path(Path::new(name))?;
        }
        index.write()?;
        let tree_oid = index.write_tree()?;

        // If the tree is identical to the parent's tree, skip commit and push.
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        if let Some(ref parent) = parent {
            if parent.tree_id() == tree_oid {
                return Ok((owner, parent.id().to_string()));
            }
        }

        let tree = repo.find_tree(tree_oid)?;

        // Create commit.
        let sig = git2::Signature::now(&commit_author_name, &commit_author_email)?;
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        let commit_oid = repo.commit(
            Some("HEAD"), &sig, &sig, commit_message, &tree, &parents,
        )?;

        // Push using credentials callback (token stays in memory only).
        let mut remote = repo.find_remote("origin")?;
        let head_ref = repo.head()?;
        let branch_name = head_ref.shorthand().unwrap_or("main");
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            git2::Cred::userpass_plaintext("x-access-token", token)
        });
        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_options))?;

        Ok((owner, commit_oid.to_string()))
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
