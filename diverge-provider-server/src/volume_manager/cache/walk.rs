//! The walk: how many bytes a volume holds, and the hash of what it
//! holds.

use std::io;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest as _, Sha256};
use tokio::fs;
use tokio::io::AsyncReadExt as _;

/// What a walk of a volume found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walked {
    /// The sum of the sizes of every file in the volume, in bytes.
    /// Directories and symbolic links add nothing.
    pub bytes_used: u64,
    /// The hash of the volume's content, as the specification defines
    /// it — see [`walk`].
    pub dirhash: String,
}

/// How much of a file is read at a time while it is hashed.
const CHUNK: usize = 64 * 1024;

/// Walk the volume at `root` and hash its content.
///
/// The hash is the specification's `dirhash`: the SHA-256, written
/// base64url without padding, of the volume's manifest. The manifest
/// has one line per regular file, `<hash> <size> <path>`: `<hash>` the
/// SHA-256 of the file's bytes, base64url without padding; `<size>`
/// its length in bytes, decimal; `<path>` the file's path relative to
/// `root`, its components joined with `/`, whatever the host's own
/// separator. The lines are sorted bytewise, and each is terminated
/// with `\n`, so a volume with no file has the empty manifest, whose
/// hash is `47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU`. The same
/// manifest, with the total size in front, is what a directory's
/// identity mount is verified against, and a verifier writes it the
/// same way.
///
/// A symbolic link is never followed and is not a file; a directory
/// contributes its entries and nothing of its own. A directory that
/// cannot be read, or a file that cannot be opened or read, is the
/// walk's error: a hash of half a tree is a wrong answer, not a
/// partial one.
///
/// Every read is `tokio::fs`'s, so the walk holds no thread of the
/// runtime while the disk answers.
pub async fn walk(root: &Path) -> io::Result<Walked> {
    let mut lines: Vec<Vec<u8>> = Vec::new();
    let mut bytes_used = 0u64;
    let mut pending: Vec<(PathBuf, Vec<String>)> = vec![(root.to_path_buf(), Vec::new())];
    while let Some((dir, components)) = pending.pop() {
        let mut entries = fs::read_dir(&dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let kind = fs::symlink_metadata(&path).await?.file_type();
            let mut below = components.clone();
            below.push(entry.file_name().to_string_lossy().into_owned());
            if kind.is_dir() {
                pending.push((path, below));
            } else if kind.is_file() {
                let (hash, size) = hash_file(&path).await?;
                bytes_used += size;
                let mut line = Vec::new();
                line.extend_from_slice(hash.as_bytes());
                line.push(b' ');
                line.extend_from_slice(size.to_string().as_bytes());
                line.push(b' ');
                line.extend_from_slice(below.join("/").as_bytes());
                line.push(b'\n');
                lines.push(line);
            }
        }
    }
    lines.sort();
    let mut manifest = Sha256::new();
    for line in &lines {
        manifest.update(line);
    }
    Ok(Walked {
        bytes_used,
        dirhash: URL_SAFE_NO_PAD.encode(manifest.finalize()),
    })
}

/// One file's SHA-256, base64url without padding, and its length in
/// bytes, read in chunks so a large file is never held whole.
async fn hash_file(path: &Path) -> io::Result<(String, u64)> {
    let mut file = fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut size = 0u64;
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        size += n as u64;
    }
    Ok((URL_SAFE_NO_PAD.encode(hasher.finalize()), size))
}
