/// Extractable file content from a media type.
///
/// `content` is the raw payload (e.g. base64-encoded data).
/// `extension` is the file extension to use (e.g. `"png"`, `"wav"`).
pub struct FileContent<'s> {
    pub content: &'s str,
    pub extension: &'s str,
}

impl FileContent<'_> {
    /// Decodes the base64 content into raw bytes.
    pub fn decode(&self) -> std::io::Result<Vec<u8>> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(self.content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Writes the decoded content to `path` with the extension appended.
    ///
    /// Parent directories are created if they don't exist.
    pub fn write(&self, path: &std::path::Path) -> std::io::Result<()> {
        let path = path.with_extension(self.extension);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, self.decode()?)
    }
}

/// Parses a data URL, returning `(full_mime, base64_payload)`.
///
/// Expects the format `data:{type}/{subtype};base64,{payload}`.
/// Returns `None` if the URL is not a valid base64 data URL.
pub(crate) fn parse_data_url(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("data:")?;
    let (mime, payload) = rest.split_once(";base64,")?;
    Some((mime, payload))
}

/// Maps a full MIME type to a file extension using `mime2ext`.
///
/// Falls back to `"bin"` if the MIME type is not recognized.
pub(crate) fn mime_to_ext(mime: &str) -> &str {
    mime2ext::mime2ext(mime).unwrap_or("bin")
}
