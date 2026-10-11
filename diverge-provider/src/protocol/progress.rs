//! What a build says as it goes, and how it ends.

use diverge_sdk::provider::endpoints::images::build::server::response::Built;

/// One item of a build, as an
/// [`ImageManager::Build`](super::image_manager::ImageManager::Build)
/// yields them: a line the build tool wrote, as it wrote it; or, last,
/// the image built, or the word that the store has no room for it. A
/// build that failed is the stream's error, after whatever lines it
/// said, and nothing exists. The handler sends each as the endpoint's
/// frame and finishes the scope at the first that ends the build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// A line of the build's output, decoded: a caller shows it to a
    /// person and does not parse it.
    Output(String),
    /// The image exists, under the name asked for. Nothing follows.
    Built(Built),
    /// The store has no room for the image. Nothing exists, and
    /// nothing follows.
    InsufficientCapacity,
}
