//! The root, and paths as components of it.

use std::io;
use std::path::{Component, Path, PathBuf};

/// The root as the host spells it in full — what the watcher reports
/// its paths under, so they strip against it exactly: `\\?\C:\…` on
/// Windows, `/private/tmp/…` where `/tmp` is a link on macOS, a link
/// anywhere followed to what it names.
pub async fn canonical(root: &Path) -> io::Result<PathBuf> {
    tokio::fs::canonicalize(root).await
}

/// `path` as components from `root`: `None` for the root itself and
/// for a path outside it. Only names are kept; what a platform puts
/// before them — a drive, a `\\?\`, a leading separator — is not a
/// component of anything.
pub fn components(root: &Path, path: &Path) -> Option<Vec<String>> {
    let relative = path.strip_prefix(root).ok()?;
    let components: Vec<String> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    if components.is_empty() { None } else { Some(components) }
}

/// A link's target as components from `root`, resolved LEXICALLY:
/// an absolute target is its own components after `root` where it
/// lies under it, and its own names where it does not; a relative one
/// is walked from the link's parent, `.` dropped and `..` stepping
/// up, never above the root. The target is never touched, so a
/// dangling link resolves like any other.
pub fn target(root: &Path, link: &Path, target: &Path) -> Vec<String> {
    let mut components = Vec::new();
    if target.is_absolute() {
        push(&mut components, target.strip_prefix(root).unwrap_or(target));
    } else {
        if let Some(parent) = link.parent().and_then(|parent| parent.strip_prefix(root).ok()) {
            push(&mut components, parent);
        }
        push(&mut components, target);
    }
    components
}

/// Walk `path`'s components onto `components`, lexically.
fn push(components: &mut Vec<String>, path: &Path) {
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                components.push(name.to_string_lossy().into_owned());
            }
            Component::ParentDir => {
                components.pop();
            }
            Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        }
    }
}
