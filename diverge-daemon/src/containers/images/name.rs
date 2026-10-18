//! The repository path an image is answered under.

/// The repository path the daemon holds an image under, as far as a
/// provider is told: the digest with its `:` made a `/`,
/// `sha256/<hex>`, which is a repository path by the OCI grammar.
/// The asking provider serves the image as
/// `<registry>/<repository>/<name>@<digest>` and reads nothing of the
/// name; the daemon answers one path for one digest wherever the
/// bytes come from.
pub fn name(digest: &str) -> String {
    digest.replacen(':', "/", 1)
}
