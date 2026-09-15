//! The walk: how many bytes a volume holds, and the hash of what it
//! holds.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use futures_util::future;
use sha2::{Digest as _, Sha256};
use tokio::fs::{self, DirEntry};
use tokio::io::AsyncReadExt as _;

/// What a walk of a volume found.
///
/// A volume with no file: `bytes_used` of `0`, and the `dirhash`
/// `h1:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Walked {
    /// The sum of the lengths of every file the hash covers, in bytes.
    /// A symbolic link counts the length of what it opens to; a
    /// directory adds nothing.
    pub bytes_used: u64,
    /// The hash of the volume's content, as the specification defines
    /// it — see [`walk_directory`].
    pub dirhash: String,
}

/// One file, as a walk found it: its name in the hash, the SHA-256
/// of its bytes as lowercase hexadecimal, and its length. What both
/// walks — of a directory here, of an image in
/// [`image`](super::image) — produce, and what [`digest`] takes.
pub(super) struct Line {
    pub name: String,
    pub hex: String,
    pub size: u64,
}

/// How much of a file is read at a time while it is hashed.
const CHUNK: usize = 64 * 1024;

/// Walk the fixed volume at `root` and hash its content.
///
/// The hash is Go's `h1:` directory hash: the string
/// [`golang.org/x/mod/sumdb/dirhash`](https://pkg.go.dev/golang.org/x/mod/sumdb/dirhash)
/// returns from `HashDir(root, "", Hash1)`, as that package defines
/// it, which is this. The files are every entry beneath `root` that
/// is not a directory as `lstat` reports it, each named by its path
/// relative to `root` with its components joined by `/`, whatever the
/// host's own separator; a symbolic link is a file, opened through
/// the link. The names are sorted bytewise. A name containing a
/// newline is an error. For each file, in that order, one line is
/// written into one SHA-256: the SHA-256 of the file's bytes as
/// lowercase hexadecimal, two spaces, the name, and a newline. The
/// hash is the string `h1:` followed by that SHA-256 encoded as
/// standard base64 with padding. A volume with no file has the hash
/// `h1:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=`. A Go program
/// running `HashDir` over the same directory produces the same
/// string, byte for byte.
///
/// A directory that cannot be read, a file that cannot be opened or
/// read — a dangling link, a link to a directory — and a name with a
/// newline are the walk's error: a hash of half a tree is a wrong
/// answer, not a partial one.
///
/// Everything independent runs at once: every entry of a directory
/// beside every other, every subdirectory's walk beside its siblings',
/// every file's hash beside every other's, at every level. The
/// order of the lines is the sort's, never the walk's. The concurrency
/// is unbounded: a volume of very many files opens very many files at
/// once. Every read is `tokio::fs`'s.
pub async fn walk_directory(root: &Path) -> io::Result<Walked> {
    Ok(digest(walk_dir(root.to_path_buf(), Vec::new()).await?))
}

/// The lines of a walk, in whatever order, made into what it found:
/// sorted bytewise by name, summed, and hashed as [`walk_directory`]
/// states.
pub(super) fn digest(mut lines: Vec<Line>) -> Walked {
    lines.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    let bytes_used = lines.iter().map(|line| line.size).sum();
    let mut digest = Sha256::new();
    for line in &lines {
        digest.update(line.hex.as_bytes());
        digest.update(b"  ");
        digest.update(line.name.as_bytes());
        digest.update(b"\n");
    }
    Walked {
        bytes_used,
        dirhash: format!("h1:{}", STANDARD.encode(digest.finalize())),
    }
}

/// The error for a name with a newline in it, as Go words it.
pub(super) fn newline(name: &str) -> io::Error {
    io::Error::other(format!("dirhash: filenames with newlines are not supported: {name:?}"))
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

/// One entry: a directory is walked, and anything else — a regular
/// file, or a symbolic link opened through — is hashed as a file.
async fn walk_entry(entry: DirEntry, components: &[String]) -> io::Result<Vec<Line>> {
    let path = entry.path();
    let mut below = components.to_vec();
    below.push(entry.file_name().to_string_lossy().into_owned());
    if fs::symlink_metadata(&path).await?.file_type().is_dir() {
        walk_dir(path, below).await
    } else {
        Ok(vec![hash_file(&path, below.join("/")).await?])
    }
}

/// One file: the SHA-256 of its bytes as lowercase hexadecimal, its
/// name, and its length, read in chunks so a large file is never held
/// whole. A name containing a newline is refused, as Go refuses it.
async fn hash_file(path: &Path, name: String) -> io::Result<Line> {
    if name.contains('\n') {
        return Err(newline(&name));
    }
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
    Ok(Line {
        name,
        hex: hex::encode(hasher.finalize()),
        size,
    })
}
