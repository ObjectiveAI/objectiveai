//! Git checkouts for plugin-image builds: fetch exactly ONE tag of a
//! plugin repo into a throwaway dir under `<bin>/temp`.
//!
//! The source is LOCAL-FIRST: if `<objectiveai_dir>/plugins/<owner>/<name>`
//! holds a git repo containing the tag, the fetch pulls from that repo
//! (how local plugin installation works — no push to GitHub needed);
//! otherwise it pulls from `https://github.com/{owner}/{name}.git`.
//! The owner/name directory segments match case-INSENSITIVELY (coords
//! arrive lowercased, the on-disk dirs may keep the repo's original
//! casing) while the tag stays exact-case everywhere.
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

/// Shallow-fetch `refs/tags/{tag}` of the plugin repo — the local
/// `<objectiveai_dir>/plugins/<owner>/<name>` repo when it holds the
/// tag, `https://github.com/{owner}/{name}.git` otherwise — into a
/// fresh `<bin>/temp/<uuid>` dir and check it out detached.
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
        // `<objectiveai_dir>/plugins` — bin_dir is always
        // `<objectiveai_dir>/bin` (main.rs derives it that way).
        let plugins_dir = bin_dir.parent().map(|dir| dir.join("plugins"));
        let refspec = format!("+refs/tags/{tag}:refs/tags/{tag}");
        let reference = format!("refs/tags/{tag}");
        let owner = owner.to_string();
        let name = name.to_string();
        let tag = tag.to_string();
        tokio::task::spawn_blocking(move || {
            let source = resolve_source(plugins_dir.as_deref(), &owner, &name, &tag);
            fetch_at_tag_blocking(&dir, &source, &refspec, &reference, &owner, &name, &tag)
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

/// A resolved fetch source. The distinction matters at fetch time:
/// libgit2's LOCAL transport does not support shallow fetch (it
/// errors `shallow fetch is not supported by the local transport`),
/// so local fetches go full-depth — cheap, the repo is on-disk.
enum Source {
    /// The local `<plugins>/<owner>/<name>` override repo.
    Local(String),
    /// `https://github.com/{owner}/{name}.git`.
    Remote(String),
}

impl Source {
    fn url(&self) -> &str {
        match self {
            Source::Local(url) | Source::Remote(url) => url,
        }
    }
}

/// Pick the fetch source: the local `<plugins>/<owner>/<name>` repo
/// when it exists (case-insensitive segments) AND contains the tag
/// (exact-case); the GitHub URL otherwise. Once the local source is
/// chosen a fetch failure is a hard error — a present tag is a
/// deliberate local override, never silently skipped.
fn resolve_source(plugins_dir: Option<&Path>, owner: &str, name: &str, tag: &str) -> Source {
    if let Some(plugins_dir) = plugins_dir
        && let Some(local) = local_plugin_dir(plugins_dir, owner, name)
        && local_has_tag(&local, tag)
    {
        // libgit2's local transport takes a plain path on every OS.
        // Backslashes are path separators ONLY on Windows — on
        // Linux/macOS they are legal filename bytes and must pass
        // through untouched.
        let local = local.to_string_lossy();
        return Source::Local(if cfg!(windows) {
            local.replace('\\', "/")
        } else {
            local.into_owned()
        });
    }
    Source::Remote(format!("https://github.com/{owner}/{name}.git"))
}

/// `<plugins>/<owner>/<name>` with BOTH segments resolved
/// case-insensitively against the actual directory entries.
fn local_plugin_dir(plugins_dir: &Path, owner: &str, name: &str) -> Option<PathBuf> {
    let owner_dir = resolve_segment_ci(plugins_dir, owner)?;
    resolve_segment_ci(&owner_dir, name)
}

/// Resolve one child of `parent` by name, case-insensitively: an
/// exact-case match wins; otherwise the first `eq_ignore_ascii_case`
/// entry. `None` on an unreadable parent or no match.
fn resolve_segment_ci(parent: &Path, target: &str) -> Option<PathBuf> {
    let entries: Vec<PathBuf> = std::fs::read_dir(parent)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    let matches_target = |path: &&PathBuf, exact: bool| {
        path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            if exact {
                n == target
            } else {
                n.eq_ignore_ascii_case(target)
            }
        })
    };
    entries
        .iter()
        .find(|path| matches_target(path, true))
        .or_else(|| entries.iter().find(|path| matches_target(path, false)))
        .cloned()
}

/// Whether `dir` is a git repo containing `refs/tags/{tag}`
/// (exact-case — the tag IS the version). False on not-a-repo or
/// missing tag; both mean "fall back to GitHub".
fn local_has_tag(dir: &Path, tag: &str) -> bool {
    git2::Repository::open(dir)
        .and_then(|repo| repo.refname_to_id(&format!("refs/tags/{tag}")))
        .is_ok()
}

fn fetch_at_tag_blocking(
    dir: &Path,
    source: &Source,
    refspec: &str,
    reference: &str,
    owner: &str,
    name: &str,
    tag: &str,
) -> Result<String, String> {
    let url = source.url();
    let repo = git2::Repository::init(dir).map_err(|e| format!("git init: {e}"))?;
    let mut remote = repo
        .remote_anonymous(url)
        .map_err(|e| format!("git remote: {e}"))?;
    let mut options = git2::FetchOptions::new();
    // Shallow ONLY over the network: libgit2's local transport
    // rejects depth ("shallow fetch is not supported by the local
    // transport"), and a full-depth fetch from an on-disk repo is
    // cheap anyway.
    if matches!(source, Source::Remote(_)) {
        options.depth(1);
    }
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
