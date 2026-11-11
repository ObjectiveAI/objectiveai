//! What a deployer read of an image's `/DIVERGE.md`.

/// The answer to [`read_info`](super::container_deployer::ContainerDeployer::read_info):
/// three answers, none of them a failure, since a failure is the
/// deployer's own error.
///
/// The bytes are the file's as the image holds them, unread: whether
/// they are text, and a document, is the handler's to judge, with
/// [`Document::parse`](diverge_sdk::shared::containers::info::Document::parse),
/// so a deployer never speaks the document's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageInfo {
    /// The file, a regular one at the image's root.
    Found(Vec<u8>),
    /// The image ships no `/DIVERGE.md`, or what is at that path is
    /// not a regular file.
    Missing,
    /// The image is in none of the places the deployer looks.
    Unavailable,
}
