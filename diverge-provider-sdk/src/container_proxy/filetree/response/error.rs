//! Why a frame on the `/filetree` path could not be decoded.

/// A frame that could not be read: the postcard payload would not
/// decode as a filetree frame.
#[derive(Debug)]
pub struct FrameError(pub postcard::Error);

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "filetree frame did not decode: {}", self.0)
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}
