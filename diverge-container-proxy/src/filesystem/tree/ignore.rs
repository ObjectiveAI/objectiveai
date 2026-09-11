//! What the tree does not contain.

use std::path::{Path, PathBuf};

/// The paths that do not exist as far as the stream is concerned:
/// never walked, never watched, and an event under one is dropped.
///
/// Three are the proxy's own — `/proc`, `/sys` and `/dev`, the
/// pseudo-filesystems, which churn, hold nothing a caller wants, and
/// break a recursive watch — and the rest are the server's: the
/// mounts it placed, named in the request that opens the watch,
/// content the caller already holds or serves, which a watch of
/// would cost the walk and yield nothing the caller wants.
pub struct Ignore {
    paths: Vec<PathBuf>,
}

impl Ignore {
    /// The three, plus the server's. An empty path in the server's
    /// list would name the root and is dropped.
    pub fn new(ignore: Vec<Vec<String>>) -> Self {
        let mut paths: Vec<PathBuf> =
            ["/proc", "/sys", "/dev"].iter().map(PathBuf::from).collect();
        paths.extend(
            ignore
                .into_iter()
                .filter(|components| !components.is_empty())
                .map(|components| {
                    let mut path = PathBuf::from("/");
                    path.extend(components);
                    path
                }),
        );
        Self { paths }
    }

    /// Whether `path` is an ignored path or lies under one.
    pub fn excluded(&self, path: &Path) -> bool {
        self.paths.iter().any(|ignored| path.starts_with(ignored))
    }

    /// Whether an ignored path lies under `dir` — the cheap pre-check
    /// that decides whether a subtree is safe for an indiscriminate
    /// recursive watch.
    pub fn contains_excluded(&self, dir: &Path) -> bool {
        self.paths.iter().any(|ignored| ignored.starts_with(dir))
    }
}
