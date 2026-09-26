//! What a request's path asks for.

/// A request to the registry, read from the path after `/v2/`: the
/// repository — the first segment, the name the SDK minted for the
/// run — and what is asked under it. The segments between the
/// repository and the last two are the image's name as the caller
/// wrote it, and are not read: the digest identifies the bytes, and
/// the repository identifies the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub repository: String,
    pub kind: Kind,
}

/// The two things a pull asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// `…/manifests/<reference>`, the reference a digest or a tag.
    Manifest(String),
    /// `…/blobs/<digest>`.
    Blob(String),
}

impl Ask {
    /// The ask in `rest`, the path after `/v2/`, or `None` for a path
    /// that asks for neither of the two things — the catalog, a tag
    /// list, referrers, a push's uploads — or that names no
    /// repository or no name.
    pub fn parse(rest: &str) -> Option<Self> {
        let segments: Vec<&str> = rest.split('/').collect();
        // The repository, at least one segment of name, the kind, and
        // its reference.
        if segments.len() < 4 || segments.iter().any(|segment| segment.is_empty()) {
            return None;
        }
        let repository = segments[0].to_string();
        let kind = match (segments[segments.len() - 2], segments[segments.len() - 1]) {
            ("manifests", reference) => Kind::Manifest(reference.to_string()),
            ("blobs", digest) => Kind::Blob(digest.to_string()),
            _ => return None,
        };
        Some(Ask { repository, kind })
    }
}
