//! The ids that are hashes: a template's, a file's, a directory's.

use base64::Engine as _;
use serde::Serialize;
use sha2::{Digest as _, Sha256};

/// A template's id: the lowercase hex SHA-256 of its compact JSON —
/// members in the order the type declares them, `type` first, absent
/// members omitted, no whitespace — which is what `serde_json` writes
/// for it, so a caller that hashes the same bytes gets the same id.
pub fn template_id<T: Serialize>(template: &T) -> Result<String, serde_json::Error> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(template)?)))
}

/// A file resource's id: the lowercase hex SHA-256 of its bytes, as a
/// digest already computed over them.
pub fn file_id(digest: &[u8]) -> String {
    hex::encode(digest)
}

/// A directory resource's id: `h1:` and the standard base64 of the
/// SHA-256 of the directory's summary — its files sorted bytewise by
/// path, one line each of the file's hex hash, two spaces, the path,
/// a newline — which is what Go's `dirhash` `Hash1` writes.
pub fn directory_id(files: &[(String, String)]) -> String {
    let mut sorted: Vec<&(String, String)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut summary = String::new();
    for (path, hex) in sorted {
        summary.push_str(hex);
        summary.push_str("  ");
        summary.push_str(path);
        summary.push('\n');
    }
    format!("h1:{}", base64::engine::general_purpose::STANDARD.encode(Sha256::digest(summary.as_bytes())))
}
