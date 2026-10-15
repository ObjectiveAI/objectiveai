//! The paths a request names, checked.

use std::collections::HashSet;
use std::path::PathBuf;

use super::Error;

/// Whether `component` is a name: not empty, not `.` or `..`, and
/// holding no `/` and no NUL.
pub fn is_name(component: &str) -> bool {
    !component.is_empty() && component != "." && component != ".." && !component.contains('/') && !component.contains('\0')
}

/// The components of a `/`-separated path an upload names, checked:
/// at least one, and every one a name.
pub fn components(path: &str) -> Result<Vec<String>, Error> {
    let components: Vec<String> = path.split('/').map(str::to_string).collect();
    if components.is_empty() || !components.iter().all(|component| is_name(component)) {
        return Err(Error::Path(path.to_string()));
    }
    Ok(components)
}

/// Every path a directory upload names, checked together: at least
/// one, none twice, and none under another — a file cannot be a
/// directory too.
pub fn validate(paths: &[String]) -> Result<Vec<Vec<String>>, Error> {
    if paths.is_empty() {
        return Err(Error::Path(String::new()));
    }
    let mut seen: HashSet<&str> = HashSet::new();
    let mut all = Vec::with_capacity(paths.len());
    for path in paths {
        if !seen.insert(path) {
            return Err(Error::Path(path.clone()));
        }
        all.push(components(path)?);
    }
    for (i, one) in all.iter().enumerate() {
        for (j, other) in all.iter().enumerate() {
            if i != j && other.len() > one.len() && other[..one.len()] == one[..] {
                return Err(Error::Path(paths[i].clone()));
            }
        }
    }
    Ok(all)
}

/// The components a download or a filetree names inside what it
/// reads, checked: each a name. Empty is the root itself.
pub fn inside(path: &[String]) -> Result<(), Error> {
    match path.iter().find(|component| !is_name(component)) {
        Some(bad) => Err(Error::Path(bad.clone())),
        None => Ok(()),
    }
}

/// The components joined onto `root`.
pub fn join(root: &std::path::Path, components: &[String]) -> PathBuf {
    let mut path = root.to_path_buf();
    for component in components {
        path.push(component);
    }
    path
}
