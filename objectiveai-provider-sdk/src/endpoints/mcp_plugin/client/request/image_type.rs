//! Who produces a plugin's image.

/// Who is responsible for producing the image.
///
/// The same three answers a laboratory's creation takes, meaning the
/// same three things — a caller-served registry, the provider's own
/// resolution, or a registry the caller names. Nothing about serving
/// an image differs because the container inside it serves MCP.
///
/// See
/// [`laboratories::create`'s](crate::endpoints::laboratories::create::client::request::ImageType)
/// for what each answer obliges of whom.
pub type ImageType =
    crate::endpoints::laboratories::create::client::request::ImageType;
