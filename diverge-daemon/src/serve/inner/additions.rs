//! The FUSE mounts a result adds to a record's lists, when that is
//! all it does to them.

use diverge_sdk::daemon::endpoints::agents::create::client::request::FuseMount;

/// The FUSE mounts the result appends to the record's two lists, when
/// each result list is the record's list followed by zero or more
/// entries — the files, then the directories. `None` when either
/// list is anything else: a mount removed, replaced or reordered,
/// which a running container cannot take.
pub fn additions(current_files: &[FuseMount], result_files: &[FuseMount], current_directories: &[FuseMount], result_directories: &[FuseMount]) -> Option<(Vec<FuseMount>, Vec<FuseMount>)> {
    Some((appended(current_files, result_files)?, appended(current_directories, result_directories)?))
}

/// What `result` has after `current`, when it begins with `current`.
fn appended(current: &[FuseMount], result: &[FuseMount]) -> Option<Vec<FuseMount>> {
    result.starts_with(current).then(|| result[current.len()..].to_vec())
}
