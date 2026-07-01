pub fn vec_is_none_or_empty<T>(opt: &Option<Vec<T>>) -> bool {
    opt.as_ref().is_none_or(|vec| vec.is_empty())
}

/// Atomic-replace file write. The bytes land in a sibling temp file
/// under a random name first, then a single `rename` moves it over
/// `path` — a process killed mid-write can never leave a truncated
/// file at `path`, only an orphan temp file the next writer/OS reaps.
///
/// The temp file is staged in `path`'s own parent directory (not the
/// system temp dir) so the rename is always same-volume: a cross-volume
/// rename fails loudly (Windows `os error 17`) rather than being atomic,
/// and a repo on one drive with `%TEMP%` on another is a routine layout.
/// Callers ensure the parent exists before calling. Only when `path` has
/// no parent do we fall back to the system temp dir.
pub async fn write_atomic(
    path: &std::path::Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    use rand::RngCore as _;
    let mut suffix = [0u8; 8];
    rand::rng().fill_bytes(&mut suffix);
    let name =
        format!("objectiveai-write-{:016x}.tmp", u64::from_le_bytes(suffix));
    let tmp = match path.parent().filter(|p| !p.as_os_str().is_empty()) {
        Some(parent) => parent.join(name),
        None => std::env::temp_dir().join(name),
    };
    tokio::fs::write(&tmp, bytes).await?;
    match tokio::fs::rename(&tmp, path).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            Err(e)
        }
    }
}
