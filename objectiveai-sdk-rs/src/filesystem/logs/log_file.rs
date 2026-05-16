/// A structured log file entry produced by streaming chunks.
///
/// Instead of opaque path strings, each file carries semantic fields
/// (route, id, optional indices) from which the on-disk path is derived.
pub struct LogFile {
    /// Directory route, e.g. `"agents/completions/messages/image"`.
    pub route: String,
    /// Chunk ID, e.g. `"acc-1"`.
    pub id: String,
    /// Message index within the completion (if applicable).
    pub message_index: Option<u64>,
    /// Media part index within the message (if applicable).
    pub media_index: Option<u64>,
    /// File extension without dot, e.g. `"json"`, `"png"`.
    pub extension: String,
    /// File content bytes.
    pub content: Vec<u8>,
}

impl LogFile {
    /// The filename stem without extension, e.g. `"acc-1"`, `"acc-1_0"`, `"acc-1_0_2"`.
    pub fn stem(&self) -> String {
        match (self.message_index, self.media_index) {
            (None, _) => self.id.clone(),
            (Some(mi), None) => format!("{}_{mi}", self.id),
            (Some(mi), Some(mdi)) => format!("{}_{mi}_{mdi}", self.id),
        }
    }

    /// The filename with extension, e.g. `"acc-1.json"`, `"acc-1_0_2.png"`.
    pub fn filename(&self) -> String {
        format!("{}.{}", self.stem(), self.extension)
    }

    /// The full relative path, e.g. `"agents/completions/messages/image/acc-1_0_2.png"`.
    pub fn path(&self) -> String {
        format!("{}/{}", self.route, self.filename())
    }
}
