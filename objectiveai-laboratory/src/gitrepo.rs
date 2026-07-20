//! Git checkouts for plugin-image builds: fetch exactly ONE tag of a
//! GitHub repo into a throwaway dir under `<bin>/temp`.
//!
//! Not a general clone: `Repository::init` + an anonymous remote + a
//! single-refspec shallow fetch (`+refs/tags/{tag}:refs/tags/{tag}`,
//! depth 1, no auto-tags) pulls the tag's tree and nothing else — a
//! `RepoBuilder` clone can't shallow-fetch a non-default-branch tag.
//! A MISSING TAG IS A HARD ERROR: the version names the tag
//! (Go-modules convention, `v`-prefixed, case-SENSITIVE); there is no
//! HEAD fallback.
//!
//! Checkouts are transient by contract: [`remove_checkout`] deletes
//! them after the build (success AND failure), and the boot sweep
//! clears any leftovers under `<bin>/temp` from a hard-killed
//! predecessor.

use std::path::{Path, PathBuf};

/// A tag's tree on disk, plus the commit the tag peeled to.
pub struct CheckedOutRepo {
    /// The checkout root — a fresh `<bin>/temp/<uuid>` dir.
    pub dir: PathBuf,
    /// The tag's commit SHA (hex) — stamped onto the built image.
    pub commit_sha: String,
}

/// `<bin>/temp` — where checkouts live.
pub fn temp_dir(bin_dir: &Path) -> PathBuf {
    bin_dir.join("temp")
}

/// Shallow-fetch `refs/tags/{tag}` of `https://github.com/{owner}/{name}.git`
/// into a fresh `<bin>/temp/<uuid>` dir and check it out detached.
/// The caller owns the returned dir and MUST [`remove_checkout`] it
/// when done — this function only cleans up after its own failures.
///
/// All libgit2 work runs on the blocking pool (libgit2 is synchronous;
/// the daemon's filesystem git code takes the same shape).
pub async fn fetch_at_tag(
    bin_dir: &Path,
    owner: &str,
    name: &str,
    tag: &str,
) -> Result<CheckedOutRepo, String> {
    let dir = temp_dir(bin_dir).join(uuid::Uuid::new_v4().to_string());
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("create checkout dir: {e}"))?;
    let result = {
        let dir = dir.clone();
        let url = format!("https://github.com/{owner}/{name}.git");
        let refspec = format!("+refs/tags/{tag}:refs/tags/{tag}");
        let reference = format!("refs/tags/{tag}");
        let owner = owner.to_string();
        let name = name.to_string();
        let tag = tag.to_string();
        tokio::task::spawn_blocking(move || {
            fetch_at_tag_blocking(&dir, &url, &refspec, &reference, &owner, &name, &tag)
        })
        .await
        .map_err(|e| format!("fetch task panicked: {e}"))?
    };
    match result {
        Ok(commit_sha) => Ok(CheckedOutRepo { dir, commit_sha }),
        Err(message) => {
            remove_checkout(&dir).await;
            Err(message)
        }
    }
}

fn fetch_at_tag_blocking(
    dir: &Path,
    url: &str,
    refspec: &str,
    reference: &str,
    owner: &str,
    name: &str,
    tag: &str,
) -> Result<String, String> {
    let repo = git2::Repository::init(dir).map_err(|e| format!("git init: {e}"))?;
    let mut remote = repo
        .remote_anonymous(url)
        .map_err(|e| format!("git remote: {e}"))?;
    let mut options = git2::FetchOptions::new();
    options.depth(1);
    options.download_tags(git2::AutotagOption::None);
    remote
        .fetch(&[refspec], Some(&mut options), None)
        .map_err(|e| {
            // libgit2 reports a refspec that matched nothing with an
            // error naming the ref — that IS the missing-tag case.
            let message = e.message().to_ascii_lowercase();
            if message.contains("couldn't find remote ref")
                || message.contains("could not find remote ref")
                || message.contains(&format!("refs/tags/{tag}").to_ascii_lowercase())
            {
                format!("plugin tag '{tag}' not found in {owner}/{name}")
            } else if matches!(
                e.class(),
                git2::ErrorClass::Net | git2::ErrorClass::Http
            ) {
                format!("fetch {url}: {e}")
            } else {
                format!("git fetch {url}: {e}")
            }
        })?;
    let commit = repo
        .find_reference(reference)
        .map_err(|_| format!("plugin tag '{tag}' not found in {owner}/{name}"))?
        .peel_to_commit()
        .map_err(|e| format!("peel tag '{tag}': {e}"))?;
    let commit_sha = commit.id().to_string();
    repo.set_head_detached(commit.id())
        .map_err(|e| format!("detach head at '{tag}': {e}"))?;
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    repo.checkout_head(Some(&mut checkout))
        .map_err(|e| format!("checkout '{tag}': {e}"))?;
    Ok(commit_sha)
}

/// Delete a checkout dir, best-effort — errors go to stderr and never
/// propagate (the dir is transient scratch; a leftover is re-swept at
/// the next boot). Windows needs the readonly bits cleared first: git
/// writes its pack files read-only, and `remove_dir_all` fails on
/// them with ACCESS_DENIED.
pub async fn remove_checkout(dir: &Path) {
    let dir = dir.to_path_buf();
    let result = tokio::task::spawn_blocking(move || {
        clear_readonly(&dir);
        std::fs::remove_dir_all(&dir)
    })
    .await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
        Ok(Err(e)) => eprintln!("remove checkout: {e}"),
        Err(e) => eprintln!("remove checkout task panicked: {e}"),
    }
}

/// Recursively clear the readonly bit — a no-op on entries we cannot
/// touch (the subsequent delete reports those).
fn clear_readonly(path: &Path) {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        let mut permissions = metadata.permissions();
        if permissions.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = std::fs::set_permissions(path, permissions);
        }
        if metadata.is_dir()
            && let Ok(entries) = std::fs::read_dir(path)
        {
            for entry in entries.flatten() {
                clear_readonly(&entry.path());
            }
        }
    }
}

/// Boot-sweep pass: delete EVERY leftover under `<bin>/temp` — a
/// hard-killed predecessor's checkouts. Best-effort by design; runs
/// before any daemon channel serves, and new checkouts mint fresh
/// uuid dirs, so nothing races it.
pub async fn sweep_temp(bin_dir: &Path) {
    let temp = temp_dir(bin_dir);
    let Ok(mut entries) = tokio::fs::read_dir(&temp).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        remove_checkout(&entry.path()).await;
    }
}
