//! The walk: how many bytes a volume holds, and the hash of what it
//! holds.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use futures_util::future;
use sha2::{Digest as _, Sha256};
use tokio::fs::{self, DirEntry};
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

/// One file, as the walk found it: its manifest line, and its size.
struct Line {
    text: Vec<u8>,
    size: u64,
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
/// Everything independent runs at once: every entry of a directory
/// beside every other, every subdirectory's walk beside its siblings',
/// every file's hash beside every other's, at every level. The
/// manifest's order is the sort's, never the walk's. The concurrency
/// is unbounded: a volume of very many files opens very many files at
/// once. Every read is `tokio::fs`'s.
pub async fn walk(root: &Path) -> io::Result<Walked> {
    let mut lines = walk_dir(root.to_path_buf(), Vec::new()).await?;
    lines.sort_by(|a, b| a.text.cmp(&b.text));
    let bytes_used = lines.iter().map(|line| line.size).sum();
    let mut manifest = Sha256::new();
    for line in &lines {
        manifest.update(&line.text);
    }
    Ok(Walked {
        bytes_used,
        dirhash: URL_SAFE_NO_PAD.encode(manifest.finalize()),
    })
}

/// One directory: its entries are read, and every one is walked at
/// once. Boxed, because the walk recurses through itself.
fn walk_dir(dir: PathBuf, components: Vec<String>) -> Pin<Box<dyn Future<Output = io::Result<Vec<Line>>> + Send>> {
    Box::pin(async move {
        let mut entries = fs::read_dir(&dir).await?;
        let mut found = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            found.push(entry);
        }
        let walked = future::try_join_all(found.into_iter().map(|entry| walk_entry(entry, &components))).await?;
        Ok(walked.into_iter().flatten().collect())
    })
}

/// One entry: a directory is walked, a file is hashed, and anything
/// else — a symbolic link above all — is nothing.
async fn walk_entry(entry: DirEntry, components: &[String]) -> io::Result<Vec<Line>> {
    let path = entry.path();
    let kind = fs::symlink_metadata(&path).await?.file_type();
    let mut below = components.to_vec();
    below.push(entry.file_name().to_string_lossy().into_owned());
    if kind.is_dir() {
        walk_dir(path, below).await
    } else if kind.is_file() {
        Ok(vec![hash_file(&path, below).await?])
    } else {
        Ok(Vec::new())
    }
}

/// One file: its manifest line — the SHA-256 of its bytes, base64url
/// without padding, its length, and its `/`-joined path — and its
/// size, read in chunks so a large file is never held whole.
async fn hash_file(path: &Path, components: Vec<String>) -> io::Result<Line> {
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
    let mut text = Vec::new();
    text.extend_from_slice(URL_SAFE_NO_PAD.encode(hasher.finalize()).as_bytes());
    text.push(b' ');
    text.extend_from_slice(size.to_string().as_bytes());
    text.push(b' ');
    text.extend_from_slice(components.join("/").as_bytes());
    text.push(b'\n');
    Ok(Line { text, size })
}
