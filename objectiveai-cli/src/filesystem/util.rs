pub fn vec_is_none_or_empty<T>(opt: &Option<Vec<T>>) -> bool {
    opt.as_ref().is_none_or(|vec| vec.is_empty())
}

/// Atomic-replace file write. The bytes land in a system-temp file
/// under a random name first, then a single `rename` moves it over
/// `path` — a process killed mid-write can never leave a truncated
/// file at `path`, only an orphan temp file the OS temp cleaner
/// reaps. (The rename is atomic only when the temp dir and `path`
/// share a volume; a cross-volume layout fails the rename loudly
/// rather than tearing the target.)
pub async fn write_atomic(
    path: &std::path::Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    use rand::RngCore as _;
    let mut suffix = [0u8; 8];
    rand::rng().fill_bytes(&mut suffix);
    let tmp = std::env::temp_dir().join(format!(
        "objectiveai-write-{:016x}.tmp",
        u64::from_le_bytes(suffix),
    ));
    tokio::fs::write(&tmp, bytes).await?;
    match tokio::fs::rename(&tmp, path).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            Err(e)
        }
    }
}
