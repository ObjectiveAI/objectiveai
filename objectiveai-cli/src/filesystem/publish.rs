use std::path::{Path, PathBuf};

use super::{Client, Error};

/// The kind of resource being published.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Agents,
    Swarms,
    Functions,
    Profiles,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Agents => "agents",
            Kind::Swarms => "swarms",
            Kind::Functions => "functions",
            Kind::Profiles => "profiles",
        }
    }

    /// The filename used for this kind's primary JSON file.
    fn filename(self) -> &'static str {
        match self {
            Kind::Agents => "agent.json",
            Kind::Swarms => "swarm.json",
            Kind::Functions => "function.json",
            Kind::Profiles => "profile.json",
        }
    }
}

fn validate_repository_name(name: &str) -> Result<(), Error> {
    if name.is_empty() || name.len() > 100 {
        return Err(Error::InvalidRepositoryName(name.to_string()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(Error::InvalidRepositoryName(name.to_string()));
    }
    Ok(())
}

fn repo_path(client: &Client, kind: Kind, repository: &str) -> PathBuf {
    client
        .state_dir()
        .join(kind.as_str())
        .join(&client.commit_author_name)
        .join(repository)
}

/// Publishes an agent to the local filesystem git repository.
pub async fn publish_agent(
    client: &Client,
    repository: &str,
    agent: &objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks,
    message: &str,
    overwrite: bool,
) -> Result<String, Error> {
    let content =
        serde_json::to_string_pretty(agent).map_err(Error::Serialize)?;
    publish(
        client,
        Kind::Agents,
        repository,
        &content,
        message,
        overwrite,
    )
    .await
}

/// Publishes a swarm to the local filesystem git repository.
pub async fn publish_swarm(
    client: &Client,
    repository: &str,
    swarm: &objectiveai_sdk::swarm::RemoteSwarmBase,
    message: &str,
    overwrite: bool,
) -> Result<String, Error> {
    let content =
        serde_json::to_string_pretty(swarm).map_err(Error::Serialize)?;
    publish(
        client,
        Kind::Swarms,
        repository,
        &content,
        message,
        overwrite,
    )
    .await
}

/// Publishes a function to the local filesystem git repository.
pub async fn publish_function(
    client: &Client,
    repository: &str,
    function: &objectiveai_sdk::functions::FullRemoteFunction,
    message: &str,
    overwrite: bool,
) -> Result<String, Error> {
    let content =
        serde_json::to_string_pretty(function).map_err(Error::Serialize)?;
    publish(
        client,
        Kind::Functions,
        repository,
        &content,
        message,
        overwrite,
    )
    .await
}

/// Publishes a profile to the local filesystem git repository.
pub async fn publish_profile(
    client: &Client,
    repository: &str,
    profile: &objectiveai_sdk::functions::RemoteProfile,
    message: &str,
    overwrite: bool,
) -> Result<String, Error> {
    let content =
        serde_json::to_string_pretty(profile).map_err(Error::Serialize)?;
    publish(
        client,
        Kind::Profiles,
        repository,
        &content,
        message,
        overwrite,
    )
    .await
}

/// Core publish: writes content to a git repository, commits, returns the commit SHA.
///
/// If the repository already exists and the file already exists, `overwrite`
/// must be true or an error is returned.
async fn publish(
    client: &Client,
    kind: Kind,
    repository: &str,
    content: &str,
    message: &str,
    overwrite: bool,
) -> Result<String, Error> {
    validate_repository_name(repository)?;

    let path = repo_path(client, kind, repository);
    let filename = kind.filename();

    tokio::fs::create_dir_all(&path).await?;

    // Check overwrite guard before doing git work.
    if !overwrite && tokio::fs::metadata(path.join(filename)).await.is_ok() {
        return Err(Error::Write(
            path.join(filename),
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "file already exists (use overwrite to replace)",
            ),
        ));
    }

    // Write file.
    tokio::fs::write(path.join(filename), content.as_bytes()).await?;

    // Git operations are synchronous — run on a blocking thread.
    let commit_author_name = client.commit_author_name.clone();
    let commit_author_email = client.commit_author_email.clone();
    let message = message.to_string();
    let filename = filename.to_string();

    tokio::task::spawn_blocking(move || {
        git_commit(
            &path,
            &filename,
            &commit_author_name,
            &commit_author_email,
            &message,
        )
    })
    .await
    .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
}

/// Opens or initializes the git repo, stages the file, and commits.
fn git_commit(
    repo_path: &Path,
    filename: &str,
    author_name: &str,
    author_email: &str,
    message: &str,
) -> Result<String, Error> {
    // Open or initialize the git repository.
    let repo = match git2::Repository::open(repo_path) {
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
        Err(_) => git2::Repository::init(repo_path)?,
    };

    // Stage.
    let mut index = repo.index()?;
    index.add_path(Path::new(filename))?;
    index.write()?;
    let tree_oid = index.write_tree()?;

    // If the tree is identical to the parent's, skip the commit.
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    if let Some(ref parent) = parent {
        if parent.tree_id() == tree_oid {
            return Ok(parent.id().to_string());
        }
    }

    let tree = repo.find_tree(tree_oid)?;

    // Commit.
    let sig = git2::Signature::now(author_name, author_email)?;
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let commit_oid =
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

    Ok(commit_oid.to_string())
}
