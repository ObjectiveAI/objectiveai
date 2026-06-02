//! Shared `--source all` merge helpers for the four resource-list
//! leaves (`agents list available`, `swarms list`, `functions list`,
//! `functions profiles list`).
//!
//! Mirrors the dedup logic from the legacy `crate::list::all`:
//! favorites come first, filesystem items skip anything already
//! covered by a favorite, and ObjectiveAI items skip anything covered
//! by a favorite *or* an already-emitted filesystem item.

use objectiveai_sdk::{RemotePath, RemotePathCommitOptional};

/// Returns true if a favorite's `RemotePathCommitOptional` matches a
/// concrete `RemotePath`. Same remote variant required on both sides;
/// `owner`/`repository`/`name` must match exactly; a favorite with
/// `commit: None` matches any commit on the same repo.
pub fn favorite_matches_path(
    fav_path: &RemotePathCommitOptional,
    path: &RemotePath,
) -> bool {
    match (fav_path, path) {
        (
            RemotePathCommitOptional::Github {
                owner: fo,
                repository: fr,
                commit: fc,
            },
            RemotePath::Github {
                owner: po,
                repository: pr,
                commit: pc,
            },
        ) => fo == po && fr == pr && fc.as_ref().is_none_or(|c| c == pc),
        (
            RemotePathCommitOptional::Filesystem {
                owner: fo,
                repository: fr,
                commit: fc,
            },
            RemotePath::Filesystem {
                owner: po,
                repository: pr,
                commit: pc,
            },
        ) => fo == po && fr == pr && fc.as_ref().is_none_or(|c| c == pc),
        (
            RemotePathCommitOptional::Mock { name: fn_ },
            RemotePath::Mock { name: pn },
        ) => fn_ == pn,
        _ => false,
    }
}
